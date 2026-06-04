use std::path::{Path, PathBuf};
use std::process::Command;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const LINUX_DESKTOP_ENTRY: &str = "[Desktop Entry]\nType=Application\nName=Cursor Optimizer\nComment=Reclaim gigabytes from Cursor\nExec=cursor-optimizer\nIcon=cursor-optimizer\nCategories=Utility;\nTerminal=false\n";

const LINUX_APP_RUN: &str = "#!/bin/sh\nHERE=\"$(dirname \"$(readlink -f \"$0\")\")\"\nexec \"${HERE}/usr/bin/cursor-optimizer\" \"$@\"\n";

fn main() {
  if let Err(message) = run() {
    eprintln!("packaging failed: {message}");
    std::process::exit(1);
  }
}

fn run() -> Result<(), String> {
  let workspace_root = workspace_root();
  let release_directory = workspace_root.join("distributions").join("release");
  let artifacts_directory = workspace_root.join("artifacts");

  ensure_dependencies(&workspace_root)?;

  println!("building application web bundle...");
  run_npm(
    &workspace_root,
    &["run", "build", "--workspace", "cursor-optimizer-web"],
  )?;

  println!("building application binary...");
  run_command(
    &workspace_root,
    "cargo",
    &["build", "--release", "-p", "cursor-optimizer"],
  )?;

  let application_binary =
    release_directory.join(format!("cursor-optimizer{}", executable_extension()));
  if !application_binary.is_file() {
    return Err(format!(
      "expected application binary not found at {}",
      application_binary.display()
    ));
  }

  std::fs::create_dir_all(&artifacts_directory)
    .map_err(map_io("create the artifacts directory"))?;

  match std::env::consts::OS {
    "windows" => package_windows_installer(
      &workspace_root,
      &release_directory,
      &artifacts_directory,
      &application_binary,
    ),
    "macos" => {
      package_macos_application(&workspace_root, &artifacts_directory, &application_binary)
    }
    "linux" => package_linux_appimage(&workspace_root, &artifacts_directory, &application_binary),
    other => Err(format!("unsupported operating system: {other}")),
  }
}

fn package_windows_installer(
  workspace_root: &Path,
  release_directory: &Path,
  artifacts_directory: &Path,
  application_binary: &Path,
) -> Result<(), String> {
  println!("building installer web bundle...");
  run_npm(
    workspace_root,
    &[
      "run",
      "build",
      "--workspace",
      "cursor-optimizer-installer-web",
    ],
  )?;

  println!("building installer binary...");
  let application_binary_text = application_binary.to_string_lossy().to_string();
  run_command_with_env(
    workspace_root,
    "cargo",
    &["build", "--release", "-p", "cursor-optimizer-installer"],
    &[("CURSOR_OPTIMIZER_BINARY", application_binary_text.as_str())],
  )?;

  let installer_binary = release_directory.join("cursor-optimizer-installer.exe");
  if !installer_binary.is_file() {
    return Err(format!(
      "expected installer binary not found at {}",
      installer_binary.display()
    ));
  }

  let destination = artifacts_directory.join(format!("{}.exe", installer_stem()));
  copy_file(&installer_binary, &destination)?;
  println!("installer written to {}", destination.display());
  Ok(())
}

