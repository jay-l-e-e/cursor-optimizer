use std::collections::HashSet;

use rusqlite::Connection;
use rusqlite::types::ValueRef;
use tiny_json::Value;

use crate::tools::vacuum_with_progress;
use crate::{
  CancelToken, CursorLocations, ProgressReporter, build_object, cancellation_requested,
  cancelled_error, current_time_millis, file_size, human_readable_size, integer_value,
  open_for_read, open_for_write, read_pragma_integer, read_single_integer, report_stage,
  text_value,
};

const COMPOSER_DATA_KEY_RANGE: &str = "key >= 'composerData:' AND key < 'composerData;'";
const CHILD_CONVERSATION_KEY_RANGES: [&str; 3] = [
  "key >= 'bubbleId:' AND key < 'bubbleId;'",
  "key >= 'messageRequestContext:' AND key < 'messageRequestContext;'",
  "key >= 'checkpointId:' AND key < 'checkpointId;'",
];

const TIMESTAMP_FIELDS: [&[u8]; 4] = [
  b"\"lastUpdatedAt\"",
  b"\"timestamp\"",
  b"\"unixMs\"",
  b"\"createdAt\"",
];

pub fn analyze_light_clean(
  locations: &CursorLocations,
  report_progress: ProgressReporter<'_>,
  cancel: CancelToken<'_>,
) -> Result<Value, String> {
  let connection = open_for_read(locations)?;
  report_stage(report_progress, 1, 1, "Estimating compaction", None, None);
  if cancellation_requested(cancel) {
    return Err(cancelled_error());
  }
  let compaction_bytes = compaction_reclaim_bytes(&connection);

  Ok(build_object(vec![
    ("compactionReclaimBytes", integer_value(compaction_bytes)),
    (
      "compactionReclaimHuman",
      text_value(human_readable_size(compaction_bytes.max(0) as u64)),
    ),
    ("estimatedReclaimBytes", integer_value(compaction_bytes)),
    (
      "estimatedReclaimHuman",
      text_value(human_readable_size(compaction_bytes.max(0) as u64)),
    ),
  ]))
}

fn compaction_reclaim_bytes(connection: &Connection) -> i64 {
  let freelist_pages = read_pragma_integer(connection, "freelist_count").unwrap_or(0);
  let page_size = read_pragma_integer(connection, "page_size").unwrap_or(0);
  freelist_pages.saturating_mul(page_size).max(0)
}

pub fn run_light_clean(
  locations: &CursorLocations,
  report_progress: ProgressReporter<'_>,
  cancel: CancelToken<'_>,
) -> Result<Value, String> {
  let connection = open_for_write(locations)?;
  let before_bytes = file_size(&locations.state_database);
  if cancellation_requested(cancel) {
    return Err(cancelled_error());
  }
  report_stage(report_progress, 1, 1, "Compacting storage", None, None);
  checkpoint_and_vacuum(locations, connection, report_progress, 1, 1)?;

  let after_bytes = file_size(&locations.state_database);
  Ok(build_object(vec![
    ("deletedRows", integer_value(0)),
    ("beforeBytes", integer_value(before_bytes as i64)),
    ("afterBytes", integer_value(after_bytes as i64)),
    ("beforeHuman", text_value(human_readable_size(before_bytes))),
    ("afterHuman", text_value(human_readable_size(after_bytes))),
    (
      "reclaimedHuman",
      text_value(human_readable_size(
        before_bytes.saturating_sub(after_bytes),
      )),
    ),
  ]))
}

fn checkpoint_and_vacuum(
  locations: &CursorLocations,
  connection: Connection,
  report_progress: ProgressReporter<'_>,
  stage: i64,
  stage_count: i64,
) -> Result<(), String> {
  report_progress("Flushing pending writes...");
  connection
    .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
    .map_err(|error| format!("could not checkpoint: {error}"))?;
  vacuum_with_progress(
    locations,
    connection,
    report_progress,
    stage,
    stage_count,
    "Compacting storage",
  )?;
  report_progress("Compaction complete");
  Ok(())
}

fn normalize_to_millis(value: i64) -> i64 {
  if value < 1_000_000_000_000 {
    value * 1000
  } else {
    value
  }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
  if needle.is_empty() || haystack.len() < needle.len() {
    return None;
  }
  let last_start = haystack.len() - needle.len();
  for start in 0..=last_start {
    if &haystack[start..start + needle.len()] == needle {
      return Some(start);
    }
  }
  None
}

