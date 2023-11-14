use std::{fmt, fs::DirEntry, path::PathBuf, time::Duration};

use inquire::{Select, Text, Confirm};
use log::{error, info, trace, debug, warn};
use reqwest::{header::{HeaderMap, HeaderValue}, blocking::Client};
use sanitise_file_name::sanitise;
use serde::Deserialize;
use urlencoding::encode;
use walkdir::WalkDir;
use inline_colorization::*;
use regex::RegexBuilder;

use crate::{config::Config, media::{Move, self, get_file_header}, directory::search_path};

// Struct to hold the TMDB API response
#[derive(Deserialize, Debug)]
struct TMDBResponse {
    results: Vec<TMDBEntry>,
    total_results: i32
}

// Struct to hold a show from the TMDB API response
#[derive(Deserialize, Debug, Clone)]
struct TMDBEntry {
    id: i32,
    name: String,
    original_language: Option<String>,
    first_air_date: Option<String>,
}

// Display implementation for the inquire selection dialog
impl fmt::Display for TMDBEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
         write!(f, "{} ({}, {}) (ID: {})", self.name, self.first_air_date.clone().unwrap_or("unknown".to_string()), self.original_language.as_ref().unwrap(), self.id)
    }
}

// Use directory name to find out show name, as opposed to file name for movies
fn check_show_name(entry: PathBuf, cfg: Config) -> Option<TMDBEntry> {
    info!("Found folder: {:#?}", entry);

    let folder_name = entry.file_name().unwrap_or_default();
    trace!("Folder name is: {:#?}", folder_name);

    let name_tokens = media::tokenize_media_name(folder_name.to_str().unwrap_or_default().to_string());
    lookup_show(entry, name_tokens, cfg)
}

// Look up show on the TMDB API
fn lookup_show(folder_name: PathBuf, mut name_tokens: Vec<String>, cfg: Config) -> Option<TMDBEntry> {
    if name_tokens.first().unwrap_or(&"".to_string()).eq_ignore_ascii_case("season") {
        // Is a season folder most likely, skip useless TMDB requests
        return None;
    }
    let mut h = HeaderMap::new();
    h.insert("Accept", HeaderValue::from_static("application/json"));
    h.insert("Authorization", HeaderValue::from_str(format!("Bearer {}", cfg.tmdb_key).as_str()).unwrap());
    
    let client = Client::builder()
        .default_headers(h)
        .build().unwrap();

    let mut response: TMDBResponse;
    loop {
        if name_tokens.len() == 0 {
            error!("Could not find title on TMDB!");
            return None;
        }

        let name = name_tokens.join(" ");
        trace!("Searching on TMDB for {:#?}", name);

        let http_response = client
            .get(format!("https://api.themoviedb.org/3/search/tv?query={}&include_adult=false&language=en-US&page=1", encode(name.as_str()).into_owned()))
            .timeout(Duration::from_secs(120))
            .send();

        if http_response.is_err() {
            warn!("Request error: {:#?}", http_response.unwrap_err());
            return None;
        }

        response = http_response.unwrap().json::<TMDBResponse>().unwrap();
        trace!("TMDB Reponse: {:#?}", response);

        if response.total_results == 0 {
            name_tokens.pop();
        } else {
            break;
        }
    }

    let options = response.results;

    let ans = Select::new(format!("Select show that resides in folder {style_bold}{}{style_reset} (Ctrl-C to skip):", folder_name.display()).as_str(), options).prompt();
    match ans {
        Ok(choice) => {
            debug!("Selected: {:#?}", choice);
            return Some(choice);
        },
        Err(e) => {
            error!("Error while selecting content: {:#?}", e);
            return None;
        },
    }
}

// Handler for the sorted vectors of files and folders, gets called recursively for subfolders, if no primary media can be found
pub fn handle_show_files_and_folders(directory: PathBuf, files: Vec<DirEntry>, folders: Vec<DirEntry>, cfg: Config) -> Vec<Move> {
    let mut moves: Vec<Move> = Vec::new();
    let mut primary_media: Option<TMDBEntry>;
    
    // Check current directory for possible name
    primary_media = check_show_name(directory, cfg.clone());
    match primary_media {
        Some(_) => {
            // There is already primary media, check files and directories for more media for same show
            for file in files {
                if file.file_type().unwrap().is_file() {
                    if file.path().to_str().unwrap_or_default().to_string().to_ascii_lowercase().contains("sample") {
                        continue;
                    }
                    check_show_file(file.path(), &mut primary_media, &cfg, &mut moves);
                }
            }
            for folder in folders {
                for entry in WalkDir::new(folder.path()) {
                    match entry {
                        Ok(entry) => {
                            if entry.file_type().is_file() {
                                if entry.path().to_str().unwrap_or_default().to_string().to_ascii_lowercase().contains("sample") {
                                    continue;
                                }
                                check_show_file(entry.into_path(), &mut primary_media, &cfg, &mut moves);
                            }
                        },
                        Err(e) => {
                            error!("Error walking the directory: {:#?}", e);
                            continue;
                        }
                    }
                }
            }
        },
        None => {
            // There is no primary media yet, try every folder as main folder
            for folder in folders {
                moves.append(&mut search_path(folder.path(), cfg.clone(), true).unwrap());
            }
        }
    }
    moves
}

