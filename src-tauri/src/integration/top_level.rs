//! "Rename with Batch Rename" at the top of the Windows 11 File Explorer context menu.
//!
//! Windows 11 only shows handlers with package identity there, so the app registers a sparse
//! package (built by `build.rs`) whose external location is a folder in the app's local data
//! with the handler DLL and its `config.txt`. The package is signed with a certificate created
//! on this PC (see `signing`) that the user trusts once through an elevation prompt (`trust`).

use std::{
  fs, io,
  path::{Path, PathBuf},
};

use windows::{
  Foundation::Uri,
  Management::Deployment::{AddPackageOptions, PackageManager},
  Win32::{
    Foundation::HWND,
    System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance},
    UI::Shell::{IPackageDebugSettings, PackageDebugSettings},
  },
  core::{HSTRING, Result},
};
use winreg::{RegKey, enums::HKEY_LOCAL_MACHINE};

use super::{
  com::Com,
  signing,
  trust::{self, TrustError},
};

pub const PUBLISHER: &str = "CN=Batch Rename Context Menu";
const PACKAGE_NAME: &str = "BatchRename.ContextMenu";
const DLL_NAME: &str = "context-menu.dll";
const CONFIG_NAME: &str = "config.txt";
const PACKAGE_FILE: &str = "context-menu.msix";

static DLL: &[u8] = include_bytes!(env!("CONTEXT_MENU_DLL"));
static PACKAGE: &[u8] = include_bytes!(env!("CONTEXT_MENU_MSIX"));

/// The top-level menu exists since Windows 11 (build 22000).
pub fn is_available() -> bool {
  RegKey::predef(HKEY_LOCAL_MACHINE)
    .open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
    .and_then(|key| key.get_value::<String, _>("CurrentBuildNumber"))
    .ok()
    .and_then(|build| build.parse::<u32>().ok())
    .is_some_and(|build| build >= 22000)
}

/// Full name of the registered package, if any.
fn find_package() -> Result<Option<HSTRING>> {
  let _com = Com::init();
  let manager = PackageManager::new()?;
  // An empty user SID means the current user.
  let packages = manager.FindPackagesByUserSecurityIdNamePublisher(
    &HSTRING::new(),
    &PACKAGE_NAME.into(),
    &PUBLISHER.into(),
  )?;
  match packages.into_iter().next() {
    Some(package) => Ok(Some(package.Id()?.FullName()?)),
    None => Ok(None),
  }
}

pub fn is_enabled() -> bool {
  find_package().is_ok_and(|p| p.is_some())
}

