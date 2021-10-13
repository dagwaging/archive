use std::fs::File;
use std::path::Path;
use std::io::BufReader;
use std::io::BufWriter;
use std::collections::HashMap;
use serde_json::json;
use winreg;

pub fn is_installed(name: &str) -> bool {
  // TODO: if this fails due to e.kind() == io::ErrorKind::NotFound we should return Some(false)
  // but otherwise we should return None since we weren't able to find out
  winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).open_subkey(
    format!("Software\\Google\\Chrome\\NativeMessagingHosts\\{}", name)
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
    path == std::env::current_exe().unwrap().to_str().unwrap()
  ).unwrap_or(false)
}

pub fn install(name: &str, description: &str, extension_id: &str) {
  let exe_path = &std::env::current_exe().unwrap();

  File::create("host.json").ok().map(|host|
    serde_json::to_writer(BufWriter::new(host), &json!({
      "name": name,
      "description": description,
      "path": exe_path, // TODO: use absolute or relative path? if user moves either host.json or exe the extension will break
      "type": "stdio",
      "allowed_origins": [format!("chrome-extension://{}/", extension_id)]
    }))
  );

  let (key, _) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).create_subkey(
    &Path::new(&format!("Software\\Google\\Chrome\\NativeMessagingHosts\\{}", name))
  ).unwrap();

  key.set_value("".to_string(), &exe_path.parent().unwrap().join("host.json").to_str().unwrap().to_string()).unwrap();
}

pub fn uninstall(name: &str) {
  if Path::new("host.json").exists() {
    std::fs::remove_file("host.json").unwrap();
  }

  winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).delete_subkey(
    format!("Software\\Google\\Chrome\\NativeMessagingHosts\\{}", name)
  ).unwrap();
}