fn package_macos_application(
  workspace_root: &Path,
  artifacts_directory: &Path,
  application_binary: &Path,
) -> Result<(), String> {
  let bundle_root = workspace_root.join("distributions").join("bundle");
  reset_directory(&bundle_root)?;

  let application_bundle = bundle_root.join("Cursor Optimizer.app");
  let macos_directory = application_bundle.join("Contents").join("MacOS");
  let resources_directory = application_bundle.join("Contents").join("Resources");
  std::fs::create_dir_all(&macos_directory).map_err(map_io("create the application bundle"))?;
  std::fs::create_dir_all(&resources_directory).map_err(map_io("create the application bundle"))?;

  copy_file(
    application_binary,
    &macos_directory.join("cursor-optimizer"),
  )?;

  let icon_source = workspace_root.join("assets").join("icon.png");
  let iconset = bundle_root.join("icon.iconset");
  std::fs::create_dir_all(&iconset).map_err(map_io("create the icon set"))?;
  for size in [16u32, 32, 128, 256, 512] {
    let double = size.saturating_mul(2);
    generate_icon(
      workspace_root,
      &icon_source,
      &iconset.join(format!("icon_{size}x{size}.png")),
      size,
    )?;
    generate_icon(
      workspace_root,
      &icon_source,
      &iconset.join(format!("icon_{size}x{size}@2x.png")),
      double,
    )?;
  }
  let iconset_text = iconset.to_string_lossy().to_string();
  let icns_text = resources_directory
    .join("icon.icns")
    .to_string_lossy()
    .to_string();
  run_command(
    workspace_root,
    "iconutil",
    &["-c", "icns", &iconset_text, "-o", &icns_text],
  )?;

  write_file(
    &application_bundle.join("Contents").join("Info.plist"),
    &macos_information_property_list(),
  )?;

  let application_bundle_text = application_bundle.to_string_lossy().to_string();
  run_command(
    workspace_root,
    "codesign",
    &["--force", "--deep", "--sign", "-", &application_bundle_text],
  )?;

  let background_text = bundle_root
    .join("background.png")
    .to_string_lossy()
    .to_string();
  let background_source_text = workspace_root
    .join("assets")
    .join("dmg-background.png")
    .to_string_lossy()
    .to_string();
  run_command(
    workspace_root,
    "sips",
    &[
      "-z",
      "400",
      "600",
      &background_source_text,
      "--out",
      &background_text,
    ],
  )?;

  let configuration_path = bundle_root.join("appdmg.json");
  write_file(
    &configuration_path,
    &appdmg_configuration(&application_bundle_text, &background_text),
  )?;
  let configuration_text = configuration_path.to_string_lossy().to_string();

  let destination = artifacts_directory.join(format!("{}.dmg", artifact_stem()));
  let _ = std::fs::remove_file(&destination);
  let destination_text = destination.to_string_lossy().to_string();
  run_command(
    workspace_root,
    "npx",
    &["--yes", "appdmg", &configuration_text, &destination_text],
  )?;
  println!("disk image written to {}", destination.display());
  Ok(())
}

fn appdmg_configuration(application_bundle_text: &str, background_text: &str) -> String {
  format!(
    "{{\n  \"title\": \"Cursor Optimizer\",\n  \"background\": \"{background_text}\",\n  \"icon-size\": 128,\n  \"window\": {{ \"size\": {{ \"width\": 600, \"height\": 400 }} }},\n  \"contents\": [\n    {{ \"x\": 150, \"y\": 210, \"type\": \"file\", \"path\": \"{application_bundle_text}\" }},\n    {{ \"x\": 450, \"y\": 210, \"type\": \"link\", \"path\": \"/Applications\" }}\n  ]\n}}\n"
  )
}

fn package_linux_appimage(
  workspace_root: &Path,
  artifacts_directory: &Path,
  application_binary: &Path,
) -> Result<(), String> {
  let bundle_root = workspace_root.join("distributions").join("bundle");
  reset_directory(&bundle_root)?;

  let application_directory = bundle_root.join("AppDir");
  let binary_directory = application_directory.join("usr").join("bin");
  std::fs::create_dir_all(&binary_directory).map_err(map_io("create the AppDir"))?;

  copy_file(
    application_binary,
    &binary_directory.join("cursor-optimizer"),
  )?;
  copy_file(
    &workspace_root.join("assets").join("icon.png"),
    &application_directory.join("cursor-optimizer.png"),
  )?;
  write_file(
    &application_directory.join("cursor-optimizer.desktop"),
    LINUX_DESKTOP_ENTRY,
  )?;
  let application_run = application_directory.join("AppRun");
  write_file(&application_run, LINUX_APP_RUN)?;
  make_executable(workspace_root, &application_run)?;

  let architecture = match std::env::consts::ARCH {
    "aarch64" => "aarch64",
    _ => "x86_64",
  };
  let tool = bundle_root.join("appimagetool");
  let tool_text = tool.to_string_lossy().to_string();
  let download_url = format!(
    "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-{architecture}.AppImage"
  );
  run_command(
    workspace_root,
    "curl",
    &["-fL", "-o", &tool_text, &download_url],
  )?;
  make_executable(workspace_root, &tool)?;

  let destination = artifacts_directory.join(format!("{}.AppImage", artifact_stem()));
  let _ = std::fs::remove_file(&destination);
  let application_directory_text = application_directory.to_string_lossy().to_string();
  let destination_text = destination.to_string_lossy().to_string();
  run_command_with_env(
    workspace_root,
    &tool_text,
    &[
      "--appimage-extract-and-run",
      &application_directory_text,
      &destination_text,
    ],
    &[("ARCH", architecture)],
  )?;
  println!("AppImage written to {}", destination.display());
  Ok(())
}

