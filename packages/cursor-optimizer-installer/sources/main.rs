#![cfg_attr(
  all(target_os = "windows", not(debug_assertions)),
  windows_subsystem = "windows"
)]

use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;

use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::window::{Icon, Window, WindowBuilder};
use tiny_json::Value;
use wry::http::{Request, Response};
use wry::{WebContext, WebView, WebViewBuilder};

mod actions;
mod common;
mod plan;
mod platform;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(all(unix, not(target_os = "macos")))]
mod linux;

use actions::InstallerMode;

include!(concat!(env!("OUT_DIR"), "/embedded_assets.rs"));
include!(concat!(env!("OUT_DIR"), "/embedded_binary.rs"));

const ASSET_SCHEME: &str = "cursoropt";

enum UserEvent {
  DispatchRequest(String),
  EmitProgress {
    request_id: i64,
    text: String,
  },
  ResolveRequest {
    request_id: i64,
    succeeded: bool,
    payload_json: String,
  },
}

fn main() {
  let mode = actions::detect_mode();

  let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
  let proxy = event_loop.create_proxy();

  let window = match WindowBuilder::new()
    .with_title(window_title(mode))
    .with_decorations(false)
    .with_window_icon(application_icon())
    .with_inner_size(LogicalSize::new(760.0, 600.0))
    .with_min_inner_size(LogicalSize::new(640.0, 520.0))
    .build(&event_loop)
  {
    Ok(window) => window,
    Err(error) => {
      eprintln!("could not create the installer window: {error}");
      return;
    }
  };

  let webview_directory = std::env::temp_dir().join("CursorOptimizerInstaller");
  let _ = fs::create_dir_all(&webview_directory);
  let mut web_context = WebContext::new(Some(webview_directory.clone()));
  let ipc_proxy = proxy.clone();

  let mut builder = WebViewBuilder::new_with_web_context(&mut web_context).with_ipc_handler(
    move |request: Request<String>| {
      let _ = ipc_proxy.send_event(UserEvent::DispatchRequest(request.body().clone()));
    },
  );

  if let Ok(url) = std::env::var("CURSOR_OPTIMIZER_INSTALLER_DEV_URL") {
    builder = builder.with_url(url);
  } else {
    builder = builder
      .with_custom_protocol(ASSET_SCHEME.to_string(), handle_asset_request)
      .with_url(format!("{ASSET_SCHEME}://localhost/index.html"));
  }

  let webview = match builder.build(&window) {
    Ok(webview) => webview,
    Err(error) => {
      eprintln!("could not create the installer view: {error}");
      return;
    }
  };

  let mut webview_holder: Option<WebView> = Some(webview);
  let mut web_context_holder: Option<WebContext> = Some(web_context);

  event_loop.run(move |event, _target, control_flow| {
    *control_flow = ControlFlow::Wait;
    match event {
      Event::UserEvent(UserEvent::DispatchRequest(message)) => {
        if !handle_window_request(&message, &window, &proxy, control_flow) {
          dispatch_request(&message, mode, &proxy);
        }
      }
      Event::UserEvent(UserEvent::EmitProgress { request_id, text }) => {
        if let Some(ref webview) = webview_holder {
          let script = format!(
            "window.__progress({request_id},{})",
            Value::Text(text).to_json_string()
          );
          let _ = webview.evaluate_script(&script);
        }
      }
      Event::UserEvent(UserEvent::ResolveRequest {
        request_id,
        succeeded,
        payload_json,
      }) => {
        if let Some(ref webview) = webview_holder {
          let script = format!("window.__resolve({request_id},{succeeded},{payload_json})");
          let _ = webview.evaluate_script(&script);
        }
      }
      Event::WindowEvent {
        event: WindowEvent::CloseRequested,
        ..
      } => {
        *control_flow = ControlFlow::Exit;
      }
      Event::LoopDestroyed => {
        drop(webview_holder.take());
        drop(web_context_holder.take());
        cleanup_webview_directory(&webview_directory);
      }
      _ => {}
    }
  });
}

