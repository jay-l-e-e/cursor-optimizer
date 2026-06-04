use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use curator::CursorLocations;
use tiny_json::Value;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OperationAccess {
  Neutral,
  Read,
  Write,
}

#[derive(Clone)]
struct OperationSnapshot {
  request_id: i64,
  action: String,
  title: String,
  access: OperationAccess,
  started_millis: i64,
  progress: String,
}

struct OperationState {
  active_write: Option<OperationSnapshot>,
  active_reads: HashMap<i64, OperationSnapshot>,
  recovery_pending: bool,
}

#[derive(Clone)]
pub struct OperationCoordinator {
  state: Arc<Mutex<OperationState>>,
  journal_path: PathBuf,
}

pub struct OperationTicket {
  coordinator: OperationCoordinator,
  request_id: i64,
  action: String,
  access: OperationAccess,
}

pub fn classify_action(action: &str) -> OperationAccess {
  match action {
    "overview" | "lightCleanAnalyze" | "deepCleanAnalyze" | "toolIntegrityCheck" | "toolBackup"
    | "quickSummary" | "storageEstimate" => OperationAccess::Read,
    "lightCleanRun" | "deepCleanRun" | "toolCheckpoint" | "toolVacuum" | "toolAnalyze"
    | "toolFlushDatabase" | "recoverOperations" => OperationAccess::Write,
    _ => OperationAccess::Neutral,
  }
}

impl OperationCoordinator {
  pub fn new(journal_path: PathBuf) -> Self {
    let recovery_pending = journal_path.exists();
    Self {
      state: Arc::new(Mutex::new(OperationState {
        active_write: None,
        active_reads: HashMap::new(),
        recovery_pending,
      })),
      journal_path,
    }
  }

  pub fn begin(
    &self,
    request_id: i64,
    action: &str,
    locations: Option<&CursorLocations>,
  ) -> Result<OperationTicket, String> {
    let access = classify_action(action);
    let title = operation_title(action);
    let mut state = self
      .state
      .lock()
      .map_err(|_| "Temporarily unavailable. Please try again.".to_string())?;
    if state.recovery_pending && access != OperationAccess::Neutral && action != "recoverOperations"
    {
      return Err("A previous task was interrupted and needs recovery.".to_string());
    }
    match access {
      OperationAccess::Neutral => {}
      OperationAccess::Read => {
        if state.active_write.is_some() {
          return Err("Another task is running. Please wait for it to finish.".to_string());
        }
        state.active_reads.insert(
          request_id,
          OperationSnapshot {
            request_id,
            action: action.to_string(),
            title,
            access,
            started_millis: current_time_millis(),
            progress: String::new(),
          },
        );
      }
      OperationAccess::Write => {
        if state.active_write.is_some() || !state.active_reads.is_empty() {
          return Err("Another task is running. Please wait for it to finish.".to_string());
        }
        let snapshot = OperationSnapshot {
          request_id,
          action: action.to_string(),
          title,
          access,
          started_millis: current_time_millis(),
          progress: String::new(),
        };
        state.active_write = Some(snapshot.clone());
        state.recovery_pending = true;
        drop(state);
        if let Err(message) = self.write_journal(&snapshot, locations) {
          if let Ok(mut failed_state) = self.state.lock() {
            failed_state.active_write = None;
            failed_state.recovery_pending = self.journal_path.exists();
          }
          return Err(message);
        }
      }
    }
    Ok(OperationTicket {
      coordinator: self.clone(),
      request_id,
      action: action.to_string(),
      access,
    })
  }

  pub fn update_progress(&self, request_id: i64, text: &str, locations: Option<&CursorLocations>) {
    let mut snapshot = None;
    if let Ok(mut state) = self.state.lock() {
      if let Some(active_write) = state.active_write.as_mut()
        && active_write.request_id == request_id
      {
        active_write.progress = text.to_string();
        snapshot = Some(active_write.clone());
      }
      if let Some(active_read) = state.active_reads.get_mut(&request_id) {
        active_read.progress = text.to_string();
      }
    }
    if let Some(active_snapshot) = snapshot {
      let _ = self.write_journal(&active_snapshot, locations);
    }
  }