fn file_uri(path: &Path) -> Result<Uri> {
  let text = path.to_string_lossy();
  let text = text
    .strip_prefix(r"\\?\")
    .unwrap_or(&text)
    .replace('\\', "/");
  let mut uri = String::from(if text.starts_with("//") {
    "file:"
  } else {
    "file:///"
  });
  // Non-ASCII characters stay as they are: Windows.Foundation.Uri takes IRIs and would decode
  // percent-encoded UTF-8 byte by byte.
  for c in text.chars() {
    match c {
      'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' | '/' | ':' => uri.push(c),
      c if c.is_ascii() => uri.push_str(&format!("%{:02X}", c as u8)),
      c => uri.push(c),
    }
  }
  Uri::CreateUri(&uri.into())
}

fn write_if_changed(path: &Path, contents: &[u8]) -> io::Result<()> {
  if fs::read(path).is_ok_and(|current| current == contents) {
    return Ok(());
  }
  match fs::write(path, contents) {
    // A DLL that Explorer has loaded can't be overwritten, but it can be renamed.
    Err(_) if path.exists() => {
      fs::rename(
        path,
        path.with_extension(format!("{:x}.old", fastrand::u64(..))),
      )?;
      fs::write(path, contents)
    }
    result => result,
  }
}

/// Writes the handler and its configuration into `dir`, the package's external location.
fn write_files(dir: &Path, exe: &Path, title: &str) -> io::Result<()> {
  fs::create_dir_all(dir)?;
  let title: String = title.chars().filter(|c| !c.is_control()).collect();
  write_if_changed(
    &dir.join(CONFIG_NAME),
    format!("{}\n{title}\n", exe.display()).as_bytes(),
  )?;
  write_if_changed(&dir.join(DLL_NAME), DLL)?;
  // Remove DLLs replaced earlier that are no longer in use.
  for entry in fs::read_dir(dir)?.flatten() {
    if entry.file_name().to_string_lossy().ends_with(".old") {
      let _ = fs::remove_file(entry.path());
    }
  }
  Ok(())
}

pub enum EnableError {
  Cancelled,
  Failed(String),
}

impl From<io::Error> for EnableError {
  fn from(e: io::Error) -> Self {
    EnableError::Failed(e.to_string())
  }
}

impl From<windows::core::Error> for EnableError {
  fn from(e: windows::core::Error) -> Self {
    EnableError::Failed(e.message())
  }
}

pub fn enable(
  dir: &Path,
  exe: &Path,
  title: &str,
  owner: HWND,
) -> std::result::Result<(), EnableError> {
  write_files(dir, exe, title)?;
  if find_package()?.is_some() {
    return Ok(());
  }
  let package = dir.join(PACKAGE_FILE);
  fs::write(&package, PACKAGE)?;
  let result = sign_trust_register(&package, dir, owner);
  let _ = fs::remove_file(&package);
  result
}

fn sign_trust_register(
  package: &Path,
  dir: &Path,
  owner: HWND,
) -> std::result::Result<(), EnableError> {
  let certificate = signing::sign_package(package, PUBLISHER)?;
  trust::request(&certificate, owner).map_err(|e| match e {
    TrustError::Cancelled => EnableError::Cancelled,
    TrustError::Failed(message) => EnableError::Failed(message),
  })?;
  register(package, dir).map_err(EnableError::Failed)
}

/// Waits for a deployment operation. Failures can be reported either by the operation itself or
/// only in its result, whose error text is much more helpful than the HRESULT.
fn wait(
  operation: &windows_future::IAsyncOperationWithProgress<
    windows::Management::Deployment::DeploymentResult,
    windows::Management::Deployment::DeploymentProgress,
  >,
) -> std::result::Result<(), String> {
  let joined = operation.join();
  let result = match &joined {
    Ok(result) => Some(result.clone()),
    Err(_) => operation.GetResults().ok(),
  };
  let failed = joined.is_err()
    || result
      .as_ref()
      .is_some_and(|r| r.ExtendedErrorCode().is_ok_and(|c| c.is_err()));
  if !failed {
    return Ok(());
  }
  let details = result
    .and_then(|r| r.ErrorText().ok())
    .map(|t| t.to_string())
    .unwrap_or_default();
  Err(match (details.is_empty(), joined) {
    (false, _) => details,
    (true, Err(e)) => e.message(),
    (true, Ok(_)) => "The package deployment failed.".into(),
  })
}

fn register(package: &Path, dir: &Path) -> std::result::Result<(), String> {
  let _com = Com::init();
  let run = || -> Result<_> {
    let options = AddPackageOptions::new()?;
    options.SetExternalLocationUri(&file_uri(dir)?)?;
    PackageManager::new()?.AddPackageByUriAsync(&file_uri(package)?, &options)
  };
  wait(&run().map_err(|e| e.message())?)
}

pub fn disable(dir: &Path) -> std::result::Result<(), String> {
  if let Some(full_name) = find_package().map_err(|e| e.message())? {
    let _com = Com::init();
    // Explorer keeps the handler loaded in a COM surrogate (dllhost.exe) that runs with the
    // package's identity, and removal waits for it indefinitely. End it first; Explorer simply
    // stops showing the entry once the package is gone.
    unsafe {
      if let Ok(debug) = CoCreateInstance::<_, IPackageDebugSettings>(
        &PackageDebugSettings,
        None,
        CLSCTX_INPROC_SERVER,
      ) {
        let _ = debug.TerminateAllProcesses(&full_name);
      }
    }
    let operation = PackageManager::new()
      .and_then(|m| m.RemovePackageAsync(&full_name))
      .map_err(|e| e.message())?;
    wait(&operation)?;
  }
  // The DLL may still be loaded for a moment; leftovers are removed next time.
  let _ = fs::remove_dir_all(dir);
  Ok(())
}

/// Updates the handler's files (app path, title, DLL) without asking for elevation.
pub fn sync(dir: &Path, exe: &Path, title: &str) -> io::Result<()> {
  if is_enabled() {
    write_files(dir, exe, title)
  } else {
    Ok(())
  }
}

pub fn files_dir(data_dir: &Path) -> PathBuf {
  data_dir.join("context-menu")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn file_uris() {
    let uri = |p: &str| {
      file_uri(Path::new(p))
        .unwrap()
        .AbsoluteUri()
        .unwrap()
        .to_string()
    };
    assert_eq!(
      uri(r"C:\Users\me\AppData\x.msix"),
      "file:///C:/Users/me/AppData/x.msix"
    );
    assert_eq!(uri(r"\\?\C:\a\b"), "file:///C:/a/b");
    let unicode = uri(r"C:\My Apps\100%\批量");
    assert!(
      unicode == "file:///C:/My%20Apps/100%25/批量"
        || unicode == "file:///C:/My%20Apps/100%25/%E6%89%B9%E9%87%8F",
      "{unicode}"
    );
  }

  #[test]
  fn embedded_files_are_present() {
    assert_eq!(&DLL[..2], b"MZ");
    assert_eq!(&PACKAGE[..2], b"PK");
  }

  #[test]
  fn writes_handler_files() {
    let t = tempfile::tempdir().unwrap();
    let dir = t.path().join("context-menu");
    write_files(
      &dir,
      Path::new(r"C:\Apps\batch-rename.exe"),
      "Rename\nwith Batch Rename",
    )
    .unwrap();
    assert_eq!(
      fs::read_to_string(dir.join(CONFIG_NAME)).unwrap(),
      "C:\\Apps\\batch-rename.exe\nRenamewith Batch Rename\n"
    );
    assert_eq!(fs::read(dir.join(DLL_NAME)).unwrap(), DLL);
  }

  /// Signs the real package and lets Windows check it without trusting the certificate.
  /// "The root certificate ... must be trusted" proves the package and its signature are valid
  /// and that the publisher matches the certificate.
  #[test]
  fn signed_package_is_valid_but_untrusted() {
    let t = tempfile::tempdir().unwrap();
    let package = t.path().join(PACKAGE_FILE);
    fs::write(&package, PACKAGE).unwrap();
    signing::sign_package(&package, PUBLISHER).unwrap();
    let error = register(&package, t.path()).unwrap_err();
    assert!(
      error.contains("0x800B0109") || error.contains("must be trusted"),
      "{error}"
    );
  }
}