fn cleanup_webview_directory(directory: &PathBuf) {
  std::thread::sleep(std::time::Duration::from_millis(300));
  for _ in 0..10 {
    if fs::remove_dir_all(directory).is_ok() {
      return;
    }
    std::thread::sleep(std::time::Duration::from_millis(500));
  }
}

fn window_title(mode: InstallerMode) -> &'static str {
  match mode {
    InstallerMode::Install => "Install Cursor Optimizer",
    InstallerMode::Uninstall => "Uninstall Cursor Optimizer",
  }
}

fn application_icon() -> Option<Icon> {
  let decoder = png::Decoder::new(std::io::Cursor::new(common::APPLICATION_ICON_PNG));
  let mut reader = decoder.read_info().ok()?;
  let output_size = reader.output_buffer_size()?;
  let mut buffer = vec![0; output_size];
  let info = reader.next_frame(&mut buffer).ok()?;
  let data = buffer.get(..info.buffer_size())?;
  match info.color_type {
    png::ColorType::Rgba => Icon::from_rgba(data.to_vec(), info.width, info.height).ok(),
    png::ColorType::Rgb => {
      let mut rgba = Vec::with_capacity(data.len() / 3 * 4);
      for chunk in data.chunks_exact(3) {
        if let [red, green, blue] = chunk {
          rgba.extend([*red, *green, *blue, 255]);
        }
      }
      Icon::from_rgba(rgba, info.width, info.height).ok()
    }
    _ => None,
  }
}

fn resolve_request(proxy: &EventLoopProxy<UserEvent>, request_id: i64, payload_json: String) {
  let _ = proxy.send_event(UserEvent::ResolveRequest {
    request_id,
    succeeded: true,
    payload_json,
  });
}

fn resolve_null(proxy: &EventLoopProxy<UserEvent>, request_id: i64) {
  resolve_request(proxy, request_id, "null".to_string());
}

fn reject_request(proxy: &EventLoopProxy<UserEvent>, request_id: i64, message: &str) {
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

fn handle_window_request(
  message: &str,
  window: &Window,
  proxy: &EventLoopProxy<UserEvent>,
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
      resolve_null(proxy, request_id);
      true
    }
    "windowDrag" => {
      let _ = window.drag_window();
      resolve_null(proxy, request_id);
      true
    }
    "windowClose" => {
      resolve_null(proxy, request_id);
      *control_flow = ControlFlow::Exit;
      true
    }
    _ => false,
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

fn dispatch_request(message: &str, mode: InstallerMode, proxy: &EventLoopProxy<UserEvent>) {
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

  match action.as_str() {
    "installerInfo" => {
      resolve_request(
        proxy,
        request_id,
        actions::info_value(mode).to_json_string(),
      );
    }
    "browseDirectories" => {
      resolve_request(
        proxy,
        request_id,
        actions::browse_directories_value(&params).to_json_string(),
      );
    }
    "launchApp" => match actions::launch_value(&params) {
      Ok(_) => resolve_null(proxy, request_id),
      Err(message) => reject_request(proxy, request_id, &message),
    },
    "install" | "uninstall" => run_worker(action, params, request_id, proxy.clone()),
    other => reject_request(proxy, request_id, &format!("Unrecognized request: {other}")),
  }
}

fn run_worker(action: String, params: Value, request_id: i64, proxy: EventLoopProxy<UserEvent>) {
  std::thread::spawn(move || {
    let progress_proxy = proxy.clone();
    let report = move |text: &str| {
      let _ = progress_proxy.send_event(UserEvent::EmitProgress {
        request_id,
        text: text.to_string(),
      });
    };
    let outcome = match action.as_str() {
      "install" => actions::install_value(&params, &report),
      "uninstall" => actions::uninstall_value(&params, &report),
      other => Err(format!("Unrecognized request: {other}")),
    };
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
