use std::fs;
use std::path::Path;

pub const PRODUCT_NAME: &str = "Cursor Optimizer";
#[cfg(target_os = "windows")]
pub const INSTALL_DIRECTORY_NAME: &str = "CursorOptimizer";
#[cfg(target_os = "windows")]
pub const PUBLISHER: &str = "Jay Lee";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APPLICATION_ICON_PNG: &[u8] = include_bytes!("../../../assets/icon.png");

pub fn application_executable_file_name() -> &'static str {
  if cfg!(target_os = "windows") {
    "cursor-optimizer.exe"
  } else {
    "cursor-optimizer"
  }
}

pub fn uninstaller_file_name() -> &'static str {
  if cfg!(target_os = "windows") {
    "uninstall.exe"
  } else {
    "uninstall"
  }
}

pub fn has_embedded_binary() -> bool {
  !crate::EMBEDDED_APPLICATION_BINARY.is_empty()
}

pub fn write_application_binary(destination: &Path) -> Result<(), String> {
  if !has_embedded_binary() {
    return Err("The installer package is incomplete.".to_string());
  }
  fs::write(destination, crate::EMBEDDED_APPLICATION_BINARY)
    .map_err(|error| format!("Could not write the application: {error}"))?;
  mark_executable(destination)
}

pub fn copy_self_as_uninstaller(destination: &Path) -> Result<(), String> {
  let current =
    std::env::current_exe().map_err(|error| format!("Could not locate the installer: {error}"))?;
  if current.as_path() == destination {
    return Ok(());
  }
  fs::copy(&current, destination)
    .map_err(|error| format!("Could not write the uninstaller: {error}"))?;
  mark_executable(destination)
}

pub fn remove_application_data() -> Result<(), String> {
  let directory = application_paths::application_data_directory();
  if directory.is_dir() {
    fs::remove_dir_all(&directory)
      .map_err(|error| format!("Could not remove the application data folder: {error}"))?;
  }
  Ok(())
}

#[cfg(unix)]
pub fn mark_executable(path: &Path) -> Result<(), String> {
  use std::os::unix::fs::PermissionsExt;
  let metadata =
    fs::metadata(path).map_err(|error| format!("Could not read file permissions: {error}"))?;
  let mut permissions = metadata.permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(path, permissions)
    .map_err(|error| format!("Could not set file permissions: {error}"))
}

#[cfg(not(unix))]
pub fn mark_executable(_path: &Path) -> Result<(), String> {
  Ok(())
}
