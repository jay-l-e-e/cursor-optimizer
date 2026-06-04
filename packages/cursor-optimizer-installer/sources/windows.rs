use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::actions::{InstallRequest, InstallerMode, UninstallRequest};
use crate::common;
use crate::plan;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
const REGISTRY_KEY: &str =
  r"HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\CursorOptimizer";

pub fn default_install_directory() -> PathBuf {
  std::env::var_os("ProgramFiles")
    .map(PathBuf::from)
    .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"))
    .join(common::INSTALL_DIRECTORY_NAME)
}

pub fn installed_directory(mode: InstallerMode) -> Option<PathBuf> {
  match mode {
    InstallerMode::Uninstall => current_install_directory(),
    InstallerMode::Install => read_install_location(),
  }
}

fn current_install_directory() -> Option<PathBuf> {
  std::env::current_exe()
    .ok()
    .and_then(|path| path.parent().map(Path::to_path_buf))
}

fn read_install_location() -> Option<PathBuf> {
  read_registry_value("InstallLocation").map(PathBuf::from)
}

pub fn installed_version(_mode: InstallerMode) -> Option<String> {
  read_registry_value("DisplayVersion")
}

fn read_registry_value(name: &str) -> Option<String> {
  let output = Command::new("reg")
    .args(["query", REGISTRY_KEY, "/v", name])
    .creation_flags(CREATE_NO_WINDOW)
    .output()
    .ok()?;
  if !output.status.success() {
    return None;
  }
  let text = String::from_utf8_lossy(&output.stdout);
  for line in text.lines() {
    if let Some(value) = line.split("REG_SZ").nth(1) {
      let trimmed = value.trim();
      if !trimmed.is_empty() {
        return Some(trimmed.to_string());
      }
    }
  }
  None
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

  let uninstaller = plan::file_in(&request.install_directory, common::uninstaller_file_name());
  common::copy_self_as_uninstaller(&uninstaller)?;

  if request.create_start_menu_shortcut {
    report("Creating the Start menu shortcut");
    if let Some(directory) = start_menu_directory() {
      let _ = std::fs::create_dir_all(&directory);
      create_shortcut(
        &plan::windows_shortcut_in(&directory, common::PRODUCT_NAME),
        &executable,
        &request.install_directory,
      )?;
    }
  }

  if request.create_desktop_shortcut {
    report("Creating the desktop shortcut");
    if let Some(directory) = desktop_directory() {
      create_shortcut(
        &plan::windows_shortcut_in(&directory, common::PRODUCT_NAME),
        &executable,
        &request.install_directory,
      )?;
    }
  }

  report("Registering the application");
  register_uninstall(&request.install_directory, &executable, &uninstaller)?;

  report("Installation complete");
  Ok(executable)
}

pub fn perform_uninstall(request: &UninstallRequest, report: &dyn Fn(&str)) -> Result<(), String> {
  report("Removing the application registration");
  remove_registry();

  report("Removing shortcuts");
  if let Some(directory) = start_menu_directory() {
    let _ = std::fs::remove_file(plan::windows_shortcut_in(&directory, common::PRODUCT_NAME));
  }
  if let Some(directory) = desktop_directory() {
    let _ = std::fs::remove_file(plan::windows_shortcut_in(&directory, common::PRODUCT_NAME));
  }

  if request.remove_application_data {
    report("Removing the application data folder");
    common::remove_application_data()?;
  }

  report("Removing application files");
  if let Some(directory) = current_install_directory() {
    schedule_directory_removal(&directory)?;
  }

  report("Uninstall complete");
  Ok(())
}

pub fn launch_executable(path: &Path) -> Result<(), String> {
  Command::new("explorer")
    .arg(path)
    .creation_flags(CREATE_NO_WINDOW)
    .spawn()
    .map(|_| ())
    .map_err(|error| format!("Could not start the application: {error}"))
}

fn start_menu_directory() -> Option<PathBuf> {
  std::env::var_os("ProgramData").map(|value| {
    PathBuf::from(value)
      .join("Microsoft")
      .join("Windows")
      .join("Start Menu")
      .join("Programs")
  })
}

fn desktop_directory() -> Option<PathBuf> {
  std::env::var_os("PUBLIC").map(|value| PathBuf::from(value).join("Desktop"))
}

fn powershell_quote(path: &Path) -> String {
  path.display().to_string().replace('\'', "''")
}

