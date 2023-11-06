use std::{path::PathBuf, error::Error, io::Read, fs::{File, DirEntry}, cmp, fmt};
use infer;
use inquire::{Select, Text, Confirm};
use log::{info, warn, error, trace, debug};
use reqwest::{blocking::Client, header::{HeaderMap, HeaderValue}};
use serde::Deserialize;
use urlencoding::encode;
use inline_colorization::*;
use sanitise_file_name::sanitise;
use walkdir::WalkDir;

use crate::{config::Config, directory::search_path};

#[derive(Deserialize, Debug)]
struct TMDBResponse {
    results: Vec<TMDBEntry>,
    total_results: i32
}

#[derive(Deserialize, Debug, Clone)]
struct TMDBEntry {
    id: i32,
    #[serde(alias = "name")]
    title: String,
    original_language: Option<String>,
    media_type: String,
    #[serde(alias = "first_air_date")]
    release_date: Option<String>,
}

impl fmt::Display for TMDBEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.media_type == "movie" {
            write!(f, "[MOVIE] {} ({}, {}) (ID: {})", self.title, self.release_date.clone().unwrap_or("unknown".to_string()), self.original_language.as_ref().unwrap(), self.id)
        } else if self.media_type == "tv" {
            write!(f, "[SHOW] {} ({}, {}) (ID: {})", self.title, self.release_date.clone().unwrap_or("unknown".to_string()), self.original_language.as_ref().unwrap(), self.id)
        } else {
            write!(f, "[{}] {} (ID: {})", self.media_type, self.title, self.id)
        }
    }
}

#[derive(Debug)]
pub struct Move {
    pub from: PathBuf,
    pub to: PathBuf
}

