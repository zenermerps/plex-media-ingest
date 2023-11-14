use std::{path::PathBuf, fs::{self, DirEntry}, error::Error};

use log::trace;

use crate::{movie::handle_movie_files_and_folders, config::Config, media::Move, show::handle_show_files_and_folders};

// Search a given path for movies or shows
// TODO: Add support for single file as well
pub fn search_path(path: PathBuf, cfg: Config, shows: bool) -> Result<Vec<Move>, Box<dyn Error>> {
    let entries = fs::read_dir(path.clone())?;
    let mut files: Vec<DirEntry> = Vec::new();
    let mut folders: Vec<DirEntry> = Vec::new();

    // Put all files and folders in corresponding vectors
    for entry in entries {
        if let Ok(entry) = entry {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    folders.push(entry);
                } else if file_type.is_file() {
                    files.push(entry);
                }
            }
        }
    }

    // Sort the files and directory vectors by size, so the  main movie file (the biggest usually) is the first
    folders.sort_by(|a, b| b.metadata().unwrap().len().cmp(&a.metadata().unwrap().len()));
    files.sort_by(|a, b| b.metadata().unwrap().len().cmp(&a.metadata().unwrap().len()));
    trace!("Sorted Dirs: {:#?}", folders);
    trace!("Sorted Files: {:#?}", files);

    let mut moves: Vec<Move> = Vec::new();
    if shows {
        // Find shows in directory (only one show per run supported right now)
        moves.append(&mut handle_show_files_and_folders(path, files, folders, cfg.clone()));
    } else {
        // Find movies in directory or subdirectories, find extras
        moves.append(&mut handle_movie_files_and_folders(files, folders, cfg.clone()));
    }

    Ok(moves)
}

// Some lgecy documentation, rough description of the algorithm
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