// Check files for episodes or subtitles, show required inquire dialoges
fn check_show_file(file: PathBuf, primary_media: &mut Option<TMDBEntry>, cfg: &Config, moves: &mut Vec<Move>) {
    trace!("Checking {:#?}", file);
    match get_file_header(file.clone()) {
        Ok(header) => {
            // Try to parse Season/Episode from filename
            let re = RegexBuilder::new(r"(?:S(?<season0>[0-9]+)\.?E(?<episode0>[0-9]+)|(?<season1>[0-9]+)x(?<episode1>[0-9]+))")
                .case_insensitive(true).build().unwrap();
            let Some(caps) = re.captures(file.to_str().unwrap_or_default()) else { warn!("Regex doesn't match {:#?}, skipping", file); return; };
            let season: i32 = caps.name("season0").map_or_else(||caps.name("season1").map_or("", |m| m.as_str()), |m| m.as_str()).parse().unwrap();
            let episode: i32 = caps.name("episode0").map_or_else(||caps.name("episode1").map_or("", |m| m.as_str()), |m| m.as_str()).parse().unwrap();
            trace!("Found Season {0:02}, Episode {1:02}", season, episode);

            // Handle video files
            if infer::is_video(&header) {
                match primary_media.as_ref() {
                    None => {
                        error!("Can not parse files without matched show!");
                        return;
                    },
                    Some(primary_media) => {
                        let original_path = file;
                        let ext = original_path.extension().unwrap_or_default();
                        let year: String;
                        match primary_media.clone().first_air_date.unwrap_or_default().split('-').nth(0) {
                            Some(y) => year = format!("({}) ", y),
                            None => year = "".to_string()
                        }
                        let new_path = cfg.plex_library.join(format!("TV Shows/{0} {3}{{tmdb-{1}}}/Season {4:02}/{0} - S{4:02}E{5:02}.{2}", sanitise(primary_media.name.as_str()), primary_media.id, ext.to_str().unwrap_or_default(), year, season, episode));
                        moves.push(Move { from: original_path, to: new_path });
                    }
                }
            } else {
                match file.extension() {
                    Some(ext) => {
                        if ext.eq_ignore_ascii_case("srt") ||
                            ext.eq_ignore_ascii_case("ass") ||
                            ext.eq_ignore_ascii_case("ssa") ||
                            ext.eq_ignore_ascii_case("smi") ||
                            ext.eq_ignore_ascii_case("pgs") ||
                            ext.eq_ignore_ascii_case("vob") {
                            // Subtitle file
                            if primary_media.is_none() {
                                warn!("Can not categorize subtitle file without primary media, skipping.");
                                return;
                            }

                            let lang_code = Text::new(format!("Specify ISO-639-1 (2-letter) language code (e.g. 'en', 'de') or leave empty to discard for {style_bold}{}{style_reset}:", file.display()).as_str()).prompt();
                            match lang_code {
                                Ok(lang_code) => {
                                    if lang_code == "" {
                                        return;
                                    }
                                    let forced = Confirm::new("Is this a forced sub?").with_default(false).prompt();
                                    match forced {
                                        Ok(true) => {
                                            // Forced
                                            let original_path = file;
                                            let ext = original_path.extension().unwrap_or_default();
                                            let year: String;
                                            match primary_media.clone().unwrap().first_air_date.unwrap_or_default().split('-').nth(0) {
                                                Some(y) => year = format!("({}) ", y),
                                                None => year = "".to_string()
                                            }
                                            let new_path = cfg.plex_library.join(format!("TV Shows/{0} {4}{{tmdb-{1}}}/Season {5:02}/{0} - S{5:02}E{6:02}.{3}.forced.{2}", sanitise(primary_media.as_ref().unwrap().name.as_str()), primary_media.as_ref().unwrap().id, ext.to_str().unwrap_or_default(), lang_code.to_ascii_lowercase(), year, season, episode));
                                            moves.push(Move { from: original_path, to: new_path });
                                            return;
                                        },
                                        Ok(false) => {
                                            // Non-forced
                                            let original_path = file;
                                            let ext = original_path.extension().unwrap_or_default();
                                            let year: String;
                                            match primary_media.clone().unwrap().first_air_date.unwrap_or_default().split('-').nth(0) {
                                                Some(y) => year = format!("({}) ", y),
                                                None => year = "".to_string()
                                            }
                                            let new_path = cfg.plex_library.join(format!("TV Shows/{0} {4}{{tmdb-{1}}}/Season {5:02}/{0} - S{5:02}E{6:02}.{3}.{2}", sanitise(primary_media.as_ref().unwrap().name.as_str()), primary_media.as_ref().unwrap().id, ext.to_str().unwrap_or_default(), lang_code.to_ascii_lowercase(), year, season, episode));
                                            moves.push(Move { from: original_path, to: new_path });
                                            return;
                                        },
                                        Err(e) => {
                                            error!("There was an error: {:#?}", e);
                                            return;
                                        },
                                    }
                                },
                                Err(e) => {
                                    error!("There was an error: {:#?}", e);
                                    return;
                                },
                            }
                        } else {
                            info!("Not a video file nor subtitle, skipping");
                            return;
                        }
                    },
                    None => {
                        error!("File {:#?} has no file extension", file);
                        return;
                    }
                }
            }
        },
        Err(error) => error!("Can not get file header for {:#?}, Error: {:#?}", file, error),
    }
}