fn create_shortcut(shortcut: &Path, target: &Path, working_directory: &Path) -> Result<(), String> {
  let script = format!(
    "$shell = New-Object -ComObject WScript.Shell; $link = $shell.CreateShortcut('{}'); $link.TargetPath = '{}'; $link.WorkingDirectory = '{}'; $link.IconLocation = '{},0'; $link.Save()",
    powershell_quote(shortcut),
    powershell_quote(target),
    powershell_quote(working_directory),
    powershell_quote(target)
  );
  let status = Command::new("powershell")
    .args(["-NoProfile", "-NonInteractive", "-Command", &script])
    .creation_flags(CREATE_NO_WINDOW)
    .status()
    .map_err(|error| format!("Could not create a shortcut: {error}"))?;
  if status.success() {
    Ok(())
  } else {
    Err("A shortcut could not be created.".to_string())
  }
}

fn register_uninstall(
  install_directory: &Path,
  executable: &Path,
  uninstaller: &Path,
) -> Result<(), String> {
  let entries = [
    ("DisplayName", "REG_SZ", common::PRODUCT_NAME.to_string()),
    ("DisplayVersion", "REG_SZ", common::VERSION.to_string()),
    ("Publisher", "REG_SZ", common::PUBLISHER.to_string()),
    ("DisplayIcon", "REG_SZ", executable.display().to_string()),
    (
      "InstallLocation",
      "REG_SZ",
      install_directory.display().to_string(),
    ),
    (
      "UninstallString",
      "REG_SZ",
      plan::uninstall_command(uninstaller),
    ),
    ("NoModify", "REG_DWORD", "1".to_string()),
    ("NoRepair", "REG_DWORD", "1".to_string()),
  ];
  for (name, kind, data) in entries {
    register_value(name, kind, &data)?;
  }
  if let Some(size) = estimated_size_kibibytes() {
    let _ = register_value("EstimatedSize", "REG_DWORD", &size.to_string());
  }
  Ok(())
}

fn estimated_size_kibibytes() -> Option<u32> {
  u32::try_from(crate::EMBEDDED_APPLICATION_BINARY.len() / 1024).ok()
}

fn register_value(value_name: &str, value_type: &str, data: &str) -> Result<(), String> {
  let status = Command::new("reg")
    .args([
      "add",
      REGISTRY_KEY,
      "/v",
      value_name,
      "/t",
      value_type,
      "/d",
      data,
      "/f",
    ])
    .creation_flags(CREATE_NO_WINDOW)
    .status()
    .map_err(|error| format!("Could not register the application: {error}"))?;
  if status.success() {
    Ok(())
  } else {
    Err("Could not register the application.".to_string())
  }
}

fn remove_registry() {
  let _ = Command::new("reg")
    .args(["delete", REGISTRY_KEY, "/f"])
    .creation_flags(CREATE_NO_WINDOW)
    .status();
}

fn schedule_directory_removal(directory: &Path) -> Result<(), String> {
  let process_id = std::process::id();
  let temp = std::env::temp_dir();
  let script_path = temp.join(format!("cursor-optimizer-cleanup-{process_id}.cmd"));
  let target = directory.display();

  let script = format!(
    "@echo off\r\ncd /d \"%~dp0\"\r\nset /a tries=0\r\n:repeat\r\nrmdir /s /q \"{target}\" 2>nul\r\nif not exist \"{target}\" goto done\r\nset /a tries+=1\r\nif %tries% geq 120 goto done\r\nping -n 2 127.0.0.1 >nul\r\ngoto repeat\r\n:done\r\ndel /f /q \"%~f0\"\r\n"
  );
  std::fs::write(&script_path, script)
    .map_err(|error| format!("Could not prepare cleanup of the installation folder: {error}"))?;

  let spawn_cleanup = |extra_flags: u32| {
    Command::new("cmd")
      .arg("/c")
      .arg(&script_path)
      .current_dir(&temp)
      .creation_flags(CREATE_NO_WINDOW | extra_flags)
      .spawn()
  };

  match spawn_cleanup(CREATE_BREAKAWAY_FROM_JOB) {
    Ok(_) => Ok(()),
    Err(_) => spawn_cleanup(0)
      .map(|_| ())
      .map_err(|error| format!("Could not schedule cleanup of the installation folder: {error}")),
  }
}