fn parse_number_after_colon(data: &[u8]) -> Option<i64> {
  let mut index = 0;
  while index < data.len() && (data[index] == b' ' || data[index] == b'\t') {
    index += 1;
  }
  if index < data.len() && data[index] == b':' {
    index += 1;
  }
  while index < data.len() && (data[index] == b' ' || data[index] == b'\t' || data[index] == b'"') {
    index += 1;
  }
  let mut value: i64 = 0;
  let mut digit_count = 0;
  while index < data.len() && data[index].is_ascii_digit() {
    value = value
      .saturating_mul(10)
      .saturating_add((data[index] - b'0') as i64);
    digit_count += 1;
    index += 1;
  }
  if digit_count >= 9 { Some(value) } else { None }
}

fn classify_value_age(data: &[u8], cutoff_millis: i64) -> Option<bool> {
  let mut found_timestamp = false;
  let mut newest_millis = i64::MIN;
  for field in TIMESTAMP_FIELDS {
    let mut search_from = 0;
    while let Some(search_area) = data.get(search_from..) {
      if let Some(found) = find_subsequence(search_area, field) {
        let absolute = search_from
          .saturating_add(found)
          .saturating_add(field.len());
        if let Some(remaining) = data.get(absolute..)
          && let Some(parsed) = parse_number_after_colon(remaining)
        {
          found_timestamp = true;
          let millis = normalize_to_millis(parsed);
          if millis > newest_millis {
            newest_millis = millis;
          }
        }
        search_from = absolute;
      } else {
        break;
      }
    }
  }
  if found_timestamp {
    Some(newest_millis < cutoff_millis)
  } else {
    None
  }
}

fn composer_id_from_key(key: &str) -> Option<&str> {
  let after_prefix = key.split_once(':')?.1;
  match after_prefix.split_once(':') {
    Some((composer_id, _)) => Some(composer_id),
    None => Some(after_prefix),
  }
}

struct AgedKeySelection {
  keys: Vec<String>,
  total_bytes: i64,
}

fn select_aged_conversation_keys(
  connection: &Connection,
  cutoff_millis: i64,
  report_progress: ProgressReporter<'_>,
  cancel: CancelToken<'_>,
  stage: i64,
  stage_count: i64,
) -> Result<AgedKeySelection, String> {
  let composer_data_rows = read_single_integer(
    connection,
    &format!("SELECT COUNT(*) FROM cursorDiskKV WHERE {COMPOSER_DATA_KEY_RANGE}"),
  )
  .unwrap_or(0);
  let child_rows = CHILD_CONVERSATION_KEY_RANGES
    .into_iter()
    .map(|range| {
      read_single_integer(
        connection,
        &format!("SELECT COUNT(*) FROM cursorDiskKV WHERE {range}"),
      )
      .unwrap_or(0)
    })
    .fold(0_i64, i64::saturating_add);
  let total_rows = composer_data_rows.saturating_add(child_rows);
  report_stage(
    report_progress,
    stage,
    stage_count,
    "Scanning conversations",
    Some(0),
    Some(total_rows),
  );

  let mut keys = Vec::new();
  let mut total_bytes: i64 = 0;
  let mut scanned_rows: i64 = 0;
  let mut aged_composers: HashSet<String> = HashSet::new();
  let mut recent_composers: HashSet<String> = HashSet::new();

  {
    let mut statement = connection
      .prepare(&format!(
        "SELECT key, value FROM cursorDiskKV WHERE {COMPOSER_DATA_KEY_RANGE}"
      ))
      .map_err(|error| format!("could not prepare session scan: {error}"))?;
    let mut rows = statement
      .query([])
      .map_err(|error| format!("could not run session scan: {error}"))?;
    while let Some(row) = rows
      .next()
      .map_err(|error| format!("could not read session row: {error}"))?
    {
      scanned_rows = scanned_rows.saturating_add(1);
      if scanned_rows % 5_000 == 0 {
        if cancellation_requested(cancel) {
          return Err(cancelled_error());
        }
        report_stage(
          report_progress,
          stage,
          stage_count,
          "Scanning conversations",
          Some(scanned_rows),
          Some(total_rows),
        );
      }
      let key: String = row
        .get(0)
        .map_err(|error| format!("could not decode session key: {error}"))?;
      let (classification, value_length) = match row.get_ref(1) {
        Ok(ValueRef::Text(bytes)) | Ok(ValueRef::Blob(bytes)) => {
          (classify_value_age(bytes, cutoff_millis), bytes.len() as i64)
        }
        _ => (None, 0),
      };
      if let Some(composer_id) = composer_id_from_key(&key) {
        match classification {
          Some(true) => {
            aged_composers.insert(composer_id.to_string());
          }
          Some(false) => {
            recent_composers.insert(composer_id.to_string());
          }
          None => {}
        }
      }
      if classification == Some(true) {
        keys.push(key);
        total_bytes = total_bytes.saturating_add(value_length);
      }
    }
  }

  let mut unresolved_keys: Vec<String> = Vec::new();
  for range in CHILD_CONVERSATION_KEY_RANGES {
    let mut statement = connection
      .prepare(&format!(
        "SELECT key, octet_length(value) FROM cursorDiskKV WHERE {range}"
      ))
      .map_err(|error| format!("could not prepare conversation scan: {error}"))?;
    let mut rows = statement
      .query([])
      .map_err(|error| format!("could not run conversation scan: {error}"))?;
    while let Some(row) = rows
      .next()
      .map_err(|error| format!("could not read conversation row: {error}"))?
    {
      scanned_rows = scanned_rows.saturating_add(1);
      if scanned_rows % 5_000 == 0 {
        if cancellation_requested(cancel) {
          return Err(cancelled_error());
        }
        report_stage(
          report_progress,
          stage,
          stage_count,
          "Scanning conversations",
          Some(scanned_rows),
          Some(total_rows),
        );
      }
      let key: String = row
        .get(0)
        .map_err(|error| format!("could not decode conversation key: {error}"))?;
      let value_length: i64 = row.get(1).unwrap_or(0);
      let mut aged = false;
      let mut unresolved = false;
      match composer_id_from_key(&key) {
        Some(id) if recent_composers.contains(id) => {}
        Some(id) if aged_composers.contains(id) => {
          aged = true;
        }
        _ => {
          unresolved = true;
        }
      }
      if aged {
        keys.push(key);
        total_bytes = total_bytes.saturating_add(value_length);
      } else if unresolved {
        unresolved_keys.push(key);
      }
    }
  }

  resolve_unassociated_keys(
    connection,
    cutoff_millis,
    &unresolved_keys,
    &mut keys,
    &mut total_bytes,
  )?;

  Ok(AgedKeySelection { keys, total_bytes })
}

