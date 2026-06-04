use std::path::{Path, PathBuf};

pub fn file_in(directory: &Path, file_name: &str) -> PathBuf {
  directory.join(file_name)
}

#[cfg(target_os = "windows")]
pub fn windows_shortcut_in(directory: &Path, product_name: &str) -> PathBuf {
  directory.join(format!("{product_name}.lnk"))
}

#[cfg(target_os = "windows")]
pub fn uninstall_command(uninstaller: &Path) -> String {
  format!("\"{}\" --uninstall", uninstaller.display())
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn desktop_entry_in(directory: &Path, identifier: &str) -> PathBuf {
  directory.join(format!("{identifier}.desktop"))
}

#[cfg(test)]
mod tests {
  use std::path::{Path, PathBuf};

  use super::file_in;

  #[test]
  fn joins_file_name() {
    assert_eq!(
      file_in(Path::new("/opt/CursorOptimizer"), "cursor-optimizer"),
      PathBuf::from("/opt/CursorOptimizer/cursor-optimizer")
    );
  }

  #[cfg(target_os = "windows")]
  #[test]
  fn builds_windows_shortcut_name() {
    use super::windows_shortcut_in;
    assert_eq!(
      windows_shortcut_in(Path::new("/start"), "Cursor Optimizer"),
      PathBuf::from("/start/Cursor Optimizer.lnk")
    );
  }

  #[cfg(target_os = "windows")]
  #[test]
  fn quotes_uninstall_command() {
    use super::uninstall_command;
    assert_eq!(
      uninstall_command(Path::new("/opt/CursorOptimizer/uninstall")),
      "\"/opt/CursorOptimizer/uninstall\" --uninstall"
    );
  }

  #[cfg(all(unix, not(target_os = "macos")))]
  #[test]
  fn builds_desktop_entry_name() {
    use super::desktop_entry_in;
    assert_eq!(
      desktop_entry_in(Path::new("/applications"), "cursor-optimizer"),
      PathBuf::from("/applications/cursor-optimizer.desktop")
    );
  }
}
