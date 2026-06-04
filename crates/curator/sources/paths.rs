use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CursorLocations {
  pub base_directory: PathBuf,
  pub global_storage_directory: PathBuf,
  pub state_database: PathBuf,
  pub write_ahead_log: PathBuf,
  pub rollback_journal: PathBuf,
  pub shared_memory: PathBuf,
}

pub fn base_directory() -> Option<PathBuf> {
  #[cfg(target_os = "windows")]
  {
    let application_data = env::var_os("APPDATA")?;
    let mut directory = PathBuf::from(application_data);
    directory.push("Cursor");
    Some(directory)
  }

  #[cfg(target_os = "macos")]
  {
    let home = env::var_os("HOME")?;
    let mut directory = PathBuf::from(home);
    directory.push("Library");
    directory.push("Application Support");
    directory.push("Cursor");
    Some(directory)
  }

  #[cfg(all(unix, not(target_os = "macos")))]
  {
    if let Some(configuration_home) = env::var_os("XDG_CONFIG_HOME") {
      let mut directory = PathBuf::from(configuration_home);
      directory.push("Cursor");
      return Some(directory);
    }
    let home = env::var_os("HOME")?;
    let mut directory = PathBuf::from(home);
    directory.push(".config");
    directory.push("Cursor");
    Some(directory)
  }

  #[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
  {
    None
  }
}

pub fn locations_from_base(base_directory: PathBuf) -> CursorLocations {
  let mut global_storage_directory = base_directory.clone();
  global_storage_directory.push("User");
  global_storage_directory.push("globalStorage");

  let mut state_database = global_storage_directory.clone();
  state_database.push("state.vscdb");

  let mut write_ahead_log = global_storage_directory.clone();
  write_ahead_log.push("state.vscdb-wal");

  let mut rollback_journal = global_storage_directory.clone();
  rollback_journal.push("state.vscdb-journal");

  let mut shared_memory = global_storage_directory.clone();
  shared_memory.push("state.vscdb-shm");

  CursorLocations {
    base_directory,
    global_storage_directory,
    state_database,
    write_ahead_log,
    rollback_journal,
    shared_memory,
  }
}

pub fn locate() -> Option<CursorLocations> {
  base_directory().map(locations_from_base)
}

impl CursorLocations {
  pub fn database_exists(&self) -> bool {
    self.state_database.is_file()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn derives_expected_locations() {
    let locations = locations_from_base(PathBuf::from("/base"));
    assert!(
      locations
        .global_storage_directory
        .ends_with("globalStorage")
    );
    assert!(locations.state_database.ends_with("state.vscdb"));
    assert!(locations.write_ahead_log.ends_with("state.vscdb-wal"));
    assert!(locations.rollback_journal.ends_with("state.vscdb-journal"));
    assert!(locations.shared_memory.ends_with("state.vscdb-shm"));
  }
}
