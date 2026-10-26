mod engine;
mod fsops;
mod integration;
mod naming;
mod session;

use std::{
  collections::HashSet,
  fs,
  path::{Path, PathBuf},
  sync::{Mutex, MutexGuard},
};

use serde::Serialize;
use tauri::{AppHandle, Manager, State, WebviewWindow};

use engine::Options;
use session::{AddReport, ItemDto, Preview, RenameReport, Session};

pub use integration::run_cli_task;

#[derive(Default)]
struct AppState {
  session: Mutex<Session>,
  /// Paths forwarded by later launches (single-instance plugin) that the UI hasn't picked up yet.
  pending: Mutex<Vec<PathBuf>>,
  /// This launch's arguments and working directory. They're parsed only when the UI asks for
  /// them: if another instance is running, this process just forwards them and exits, and
  /// parsing would consume (delete) a `--paths-from` list file meant for that instance.
  startup_args: Mutex<Option<(Vec<String>, PathBuf)>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
  m.lock().unwrap_or_else(|e| e.into_inner())
}

const LIST_FLAG: &str = "--paths-from";

fn paths_from_args(args: impl IntoIterator<Item = String>, cwd: &Path) -> Vec<PathBuf> {
  let mut paths = Vec::new();
  let mut args = args.into_iter();
  while let Some(arg) = args.next() {
    if arg == LIST_FLAG {
      paths.extend(
        args
          .next()
          .map(|file| read_list_file(Path::new(&file)))
          .unwrap_or_default(),
      );
    } else if !arg.is_empty() && !arg.starts_with("--") {
      paths.push(cwd.join(arg));
    }
  }
  paths
}

