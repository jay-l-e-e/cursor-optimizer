use rusqlite::Connection;
use tiny_json::Value;

use crate::{
  CancelToken, CursorLocations, ProgressReporter, build_object, cancellation_requested,
  cancelled_error, file_size, human_readable_size, integer_value, open_for_read,
  read_pragma_integer, read_single_integer, report_stage, text_value,
};

const AGENT_BLOB_KEY_RANGE: &str = "key >= 'agentKv:blob:' AND key < 'agentKv:blob;'";

const BREAKDOWN_PREFIXES: [&str; 8] = [
  "agentKv",
  "bubbleId",
  "composerData",
  "checkpointId",
  "codeBlockDiff",
  "ofsContent",
  "codeBlockPartialInlineDiffFates",
  "messageRequestContext",
];

pub fn quick_summary(locations: &CursorLocations) -> Value {
  let database_bytes = file_size(&locations.state_database);
  let write_ahead_log_bytes = file_size(&locations.write_ahead_log);
  let shared_memory_bytes = file_size(&locations.shared_memory);
  let total_bytes = database_bytes + write_ahead_log_bytes + shared_memory_bytes;
  build_object(vec![
    ("databaseBytes", integer_value(database_bytes as i64)),
    (
      "databaseHuman",
      text_value(human_readable_size(database_bytes)),
    ),
    (
      "writeAheadLogBytes",
      integer_value(write_ahead_log_bytes as i64),
    ),
    (
      "writeAheadLogHuman",
      text_value(human_readable_size(write_ahead_log_bytes)),
    ),
    ("totalBytes", integer_value(total_bytes as i64)),
    ("totalHuman", text_value(human_readable_size(total_bytes))),
  ])
}

pub fn gather_overview(
  locations: &CursorLocations,
  report_progress: ProgressReporter<'_>,
  cancel: CancelToken<'_>,
) -> Result<Value, String> {
  report_stage(report_progress, 1, 2, "Reading storage", None, None);
  let connection = open_for_read(locations)?;
  let page_size = read_pragma_integer(&connection, "page_size")?;
  let freelist_count = read_pragma_integer(&connection, "freelist_count")?;
  let reclaimable_bytes = freelist_count.saturating_mul(page_size);
  let agent_blob_count = read_single_integer(
    &connection,
    &format!("SELECT COUNT(*) FROM cursorDiskKV WHERE {AGENT_BLOB_KEY_RANGE}"),
  )?;

  let key_prefixes = gather_prefix_overview(&connection, report_progress, cancel, 2, 2)?;

  Ok(build_object(vec![
    (
      "storage",
      build_object(vec![
        ("reclaimableBytes", integer_value(reclaimable_bytes)),
        (
          "reclaimableHuman",
          text_value(human_readable_size(reclaimable_bytes as u64)),
        ),
      ]),
    ),
    (
      "agentBlobs",
      build_object(vec![("count", integer_value(agent_blob_count))]),
    ),
    ("keyPrefixes", key_prefixes),
  ]))
}

fn gather_prefix_overview(
  connection: &Connection,
  report_progress: ProgressReporter<'_>,
  cancel: CancelToken<'_>,
  stage: i64,
  stage_count: i64,
) -> Result<Value, String> {
  let total_rows =
    read_single_integer(connection, "SELECT COUNT(*) FROM cursorDiskKV").unwrap_or(0);
  report_stage(
    report_progress,
    stage,
    stage_count,
    "Measuring space used",
    Some(0),
    Some(total_rows),
  );

  let prefix_keys: Vec<String> = BREAKDOWN_PREFIXES
    .iter()
    .map(|prefix| format!("{prefix}:"))
    .collect();
  let mut known_totals = vec![(0_i64, 0_i64); BREAKDOWN_PREFIXES.len()];
  let mut other_bytes: i64 = 0;
  let mut other_count: i64 = 0;

  let mut statement = connection
    .prepare("SELECT key, octet_length(value) FROM cursorDiskKV")
    .map_err(|error| format!("could not prepare space scan: {error}"))?;
  let mut rows = statement
    .query([])
    .map_err(|error| format!("could not run space scan: {error}"))?;
  let mut scanned_rows: i64 = 0;
  while let Some(row) = rows
    .next()
    .map_err(|error| format!("could not read space row: {error}"))?
  {
    let key: String = match row.get(0) {
      Ok(value) => value,
      Err(_) => continue,
    };
    let bytes: i64 = row.get(1).unwrap_or(0);
    let mut matched = false;
    for (index, prefix_key) in prefix_keys.iter().enumerate() {
      if key.starts_with(prefix_key.as_str()) {
        if let Some(slot) = known_totals.get_mut(index) {
          slot.0 = slot.0.saturating_add(bytes);
          slot.1 = slot.1.saturating_add(1);
        }
        matched = true;
        break;
      }
    }
    if !matched {
      other_bytes = other_bytes.saturating_add(bytes);
      other_count = other_count.saturating_add(1);
    }
    scanned_rows = scanned_rows.saturating_add(1);
    if scanned_rows % 20_000 == 0 {
      if cancellation_requested(cancel) {
        return Err(cancelled_error());
      }
      report_stage(
        report_progress,
        stage,
        stage_count,
        "Measuring space used",
        Some(scanned_rows),
        Some(total_rows),
      );
    }
  }

  let mut measured: Vec<(String, i64, i64)> = Vec::new();
  for (index, prefix) in BREAKDOWN_PREFIXES.iter().enumerate() {
    if let Some(slot) = known_totals.get(index)
      && slot.1 > 0
    {
      measured.push(((*prefix).to_string(), slot.0, slot.1));
    }
  }
  if other_count > 0 {
    measured.push(("other".to_string(), other_bytes, other_count));
  }

  measured.sort_by_key(|entry| std::cmp::Reverse(entry.1));
  measured.truncate(40);

  let entries = measured
    .into_iter()
    .map(|(prefix, bytes, count)| {
      build_object(vec![
        ("prefix", text_value(prefix)),
        ("rowCount", integer_value(count)),
        ("bytes", integer_value(bytes)),
        (
          "human",
          text_value(human_readable_size(bytes.max(0) as u64)),
        ),
      ])
    })
    .collect();
  report_stage(
    report_progress,
    stage,
    stage_count,
    "Measuring space used",
    Some(scanned_rows),
    Some(total_rows),
  );
  Ok(Value::Array(entries))
}
