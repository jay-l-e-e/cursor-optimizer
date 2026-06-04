use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::actions::{InstallRequest, InstallerMode, UninstallRequest};
use crate::common;
use crate::plan;

const APPLICATIONS_DIRECTORY: &str = "/Applications";
const BUNDLE_NAME: &str = "Cursor Optimizer.app";
const BUNDLE_IDENTIFIER: &str = "kr.co.vendit.cursor-optimizer";

pub fn default_install_directory() -> PathBuf {
  PathBuf::from(APPLICATIONS_DIRECTORY).join(BUNDLE_NAME)
}

pub fn installed_directory(mode: InstallerMode) -> Option<PathBuf> {
  match mode {
    InstallerMode::Uninstall => current_bundle(),
    InstallerMode::Install => {
      let candidate = default_install_directory();
      if candidate.is_dir() {
        Some(candidate)
      } else {
        None
      }
    }
  }
}

pub fn installed_version(mode: InstallerMode) -> Option<String> {
  let bundle = installed_directory(mode)?;
  let text = std::fs::read_to_string(bundle.join("Contents").join("Info.plist")).ok()?;
  read_plist_string(&text, "CFBundleShortVersionString")
}

fn read_plist_string(text: &str, key: &str) -> Option<String> {
  let after_key = text.split_once(&format!("<key>{key}</key>"))?.1;
  let value = after_key
    .split_once("<string>")?
    .1
    .split_once("</string>")?
    .0;
  let trimmed = value.trim();
  if trimmed.is_empty() {
    None
  } else {
    Some(trimmed.to_string())
  }
}

fn current_bundle() -> Option<PathBuf> {
  let executable = std::env::current_exe().ok()?;
  executable
    .parent()
    .and_then(Path::parent)
    .and_then(Path::parent)
    .map(Path::to_path_buf)
}

fn home_desktop_directory() -> Option<PathBuf> {
  std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Desktop"))
}

fn information_property_list() -> String {
  format!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n  <key>CFBundleName</key>\n  <string>{name}</string>\n  <key>CFBundleDisplayName</key>\n  <string>{name}</string>\n  <key>CFBundleExecutable</key>\n  <string>{executable}</string>\n  <key>CFBundleIdentifier</key>\n  <string>{identifier}</string>\n  <key>CFBundleVersion</key>\n  <string>{version}</string>\n  <key>CFBundleShortVersionString</key>\n  <string>{version}</string>\n  <key>CFBundlePackageType</key>\n  <string>APPL</string>\n  <key>NSHighResolutionCapable</key>\n  <true/>\n</dict>\n</plist>\n",
    name = common::PRODUCT_NAME,
    executable = common::application_executable_file_name(),
    identifier = BUNDLE_IDENTIFIER,
    version = common::VERSION
  )
}

pub fn perform_install(request: &InstallRequest, report: &dyn Fn(&str)) -> Result<PathBuf, String> {
  report("Preparing the application bundle");
  let contents = request.install_directory.join("Contents");
  let macos_directory = contents.join("MacOS");
  let resources_directory = contents.join("Resources");
  std::fs::create_dir_all(&macos_directory)
    .map_err(|error| format!("Could not create the application bundle: {error}"))?;
  std::fs::create_dir_all(&resources_directory)
    .map_err(|error| format!("Could not create the application bundle: {error}"))?;

  report("Writing application metadata");
  std::fs::write(contents.join("Info.plist"), information_property_list())
    .map_err(|error| format!("Could not write the application metadata: {error}"))?;
  std::fs::write(
    resources_directory.join("icon.png"),
    common::APPLICATION_ICON_PNG,
  )
  .map_err(|error| format!("Could not write the application icon: {error}"))?;

  report("Copying application files");
  let executable = plan::file_in(&macos_directory, common::application_executable_file_name());
  common::write_application_binary(&executable)?;

  let uninstaller = plan::file_in(&macos_directory, common::uninstaller_file_name());
  common::copy_self_as_uninstaller(&uninstaller)?;

  if request.create_desktop_shortcut {
    report("Creating the desktop shortcut");
    if let Some(directory) = home_desktop_directory() {
      let _ = std::fs::create_dir_all(&directory);
      let shortcut = directory.join(BUNDLE_NAME);
      let _ = std::fs::remove_file(&shortcut);
      symlink(&request.install_directory, &shortcut)
        .map_err(|error| format!("Could not create the desktop shortcut: {error}"))?;
    }
  }

  report("Installation complete");
  Ok(executable)
}

pub fn perform_uninstall(request: &UninstallRequest, report: &dyn Fn(&str)) -> Result<(), String> {
  report("Removing shortcuts");
  if let Some(directory) = home_desktop_directory() {
    let _ = std::fs::remove_file(directory.join(BUNDLE_NAME));
  }

  if request.remove_application_data {
    report("Removing the application data folder");
    common::remove_application_data()?;
  }

  report("Removing application files");
  if let Some(bundle) = current_bundle() {
    std::fs::remove_dir_all(&bundle)
      .map_err(|error| format!("Could not remove the application bundle: {error}"))?;
  }

  report("Uninstall complete");
  Ok(())
}

pub fn launch_executable(path: &Path) -> Result<(), String> {
  let bundle = path
    .parent()
    .and_then(Path::parent)
    .and_then(Path::parent)
    .map(Path::to_path_buf);
  let mut command = Command::new("open");
  match bundle {
    Some(bundle) => command.arg(bundle),
    None => command.arg(path),
  };
  command
    .spawn()
    .map(|_| ())
    .map_err(|error| format!("Could not start the application: {error}"))
}
