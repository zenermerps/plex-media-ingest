use std::{path::PathBuf, error::Error, io::Read, fs::{File, DirEntry}, cmp, fmt, ops::Deref};
use infer;
use inquire::Select;
use log::{info, warn, error, trace, debug};
use serde::Deserialize;
use urlencoding::encode;

use crate::config::Config;

#[derive(Deserialize, Debug)]
struct TMDBResponse {
    page: i32,
    results: Vec<TMDBEntry>,
    total_pages: i32,
    total_results: i32
}

#[derive(Deserialize, Debug)]
struct TMDBEntry {
    id: i32,
    #[serde(alias = "name")]
    title: String,
    original_language: Option<String>,
    #[serde(alias = "original_name")]
    original_title: String,
    overview: Option<String>,
    media_type: String,
    popularity: f32,
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
    let mut h = reqwest::header::HeaderMap::new();
    h.insert("Accept", reqwest::header::HeaderValue::from_static("application/json"));
    h.insert("Authorization", reqwest::header::HeaderValue::from_str(format!("Bearer {}", cfg.tmdb_key).as_str()).unwrap());
    
    let client = reqwest::blocking::Client::builder()
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

    let ans = Select::new(format!("Select movie or show that matches the file \x1b[93m{}\x1b[0m:", file_name.display()).as_str(), options).prompt();
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

fn video_file_handler(entry: DirEntry, cfg: Config) {
    let path = entry.path();
    info!("Found video file: {:#?}", path);

    let file_name = path.file_name().unwrap_or_default();
    trace!("File name is: {:#?}", file_name);

    let name_tokens = tokenize_media_name(file_name.to_str().unwrap_or_default().to_string());
    
    match lookup_media(entry.path(), name_tokens, cfg) {
        Some(entry) => todo!("Save media info in some struct to move media afterwards, or move directly"),
        None => {}, 
    }

}

pub fn handle_media(entry: DirEntry, cfg: Config) {
    if entry.file_type().is_ok_and(|t| t.is_dir()) {
        warn!("Directory passed to handle_media, {:#?} will be skipped", entry);
        return
    }

    match get_file_header(entry.path()) {
        Ok(header) => {
            // Handle video files
            if infer::is_video(&header) {
                video_file_handler(entry, cfg.clone());
            }
        },
        Err(error) => error!("Can not get file header for {:#?}, Error: {:#?}", entry, error),
    }    
}