use std::path::PathBuf;

use walkdir::{DirEntry, WalkDir};

fn is_not_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| entry.depth() == 0 || !s.starts_with("."))
        .unwrap_or(false)
}

pub fn list_files(path: PathBuf) -> Vec<PathBuf>{
    let mut entries: Vec<PathBuf> = vec![];
    WalkDir::new(path)
        .into_iter()
        .filter_entry(|e| is_not_hidden(e))
        .filter_map(|v| v.ok())
        .for_each(|x| entries.push(x.into_path()));
    entries
}