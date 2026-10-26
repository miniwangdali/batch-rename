//! Parsing and rendering of the "Replace with" text.
//!
//! Supported syntax (compatible with PowerToys PowerRename):
//! - `$$` literal dollar sign
//! - `$1`..`$99`, `$&`/`$0`, `` $` ``, `$'`, `$<name>` regex references (regex mode only)
//! - `${}`, `${start=1,increment=2,padding=3}` counters (when enumeration is enabled)
//! - `${rstringalnum=N}`, `${rstringalpha=N}`, `${rstringdigit=N}`, `${ruuidv4}` (when random values are enabled)
//! - `$YYYY`, `$MM`, `$DD`, `$hh`, ... date/time of the file

use chrono::{DateTime, Datelike, Local, Timelike};
use serde::Deserialize;

const MAX_GENERATED_LEN: usize = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateToken {
  Year4,
  Year2,
  Year1,
  MonthName,
  MonthAbbr,
  Month2,
  Month1,
  DayName,
  DayAbbr,
  Day2,
  Day1,
  Hour2,
  Hour1,
  Minute2,
  Minute1,
  Second2,
  Second1,
  Milli3,
  Milli2,
  Milli1,
}

// Longer tokens of the same family come first so that `$YYYY` wins over `$YY`.
const DATE_TOKENS: &[(&str, DateToken)] = &[
  ("YYYY", DateToken::Year4),
  ("YY", DateToken::Year2),
  ("Y", DateToken::Year1),
  ("MMMM", DateToken::MonthName),
  ("MMM", DateToken::MonthAbbr),
  ("MM", DateToken::Month2),
  ("M", DateToken::Month1),
  ("DDDD", DateToken::DayName),
  ("DDD", DateToken::DayAbbr),
  ("DD", DateToken::Day2),
  ("D", DateToken::Day1),
  ("hh", DateToken::Hour2),
  ("h", DateToken::Hour1),
  ("mm", DateToken::Minute2),
  ("m", DateToken::Minute1),
  ("ss", DateToken::Second2),
  ("s", DateToken::Second1),
  ("fff", DateToken::Milli3),
  ("ff", DateToken::Milli2),
  ("f", DateToken::Milli1),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Charset {
  Alnum,
  Alpha,
  Digit,
}

#[derive(Debug, Clone, PartialEq)]
enum Segment {
  Lit(String),
  Group(usize),
  Prefix,
  Suffix,
  Date(DateToken, &'static str),
  Counter { start: i64, increment: i64, padding: usize },
  Random(Charset, usize),
  Uuid,
}

/// Localized month/day names used by `$MMMM`, `$MMM`, `$DDDD` and `$DDD`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DateNames {
  pub months: Vec<String>,
  pub months_short: Vec<String>,
  /// Starts with Sunday.
  pub days: Vec<String>,
  /// Starts with Sunday.
  pub days_short: Vec<String>,
}

impl Default for DateNames {
  fn default() -> Self {
    let v = |s: &str| s.split(',').map(str::to_string).collect::<Vec<_>>();
    DateNames {
      months: v("January,February,March,April,May,June,July,August,September,October,November,December"),
      months_short: v("Jan,Feb,Mar,Apr,May,Jun,Jul,Aug,Sep,Oct,Nov,Dec"),
      days: v("Sunday,Monday,Tuesday,Wednesday,Thursday,Friday,Saturday"),
      days_short: v("Sun,Mon,Tue,Wed,Thu,Fri,Sat"),
    }
  }
}

pub struct ParseOptions<'a> {
  /// `Some((group_count, names))` in regex mode; `names[i]` is the name of group `i`, if any.
  pub groups: Option<(usize, &'a [Option<String>])>,
  pub enumerate: bool,
  pub random: bool,
}

pub struct RenderContext<'a> {
  pub text: &'a str,
  /// Capture spans; index 0 is the whole match.
  pub spans: &'a [Option<(usize, usize)>],
  pub index: i64,
  pub time: Option<&'a DateTime<Local>>,
  pub names: &'a DateNames,
}

