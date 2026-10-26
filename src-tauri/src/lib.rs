mod engine;
mod fsops;
mod naming;
mod session;
#[cfg(windows)]
mod shell;

use std::{
  collections::HashSet,
  path::{Path, PathBuf},
  sync::{Mutex, MutexGuard},
};

use serde::Serialize;
use tauri::State;

use engine::Options;
use session::{AddReport, ItemDto, Preview, RenameReport, Session};

#[derive(Default)]
struct AppState {
  session: Mutex<Session>,
  /// Paths from the command line (first launch or later launches forwarded by the
  /// single-instance plugin) that the UI hasn't picked up yet.
  pending: Mutex<Vec<PathBuf>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
  m.lock().unwrap_or_else(|e| e.into_inner())
}

fn paths_from_args(args: impl IntoIterator<Item = String>, cwd: &Path) -> Vec<PathBuf> {
  args.into_iter().filter(|a| !a.is_empty() && !a.starts_with("--")).map(|a| cwd.join(a)).collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ItemsResponse {
  items: Vec<ItemDto>,
  report: AddReport,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RenameResponse {
  items: Vec<ItemDto>,
  report: RenameReport,
}

// Commands that touch the file system are async so they run off the main (UI) thread.

#[tauri::command]
async fn add_paths(state: State<'_, AppState>, paths: Vec<PathBuf>) -> Result<ItemsResponse, String> {
  let mut session = lock(&state.session);
  let report = session.add_paths(&paths);
  Ok(ItemsResponse { items: session.items(), report })
}

#[tauri::command]
async fn add_pending(state: State<'_, AppState>) -> Result<ItemsResponse, String> {
  let paths = std::mem::take(&mut *lock(&state.pending));
  let mut session = lock(&state.session);
  let report = session.add_paths(&paths);
  Ok(ItemsResponse { items: session.items(), report })
}

#[tauri::command]
async fn clear_items(state: State<'_, AppState>) -> Result<(), String> {
  lock(&state.session).clear();
  Ok(())
}

#[tauri::command]
async fn preview(state: State<'_, AppState>, options: Options, deselected: Vec<u32>) -> Result<Preview, String> {
  let deselected: HashSet<u32> = deselected.into_iter().collect();
  lock(&state.session).preview(&options, &deselected)
}

#[tauri::command]
async fn rename(state: State<'_, AppState>, version: u64) -> Result<RenameResponse, String> {
  let mut session = lock(&state.session);
  let report = session.rename(version)?;
  Ok(RenameResponse { items: session.items(), report })
}

#[tauri::command]
async fn undo(state: State<'_, AppState>) -> Result<RenameResponse, String> {
  let mut session = lock(&state.session);
  let report = session.undo()?;
  Ok(RenameResponse { items: session.items(), report })
}

/// `None` when Explorer integration isn't available on this platform.
#[tauri::command]
fn shell_integration() -> Option<bool> {
  #[cfg(windows)]
  return Some(shell::is_enabled());
  #[cfg(not(windows))]
  None
}

#[tauri::command]
fn set_shell_integration(enabled: bool, label: String) -> Result<(), String> {
  #[cfg(windows)]
  return shell::set_enabled(enabled, &label).map_err(|e| e.to_string());
  #[cfg(not(windows))]
  {
    let _ = (enabled, label);
    Err("Not supported on this platform".into())
  }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let cwd = std::env::current_dir().unwrap_or_default();
  let state = AppState { pending: Mutex::new(paths_from_args(std::env::args().skip(1), &cwd)), ..Default::default() };

  let mut builder = tauri::Builder::default();
  #[cfg(desktop)]
  {
    use tauri::{Emitter, Manager};
    builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
      let paths = paths_from_args(argv.into_iter().skip(1), Path::new(&cwd));
      if !paths.is_empty() {
        lock(&app.state::<AppState>().pending).extend(paths);
        let _ = app.emit("pending-paths", ());
      }
      if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.set_focus();
      }
    }));
  }
  builder
    .plugin(tauri_plugin_dialog::init())
    .manage(state)
    .invoke_handler(tauri::generate_handler![
      add_paths,
      add_pending,
      clear_items,
      preview,
      rename,
      undo,
      shell_integration,
      set_shell_integration
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
