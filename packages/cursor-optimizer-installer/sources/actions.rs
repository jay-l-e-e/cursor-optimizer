use std::path::{Path, PathBuf};

use tiny_json::Value;

use crate::common;
use crate::platform;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InstallerMode {
  Install,
  Uninstall,
}

pub struct InstallRequest {
  pub install_directory: PathBuf,
  #[cfg_attr(target_os = "macos", allow(dead_code))]
  pub create_start_menu_shortcut: bool,
  pub create_desktop_shortcut: bool,
}

pub struct UninstallRequest {
  pub remove_application_data: bool,
}

pub fn detect_mode() -> InstallerMode {
  if std::env::args().any(|argument| argument == "--uninstall") {
    InstallerMode::Uninstall
  } else {
    InstallerMode::Install
  }
}

fn mode_label(mode: InstallerMode) -> &'static str {
  match mode {
    InstallerMode::Install => "install",
    InstallerMode::Uninstall => "uninstall",
  }
}

fn text_or_null(value: Option<PathBuf>) -> Value {
  match value {
    Some(path) => Value::Text(path.display().to_string()),
    None => Value::Null,
  }
}

pub fn info_value(mode: InstallerMode) -> Value {
  Value::Object(vec![
    (
      "mode".to_string(),
      Value::Text(mode_label(mode).to_string()),
    ),
    (
      "productName".to_string(),
      Value::Text(common::PRODUCT_NAME.to_string()),
    ),
    (
      "version".to_string(),
      Value::Text(common::VERSION.to_string()),
    ),
    (
      "defaultInstallDirectory".to_string(),
      Value::Text(platform::default_install_directory().display().to_string()),
    ),
    (
      "installedDirectory".to_string(),
      text_or_null(platform::installed_directory(mode)),
    ),
    (
      "installedVersion".to_string(),
      platform::installed_version(mode).map_or(Value::Null, Value::Text),
    ),
    (
      "hasEmbeddedBinary".to_string(),
      Value::Boolean(common::has_embedded_binary()),
    ),
  ])
}

pub fn install_value(params: &Value, report: &dyn Fn(&str)) -> Result<Value, String> {
  let install_directory = params
    .get("installDirectory")
    .and_then(Value::as_text)
    .filter(|value| !value.trim().is_empty())
    .map(PathBuf::from)
    .unwrap_or_else(platform::default_install_directory);
  let request = InstallRequest {
    install_directory,
    create_start_menu_shortcut: params
      .get("createStartMenuShortcut")
      .and_then(Value::as_boolean)
      .unwrap_or(true),
    create_desktop_shortcut: params
      .get("createDesktopShortcut")
      .and_then(Value::as_boolean)
      .unwrap_or(true),
  };
  let executable = platform::perform_install(&request, report)?;
  Ok(Value::Object(vec![(
    "executablePath".to_string(),
    Value::Text(executable.display().to_string()),
  )]))
}

pub fn uninstall_value(params: &Value, report: &dyn Fn(&str)) -> Result<Value, String> {
  let request = UninstallRequest {
    remove_application_data: params
      .get("removeApplicationData")
      .and_then(Value::as_boolean)
      .unwrap_or(true),
  };
  platform::perform_uninstall(&request, report)?;
  Ok(Value::Null)
}

pub fn launch_value(params: &Value) -> Result<Value, String> {
  let executable = params
    .get("executablePath")
    .and_then(Value::as_text)
    .ok_or_else(|| "No install location was provided.".to_string())?;
  platform::launch_executable(Path::new(executable))?;
  Ok(Value::Null)
}

pub fn browse_directories_value(params: &Value) -> Value {
  let raw_path = params
    .get("path")
    .and_then(Value::as_text)
    .filter(|value| !value.trim().is_empty());
  let fallback = platform::default_install_directory();
  let (directory, pending_segments) = resolve_directory_with_pending(raw_path, &fallback);
  let parent = directory.parent().map(|path| path.display().to_string());

  let mut entries = Vec::new();
  if let Ok(read) = std::fs::read_dir(&directory) {
    for item in read.flatten() {
      let path = item.path();
      if path.is_dir() {
        let name = item.file_name().to_string_lossy().to_string();
        entries.push(Value::Object(vec![
          ("name".to_string(), Value::Text(name)),
          ("path".to_string(), Value::Text(path.display().to_string())),
        ]));
      }
    }
  }
  entries.sort_by(|left, right| {
    let left_name = left.get("name").and_then(Value::as_text).unwrap_or("");
    let right_name = right.get("name").and_then(Value::as_text).unwrap_or("");
    left_name.to_lowercase().cmp(&right_name.to_lowercase())
  });

  Value::Object(vec![
    (
      "currentDirectory".to_string(),
      Value::Text(directory.display().to_string()),
    ),
    (
      "parentDirectory".to_string(),
      parent.map_or(Value::Null, Value::Text),
    ),
    ("roots".to_string(), Value::Array(root_directories())),
    (
      "quickLocations".to_string(),
      Value::Array(quick_locations()),
    ),
    ("entries".to_string(), Value::Array(entries)),
    (
      "pendingSegments".to_string(),
      Value::Array(pending_segments.into_iter().map(Value::Text).collect()),
    ),
  ])
}

fn resolve_directory_with_pending(
  requested_path: Option<&str>,
  fallback: &Path,
) -> (PathBuf, Vec<String>) {
  let Some(raw) = requested_path else {
    return (fallback.to_path_buf(), Vec::new());
  };
  let mut directory = PathBuf::from(raw);
  if directory.is_dir() {
    return (directory, Vec::new());
  }
  let mut pending_segments: Vec<String> = Vec::new();
  while !directory.is_dir() {
    match directory.file_name() {
      Some(name) => pending_segments.push(name.to_string_lossy().to_string()),
      None => break,
    }
    match directory.parent() {
      Some(parent) => directory = parent.to_path_buf(),
      None => break,
    }
  }
  pending_segments.reverse();
  if directory.is_dir() {
    (directory, pending_segments)
  } else {
    (fallback.to_path_buf(), Vec::new())
  }
}

fn root_directories() -> Vec<Value> {
  let mut entries = Vec::new();
  #[cfg(target_os = "windows")]
  {
    for letter in b'A'..=b'Z' {
      let path = format!("{}:\\", letter as char);
      if Path::new(&path).is_dir() {
        entries.push(Value::Object(vec![("path".to_string(), Value::Text(path))]));
      }
    }
  }
  #[cfg(not(target_os = "windows"))]
  {
    entries.push(Value::Object(vec![(
      "path".to_string(),
      Value::Text("/".to_string()),
    )]));
  }
  entries
}

fn quick_locations() -> Vec<Value> {
  let mut entries = Vec::new();
  if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
    let home_path = PathBuf::from(home);
    entries.push(Value::Object(vec![
      ("name".to_string(), Value::Text("Home".to_string())),
      (
        "path".to_string(),
        Value::Text(home_path.display().to_string()),
      ),
    ]));
    for name in ["Desktop", "Documents", "Downloads"] {
      let path = home_path.join(name);
      if path.is_dir() {
        entries.push(Value::Object(vec![
          ("name".to_string(), Value::Text(name.to_string())),
          ("path".to_string(), Value::Text(path.display().to_string())),
        ]));
      }
    }
  }
  let default_install = platform::default_install_directory();
  if let Some(parent) = default_install.parent()
    && parent.is_dir()
  {
    entries.push(Value::Object(vec![
      ("name".to_string(), Value::Text("Program Files".to_string())),
      (
        "path".to_string(),
        Value::Text(parent.display().to_string()),
      ),
    ]));
  }
  entries
}
