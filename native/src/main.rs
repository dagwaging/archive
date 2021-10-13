use chrome_native_messaging::event_loop;
use std::io::BufReader;
use std::io::BufWriter;
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::sync::Arc;
use std::path::Path;
use std::fs::File;
use std::time;
use md5::Digest;
use std::io::Read;
use std::collections::HashMap;
use std::collections::HashSet;
use std::cell::RefCell;
use downloader::verify::Verification;
use downloader::Downloader;
use downloader::Download;
use rayon::prelude::*;
use wfd;

mod extension;

#[derive(Deserialize)]
enum Message {
  Get {
    directory: String,
    // hashes: HashSet<String>,
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

thread_local!(static TIME: RefCell<time::Instant> = RefCell::new(time::Instant::now()));

fn time(msg: &str) {
  TIME.with(|earlier| {
    let now = time::Instant::now();
    eprintln!(
      "{:?} microseconds elapsed\n{}",
      now.duration_since(*earlier.borrow()).as_micros(),
      msg
    );
    *earlier.borrow_mut() = now;
  });
}

fn base64_hash(mut file: File) -> String {
  let mut digest = md5::Md5::new();
  let mut buffer = [0; 64 * 1024];
  let mut bytes_read = file.read(&mut buffer).unwrap();

  while bytes_read > 0 {
    digest.update(&buffer[0..bytes_read]);
    bytes_read = file.read(&mut buffer).unwrap();
  }

  base64::encode(digest.finalize())
}

fn update_cache(cache: &mut HashMap<String, HashMap<String, String>>, cache_as_of: &Option<time::SystemTime>, directory: &Path) {
  time("Checking for directory changes");

  let directory_modified = directory.metadata().ok().and_then(|metadata|
    metadata.modified().ok()
  ).zip(*cache_as_of).map_or(true, |(modified, cache_modified)|
    modified > cache_modified
  );

  if directory_modified {
    time("Directory changes detected, checking subdirectories");

    let subdirectories: HashSet<String> = directory.read_dir().unwrap().filter_map(|child| child.ok()).filter(|child|
      child.file_type().map_or(false, |file_type|
        file_type.is_dir()
      )
    ).map(|child| child.file_name().into_string().unwrap()).collect();
    
    subdirectories.iter().for_each(|subdirectory| {
      if !cache.contains_key(subdirectory) {
        eprintln!("Added subdirectory '{}'", subdirectory);
        cache.insert(subdirectory.to_string(), HashMap::<String, String>::new());
      }
    });

    cache.retain(|subdirectory, _| {
      if subdirectories.contains(subdirectory) {
        true
      }
      else {
        eprintln!("Removed subdirectory '{}'", subdirectory);
        false
      }
    });
  }
  else {
    time("No directory changes detected");
  }

  for (subdirectory, cached_files) in cache.iter_mut() {
    let subdirectory_path = directory.join(subdirectory);

    let subdirectory_modified = subdirectory_path.metadata().ok().and_then(|metadata|
      metadata.modified().ok()
    ).zip(*cache_as_of).map_or(true, |(modified, cache_modified)|
      modified > cache_modified
    );

    if subdirectory_modified {
      eprintln!("Subdirectory changes detected in '{}', checking files", subdirectory);

      let files: HashSet<String> = subdirectory_path.read_dir().unwrap().filter_map(|child| child.ok()).filter(|child|
        child.file_type().map_or(false, |file_type|
          file_type.is_file()
        )
      ).map(|child| child.file_name().into_string().unwrap()).collect();

      files.iter().for_each(|file| {
        if !cached_files.contains_key(file) {
          eprintln!("Added file '{}/{}'", subdirectory, file);
          cached_files.insert(file.to_string(), "".to_string());
          /*
          File::open(subdirectory_path.join(file)).map(|file_fs|
            cached_files.insert(file.to_string(), base64_hash(file_fs))
          ).unwrap();
          */
        }
      });

      cached_files.retain(|file, _| {
        if files.contains(file) {
          true
        }
        else {
          eprintln!("Removed file '{}/{}'", subdirectory, file);
          false
        }
      });
    }
    else {
      eprintln!("No subdirectory changes detected in '{}'", subdirectory);
    }
  }
}

fn hash_files(directory: &String) -> Result<HashMap<String, String>, String> {
  time("Reading directory");

  fs::read_dir(&directory).map_err(|error|
    format!("Unable to read '{}': {}", directory, error)
  ).map(|files| {
    let cache_path = Path::new(&directory).join("cache.json");

    // read the cache, if possible
    time("Reading cache");

    let (cache, cache_as_of): (HashMap<String, String>, Option<time::SystemTime>) = File::open(&cache_path).map(|cache|
      (
        serde_json::from_reader(BufReader::new(&cache)).unwrap_or_default(),
        cache.metadata().and_then(|metadata| metadata.modified()).ok()
      )
    ).unwrap_or_default();

    // read files and compute checksums if necessary
    time("Enumerating files");

    let data: HashMap<String, (String, String, Option<time::SystemTime>)> = files.filter_map(|path| path.ok()).filter(|path|
      path.file_type().map_or(false, |file_type| file_type.is_dir())
    ).filter_map(move |path|
      fs::read_dir(path.path()).ok().map(move |dir| {
        dir.filter_map(move |p| p.ok().zip(path.file_name().into_string().ok())).filter(|(p, _)|
          p.file_type().map_or(false, |file_type| file_type.is_file())
        )
      })
    ).flatten().par_bridge().filter_map(|(p, name)|
      p.path().to_str().map(|path| path.to_string()).and_then(|path| {
        let modified = p.metadata().and_then(|metadata| metadata.modified()).ok();
        // let modified = Some(time::SystemTime::UNIX_EPOCH);

        cache.get(&path).filter(|_| {
          cache_as_of.zip(modified).map(|(time, modified)|
            modified < time
          ).unwrap_or(false)
        }).map(|hash| {
          // eprintln!("Found '{}' in cache", path);
          hash.to_string()
        }).or_else(|| {
          // eprintln!("Hashing '{}'", path);
          // Some("".to_string())
          File::open(p.path()).ok().map(|file|
            base64_hash(file)
          )
        }).map(|hash| (path, (name, hash, modified)))
      })
    ).collect();

    // eprintln!("Found {} files", data.len());

    time("Updating cache");

    let cache_data: HashMap<&String, &String> = data.iter().map(|(path, (_, hash, _))| (path, hash)).collect();

    // update the cache
    File::create(cache_path).ok().map(|cache|
      serde_json::to_writer(BufWriter::new(cache), &cache_data)
    );

    time("Preparing output");

    let mut entries: Vec<_> = data.into_iter().collect();

    entries.sort_unstable_by_key(|(_, (_, _, modified))| modified.clone());
    entries.iter().map(|(_, (name, hash, _))|
      (hash.to_string(), name.to_string())
    ).collect::<HashMap<String, String>>()
  })
}

fn main() {
  time("Starting");

  if std::env::args().len() == 1 {
    let name = "com.dagwaging.archive";

    if extension::is_installed(name) {
      extension::uninstall(name);
    }
    else {
      extension::install(name, "Simple archiver extension for drawthreads", "bnbdjefgpgplehagifjhjecifmlhhnai");
    }

    /*
    let directory = Path::new("F:\\nyu\\dataset");

    let cache_path = Path::new(&directory).join("cache.json");

    time("Reading cache");

    let (mut cache, cache_as_of): (HashMap<String, HashMap<String, String>>, Option<time::SystemTime>) = File::open(&cache_path).map(|cache|
      (
        serde_json::from_reader(BufReader::new(&cache)).unwrap_or_default(),
        cache.metadata().and_then(|metadata| metadata.modified()).ok()
      )
    ).unwrap_or_default();

    update_cache(&mut cache, &cache_as_of, directory);

    time("Updating cache");

    File::create(cache_path).ok().map(|cache_file|
      serde_json::to_writer(BufWriter::new(cache_file), &cache)
    );

    time("Done");
    */

    return;
  }

  event_loop(|message| {
    time("Got message");

    let result = serde_json::from_value::<Message>(message).map_err(|error|
      format!("Invalid message: {}", error)
    ).and_then(|en| {
      time("Processing message");

      match en {
        Message::Pick => {
          Ok(json!({
            "msg": wfd::open_dialog(
              wfd::DialogParams {
                options: wfd::FOS_PICKFOLDERS,
                title: "Pick archive location",
                ..Default::default()
              }
            ).map(|result|
              result.selected_file_path.to_str().unwrap().to_string()
            ).unwrap_or("".to_string())
          }))
        },
        Message::Get { directory } => {
          hash_files(&directory).map(|names| {
            // output the result
            // eprintln!("Done");
            json!({ "msg": names })
          })
        },
        Message::Set { directory, url, hash, name, filename } => {
          if hash_files(&directory).unwrap_or_default().get(&hash).map(|found_name| found_name.to_string() == name).unwrap_or(false) {
            // eprintln!("File with hash '{}' already exists", hash);
            return Ok(json!({ "msg": { hash: name } }))
          }

          let filename_path = Path::new(&filename);
          let destination = Path::new(&directory).join(&name);
          let mut final_filename = filename.clone();
          let mut number: u16 = 0;

          fs::create_dir_all(&destination).unwrap();

          while destination.join(&final_filename).exists() {
            number += 1;

            final_filename = format!(
              "{} ({}){}",
              filename_path.file_stem().unwrap().to_str().unwrap(),
              number,
              filename_path.extension().map(|extension| ".".to_string() + extension.to_str().unwrap()).unwrap_or("".to_string())
            );
          }

          let mut downloader = Downloader::builder().download_folder(&destination).build().unwrap();

          Ok(
            downloader.download(&[
              Download::new(&url).file_name(Path::new(&final_filename)).verify(Arc::new(move |path, _|
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
              result.iter().map(|item|
                item.as_ref().map_err(|err| err.to_string()).map(|summary| {
                  summary.to_string()
                })
              ).collect::<Vec<_>>();

              json!({ "msg": { hash: name } })
            })
          )
        }
      }
    });

    time("Processed message");

    result
  });

  time("Done");
}
