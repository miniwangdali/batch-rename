//! File-name helpers: validation, platform-aware comparison keys and natural sorting.

use std::{cmp::Ordering, path::Path};

/// Comparison key for names on the current platform. Windows and macOS file systems are
/// case-insensitive by default, so names that differ only by case refer to the same entry.
pub fn name_key(name: &str) -> String {
  if cfg!(any(windows, target_os = "macos")) {
    name.to_lowercase()
  } else {
    name.to_string()
  }
}

pub fn path_key(path: &Path) -> String {
  name_key(&path.to_string_lossy())
}

/// Error codes are sent to the UI, which translates them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameError {
  Empty,
  IllegalChars,
  Reserved,
  Trailing,
  TooLong,
}

impl NameError {
  pub fn code(self) -> &'static str {
    match self {
      NameError::Empty => "empty",
      NameError::IllegalChars => "illegalChars",
      NameError::Reserved => "reserved",
      NameError::Trailing => "trailing",
      NameError::TooLong => "tooLong",
    }
  }
}

const MAX_NAME_LEN: usize = 255;

pub fn validate_name(name: &str) -> Result<(), NameError> {
  if name.is_empty() || name == "." || name == ".." {
    return Err(NameError::Empty);
  }
  if name.contains(['/', '\0']) {
    return Err(NameError::IllegalChars);
  }
  if cfg!(windows) {
    if name.chars().any(|c| matches!(c, '<' | '>' | ':' | '"' | '\\' | '|' | '?' | '*') || (c as u32) < 32) {
      return Err(NameError::IllegalChars);
    }
    if name.ends_with(['.', ' ']) {
      return Err(NameError::Trailing);
    }
    if is_reserved_windows_name(name) {
      return Err(NameError::Reserved);
    }
    if name.encode_utf16().count() > MAX_NAME_LEN {
      return Err(NameError::TooLong);
    }
  } else if name.len() > MAX_NAME_LEN {
    return Err(NameError::TooLong);
  }
  Ok(())
}

fn is_reserved_windows_name(name: &str) -> bool {
  let stem = name.split('.').next().unwrap_or(name).trim_end().to_ascii_uppercase();
  match stem.as_str() {
    "CON" | "PRN" | "AUX" | "NUL" => true,
    s if s.len() == 4 && (s.starts_with("COM") || s.starts_with("LPT")) => {
      matches!(s.as_bytes()[3], b'1'..=b'9')
    }
    _ => false,
  }
}

/// Explorer-like ordering: digit runs compare numerically, everything else case-insensitively.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
  let mut ai = a.chars().peekable();
  let mut bi = b.chars().peekable();
  loop {
    match (ai.peek().copied(), bi.peek().copied()) {
      (None, None) => return a.cmp(b),
      (None, Some(_)) => return Ordering::Less,
      (Some(_), None) => return Ordering::Greater,
      (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
        let na = take_digits(&mut ai);
        let nb = take_digits(&mut bi);
        let (ta, tb) = (na.trim_start_matches('0'), nb.trim_start_matches('0'));
        let ord = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb)).then_with(|| na.len().cmp(&nb.len()));
        if ord != Ordering::Equal {
          return ord;
        }
      }
      (Some(x), Some(y)) => {
        let ord = x.to_lowercase().cmp(y.to_lowercase());
        if ord != Ordering::Equal {
          return ord;
        }
        ai.next();
        bi.next();
      }
    }
  }
}

fn take_digits(it: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
  let mut s = String::new();
  while let Some(c) = it.peek().copied().filter(char::is_ascii_digit) {
    s.push(c);
    it.next();
  }
  s
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn natural_order() {
    let mut v = vec!["file10.txt", "file2.txt", "File1.txt", "file02.txt", "a", "B"];
    v.sort_by(|a, b| natural_cmp(a, b));
    assert_eq!(v, ["a", "B", "File1.txt", "file2.txt", "file02.txt", "file10.txt"]);
  }

  #[test]
  fn validation_common() {
    assert_eq!(validate_name(""), Err(NameError::Empty));
    assert_eq!(validate_name(".."), Err(NameError::Empty));
    assert_eq!(validate_name("a/b"), Err(NameError::IllegalChars));
    assert_eq!(validate_name(&"x".repeat(256)), Err(NameError::TooLong));
    assert!(validate_name("ok name.txt").is_ok());
  }

  #[cfg(windows)]
  #[test]
  fn validation_windows() {
    assert_eq!(validate_name("a:b"), Err(NameError::IllegalChars));
    assert_eq!(validate_name("a?"), Err(NameError::IllegalChars));
    assert_eq!(validate_name("name."), Err(NameError::Trailing));
    assert_eq!(validate_name("name "), Err(NameError::Trailing));
    assert_eq!(validate_name("con.txt"), Err(NameError::Reserved));
    assert_eq!(validate_name("LPT1"), Err(NameError::Reserved));
    assert!(validate_name("console.txt").is_ok());
    assert!(validate_name("COM0").is_ok());
  }
}
