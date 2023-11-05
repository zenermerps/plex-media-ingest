use std::{path::PathBuf, error::Error, io::Read, fs::{File, DirEntry}, cmp};
use infer;
use log::{info, warn, error, trace, debug};

#[derive(Debug)]
struct MediaName {
    name: String,
    year: String
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

fn token_year_likely(t: &&str) -> bool {
    if t.len() == 6 &&
        (t.starts_with('[') && t.ends_with(']')) ||
        (t.starts_with('(') && t.ends_with(')')) ||
        (t.starts_with('{') && t.ends_with('}'))   
    {
        return true;    
    }
    return false
}

fn token_valid(t: &&str) -> bool {
    if
        t.eq_ignore_ascii_case("dvd") ||
        t.eq_ignore_ascii_case("bluray") ||
        t.eq_ignore_ascii_case("webrip") ||
        t.eq_ignore_ascii_case("web") ||
        t.eq_ignore_ascii_case("uhd") ||
        t.eq_ignore_ascii_case("hd") ||
        t.eq_ignore_ascii_case("tv") ||
        t.eq_ignore_ascii_case("tvrip") ||
        t.eq_ignore_ascii_case("1080p") ||
        t.eq_ignore_ascii_case("1080i") ||
        t.eq_ignore_ascii_case("2160p") ||
        (t.len() != 6 && t.starts_with('[') && t.ends_with(']')) ||
        (t.len() != 6 && t.starts_with('(') && t.ends_with(')')) ||
        (t.len() != 6 && t.starts_with('{') && t.ends_with('}'))
    {
        return false;
    }
    true
}

fn find_media_name(file_name: String) -> MediaName {
    let mut tokens: Vec<&str> = file_name.split(&['-', ' ', ':', '@', '.'][..]).filter(|t| token_valid(t)).collect();
    trace!("Tokens are: {:#?}", tokens);

    // Remove last token (file ext)
    _ = tokens.pop();

    let mut year = String::new();
    let mut name = String::new();

    for token in tokens {
        if token_year_likely(&token)  {
            year = token.strip_prefix(['(', '[', '{']).unwrap().strip_suffix([')', ']', '}']).unwrap().to_string();
        } else if token.len() != 0 {
            name.push_str(token);
            name.push(' ');
        }
    }

    // Remove last added space
    name.pop();

    let media_name = MediaName { name: name, year: year };
    debug!("Name is now: {:#?}", media_name);
    media_name
}

fn video_file_handler(entry: DirEntry) {
    let path = entry.path();
    info!("Found video file: {:#?}", path);

    let file_name = path.file_name().unwrap_or_default();
    trace!("File name is: {:#?}", file_name);

    let name = find_media_name(file_name.to_str().unwrap_or_default().to_string());
    todo!("Do TMDB API calls");
}

pub fn handle_media(entry: DirEntry) {
    if entry.file_type().is_ok_and(|t| t.is_dir()) {
        warn!("Directory passed to handle_media, {:#?} will be skipped", entry);
        return
    }

    match get_file_header(entry.path()) {
        Ok(header) => {
            // Handle video files
            if infer::is_video(&header) {
                video_file_handler(entry);
            }
        },
        Err(error) => error!("Can not get file header for {:#?}, Error: {:#?}", entry, error),
    }    
}