#[derive(Debug, Clone, Default)]
pub struct Template {
  segments: Vec<Segment>,
}

impl Template {
  pub fn parse(src: &str, opts: &ParseOptions) -> Template {
    let mut segments = Vec::new();
    let mut lit = String::new();
    let mut rest = src;
    while let Some(pos) = rest.find('$') {
      lit.push_str(&rest[..pos]);
      let after = &rest[pos + 1..];
      match parse_dollar(after, opts) {
        Some((seg, consumed)) => {
          if let Segment::Lit(s) = &seg {
            lit.push_str(s);
          } else {
            if !lit.is_empty() {
              segments.push(Segment::Lit(std::mem::take(&mut lit)));
            }
            segments.push(seg);
          }
          rest = &after[consumed..];
        }
        None => {
          lit.push('$');
          rest = after;
        }
      }
    }
    lit.push_str(rest);
    if !lit.is_empty() {
      segments.push(Segment::Lit(lit));
    }
    Template { segments }
  }

  pub fn uses_date(&self) -> bool {
    self.segments.iter().any(|s| matches!(s, Segment::Date(..)))
  }

  pub fn render(&self, ctx: &RenderContext, out: &mut String) {
    let span_text = |i: usize| ctx.spans.get(i).copied().flatten().map(|(s, e)| &ctx.text[s..e]);
    for seg in &self.segments {
      match seg {
        Segment::Lit(s) => out.push_str(s),
        Segment::Group(i) => out.push_str(span_text(*i).unwrap_or("")),
        Segment::Prefix => {
          if let Some(Some((s, _))) = ctx.spans.first() {
            out.push_str(&ctx.text[..*s]);
          }
        }
        Segment::Suffix => {
          if let Some(Some((_, e))) = ctx.spans.first() {
            out.push_str(&ctx.text[*e..]);
          }
        }
        Segment::Date(token, raw) => match ctx.time {
          Some(t) => out.push_str(&format_date(*token, t, ctx.names)),
          None => {
            out.push('$');
            out.push_str(raw);
          }
        },
        Segment::Counter { start, increment, padding } => {
          let value = start.saturating_add(increment.saturating_mul(ctx.index));
          out.push_str(&format!("{value:0padding$}"));
        }
        Segment::Random(charset, len) => {
          out.extend((0..*len).map(|_| match charset {
            Charset::Alnum => fastrand::alphanumeric(),
            Charset::Alpha => fastrand::alphabetic(),
            Charset::Digit => fastrand::digit(10),
          }));
        }
        Segment::Uuid => out.push_str(&uuid_v4()),
      }
    }
  }
}

/// Parses what follows a `$`. Returns the segment and the number of bytes consumed after the `$`.
fn parse_dollar(after: &str, opts: &ParseOptions) -> Option<(Segment, usize)> {
  let first = after.chars().next()?;
  if first == '$' {
    return Some((Segment::Lit("$".into()), 1));
  }
  if first == '{' {
    let close = after.find('}')?;
    return parse_braced(&after[1..close], opts).map(|seg| (seg, close + 1));
  }
  if let Some((count, names)) = opts.groups {
    match first {
      '&' => return Some((Segment::Group(0), 1)),
      '`' => return Some((Segment::Prefix, 1)),
      '\'' => return Some((Segment::Suffix, 1)),
      '<' => {
        let close = after.find('>')?;
        let name = &after[1..close];
        let idx = names.iter().position(|n| n.as_deref() == Some(name))?;
        return Some((Segment::Group(idx), close + 1));
      }
      '0'..='9' => {
        let bytes = after.as_bytes();
        let one = (bytes[0] - b'0') as usize;
        if let Some(d2) = bytes.get(1).filter(|b| b.is_ascii_digit()) {
          let two = one * 10 + (*d2 - b'0') as usize;
          if (1..=count).contains(&two) {
            return Some((Segment::Group(two), 2));
          }
        }
        if one == 0 || one <= count {
          return Some((Segment::Group(one), 1));
        }
        return None;
      }
      _ => {}
    }
  }
  DATE_TOKENS
    .iter()
    .find(|(raw, _)| after.starts_with(raw))
    .map(|(raw, token)| (Segment::Date(*token, raw), raw.len()))
}

