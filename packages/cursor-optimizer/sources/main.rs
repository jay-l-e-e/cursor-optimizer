#![cfg_attr(
  all(target_os = "windows", not(debug_assertions)),
  windows_subsystem = "windows"
)]

use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use command_group::GroupChild;
use curator::CursorLocations;
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::window::{Icon, Window, WindowBuilder};
use tiny_json::Value;
use wry::http::{Request, Response};
use wry::{WebContext, WebViewBuilder};

mod operations;

include!(concat!(env!("OUT_DIR"), "/embedded_assets.rs"));

const ASSET_SCHEME: &str = "cursoropt";
const DEVELOPMENT_URL: &str = "http://localhost:5173";
const LOADING_HTML: &str = "<!doctype html><html><head><meta charset=\"utf-8\"><style>html,body{height:100%;margin:0}body{display:flex;align-items:center;justify-content:center;background:#f7f7f4;color:#6f6d63;font-family:system-ui,-apple-system,sans-serif;font-size:14px}</style></head><body>Starting the development server</body></html>";

type CancellationRegistry = Arc<Mutex<HashMap<i64, Arc<AtomicBool>>>>;
type DevelopmentServer = Arc<Mutex<Option<GroupChild>>>;

enum UserEvent {
  DispatchRequest(String),
  EmitProgress {
    request_id: i64,
    text: String,
  },
  DevServerReady,
  ResolveRequest {
    request_id: i64,
    succeeded: bool,
    payload_json: String,
  },
  PushCursorStatus(String),
}

fn main() {
  let locations = curator::locate();

  let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
  let proxy = event_loop.create_proxy();

  let window = match WindowBuilder::new()
    .with_title("Cursor Optimizer")
    .with_decorations(false)
    .with_window_icon(application_icon())
    .with_inner_size(LogicalSize::new(1180.0, 820.0))
    .with_min_inner_size(LogicalSize::new(920.0, 640.0))
    .build(&event_loop)
  {
    Ok(window) => window,
    Err(error) => {
      eprintln!("could not create the application window: {error}");
      return;
    }
  };

  let webview_directory = application_paths::application_data_directory().join("webview");
  let _ = fs::create_dir_all(&webview_directory);
  let mut web_context = WebContext::new(Some(webview_directory));
  let ipc_proxy = proxy.clone();

  let mut builder = WebViewBuilder::new_with_web_context(&mut web_context).with_ipc_handler(
    move |request: Request<String>| {
      let _ = ipc_proxy.send_event(UserEvent::DispatchRequest(request.body().clone()));
    },
  );

  if cfg!(debug_assertions) {
    builder = builder.with_html(LOADING_HTML);
  } else {
    builder = builder
      .with_custom_protocol(ASSET_SCHEME.to_string(), handle_asset_request)
      .with_url(format!("{ASSET_SCHEME}://localhost/index.html"));
  }

  let webview = match builder.build(&window) {
    Ok(webview) => webview,
    Err(error) => {
      eprintln!("could not create the webview: {error}");
      return;
    }
  };

  let development_server: DevelopmentServer = Arc::new(Mutex::new(None));
  if cfg!(debug_assertions) {
    let server_proxy = proxy.clone();
    let server_slot = development_server.clone();
    std::thread::spawn(move || {
      let child = prepare_development_server();
      if let Ok(mut guard) = server_slot.lock() {
        *guard = child;
      }
      let _ = server_proxy.send_event(UserEvent::DevServerReady);
    });
  }

  {
    let status_proxy = proxy.clone();
    let status_locations = locations.clone();
    std::thread::spawn(move || {
      let mut last: Option<(bool, bool, String)> = None;
      loop {
        let cursor_running = curator::is_cursor_running();
        let write_ahead_log_present = status_locations
          .as_ref()
          .map(|location| {
            fs::metadata(&location.write_ahead_log)
              .map(|metadata| metadata.len() > 0)
              .unwrap_or(false)
          })
          .unwrap_or(false);
        let fingerprint = database_fingerprint(status_locations.as_ref());
        let current = (cursor_running, write_ahead_log_present, fingerprint);
        if last.as_ref() != Some(&current) {
          let payload = format!(
            "{{\"cursorRunning\":{},\"writeAheadLogPresent\":{},\"databaseFingerprint\":\"{}\"}}",
            current.0, current.1, current.2
          );
          let _ = status_proxy.send_event(UserEvent::PushCursorStatus(payload));
          last = Some(current);
        }
        std::thread::sleep(std::time::Duration::from_millis(1500));
      }
    });
  }

  let cancellation_registry: CancellationRegistry = Arc::new(Mutex::new(HashMap::new()));
  let operation_coordinator =
    operations::OperationCoordinator::new(application_paths::operation_journal_path());

  event_loop.run(move |event, _target, control_flow| {
    *control_flow = ControlFlow::Wait;
    let _ = &web_context;
    match event {
      Event::UserEvent(UserEvent::DispatchRequest(message)) => {
        if !handle_window_request(
          &message,
          &window,
          &proxy,
          &development_server,
          &operation_coordinator,
          control_flow,
        ) {
          dispatch_request(
            &message,
            locations.as_ref(),
            &proxy,
            &cancellation_registry,
            &operation_coordinator,
          );
        }
      }
      Event::UserEvent(UserEvent::EmitProgress { request_id, text }) => {
        let script = format!(
          "window.__progress({request_id},{})",
          Value::Text(text).to_json_string()
        );
        let _ = webview.evaluate_script(&script);
      }
      Event::UserEvent(UserEvent::DevServerReady) => {
        let url =
          std::env::var("CURSOR_OPTIMIZER_DEV_URL").unwrap_or_else(|_| DEVELOPMENT_URL.to_string());
        let _ = webview.load_url(&url);
      }
      Event::UserEvent(UserEvent::ResolveRequest {
        request_id,
        succeeded,
        payload_json,
      }) => {
        let script = format!("window.__resolve({request_id},{succeeded},{payload_json})");
        let _ = webview.evaluate_script(&script);
      }
      Event::UserEvent(UserEvent::PushCursorStatus(payload_json)) => {
        let script = format!("window.__cursorStatus&&window.__cursorStatus({payload_json})");
        let _ = webview.evaluate_script(&script);
      }
      Event::WindowEvent {
        event: WindowEvent::CloseRequested,
        ..
      } if !operation_coordinator.has_running_write() => {
        shutdown_development_server(&development_server);
        *control_flow = ControlFlow::Exit;
      }
      _ => {}
    }
  });
}