fn resolve_unassociated_keys(
  connection: &Connection,
  cutoff_millis: i64,
  unresolved_keys: &[String],
  keys: &mut Vec<String>,
  total_bytes: &mut i64,
) -> Result<(), String> {
  if unresolved_keys.is_empty() {
    return Ok(());
  }
  let mut statement = connection
    .prepare("SELECT value FROM cursorDiskKV WHERE key = ?1")
    .map_err(|error| format!("could not prepare unassociated scan: {error}"))?;
  for key in unresolved_keys {
    let outcome = statement.query_row([key], |row| {
      let classified = match row.get_ref(0) {
        Ok(ValueRef::Text(bytes)) | Ok(ValueRef::Blob(bytes)) => (
          classify_value_age(bytes, cutoff_millis) == Some(true),
          bytes.len() as i64,
        ),
        _ => (false, 0),
      };
      Ok(classified)
    });
    match outcome {
      Ok((true, value_length)) => {
        keys.push(key.clone());
        *total_bytes = total_bytes.saturating_add(value_length);
      }
      Ok((false, _)) | Err(rusqlite::Error::QueryReturnedNoRows) => {}
      Err(error) => return Err(format!("could not scan unassociated key: {error}")),
    }
  }
  Ok(())
}

fn delete_selected_conversation_keys(
  connection: &Connection,
  keys: &[String],
) -> Result<i64, String> {
  if keys.is_empty() {
    return Ok(0);
  }
  connection
    .execute_batch(
      "CREATE TEMP TABLE IF NOT EXISTS delete_conversation_keys (key TEXT PRIMARY KEY)",
    )
    .map_err(|error| format!("could not create temp delete table: {error}"))?;
  connection
    .execute("DELETE FROM delete_conversation_keys", [])
    .map_err(|error| format!("could not reset temp delete table: {error}"))?;

  {
    let transaction = connection
      .unchecked_transaction()
      .map_err(|error| format!("could not begin delete key transaction: {error}"))?;
    {
      let mut insert_statement = transaction
        .prepare("INSERT OR IGNORE INTO delete_conversation_keys (key) VALUES (?1)")
        .map_err(|error| format!("could not prepare delete key insert: {error}"))?;
      for key in keys {
        insert_statement
          .execute([key])
          .map_err(|error| format!("could not insert delete key: {error}"))?;
      }
    }
    transaction
      .commit()
      .map_err(|error| format!("could not commit delete keys: {error}"))?;
  }

  connection
    .execute(
      "DELETE FROM cursorDiskKV WHERE key IN (SELECT key FROM delete_conversation_keys)",
      [],
    )
    .map(|deleted| deleted as i64)
    .map_err(|error| format!("could not delete conversation rows: {error}"))
}

