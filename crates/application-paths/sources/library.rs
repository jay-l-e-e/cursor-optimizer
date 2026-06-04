use std::env;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TargetFamily {
  Windows,
  Unix,
}

const APPLICATION_DIRECTORY_NAME: &str = "CursorOptimizer";
const OPERATION_JOURNAL_FILE_NAME: &str = "operation-journal.json";

fn resolve_base_directory(
  family: TargetFamily,
  local_app_data: Option<PathBuf>,
  data_home: Option<PathBuf>,
  home: Option<PathBuf>,
) -> Option<PathBuf> {
  match family {
    TargetFamily::Windows => local_app_data,
    TargetFamily::Unix => data_home.or_else(|| {
      home.map(|mut directory| {
        directory.push(".local");
        directory.push("share");
        directory
      })
    }),
  }
}

fn current_family() -> TargetFamily {
  if cfg!(target_os = "windows") {
    TargetFamily::Windows
  } else {
    TargetFamily::Unix
  }
}

fn environment_base_directory() -> PathBuf {
  let local_app_data = env::var_os("LOCALAPPDATA").map(PathBuf::from);
  let data_home = env::var_os("XDG_DATA_HOME").map(PathBuf::from);
  let home = env::var_os("HOME").map(PathBuf::from);
  resolve_base_directory(current_family(), local_app_data, data_home, home)
    .unwrap_or_else(env::temp_dir)
}

pub fn base_data_directory() -> PathBuf {
  environment_base_directory()
}

pub fn application_data_directory_in(base_directory: &Path) -> PathBuf {
  base_directory.join(APPLICATION_DIRECTORY_NAME)
}

pub fn application_data_directory() -> PathBuf {
  application_data_directory_in(&environment_base_directory())
}

pub fn operation_journal_path() -> PathBuf {
  application_data_directory().join(OPERATION_JOURNAL_FILE_NAME)
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use super::{
    APPLICATION_DIRECTORY_NAME, TargetFamily, application_data_directory_in, resolve_base_directory,
  };

  #[test]
  fn windows_uses_local_app_data() {
    let base = resolve_base_directory(
      TargetFamily::Windows,
      Some(PathBuf::from("C:/Users/example/AppData/Local")),
      Some(PathBuf::from("/should/be/ignored")),
      Some(PathBuf::from("/home/example")),
    );
    assert_eq!(base, Some(PathBuf::from("C:/Users/example/AppData/Local")));
  }

  #[test]
  fn windows_without_local_app_data_is_none() {
    let base = resolve_base_directory(TargetFamily::Windows, None, None, None);
    assert_eq!(base, None);
  }

  #[test]
  fn unix_prefers_data_home() {
    let base = resolve_base_directory(
      TargetFamily::Unix,
      None,
      Some(PathBuf::from("/home/example/.local/share")),
      Some(PathBuf::from("/home/example")),
    );
    assert_eq!(base, Some(PathBuf::from("/home/example/.local/share")));
  }

  #[test]
  fn unix_falls_back_to_home_local_share() {
    let base = resolve_base_directory(
      TargetFamily::Unix,
      None,
      None,
      Some(PathBuf::from("/home/example")),
    );
    assert_eq!(base, Some(PathBuf::from("/home/example/.local/share")));
  }

  #[test]
  fn appends_application_directory_name() {
    let directory = application_data_directory_in(&PathBuf::from("/data"));
    assert_eq!(
      directory,
      PathBuf::from(format!("/data/{APPLICATION_DIRECTORY_NAME}"))
    );
  }
}
