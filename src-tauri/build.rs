use std::{
  env, fs,
  path::{Path, PathBuf},
  process::Command,
};

fn main() {
  if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
    build_context_menu();
  }
  tauri_build::build()
}

/// Builds the Windows 11 context menu handler DLL and its sparse package, and hands both to the
/// app through `CONTEXT_MENU_DLL` / `CONTEXT_MENU_MSIX` so they can be embedded with
/// `include_bytes!`. This keeps the portable app a single executable.
fn build_context_menu() {
  let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
  let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
  let crate_dir = manifest_dir.join("context-menu");
  for path in [
    "context-menu/src",
    "context-menu/Cargo.toml",
    "context-menu/build.rs",
    "context-menu/package",
    "icons",
    "Cargo.lock",
  ] {
    println!("cargo:rerun-if-changed={path}");
  }

  // The DLL is always built optimized, in its own target folder so the nested build doesn't wait
  // on the lock of the build that is running this script.
  let target = env::var("TARGET").unwrap();
  let target_dir = out_dir
    .ancestors()
    .find(|p| p.join("CACHEDIR.TAG").is_file())
    .map_or_else(|| manifest_dir.join("target"), Path::to_path_buf)
    .join("context-menu");
  let status = Command::new(env::var("CARGO").unwrap())
    .args(["build", "--release", "--target", &target, "--manifest-path"])
    .arg(crate_dir.join("Cargo.toml"))
    .arg("--target-dir")
    .arg(&target_dir)
    // Don't run clippy or similar wrappers on the nested build.
    .env_remove("RUSTC_WORKSPACE_WRAPPER")
    .status()
    .expect("failed to run cargo for the context menu DLL");
  assert!(status.success(), "building the context menu DLL failed");
  let dll = target_dir
    .join(&target)
    .join("release")
    .join("batch_rename_context_menu.dll");
  println!("cargo:rustc-env=CONTEXT_MENU_DLL={}", dll.display());

  let staging = out_dir.join("context-menu-package");
  let _ = fs::remove_dir_all(&staging);
  fs::create_dir_all(staging.join("Assets")).unwrap();
  fs::copy(
    crate_dir.join("package").join("AppxManifest.xml"),
    staging.join("AppxManifest.xml"),
  )
  .unwrap();
  for logo in [
    "StoreLogo.png",
    "Square150x150Logo.png",
    "Square44x44Logo.png",
  ] {
    fs::copy(
      manifest_dir.join("icons").join(logo),
      staging.join("Assets").join(logo),
    )
    .unwrap();
  }
  let msix = out_dir.join("context-menu.msix");
  // /nv: the manifest refers to files in the external location, which aren't in the package.
  let output = Command::new(makeappx())
    .args(["pack", "/o", "/nv", "/d"])
    .arg(&staging)
    .arg("/p")
    .arg(&msix)
    .output()
    .expect("failed to run makeappx");
  assert!(
    output.status.success(),
    "makeappx failed:\n{}",
    String::from_utf8_lossy(&output.stdout)
  );
  println!("cargo:rustc-env=CONTEXT_MENU_MSIX={}", msix.display());
}

/// `makeappx.exe` from the Windows SDK (installed with the MSVC build tools). Set `MAKEAPPX` to
/// use a specific copy.
fn makeappx() -> PathBuf {
  println!("cargo:rerun-if-env-changed=MAKEAPPX");
  if let Some(path) = env::var_os("MAKEAPPX") {
    return path.into();
  }
  let arch = match env::consts::ARCH {
    "aarch64" => "arm64",
    "x86" => "x86",
    _ => "x64",
  };
  let kits = PathBuf::from(
    env::var("ProgramFiles(x86)").unwrap_or_else(|_| r"C:\Program Files (x86)".into()),
  )
  .join(r"Windows Kits\10\bin");
  let mut versions: Vec<PathBuf> = fs::read_dir(&kits)
    .map(|entries| {
      entries
        .filter_map(Result::ok)
        .map(|e| e.path().join(arch).join("makeappx.exe"))
        .collect()
    })
    .unwrap_or_default();
  versions.retain(|p| p.is_file());
  versions.sort();
  versions.pop().unwrap_or_else(|| {
    panic!(
      "makeappx.exe wasn't found in {}. Install the Windows SDK or set MAKEAPPX.",
      kits.display()
    )
  })
}
