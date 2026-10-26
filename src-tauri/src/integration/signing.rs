//! Signs the context menu package with a certificate created on this PC. The private key is
//! created under a random name in the user's key store, used for this one signature and deleted
//! right after, so nothing can sign other packages with the certificate the user trusts. All
//! key operations are silent: they fail rather than show UI (such as a smart card prompt).

use std::{
  ffi::c_void,
  mem::size_of,
  path::Path,
  ptr::{addr_of, null, null_mut},
};

use windows::{
  Win32::{
    Foundation::{HANDLE, HWND},
    Security::Cryptography::*,
    System::SystemInformation::GetSystemTime,
  },
  core::{Error, HSTRING, IUnknown, Interface, PCSTR, PCWSTR, PSTR, PWSTR, Result},
};

/// `SIGNER_SIGN_EX2_PARAMS` and `APPX_SIP_CLIENT_DATA` from the Windows SDK; the `windows`
/// crate doesn't include them. See "How to programmatically sign an app package".
#[repr(C)]
struct SignerSignEx2Params {
  flags: u32,
  subject_info: *const SIGNER_SUBJECT_INFO,
  signing_cert: *const SIGNER_CERT,
  signature_info: *const SIGNER_SIGNATURE_INFO,
  provider_info: *const SIGNER_PROVIDER_INFO,
  timestamp_flags: u32,
  algorithm_oid: PCSTR,
  timestamp_url: PCWSTR,
  crypt_attrs: *const CRYPT_ATTRIBUTES,
  sip_data: *mut c_void,
  signer_context: *mut *mut SIGNER_CONTEXT,
  crypto_policy: *const c_void,
  reserved: *const c_void,
}

#[repr(C)]
struct AppxSipClientData {
  signer_params: *const SignerSignEx2Params,
  appx_sip_state: *mut c_void,
}

/// An NCrypt provider handle, freed on drop.
struct Provider(NCRYPT_PROV_HANDLE);

impl Drop for Provider {
  fn drop(&mut self) {
    unsafe {
      let _ = NCryptFreeObject(NCRYPT_HANDLE(self.0.0));
    }
  }
}

/// A persisted key that is deleted from the key store on drop.
struct TemporaryKey(NCRYPT_KEY_HANDLE);

impl Drop for TemporaryKey {
  fn drop(&mut self) {
    unsafe {
      // Deleting also frees the handle; only free it separately if deleting failed.
      if NCryptDeleteKey(self.0, NCRYPT_SILENT_FLAG.0).is_err() {
        let _ = NCryptFreeObject(NCRYPT_HANDLE(self.0.0));
      }
    }
  }
}

struct Certificate(*mut CERT_CONTEXT);

impl Drop for Certificate {
  fn drop(&mut self) {
    unsafe {
      let _ = CertFreeCertificateContext(Some(self.0));
    }
  }
}

impl Certificate {
  fn der(&self) -> Vec<u8> {
    unsafe {
      std::slice::from_raw_parts((*self.0).pbCertEncoded, (*self.0).cbCertEncoded as usize).to_vec()
    }
  }
}

/// A certificate and its private key. Fields drop in order: the certificate (and any key handle
/// it cached while signing) goes first, then the key is deleted, then its provider is closed.
struct SigningCertificate {
  certificate: Certificate,
  _key: TemporaryKey,
  _provider: Provider,
  #[cfg_attr(not(test), allow(dead_code))]
  key_name: String,
}

/// Signs the package at `package` in place and returns the DER-encoded certificate to trust.
pub fn sign_package(package: &Path, subject: &str) -> Result<Vec<u8>> {
  let signing = create_certificate(subject)?;
  sign(package, &signing.certificate)?;
  Ok(signing.certificate.der())
}

/// Adds the failing step to an error, which makes signing problems diagnosable.
fn at<T>(step: &str, result: Result<T>) -> Result<T> {
  result.map_err(|e| Error::new(e.code(), format!("{step}: {}", e.message())))
}

fn encode(struct_type: PCSTR, info: *const c_void) -> Result<Vec<u8>> {
  let mut len = 0u32;
  unsafe {
    CryptEncodeObjectEx(
      X509_ASN_ENCODING,
      struct_type,
      info,
      CRYPT_ENCODE_OBJECT_FLAGS(0),
      None,
      None,
      &mut len,
    )?;
  }
  let mut buf = vec![0u8; len as usize];
  unsafe {
    CryptEncodeObjectEx(
      X509_ASN_ENCODING,
      struct_type,
      info,
      CRYPT_ENCODE_OBJECT_FLAGS(0),
      None,
      Some(buf.as_mut_ptr().cast()),
      &mut len,
    )?;
  }
  buf.truncate(len as usize);
  Ok(buf)
}

fn extension(oid: PCSTR, critical: bool, value: &mut [u8]) -> CERT_EXTENSION {
  CERT_EXTENSION {
    pszObjId: PSTR(oid.0 as *mut u8),
    fCritical: critical.into(),
    Value: CRYPT_INTEGER_BLOB {
      cbData: value.len() as u32,
      pbData: value.as_mut_ptr(),
    },
  }
}

