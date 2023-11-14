mod config;
mod directory;
mod movie;
mod show;
mod media;

use log::*;
use clap::Parser;
use std::{path::PathBuf, env, fs};
use inline_colorization::*;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Quiet mode
    #[arg(short, long)]
    quiet: bool,

    /// Verbosity
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// First run mode
    #[arg(short, long)]
    first_run: bool,

    /// Move files rather than copying them
    #[arg(short, long="move")]
    moov: bool,

    /// Output moves/copies instead of actually doing them
    #[arg(short, long)]
    dry_run: bool,

    /// Look for shows instead of movies
    #[arg(short, long)]
    shows: bool,

    /// Custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Path to look for media in
    path: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();

    // Initialise error logger to use `stderr` and verbosity/quiet mode from command line flags
    stderrlog::new()
        .module(module_path!())
        .quiet(args.quiet)
        .verbosity(args.verbose as usize + 1)
        .init()
        .unwrap();

    // Set config path config to home folder, or if provided to specified file
    let config_path = if args.config.is_none() {
        PathBuf::from(std::env::var("HOME").unwrap()).join(".plex-media-ingest").join("config.json")
    } else {
        args.config.unwrap()
    };

    info!("Loading config from \"{}\"", config_path.to_str().unwrap());

    // Read config, or run first run wizard and write config, if none can be found
    let cfg = config::load(&config_path, args.first_run).unwrap();

    info!("Found config: {:#?}", cfg);

    // Use either provided or current path as search path for movies/shows
    let search_path = if args.path.is_none() {
        env::current_dir().unwrap()
    } else {
        args.path.unwrap()
    };

    // Search path and put everything in vector to hold all the file moves (or copies)
    let moves = directory::search_path(search_path, cfg, args.shows).unwrap();

    for move_file in moves {
        if args.moov {
            // Move files instead of copying
            println!("Moving {style_bold}{color_red}{}{color_reset}{style_reset} -> {style_bold}{color_green}{}{color_reset}{style_reset}", move_file.from.display(), move_file.to.display());
            if args.dry_run {
                continue;
            }
            fs::create_dir_all(move_file.to.parent().unwrap()).unwrap();
            match fs::rename(&move_file.from, &move_file.to) {
                Ok(_) => continue,
                Err(e) => {
                    warn!("Can not rename, error {:#?}, copying and deleting instead", e);
                    match fs::copy(&move_file.from, &move_file.to) {
                        Ok(_) => _ = fs::remove_file(&move_file.from),
                        Err(e) => {
                            error!("Copy also failed with error {:#?}", e);
                            continue;
                        }
                    }
                }
            }
        } else {
            // Copy files
            println!("Copying {style_bold}{color_red}{}{color_reset}{style_reset} -> {style_bold}{color_green}{}{color_reset}{style_reset}", move_file.from.display(), move_file.to.display());
            if args.dry_run {
                continue;
            }
            fs::create_dir_all(move_file.to.parent().unwrap()).unwrap();
            match fs::copy(&move_file.from, &move_file.to) {
                Ok(_) => _ = (),
                Err(e) => {
                    error!("Copy failed with error {:#?}", e);
                    continue;
                }
            }
        }
    }
}