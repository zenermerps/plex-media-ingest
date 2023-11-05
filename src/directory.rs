use std::{path::PathBuf, fs::{self, DirEntry}, error::Error};

use crate::media::handle_media;

/*fn is_not_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| entry.depth() == 0 || (!s.starts_with(".") && !s.starts_with("@"))) // todo!: Allow ignored chars to be configured, here, @ is QNAP special folders
        .unwrap_or(false)
}

pub fn walk_path(path: PathBuf) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = vec![];
    WalkDir::new(path)
        .into_iter()
        .filter_entry(|e| is_not_hidden(e))
        .filter_map(|v| v.ok())
        .for_each(|x| entries.push(x.into_path()));
    entries
}*/

pub fn search_path(path: PathBuf) -> Result<(), Box<dyn Error>> {
    let entries = fs::read_dir(path)?;
    let mut files: Vec<DirEntry> = Vec::new();
    let mut dirs: Vec<DirEntry> = Vec::new();

    // Put all files and folders in corresponding vectors
    for entry in entries {
        if let Ok(entry) = entry {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    dirs.push(entry);
                } else if file_type.is_file() {
                    files.push(entry);
                }
            }
        }
    }

    if dirs.len() == 0 {
        // No folders present, assuming there are only distinct media files
        for file in files {
            handle_media(file);
        }
    }

    Ok(())
}

/*
Look at current directory:
    Only directories, no media files ->
        Media must be in subfolders, look at name of folder (in case media file has cryptic name) and traverse into it, look at media files

    Media files present, folders as well ->
        Treat media as media to add, traverse into subfolders and look for eventual extra content

    Media file(s), but no folders present ->
        Treat media as media to add
*/

/*
Use folder/file name as name to look up on tmdb (replace . with ' ' till first occurence of non alphanumeric symbol ([]())) (For shows, look for SxxEyy or similar tokens, if single file assume movie by default)
If there is a token with only 4 digits (and maybe parantheses), assume this is a year, add it to tmdb search, retry search without 'year' if result is empty
Show user file with title(s) found on tmdb, make user select one
Remember selection and look in current folder for extra content (deleted scenes, trailers, featurettes) -> show user what files were found and have them select which files they want
For each selection show file name plus try to match to extras category, show user selection of which kind of extra it is, then allow them to enter a arbitary name (prefill from file if possible)
*/