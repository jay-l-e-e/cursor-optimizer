mod cleanup;
mod overview;
mod paths;
mod process;
mod tools;

pub use cleanup::{analyze_deep_clean, analyze_light_clean, run_deep_clean, run_light_clean};
pub use overview::{gather_overview, quick_summary};
pub use paths::{CursorLocations, locate};
pub use process::{force_quit_cursor, is_cursor_running};
pub use tools::{
  browse_directories, storage_estimate, tool_analyze, tool_backup, tool_checkpoint,
  tool_flush_database, tool_integrity_check, tool_vacuum,
};

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::Connection;
use tiny_json::Value;

pub type ProgressReporter<'reporter> = &'reporter dyn Fn(&str);
pub type CancelToken<'token> = &'token AtomicBool;

pub(crate) struct ProgressDetail<'content> {
  pub detail: Option<&'content str>,
  pub done_bytes: Option<i64>,
  pub total_bytes: Option<i64>,
}

pub(crate) fn cancellation_requested(token: CancelToken<'_>) -> bool {
  token.load(Ordering::Relaxed)
}

pub(crate) fn cancelled_error() -> String {
  "cancelled".to_string()
}

fn open_common(locations: &CursorLocations) -> Result<Connection, String> {
  if !locations.database_exists() {
    return Err(format!(
      "Cursor database was not found at {}",
      locations.state_database.display()
    ));
  }
  let connection = Connection::open(&locations.state_database)
    .map_err(|error| format!("could not open database: {error}"))?;
  connection
    .busy_timeout(Duration::from_secs(120))
    .map_err(|error| format!("could not set busy timeout: {error}"))?;
  Ok(connection)
}

pub(crate) fn open_for_read(locations: &CursorLocations) -> Result<Connection, String> {
  let connection = open_common(locations)?;
  connection
    .pragma_update(None, "temp_store", "MEMORY")
    .map_err(|error| format!("could not set temp_store: {error}"))?;
  connection
    .pragma_update(None, "cache_size", -65_536)
    .map_err(|error| format!("could not set cache_size: {error}"))?;
  connection
    .pragma_update(None, "mmap_size", 268_435_456)
    .map_err(|error| format!("could not set mmap_size: {error}"))?;
  connection
    .pragma_update(None, "query_only", true)
    .map_err(|error| format!("could not enable read-only mode: {error}"))?;
  Ok(connection)
}

pub(crate) fn open_for_write(locations: &CursorLocations) -> Result<Connection, String> {
  let connection = open_common(locations)?;
  connection
    .pragma_update(None, "temp_store", "MEMORY")
    .map_err(|error| format!("could not set temp_store: {error}"))?;
  Ok(connection)
}

pub(crate) fn file_size(path: &Path) -> u64 {
  fs::metadata(path)
    .map(|metadata| metadata.len())
    .unwrap_or(0)
}

pub(crate) fn human_readable_size(bytes: u64) -> String {
  const UNIT_LABELS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
  let mut value = bytes as f64;
  let mut unit_index = 0;
  while value >= 1024.0 && unit_index < UNIT_LABELS.len() - 1 {
    value /= 1024.0;
    unit_index += 1;
  }
  if unit_index == 0 {
    format!("{bytes} {}", UNIT_LABELS[unit_index])
  } else {
    format!("{value:.2} {}", UNIT_LABELS[unit_index])
  }
}

pub(crate) fn current_time_millis() -> i64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|duration| duration.as_millis() as i64)
    .unwrap_or(0)
}

pub(crate) fn read_single_integer(connection: &Connection, sql: &str) -> Result<i64, String> {
  connection
    .query_row(sql, [], |row| row.get(0))
    .map_err(|error| format!("query failed ({sql}): {error}"))
}

pub(crate) fn read_pragma_integer(
  connection: &Connection,
  pragma_name: &str,
) -> Result<i64, String> {
  connection
    .query_row(&format!("PRAGMA {pragma_name}"), [], |row| row.get(0))
    .map_err(|error| format!("could not read pragma {pragma_name}: {error}"))
}

pub(crate) fn build_object(entries: Vec<(&str, Value)>) -> Value {
  Value::Object(
    entries
      .into_iter()
      .map(|(key, value)| (key.to_string(), value))
      .collect(),
  )
}

pub(crate) fn text_value(content: impl Into<String>) -> Value {
  Value::Text(content.into())
}

pub(crate) fn integer_value(number: i64) -> Value {
  Value::Number(number as f64)
}

pub(crate) fn boolean_value(flag: bool) -> Value {
  Value::Boolean(flag)
}

pub(crate) fn report_stage(
  reporter: ProgressReporter<'_>,
  stage: i64,
  stage_count: i64,
  label: &str,
  done: Option<i64>,
  total: Option<i64>,
) {
  let mut entries = vec![
    ("kind", text_value("progress")),
    ("stage", integer_value(stage)),
    ("stageCount", integer_value(stage_count)),
    ("label", text_value(label)),
  ];
  if let Some(value) = done {
    entries.push(("done", integer_value(value)));
  }
  if let Some(value) = total {
    entries.push(("total", integer_value(value)));
  }
  reporter(&build_object(entries).to_json_string());
}

pub(crate) fn report_stage_detail(
  reporter: ProgressReporter<'_>,
  stage: i64,
  stage_count: i64,
  label: &str,
  done: Option<i64>,
  total: Option<i64>,
  detail: ProgressDetail<'_>,
) {
  let mut entries = vec![
    ("kind", text_value("progress")),
    ("stage", integer_value(stage)),
    ("stageCount", integer_value(stage_count)),
    ("label", text_value(label)),
  ];
  if let Some(value) = done {
    entries.push(("done", integer_value(value)));
  }
  if let Some(value) = total {
    entries.push(("total", integer_value(value)));
  }
  if let Some(value) = detail.detail {
    entries.push(("detail", text_value(value)));
  }
  if let Some(value) = detail.done_bytes {
    entries.push(("doneBytes", integer_value(value)));
  }
  if let Some(value) = detail.total_bytes {
    entries.push(("totalBytes", integer_value(value)));
  }
  reporter(&build_object(entries).to_json_string());
}

#[cfg(test)]
mod tests {
  use super::{current_time_millis, human_readable_size};

  #[test]
  fn formats_human_sizes() {
    assert_eq!(human_readable_size(0), "0 B");
    assert_eq!(human_readable_size(512), "512 B");
    assert_eq!(human_readable_size(1024), "1.00 KiB");
    assert_eq!(human_readable_size(1_572_864), "1.50 MiB");
  }

  #[test]
  fn current_time_is_positive() {
    assert!(current_time_millis() > 0);
  }
}