fn application_icon() -> Option<Icon> {
  let bytes = include_bytes!("../../../assets/icon.png");
  let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
  let mut reader = decoder.read_info().ok()?;
  let output_size = reader.output_buffer_size()?;
  let mut buffer = vec![0; output_size];
  let info = reader.next_frame(&mut buffer).ok()?;
  let data = &buffer[..info.buffer_size()];
  match info.color_type {
    png::ColorType::Rgba => Icon::from_rgba(data.to_vec(), info.width, info.height).ok(),
    png::ColorType::Rgb => {
      let mut rgba = Vec::with_capacity(data.len() / 3 * 4);
      for chunk in data.chunks_exact(3) {
        rgba.extend([chunk[0], chunk[1], chunk[2], 255]);
      }
      Icon::from_rgba(rgba, info.width, info.height).ok()
    }
    _ => None,
  }
}

fn shutdown_development_server(development_server: &DevelopmentServer) {
  if let Ok(mut guard) = development_server.lock()
    && let Some(child) = guard.as_mut()
  {
    let _ = child.kill();
  }
}

fn resolve_window_request(
  proxy: &EventLoopProxy<UserEvent>,
  request_id: i64,
  payload_json: String,
) {
  let _ = proxy.send_event(UserEvent::ResolveRequest {
    request_id,
    succeeded: true,
    payload_json,
  });
}

fn reject_window_request(proxy: &EventLoopProxy<UserEvent>, request_id: i64, message: &str) {
  let _ = proxy.send_event(UserEvent::ResolveRequest {
    request_id,
    succeeded: false,
    payload_json: Value::Object(vec![(
      "message".to_string(),
      Value::Text(message.to_string()),
    )])
    .to_json_string(),
  });
}

fn resolve_window_null(proxy: &EventLoopProxy<UserEvent>, request_id: i64) {
  resolve_window_request(proxy, request_id, "null".to_string());
}

fn resolve_window_state(proxy: &EventLoopProxy<UserEvent>, request_id: i64, window: &Window) {
  let payload_json = Value::Object(vec![(
    "maximized".to_string(),
    Value::Boolean(window.is_maximized()),
  )])
  .to_json_string();
  resolve_window_request(proxy, request_id, payload_json);
}

