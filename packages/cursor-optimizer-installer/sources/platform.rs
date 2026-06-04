#[cfg(target_os = "windows")]
pub use crate::windows::{
  default_install_directory, installed_directory, installed_version, launch_executable,
  perform_install, perform_uninstall,
};

#[cfg(target_os = "macos")]
pub use crate::macos::{
  default_install_directory, installed_directory, installed_version, launch_executable,
  perform_install, perform_uninstall,
};

#[cfg(all(unix, not(target_os = "macos")))]
pub use crate::linux::{
  default_install_directory, installed_directory, installed_version, launch_executable,
  perform_install, perform_uninstall,
};