fn create_certificate(subject: &str) -> Result<SigningCertificate> {
  let mut provider = NCRYPT_PROV_HANDLE::default();
  unsafe {
    at(
      "open provider",
      NCryptOpenStorageProvider(&mut provider, MS_KEY_STORAGE_PROVIDER, 0),
    )?
  };
  let provider = Provider(provider);
  let key_name = format!(
    "BatchRename-{:016x}{:016x}",
    fastrand::u64(..),
    fastrand::u64(..)
  );
  let key_name_w = HSTRING::from(&key_name);
  let mut key = NCRYPT_KEY_HANDLE::default();
  unsafe {
    at(
      "create key",
      NCryptCreatePersistedKey(
        provider.0,
        &mut key,
        BCRYPT_RSA_ALGORITHM,
        &key_name_w,
        CERT_KEY_SPEC(0),
        NCRYPT_FLAGS(0),
      ),
    )?;
  }
  let key = TemporaryKey(key);
  unsafe {
    at(
      "set key length",
      NCryptSetProperty(
        NCRYPT_HANDLE(key.0.0),
        NCRYPT_LENGTH_PROPERTY,
        &2048u32.to_le_bytes(),
        NCRYPT_SILENT_FLAG,
      ),
    )?;
    at("finalize key", NCryptFinalizeKey(key.0, NCRYPT_SILENT_FLAG))?;
  }

  let subject_w = HSTRING::from(subject);
  let mut name_len = 0u32;
  unsafe {
    CertStrToNameW(
      X509_ASN_ENCODING,
      &subject_w,
      CERT_X500_NAME_STR,
      None,
      None,
      &mut name_len,
      None,
    )?
  };
  let mut name = vec![0u8; name_len as usize];
  unsafe {
    CertStrToNameW(
      X509_ASN_ENCODING,
      &subject_w,
      CERT_X500_NAME_STR,
      None,
      Some(name.as_mut_ptr()),
      &mut name_len,
      None,
    )?;
  }
  let name_blob = CRYPT_INTEGER_BLOB {
    cbData: name_len,
    pbData: name.as_mut_ptr(),
  };

  // MSIX signing certificates need the code signing EKU and must not be CAs.
  let mut code_signing = PSTR(szOID_PKIX_KP_CODE_SIGNING.0 as *mut u8);
  let usage = CTL_USAGE {
    cUsageIdentifier: 1,
    rgpszUsageIdentifier: &mut code_signing,
  };
  let mut eku = encode(X509_ENHANCED_KEY_USAGE, addr_of!(usage).cast())?;
  let constraints = CERT_BASIC_CONSTRAINTS2_INFO {
    fCA: false.into(),
    fPathLenConstraint: false.into(),
    dwPathLenConstraint: 0,
  };
  let mut basic = encode(X509_BASIC_CONSTRAINTS2, addr_of!(constraints).cast())?;
  let mut usage_bits = CERT_DIGITAL_SIGNATURE_KEY_USAGE as u8;
  let key_usage_bits = CRYPT_BIT_BLOB {
    cbData: 1,
    pbData: &mut usage_bits,
    cUnusedBits: 0,
  };
  let mut key_usage = encode(X509_KEY_USAGE, addr_of!(key_usage_bits).cast())?;
  let mut extensions = [
    extension(szOID_ENHANCED_KEY_USAGE, false, &mut eku),
    extension(szOID_BASIC_CONSTRAINTS2, true, &mut basic),
    extension(szOID_KEY_USAGE, true, &mut key_usage),
  ];
  let extensions = CERT_EXTENSIONS {
    cExtension: extensions.len() as u32,
    rgExtension: extensions.as_mut_ptr(),
  };
  let algorithm = CRYPT_ALGORITHM_IDENTIFIER {
    pszObjId: PSTR(szOID_RSA_SHA256RSA.0 as *mut u8),
    Parameters: CRYPT_INTEGER_BLOB::default(),
  };
  let mut end = unsafe { GetSystemTime() };
  end.wYear += 20;
  if end.wMonth == 2 && end.wDay == 29 {
    end.wDay = 28;
  }

  // Records where the key lives, so the signer finds it by name. CRYPT_SILENT makes acquiring
  // it fail instead of prompting (e.g. for a smart card).
  let provider_name = HSTRING::from("Microsoft Software Key Storage Provider");
  let key_info = CRYPT_KEY_PROV_INFO {
    pwszContainerName: PWSTR(key_name_w.as_ptr() as *mut u16),
    pwszProvName: PWSTR(provider_name.as_ptr() as *mut u16),
    dwProvType: 0,
    dwFlags: CRYPT_KEY_FLAGS(CRYPT_SILENT),
    cProvParam: 0,
    rgProvParam: null_mut(),
    dwKeySpec: 0,
  };
  let context = unsafe {
    CertCreateSelfSignCertificate(
      Some(HCRYPTPROV_OR_NCRYPT_KEY_HANDLE(key.0.0)),
      &name_blob,
      CERT_CREATE_SELFSIGN_FLAGS(0),
      Some(&key_info),
      Some(&algorithm),
      None,
      Some(&end),
      Some(&extensions),
    )
  };
  if context.is_null() {
    return at("create certificate", Err(Error::from_thread()));
  }
  Ok(SigningCertificate {
    certificate: Certificate(context),
    _key: key,
    _provider: provider,
    key_name,
  })
}

