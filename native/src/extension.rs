use std::fs::File;
use std::path::Path;
use std::io::BufReader;
use std::io;
use std::io::BufWriter;
use std::collections::HashMap;
use serde_json::json;
use winreg;

pub const NAME: &str = "com.dagwaging.archive";
pub const DESCRIPTION: &str = "Simple archiver extension for drawthreads";
pub const ID: &str = "fdnmnpnjacfjphfmhlfgjpmkimbekmnd";
pub const NATIVE_MESSAGING_REGISTRY_KEY: &str = "Software\\Google\\Chrome\\NativeMessagingHosts\\";
pub const HOST_PATH: &str = "host.json";

pub fn is_installed(name: &str) -> Result<bool, io::Error> {
  let file = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).open_subkey(
    format!("{}{}", NATIVE_MESSAGING_REGISTRY_KEY, name)
  ).and_then(|key|
    key.get_value("")
  ).and_then(|value: String|
    File::open(value)
  ).map_or_else(|err|
    match err.kind() {
      io::ErrorKind::NotFound => Ok(None),
      _ => Err(err)
    },
    |file| Ok(Some(file))
  )?;

  let exe_path = std::env::current_exe()?;

  Ok(
    file.and_then(|file|
      serde_json::from_reader::<_, HashMap<String, serde_json::Value>>(BufReader::new(file)).ok()
    ).map(|host_manifest|
      host_manifest.get("path").and_then(|path|
        path.as_str()
      ).zip(exe_path.to_str()).map(|(string, exe_path)|
        string.to_string() == exe_path
      )
    ).flatten().unwrap_or(false)
  )
}

pub fn install(name: &str, description: &str, extension_id: &str) -> Result<(), io::Error> {
  let exe_path = &std::env::current_exe()?;

  let file = File::create(HOST_PATH)?;

  serde_json::to_writer(BufWriter::new(file), &json!({
    "name": name,
    "description": description,
    "path": exe_path,
    "type": "stdio",
    "allowed_origins": [format!("chrome-extension://{}/", extension_id)]
  }))?;

  let (key, _) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).create_subkey(
    &Path::new(&format!("{}{}", NATIVE_MESSAGING_REGISTRY_KEY, name))
  )?;

  // not really anything reasonable we can do if the path isn't valid UTF-8 since it has to go into json...
  let host_filename = &exe_path.parent().unwrap().join(HOST_PATH).to_str().unwrap().to_string();

  key.set_value("".to_string(), host_filename)?;

  Ok(())
}

pub fn uninstall(name: &str) -> Result<(), io::Error> {
  if Path::new(HOST_PATH).exists() {
    std::fs::remove_file(HOST_PATH)?;
  }

  winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).delete_subkey(
    format!("{}{}", NATIVE_MESSAGING_REGISTRY_KEY, name)
  )?;

  Ok(())
}
