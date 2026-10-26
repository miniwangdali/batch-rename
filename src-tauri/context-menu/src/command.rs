//! The `IExplorerCommand` Explorer shows in its context menu.

use std::sync::atomic::Ordering;

use windows::{
  Win32::{
    Foundation::{E_FAIL, E_NOTIMPL},
    System::Com::{CoTaskMemFree, IBindCtx},
    UI::Shell::{
      ECF_DEFAULT, ECS_ENABLED, ECS_HIDDEN, IEnumExplorerCommand, IExplorerCommand,
      IExplorerCommand_Impl, IShellItemArray, SHStrDupW, SIGDN_FILESYSPATH,
    },
  },
  core::{BOOL, Error, GUID, HRESULT, HSTRING, PWSTR, Ref, Result, implement},
};

use crate::{CLSID_RENAME_COMMAND, REFERENCES, config, launch};

#[implement(IExplorerCommand)]
pub struct RenameCommand;

impl RenameCommand {
  pub fn new() -> Self {
    REFERENCES.fetch_add(1, Ordering::SeqCst);
    RenameCommand
  }
}

impl Default for RenameCommand {
  fn default() -> Self {
    Self::new()
  }
}

impl Drop for RenameCommand {
  fn drop(&mut self) {
    REFERENCES.fetch_sub(1, Ordering::SeqCst);
  }
}

fn dup(text: &str) -> Result<PWSTR> {
  unsafe { SHStrDupW(&HSTRING::from(text)) }
}

/// File-system paths of the selected items; virtual items (without a path) are skipped.
pub fn item_paths(items: &IShellItemArray) -> Result<Vec<String>> {
  let count = unsafe { items.GetCount()? };
  let mut paths = Vec::with_capacity(count as usize);
  for i in 0..count {
    let item = unsafe { items.GetItemAt(i)? };
    if let Ok(name) = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) } {
      let path = unsafe { name.to_string() };
      unsafe { CoTaskMemFree(Some(name.0 as _)) };
      if let Ok(path) = path {
        paths.push(path);
      }
    }
  }
  Ok(paths)
}

impl IExplorerCommand_Impl for RenameCommand_Impl {
  fn GetTitle(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
    dup(&config::title())
  }

  fn GetIcon(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
    let config = config::Config::load().ok_or(Error::from(E_FAIL))?;
    dup(&format!("{},0", config.exe.display()))
  }

  fn GetToolTip(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
    Err(E_NOTIMPL.into())
  }

  fn GetCanonicalName(&self) -> Result<GUID> {
    Ok(CLSID_RENAME_COMMAND)
  }

  fn GetState(&self, _items: Ref<IShellItemArray>, _ok_to_be_slow: BOOL) -> Result<u32> {
    // Hide the entry if the (portable) app was moved or deleted and hasn't updated the config.
    let available = config::Config::load().is_some_and(|c| c.exe.is_file());
    Ok(if available { ECS_ENABLED } else { ECS_HIDDEN }.0 as u32)
  }

  fn Invoke(&self, items: Ref<IShellItemArray>, _bind: Ref<IBindCtx>) -> Result<()> {
    let config = config::Config::load().ok_or(Error::from(E_FAIL))?;
    let paths = item_paths(items.ok()?)?;
    launch::launch(&config.exe, &paths)
      .map_err(|e| Error::from(HRESULT::from_win32(e.raw_os_error().unwrap_or(0x1f) as u32)))
  }

  fn GetFlags(&self) -> Result<u32> {
    Ok(ECF_DEFAULT.0 as u32)
  }

  fn EnumSubCommands(&self) -> Result<IEnumExplorerCommand> {
    Err(E_NOTIMPL.into())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use windows::Win32::{
    System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx},
    UI::Shell::{IShellItem, SHCreateItemFromParsingName, SHCreateShellItemArrayFromShellItem},
  };

  #[test]
  fn reads_paths_from_shell_items() {
    unsafe {
      let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
    let t = tempfile::tempdir().unwrap();
    let file = t.path().join("photo 1.jpg");
    std::fs::write(&file, "x").unwrap();
    let item: IShellItem =
      unsafe { SHCreateItemFromParsingName(&HSTRING::from(file.as_os_str()), None) }.unwrap();
    let array: IShellItemArray = unsafe { SHCreateShellItemArrayFromShellItem(&item) }.unwrap();
    assert_eq!(
      item_paths(&array).unwrap(),
      vec![file.to_string_lossy().into_owned()]
    );

    let command: IExplorerCommand = RenameCommand::new().into();
    // Without config.txt next to the module the title falls back and the entry is hidden.
    let title = unsafe { command.GetTitle(None) }.unwrap();
    assert_eq!(unsafe { title.to_string() }.unwrap(), config::DEFAULT_TITLE);
    unsafe { CoTaskMemFree(Some(title.0 as _)) };
    assert_eq!(
      unsafe { command.GetState(None, false) }.unwrap(),
      ECS_HIDDEN.0 as u32
    );
  }
}