  pub fn finish(&self, request_id: i64, action: &str, access: OperationAccess, succeeded: bool) {
    if let Ok(mut state) = self.state.lock() {
      match access {
        OperationAccess::Neutral => {}
        OperationAccess::Read => {
          state.active_reads.remove(&request_id);
        }
        OperationAccess::Write => {
          if state
            .active_write
            .as_ref()
            .is_some_and(|active_write| active_write.request_id == request_id)
          {
            state.active_write = None;
          }
          if succeeded {
            state.recovery_pending = false;
          } else {
            state.recovery_pending = action != "recoverOperations";
          }
        }
      }
    }
    if access == OperationAccess::Write && succeeded {
      self.remove_journal();
    }
  }

  fn remove_journal(&self) {
    for attempt in 0..5 {
      if !self.journal_path.exists() || fs::remove_file(&self.journal_path).is_ok() {
        return;
      }
      std::thread::sleep(std::time::Duration::from_millis(40 * (attempt + 1)));
    }
  }

  pub fn has_running_write(&self) -> bool {
    self
      .state
      .lock()
      .map(|state| state.active_write.is_some())
      .unwrap_or(false)
  }

  pub fn status_value(&self) -> Value {
    let journal = fs::read_to_string(&self.journal_path)
      .ok()
      .and_then(|content| tiny_json::parse(&content).ok())
      .unwrap_or(Value::Null);
    match self.state.lock() {
      Ok(state) => Value::Object(vec![
        (
          "recoveryPending".to_string(),
          Value::Boolean(state.recovery_pending),
        ),
        (
          "closeBlocked".to_string(),
          Value::Boolean(state.active_write.is_some()),
        ),
        (
          "activeWrite".to_string(),
          state
            .active_write
            .as_ref()
            .map(snapshot_value)
            .unwrap_or(Value::Null),
        ),
        (
          "activeReads".to_string(),
          Value::Array(state.active_reads.values().map(snapshot_value).collect()),
        ),
        ("journal".to_string(), journal),
      ]),
      Err(_) => Value::Object(vec![
        ("recoveryPending".to_string(), Value::Boolean(true)),
        ("closeBlocked".to_string(), Value::Boolean(true)),
        ("activeWrite".to_string(), Value::Null),
        ("activeReads".to_string(), Value::Array(Vec::new())),
        ("journal".to_string(), journal),
      ]),
    }
  }

  fn write_journal(
    &self,
    snapshot: &OperationSnapshot,
    locations: Option<&CursorLocations>,
  ) -> Result<(), String> {
    if let Some(parent) = self.journal_path.parent() {
      fs::create_dir_all(parent)
        .map_err(|error| format!("Could not prepare for this task: {error}"))?;
    }
    let database_bytes = locations
      .map(|location| file_size(&location.state_database))
      .unwrap_or(0);
    let write_ahead_log_bytes = locations
      .map(|location| file_size(&location.write_ahead_log))
      .unwrap_or(0);
    let value = Value::Object(vec![
      ("action".to_string(), Value::Text(snapshot.action.clone())),
      ("title".to_string(), Value::Text(snapshot.title.clone())),
      ("requestId".to_string(), integer_value(snapshot.request_id)),
      (
        "startedMillis".to_string(),
        integer_value(snapshot.started_millis),
      ),
      (
        "updatedMillis".to_string(),
        integer_value(current_time_millis()),
      ),
      (
        "progress".to_string(),
        Value::Text(snapshot.progress.clone()),
      ),
      (
        "databaseBytes".to_string(),
        integer_value(database_bytes as i64),
      ),
      (
        "writeAheadLogBytes".to_string(),
        integer_value(write_ahead_log_bytes as i64),
      ),
    ]);
    fs::write(&self.journal_path, value.to_json_string())
      .map_err(|error| format!("Could not save progress: {error}"))
  }
}

