#![feature(test)]

extern crate test;

use std::fs;
use std::io::prelude::*;
use std::collections::HashMap;
use std::time;
use std::path;
use tempdir::TempDir;

fn setup(subdirectories: u16, file_count: u16, file_size: usize, bench: impl FnOnce(&path::Path, &HashMap::<String, HashMap<String, String>>)) {
  let mut cache = HashMap::<String, HashMap<String, String>>::new();

  let temp_dir = TempDir::new("").unwrap();
  let directory = temp_dir.path();
  let data = vec![32; file_size];

  (0..subdirectories).for_each(|_| {
    let subdirectory = TempDir::new_in(directory, "").unwrap().into_path();
    let mut files = HashMap::<String, String>::new();

    (0..file_count).for_each(|n| {
      fs::File::create(subdirectory.join(format!("{}", n))).unwrap().write_all(&data).unwrap();
      files.insert(format!("{}", n), "".to_string());
    });

    cache.insert(subdirectory.file_name().unwrap().to_str().unwrap().to_string(), files);
  });

  bench(&directory, &cache);
}

#[bench]
fn full_cache_no_changes(b: &mut test::Bencher) {
  setup(100, 100, 128 * 1024, |directory, cache| {
    let cache_as_of = time::SystemTime::now();

    b.iter(|| {
      archive::update_cache(&cache, &Some(cache_as_of), &directory)
    });
  });
}

#[bench]
fn full_cache_with_changes(b: &mut test::Bencher) {
  let cache_as_of = time::SystemTime::now();

  setup(100, 100, 128 * 1024, |directory, cache| {
    b.iter(|| {
      archive::update_cache(&cache, &Some(cache_as_of), &directory)
    });
  });
}

#[bench]
fn empty_cache(b: &mut test::Bencher) {
  setup(100, 100, 128 * 1024, |directory, _| {
    b.iter(|| {
      archive::update_cache(&HashMap::new(), &None, &directory)
    });
  });
}
