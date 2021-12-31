#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrome_native_messaging::{read_input, send_message, Error};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Arc;
use std::path::Path;
use std::path::PathBuf;
use std::time;
use std::collections::{HashMap, HashSet};
use std::process;
use std::io;
use std::panic;
use exitcode;
use downloader::verify::Verification;
use downloader::Downloader;
use downloader::Download;
use wfd;
use winapi::um::winuser;
use user32;

mod extension;
mod lib;

#[allow(dead_code)]
#[derive(Deserialize)]
struct Cache {
  as_of: time::SystemTime,
  hashes: HashMap<String, String>
}

#[derive(Deserialize)]
enum Message {
  Get {
    directory: String,
    hashes: Vec<String>
  },
  Set {
    directory: String,
    url: String,
    hash: String,
    name: String,
    filename: String,
  },
  Pick,
}

#[derive(Serialize)]
#[serde(tag="type", rename_all="lowercase")]
enum Response {
  Get {
    msg: HashMap<String, Option<String>>,
  },
  Suggestions {
    msg: HashSet<String>,
  },
  Pick {
    msg: String,
  },
  Error {
    error: String,
  },
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

// taken from https://github.com/neon64/chrome-native-messaging/blob/master/src/lib.rs#L130-L144
fn handle_panic(info: &std::panic::PanicInfo) {
  let msg = match info.payload().downcast_ref::<&'static str>() {
    Some(s) => *s,
    None => match info.payload().downcast_ref::<String>() {
      Some(s) => &s[..],
      None => "Box<Any>",
    }
  };

  send_message(
    io::stdout(),
    &Response::Error {
      error: format!(
        "Panic:\n{}\n{}\n{}",
        msg,
        info.location().map_or("", |l| l.file()),
        info.location().map_or("".to_string(), |l| format!("{}", l.line()))
      )
    }
  ).unwrap();
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

  panic::set_hook(Box::new(handle_panic));

  loop {
    match read_input(io::stdin()) {
      Ok(message) => {
        let result = serde_json::from_value::<Message>(message).map_err(|error|
          format!("Invalid message: {}", error)
        ).and_then(|en| {
          match en {
            Message::Pick => {
              Ok(Response::Pick {
                msg: wfd::open_dialog(
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
              })
            },
            Message::Get { directory, hashes } => {
              lib::hash_files(&directory).map(|names| {
                send_message(
                  io::stdout(),
                  &Response::Suggestions {
                    msg: names.values().cloned().collect::<HashSet<String>>()
                  }
                ).unwrap();

                Response::Get {
                  msg: hashes.iter().map(|hash|
                    (hash.to_string(), names.get(hash).map(|name| name.to_string()))
                  ).collect::<HashMap<String, Option<String>>>()
                }
              })
            },
            Message::Set { directory, url, hash, name, filename } => {
              if lib::hash_files(&directory).unwrap_or_default().get(&hash).map(|found_name| found_name.to_string() == name).unwrap_or(false) {
                return Ok(Response::Get { msg: HashMap::from([(hash, Some(name))]) })
              }

              let destination = Path::new(&directory).join(&name);
              fs::create_dir_all(&destination).unwrap();

              let destination_filename = unique_filename(&destination.join(filename)).unwrap();
              let mut downloader = Downloader::builder().download_folder(&destination).build().unwrap();

              Ok(
                downloader.download(&[
                  Download::new(&url).file_name(&destination_filename).verify(Arc::new(move |_path, _|
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
                  Response::Error { error: err.to_string() },
                  |_result| {
                    // TODO: error handling
                    /*
                    result.iter().map(|item|
                      item.as_ref().map_err(|err| err.to_string()).map(|summary| {
                        summary.to_string()
                      })
                    ).collect::<Vec<_>>();
                    */
                  send_message(
                    io::stdout(),
                    &Response::Suggestions {
                      msg: HashSet::from([name.clone()])
                    }
                  ).unwrap();

                  Response::Get {
                    msg: HashMap::from([(hash, Some(name))])
                  }
                })
              )
            }
          }
        });

        match result {
          Ok(response) => send_message(io::stdout(), &response).unwrap(),
          Err(error) => send_message(
            io::stdout(),
            &Response::Error {
              error: error
            }
          ).unwrap()
        }
      },
      Err(Error::NoMoreInput) => {
        break;
      },
      Err(error) => {
        send_message(
          io::stdout(),
          &Response::Error {
            error: format!("{}", error)
          }
        ).unwrap()
      },
    }
  }
}
