//! Trusting the context menu certificate needs administrator rights, so the app starts itself
//! elevated with `--trust-cert <hex>`. That instance only updates the machine's Trusted People
//! store and exits. The certificate is passed on the command line rather than in a file so it
//! can't be swapped while the elevation prompt is showing.

use std::fmt::Write;

use windows::{
  Win32::{
    Foundation::{CloseHandle, ERROR_CANCELLED, HWND, WAIT_OBJECT_0},
    Security::Cryptography::*,
    System::Threading::{GetExitCodeProcess, INFINITE, WaitForSingleObject},
    UI::{
      Shell::{SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW},
      WindowsAndMessaging::SW_HIDE,
    },
  },
  core::{Error, HRESULT, HSTRING, PCWSTR, Result, w},
};

use super::top_level::PUBLISHER;

pub const FLAG: &str = "--trust-cert";
const ENCODING: CERT_QUERY_ENCODING_TYPE =
  CERT_QUERY_ENCODING_TYPE(X509_ASN_ENCODING.0 | PKCS_7_ASN_ENCODING.0);

pub enum TrustError {
  Cancelled,
  Failed(String),
}

fn to_hex(bytes: &[u8]) -> String {
  bytes
    .iter()
    .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
      let _ = write!(s, "{b:02x}");
      s
    })
}

fn from_hex(text: &str) -> Option<Vec<u8>> {
  if !text.len().is_multiple_of(2) {
    return None;
  }
  (0..text.len())
    .step_by(2)
    .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
    .collect()
}

/// Asks for elevation (UAC) and adds `certificate` to the machine's Trusted People store.
pub fn request(certificate: &[u8], owner: HWND) -> std::result::Result<(), TrustError> {
  let exe = std::env::current_exe().map_err(|e| TrustError::Failed(e.to_string()))?;
  let file = HSTRING::from(exe.as_os_str());
  let params = HSTRING::from(format!("{FLAG} {}", to_hex(certificate)));
  let mut info = SHELLEXECUTEINFOW {
    cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
    fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
    hwnd: owner,
    lpVerb: w!("runas"),
    lpFile: PCWSTR(file.as_ptr()),
    lpParameters: PCWSTR(params.as_ptr()),
    nShow: SW_HIDE.0,
    ..Default::default()
  };
  if let Err(e) = unsafe { ShellExecuteExW(&mut info) } {
    return Err(if e.code() == HRESULT::from_win32(ERROR_CANCELLED.0) {
      TrustError::Cancelled
    } else {
      TrustError::Failed(e.message())
    });
  }
  let mut code = 1u32;
  unsafe {
    if WaitForSingleObject(info.hProcess, INFINITE) == WAIT_OBJECT_0 {
      let _ = GetExitCodeProcess(info.hProcess, &mut code);
    }
    let _ = CloseHandle(info.hProcess);
  }
  match code {
    0 => Ok(()),
    code => Err(TrustError::Failed(format!(
      "Couldn't trust the certificate (error {code})."
    ))),
  }
}

/// Handles `--trust-cert` when the app was started elevated by [`request`]. Returns the exit code,
/// or `None` for a normal launch.
pub fn run_elevated() -> Option<i32> {
  let mut args = std::env::args().skip(1);
  if args.next().as_deref() != Some(FLAG) {
    return None;
  }
  let Some(der) = args.next().and_then(|hex| from_hex(&hex)) else {
    return Some(2);
  };
  Some(if trust(&der).is_ok() { 0 } else { 1 })
}

fn subject(context: *const CERT_CONTEXT) -> String {
  let mut buf = [0u16; 512];
  let len = unsafe {
    CertNameToStrW(
      ENCODING,
      &(*(*context).pCertInfo).Subject,
      CERT_X500_NAME_STR,
      Some(&mut buf),
    )
  };
  String::from_utf16_lossy(&buf[..(len as usize).saturating_sub(1)])
}

struct Store(HCERTSTORE);

impl Drop for Store {
  fn drop(&mut self) {
    unsafe {
      let _ = CertCloseStore(Some(self.0), 0);
    }
  }
}

fn trust(der: &[u8]) -> Result<()> {
  let context = unsafe { CertCreateCertificateContext(ENCODING, der) };
  if context.is_null() {
    return Err(Error::from_thread());
  }
  let ours = subject(context) == PUBLISHER;
  unsafe {
    let _ = CertFreeCertificateContext(Some(context));
  }
  // This helper runs elevated; it must only ever trust the app's own certificate.
  if !ours {
    return Err(Error::empty());
  }
  let store = Store(unsafe {
    CertOpenStore(
      CERT_STORE_PROV_SYSTEM_W,
      CERT_QUERY_ENCODING_TYPE(0),
      None,
      CERT_OPEN_STORE_FLAGS(CERT_SYSTEM_STORE_LOCAL_MACHINE),
      Some(w!("TrustedPeople").as_ptr().cast()),
    )?
  });
  // Certificates from earlier activations are useless: their keys no longer exist.
  let mut previous: *mut CERT_CONTEXT = std::ptr::null_mut();
  loop {
    previous = unsafe {
      CertFindCertificateInStore(store.0, ENCODING, 0, CERT_FIND_ANY, None, Some(previous))
    };
    if previous.is_null() {
      break;
    }
    if subject(previous) == PUBLISHER {
      unsafe { CertDeleteCertificateFromStore(CertDuplicateCertificateContext(Some(previous)))? };
    }
  }
  unsafe {
    CertAddEncodedCertificateToStore(
      Some(store.0),
      ENCODING,
      der,
      CERT_STORE_ADD_REPLACE_EXISTING,
      None,
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn hex_round_trip() {
    let bytes = [0u8, 1, 0xab, 0xff];
    assert_eq!(to_hex(&bytes), "0001abff");
    assert_eq!(from_hex("0001abff").unwrap(), bytes);
    assert_eq!(from_hex("abc"), None);
    assert_eq!(from_hex("zz"), None);
  }
}
