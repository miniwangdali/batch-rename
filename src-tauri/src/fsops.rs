//! File-system side of renaming: a rename that never overwrites, and a batch executor that
//! handles swaps/chains (a→b, b→a) through temporary names and renames children before parents.

use std::{
  cmp::Reverse,
  collections::HashSet,
  io,
  path::{Path, PathBuf},
};

use crate::naming::path_key;

#[derive(Debug, Clone)]
pub struct Planned {
  pub id: u32,
  pub dir: PathBuf,
  pub from: String,
  pub to: String,
}

#[derive(Debug)]
pub struct Outcome {
  pub id: u32,
  pub result: Result<(), String>,
}

/// Renames `from` to `to`, failing instead of replacing an existing entry. Renaming to a name
/// that differs only by case (same file) is allowed.
#[cfg(windows)]
pub fn rename_no_replace(from: &Path, to: &Path) -> io::Result<()> {
  use windows_sys::Win32::Storage::FileSystem::MoveFileExW;
  let (from_w, to_w) = (to_wide(from), to_wide(to));
  // Flags = 0: no MOVEFILE_REPLACE_EXISTING, no copying across volumes.
  if unsafe { MoveFileExW(from_w.as_ptr(), to_w.as_ptr(), 0) } == 0 {
    Err(io::Error::last_os_error())
  } else {
    Ok(())
  }
}

#[cfg(windows)]
fn to_wide(path: &Path) -> Vec<u16> {
  use std::os::windows::ffi::OsStrExt;
  let s = path.as_os_str();
  let raw: Vec<u16> = s.encode_wide().collect();
  let text = String::from_utf16_lossy(&raw);
  // Long paths need the verbatim prefix when calling Win32 directly.
  let prefix: Vec<u16> = if raw.len() < 248 || text.starts_with(r"\\?\") {
    Vec::new()
  } else if let Some(unc) = text.strip_prefix(r"\\") {
    return r"\\?\UNC\"
      .encode_utf16()
      .chain(unc.encode_utf16())
      .chain(Some(0))
      .collect();
  } else {
    r"\\?\".encode_utf16().collect()
  };
  prefix.into_iter().chain(raw).chain(Some(0)).collect()
}

#[cfg(unix)]
pub fn rename_no_replace(from: &Path, to: &Path) -> io::Result<()> {
  use std::os::unix::fs::MetadataExt;
  match std::fs::symlink_metadata(to) {
    Ok(target) => {
      let source = std::fs::symlink_metadata(from)?;
      if source.dev() == target.dev() && source.ino() == target.ino() {
        std::fs::rename(from, to)
      } else {
        Err(io::Error::new(
          io::ErrorKind::AlreadyExists,
          "an item with that name already exists",
        ))
      }
    }
    Err(e) if e.kind() == io::ErrorKind::NotFound => std::fs::rename(from, to),
    Err(e) => Err(e),
  }
}

fn temp_path(dir: &Path) -> PathBuf {
  loop {
    let candidate = dir.join(format!(".~batch-rename-{:016x}.tmp", fastrand::u64(..)));
    if std::fs::symlink_metadata(&candidate).is_err() {
      return candidate;
    }
  }
}

fn describe(e: &io::Error) -> String {
  e.to_string()
}

/// Executes a batch of renames. Deeper entries go first so that renaming a folder doesn't
/// invalidate the paths of its children.
pub fn execute(mut planned: Vec<Planned>) -> Vec<Outcome> {
  planned.sort_by_key(|p| Reverse(p.dir.components().count()));
  let mut outcomes = Vec::with_capacity(planned.len());
  let mut start = 0;
  while start < planned.len() {
    let depth = planned[start].dir.components().count();
    let end = planned[start..]
      .iter()
      .position(|p| p.dir.components().count() != depth)
      .map_or(planned.len(), |n| start + n);
    execute_level(&planned[start..end], &mut outcomes);
    start = end;
  }
  outcomes
}

