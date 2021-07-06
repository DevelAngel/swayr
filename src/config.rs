// Copyright (C) 2021  Tassilo Horn <tsdh@gnu.org>
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
// FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License for
// more details.
//
// You should have received a copy of the GNU General Public License along with
// this program.  If not, see <https://www.gnu.org/licenses/>.

//! TOML configuration for swayr.

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::DirBuilder;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub menu: Option<Menu>,
    pub format: Option<Format>,
    pub layout: Option<Layout>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Menu {
    pub executable: Option<String>,
    pub args: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Format {
    pub window_format: Option<String>,
    pub workspace_format: Option<String>,
    pub urgency_start: Option<String>,
    pub urgency_end: Option<String>,
    pub icon_dirs: Option<Vec<String>>,
    pub fallback_icon: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Layout {
    pub auto_tile: Option<bool>,
    pub auto_tile_min_window_width_per_output_width: Option<Vec<[i32; 2]>>,
}

impl Layout {
    pub fn auto_tile_min_window_width_per_output_width_as_map(
        &self,
    ) -> Option<HashMap<i32, i32>> {
        if let Some(vec) = &self.auto_tile_min_window_width_per_output_width {
            let mut map = HashMap::new();
            for tup in vec {
                map.insert(tup[0], tup[1]);
            }
            Some(map)
        } else {
            None
        }
    }
}

impl Default for Menu {
    fn default() -> Self {
        Menu {
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
        }
    }
}

impl Default for Format {
    fn default() -> Self {
        Format {
            window_format: Some(
                "{urgency_start}<b>“{title}”</b>{urgency_end} \
                 — <i>{app_name}</i> on workspace {workspace_name}   \
                 <span alpha=\"20000\">({id})</span>"
                    .to_string(),
            ),
            workspace_format: Some(
                "<b>Workspace {name}</b>   \
                 <span alpha=\"20000\">({id})</span>"
                    .to_string(),
            ),
            urgency_start: Some(
                "<span background=\"darkred\" foreground=\"yellow\">"
                    .to_string(),
            ),
            urgency_end: Some("</span>".to_string()),
            icon_dirs: Some(vec![
                "/usr/share/icons/hicolor/scalable/apps".to_string(),
                "/usr/share/icons/hicolor/48x48/apps".to_string(),
                "/usr/share/pixmaps".to_string(),
            ]),
            fallback_icon: None,
        }
    }
}

impl Default for Layout {
    fn default() -> Layout {
        let resolution_min_width_vec = vec![
            [1024, 500],
            [1280, 600],
            [1400, 680],
            [1440, 700],
            [1600, 780],
            [1920, 920],
            [2560, 1000],
            [3440, 1000],
            [4096, 1200],
        ];

        Layout {
            auto_tile: Some(false),
            auto_tile_min_window_width_per_output_width: Some(
                resolution_min_width_vec,
            ),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            menu: Some(Menu::default()),
            format: Some(Format::default()),
            layout: Some(Layout::default()),
        }
    }
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
        // Tell the user that a fresh default config has been created.
        std::process::Command::new("swaynag")
            .arg("--background")
            .arg("00FF44")
            .arg("--text")
            .arg("0000CC")
            .arg("--message")
            .arg(
                "Welcome to swayr! ".to_owned()
                    + "I've created a fresh config for use with wofi for you in "
                    + &path.to_string_lossy()
                    + ". Adapt it to your needs.",
            )
            .arg("--type")
            .arg("warning")
            .arg("--dismiss-button")
            .arg("Thanks!")
            .spawn()
            .ok();
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