fn get_file_header(path: PathBuf) -> Result<Vec<u8>, Box<dyn Error>> {
    let f = File::open(path)?;

    let limit = f
        .metadata()
        .map(|m| cmp::min(m.len(), 8192) as usize + 1)
        .unwrap_or(0);
    let mut bytes = Vec::with_capacity(limit);
    f.take(8192).read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn token_valid(t: &&str) -> bool {
    if
        t.eq_ignore_ascii_case("dvd") ||
        t.eq_ignore_ascii_case("bluray") ||
        t.eq_ignore_ascii_case("webrip") ||
        t.eq_ignore_ascii_case("youtube") ||
        t.eq_ignore_ascii_case("download") ||
        t.eq_ignore_ascii_case("web") ||
        t.eq_ignore_ascii_case("uhd") ||
        t.eq_ignore_ascii_case("hd") ||
        t.eq_ignore_ascii_case("tv") ||
        t.eq_ignore_ascii_case("tvrip") ||
        t.eq_ignore_ascii_case("1080p") ||
        t.eq_ignore_ascii_case("1080i") ||
        t.eq_ignore_ascii_case("2160p") ||
        t.eq_ignore_ascii_case("x264") ||
        t.eq_ignore_ascii_case("x265") ||
        t.eq_ignore_ascii_case("h265") ||
        t.eq_ignore_ascii_case("dts") ||
        t.eq_ignore_ascii_case("hevc") ||
        t.eq_ignore_ascii_case("10bit") ||
        t.eq_ignore_ascii_case("12bit") ||
        t.eq_ignore_ascii_case("hdr") ||
        t.eq_ignore_ascii_case("xvid") ||
        t.eq_ignore_ascii_case("AAC5") ||
        t.eq_ignore_ascii_case("AAC") ||
        t.eq_ignore_ascii_case("AC3") ||
        t.eq_ignore_ascii_case("sample") ||             // This just removes the word sample, maybe we want to ban files with the word sample all together
        (t.starts_with('[') || t.ends_with(']')) ||
        (t.starts_with('(') || t.ends_with(')')) ||
        (t.starts_with('{') || t.ends_with('}'))
    {
        return false;
    }
    true
}

fn tokenize_media_name(file_name: String) -> Vec<String> {
    let mut tokens: Vec<String> = file_name.split(&['-', ' ', ':', '@', '.'][..]).filter(|t| token_valid(t)).map(String::from).collect();
    trace!("Tokens are: {:#?}", tokens);

    // Remove last token (file ext)
    _ = tokens.pop();
    tokens
}

fn lookup_media(file_name: PathBuf, mut name_tokens: Vec<String>, cfg: Config) -> Option<TMDBEntry> {
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
            .get(format!("https://api.themoviedb.org/3/search/multi?query={}&include_adult=false&language=en-US&page=1", encode(name.as_str()).into_owned()))
            .send().unwrap();

        response = http_response.json::<TMDBResponse>().unwrap();
        trace!("TMDB Reponse: {:#?}", response);

        if response.total_results == 0 {
            name_tokens.pop();
        } else {
            break;
        }
    }

    let options = response.results;

    let ans = Select::new(format!("Select movie or show that matches the file {style_bold}{}{style_reset}:", file_name.display()).as_str(), options).prompt();
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

fn video_file_handler(entry: PathBuf, cfg: Config) -> Option<TMDBEntry> {
    info!("Found video file: {:#?}", entry);

    let file_name = entry.file_name().unwrap_or_default();
    trace!("File name is: {:#?}", file_name);

    let name_tokens = tokenize_media_name(file_name.to_str().unwrap_or_default().to_string());
    
    lookup_media(entry, name_tokens, cfg)
}

pub fn handle_movie_files_and_folders(files: Vec<DirEntry>, folders: Vec<DirEntry>, cfg: Config) -> Vec<Move> {
    let mut moves: Vec<Move> = Vec::new();
    let mut primary_media: Option<TMDBEntry> = None; // Assuming first file (biggest file) is primary media, store the information of this, for the rest, do lazy matching for extra content/subs and so on
    for file in files {
        check_movie_file(file.path(), &mut primary_media, &cfg, &mut moves);
    }
    match primary_media {
        Some(_) => {
            // There is already primary media, check directories for more media for same movie
            for folder in folders {
                for entry in WalkDir::new(folder.path()) {
                    match entry {
                        Ok(entry) => {
                            if entry.file_type().is_file() {
                                check_movie_file(entry.into_path(), &mut primary_media, &cfg, &mut moves);
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
                moves.append(&mut search_path(folder.path(), cfg.clone()).unwrap());
            }
        }
    }
    moves
}

fn check_movie_file(file: PathBuf, primary_media: &mut Option<TMDBEntry>, cfg: &Config, moves: &mut Vec<Move>) {
    trace!("Checking {:#?}", file);
    match get_file_header(file.clone()) {
        Ok(header) => {
            // Handle video files
            if infer::is_video(&header) {
                match primary_media.as_ref() {
                    None => {
                        // No primary media found yet, look up media on TMDB
                        match video_file_handler(file.clone(), cfg.clone()) {
                            Some(meta) => {
                                *primary_media = Some(meta.clone());
                                let original_path = file;
                                let ext = original_path.extension().unwrap_or_default();
                                let new_path = cfg.plex_library.join(format!("Movies/{0} {{tmdb-{1}}}/{0} {{tmdb-{1}}}.{2}", sanitise(meta.title.as_str()), meta.id, ext.to_str().unwrap_or_default()));
                                moves.push(Move { from: original_path, to: new_path });
                            },
                            None => {
                                warn!("Could not find a TMDB entry for {:#?}", file);
                                return;
                            },
                        }
                    },
                    Some(primary_media) => {
                        // No additional TMDB lookup needed, treat media as extras
                        let extra_types: Vec<&str> = vec!["Ignore", "Edition", "Behind The Scenes", "Deleted Scenes", "Featurettes", "Interviews", "Scenes", "Shorts", "Trailers", "Other"];
                        let ans = Select::new(format!("Select extra type {style_bold}{}{style_reset} (Ignore to ignore the file, Edition to treat it as alternate edition of the main movie):", file.display()).as_str(), extra_types).prompt();

                        match ans {
                            Ok(choice) => {
                                if choice == "Ignore" {
                                    // Ignoring the given file
                                    return;
                                }
                                if choice == "Edition" {
                                    // Treat the given file as different edition of main movie
                                    let edition_name = Text::new("Specify the edition's name (e.g. Director's Cut, Theatrical Version):").prompt();
                                    match edition_name {
                                        Ok(edition_name) => {
                                            let original_path = file;
                                            let ext = original_path.extension().unwrap_or_default();
                                            let new_path = cfg.plex_library.join(format!("Movies/{0} {{tmdb-{1}}}/{0} {{tmdb-{1}}} {{edition-{3}}}.{2}", sanitise(primary_media.title.as_str()), primary_media.id, ext.to_str().unwrap_or_default(), edition_name));
                                            moves.push(Move { from: original_path, to: new_path });
                                            return;
                                        },
                                        Err(e) => {
                                            error!("There was an error: {:#?}", e);
                                            return;
                                        },
                                    }
                                }
                                let initial_value = file.file_stem().unwrap_or_default().to_str().unwrap_or_default();
                                let description = Text::new(format!("Give this {} a descriptive name:", choice).as_str()).with_initial_value(initial_value).prompt();
                                match description {
                                    Ok(description) => {
                                        let original_path = file;
                                        let ext = original_path.extension().unwrap_or_default();
                                        let new_path = cfg.plex_library.join(format!("Movies/{0} {{tmdb-{1}}}/{3}/{4}.{2}", sanitise(primary_media.title.as_str()), primary_media.id, ext.to_str().unwrap_or_default(), choice, description));
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
                                            let new_path = cfg.plex_library.join(format!("Movies/{0} {{tmdb-{1}}}/{0} {{tmdb-{1}}}.{3}.forced.{2}", sanitise(primary_media.as_ref().unwrap().title.as_str()), primary_media.as_ref().unwrap().id, ext.to_str().unwrap_or_default(), lang_code.to_ascii_lowercase()));
                                            moves.push(Move { from: original_path, to: new_path });
                                            return;
                                        },
                                        Ok(false) => {
                                            // Non-forced
                                            let original_path = file;
                                            let ext = original_path.extension().unwrap_or_default();
                                            let new_path = cfg.plex_library.join(format!("Movies/{0} {{tmdb-{1}}}/{0} {{tmdb-{1}}}.{3}.{2}", sanitise(primary_media.as_ref().unwrap().title.as_str()), primary_media.as_ref().unwrap().id, ext.to_str().unwrap_or_default(), lang_code.to_ascii_lowercase()));
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