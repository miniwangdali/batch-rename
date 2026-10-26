//! A per-user Start menu shortcut, so the portable app can be found without an installer.

use std::path::{Path, PathBuf};

use windows::{
  Win32::{
    System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree, IPersistFile, STGM_READ},
    UI::Shell::{FOLDERID_Programs, IShellLinkW, KF_FLAG_DEFAULT, SHGetKnownFolderPath, ShellLink},
  },
  core::{HSTRING, Interface, Result},
};

use super::com::Com;

const FILE_NAME: &str = "Batch Rename.lnk";

fn shortcut_path() -> Result<PathBuf> {
  let folder = unsafe { SHGetKnownFolderPath(&FOLDERID_Programs, KF_FLAG_DEFAULT, None)? };
  let path = unsafe { folder.to_string() };
  unsafe { CoTaskMemFree(Some(folder.0 as _)) };
  Ok(PathBuf::from(path?).join(FILE_NAME))
}

pub fn is_enabled() -> bool {
  shortcut_path().is_ok_and(|p| p.is_file())
}

pub fn enable(exe: &Path) -> Result<()> {
  let _com = Com::init();
  let link: IShellLinkW = unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)? };
  let exe_w = HSTRING::from(exe.as_os_str());
  unsafe {
    link.SetPath(&exe_w)?;
    if let Some(dir) = exe.parent() {
      link.SetWorkingDirectory(&HSTRING::from(dir.as_os_str()))?;
    }
    link.SetIconLocation(&exe_w, 0)?;
    link.SetDescription(&HSTRING::from("Batch rename files and folders"))?;
    link
      .cast::<IPersistFile>()?
      .Save(&HSTRING::from(shortcut_path()?.as_os_str()), true)
  }
}

pub fn disable() -> std::io::Result<()> {
  match std::fs::remove_file(shortcut_path()?) {
    Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
    _ => Ok(()),
  }
}

fn target() -> Result<PathBuf> {
  let _com = Com::init();
  let link: IShellLinkW = unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)? };
  let mut buf = vec![0u16; 32_768];
  unsafe {
    link
      .cast::<IPersistFile>()?
      .Load(&HSTRING::from(shortcut_path()?.as_os_str()), STGM_READ)?;
    link.GetPath(&mut buf, std::ptr::null_mut(), 0)?;
  }
  let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
  Ok(PathBuf::from(String::from_utf16_lossy(&buf[..len])))
}

/// Points an existing shortcut at `exe` again, e.g. after the portable app was moved.
pub fn sync(exe: &Path) -> Result<()> {
  if is_enabled() && !target().is_ok_and(|t| t.as_path() == exe) {
    enable(exe)?;
  }
  Ok(())
}