fn handle_window_request(
  message: &str,
  window: &Window,
  proxy: &EventLoopProxy<UserEvent>,
  development_server: &DevelopmentServer,
  operation_coordinator: &operations::OperationCoordinator,
  control_flow: &mut ControlFlow,
) -> bool {
  let parsed = match tiny_json::parse(message) {
    Ok(value) => value,
    Err(_) => return false,
  };
  let request_id = parsed
    .get("requestId")
    .and_then(Value::as_integer)
    .unwrap_or(-1);
  match parsed.get("action").and_then(Value::as_text).unwrap_or("") {
    "windowMinimize" => {
      window.set_minimized(true);
      resolve_window_null(proxy, request_id);
      true
    }
    "windowToggleMaximize" => {
      window.set_maximized(!window.is_maximized());
      resolve_window_state(proxy, request_id, window);
      true
    }
    "windowState" => {
      resolve_window_state(proxy, request_id, window);
      true
    }
    "windowDrag" => {
      let _ = window.drag_window();
      resolve_window_null(proxy, request_id);
      true
    }
    "windowClose" => {
      if operation_coordinator.has_running_write() {
        reject_window_request(
          proxy,
          request_id,
          "A task is running. Keep this window open.",
        );
      } else {
        resolve_window_null(proxy, request_id);
        shutdown_development_server(development_server);
        *control_flow = ControlFlow::Exit;
      }
      true
    }
    _ => false,
  }
}

fn prepare_development_server() -> Option<GroupChild> {
  use command_group::CommandGroup;
  use std::time::{Duration, Instant};

  kill_previous_instances();
  free_development_port();
  std::thread::sleep(Duration::from_millis(500));

  let web_directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("sources")
    .join("web");
  let mut command = if cfg!(target_os = "windows") {
    let mut shell = std::process::Command::new("cmd");
    shell.args(["/C", "npm", "run", "development"]);
    shell
  } else {
    let mut runner = std::process::Command::new("npm");
    runner.args(["run", "development"]);
    runner
  };
  command.current_dir(web_directory);
  command.env("CURSOR_OPTIMIZER_PID", std::process::id().to_string());
  let child = command.group_spawn().ok();

  let deadline = Instant::now() + Duration::from_secs(60);
  while Instant::now() < deadline {
    if development_server_ready() {
      break;
    }
    std::thread::sleep(Duration::from_millis(150));
  }
  child
}

fn development_server_ready() -> bool {
  use std::net::{SocketAddr, TcpStream};
  use std::time::Duration;

  for candidate in ["127.0.0.1:5173", "[::1]:5173"] {
    if let Ok(address) = candidate.parse::<SocketAddr>()
      && TcpStream::connect_timeout(&address, Duration::from_millis(250)).is_ok()
    {
      return true;
    }
  }
  false
}

fn free_development_port() {
  #[cfg(target_os = "windows")]
  {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let _ = std::process::Command::new("powershell")
      .args([
        "-NoProfile",
        "-Command",
        "Get-NetTCPConnection -LocalPort 5173 -State Listen -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }",
      ])
      .creation_flags(CREATE_NO_WINDOW)
      .output();
  }
  #[cfg(not(target_os = "windows"))]
  {
    let _ = std::process::Command::new("sh")
      .arg("-c")
      .arg("lsof -ti tcp:5173 | xargs -r kill -9")
      .output();
  }
}

fn kill_previous_instances() {
  let self_pid = std::process::id();
  #[cfg(target_os = "windows")]
  {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let _ = std::process::Command::new("taskkill")
      .args([
        "/F",
        "/IM",
        "cursor-optimizer.exe",
        "/FI",
        &format!("PID ne {self_pid}"),
      ])
      .creation_flags(CREATE_NO_WINDOW)
      .output();
  }
  #[cfg(not(target_os = "windows"))]
  {
    if let Ok(output) = std::process::Command::new("pgrep")
      .arg("-x")
      .arg("cursor-optimizer")
      .output()
      && let Ok(text) = String::from_utf8(output.stdout)
    {
      for line in text.lines() {
        if let Ok(pid) = line.trim().parse::<u32>()
          && pid != self_pid
        {
          let _ = std::process::Command::new("kill")
            .arg(pid.to_string())
            .output();
        }
      }
    }
  }
}

