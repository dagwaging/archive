use archive;
use tempdir::TempDir;
use std::collections::HashMap;
use std::time;
use std::thread;
use std::fs;
use std::io::prelude::*;
use quickcheck::Gen;
use quickcheck::Arbitrary;
use quickcheck_macros::quickcheck;
use md5::Digest;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct LegalPath(String);

impl Arbitrary for LegalPath {
  fn arbitrary(g: &mut Gen) -> LegalPath {
    let s = (1..g.size()).map(|_| g.choose(&['a', 'b', 'c', 'd']).unwrap().to_owned()).collect();

    LegalPath(s)
  }

  fn shrink(&self) -> Box<dyn Iterator<Item = LegalPath>> {
    let chars: Vec<char> = self.0.chars().collect();
    Box::new(chars.shrink().map(|x| LegalPath(x.into_iter().filter(|c| c == &'a' || c == &'b' || c == &'c' || c == &'d').collect::<String>())).filter(|c| c.0.len() > 0))
  }
}

#[quickcheck]
fn test(
  mut cache: HashMap<LegalPath, HashMap<LegalPath, String>>,
  mut changes: HashMap<LegalPath, Option<HashMap<LegalPath, Option<String>>>>
) -> bool {
  let cache: HashMap<String, HashMap<String, String>> = cache.drain().map(|(key, mut value)|
    (
      key.0,
      value.drain().map(|(key, value)|
        (key.0, value)
      ).collect()
    )
  ).collect();

  let changes: HashMap<String, Option<HashMap<String, Option<String>>>> = changes.drain().map(|(key, value)|
    (
      key.0,
      value.map(|mut v|
        v.drain().map(|(key, value)|
          (key.0, value)
        ).collect()
      )
    )
  ).collect();

  let applicable_changes: HashMap<String, Option<HashMap<String, Option<String>>>> = changes.iter().filter(|(subdirectory, change)|
    !(**change == None && !cache.contains_key(*subdirectory))
  ).map(|(subdirectory, change)|
    match change {
      None => (subdirectory.to_string(), change.clone()),
      Some(files) => {
        (
          subdirectory.to_string(),
          Some(
            files.iter().filter(|(file, change)|
              !(**change == None && !cache.get(subdirectory).map(|cached| cached.contains_key(*file)).unwrap_or(false))
            ).map(|(f, c)| (f.to_string(), c.clone().map(|_| "1B2M2Y8AsgTpgAmY7PhCfg==".to_string()))).collect()
          )
        )
      }
    }
  ).collect();

  ["create", "copy_files", "rename_files"].iter().cloned().map(|method| {
    let copy_temp_dir = TempDir::new("").unwrap();
    let copy_directory = copy_temp_dir.path();

    for (subdirectory, change) in &changes {
      match change {
        Some(files) => {
          if !copy_directory.join(&subdirectory).exists() {
            fs::create_dir(copy_directory.join(&subdirectory)).unwrap();
          }

          for (file, change) in files {
            match change {
              Some(_content) => {
                fs::File::create(copy_directory.join(&subdirectory).join(file)).unwrap();
              },
              _ => {}
            }
          }
        },
        _ => {}
      };
    }

    let temp_dir = TempDir::new("").unwrap();
    let directory = temp_dir.path();

    for (subdirectory, files) in &cache {
      fs::create_dir(directory.join(&subdirectory)).unwrap();

      for (file, content) in files {
        fs::File::create(directory.join(&subdirectory).join(file)).unwrap().write_all(content.as_bytes()).unwrap();
      }
    }

    let as_of = time::SystemTime::now();

    thread::sleep(time::Duration::from_millis(1));

    for (subdirectory, change) in &changes {
      match change {
        Some(files) => {
          if !directory.join(&subdirectory).exists() {
            match method {
              "rename_directory" => {
                fs::rename(copy_directory.join(&subdirectory), directory.join(&subdirectory)).unwrap();
              },
              _ => {
                fs::create_dir(directory.join(&subdirectory)).unwrap();
              }
            }
          }

          for (file, change) in files {
            match change {
              Some(_content) => {
                match method {
                  "create" => {
                    fs::File::create(directory.join(&subdirectory).join(file)).unwrap();
                  },
                  "copy_files" => {
                    fs::copy(copy_directory.join(&subdirectory).join(file), directory.join(&subdirectory).join(file)).unwrap();
                  },
                  "rename_files" => {
                    fs::rename(copy_directory.join(&subdirectory).join(file), directory.join(&subdirectory).join(file)).unwrap();
                  },
                  _ => {}
                }
              },
              None => {
                if directory.join(&subdirectory).join(file).exists() {
                  fs::remove_file(directory.join(&subdirectory).join(file)).unwrap();
                }
              }
            }
          }
        },
        None => {
          if directory.join(&subdirectory).exists() {
            fs::remove_dir(directory.join(&subdirectory)).unwrap();
          }
        }
      }
    }

    let result = archive::update_cache(&cache, &Some(as_of), &directory);
    println!("Expected {:?} to equal {:?}", result, applicable_changes);
    result == applicable_changes
  }).all(|result| result)
}
