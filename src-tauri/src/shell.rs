//! Windows File Explorer integration: a "Rename with Batch Rename" entry in the context menu of
//! files and folders. Uses per-user registry keys, so no admin rights are needed. On Windows 11
//! the entry shows up under "Show more options".

use std::io;

use winreg::{RegKey, enums::HKEY_CURRENT_USER};

const VERB: &str = "BatchRename";
const PARENTS: [&str; 2] = [r"Software\Classes\*\shell", r"Software\Classes\Directory\shell"];

fn command_line() -> io::Result<String> {
  let exe = std::env::current_exe()?;
  Ok(format!("\"{}\" \"%1\"", exe.display()))
}

pub fn is_enabled() -> bool {
  let Ok(expected) = command_line() else { return false };
  let hkcu = RegKey::predef(HKEY_CURRENT_USER);
  PARENTS.iter().all(|parent| {
    hkcu
      .open_subkey(format!(r"{parent}\{VERB}\command"))
      .and_then(|k| k.get_value::<String, _>(""))
      .is_ok_and(|cmd| cmd.eq_ignore_ascii_case(&expected))
  })
}

pub fn set_enabled(enabled: bool, label: &str) -> io::Result<()> {
  let hkcu = RegKey::predef(HKEY_CURRENT_USER);
  for parent in PARENTS {
    let path = format!(r"{parent}\{VERB}");
    if enabled {
      let exe = std::env::current_exe()?;
      let (key, _) = hkcu.create_subkey(&path)?;
      key.set_value("MUIVerb", &label)?;
      key.set_value("Icon", &format!("{},0", exe.display()))?;
      // "Player" lifts Explorer's 15-item limit; extra launches are merged by the single-instance plugin.
      key.set_value("MultiSelectModel", &"Player")?;
      let (command, _) = key.create_subkey("command")?;
      command.set_value("", &command_line()?)?;
    } else {
      match hkcu.delete_subkey_all(&path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
        _ => {}
      }
    }
  }
  Ok(())
}
