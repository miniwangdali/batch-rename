//! "Rename with Batch Rename" for the Windows 11 File Explorer context menu.
//!
//! Explorer loads this DLL through the app's sparse package (see `package/AppxManifest.xml`).
//! The app writes `config.txt` next to the DLL with its own path and the localized menu title;
//! the handler starts the app with the selected items.
#![cfg(windows)]

mod command;
mod config;
mod launch;

use std::{
  ffi::c_void,
  ptr::null_mut,
  sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
};

use windows::{
  Win32::{
    Foundation::{
      CLASS_E_CLASSNOTAVAILABLE, CLASS_E_NOAGGREGATION, E_POINTER, HINSTANCE, S_FALSE, S_OK,
    },
    System::{
      Com::{IClassFactory, IClassFactory_Impl},
      SystemServices::DLL_PROCESS_ATTACH,
    },
    UI::Shell::IExplorerCommand,
  },
  core::{BOOL, GUID, HRESULT, IUnknown, Interface, Ref, Result, implement},
};

use command::RenameCommand;

/// Must match the `Clsid` in `package/AppxManifest.xml`.
pub const CLSID_RENAME_COMMAND: GUID = GUID::from_u128(0x2a2f3fb3_3ae2_42ad_98b9_9e56ec55e778);

static MODULE: AtomicPtr<c_void> = AtomicPtr::new(null_mut());
/// Live COM objects and server locks; the DLL may be unloaded when this reaches zero.
static REFERENCES: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn module() -> HINSTANCE {
  HINSTANCE(MODULE.load(Ordering::Relaxed))
}

#[unsafe(no_mangle)]
extern "system" fn DllMain(module: HINSTANCE, reason: u32, _reserved: *mut c_void) -> BOOL {
  if reason == DLL_PROCESS_ATTACH {
    MODULE.store(module.0, Ordering::Relaxed);
  }
  true.into()
}

/// # Safety
/// Called by COM with valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllGetClassObject(
  clsid: *const GUID,
  iid: *const GUID,
  out: *mut *mut c_void,
) -> HRESULT {
  if out.is_null() {
    return E_POINTER;
  }
  unsafe { *out = null_mut() };
  if clsid.is_null() || iid.is_null() || unsafe { *clsid } != CLSID_RENAME_COMMAND {
    return CLASS_E_CLASSNOTAVAILABLE;
  }
  let factory: IClassFactory = Factory.into();
  unsafe { factory.query(iid, out) }
}

#[unsafe(no_mangle)]
extern "system" fn DllCanUnloadNow() -> HRESULT {
  if REFERENCES.load(Ordering::SeqCst) == 0 {
    S_OK
  } else {
    S_FALSE
  }
}

#[implement(IClassFactory)]
struct Factory;

impl IClassFactory_Impl for Factory_Impl {
  fn CreateInstance(
    &self,
    outer: Ref<IUnknown>,
    iid: *const GUID,
    object: *mut *mut c_void,
  ) -> Result<()> {
    if object.is_null() {
      return Err(E_POINTER.into());
    }
    unsafe { *object = null_mut() };
    if outer.is_some() {
      return Err(CLASS_E_NOAGGREGATION.into());
    }
    let command: IExplorerCommand = RenameCommand::new().into();
    unsafe { command.query(iid, object) }.ok()
  }

  fn LockServer(&self, lock: BOOL) -> Result<()> {
    if lock.as_bool() {
      REFERENCES.fetch_add(1, Ordering::SeqCst);
    } else {
      REFERENCES.fetch_sub(1, Ordering::SeqCst);
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use windows::Win32::System::Com::IClassFactory;

  #[test]
  fn class_factory_creates_the_command() {
    let mut factory: Option<IClassFactory> = None;
    let hr = unsafe {
      DllGetClassObject(
        &CLSID_RENAME_COMMAND,
        &IClassFactory::IID,
        &mut factory as *mut _ as *mut *mut c_void,
      )
    };
    assert_eq!(hr, S_OK);
    let command: IExplorerCommand = unsafe { factory.unwrap().CreateInstance(None) }.unwrap();
    assert_eq!(
      unsafe { command.GetCanonicalName() }.unwrap(),
      CLSID_RENAME_COMMAND
    );

    let mut none: Option<IClassFactory> = None;
    let other = GUID::from_u128(1);
    let hr = unsafe {
      DllGetClassObject(
        &other,
        &IClassFactory::IID,
        &mut none as *mut _ as *mut *mut c_void,
      )
    };
    assert_eq!(hr, CLASS_E_CLASSNOTAVAILABLE);
    assert!(none.is_none());
  }
}
