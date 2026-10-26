//! The rename engine: turns an item name plus the user's options into a new name.
//! Pure logic without any file-system access, so it's easy to unit test.

pub mod template;
pub mod textcase;

use chrono::{DateTime, Local};
use fancy_regex::{Regex, RegexBuilder};
use serde::Deserialize;

pub use template::DateNames;
use template::{ParseOptions, RenderContext, Template};
pub use textcase::TextCase;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ApplyTo {
  #[default]
  Name,
  Extension,
  Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TimeSource {
  #[default]
  Created,
  Modified,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Options {
  pub search: String,
  pub replace: String,
  pub use_regex: bool,
  pub match_all: bool,
  pub case_sensitive: bool,
  pub apply_to: ApplyTo,
  pub include_files: bool,
  pub include_folders: bool,
  pub include_subfolders: bool,
  pub text_case: TextCase,
  pub enumerate: bool,
  pub random: bool,
  pub time_source: TimeSource,
  pub date_names: Option<DateNames>,
}

impl Default for Options {
  fn default() -> Self {
    Options {
      search: String::new(),
      replace: String::new(),
      use_regex: false,
      match_all: false,
      case_sensitive: false,
      apply_to: ApplyTo::Name,
      include_files: true,
      include_folders: true,
      include_subfolders: true,
      text_case: TextCase::None,
      enumerate: true,
      random: false,
      time_source: TimeSource::Created,
      date_names: None,
    }
  }
}

pub struct ItemInfo<'a> {
  pub name: &'a str,
  pub is_dir: bool,
  /// 0 for items the user added, > 0 for items found inside an added folder.
  pub depth: u32,
  pub time: Option<DateTime<Local>>,
}

pub struct Renamer {
  regex: Option<Regex>,
  template: Template,
  match_all: bool,
  apply_to: ApplyTo,
  text_case: TextCase,
  include_files: bool,
  include_folders: bool,
  include_subfolders: bool,
  names: DateNames,
  index: i64,
}

const BACKTRACK_LIMIT: usize = 100_000;

impl Renamer {
  pub fn new(opts: &Options) -> Result<Renamer, String> {
    let regex = if opts.search.is_empty() {
      None
    } else {
      let pattern = if opts.use_regex { opts.search.clone() } else { fancy_regex::escape(&opts.search).into_owned() };
      let re = RegexBuilder::new(&pattern)
        .case_insensitive(!opts.case_sensitive)
        .backtrack_limit(BACKTRACK_LIMIT)
        .build()
        .map_err(|e| e.to_string())?;
      Some(re)
    };
    let group_names: Vec<Option<String>> = match &regex {
      Some(re) if opts.use_regex => re.capture_names().map(|n| n.map(str::to_string)).collect(),
      _ => Vec::new(),
    };
    let groups = match &regex {
      Some(re) if opts.use_regex => Some((re.captures_len().saturating_sub(1), group_names.as_slice())),
      _ => None,
    };
    let template = Template::parse(&opts.replace, &ParseOptions { groups, enumerate: opts.enumerate, random: opts.random });
    Ok(Renamer {
      regex,
      template,
      match_all: opts.match_all,
      apply_to: opts.apply_to,
      text_case: opts.text_case,
      include_files: opts.include_files,
      include_folders: opts.include_folders,
      include_subfolders: opts.include_subfolders,
      names: opts.date_names.clone().unwrap_or_default(),
      index: 0,
    })
  }

  /// Whether the replacement needs file timestamps (lets callers skip loading them).
  pub fn uses_time(&self) -> bool {
    self.regex.is_some() && self.template.uses_date()
  }

  pub fn is_included(&self, item: &ItemInfo) -> bool {
    (if item.is_dir { self.include_folders } else { self.include_files })
      && (item.depth == 0 || self.include_subfolders)
  }

  /// Returns the new name, or `None` if the item isn't affected.
  pub fn rename(&mut self, item: &ItemInfo) -> Result<Option<String>, String> {
    if !self.is_included(item) {
      return Ok(None);
    }
    let (stem, ext) = split_extension(item.name, item.is_dir);
    let part = match self.apply_to {
      ApplyTo::Name => stem,
      ApplyTo::Extension if item.is_dir => return Ok(None),
      ApplyTo::Extension => ext.unwrap_or(""),
      ApplyTo::Both => item.name,
    };

    let mut new_part = match &self.regex {
      Some(re) => match self.replace(re, part, item.time.as_ref())? {
        Some(replaced) => {
          self.index += 1;
          replaced
        }
        None => return Ok(None),
      },
      None if self.text_case == TextCase::None => return Ok(None),
      None => part.to_string(),
    };
    new_part = textcase::apply(&new_part, self.text_case);

    let new_name = match self.apply_to {
      ApplyTo::Name => match ext {
        Some(ext) => format!("{new_part}.{ext}"),
        None => new_part,
      },
      ApplyTo::Extension if new_part.is_empty() => stem.to_string(),
      ApplyTo::Extension => format!("{stem}.{new_part}"),
      ApplyTo::Both => new_part,
    };
    Ok((new_name != item.name).then_some(new_name))
  }

  fn replace(&self, re: &Regex, text: &str, time: Option<&DateTime<Local>>) -> Result<Option<String>, String> {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    let mut matched = false;
    let mut spans = Vec::new();
    for caps in re.captures_iter(text) {
      let caps = caps.map_err(|e| e.to_string())?;
      let Some(whole) = caps.get(0) else { continue };
      matched = true;
      spans.clear();
      spans.extend((0..caps.len()).map(|i| caps.get(i).map(|m| (m.start(), m.end()))));
      out.push_str(&text[last..whole.start()]);
      let ctx = RenderContext { text, spans: &spans, index: self.index, time, names: &self.names };
      self.template.render(&ctx, &mut out);
      last = whole.end();
      if !self.match_all {
        break;
      }
    }
    if !matched {
      return Ok(None);
    }
    out.push_str(&text[last..]);
    Ok(Some(out))
  }
}

/// Splits `name` into stem and extension. Folders and dot-files (".gitignore") have no extension.
pub fn split_extension(name: &str, is_dir: bool) -> (&str, Option<&str>) {
  if is_dir {
    return (name, None);
  }
  match name.rfind('.') {
    Some(i) if i > 0 => (&name[..i], Some(&name[i + 1..])),
    _ => (name, None),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use chrono::TimeZone;

  fn opts(search: &str, replace: &str) -> Options {
    Options { search: search.into(), replace: replace.into(), ..Options::default() }
  }

  fn file(name: &str) -> ItemInfo<'_> {
    ItemInfo { name, is_dir: false, depth: 0, time: None }
  }

  fn run(o: &Options, names: &[&str]) -> Vec<Option<String>> {
    let mut r = Renamer::new(o).unwrap();
    names.iter().map(|n| r.rename(&file(n)).unwrap()).collect()
  }

  fn one(o: &Options, name: &str) -> Option<String> {
    run(o, &[name]).pop().unwrap()
  }

  #[test]
  fn match_first_or_all() {
    let mut o = opts("power", "super");
    o.apply_to = ApplyTo::Both;
    assert_eq!(one(&o, "powertoys-powerrename.txt").as_deref(), Some("supertoys-powerrename.txt"));
    o.match_all = true;
    assert_eq!(one(&o, "powertoys-powerrename.txt").as_deref(), Some("supertoys-superrename.txt"));
  }

  #[test]
  fn case_sensitivity() {
    let mut o = opts("foo", "bar");
    assert_eq!(one(&o, "FOO.txt").as_deref(), Some("bar.txt"));
    o.case_sensitive = true;
    assert_eq!(one(&o, "FOO.txt"), None);
  }

  #[test]
  fn literal_mode_escapes_regex_chars() {
    let o = opts("a.b", "x");
    assert_eq!(one(&o, "a.b.txt").as_deref(), Some("x.txt"));
    assert_eq!(one(&o, "axb.txt"), None);
    assert_eq!(one(&opts("(1)", "$1"), "file (1).txt").as_deref(), Some("file $1.txt"));
  }

  #[test]
  fn apply_to_parts() {
    let mut o = opts("txt", "md");
    assert_eq!(one(&o, "txt.txt").as_deref(), Some("md.txt"));
    o.apply_to = ApplyTo::Extension;
    assert_eq!(one(&o, "txt.txt").as_deref(), Some("txt.md"));
    o.apply_to = ApplyTo::Both;
    o.match_all = true;
    assert_eq!(one(&o, "txt.txt").as_deref(), Some("md.md"));

    let mut strip = opts(".*", "");
    strip.use_regex = true;
    strip.apply_to = ApplyTo::Extension;
    assert_eq!(one(&strip, "photo.jpeg").as_deref(), Some("photo"));

    let mut add = opts("^$", "txt");
    add.use_regex = true;
    add.apply_to = ApplyTo::Extension;
    assert_eq!(one(&add, "README").as_deref(), Some("README.txt"));
  }

  #[test]
  fn folders_have_no_extension() {
    let mut o = opts("v1.2", "v2.0");
    let mut r = Renamer::new(&o).unwrap();
    let dir = ItemInfo { name: "lib v1.2", is_dir: true, depth: 0, time: None };
    assert_eq!(r.rename(&dir).unwrap().as_deref(), Some("lib v2.0"));
    o.apply_to = ApplyTo::Extension;
    let mut r = Renamer::new(&o).unwrap();
    assert_eq!(r.rename(&dir).unwrap(), None);
  }

  #[test]
  fn dotfiles_have_no_extension() {
    assert_eq!(split_extension(".gitignore", false), (".gitignore", None));
    assert_eq!(split_extension("a.tar.gz", false), ("a.tar", Some("gz")));
  }

  #[test]
  fn regex_groups() {
    let mut o = opts(r"(\d{2})-(\d{2})-(\d{4})", "$3-$2-$1");
    o.use_regex = true;
    assert_eq!(one(&o, "report 29-03-2020.pdf").as_deref(), Some("report 2020-03-29.pdf"));

    let mut o = opts("^", "foo_");
    o.use_regex = true;
    assert_eq!(one(&o, "bar.png").as_deref(), Some("foo_bar.png"));

    let mut o = opts(".+?(?=bar)", "X");
    o.use_regex = true;
    assert_eq!(one(&o, "foobar.txt").as_deref(), Some("Xbar.txt"));
  }

  #[test]
  fn invalid_regex_is_an_error() {
    let mut o = opts("(", "");
    o.use_regex = true;
    assert!(Renamer::new(&o).is_err());
    assert!(Renamer::new(&opts("(", "")).is_ok());
  }

  #[test]
  fn enumeration_counts_only_matches() {
    let o = opts("a", "Image_${padding=4;increment=2;start=10}_");
    let out = run(&o, &["a.jpg", "x.jpg", "ab.jpg", "abc.jpg"]);
    assert_eq!(
      out,
      vec![Some("Image_0010_.jpg".into()), None, Some("Image_0012_b.jpg".into()), Some("Image_0014_bc.jpg".into())]
    );
  }

  #[test]
  fn formatting_without_search() {
    let mut o = opts("", "");
    assert_eq!(one(&o, "Photo.JPG"), None);
    o.text_case = TextCase::Lower;
    assert_eq!(one(&o, "Photo.JPG").as_deref(), Some("photo.JPG"));
    o.apply_to = ApplyTo::Both;
    assert_eq!(one(&o, "Photo.JPG").as_deref(), Some("photo.jpg"));
    o.text_case = TextCase::Upper;
    o.apply_to = ApplyTo::Extension;
    assert_eq!(one(&o, "photo.jpg").as_deref(), Some("photo.JPG"));
  }

  #[test]
  fn include_filters() {
    let mut o = opts("a", "b");
    o.include_folders = false;
    let mut r = Renamer::new(&o).unwrap();
    assert_eq!(r.rename(&ItemInfo { name: "a", is_dir: true, depth: 0, time: None }).unwrap(), None);
    assert!(r.rename(&ItemInfo { name: "a", is_dir: false, depth: 0, time: None }).unwrap().is_some());
    o.include_subfolders = false;
    let mut r = Renamer::new(&o).unwrap();
    assert_eq!(r.rename(&ItemInfo { name: "a", is_dir: false, depth: 1, time: None }).unwrap(), None);
  }

  #[test]
  fn date_tokens_use_item_time() {
    let o = opts("powertoys", "$MMM-$DD-$YY-powertoys");
    let mut r = Renamer::new(&o).unwrap();
    assert!(r.uses_time());
    let time = Some(Local.with_ymd_and_hms(2020, 11, 2, 0, 0, 0).unwrap());
    let item = ItemInfo { name: "powertoys.png", is_dir: false, depth: 0, time };
    assert_eq!(r.rename(&item).unwrap().as_deref(), Some("Nov-02-20-powertoys.png"));
  }

  #[test]
  fn unchanged_names_return_none() {
    assert_eq!(one(&opts("a", "a"), "a.txt"), None);
  }
}