fn parse_braced(inner: &str, opts: &ParseOptions) -> Option<Segment> {
  let inner = inner.trim();
  if opts.random {
    if inner == "ruuidv4" {
      return Some(Segment::Uuid);
    }
    if let Some((key, value)) = inner.split_once('=') {
      let charset = match key.trim() {
        "rstringalnum" => Some(Charset::Alnum),
        "rstringalpha" => Some(Charset::Alpha),
        "rstringdigit" => Some(Charset::Digit),
        _ => None,
      };
      if let Some(charset) = charset {
        let len: usize = value.trim().parse().ok()?;
        return Some(Segment::Random(charset, len.min(MAX_GENERATED_LEN)));
      }
    }
  }
  if !opts.enumerate {
    return None;
  }
  let (mut start, mut increment, mut padding) = (0i64, 1i64, 0usize);
  for part in inner.split([',', ';']).map(str::trim).filter(|p| !p.is_empty()) {
    let (key, value) = part.split_once('=')?;
    match key.trim() {
      "start" => start = value.trim().parse().ok()?,
      "increment" => increment = value.trim().parse().ok()?,
      "padding" => padding = value.trim().parse::<usize>().ok()?.min(MAX_GENERATED_LEN),
      _ => return None,
    }
  }
  Some(Segment::Counter { start, increment, padding })
}

fn format_date(token: DateToken, t: &DateTime<Local>, names: &DateNames) -> String {
  let pick = |list: &[String], i: usize| list.get(i).cloned().unwrap_or_default();
  let month0 = t.month0() as usize;
  let weekday = t.weekday().num_days_from_sunday() as usize;
  let millis = t.timestamp_subsec_millis();
  match token {
    DateToken::Year4 => format!("{:04}", t.year()),
    DateToken::Year2 => format!("{:02}", t.year().rem_euclid(100)),
    DateToken::Year1 => format!("{}", t.year().rem_euclid(10)),
    DateToken::MonthName => pick(&names.months, month0),
    DateToken::MonthAbbr => pick(&names.months_short, month0),
    DateToken::Month2 => format!("{:02}", t.month()),
    DateToken::Month1 => t.month().to_string(),
    DateToken::DayName => pick(&names.days, weekday),
    DateToken::DayAbbr => pick(&names.days_short, weekday),
    DateToken::Day2 => format!("{:02}", t.day()),
    DateToken::Day1 => t.day().to_string(),
    DateToken::Hour2 => format!("{:02}", t.hour()),
    DateToken::Hour1 => t.hour().to_string(),
    DateToken::Minute2 => format!("{:02}", t.minute()),
    DateToken::Minute1 => t.minute().to_string(),
    DateToken::Second2 => format!("{:02}", t.second()),
    DateToken::Second1 => t.second().to_string(),
    DateToken::Milli3 => format!("{millis:03}"),
    DateToken::Milli2 => format!("{:02}", millis / 10),
    DateToken::Milli1 => format!("{}", millis / 100),
  }
}

