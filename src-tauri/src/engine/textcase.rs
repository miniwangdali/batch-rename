//! Text formatting options (lowercase, UPPERCASE, Title Case, Capitalize Each Word).

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextCase {
  #[default]
  None,
  Lower,
  Upper,
  Title,
  Capitalize,
}

// Same small-word list PowerRename keeps lowercase in title case.
const TITLE_EXCEPTIONS: &[&str] = &[
  "a", "an", "to", "the", "at", "by", "for", "in", "of", "on", "up", "and", "as", "but", "or", "nor",
];

fn is_word_char(c: char) -> bool {
  c.is_alphanumeric() || c == '\''
}

fn capitalize(word: &str) -> String {
  let mut chars = word.chars();
  match chars.next() {
    Some(first) => first.to_uppercase().chain(chars.as_str().to_lowercase().chars()).collect(),
    None => String::new(),
  }
}

pub fn apply(text: &str, case: TextCase) -> String {
  match case {
    TextCase::None => text.to_string(),
    TextCase::Lower => text.to_lowercase(),
    TextCase::Upper => text.to_uppercase(),
    TextCase::Title | TextCase::Capitalize => {
      let mut out = String::with_capacity(text.len());
      let mut word_index = 0;
      let mut rest = text;
      while !rest.is_empty() {
        let in_word = rest.chars().next().is_some_and(is_word_char);
        let end = rest.find(|c: char| is_word_char(c) != in_word).unwrap_or(rest.len());
        let (run, tail) = rest.split_at(end);
        if in_word {
          let lower = run.to_lowercase();
          if case == TextCase::Title && word_index > 0 && TITLE_EXCEPTIONS.contains(&lower.as_str()) {
            out.push_str(&lower);
          } else {
            out.push_str(&capitalize(run));
          }
          word_index += 1;
        } else {
          out.push_str(run);
        }
        rest = tail;
      }
      out
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn transforms() {
    assert_eq!(apply("Hello World", TextCase::Lower), "hello world");
    assert_eq!(apply("Hello World", TextCase::Upper), "HELLO WORLD");
    assert_eq!(apply("the lord OF the rings", TextCase::Title), "The Lord of the Rings");
    assert_eq!(apply("the lord OF the rings", TextCase::Capitalize), "The Lord Of The Rings");
    assert_eq!(apply("my_file-name (draft)", TextCase::Capitalize), "My_File-Name (Draft)");
    assert_eq!(apply("don't stop", TextCase::Capitalize), "Don't Stop");
    assert_eq!(apply("照片 photo", TextCase::Capitalize), "照片 Photo");
    assert_eq!(apply("", TextCase::Title), "");
  }
}
