// Prevents an additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
  // The app starts itself elevated for a few setup tasks; those exit without opening a window.
  if let Some(code) = batch_rename_lib::run_cli_task() {
    std::process::exit(code);
  }
  batch_rename_lib::run()
}