fn sign(package: &Path, certificate: &Certificate) -> Result<()> {
  let path = HSTRING::from(package.as_os_str());
  let mut file_info = SIGNER_FILE_INFO {
    cbSize: size_of::<SIGNER_FILE_INFO>() as u32,
    pwszFileName: PCWSTR(path.as_ptr()),
    hFile: HANDLE::default(),
  };
  let mut index = 0u32;
  let subject_info = SIGNER_SUBJECT_INFO {
    cbSize: size_of::<SIGNER_SUBJECT_INFO>() as u32,
    pdwIndex: &mut index,
    dwSubjectChoice: SIGNER_SUBJECT_FILE,
    Anonymous: SIGNER_SUBJECT_INFO_0 {
      pSignerFileInfo: &mut file_info,
    },
  };
  let mut store_info = SIGNER_CERT_STORE_INFO {
    cbSize: size_of::<SIGNER_CERT_STORE_INFO>() as u32,
    pSigningCert: certificate.0,
    dwCertPolicy: SIGNER_CERT_POLICY_CHAIN,
    hCertStore: HCERTSTORE::default(),
  };
  let signer_cert = SIGNER_CERT {
    cbSize: size_of::<SIGNER_CERT>() as u32,
    dwCertChoice: SIGNER_CERT_STORE,
    Anonymous: SIGNER_CERT_0 {
      pCertStoreInfo: &mut store_info,
    },
    hwnd: HWND::default(),
  };
  // Must match the hash algorithm the package was created with (makeappx uses SHA-256).
  let signature_info = SIGNER_SIGNATURE_INFO {
    cbSize: size_of::<SIGNER_SIGNATURE_INFO>() as u32,
    algidHash: CALG_SHA_256,
    dwAttrChoice: SIGNER_NO_ATTR,
    ..Default::default()
  };
  let mut signer_context: *mut SIGNER_CONTEXT = null_mut();
  let mut params = SignerSignEx2Params {
    flags: 0,
    subject_info: &subject_info,
    signing_cert: &signer_cert,
    signature_info: &signature_info,
    provider_info: null(),
    timestamp_flags: 0,
    algorithm_oid: PCSTR::null(),
    timestamp_url: PCWSTR::null(),
    crypt_attrs: null(),
    sip_data: null_mut(),
    signer_context: &mut signer_context,
    crypto_policy: null(),
    reserved: null(),
  };
  let mut sip_data = AppxSipClientData {
    signer_params: addr_of!(params),
    appx_sip_state: null_mut(),
  };
  params.sip_data = (&raw mut sip_data).cast();

  let result = unsafe {
    SignerSignEx2(
      SIGNER_SIGN_FLAGS(params.flags),
      params.subject_info,
      params.signing_cert,
      params.signature_info,
      None,
      None,
      params.algorithm_oid,
      params.timestamp_url,
      None,
      Some(params.sip_data),
      params.signer_context,
      None,
      None,
    )
  };
  unsafe {
    if !signer_context.is_null() {
      let _ = SignerFreeSignerContext(signer_context);
    }
    if !sip_data.appx_sip_state.is_null() {
      drop(IUnknown::from_raw(sip_data.appx_sip_state));
    }
  }
  at("sign package", result)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn creates_code_signing_certificate_and_deletes_its_key() {
    let signing = create_certificate("CN=Batch Rename Test").unwrap();
    let key_name = HSTRING::from(&signing.key_name);
    let certificate = &signing.certificate;
    let der = certificate.der();
    assert!(der.len() > 500);
    let mut buf = [0u16; 256];
    let len = unsafe {
      CertNameToStrW(
        X509_ASN_ENCODING,
        &(*(*certificate.0).pCertInfo).Subject,
        CERT_X500_NAME_STR,
        Some(&mut buf),
      )
    };
    assert_eq!(
      String::from_utf16_lossy(&buf[..len as usize - 1]),
      "CN=Batch Rename Test"
    );
    drop(signing);

    let mut provider = NCRYPT_PROV_HANDLE::default();
    unsafe { NCryptOpenStorageProvider(&mut provider, MS_KEY_STORAGE_PROVIDER, 0).unwrap() };
    let provider = Provider(provider);
    let mut key = NCRYPT_KEY_HANDLE::default();
    let reopened = unsafe {
      NCryptOpenKey(
        provider.0,
        &mut key,
        &key_name,
        CERT_KEY_SPEC(0),
        NCRYPT_SILENT_FLAG,
      )
    };
    assert!(reopened.is_err(), "the private key must be deleted");
  }
}