fn handle_asset_request(
  _webview_id: &str,
  request: Request<Vec<u8>>,
) -> Response<Cow<'static, [u8]>> {
  let raw_path = request.uri().path();
  let normalized = raw_path.trim_start_matches('/');
  let normalized = if normalized.is_empty() {
    "index.html"
  } else {
    normalized
  };
  for (asset_path, content_type, bytes) in EMBEDDED_ASSETS {
    if *asset_path == normalized {
      return Response::builder()
        .status(200)
        .header("Content-Type", *content_type)
        .header("Access-Control-Allow-Origin", "*")
        .body(Cow::Borrowed(*bytes))
        .unwrap_or_else(|_| Response::new(Cow::Borrowed(*bytes)));
    }
  }
  Response::builder()
    .status(404)
    .body(Cow::Owned(Vec::new()))
    .unwrap_or_else(|_| Response::new(Cow::Owned(Vec::new())))
}

fn dispatch_request(
  message: &str,
  locations: Option<&CursorLocations>,
  proxy: &EventLoopProxy<UserEvent>,
  cancellation_registry: &CancellationRegistry,
  operation_coordinator: &operations::OperationCoordinator,
) {
  let parsed = match tiny_json::parse(message) {
    Ok(value) => value,
    Err(_) => return,
  };
  let request_id = parsed
    .get("requestId")
    .and_then(Value::as_integer)
    .unwrap_or(-1);
  let action = parsed
    .get("action")
    .and_then(Value::as_text)
    .unwrap_or("")
    .to_string();
  let params = parsed.get("params").cloned().unwrap_or(Value::Null);

  if action == "operationStatus" {
    let _ = proxy.send_event(UserEvent::ResolveRequest {
      request_id,
      succeeded: true,
      payload_json: operation_coordinator.status_value().to_json_string(),
    });
    return;
  }

  if action == "cancel" {
    let target = params
      .get("targetId")
      .and_then(Value::as_integer)
      .unwrap_or(-1);
    if let Ok(registry) = cancellation_registry.lock()
      && let Some(token) = registry.get(&target)
    {
      token.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    let _ = proxy.send_event(UserEvent::ResolveRequest {
      request_id,
      succeeded: true,
      payload_json: "null".to_string(),
    });
    return;
  }

  let access = operations::classify_action(&action);
  if access != operations::OperationAccess::Neutral && locations.is_none() {
    let _ = proxy.send_event(UserEvent::ResolveRequest {
      request_id,
      succeeded: false,
      payload_json: Value::Object(vec![(
        "message".to_string(),
        Value::Text("Could not locate Cursor storage.".to_string()),
      )])
      .to_json_string(),
    });
    return;
  }
  if access == operations::OperationAccess::Write && curator::is_cursor_running() {
    let _ = proxy.send_event(UserEvent::ResolveRequest {
      request_id,
      succeeded: false,
      payload_json: Value::Object(vec![(
        "message".to_string(),
        Value::Text("Cursor is still running. Close it first.".to_string()),
      )])
      .to_json_string(),
    });
    return;
  }
  if action == "deepCleanRun" && read_days(&params).is_err() {
    let _ = proxy.send_event(UserEvent::ResolveRequest {
      request_id,
      succeeded: false,
      payload_json: Value::Object(vec![(
        "message".to_string(),
        Value::Text("Please enter at least 1 day.".to_string()),
      )])
      .to_json_string(),
    });
    return;
  }
  let ticket = match operation_coordinator.begin(request_id, &action, locations) {
    Ok(ticket) => ticket,
    Err(message) => {
      let _ = proxy.send_event(UserEvent::ResolveRequest {
        request_id,
        succeeded: false,
        payload_json: Value::Object(vec![("message".to_string(), Value::Text(message))])
          .to_json_string(),
      });
      return;
    }
  };

  let cancel = Arc::new(AtomicBool::new(false));
  if let Ok(mut registry) = cancellation_registry.lock() {
    registry.insert(request_id, cancel.clone());
  }
  let locations = locations.cloned();
  let proxy = proxy.clone();
  let registry = cancellation_registry.clone();
  let operation_coordinator = operation_coordinator.clone();

  std::thread::spawn(move || {
    let logger_proxy = proxy.clone();
    let progress_coordinator = operation_coordinator.clone();
    let progress_locations = locations.clone();
    let report = move |text: &str| {
      progress_coordinator.update_progress(request_id, text, progress_locations.as_ref());
      let _ = logger_proxy.send_event(UserEvent::EmitProgress {
        request_id,
        text: text.to_string(),
      });
    };
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
      execute_action(&action, &params, locations.as_ref(), &report, &cancel)
    }))
    .unwrap_or_else(|_| Err("Something went wrong. Please try again.".to_string()));
    if let Ok(mut guard) = registry.lock() {
      guard.remove(&request_id);
    }
    let succeeded = outcome.is_ok();
    ticket.finish(succeeded);
    let resolve = match outcome {
      Ok(value) => UserEvent::ResolveRequest {
        request_id,
        succeeded: true,
        payload_json: value.to_json_string(),
      },
      Err(message) => UserEvent::ResolveRequest {
        request_id,
        succeeded: false,
        payload_json: Value::Object(vec![("message".to_string(), Value::Text(message))])
          .to_json_string(),
      },
    };
    let _ = proxy.send_event(resolve);
  });
}

