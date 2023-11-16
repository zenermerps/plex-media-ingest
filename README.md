# plex-media-ingest

plex-media-ingest is a CLI application which helps you to ingest and
organize your movies and TV shows in your [Plex](https://www.plex.tv/) library.
It is currently in development and not yet feature-complete.

## Installation

Use cargo to install plex-media-ingest. Since it is not yet published
on [crates.io](https://crates.io), use it with the `--git` flag and
the repository.

```bash
cargo install --git https://github.com/EranMorkon/plex-media-ingest.git
```

## Usage

```
Usage: plex-media-ingest [OPTIONS] [PATH]

Arguments:
  [PATH]  Path to look for media in

Options:
  -q, --quiet          Quiet mode
  -v, --verbose...     Verbosity
  -f, --first-run      First run mode
  -m, --move           Move files rather than copying them
  -d, --dry-run        Output moves/copies instead of actually doing them
  -s, --shows          Look for shows instead of movies
  -c, --config <FILE>  Custom config file
  -h, --help           Print help
  -V, --version        Print version
```

## Features

The following features are currently implemented:

* Movie matching based on file name with interactive selection from [TMDB](https://themoviedb.org) query
    * Extras and different version support with interactive selection (Support for all Plex extra types and Plex movie `edition`-field)
    * Subtitle matching if they are in separate files in the same folder as the main movie
* TV Show matching based on directory name with interactive selection from [TMDB](https://themoviedb.org) query
    * Matches Seasons and Episode numbers based on the file name of the video files
    * Subtitle matching if subtitle file name contains season and episode key
    * Support for Specials if they are named as `S00Exx`, matching like on [TMDB](https://themoviedb.org)

## Known Limitations

* Movies
    * Does not yet support passing a single file as path, only folders, and will therefore match a folder with multiple movies as one movie and extras
    * Does not yet support multiple encodings of the same edition of a movie (e.g. you have a 1080p and 2160p encode, it will only regard the one with the bigger file size as main movie, the other as extras)
    * Does not support any artworks (poster or fanart) yet
* TV Shows
    * No support for Specials named `Special` instead of `S00`
* General
    * Currently only tested on Linux with mounted SMB file system, not tested on Windows yet

## License

Dual licensed under [Apache-2.0](https://choosealicense.com/licenses/apache-2.0/) or [MIT](https://choosealicense.com/licenses/mit/)

`SPDX-License-Identifier: Apache-2.0 OR MIT`