fn main() {
  // COM entry points are found with GetProcAddress, so MSVC's advice to mark them PRIVATE
  // (to keep them out of the import library) doesn't matter here.
  if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
    println!("cargo:rustc-cdylib-link-arg=/IGNORE:4104");
  }
}
