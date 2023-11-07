use std::{fs, error::Error, path::PathBuf, io::ErrorKind, str::FromStr};
use inquire::{Text, CustomUserError, Autocomplete, autocompletion::Replacement};
use log::{warn, info, error};
use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Config {
    pub tmdb_key: String,
    pub plex_library: PathBuf,
}

pub fn load(path: &PathBuf, first: bool) -> Result<Config, Box<dyn Error>> {
    if first {
        info!("Running first run wizard...");
        let cfg = first_run()?;
        save(cfg.clone(), path)?;
        return Ok(cfg);
    }

    let f = fs::read_to_string(path);
    let f = match f {
        Ok(file) => file,
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                warn!("Config not found, running first run wizard...");
                let cfg = first_run()?;
                save(cfg.clone(), path)?;
                return Ok(cfg);
            } else {
                error!("There was an error reading the config file!");
                return Err(Box::new(e));
            }
        }
    };
    let cfg: Config = serde_json::from_str(&f)?;
    Ok(cfg)
}

pub fn first_run() -> Result<Config, Box<dyn Error>> {
    let tmdb_key = Text::new("Enter your TMDB API Read Access Token:")
    .with_help_message("The API key can be found at https://www.themoviedb.org/settings/api (you must be logged in).")
    .prompt();

    let tmdb_key = match tmdb_key {
        Ok(tmdb_key) => tmdb_key,
        Err(e) => panic!("Error retrieving TMDB key from inquire: {}", e.to_string())
    };

    let plex_library = Text::new("Enter your Plex Media Library path:")
    .with_help_message("Enter the full path of your Plex Media Library, or the path you plan to use for it.")
    .with_autocomplete(FilePathCompleter::default())
    .prompt();

    let plex_library = match plex_library {
        Ok(plex_library) => plex_library,
        Err(e) => panic!("Error retrieving Plex Library from inquire: {}", e.to_string())
    };

    let plex_library = match PathBuf::from_str(&plex_library) {
        Ok(plex_library) => plex_library,
        Err(e) => panic!("Path is not valid: {}", e.to_string())
    };

    Ok(Config { tmdb_key: tmdb_key, plex_library:  plex_library})
}

pub fn save(cfg: Config, path: &PathBuf) -> Result<(), Box<dyn Error>> {
    let serialized = serde_json::to_string_pretty(&cfg)?;
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(path, serialized)?;
    Ok(())
}

/*
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;                                                                                                  ;;
;;             ----==| A U T O C O M P L E T E   P A T H |==----                                    ;;
;;                                                                                                  ;;
;; From https://github.com/mikaelmello/inquire/blob/main/inquire/examples/complex_autocompletion.rs ;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
*/

#[derive(Clone, Default)]
pub struct FilePathCompleter {
    input: String,
    paths: Vec<String>,
    lcp: String,
}

impl FilePathCompleter {
    fn update_input(&mut self, input: &str) -> Result<(), CustomUserError> {
        if input == self.input {
            return Ok(());
        }

        self.input = input.to_owned();
        self.paths.clear();

        let input_path = std::path::PathBuf::from(input);

        let fallback_parent = input_path
            .parent()
            .map(|p| {
                if p.to_string_lossy() == "" {
                    std::path::PathBuf::from(".")
                } else {
                    p.to_owned()
                }
            })
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        let scan_dir = if input.ends_with('/') {
            input_path
        } else {
            fallback_parent.clone()
        };

        let entries = match std::fs::read_dir(scan_dir) {
            Ok(read_dir) => Ok(read_dir),
            Err(err) if err.kind() == ErrorKind::NotFound => std::fs::read_dir(fallback_parent),
            Err(err) => Err(err),
        }?
        .collect::<Result<Vec<_>, _>>()?;

        let mut idx = 0;
        let limit = 15;

        while idx < entries.len() && self.paths.len() < limit {
            let entry = entries.get(idx).unwrap();

            let path = entry.path();
            let path_str = if path.is_dir() {
                format!("{}/", path.to_string_lossy())
            } else {
                path.to_string_lossy().to_string()
            };

            if path_str.starts_with(&self.input) && path_str.len() != self.input.len() {
                self.paths.push(path_str);
            }

            idx = idx.saturating_add(1);
        }

        self.lcp = self.longest_common_prefix();

        Ok(())
    }

    fn longest_common_prefix(&self) -> String {
        let mut ret: String = String::new();

        let mut sorted = self.paths.clone();
        sorted.sort();
        if sorted.is_empty() {
            return ret;
        }

        let mut first_word = sorted.first().unwrap().chars();
        let mut last_word = sorted.last().unwrap().chars();

        loop {
            match (first_word.next(), last_word.next()) {
                (Some(c1), Some(c2)) if c1 == c2 => {
                    ret.push(c1);
                }
                _ => return ret,
            }
        }
    }
}

impl Autocomplete for FilePathCompleter {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, CustomUserError> {
        self.update_input(input)?;

        Ok(self.paths.clone())
    }

    fn get_completion(
        &mut self,
        input: &str,
        highlighted_suggestion: Option<String>,
    ) -> Result<Replacement, CustomUserError> {
        self.update_input(input)?;

        Ok(match highlighted_suggestion {
            Some(suggestion) => Replacement::Some(suggestion),
            None => match self.lcp.is_empty() {
                true => Replacement::None,
                false => Replacement::Some(self.lcp.clone()),
            },
        })
    }
}