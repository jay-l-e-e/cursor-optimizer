use std::process::Command;

pub fn force_quit_cursor() -> Result<(), String> {
  #[cfg(target_os = "windows")]
  {
    force_quit_cursor_windows()
  }

  #[cfg(not(target_os = "windows"))]
  {
    force_quit_cursor_unix()
  }
}

#[cfg(target_os = "windows")]
fn force_quit_cursor_windows() -> Result<(), String> {
  use std::os::windows::process::CommandExt;
  const CREATE_NO_WINDOW: u32 = 0x0800_0000;
  Command::new("taskkill")
    .args(["/F", "/IM", "Cursor.exe", "/T"])
    .creation_flags(CREATE_NO_WINDOW)
    .output()
    .map_err(|error| format!("could not run taskkill: {error}"))?;
  Ok(())
}

#[cfg(not(target_os = "windows"))]
fn force_quit_cursor_unix() -> Result<(), String> {
  for process_name in ["Cursor", "cursor"] {
    let _ = Command::new("pkill").arg("-x").arg(process_name).output();
  }
  Ok(())
}

pub fn is_cursor_running() -> bool {
  #[cfg(target_os = "windows")]
  {
    is_cursor_running_windows()
  }

  #[cfg(not(target_os = "windows"))]
  {
    is_cursor_running_unix()
  }
}

#[cfg(target_os = "windows")]
fn is_cursor_running_windows() -> bool {
  use std::os::windows::process::CommandExt;
  const CREATE_NO_WINDOW: u32 = 0x0800_0000;
  let output = Command::new("tasklist")
    .args(["/FI", "IMAGENAME eq Cursor.exe", "/NH"])
    .creation_flags(CREATE_NO_WINDOW)
    .output();
  match output {
    Ok(result) => String::from_utf8_lossy(&result.stdout).contains("Cursor.exe"),
    Err(_) => false,
  }
}

#[cfg(not(target_os = "windows"))]
fn is_cursor_running_unix() -> bool {
  for process_name in ["Cursor", "cursor"] {
    if let Ok(result) = Command::new("pgrep").arg("-x").arg(process_name).output()
      && result.status.success()
      && !result.stdout.is_empty()
    {
      return true;
    }
  }
  false
}