fn uuid_v4() -> String {
  let mut b: [u8; 16] = fastrand::u128(..).to_le_bytes();
  b[6] = (b[6] & 0x0f) | 0x40;
  b[8] = (b[8] & 0x3f) | 0x80;
  let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
  format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

#[cfg(test)]
mod tests {
  use super::*;
  use chrono::TimeZone;

  fn render(src: &str, opts: &ParseOptions, ctx: &RenderContext) -> String {
    let mut out = String::new();
    Template::parse(src, opts).render(ctx, &mut out);
    out
  }

  fn plain(enumerate: bool, random: bool) -> ParseOptions<'static> {
    ParseOptions { groups: None, enumerate, random }
  }

  fn ctx<'a>(index: i64, time: Option<&'a DateTime<Local>>, names: &'a DateNames) -> RenderContext<'a> {
    RenderContext { text: "", spans: &[], index, time, names }
  }

  #[test]
  fn literal_and_escapes() {
    let names = DateNames::default();
    assert_eq!(render("a$$b", &plain(true, true), &ctx(0, None, &names)), "a$b");
    assert_eq!(render("cost $5", &plain(true, true), &ctx(0, None, &names)), "cost $5");
    assert_eq!(render("end$", &plain(true, true), &ctx(0, None, &names)), "end$");
  }

  #[test]
  fn counters() {
    let names = DateNames::default();
    let o = plain(true, false);
    assert_eq!(render("${}", &o, &ctx(0, None, &names)), "0");
    assert_eq!(render("${}", &o, &ctx(3, None, &names)), "3");
    assert_eq!(render("Image_${padding=4;increment=2;start=10}_", &o, &ctx(2, None, &names)), "Image_0014_");
    assert_eq!(render("${start=1, padding=3}", &o, &ctx(0, None, &names)), "001");
    assert_eq!(render("${bogus=1}", &o, &ctx(0, None, &names)), "${bogus=1}");
    assert_eq!(render("${}", &plain(false, false), &ctx(0, None, &names)), "${}");
  }

  #[test]
  fn random_values() {
    let names = DateNames::default();
    let o = plain(false, true);
    let s = render("${rstringdigit=6}", &o, &ctx(0, None, &names));
    assert_eq!(s.len(), 6);
    assert!(s.chars().all(|c| c.is_ascii_digit()));
    let s = render("${rstringalpha=8}", &o, &ctx(0, None, &names));
    assert!(s.len() == 8 && s.chars().all(|c| c.is_ascii_alphabetic()));
    let u = render("{${ruuidv4}}", &o, &ctx(0, None, &names));
    assert_eq!(u.len(), 38);
    assert_eq!(&u[15..16], "4");
    assert!(matches!(&u[20..21], "8" | "9" | "a" | "b"));
    assert_eq!(render("${ruuidv4}", &plain(false, false), &ctx(0, None, &names)), "${ruuidv4}");
  }

  #[test]
  fn dates() {
    let names = DateNames::default();
    let t = Local.with_ymd_and_hms(2020, 11, 2, 7, 5, 9).unwrap() + chrono::Duration::milliseconds(42);
    let o = plain(true, false);
    let c = ctx(0, Some(&t), &names);
    assert_eq!(render("$MMM-$DD-$YY-powertoys", &o, &c), "Nov-02-20-powertoys");
    assert_eq!(render("$YYYY$MM$DD_$hh$mm$ss.$fff", &o, &c), "20201102_070509.042");
    assert_eq!(render("$Y $M $D $h $m $s $ff $f", &o, &c), "0 11 2 7 5 9 04 0");
    assert_eq!(render("$MMMM $DDDD $DDD", &o, &c), "November Monday Mon");
    assert_eq!(render("$YYYY", &o, &ctx(0, None, &names)), "$YYYY");
  }

  #[test]
  fn regex_references() {
    let names = DateNames::default();
    let group_names = vec![None, None, Some("ext".to_string())];
    let o = ParseOptions { groups: Some((2, &group_names)), enumerate: true, random: false };
    let text = "abc.txt";
    let spans = [Some((0, 7)), Some((0, 3)), Some((4, 7))];
    let c = RenderContext { text, spans: &spans, index: 0, time: None, names: &names };
    assert_eq!(render("$2-$1", &o, &c), "txt-abc");
    assert_eq!(render("[$&]", &o, &c), "[abc.txt]");
    assert_eq!(render("$<ext>", &o, &c), "txt");
    assert_eq!(render("$12", &o, &c), "abc2");
    assert_eq!(render("$3", &o, &c), "$3");
  }
}
