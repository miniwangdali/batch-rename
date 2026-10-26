//! Desktop integration: a Start menu shortcut and the File Explorer context menu entry.

#[cfg(windows)]
mod classic;
#[cfg(windows)]
mod com;
#[cfg(windows)]
mod signing;
#[cfg(windows)]
mod start_menu;
#[cfg(windows)]
mod top_level;
#[cfg(windows)]
mod trust;

use std::path::Path;

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
  /// `None` where the platform doesn't support it.
  pub start_menu: Option<bool>,
  pub context_menu: Option<bool>,
  /// Whether turning the context menu on asks for administrator permission (Windows 11).
  pub context_menu_needs_admin: bool,
}

/// Error returned when the user declined the elevation prompt.
pub const CANCELLED: &str = "cancelled";

pub struct Context<'a> {
  pub exe: &'a Path,
  /// The app's local data folder.
  pub data_dir: &'a Path,
  /// Localized menu title.
  pub label: &'a str,
  /// Owner window for the elevation prompt.
  pub window: isize,
}

/// The context menu entry goes at the top of the Windows 11 menu; Windows 10 only has the
/// classic menu.
#[cfg(windows)]
pub fn status() -> Status {
  let top_level = top_level::is_available();
  Status {
    start_menu: Some(start_menu::is_enabled()),
    context_menu: Some(if top_level {
      top_level::is_enabled()
    } else {
      classic::is_enabled()
    }),
    context_menu_needs_admin: top_level,
  }
}

#[cfg(windows)]
pub fn set_start_menu(exe: &Path, enabled: bool) -> Result<(), String> {
  if enabled {
    start_menu::enable(exe).map_err(|e| e.message())
  } else {
    start_menu::disable().map_err(|e| e.to_string())
  }
}

#[cfg(windows)]
pub fn set_context_menu(ctx: &Context, enabled: bool) -> Result<(), String> {
  let dir = top_level::files_dir(ctx.data_dir);
  if !enabled {
    top_level::disable(&dir)?;
    return classic::disable().map_err(|e| e.to_string());
  }
  if !top_level::is_available() {
    return classic::enable(ctx.exe, ctx.label).map_err(|e| e.to_string());
  }
  let owner = windows::Win32::Foundation::HWND(ctx.window as *mut _);
  top_level::enable(&dir, ctx.exe, ctx.label, owner).map_err(|e| match e {
    top_level::EnableError::Cancelled => CANCELLED.to_string(),
    top_level::EnableError::Failed(message) => message,
  })?;
  // Earlier versions could add the classic entry on Windows 11 too ("Show more options").
  classic::disable().map_err(|e| e.to_string())
}

/// Points enabled integrations at the current executable and label, e.g. after the portable app
/// was moved or the language changed. Never asks for elevation.
#[cfg(windows)]
pub fn sync(ctx: &Context) -> Result<(), String> {
  if classic::is_enabled() {
    classic::enable(ctx.exe, ctx.label).map_err(|e| e.to_string())?;
  }
  top_level::sync(&top_level::files_dir(ctx.data_dir), ctx.exe, ctx.label)
    .map_err(|e| e.to_string())?;
  start_menu::sync(ctx.exe).map_err(|e| e.message())
}

/// Runs a task the app was started for instead of showing its window (the elevated certificate
/// helper). Returns the process exit code.
#[cfg(windows)]
pub fn run_cli_task() -> Option<i32> {
  trust::run_elevated()
}

#[cfg(not(windows))]
pub fn status() -> Status {
  Status {
    start_menu: None,
    context_menu: None,
    context_menu_needs_admin: false,
  }
}

#[cfg(not(windows))]
pub fn set_start_menu(_exe: &Path, _enabled: bool) -> Result<(), String> {
  Err("Not supported on this platform.".into())
}

#[cfg(not(windows))]
pub fn set_context_menu(_ctx: &Context, _enabled: bool) -> Result<(), String> {
  Err("Not supported on this platform.".into())
}

#[cfg(not(windows))]
pub fn sync(_ctx: &Context) -> Result<(), String> {
  Ok(())
}

#[cfg(not(windows))]
pub fn run_cli_task() -> Option<i32> {
  None
}
