//! `config.txt` next to the DLL: line 1 is the app's path, line 2 the menu title.

use std::{
  fs,
  path::{Path, PathBuf},
};

use windows::Win32::{Foundation::HMODULE, System::LibraryLoader::GetModuleFileNameW};

pub const FILE_NAME: &str = "config.txt";
pub const DEFAULT_TITLE: &str = "Rename with Batch Rename";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
  pub exe: PathBuf,
  pub title: String,
}

impl Config {
  pub fn load() -> Option<Config> {
    Config::read(&module_dir()?.join(FILE_NAME))
  }

  pub fn read(path: &Path) -> Option<Config> {
    let text = fs::read_to_string(path).ok()?;
    let mut lines = text.lines().map(str::trim);
    let exe = lines.next().filter(|l| !l.is_empty())?;
    let title = lines
      .next()
      .filter(|l| !l.is_empty())
      .unwrap_or(DEFAULT_TITLE);
    Some(Config {
      exe: PathBuf::from(exe),
      title: title.to_string(),
    })
  }
}

pub fn title() -> String {
  Config::load().map_or_else(|| DEFAULT_TITLE.to_string(), |c| c.title)
}

fn module_dir() -> Option<PathBuf> {
  let mut buf = vec![0u16; 32_768];
  let len = unsafe { GetModuleFileNameW(Some(HMODULE(crate::module().0)), &mut buf) } as usize;
  if len == 0 || len >= buf.len() {
    return None;
  }
  PathBuf::from(String::from_utf16_lossy(&buf[..len]))
    .parent()
    .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn reads_path_and_title() {
    let t = tempfile::tempdir().unwrap();
    let file = t.path().join(FILE_NAME);
    fs::write(
      &file,
      "C:\\Apps\\batch-rename.exe\r\n使用批量文件改名重命名\r\n",
    )
    .unwrap();
    let c = Config::read(&file).unwrap();
    assert_eq!(c.exe, PathBuf::from("C:\\Apps\\batch-rename.exe"));
    assert_eq!(c.title, "使用批量文件改名重命名");

    fs::write(&file, "C:\\Apps\\batch-rename.exe\n").unwrap();
    assert_eq!(Config::read(&file).unwrap().title, DEFAULT_TITLE);
    fs::write(&file, "\n").unwrap();
    assert_eq!(Config::read(&file), None);
    assert_eq!(Config::read(&t.path().join("missing.txt")), None);
  }
}
