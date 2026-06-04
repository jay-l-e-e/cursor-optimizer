use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::actions::{InstallRequest, InstallerMode, UninstallRequest};
use crate::common;
use crate::plan;

const INSTALL_ROOT: &str = "/opt/CursorOptimizer";
const BINARY_SYMLINK: &str = "/usr/local/bin/cursor-optimizer";
const APPLICATIONS_DIRECTORY: &str = "/usr/share/applications";
const DESKTOP_IDENTIFIER: &str = "cursor-optimizer";
const ICON_FILE_NAME: &str = "icon.png";
const VERSION_FILE_NAME: &str = "version";

pub fn default_install_directory() -> PathBuf {
  PathBuf::from(INSTALL_ROOT)
}

pub fn installed_directory(mode: InstallerMode) -> Option<PathBuf> {
  match mode {
    InstallerMode::Uninstall => current_install_directory(),
    InstallerMode::Install => {
      let candidate = PathBuf::from(INSTALL_ROOT);
      if candidate.is_dir() {
        Some(candidate)
      } else {
        None
      }
    }
  }
}

fn current_install_directory() -> Option<PathBuf> {
  std::env::current_exe()
    .ok()
    .and_then(|path| path.parent().map(Path::to_path_buf))
}

pub fn installed_version(mode: InstallerMode) -> Option<String> {
  let directory = installed_directory(mode)?;
  let text = std::fs::read_to_string(directory.join(VERSION_FILE_NAME)).ok()?;
  let trimmed = text.trim();
  if trimmed.is_empty() {
    None
  } else {
    Some(trimmed.to_string())
  }
}

fn home_desktop_directory() -> Option<PathBuf> {
  std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Desktop"))
}

fn desktop_entry_contents(executable: &Path, icon: &Path) -> String {
  format!(
    "[Desktop Entry]\nType=Application\nName={}\nComment=Inspect and optimize the Cursor editor database\nExec=\"{}\"\nIcon={}\nTerminal=false\nCategories=Utility;\n",
    common::PRODUCT_NAME,
    executable.display(),
    icon.display()
  )
}

pub fn perform_install(request: &InstallRequest, report: &dyn Fn(&str)) -> Result<PathBuf, String> {
  report("Preparing the installation folder");
  std::fs::create_dir_all(&request.install_directory)
    .map_err(|error| format!("Could not create the installation folder: {error}"))?;

  report("Copying application files");
  let executable = plan::file_in(
    &request.install_directory,
    common::application_executable_file_name(),
  );
  common::write_application_binary(&executable)?;

  let icon = plan::file_in(&request.install_directory, ICON_FILE_NAME);
  std::fs::write(&icon, common::APPLICATION_ICON_PNG)
    .map_err(|error| format!("Could not write the application icon: {error}"))?;

  std::fs::write(
    plan::file_in(&request.install_directory, VERSION_FILE_NAME),
    common::VERSION,
  )
  .map_err(|error| format!("Could not write the version marker: {error}"))?;

  let uninstaller = plan::file_in(&request.install_directory, common::uninstaller_file_name());
  common::copy_self_as_uninstaller(&uninstaller)?;

  report("Linking the command-line launcher");
  let link = PathBuf::from(BINARY_SYMLINK);
  let _ = std::fs::remove_file(&link);
  if let Some(parent) = link.parent() {
    let _ = std::fs::create_dir_all(parent);
  }
  symlink(&executable, &link)
    .map_err(|error| format!("Could not create the launcher link: {error}"))?;

  if request.create_start_menu_shortcut {
    report("Creating the application menu entry");
    let applications = PathBuf::from(APPLICATIONS_DIRECTORY);
    std::fs::create_dir_all(&applications)
      .map_err(|error| format!("Could not create the applications folder: {error}"))?;
    let entry = plan::desktop_entry_in(&applications, DESKTOP_IDENTIFIER);
    std::fs::write(&entry, desktop_entry_contents(&executable, &icon))
      .map_err(|error| format!("Could not create the menu entry: {error}"))?;
  }

  if request.create_desktop_shortcut {
    report("Creating the desktop shortcut");
    if let Some(directory) = home_desktop_directory() {
      let _ = std::fs::create_dir_all(&directory);
      let entry = plan::desktop_entry_in(&directory, DESKTOP_IDENTIFIER);
      std::fs::write(&entry, desktop_entry_contents(&executable, &icon))
        .map_err(|error| format!("Could not create the desktop shortcut: {error}"))?;
      common::mark_executable(&entry)?;
    }
  }

  report("Installation complete");
  Ok(executable)
}

pub fn perform_uninstall(request: &UninstallRequest, report: &dyn Fn(&str)) -> Result<(), String> {
  report("Removing shortcuts");
  let _ = std::fs::remove_file(BINARY_SYMLINK);
  let _ = std::fs::remove_file(plan::desktop_entry_in(
    &PathBuf::from(APPLICATIONS_DIRECTORY),
    DESKTOP_IDENTIFIER,
  ));
  if let Some(directory) = home_desktop_directory() {
    let _ = std::fs::remove_file(plan::desktop_entry_in(&directory, DESKTOP_IDENTIFIER));
  }

  if request.remove_application_data {
    report("Removing the application data folder");
    common::remove_application_data()?;
  }

  report("Removing application files");
  if let Some(directory) = current_install_directory() {
    std::fs::remove_dir_all(&directory)
      .map_err(|error| format!("Could not remove the installation folder: {error}"))?;
  }

  report("Uninstall complete");
  Ok(())
}

pub fn launch_executable(path: &Path) -> Result<(), String> {
  Command::new(path)
    .spawn()
    .map(|_| ())
    .map_err(|error| format!("Could not start the application: {error}"))
}