impl OperationTicket {
  pub fn finish(self, succeeded: bool) {
    self
      .coordinator
      .finish(self.request_id, &self.action, self.access, succeeded);
  }
}

fn snapshot_value(snapshot: &OperationSnapshot) -> Value {
  Value::Object(vec![
    ("requestId".to_string(), integer_value(snapshot.request_id)),
    ("action".to_string(), Value::Text(snapshot.action.clone())),
    ("title".to_string(), Value::Text(snapshot.title.clone())),
    (
      "access".to_string(),
      Value::Text(
        match snapshot.access {
          OperationAccess::Neutral => "neutral",
          OperationAccess::Read => "read",
          OperationAccess::Write => "write",
        }
        .to_string(),
      ),
    ),
    (
      "startedMillis".to_string(),
      integer_value(snapshot.started_millis),
    ),
    (
      "progress".to_string(),
      Value::Text(snapshot.progress.clone()),
    ),
    (
      "elapsedSeconds".to_string(),
      integer_value(
        current_time_millis()
          .saturating_sub(snapshot.started_millis)
          .saturating_div(1000),
      ),
    ),
  ])
}

fn operation_title(action: &str) -> String {
  match action {
    "overview" => "Analyzing",
    "lightCleanAnalyze" => "Scanning",
    "deepCleanAnalyze" => "Calculating savings",
    "lightCleanRun" => "Cleaning up",
    "deepCleanRun" => "Deleting old conversations",
    "toolIntegrityCheck" => "Checking integrity",
    "toolCheckpoint" => "Flushing pending writes",
    "toolVacuum" => "Compacting",
    "toolAnalyze" => "Refreshing statistics",
    "toolFlushDatabase" => "Flushing database",
    "quickSummary" => "Reading storage",
    "storageEstimate" => "Checking storage",
    "recoverOperations" => "Recovering",
    _ => "Working",
  }
  .to_string()
}

fn current_time_millis() -> i64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|duration| duration.as_millis() as i64)
    .unwrap_or(0)
}

fn integer_value(value: i64) -> Value {
  Value::Number(value as f64)
}

fn file_size(path: &std::path::Path) -> u64 {
  fs::metadata(path)
    .map(|metadata| metadata.len())
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
  use std::fs;

  use super::{OperationAccess, OperationCoordinator, classify_action};

  #[test]
  fn classifies_database_actions() {
    assert!(matches!(classify_action("overview"), OperationAccess::Read));
    assert!(matches!(
      classify_action("lightCleanRun"),
      OperationAccess::Write
    ));
    assert!(matches!(
      classify_action("windowState"),
      OperationAccess::Neutral
    ));
  }

  #[test]
  fn blocks_write_while_read_is_active() {
    let path = temporary_journal_path("read-active");
    let coordinator = OperationCoordinator::new(path.clone());
    let read_ticket = match coordinator.begin(1, "overview", None) {
      Ok(ticket) => ticket,
      Err(message) => {
        assert!(message.is_empty());
        return;
      }
    };
    assert!(coordinator.begin(2, "toolVacuum", None).is_err());
    read_ticket.finish(true);
    assert!(coordinator.begin(3, "toolVacuum", None).is_ok());
    let _ = fs::remove_file(path);
  }

  #[test]
  fn write_blocks_reads_and_close_until_finished() {
    let path = temporary_journal_path("write-active");
    let coordinator = OperationCoordinator::new(path.clone());
    let write_ticket = match coordinator.begin(1, "toolVacuum", None) {
      Ok(ticket) => ticket,
      Err(message) => {
        assert!(message.is_empty());
        return;
      }
    };
    assert!(coordinator.has_running_write());
    assert!(path.exists());
    assert!(coordinator.begin(2, "overview", None).is_err());
    write_ticket.finish(true);
    assert!(!coordinator.has_running_write());
    assert!(!path.exists());
  }

  fn temporary_journal_path(name: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
      "cursor-optimizer-{name}-{}-{}.json",
      std::process::id(),
      super::current_time_millis()
    ));
    path
  }
}