fn require_cursor_closed() -> Result<(), String> {
  if curator::is_cursor_running() {
    Err("Cursor is still running. Close it first.".to_string())
  } else {
    Ok(())
  }
}

fn require_locations(locations: Option<&CursorLocations>) -> Result<&CursorLocations, String> {
  locations.ok_or_else(|| "Could not locate Cursor storage.".to_string())
}

fn execute_action(
  action: &str,
  params: &Value,
  locations: Option<&CursorLocations>,
  report: &dyn Fn(&str),
  cancel: &AtomicBool,
) -> Result<Value, String> {
  match action {
    "initialize" => build_initialize_response(locations),
    "cursorStatus" => build_cursor_status(locations),
    "quickSummary" => Ok(curator::quick_summary(require_locations(locations)?)),
    "storageEstimate" => Ok(curator::storage_estimate(require_locations(locations)?)),
    "openDatabaseDirectory" => {
      open_database_directory(require_locations(locations)?);
      Ok(Value::Null)
    }
    "revealPath" => {
      if let Some(path) = params.get("path").and_then(Value::as_text) {
        reveal_path(path);
      }
      Ok(Value::Null)
    }
    "browseDirectories" => Ok(curator::browse_directories(
      require_locations(locations)?,
      params.get("path").and_then(Value::as_text),
    )),
    "forceQuitCursor" => {
      curator::force_quit_cursor()?;
      std::thread::sleep(std::time::Duration::from_millis(800));
      build_initialize_response(locations)
    }
    "overview" => curator::gather_overview(require_locations(locations)?, report, cancel),
    "lightCleanAnalyze" => {
      curator::analyze_light_clean(require_locations(locations)?, report, cancel)
    }
    "lightCleanRun" => {
      require_cursor_closed()?;
      curator::run_light_clean(require_locations(locations)?, report, cancel)
    }
    "deepCleanAnalyze" => {
      let days = read_days(params)?;
      curator::analyze_deep_clean(require_locations(locations)?, days, report, cancel)
    }
    "deepCleanRun" => {
      require_cursor_closed()?;
      let days = read_days(params)?;
      curator::run_deep_clean(require_locations(locations)?, days, report, cancel)
    }
    "toolIntegrityCheck" => curator::tool_integrity_check(require_locations(locations)?, report),
    "toolCheckpoint" => {
      require_cursor_closed()?;
      curator::tool_checkpoint(require_locations(locations)?, report)
    }
    "toolVacuum" => {
      require_cursor_closed()?;
      curator::tool_vacuum(require_locations(locations)?, report)
    }
    "toolAnalyze" => {
      require_cursor_closed()?;
      curator::tool_analyze(require_locations(locations)?, report)
    }
    "toolFlushDatabase" => {
      require_cursor_closed()?;
      curator::tool_flush_database(require_locations(locations)?, report)
    }
    "toolBackup" => curator::tool_backup(
      require_locations(locations)?,
      params.get("backupDirectory").and_then(Value::as_text),
      params.get("backupFileName").and_then(Value::as_text),
      params.get("compressionLevel").and_then(Value::as_text),
      report,
    ),
    "recoverOperations" => {
      require_cursor_closed()?;
      recover_operations(require_locations(locations)?, report)
    }
    other => Err(format!("Unrecognized request: {other}")),
  }
}

fn recover_operations(locations: &CursorLocations, report: &dyn Fn(&str)) -> Result<Value, String> {
  report("Recovering from interrupted task");
  let integrity = curator::tool_integrity_check(locations, report)?;
  let healthy = integrity
    .get("healthy")
    .and_then(Value::as_boolean)
    .unwrap_or(false);
  if !healthy {
    return Err("Recovery could not complete automatically. Please try again.".to_string());
  }
  let checkpoint = curator::tool_checkpoint(locations, report)?;
  let vacuum = curator::tool_vacuum(locations, report)?;
  Ok(Value::Object(vec![
    ("integrity".to_string(), integrity),
    ("checkpoint".to_string(), checkpoint),
    ("vacuum".to_string(), vacuum),
  ]))
}

