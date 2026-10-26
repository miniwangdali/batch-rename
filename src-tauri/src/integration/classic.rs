//! "Rename with Batch Rename" in the classic File Explorer context menu, used on Windows 10
//! (Windows 11 gets the top-level entry instead). Uses per-user registry keys, so no admin rights
//! are needed.

use std::{io, path::Path};

use winreg::{RegKey, enums::HKEY_CURRENT_USER};

const VERB: &str = "BatchRename";
const PARENTS: [&str; 2] = [
  r"Software\Classes\*\shell",
  r"Software\Classes\Directory\shell",
];

pub fn is_enabled() -> bool {
  let hkcu = RegKey::predef(HKEY_CURRENT_USER);
  PARENTS.iter().all(|parent| {
    hkcu
      .open_subkey(format!(r"{parent}\{VERB}\command"))
      .is_ok()
  })
}

pub fn enable(exe: &Path, label: &str) -> io::Result<()> {
  let hkcu = RegKey::predef(HKEY_CURRENT_USER);
  for parent in PARENTS {
    let (key, _) = hkcu.create_subkey(format!(r"{parent}\{VERB}"))?;
    key.set_value("MUIVerb", &label)?;
    key.set_value("Icon", &format!("{},0", exe.display()))?;
    // "Player" lifts Explorer's 15-item limit; the extra launches are merged by the
    // single-instance plugin.
    key.set_value("MultiSelectModel", &"Player")?;
    let (command, _) = key.create_subkey("command")?;
    command.set_value("", &format!("\"{}\" \"%1\"", exe.display()))?;
  }
  Ok(())
}

pub fn disable() -> io::Result<()> {
  let hkcu = RegKey::predef(HKEY_CURRENT_USER);
  for parent in PARENTS {
    match hkcu.delete_subkey_all(format!(r"{parent}\{VERB}")) {
      Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
      _ => {}
    }
  }
  Ok(())
}
