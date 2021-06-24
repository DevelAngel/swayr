use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs::DirBuilder;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    launcher: Option<Launcher>,
    format: Option<Format>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            launcher: Some(Launcher {
                executable: Some("wofi".to_string()),
                args: Some(vec![
                    "--show=dmenu".to_string(),
                    "--allow-markup".to_string(),
                    "--allow-images".to_string(),
                    "--insensitive".to_string(),
                    "--cache-file=/dev/null".to_string(),
                    "--parse-search".to_string(),
                    "--prompt={prompt}".to_string(),
                ]),
            }),
            format: Some(Format {
                window_format: Some(
                    "\"{title}\"\t{app_name} on workspace {workspace_name}\t({id})"
                        .to_string(),
                ),
                workspace_format: Some("Workspace {name}\t({id})".to_string()),
            }),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Launcher {
    executable: Option<String>,
    args: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Format {
    window_format: Option<String>,
    workspace_format: Option<String>,
}

fn get_config_file_path() -> Box<Path> {
    let proj_dirs = ProjectDirs::from("", "", "swayr").expect("");
    let config_dir = proj_dirs.config_dir();
    if !config_dir.exists() {
        DirBuilder::new()
            .recursive(true)
            .create(config_dir)
            .unwrap();
    }
    config_dir.join("config.toml").into_boxed_path()
}

pub fn save_config(cfg: Config) {
    let path = get_config_file_path();
    let content =
        toml::to_string_pretty(&cfg).expect("Cannot serialize config.");
    let mut file = OpenOptions::new()
        .read(false)
        .write(true)
        .create(true)
        .open(path)
        .unwrap();
    file.write_all(content.as_str().as_bytes()).unwrap();
}

pub fn load_config() -> Config {
    let path = get_config_file_path();
    if !path.exists() {
        save_config(Config::default());
    }
    let mut file = OpenOptions::new()
        .read(true)
        .write(false)
        .create(false)
        .open(path)
        .unwrap();
    let mut buf: String = String::new();
    file.read_to_string(&mut buf).unwrap();
    toml::from_str(&buf).expect("Invalid config.")
}

#[test]
fn test_load_config() {
    let cfg = load_config();
    println!("{:?}", cfg);
}