fn open_database_directory(locations: &CursorLocations) {
  #[cfg(target_os = "windows")]
  {
    let _ = std::process::Command::new("explorer")
      .arg(format!("/select,{}", locations.state_database.display()))
      .spawn();
  }
  #[cfg(target_os = "macos")]
  {
    let _ = std::process::Command::new("open")
      .arg("-R")
      .arg(&locations.state_database)
      .spawn();
  }
  #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
  {
    let _ = std::process::Command::new("xdg-open")
      .arg(&locations.global_storage_directory)
      .spawn();
  }
}

fn reveal_path(path: &str) {
  let target = std::path::Path::new(path);
  #[cfg(target_os = "windows")]
  {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    if target.is_file() {
      let _ = std::process::Command::new("explorer")
        .arg(format!("/select,{}", target.display()))
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
    } else if let Some(parent) = target.parent().filter(|directory| directory.is_dir()) {
      let _ = std::process::Command::new("explorer")
        .arg(parent)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
    }
  }
  #[cfg(target_os = "macos")]
  {
    if target.is_file() {
      let _ = std::process::Command::new("open")
        .arg("-R")
        .arg(target)
        .spawn();
    } else if let Some(parent) = target.parent().filter(|directory| directory.is_dir()) {
      let _ = std::process::Command::new("open").arg(parent).spawn();
    }
  }
  #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
  {
    if let Some(parent) = target.parent().filter(|directory| directory.is_dir()) {
      let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
    }
  }
}

fn read_days(params: &Value) -> Result<i64, String> {
  let days = params
    .get("days")
    .and_then(Value::as_integer)
    .ok_or_else(|| "Please enter a valid number of days.".to_string())?;
  if days < 1 {
    return Err("Please enter at least 1 day.".to_string());
  }
  Ok(days)
}

fn database_fingerprint(locations: Option<&CursorLocations>) -> String {
  let Some(location) = locations else {
    return String::new();
  };
  let describe = |path: &std::path::Path| -> (u64, u128) {
    fs::metadata(path)
      .map(|metadata| {
        let modified = metadata
          .modified()
          .ok()
          .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
          .map(|elapsed| elapsed.as_millis())
          .unwrap_or(0);
        (metadata.len(), modified)
      })
      .unwrap_or((0, 0))
  };
  let (database_bytes, database_modified) = describe(&location.state_database);
  format!("{database_bytes}:{database_modified}")
}

fn build_cursor_status(locations: Option<&CursorLocations>) -> Result<Value, String> {
  let cursor_running = curator::is_cursor_running();
  let write_ahead_log_present = locations
    .map(|location| {
      fs::metadata(&location.write_ahead_log)
        .map(|metadata| metadata.len() > 0)
        .unwrap_or(false)
    })
    .unwrap_or(false);
  Ok(Value::Object(vec![
    ("cursorRunning".to_string(), Value::Boolean(cursor_running)),
    (
      "writeAheadLogPresent".to_string(),
      Value::Boolean(write_ahead_log_present),
    ),
    (
      "databaseFingerprint".to_string(),
      Value::Text(database_fingerprint(locations)),
    ),
  ]))
}

fn build_initialize_response(locations: Option<&CursorLocations>) -> Result<Value, String> {
  let cursor_running = curator::is_cursor_running();
  let version = env!("CARGO_PKG_VERSION");
  match locations {
    Some(locations) => Ok(Value::Object(vec![
      (
        "databasePath".to_string(),
        Value::Text(locations.state_database.display().to_string()),
      ),
      (
        "baseDirectory".to_string(),
        Value::Text(locations.base_directory.display().to_string()),
      ),
      (
        "databaseExists".to_string(),
        Value::Boolean(locations.database_exists()),
      ),
      ("cursorRunning".to_string(), Value::Boolean(cursor_running)),
      ("version".to_string(), Value::Text(version.to_string())),
    ])),
    None => Ok(Value::Object(vec![
      ("databasePath".to_string(), Value::Null),
      ("baseDirectory".to_string(), Value::Null),
      ("databaseExists".to_string(), Value::Boolean(false)),
      ("cursorRunning".to_string(), Value::Boolean(cursor_running)),
      ("version".to_string(), Value::Text(version.to_string())),
    ])),
  }
}
