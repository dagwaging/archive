use rayon::prelude::*;
use std::io::BufReader;
use std::io::BufWriter;
use std::path::Path;
use std::time;
use std::io::Read;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use md5::Digest;

pub fn update_cache(cache: &HashMap<String, HashMap<String, String>>, cache_as_of: &Option<time::SystemTime>, directory: &Path) -> HashMap<String, Option<HashMap<String, Option<String>>>> {
  let mut changes = HashMap::<String, Option<HashMap<String, Option<String>>>>::new();

  let directory_modified = directory.metadata().ok().and_then(|metadata|
    metadata.modified().ok()
  ).zip(*cache_as_of).map_or(true, |(modified, cache_modified)|
    modified > cache_modified
  );

  let subdirectories = if directory_modified {
    let subdirectories: HashSet<String> = directory.read_dir().unwrap().filter_map(|child| child.ok()).filter(|child|
      child.file_type().map_or(false, |file_type|
        file_type.is_dir()
      )
    ).map(|child| child.file_name().into_string().unwrap()).collect();
    
    subdirectories.iter().for_each(|subdirectory| {
      if !cache.contains_key(subdirectory) {
        changes.insert(subdirectory.to_string(), Some(HashMap::<String, Option<String>>::new()));
      }
    });

    cache.iter().for_each(|(subdirectory, _)| {
      if !subdirectories.contains(subdirectory) {
        changes.insert(subdirectory.to_string(), None);
      }
    });

    subdirectories
  }
  else {
    cache.keys().cloned().collect()
  };

  for subdirectory in subdirectories {
    let subdirectory_path = directory.join(&subdirectory);

    let subdirectory_modified = subdirectory_path.metadata().ok().and_then(|metadata|
      metadata.modified().ok()
    ).zip(*cache_as_of).map_or(true, |(modified, cache_modified)|
      modified > cache_modified
    );

    if subdirectory_modified {
      let default = HashMap::<String, String>::new();
      let cached_files = cache.get(&subdirectory).unwrap_or(&default);

      if !changes.contains_key(&subdirectory) {
        changes.insert(subdirectory.to_string(), Some(HashMap::<String, Option<String>>::new()));
      }

      let subdirectory_changes = changes.get_mut(&subdirectory).unwrap().as_mut().unwrap();

      let files: HashSet<String> = subdirectory_path.read_dir().unwrap().filter_map(|child| child.ok()).filter(|child|
        child.file_type().map_or(false, |file_type|
          file_type.is_file()
        )
      ).map(|child| child.file_name().into_string().unwrap()).collect();

      files.par_iter().filter(|file|
        !cached_files.contains_key(*file)
      ).map(|file|
        (file, base64_hash(File::open(subdirectory_path.join(file)).unwrap()))
      ).collect::<Vec<_>>().iter().for_each(|(file, hash)| {
        subdirectory_changes.insert(file.to_string(), Some(hash.to_string()));
      });

      cached_files.iter().for_each(|(file, _)| {
        if !files.contains(file) {
          subdirectory_changes.insert(file.to_string(), None);
        }
      });
    }
  }

  changes
}

pub fn hash_files(directory: &String) -> Result<HashMap<String, String>, String> {
  let cache_path = Path::new(&directory).join("cache.json");

  let (mut cache, cache_as_of): (HashMap<String, HashMap<String, String>>, Option<time::SystemTime>) = File::open(&cache_path).map(|cache|
    (
      serde_json::from_reader(BufReader::new(&cache)).unwrap_or_default(),
      cache.metadata().and_then(|metadata| metadata.modified()).ok()
    )
  ).unwrap_or_default();

  update_cache(&cache, &cache_as_of, Path::new(&directory)).iter().for_each(|(subdirectory, change)| {
    match change {
      Some(value) => {
        if !cache.contains_key(subdirectory) {
          cache.insert(subdirectory.to_string(), HashMap::new());
        }

        value.iter().for_each(|(file, change)| {
          match change {
            Some(hash) => {
              cache.get_mut(subdirectory).unwrap().insert(file.to_string(), hash.to_string());
            },
            None => {
              cache.get_mut(subdirectory).unwrap().remove(file);
            }
          }
        });
      },
      None => {
        cache.remove(subdirectory);
      }
    }
  });

  File::create(cache_path).ok().map(|cache_file|
    serde_json::to_writer(BufWriter::new(cache_file), &cache)
  );

  Ok(
    cache.iter().map(|(subdirectory, files)|
      files.iter().map(|(_, hash)|
        (hash.to_string(), subdirectory.to_string())
      )
    ).flatten().collect::<HashMap<String, String>>()
  )
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