pub fn analyze_deep_clean(
  locations: &CursorLocations,
  cutoff_days: i64,
  report_progress: ProgressReporter<'_>,
  cancel: CancelToken<'_>,
) -> Result<Value, String> {
  let connection = open_for_read(locations)?;
  let cutoff_millis = current_time_millis() - cutoff_days.saturating_mul(86_400_000);
  let selection =
    select_aged_conversation_keys(&connection, cutoff_millis, report_progress, cancel, 1, 1)?;
  let data_bytes = selection.total_bytes;
  let compaction_bytes = compaction_reclaim_bytes(&connection);
  let total_bytes = data_bytes.saturating_add(compaction_bytes);
  Ok(build_object(vec![
    ("cutoffDays", integer_value(cutoff_days)),
    (
      "matchingEntries",
      integer_value(selection.keys.len() as i64),
    ),
    ("estimatedBytes", integer_value(data_bytes)),
    (
      "estimatedHuman",
      text_value(human_readable_size(data_bytes.max(0) as u64)),
    ),
    ("compactionReclaimBytes", integer_value(compaction_bytes)),
    (
      "compactionReclaimHuman",
      text_value(human_readable_size(compaction_bytes.max(0) as u64)),
    ),
    ("totalReclaimBytes", integer_value(total_bytes)),
    (
      "totalReclaimHuman",
      text_value(human_readable_size(total_bytes.max(0) as u64)),
    ),
  ]))
}

pub fn run_deep_clean(
  locations: &CursorLocations,
  cutoff_days: i64,
  report_progress: ProgressReporter<'_>,
  cancel: CancelToken<'_>,
) -> Result<Value, String> {
  let connection = open_for_write(locations)?;
  let before_bytes = file_size(&locations.state_database);
  let cutoff_millis = current_time_millis() - cutoff_days.saturating_mul(86_400_000);
  let stage_count = 3;

  let selection = select_aged_conversation_keys(
    &connection,
    cutoff_millis,
    report_progress,
    cancel,
    1,
    stage_count,
  )?;
  report_stage(
    report_progress,
    2,
    stage_count,
    "Deleting conversations",
    None,
    None,
  );

  let deleted_conversation_rows = delete_selected_conversation_keys(&connection, &selection.keys)?;

  report_stage(
    report_progress,
    3,
    stage_count,
    "Compacting storage",
    None,
    None,
  );
  checkpoint_and_vacuum(locations, connection, report_progress, 3, stage_count)?;

  let after_bytes = file_size(&locations.state_database);
  Ok(build_object(vec![
    (
      "deletedConversationRows",
      integer_value(deleted_conversation_rows),
    ),
    ("beforeBytes", integer_value(before_bytes as i64)),
    ("afterBytes", integer_value(after_bytes as i64)),
    ("beforeHuman", text_value(human_readable_size(before_bytes))),
    ("afterHuman", text_value(human_readable_size(after_bytes))),
    (
      "reclaimedHuman",
      text_value(human_readable_size(
        before_bytes.saturating_sub(after_bytes),
      )),
    ),
  ]))
}

#[cfg(test)]
mod tests {
  use super::{
    classify_value_age, composer_id_from_key, find_subsequence, normalize_to_millis,
    parse_number_after_colon,
  };

  #[test]
  fn accepts_only_values_older_than_cutoff() {
    let old_data = br#"{"createdAt":1700000000000,"lastUpdatedAt":1700000001000}"#;
    let new_data = br#"{"createdAt":1700000000000,"lastUpdatedAt":1900000000000}"#;
    assert_eq!(classify_value_age(old_data, 1_800_000_000_000), Some(true));
    assert_eq!(classify_value_age(new_data, 1_800_000_000_000), Some(false));
    assert_eq!(classify_value_age(b"{}", 1_800_000_000_000), None);
  }

  #[test]
  fn normalizes_seconds_to_milliseconds() {
    assert_eq!(normalize_to_millis(1_000), 1_000_000);
    assert_eq!(normalize_to_millis(1_700_000_000_000), 1_700_000_000_000);
  }

  #[test]
  fn parses_a_number_after_a_colon() {
    assert_eq!(
      parse_number_after_colon(b":1700000000000"),
      Some(1_700_000_000_000)
    );
    assert_eq!(parse_number_after_colon(b":42"), None);
  }

  #[test]
  fn finds_a_subsequence() {
    assert_eq!(find_subsequence(b"hello world", b"world"), Some(6));
    assert_eq!(find_subsequence(b"hello", b"zzz"), None);
  }

  #[test]
  fn extracts_composer_id_from_key() {
    assert_eq!(
      composer_id_from_key("composerData:abc-123"),
      Some("abc-123")
    );
    assert_eq!(
      composer_id_from_key("bubbleId:abc-123:bubble-9"),
      Some("abc-123")
    );
    assert_eq!(
      composer_id_from_key("messageRequestContext:abc-123:ctx"),
      Some("abc-123")
    );
    assert_eq!(composer_id_from_key("noColon"), None);
  }
}
