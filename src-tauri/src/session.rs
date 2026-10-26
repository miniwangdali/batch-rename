//! The list of items the user is working on, plus preview/rename/undo bookkeeping.

use std::{
  collections::{HashMap, HashSet},
  fs,
  path::{Path, PathBuf},
  time::SystemTime,
};

use chrono::{DateTime, Local};
use serde::Serialize;

use crate::{
  engine::{ItemInfo, Options, Renamer, TimeSource},
  fsops::{self, Planned},
  naming::{name_key, natural_cmp, path_key, validate_name},
};

pub const MAX_ITEMS: usize = 100_000;

#[derive(Debug, Clone)]
pub struct Item {
  pub id: u32,
  pub name: String,
  pub dir: PathBuf,
  pub parent: Option<u32>,
  pub depth: u32,
  pub is_dir: bool,
  pub created: Option<SystemTime>,
  pub modified: Option<SystemTime>,
}

impl Item {
  pub fn path(&self) -> PathBuf {
    self.dir.join(&self.name)
  }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDto {
  pub id: u32,
  pub name: String,
  pub depth: u32,
  pub is_dir: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub parent: Option<u32>,
  /// Only set for top-level items; children derive their path from the parent.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub dir: Option<String>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddReport {
  pub added: usize,
  pub truncated: bool,
  pub missing: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewEntry {
  pub id: u32,
  pub new_name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub detail: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
  pub version: u64,
  pub entries: Vec<PreviewEntry>,
  pub renamable: usize,
  pub errors: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
  pub id: u32,
  pub name: String,
  pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameReport {
  pub renamed: usize,
  pub failures: Vec<Failure>,
  pub can_undo: bool,
}

#[derive(Debug, Clone)]
struct HistoryEntry {
  id: u32,
  old: String,
  new: String,
}

#[derive(Default)]
pub struct Session {
  items: Vec<Item>,
  index: HashMap<u32, usize>,
  next_id: u32,
  plan: Option<(u64, Vec<(u32, String)>)>,
  version: u64,
  history: Vec<Vec<HistoryEntry>>,
  dir_cache: HashMap<String, HashSet<String>>,
}

struct Pending {
  path: PathBuf,
  name: String,
  parent: Option<u32>,
  depth: u32,
  meta: fs::Metadata,
}

impl Session {
  pub fn items(&self) -> Vec<ItemDto> {
    self
      .items
      .iter()
      .map(|i| ItemDto {
        id: i.id,
        name: i.name.clone(),
        depth: i.depth,
        is_dir: i.is_dir,
        parent: i.parent,
        dir: i.parent.is_none().then(|| i.dir.to_string_lossy().into_owned()),
      })
      .collect()
  }

  pub fn can_undo(&self) -> bool {
    !self.history.is_empty()
  }

  pub fn clear(&mut self) {
    *self = Session { next_id: self.next_id, version: self.version, ..Session::default() };
  }

  /// Adds files/folders (folders recursively). Paths already in the list are skipped; adding a
  /// folder that contains previously added items replaces them with the folder's tree.
  pub fn add_paths(&mut self, paths: &[PathBuf]) -> AddReport {
    let mut report = AddReport::default();
    let mut roots: Vec<Vec<Item>> = self.take_roots();
    let mut known: HashSet<String> = roots.iter().flatten().map(|i| path_key(&i.path())).collect();

    for raw in paths {
      let Ok(path) = std::path::absolute(raw) else { continue };
      let (Some(name), Some(dir)) = (path.file_name().and_then(|n| n.to_str()), path.parent()) else {
        report.missing.push(raw.to_string_lossy().into_owned());
        continue;
      };
      let Ok(meta) = fs::symlink_metadata(&path) else {
        report.missing.push(raw.to_string_lossy().into_owned());
        continue;
      };
      let key = path_key(&path);
      let inside_existing = roots.iter().any(|tree| {
        let root = &tree[0];
        root.is_dir && key.starts_with(&format!("{}{}", path_key(&root.path()), std::path::MAIN_SEPARATOR))
      });
      if known.contains(&key) || inside_existing {
        continue;
      }
      if meta.is_dir() {
        let prefix = format!("{key}{}", std::path::MAIN_SEPARATOR);
        roots.retain(|tree| !path_key(&tree[0].path()).starts_with(&prefix));
      }
      let budget = MAX_ITEMS.saturating_sub(roots.iter().map(Vec::len).sum());
      if budget == 0 {
        report.truncated = true;
        break;
      }
      let root = Pending { path: path.clone(), name: name.to_string(), parent: None, depth: 0, meta };
      let (tree, truncated) = self.scan(root, dir, budget);
      report.truncated |= truncated;
      report.added += tree.len();
      known.extend(tree.iter().map(|i| path_key(&i.path())));
      roots.push(tree);
    }

    roots.sort_by(|a, b| {
      let (a, b) = (&a[0], &b[0]);
      natural_cmp(&a.dir.to_string_lossy(), &b.dir.to_string_lossy())
        .then_with(|| b.is_dir.cmp(&a.is_dir))
        .then_with(|| natural_cmp(&a.name, &b.name))
    });
    self.items = roots.into_iter().flatten().collect();
    self.reindex();
    self.invalidate();
    report
  }

  fn take_roots(&mut self) -> Vec<Vec<Item>> {
    let mut roots: Vec<Vec<Item>> = Vec::new();
    for item in std::mem::take(&mut self.items) {
      match (item.parent, roots.last_mut()) {
        (Some(_), Some(tree)) => tree.push(item),
        _ => roots.push(vec![item]),
      }
    }
    roots
  }

  /// Depth-first scan producing items in display order (parents before children, folders first).
  fn scan(&mut self, root: Pending, root_dir: &Path, mut budget: usize) -> (Vec<Item>, bool) {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(p) = stack.pop() {
      if budget == 0 {
        return (out, true);
      }
      budget -= 1;
      let id = self.next_id;
      self.next_id += 1;
      let is_symlink = p.meta.file_type().is_symlink();
      let is_dir = if is_symlink { fs::metadata(&p.path).is_ok_and(|m| m.is_dir()) } else { p.meta.is_dir() };
      let dir = p.path.parent().map_or_else(|| root_dir.to_path_buf(), Path::to_path_buf);
      out.push(Item {
        id,
        name: p.name,
        dir,
        parent: p.parent,
        depth: p.depth,
        is_dir,
        created: p.meta.created().ok(),
        modified: p.meta.modified().ok(),
      });
      if is_dir && !is_symlink {
        let Ok(entries) = fs::read_dir(&p.path) else { continue };
        let mut children: Vec<Pending> = entries
          .filter_map(Result::ok)
          .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let meta = e.metadata().ok()?;
            Some(Pending { path: e.path(), name, parent: Some(id), depth: p.depth + 1, meta })
          })
          .collect();
        // Reverse order because the stack pops from the end.
        children.sort_by(|a, b| a.meta.is_dir().cmp(&b.meta.is_dir()).then_with(|| natural_cmp(&b.name, &a.name)));
        stack.extend(children);
      }
    }
    (out, false)
  }

  fn reindex(&mut self) {
    self.index = self.items.iter().enumerate().map(|(i, item)| (item.id, i)).collect();
  }

  fn invalidate(&mut self) {
    self.plan = None;
    self.dir_cache.clear();
  }

  /// Recomputes `dir` of nested items after folders were renamed. Items are stored parents-first.
  fn refresh_dirs(&mut self) {
    for i in 0..self.items.len() {
      if let Some(parent) = self.items[i].parent.and_then(|p| self.index.get(&p).copied()) {
        self.items[i].dir = self.items[parent].path();
      }
    }
  }

  fn dir_contains(&mut self, dir: &Path, name: &str) -> bool {
    let names = self.dir_cache.entry(path_key(dir)).or_insert_with(|| {
      fs::read_dir(dir)
        .map(|entries| {
          entries.filter_map(Result::ok).map(|e| name_key(&e.file_name().to_string_lossy())).collect()
        })
        .unwrap_or_default()
    });
    names.contains(&name_key(name))
  }

  pub fn preview(&mut self, options: &Options, deselected: &HashSet<u32>) -> Result<Preview, String> {
    let mut renamer = Renamer::new(options)?;
    let with_time = renamer.uses_time();
    let mut proposals: Vec<(usize, String)> = Vec::new();
    let mut errors: HashMap<usize, (&'static str, Option<String>)> = HashMap::new();

    for (idx, item) in self.items.iter().enumerate() {
      if deselected.contains(&item.id) {
        continue;
      }
      let time = with_time
        .then(|| match options.time_source {
          TimeSource::Created => item.created.or(item.modified),
          TimeSource::Modified => item.modified.or(item.created),
        })
        .flatten()
        .map(DateTime::<Local>::from);
      let info = ItemInfo { name: &item.name, is_dir: item.is_dir, depth: item.depth, time };
      match renamer.rename(&info) {
        Ok(Some(new_name)) => {
          if let Err(e) = validate_name(&new_name) {
            errors.insert(proposals.len(), (e.code(), None));
          }
          proposals.push((idx, new_name));
        }
        Ok(None) => {}
        Err(message) => {
          errors.insert(proposals.len(), ("regex", Some(message)));
          proposals.push((idx, item.name.clone()));
        }
      }
    }

    // Two proposals in the same folder must not end up with the same name.
    let target_keys: Vec<String> = proposals
      .iter()
      .map(|(idx, new_name)| format!("{}\n{}", path_key(&self.items[*idx].dir), name_key(new_name)))
      .collect();
    let mut by_target: HashMap<&str, Vec<usize>> = HashMap::new();
    for (n, key) in target_keys.iter().enumerate() {
      by_target.entry(key).or_default().push(n);
    }
    for group in by_target.values().filter(|g| g.len() > 1) {
      for &n in group {
        errors.entry(n).or_insert(("duplicate", None));
      }
    }

    // A target may be occupied by another proposal that moves away; otherwise it must be free.
    let current: HashMap<String, usize> = proposals
      .iter()
      .enumerate()
      .map(|(n, p)| (format!("{}\n{}", path_key(&self.items[p.0].dir), name_key(&self.items[p.0].name)), n))
      .collect();
    let mut dependents: HashMap<usize, Vec<usize>> = HashMap::new();
    for (n, p) in proposals.iter().enumerate() {
      if errors.contains_key(&n) {
        continue;
      }
      match current.get(&target_keys[n]) {
        Some(&own) if own == n => {}
        Some(&other) => dependents.entry(other).or_default().push(n),
        None => {
          let dir = self.items[p.0].dir.clone();
          if self.dir_contains(&dir, &p.1) {
            errors.insert(n, ("exists", None));
          }
        }
      }
    }
    let mut work: Vec<usize> = errors.keys().copied().collect();
    while let Some(n) = work.pop() {
      for &d in dependents.get(&n).map(Vec::as_slice).unwrap_or_default() {
        if let std::collections::hash_map::Entry::Vacant(e) = errors.entry(d) {
          e.insert(("exists", None));
          work.push(d);
        }
      }
    }

    self.version += 1;
    let mut plan = Vec::new();
    let entries = proposals
      .into_iter()
      .enumerate()
      .map(|(n, (idx, new_name))| {
        let id = self.items[idx].id;
        let (error, detail) = match errors.remove(&n) {
          Some((code, detail)) => (Some(code.to_string()), detail),
          None => {
            plan.push((id, new_name.clone()));
            (None, None)
          }
        };
        PreviewEntry { id, new_name, error, detail }
      })
      .collect::<Vec<_>>();
    let renamable = plan.len();
    self.plan = Some((self.version, plan));
    Ok(Preview { version: self.version, errors: entries.len() - renamable, entries, renamable })
  }

  /// Renames according to the preview with the given version.
  pub fn rename(&mut self, version: u64) -> Result<RenameReport, String> {
    let plan = match self.plan.take() {
      Some((v, plan)) if v == version => plan,
      _ => return Err("The preview is out of date. Please try again.".into()),
    };
    let planned = plan
      .into_iter()
      .filter_map(|(id, to)| {
        let item = &self.items[*self.index.get(&id)?];
        Some(Planned { id, dir: item.dir.clone(), from: item.name.clone(), to })
      })
      .collect();
    Ok(self.apply(planned, true))
  }

  pub fn undo(&mut self) -> Result<RenameReport, String> {
    let batch = self.history.pop().ok_or("Nothing to undo.")?;
    let planned = batch
      .into_iter()
      .filter_map(|h| {
        let item = &self.items[*self.index.get(&h.id)?];
        (item.name == h.new).then(|| Planned { id: h.id, dir: item.dir.clone(), from: h.new, to: h.old })
      })
      .collect();
    Ok(self.apply(planned, false))
  }

  fn apply(&mut self, planned: Vec<Planned>, record: bool) -> RenameReport {
    let names: HashMap<u32, (String, String)> = planned.iter().map(|p| (p.id, (p.from.clone(), p.to.clone()))).collect();
    let mut history = Vec::new();
    let mut failures = Vec::new();
    for outcome in fsops::execute(planned) {
      let (from, to) = &names[&outcome.id];
      match outcome.result {
        Ok(()) => {
          if let Some(&i) = self.index.get(&outcome.id) {
            self.items[i].name = to.clone();
          }
          history.push(HistoryEntry { id: outcome.id, old: from.clone(), new: to.clone() });
        }
        Err(message) => failures.push(Failure { id: outcome.id, name: from.clone(), message }),
      }
    }
    self.refresh_dirs();
    self.invalidate();
    let renamed = history.len();
    if record && !history.is_empty() {
      self.history.push(history);
    }
    RenameReport { renamed, failures, can_undo: self.can_undo() }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn setup(files: &[&str]) -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    for f in files {
      let p = t.path().join(f);
      if f.ends_with('/') {
        fs::create_dir_all(&p).unwrap();
      } else {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, f).unwrap();
      }
    }
    t
  }

  fn opts(search: &str, replace: &str) -> Options {
    Options { search: search.into(), replace: replace.into(), ..Options::default() }
  }

  fn preview_map(p: &Preview) -> HashMap<u32, (String, Option<String>)> {
    p.entries.iter().map(|e| (e.id, (e.new_name.clone(), e.error.clone()))).collect()
  }

  #[test]
  fn scans_folders_recursively_in_natural_order() {
    let t = setup(&["root/b10.txt", "root/b2.txt", "root/sub/c.txt", "root/a.txt"]);
    let mut s = Session::default();
    let report = s.add_paths(&[t.path().join("root")]);
    assert_eq!(report.added, 6);
    let names: Vec<_> = s.items().iter().map(|i| (i.name.clone(), i.depth)).collect();
    assert_eq!(
      names,
      [("root", 0), ("sub", 1), ("c.txt", 2), ("a.txt", 1), ("b2.txt", 1), ("b10.txt", 1)]
        .map(|(n, d)| (n.to_string(), d))
    );
    // Adding a file that is already inside the folder is a no-op.
    assert_eq!(s.add_paths(&[t.path().join("root").join("a.txt")]).added, 0);
    let missing = s.add_paths(&[t.path().join("nope")]);
    assert_eq!(missing.missing.len(), 1);
  }

  #[test]
  fn adding_parent_folder_replaces_children() {
    let t = setup(&["root/a.txt"]);
    let mut s = Session::default();
    s.add_paths(&[t.path().join("root").join("a.txt")]);
    s.add_paths(&[t.path().join("root")]);
    assert_eq!(s.items().len(), 2);
  }

  #[test]
  fn preview_detects_conflicts() {
    let t = setup(&["a.txt", "b.txt", "keep.txt", "x1.txt", "x2.txt"]);
    let mut s = Session::default();
    let files: Vec<_> = ["a.txt", "b.txt", "x1.txt", "x2.txt"].iter().map(|f| t.path().join(f)).collect();
    s.add_paths(&files);
    let id = |name: &str| s.items.iter().find(|i| i.name == name).unwrap().id;
    let (a, b, x1, x2) = (id("a.txt"), id("b.txt"), id("x1.txt"), id("x2.txt"));

    let mut o = opts("^(a|b)$", "keep");
    o.use_regex = true;
    let p = s.preview(&o, &HashSet::new()).unwrap();
    let m = preview_map(&p);
    assert_eq!(m[&a].1.as_deref(), Some("duplicate"));
    assert_eq!(m[&b].1.as_deref(), Some("duplicate"));

    let mut o = opts(r"^x\d$", "keep");
    o.use_regex = true;
    let p = s.preview(&o, &HashSet::from([x2])).unwrap();
    assert_eq!(preview_map(&p)[&x1].1.as_deref(), Some("exists"));
    assert_eq!(p.renamable, 0);

    let mut swap = opts("^a$", "b");
    swap.use_regex = true;
    let p = s.preview(&swap, &HashSet::new()).unwrap();
    assert_eq!(preview_map(&p)[&a].1.as_deref(), Some("exists"));
  }

  #[test]
  fn rename_and_undo_with_folders() {
    let t = setup(&["photos/img1.jpg", "photos/img2.jpg"]);
    let mut s = Session::default();
    s.add_paths(&[t.path().join("photos")]);
    let mut o = opts("(img|photos)", "pic_$1");
    o.use_regex = true;
    let p = s.preview(&o, &HashSet::new()).unwrap();
    assert_eq!(p.renamable, 3);
    let report = s.rename(p.version).unwrap();
    assert_eq!(report.renamed, 3, "{:?}", report.failures);
    assert!(t.path().join("pic_photos").join("pic_img1.jpg").exists());
    // Paths of nested items follow the renamed folder.
    let child = s.items.iter().find(|i| i.name == "pic_img2.jpg").unwrap();
    assert_eq!(child.dir, t.path().join("pic_photos"));
    // A stale version is rejected.
    assert!(s.rename(p.version).is_err());

    let undo = s.undo().unwrap();
    assert_eq!(undo.renamed, 3);
    assert!(t.path().join("photos").join("img1.jpg").exists());
    assert!(!s.can_undo());
  }

  #[test]
  fn counters_follow_display_order() {
    let t = setup(&["b.txt", "a.txt", "c.txt"]);
    let mut s = Session::default();
    s.add_paths(&["c.txt", "a.txt", "b.txt"].map(|f| t.path().join(f)));
    let o = opts(".*", "file_${start=1}");
    let mut o = o;
    o.use_regex = true;
    let p = s.preview(&o, &HashSet::new()).unwrap();
    let names: Vec<_> = p.entries.iter().map(|e| e.new_name.as_str()).collect();
    assert_eq!(names, ["file_1.txt", "file_2.txt", "file_3.txt"]);
    let r = s.rename(p.version).unwrap();
    assert_eq!(r.renamed, 3);
    assert_eq!(fs::read_to_string(t.path().join("file_1.txt")).unwrap(), "a.txt");
  }
}
