//! Starting the app with the selected items.

use std::{
  ffi::OsString,
  fs, io,
  path::Path,
  process::{Command, Stdio},
  time::{SystemTime, UNIX_EPOCH},
};

use windows::Win32::UI::WindowsAndMessaging::{ASFW_ANY, AllowSetForegroundWindow};

/// Windows command lines are limited to 32,767 characters; longer selections are passed
/// through a list file instead. The app only accepts list files named like the ones below
/// in the temp folder, and deletes them after reading.
pub const LIST_THRESHOLD: usize = 30_000;

/// Returns the arguments to start the app with, writing a list file if they're too long.
pub fn command_for(exe: &Path, paths: &[String]) -> io::Result<Vec<OsString>> {
  // UTF-8 length never undercounts the UTF-16 length Windows limits.
  let length = exe.as_os_str().len() + paths.iter().map(|p| p.len() + 3).sum::<usize>();
  if length < LIST_THRESHOLD {
    return Ok(paths.iter().map(OsString::from).collect());
  }
  let nanos = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map_or(0, |d| d.as_nanos());
  let file =
    std::env::temp_dir().join(format!("batch-rename-{}-{nanos:x}.txt", std::process::id()));
  fs::write(&file, paths.join("\n"))?;
  Ok(vec!["--paths-from".into(), file.into()])
}

pub fn launch(exe: &Path, paths: &[String]) -> io::Result<()> {
  let args = command_for(exe, paths)?;
  // Explorer owns the foreground; let the app's window come to the front.
  unsafe {
    let _ = AllowSetForegroundWindow(ASFW_ANY);
  }
  Command::new(exe)
    .args(args)
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn()
    .map(drop)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::PathBuf;

  fn list_file(args: &[OsString]) -> Option<PathBuf> {
    match args {
      [flag, file] if flag == "--paths-from" => Some(PathBuf::from(file)),
      _ => None,
    }
  }

  #[test]
  fn short_selections_are_passed_as_arguments() {
    let paths = vec!["C:\\a b\\x.txt".to_string(), "D:\\".to_string()];
    let args = command_for(Path::new("C:\\app.exe"), &paths).unwrap();
    assert_eq!(
      args,
      vec![OsString::from("C:\\a b\\x.txt"), OsString::from("D:\\")]
    );
    assert!(list_file(&args).is_none());
  }

  #[test]
  fn long_selections_use_a_list_file() {
    let paths: Vec<String> = (0..2000)
      .map(|i| format!("C:\\photos\\holiday\\IMG_{i:05}.jpg"))
      .collect();
    let args = command_for(Path::new("C:\\app.exe"), &paths).unwrap();
    let file = list_file(&args).expect("list file");
    assert!(file.starts_with(std::env::temp_dir()));
    let name = file.file_name().unwrap().to_string_lossy().into_owned();
    assert!(name.starts_with("batch-rename-") && name.ends_with(".txt"));
    let text = fs::read_to_string(&file).unwrap();
    fs::remove_file(&file).unwrap();
    assert_eq!(text.lines().count(), 2000);
    assert_eq!(
      text.lines().next(),
      Some("C:\\photos\\holiday\\IMG_00000.jpg")
    );
  }
}