fn execute_level(group: &[Planned], outcomes: &mut Vec<Outcome>) {
  let current: HashSet<String> = group
    .iter()
    .map(|p| path_key(&p.dir.join(&p.from)))
    .collect();
  let mut direct = Vec::new();
  let mut via_temp = Vec::new();
  for p in group {
    let own = path_key(&p.dir.join(&p.from));
    let target = path_key(&p.dir.join(&p.to));
    if target != own && current.contains(&target) {
      via_temp.push(p);
    } else {
      direct.push(p);
    }
  }

  // Phase 1: move entries whose target is still occupied by another entry of this batch aside.
  let mut parked = Vec::new();
  for p in via_temp {
    let tmp = temp_path(&p.dir);
    match rename_no_replace(&p.dir.join(&p.from), &tmp) {
      Ok(()) => parked.push((p, tmp)),
      Err(e) => outcomes.push(Outcome {
        id: p.id,
        result: Err(describe(&e)),
      }),
    }
  }
  // Phase 2: everything that can go straight to its new name.
  for p in direct {
    let result =
      rename_no_replace(&p.dir.join(&p.from), &p.dir.join(&p.to)).map_err(|e| describe(&e));
    outcomes.push(Outcome { id: p.id, result });
  }
  // Phase 3: parked entries to their final names; put them back if that fails.
  for (p, tmp) in parked {
    let result = match rename_no_replace(&tmp, &p.dir.join(&p.to)) {
      Ok(()) => Ok(()),
      Err(e) => match rename_no_replace(&tmp, &p.dir.join(&p.from)) {
        Ok(()) => Err(describe(&e)),
        Err(_) => Err(format!("{} (left as {})", describe(&e), tmp.display())),
      },
    };
    outcomes.push(Outcome { id: p.id, result });
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;

  fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
      .unwrap()
      .map(|e| e.unwrap().file_name().into_string().unwrap())
      .collect();
    v.sort();
    v
  }

  fn plan(id: u32, dir: &Path, from: &str, to: &str) -> Planned {
    Planned {
      id,
      dir: dir.to_path_buf(),
      from: from.into(),
      to: to.into(),
    }
  }

  #[test]
  fn never_overwrites() {
    let t = tempfile::tempdir().unwrap();
    fs::write(t.path().join("a.txt"), "a").unwrap();
    fs::write(t.path().join("b.txt"), "b").unwrap();
    assert!(rename_no_replace(&t.path().join("a.txt"), &t.path().join("b.txt")).is_err());
    assert_eq!(fs::read_to_string(t.path().join("b.txt")).unwrap(), "b");
  }

  #[test]
  fn case_only_rename() {
    let t = tempfile::tempdir().unwrap();
    fs::write(t.path().join("photo.jpg"), "x").unwrap();
    rename_no_replace(&t.path().join("photo.jpg"), &t.path().join("Photo.JPG")).unwrap();
    assert_eq!(names(t.path()), ["Photo.JPG"]);
  }

  #[test]
  fn swap_and_chain() {
    let t = tempfile::tempdir().unwrap();
    for (n, c) in [("a", "A"), ("b", "B"), ("1", "one"), ("2", "two")] {
      fs::write(t.path().join(n), c).unwrap();
    }
    let out = execute(vec![
      plan(0, t.path(), "a", "b"),
      plan(1, t.path(), "b", "a"),
      plan(2, t.path(), "1", "2"),
      plan(3, t.path(), "2", "3"),
    ]);
    assert!(out.iter().all(|o| o.result.is_ok()), "{out:?}");
    assert_eq!(fs::read_to_string(t.path().join("a")).unwrap(), "B");
    assert_eq!(fs::read_to_string(t.path().join("b")).unwrap(), "A");
    assert_eq!(fs::read_to_string(t.path().join("2")).unwrap(), "one");
    assert_eq!(fs::read_to_string(t.path().join("3")).unwrap(), "two");
    assert_eq!(names(t.path()), ["2", "3", "a", "b"]);
  }

  #[test]
  fn children_before_parents() {
    let t = tempfile::tempdir().unwrap();
    let dir = t.path().join("old");
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join("x.txt"), "x").unwrap();
    let out = execute(vec![
      plan(0, t.path(), "old", "new"),
      plan(1, &dir, "x.txt", "y.txt"),
    ]);
    assert!(out.iter().all(|o| o.result.is_ok()), "{out:?}");
    assert!(t.path().join("new").join("y.txt").exists());
  }

  #[test]
  fn reports_conflicts_with_unrelated_files() {
    let t = tempfile::tempdir().unwrap();
    fs::write(t.path().join("a"), "a").unwrap();
    fs::write(t.path().join("taken"), "keep").unwrap();
    let out = execute(vec![plan(0, t.path(), "a", "taken")]);
    assert!(out[0].result.is_err());
    assert_eq!(fs::read_to_string(t.path().join("taken")).unwrap(), "keep");
    assert!(t.path().join("a").exists());
  }
}
