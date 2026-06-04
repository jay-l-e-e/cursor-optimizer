use std::fs;
use std::io::{BufReader, BufWriter, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use rusqlite::Connection;
use tiny_json::Value;

use crate::{
  CursorLocations, ProgressDetail, ProgressReporter, boolean_value, build_object,
  current_time_millis, file_size, human_readable_size, integer_value, open_for_read,
  open_for_write, read_pragma_integer, report_stage, report_stage_detail, text_value,
};

fn available_space(path: &Path) -> Option<u64> {
  #[cfg(target_os = "windows")]
  {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let text = path.display().to_string();
    let drive_name = text.chars().next()?.to_string();
    let output = Command::new("powershell")
      .args([
        "-NoProfile",
        "-Command",
        &format!("(Get-PSDrive -Name '{drive_name}').Free"),
      ])
      .creation_flags(CREATE_NO_WINDOW)
      .output()
      .ok()?;
    String::from_utf8(output.stdout).ok()?.trim().parse().ok()
  }
  #[cfg(not(target_os = "windows"))]
  {
    let output = Command::new("df").arg("-Pk").arg(path).output().ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    let line = text.lines().nth(1)?;
    let available_kilobytes = line.split_whitespace().nth(3)?.parse::<u64>().ok()?;
    available_kilobytes.checked_mul(1024)
  }
}

fn bytes_value(bytes: u64) -> Value {
  integer_value(bytes.min(i64::MAX as u64) as i64)
}

pub fn storage_estimate(locations: &CursorLocations) -> Value {
  let database_bytes = file_size(&locations.state_database);
  let write_ahead_log_bytes = file_size(&locations.write_ahead_log);
  let available_bytes = available_space(&locations.global_storage_directory);
  build_object(vec![
    ("databaseBytes", bytes_value(database_bytes)),
    (
      "databaseHuman",
      text_value(human_readable_size(database_bytes)),
    ),
    ("writeAheadLogBytes", bytes_value(write_ahead_log_bytes)),
    (
      "writeAheadLogHuman",
      text_value(human_readable_size(write_ahead_log_bytes)),
    ),
    (
      "availableBytes",
      available_bytes.map_or(Value::Null, bytes_value),
    ),
    (
      "availableHuman",
      available_bytes.map_or_else(
        || Value::Null,
        |bytes| text_value(human_readable_size(bytes)),
      ),
    ),
    (
      "backupDirectory",
      text_value(locations.global_storage_directory.display().to_string()),
    ),
    (
      "backupFileName",
      text_value(format!("state.vscdb.backup-{}.zst", current_time_millis())),
    ),
  ])
}

fn root_directories() -> Vec<Value> {
  let mut entries = Vec::new();
  #[cfg(target_os = "windows")]
  {
    for letter in b'A'..=b'Z' {
      let path = format!("{}:\\", letter as char);
      if Path::new(&path).is_dir() {
        entries.push(build_object(vec![("path", text_value(path.clone()))]));
      }
    }
  }
  #[cfg(not(target_os = "windows"))]
  {
    entries.push(build_object(vec![("path", text_value("/"))]));
  }
  entries
}

fn quick_locations(locations: &CursorLocations) -> Vec<Value> {
  let mut entries = Vec::new();
  entries.push(build_object(vec![
    ("name", text_value("Cursor data")),
    (
      "path",
      text_value(locations.global_storage_directory.display().to_string()),
    ),
  ]));
  if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
    let home_path = PathBuf::from(home);
    entries.push(build_object(vec![
      ("name", text_value("Home")),
      ("path", text_value(home_path.display().to_string())),
    ]));
    for name in ["Desktop", "Documents", "Downloads"] {
      let path = home_path.join(name);
      if path.is_dir() {
        entries.push(build_object(vec![
          ("name", text_value(name)),
          ("path", text_value(path.display().to_string())),
        ]));
      }
    }
  }
  entries
}

pub fn browse_directories(locations: &CursorLocations, requested_path: Option<&str>) -> Value {
  let fallback = locations.global_storage_directory.clone();
  let (directory, pending_segments) = resolve_directory_with_pending(requested_path, &fallback);
  let parent = directory.parent().map(|path| path.display().to_string());
  let mut entries = Vec::new();
  if let Ok(read_directory) = fs::read_dir(&directory) {
    for entry in read_directory.flatten() {
      let path = entry.path();
      if path.is_dir() {
        let name = entry.file_name().to_string_lossy().to_string();
        entries.push((name, path.display().to_string()));
      }
    }
  }
  entries.sort_by_key(|entry| entry.0.to_lowercase());
  Value::Object(vec![
    (
      "currentDirectory".to_string(),
      text_value(directory.display().to_string()),
    ),
    (
      "parentDirectory".to_string(),
      parent.map_or(Value::Null, text_value),
    ),
    ("roots".to_string(), Value::Array(root_directories())),
    (
      "quickLocations".to_string(),
      Value::Array(quick_locations(locations)),
    ),
    (
      "entries".to_string(),
      Value::Array(
        entries
          .into_iter()
          .map(|(name, path)| {
            build_object(vec![("name", text_value(name)), ("path", text_value(path))])
          })
          .collect(),
      ),
    ),
    (
      "pendingSegments".to_string(),
      Value::Array(pending_segments.into_iter().map(text_value).collect()),
    ),
  ])
}

