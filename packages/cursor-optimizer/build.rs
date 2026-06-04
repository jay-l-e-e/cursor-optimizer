use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
  #[cfg(target_os = "windows")]
  {
    let mut resource = winresource::WindowsResource::new();
    resource.set_icon("../../assets/icon.ico");
    if let Err(error) = resource.compile() {
      println!("cargo:warning=could not compile Windows resources: {error}");
    }
  }

  let Ok(manifest_value) = env::var("CARGO_MANIFEST_DIR") else {
    return;
  };
  let Ok(output_value) = env::var("OUT_DIR") else {
    return;
  };
  let manifest_directory = PathBuf::from(manifest_value);
  let web_distribution = manifest_directory.join("../../distributions/web");
  let generated_file = PathBuf::from(output_value).join("embedded_assets.rs");

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
