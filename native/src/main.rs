#![cfg_attr(debug_assertions, allow(dead_code, unused, unused_imports, unused_variables))]
// #![windows_subsystem = "windows"]

use chrome_native_messaging::event_loop;
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::sync::Arc;
use std::path::Path;
use std::path::PathBuf;
use std::time;
use std::collections::HashMap;
use std::process;
use exitcode;
use downloader::verify::Verification;
use downloader::Downloader;
use downloader::Download;
use wfd;
use winapi::um::winuser;
use user32;

mod extension;
mod lib;

#[derive(Deserialize)]
struct Cache {
  as_of: time::SystemTime,
  hashes: HashMap<String, String>
}

#[derive(Deserialize)]
enum Message {
  Get {
    directory: String,
    //cache: Cache
  },
  Set {
    directory: String,
    url: String,
    hash: String,
    name: String,
    filename: String,
  },
  Pick
}

// vulnerable to a race condition since we can't pass in a file to downloader.download()
fn unique_filename(filename: &Path) -> Option<PathBuf> {
  let stem = filename.file_stem()?.to_str()?;
  let extension = filename.extension().and_then(|extension|
    extension.to_str()
  ).map(|extension|
    format!(".{}", extension)
  ).unwrap_or("".to_string());

  let mut final_filename = filename.clone().to_path_buf();
  let mut number: u16 = 0;

  while final_filename.exists() {
    number += 1;
    final_filename.set_file_name(format!("{} ({}){}", stem, number, extension));
  }

  Some(final_filename)
}

fn main() {
  unsafe {
    winuser::SetProcessDpiAwarenessContext(
      winapi::shared::windef::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2
    );
  }

  if std::env::args().len() == 1 {
    let install_result = extension::is_installed(extension::NAME).map_err(|err|
      format!("Unable to check Archiver native extension installation status\n\n{}", err)
    ).and_then(|installed|
      if installed {
        extension::uninstall(extension::NAME).map_or_else(
          |err| Err(format!("Unable to uninstall Archiver native extension\n\n{}", err)),
          |_| Ok("Archiver native extension uninstalled".to_string())
        )
      }
      else {
        extension::install(
          extension::NAME,
          extension::DESCRIPTION,
          extension::ID
        ).map_or_else(
          |err| Err(format!("Unable to install Archiver native extension\n\n{}", err)),
          |_| Ok("Archiver native extension installed".to_string())
        )
      }
    );

    let (message, icon, exit_code) = match install_result {
      Ok(message) => (message, winapi::um::winuser::MB_ICONINFORMATION, exitcode::OK),
      Err(message) => (message, winapi::um::winuser::MB_ICONERROR, exitcode::IOERR)
    };

    // i solemnly swear that these strings contain no zero bytes
    let text = std::ffi::CString::new(message).unwrap();
    let caption = std::ffi::CString::new("Archiver").unwrap();

    unsafe {
      user32::MessageBoxA(
        std::ptr::null_mut(),
        text.as_ptr(),
        caption.as_ptr(),
        winapi::um::winuser::MB_OK | icon
      );
    }

    process::exit(exit_code);
  }

  event_loop(|message| {
    serde_json::from_value::<Message>(message).map_err(|error|
      format!("Invalid message: {}", error)
    ).and_then(|en| {
      match en {
        Message::Pick => {
          Ok(json!({
            "msg": wfd::open_dialog(
              wfd::DialogParams {
                options: wfd::FOS_PICKFOLDERS,
                title: "Pick archive location",
                ..Default::default()
              }
            ).ok().and_then(|result|
              result.selected_file_path.to_str().map(|path|
                path.to_string()
              ) // TODO: display an error and retry if the path is not valid UTF-8?
            ).unwrap_or("".to_string()) // TODO: don't emit anything if the selection was cancelled
          }))
        },
        Message::Get { directory } => {
          lib::hash_files(&directory).map(|names| {
            json!({ "msg": names })
          })
        },
        Message::Set { directory, url, hash, name, filename } => {
          if lib::hash_files(&directory).unwrap_or_default().get(&hash).map(|found_name| found_name.to_string() == name).unwrap_or(false) {
            return Ok(json!({ "msg": { hash: name } }))
          }

          let destination = Path::new(&directory).join(&name);
          fs::create_dir_all(&destination).unwrap();

          let destination_filename = unique_filename(&destination.join(filename)).unwrap();
          let mut downloader = Downloader::builder().download_folder(&destination).build().unwrap();

          Ok(
            downloader.download(&[
              Download::new(&url).file_name(&destination_filename).verify(Arc::new(move |path, _|
                // File::open(path).map(|file|
                  // if base64_hash(file) == hash {
                    Verification::Ok
                  // }
                  // else {
                    // Verification::Failed
                  // }
                // ).unwrap_or(Verification::Failed)
              ))
            ]).map_or_else(|err|
              json!({ "error": err.to_string() }),
              |result| {
                // TODO: error handling
                /*
                result.iter().map(|item|
                  item.as_ref().map_err(|err| err.to_string()).map(|summary| {
                    summary.to_string()
                  })
                ).collect::<Vec<_>>();
                */

              json!({ "msg": { hash: name } })
            })
          )
        }
      }
    })
  });
}