fn resolve_directory_with_pending(
  requested_path: Option<&str>,
  fallback: &Path,
) -> (PathBuf, Vec<String>) {
  let Some(raw) = requested_path.filter(|value| !value.trim().is_empty()) else {
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

pub fn tool_integrity_check(
  locations: &CursorLocations,
  report_progress: ProgressReporter<'_>,
) -> Result<Value, String> {
  report_stage(
    report_progress,
    1,
    1,
    "Checking database integrity",
    None,
    None,
  );
  let connection = open_for_read(locations)?;
  let result: String = connection
    .query_row("PRAGMA integrity_check", [], |row| row.get(0))
    .map_err(|error| format!("integrity check failed: {error}"))?;
  Ok(build_object(vec![
    ("result", text_value(result.clone())),
    ("healthy", boolean_value(result == "ok")),
  ]))
}

pub fn tool_checkpoint(
  locations: &CursorLocations,
  report_progress: ProgressReporter<'_>,
) -> Result<Value, String> {
  report_stage(report_progress, 1, 1, "Flushing pending writes", None, None);
  let connection = open_for_write(locations)?;
  let before = file_size(&locations.write_ahead_log);
  connection
    .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
    .map_err(|error| format!("checkpoint failed: {error}"))?;
  drop(connection);
  let after = file_size(&locations.write_ahead_log);
  Ok(build_object(vec![
    ("beforeHuman", text_value(human_readable_size(before))),
    ("afterHuman", text_value(human_readable_size(after))),
  ]))
}

pub fn tool_vacuum(
  locations: &CursorLocations,
  report_progress: ProgressReporter<'_>,
) -> Result<Value, String> {
  let connection = open_for_write(locations)?;
  let before = file_size(&locations.state_database);
  vacuum_with_progress(
    locations,
    connection,
    report_progress,
    1,
    1,
    "Compacting storage",
  )?;
  let after = file_size(&locations.state_database);
  Ok(build_object(vec![
    ("beforeHuman", text_value(human_readable_size(before))),
    ("afterHuman", text_value(human_readable_size(after))),
    (
      "reclaimedHuman",
      text_value(human_readable_size(before.saturating_sub(after))),
    ),
  ]))
}

pub fn tool_flush_database(
  locations: &CursorLocations,
  report_progress: ProgressReporter<'_>,
) -> Result<Value, String> {
  let connection = open_for_write(locations)?;
  let before_database_bytes = file_size(&locations.state_database);
  let before_write_ahead_log_bytes = file_size(&locations.write_ahead_log);
  report_stage(report_progress, 1, 4, "Preparing database", None, None);
  connection
    .execute_batch("PRAGMA journal_mode=DELETE")
    .map_err(|error| format!("could not prepare journal mode: {error}"))?;
  report_stage(
    report_progress,
    2,
    4,
    "Removing generated chat cache",
    None,
    None,
  );
  connection
    .execute_batch(
      "BEGIN IMMEDIATE;
       DELETE FROM cursorDiskKV WHERE key LIKE 'agentKv:%';
       DELETE FROM cursorDiskKV WHERE key LIKE 'bubbleId:%';
       DELETE FROM cursorDiskKV WHERE key LIKE 'checkpointId:%';
       DELETE FROM cursorDiskKV WHERE key LIKE 'composer.content.%';
       DELETE FROM cursorDiskKV WHERE key LIKE 'codeBlockDiff:%';
       DELETE FROM cursorDiskKV WHERE key LIKE 'ofsContent:%';
       DELETE FROM cursorDiskKV WHERE key LIKE 'codeBlockPartialInlineDiffFates:%';
       COMMIT;",
    )
    .map_err(|error| format!("could not flush generated chat cache: {error}"))?;
  vacuum_with_progress(
    locations,
    connection,
    report_progress,
    3,
    4,
    "Compacting storage",
  )?;
  let final_connection = open_for_write(locations)?;
  report_stage(report_progress, 4, 4, "Finalizing database", None, None);
  final_connection
    .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
    .map_err(|error| format!("final checkpoint failed: {error}"))?;
  drop(final_connection);
  let after_database_bytes = file_size(&locations.state_database);
  let after_write_ahead_log_bytes = file_size(&locations.write_ahead_log);
  let before_total_bytes = before_database_bytes.saturating_add(before_write_ahead_log_bytes);
  let after_total_bytes = after_database_bytes.saturating_add(after_write_ahead_log_bytes);
  Ok(build_object(vec![
    (
      "beforeDatabaseHuman",
      text_value(human_readable_size(before_database_bytes)),
    ),
    (
      "afterDatabaseHuman",
      text_value(human_readable_size(after_database_bytes)),
    ),
    (
      "beforeWriteAheadLogHuman",
      text_value(human_readable_size(before_write_ahead_log_bytes)),
    ),
    (
      "afterWriteAheadLogHuman",
      text_value(human_readable_size(after_write_ahead_log_bytes)),
    ),
    (
      "reclaimedHuman",
      text_value(human_readable_size(
        before_total_bytes.saturating_sub(after_total_bytes),
      )),
    ),
  ]))
}

pub(crate) fn vacuum_with_progress(
  locations: &CursorLocations,
  connection: Connection,
  report_progress: ProgressReporter<'_>,
  stage: i64,
  stage_count: i64,
  label: &str,
) -> Result<(), String> {
  let page_count = read_pragma_integer(&connection, "page_count").unwrap_or(0);
  let freelist_count = read_pragma_integer(&connection, "freelist_count").unwrap_or(0);
  let page_size = read_pragma_integer(&connection, "page_size").unwrap_or(0);
  let compacted_bytes = page_count
    .saturating_sub(freelist_count)
    .saturating_mul(page_size)
    .max(1) as u64;
  report_vacuum_percent(report_progress, stage, stage_count, label, 0);
  let (sender, receiver) = mpsc::channel();
  let handle = thread::spawn(move || {
    let result = connection
      .execute_batch("VACUUM")
      .map_err(|error| format!("vacuum failed: {error}"));
    let _ = sender.send(result);
  });
  let started = Instant::now();
  let mut last_percent = 0_i64;
  let mut plateau_start: Option<Instant> = None;
  let mut time_to_90_millis: Option<u64> = None;
  loop {
    match receiver.recv_timeout(Duration::from_millis(400)) {
      Ok(result) => {
        join_vacuum(handle)?;
        result?;
        break;
      }
      Err(RecvTimeoutError::Timeout) => {
        let sidecar_bytes =
          file_size(&locations.write_ahead_log).max(file_size(&locations.rollback_journal));
        let sidecar_percent =
          (sidecar_bytes.min(compacted_bytes).saturating_mul(90) / compacted_bytes) as i64;

        let percent = if sidecar_percent >= 90 {
          if plateau_start.is_none() {
            plateau_start = Some(Instant::now());
            time_to_90_millis = Some(started.elapsed().as_millis() as u64);
          }
          let half_of_90 = time_to_90_millis.unwrap_or(5000).max(500) / 2;
          let since = plateau_start.unwrap_or(started).elapsed().as_millis() as u64;
          let tail = (since.saturating_mul(9) / half_of_90.max(1)).min(9) as i64;
          90_i64.saturating_add(tail)
        } else {
          sidecar_percent
        };

        last_percent = last_percent.max(percent);
        report_vacuum_percent(report_progress, stage, stage_count, label, last_percent);
      }
      Err(RecvTimeoutError::Disconnected) => {
        join_vacuum(handle)?;
        return Err("Something went wrong. Please try again.".to_string());
      }
    }
  }
  report_vacuum_percent(report_progress, stage, stage_count, label, 100);
  Ok(())
}

fn report_vacuum_percent(
  report_progress: ProgressReporter<'_>,
  stage: i64,
  stage_count: i64,
  label: &str,
  percent: i64,
) {
  report_stage_detail(
    report_progress,
    stage,
    stage_count,
    label,
    Some(percent.clamp(0, 100)),
    Some(100),
    ProgressDetail {
      detail: None,
      done_bytes: None,
      total_bytes: None,
    },
  );
}

fn join_vacuum(handle: thread::JoinHandle<()>) -> Result<(), String> {
  handle
    .join()
    .map_err(|_| "vacuum stopped unexpectedly".to_string())
}

pub fn tool_analyze(
  locations: &CursorLocations,
  report_progress: ProgressReporter<'_>,
) -> Result<Value, String> {
  report_stage(
    report_progress,
    1,
    1,
    "Refreshing database statistics",
    None,
    None,
  );
  let connection = open_for_write(locations)?;
  connection
    .execute_batch("ANALYZE; PRAGMA optimize;")
    .map_err(|error| format!("analyze failed: {error}"))?;
  Ok(build_object(vec![(
    "message",
    text_value("statistics refreshed and optimizer run"),
  )]))
}

const ZSTD_DEFAULT_LEVEL: i32 = 10;
const COMPRESSION_CHUNK_BYTES: usize = 256 * 1024;

fn parse_compression_level(raw: Option<&str>) -> i32 {
  raw
    .and_then(|value| value.trim().parse::<i32>().ok())
    .unwrap_or(ZSTD_DEFAULT_LEVEL)
    .clamp(1, 22)
}

pub fn tool_backup(
  locations: &CursorLocations,
  backup_directory: Option<&str>,
  backup_file_name: Option<&str>,
  compression_level: Option<&str>,
  report_progress: ProgressReporter<'_>,
) -> Result<Value, String> {
  if !locations.database_exists() {
    return Err("database not found".to_string());
  }
  let level = parse_compression_level(compression_level);
  let directory = backup_directory
    .filter(|value| !value.trim().is_empty())
    .map(PathBuf::from)
    .unwrap_or_else(|| locations.global_storage_directory.clone());
  if !directory.is_dir() {
    fs::create_dir_all(&directory)
      .map_err(|error| format!("could not create backup directory: {error}"))?;
  }
  let file_name = backup_file_name
    .filter(|value| !value.trim().is_empty())
    .map(str::to_string)
    .unwrap_or_else(|| format!("state.vscdb.backup-{}.zst", current_time_millis()));
  if file_name.contains(std::path::MAIN_SEPARATOR)
    || file_name.contains('/')
    || file_name.contains('\\')
  {
    return Err("backup file name must not include a directory".to_string());
  }
  let mut backup_path = directory;
  backup_path.push(&file_name);
  if backup_path.exists() {
    return Err("backup file already exists".to_string());
  }
  let original_bytes = file_size(&locations.state_database);
  let total_bytes_signed = original_bytes.min(i64::MAX as u64) as i64;
  report_stage_detail(
    report_progress,
    1,
    1,
    "Compressing",
    Some(0),
    Some(total_bytes_signed),
    ProgressDetail {
      detail: None,
      done_bytes: Some(0),
      total_bytes: Some(total_bytes_signed),
    },
  );
  compress_file(
    &locations.state_database,
    &backup_path,
    original_bytes,
    level,
    report_progress,
  )?;
  let compressed_bytes = file_size(&backup_path);
  Ok(build_object(vec![
    ("path", text_value(backup_path.display().to_string())),
    ("human", text_value(human_readable_size(compressed_bytes))),
    (
      "originalHuman",
      text_value(human_readable_size(original_bytes)),
    ),
    (
      "ratio",
      text_value(compression_ratio_label(original_bytes, compressed_bytes)),
    ),
  ]))
}

fn compress_file(
  source: &Path,
  destination: &Path,
  total_bytes: u64,
  level: i32,
  report_progress: ProgressReporter<'_>,
) -> Result<(), String> {
  let source_file = fs::File::open(source)
    .map_err(|error| format!("could not open database for reading: {error}"))?;
  let mut reader = BufReader::new(source_file);
  let destination_file = fs::File::create(destination)
    .map_err(|error| format!("could not create backup file: {error}"))?;
  let writer = BufWriter::new(destination_file);
  let mut encoder = zstd::stream::Encoder::new(writer, level)
    .map_err(|error| format!("could not initialize compression: {error}"))?;
  let total_signed = total_bytes.min(i64::MAX as u64) as i64;
  let mut bytes_read: u64 = 0;
  let mut buffer = vec![0_u8; COMPRESSION_CHUNK_BYTES];
  loop {
    let chunk_size = reader
      .read(&mut buffer)
      .map_err(|error| format!("could not read database: {error}"))?;
    if chunk_size == 0 {
      break;
    }
    encoder
      .write_all(&buffer[..chunk_size])
      .map_err(|error| format!("compression write failed: {error}"))?;
    bytes_read = bytes_read.saturating_add(chunk_size as u64);
    let done_signed = bytes_read.min(i64::MAX as u64) as i64;
    report_stage_detail(
      report_progress,
      1,
      1,
      "Compressing",
      Some(done_signed),
      Some(total_signed),
      ProgressDetail {
        detail: None,
        done_bytes: Some(done_signed),
        total_bytes: Some(total_signed),
      },
    );
  }
  encoder
    .finish()
    .map_err(|error| format!("could not finalize compression: {error}"))?;
  Ok(())
}

fn compression_ratio_label(original: u64, compressed: u64) -> String {
  if original == 0 {
    return "—".to_string();
  }
  let ratio = (compressed as f64) / (original as f64) * 100.0;
  format!("{ratio:.1}%")
}