/// Reads and deletes a path list written by the context menu handler for large selections.
/// Because the file gets deleted, only the handler's own files in the temp folder are accepted.
fn read_list_file(file: &Path) -> Vec<PathBuf> {
  let named_ok = file
    .file_name()
    .and_then(|n| n.to_str())
    .is_some_and(|n| n.starts_with("batch-rename-") && n.ends_with(".txt"));
  let canonical_parent = file.parent().and_then(|p| p.canonicalize().ok());
  let in_temp =
    canonical_parent.is_some_and(|p| std::env::temp_dir().canonicalize().is_ok_and(|t| t == p));
  if !named_ok || !in_temp {
    return Vec::new();
  }
  let text = fs::read_to_string(file).unwrap_or_default();
  let _ = fs::remove_file(file);
  text
    .lines()
    .map(str::trim)
    .filter(|l| !l.is_empty())
    .map(PathBuf::from)
    .collect()
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
async fn add_paths(
  state: State<'_, AppState>,
  paths: Vec<PathBuf>,
) -> Result<ItemsResponse, String> {
  let mut session = lock(&state.session);
  let report = session.add_paths(&paths);
  Ok(ItemsResponse {
    items: session.items(),
    report,
  })
}

#[tauri::command]
async fn add_pending(state: State<'_, AppState>) -> Result<ItemsResponse, String> {
  let mut paths = match lock(&state.startup_args).take() {
    Some((args, cwd)) => paths_from_args(args, &cwd),
    None => Vec::new(),
  };
  paths.append(&mut lock(&state.pending));
  let mut session = lock(&state.session);
  let report = session.add_paths(&paths);
  Ok(ItemsResponse {
    items: session.items(),
    report,
  })
}

#[tauri::command]
async fn clear_items(state: State<'_, AppState>) -> Result<(), String> {
  lock(&state.session).clear();
  Ok(())
}

#[tauri::command]
async fn preview(
  state: State<'_, AppState>,
  options: Options,
  deselected: Vec<u32>,
) -> Result<Preview, String> {
  let deselected: HashSet<u32> = deselected.into_iter().collect();
  lock(&state.session).preview(&options, &deselected)
}

#[tauri::command]
async fn rename(state: State<'_, AppState>, version: u64) -> Result<RenameResponse, String> {
  let mut session = lock(&state.session);
  let report = session.rename(version)?;
  Ok(RenameResponse {
    items: session.items(),
    report,
  })
}

#[tauri::command]
async fn undo(state: State<'_, AppState>) -> Result<RenameResponse, String> {
  let mut session = lock(&state.session);
  let report = session.undo()?;
  Ok(RenameResponse {
    items: session.items(),
    report,
  })
}

// Integration commands are async so they run off the UI thread: they may wait on the
// elevation prompt or on package deployment.

#[tauri::command]
async fn integration_status() -> Result<integration::Status, String> {
  Ok(integration::status())
}

fn current_exe() -> Result<PathBuf, String> {
  std::env::current_exe().map_err(|e| e.to_string())
}

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
  app.path().app_local_data_dir().map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_start_menu(enabled: bool) -> Result<(), String> {
  integration::set_start_menu(&current_exe()?, enabled)
}

#[tauri::command]
async fn set_context_menu(
  app: AppHandle,
  window: WebviewWindow,
  enabled: bool,
  label: String,
) -> Result<(), String> {
  #[cfg(windows)]
  let owner = window.hwnd().map_or(0, |h| h.0 as isize);
  #[cfg(not(windows))]
  let owner = {
    let _ = window;
    0
  };
  let (exe, data_dir) = (current_exe()?, data_dir(&app)?);
  integration::set_context_menu(
    &integration::Context {
      exe: &exe,
      data_dir: &data_dir,
      label: &label,
      window: owner,
    },
    enabled,
  )
}

#[tauri::command]
async fn sync_integration(app: AppHandle, label: String) -> Result<(), String> {
  let (exe, data_dir) = (current_exe()?, data_dir(&app)?);
  integration::sync(&integration::Context {
    exe: &exe,
    data_dir: &data_dir,
    label: &label,
    window: 0,
  })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let cwd = std::env::current_dir().unwrap_or_default();
  let state = AppState {
    startup_args: Mutex::new(Some((std::env::args().skip(1).collect(), cwd))),
    ..Default::default()
  };

  let mut builder = tauri::Builder::default();
  #[cfg(desktop)]
  {
    use tauri::Emitter;
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
      integration_status,
      set_start_menu,
      set_context_menu,
      sync_integration
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn command_line_paths() {
    let cwd = Path::new("C:\\work");
    let args = ["a.txt", "--flag", "", "D:\\b"].map(String::from);
    assert_eq!(
      paths_from_args(args, cwd),
      vec![cwd.join("a.txt"), PathBuf::from("D:\\b")]
    );
  }

  #[test]
  fn list_files_are_read_and_deleted() {
    let temp = std::env::temp_dir();
    let list = temp.join(format!("batch-rename-test-{}.txt", std::process::id()));
    fs::write(&list, "C:\\one.txt\n\nC:\\two words.txt\n").unwrap();
    let args = [
      LIST_FLAG.to_string(),
      list.to_string_lossy().into_owned(),
      "x".into(),
    ];
    let paths = paths_from_args(args, Path::new("C:\\cwd"));
    assert_eq!(
      paths,
      vec![
        PathBuf::from("C:\\one.txt"),
        PathBuf::from("C:\\two words.txt"),
        PathBuf::from("C:\\cwd\\x")
      ]
    );
    assert!(!list.exists());
  }

  #[test]
  fn other_list_files_are_ignored() {
    let t = tempfile::tempdir().unwrap();
    let outside = t.path().join("batch-rename-1.txt");
    fs::write(&outside, "C:\\x").unwrap();
    assert!(read_list_file(&outside).is_empty());
    assert!(
      outside.exists(),
      "files outside the temp folder must not be deleted"
    );

    let wrong_name = std::env::temp_dir().join(format!("notes-{}.txt", std::process::id()));
    fs::write(&wrong_name, "C:\\x").unwrap();
    assert!(read_list_file(&wrong_name).is_empty());
    assert!(wrong_name.exists());
    fs::remove_file(wrong_name).unwrap();
  }
}
