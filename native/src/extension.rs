use std::fs::File;
use std::path::Path;
use std::io::BufReader;
use std::io::BufWriter;
use std::collections::HashMap;
use serde_json::json;
use winreg;

pub const NAME: &str = "com.dagwaging.archive";
pub const DESCRIPTION: &str = "Simple archiver extension for drawthreads";
pub const ID: &str = "fdnmnpnjacfjphfmhlfgjpmkimbekmnd";
pub const NATIVE_MESSAGING_REGISTRY_KEY: &str = "Software\\Google\\Chrome\\NativeMessagingHosts\\";
pub const HOST_PATH: &str = "host.json";

pub fn is_installed(name: &str) -> bool {
  // TODO: if this fails due to e.kind() == io::ErrorKind::NotFound we should return Some(false)
  // but otherwise we should return None since we weren't able to find out
  winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).open_subkey(
    format!("{}{}", NATIVE_MESSAGING_REGISTRY_KEY, name)
  ).ok().and_then(|key|
    // TODO: if this fails due to e.kind() == io::ErrorKind::NotFound we should return Some(false)
    // but otherwise we should return None since we weren't able to find out
    key.get_value("").ok()
  ).and_then(|value: String|
    File::open(value).ok()
  ).and_then(|file|
    serde_json::from_reader(BufReader::new(file)).ok()
  ).and_then(|host_manifest: HashMap<String, serde_json::Value>|
    host_manifest.get("path").and_then(|path|
      path.as_str().map(|string| string.to_string())
    )
  ).map(|path|
    path == std::env::current_exe().unwrap().to_str().unwrap() // TODO: what if this fails
  ).unwrap_or(false)
}

pub fn install(name: &str, description: &str, extension_id: &str) {
  let exe_path = &std::env::current_exe().unwrap(); // TODO: handle this lol

  File::create(HOST_PATH).ok().map(|host|
    serde_json::to_writer(BufWriter::new(host), &json!({
      "name": name,
      "description": description,
      "path": exe_path, // TODO: use absolute or relative path? if user moves either host.json or exe the extension will break
      "type": "stdio",
      "allowed_origins": [format!("chrome-extension://{}/", extension_id)]
    }))
  );

  let (key, _) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).create_subkey(
    &Path::new(&format!("{}{}", NATIVE_MESSAGING_REGISTRY_KEY, name))
  ).unwrap(); // TODO: also handle these lol

  key.set_value("".to_string(), &exe_path.parent().unwrap().join(HOST_PATH).to_str().unwrap().to_string()).unwrap(); // TODO: handle this
}

pub fn uninstall(name: &str) {
  if Path::new(HOST_PATH).exists() {
    std::fs::remove_file(HOST_PATH).unwrap(); // TODO: you already know
  }

  winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).delete_subkey(
    format!("{}{}", NATIVE_MESSAGING_REGISTRY_KEY, name)
  ).unwrap(); // TODO: same
}