fn generate_icon(
  workspace_root: &Path,
  source: &Path,
  destination: &Path,
  size: u32,
) -> Result<(), String> {
  let size_text = size.to_string();
  let source_text = source.to_string_lossy().to_string();
  let destination_text = destination.to_string_lossy().to_string();
  run_command(
    workspace_root,
    "sips",
    &[
      "-z",
      &size_text,
      &size_text,
      &source_text,
      "--out",
      &destination_text,
    ],
  )
}

fn make_executable(workspace_root: &Path, path: &Path) -> Result<(), String> {
  let path_text = path.to_string_lossy().to_string();
  run_command(workspace_root, "chmod", &["+x", &path_text])
}

fn artifact_stem() -> String {
  format!(
    "cursor-optimizer-{VERSION}-{}-{}",
    operating_system_label(),
    architecture_label()
  )
}

fn installer_stem() -> String {
  format!(
    "cursor-optimizer-installer-{VERSION}-{}-{}",
    operating_system_label(),
    architecture_label()
  )
}

fn macos_information_property_list() -> String {
  format!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n  <key>CFBundleName</key><string>Cursor Optimizer</string>\n  <key>CFBundleDisplayName</key><string>Cursor Optimizer</string>\n  <key>CFBundleExecutable</key><string>cursor-optimizer</string>\n  <key>CFBundleIdentifier</key><string>kr.co.vendit.cursor-optimizer</string>\n  <key>CFBundleVersion</key><string>{VERSION}</string>\n  <key>CFBundleShortVersionString</key><string>{VERSION}</string>\n  <key>CFBundleIconFile</key><string>icon</string>\n  <key>CFBundlePackageType</key><string>APPL</string>\n  <key>NSHighResolutionCapable</key><true/>\n</dict>\n</plist>\n"
  )
}

fn workspace_root() -> PathBuf {
  let manifest_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
  manifest_directory
    .join("..")
    .join("..")
    .canonicalize()
    .unwrap_or(manifest_directory)
}

fn ensure_dependencies(workspace_root: &Path) -> Result<(), String> {
  if !workspace_root.join("node_modules").is_dir() {
    println!("installing web dependencies...");
    run_npm(workspace_root, &["ci"])?;
  }
  Ok(())
}

fn reset_directory(path: &Path) -> Result<(), String> {
  if path.exists() {
    std::fs::remove_dir_all(path).map_err(map_io("clean the bundle directory"))?;
  }
  std::fs::create_dir_all(path).map_err(map_io("create the bundle directory"))
}

fn copy_file(source: &Path, destination: &Path) -> Result<(), String> {
  std::fs::copy(source, destination)
    .map(|_| ())
    .map_err(|error| {
      format!(
        "could not copy {} to {}: {error}",
        source.display(),
        destination.display()
      )
    })
}

fn write_file(path: &Path, contents: &str) -> Result<(), String> {
  std::fs::write(path, contents)
    .map_err(|error| format!("could not write {}: {error}", path.display()))
}

fn map_io(action: &'static str) -> impl Fn(std::io::Error) -> String {
  move |error| format!("could not {action}: {error}")
}

fn run_npm(working_directory: &Path, arguments: &[&str]) -> Result<(), String> {
  if cfg!(target_os = "windows") {
    let mut combined = vec!["/C", "npm"];
    combined.extend_from_slice(arguments);
    run_command(working_directory, "cmd", &combined)
  } else {
    run_command(working_directory, "npm", arguments)
  }
}

fn run_command(working_directory: &Path, program: &str, arguments: &[&str]) -> Result<(), String> {
  run_command_with_env(working_directory, program, arguments, &[])
}

fn run_command_with_env(
  working_directory: &Path,
  program: &str,
  arguments: &[&str],
  environment: &[(&str, &str)],
) -> Result<(), String> {
  let mut command = Command::new(program);
  command.args(arguments).current_dir(working_directory);
  for (key, value) in environment {
    command.env(key, value);
  }
  let status = command
    .status()
    .map_err(|error| format!("could not start {program}: {error}"))?;
  if status.success() {
    Ok(())
  } else {
    Err(format!("{program} exited with a failure status"))
  }
}

fn executable_extension() -> &'static str {
  if cfg!(target_os = "windows") {
    ".exe"
  } else {
    ""
  }
}

fn operating_system_label() -> &'static str {
  match std::env::consts::OS {
    "macos" => "macos",
    "windows" => "windows",
    "linux" => "linux",
    other => other,
  }
}

fn architecture_label() -> &'static str {
  match std::env::consts::ARCH {
    "x86_64" => "x64",
    "aarch64" => "arm64",
    other => other,
  }
}
