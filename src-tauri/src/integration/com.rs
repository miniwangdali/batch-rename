use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

/// Initializes COM on the current thread for as long as the guard lives. If the thread already
/// uses COM in another apartment (e.g. the UI thread), that apartment is used as is.
pub struct Com(bool);

impl Com {
  pub fn init() -> Com {
    Com(unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok())
  }
}

impl Drop for Com {
  fn drop(&mut self) {
    if self.0 {
      unsafe { CoUninitialize() };
    }
  }
}
