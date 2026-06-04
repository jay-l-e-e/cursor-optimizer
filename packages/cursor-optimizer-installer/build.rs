use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

const ELEVATED_MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="requireAdministrator" uiAccess="false" />
      </requestedPrivileges>
    </security>
  </trustInfo>
</assembly>
"#;

const INVOKER_MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false" />
      </requestedPrivileges>
    </security>
  </trustInfo>
</assembly>
"#;

fn main() {
  #[cfg(target_os = "windows")]
  {
    let manifest = if env::var("PROFILE").as_deref() == Ok("release") {
      ELEVATED_MANIFEST
    } else {
      INVOKER_MANIFEST
    };
    let mut resource = winresource::WindowsResource::new();
    resource.set_icon("../../assets/icon.ico");
    resource.set_manifest(manifest);
    if let Err(error) = resource.compile() {
      println!("cargo:warning=could not compile Windows resources: {error}");
    }
  }
  #[cfg(not(target_os = "windows"))]
  {
    let _ = ELEVATED_MANIFEST;
    let _ = INVOKER_MANIFEST;
  }

  let Ok(output_value) = env::var("OUT_DIR") else {
    return;
  };
  let output_directory = PathBuf::from(output_value);

  write_embedded_assets(&output_directory);
  write_embedded_binary(&output_directory);
}

fn write_embedded_assets(output_directory: &Path) {
  let Ok(manifest_value) = env::var("CARGO_MANIFEST_DIR") else {
    return;
  };
  let manifest_directory = PathBuf::from(manifest_value);
  let web_distribution = manifest_directory.join("../../distributions/web-installer");
  let generated_file = output_directory.join("embedded_assets.rs");

  println!("cargo:rerun-if-changed={}", web_distribution.display());

  let mut entries = Vec::new();
  if web_distribution.is_dir() {
    collect_files(&web_distribution, &web_distribution, &mut entries);
  }

  let mut generated = String::new();
  generated.push_str("pub static EMBEDDED_ASSETS: &[(&str, &str, &[u8])] = &[\n");
  for (relative_path, absolute_path) in entries {
    let content_type = content_type_for(&relative_path);
    let forward_slashed = absolute_path.display().to_string().replace('\\', "/");
    let _ = writeln!(
      generated,
      "  ({:?}, {:?}, include_bytes!(\"{}\")),",
      relative_path, content_type, forward_slashed
    );
  }
  generated.push_str("];\n");

  if let Err(error) = fs::write(&generated_file, generated) {
    println!("cargo:warning=could not write embedded assets: {error}");
  }
}

fn write_embedded_binary(output_directory: &Path) {
  println!("cargo:rerun-if-env-changed=CURSOR_OPTIMIZER_BINARY");
  let generated_file = output_directory.join("embedded_binary.rs");
  let binary_path = env::var("CURSOR_OPTIMIZER_BINARY")
    .ok()
    .map(PathBuf::from)
    .filter(|path| path.is_file());

  let included_path = match binary_path {
    Some(path) => {
      println!("cargo:rerun-if-changed={}", path.display());
      path.display().to_string().replace('\\', "/")
    }
    None => {
      let placeholder = output_directory.join("placeholder_application_binary");
      if let Err(error) = fs::write(&placeholder, [0u8; 0]) {
        println!("cargo:warning=could not write placeholder binary: {error}");
      }
      placeholder.display().to_string().replace('\\', "/")
    }
  };

  let generated = format!(
    "pub static EMBEDDED_APPLICATION_BINARY: &[u8] = include_bytes!(\"{included_path}\");\n"
  );
  if let Err(error) = fs::write(&generated_file, generated) {
    println!("cargo:warning=could not write embedded binary: {error}");
  }
}

fn collect_files(root: &Path, directory: &Path, entries: &mut Vec<(String, PathBuf)>) {
  let read = match fs::read_dir(directory) {
    Ok(read) => read,
    Err(_) => return,
  };
  for item in read.flatten() {
    let path = item.path();
    if path.is_dir() {
      collect_files(root, &path, entries);
    } else if let Ok(relative) = path.strip_prefix(root) {
      let relative_path = relative.to_string_lossy().replace('\\', "/");
      entries.push((relative_path, path.clone()));
    }
  }
}

fn content_type_for(path: &str) -> &'static str {
  let extension = path.rsplit('.').next().unwrap_or("");
  match extension {
    "html" => "text/html; charset=utf-8",
    "css" => "text/css; charset=utf-8",
    "js" | "mjs" => "text/javascript; charset=utf-8",
    "json" => "application/json; charset=utf-8",
    "svg" => "image/svg+xml",
    "woff2" => "font/woff2",
    "woff" => "font/woff",
    "ttf" => "font/ttf",
    "otf" => "font/otf",
    "png" => "image/png",
    "jpg" | "jpeg" => "image/jpeg",
    "gif" => "image/gif",
    "ico" => "image/x-icon",
    "wasm" => "application/wasm",
    "map" => "application/json; charset=utf-8",
    _ => "application/octet-stream",
  }
}
