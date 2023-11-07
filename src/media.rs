use std::{path::PathBuf, error::Error, fs::File, cmp, io::Read};

use log::trace;

#[derive(Debug, Clone)]
pub struct Move {
    pub from: PathBuf,
    pub to: PathBuf
}

pub fn get_file_header(path: PathBuf) -> Result<Vec<u8>, Box<dyn Error>> {
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
        t.eq_ignore_ascii_case("remux") ||
        t.eq_ignore_ascii_case("atmos") ||
        t.eq_ignore_ascii_case("pdtv") ||
        t.eq_ignore_ascii_case("td") ||
        t.eq_ignore_ascii_case("internal") ||
        t.eq_ignore_ascii_case("ma") ||
        t.eq_ignore_ascii_case("sample") ||             // This just removes the word sample, maybe we want to ban files with the word sample all together
        (t.starts_with('[') || t.ends_with(']')) ||
        (t.starts_with('(') || t.ends_with(')')) ||
        (t.starts_with('{') || t.ends_with('}')) ||
        (t.starts_with(['s','S']) && t.len() == 3 && t.chars().next().map(char::is_numeric).unwrap_or(false))      // Season specifier
    {
        return false;
    }
    true
}

pub fn tokenize_media_name(file_name: String) -> Vec<String> {
    let tokens: Vec<String> = file_name.split(&['-', ' ', ':', '@', '.'][..]).filter(|t| token_valid(t)).map(String::from).collect();
    trace!("Tokens are: {:#?}", tokens);
    tokens
}