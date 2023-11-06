mod config;
mod directory;
mod media;

use log::*;
use clap::Parser;
use std::{path::PathBuf, env};

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

    /// Custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Path to look for media in
    path: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();

    stderrlog::new()
        .module(module_path!())
        .quiet(args.quiet)
        .verbosity(args.verbose as usize + 1)
        .init()
        .unwrap();

    trace!("trace message");
    debug!("debug message");
    info!("info message");
    warn!("warn message");
    error!("error message");

    let config_path = if args.config.is_none() {
        PathBuf::from(std::env::var("HOME").unwrap()).join(".plex-media-ingest").join("config.json")
    } else {
        args.config.unwrap()
    };

    info!("Loading config from \"{}\"", config_path.to_str().unwrap());

    let cfg = config::load(&config_path, args.first_run).unwrap();

    info!("Found config: {:#?}", cfg);

    let search_path = if args.path.is_none() {
        env::current_dir().unwrap()
    } else {
        args.path.unwrap()
    };

    //let files = directory::walk_path(search_path);
    directory::search_path(search_path, cfg).unwrap();

    /*for file in files.clone() {
        info!("Found: {}", file.to_str().unwrap());
    }*/

    //search_media(files).unwrap();

}