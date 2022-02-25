// Copyright (C) 2021-2022  Tassilo Horn <tsdh@gnu.org>
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
    menu: Option<Menu>,
    format: Option<Format>,
    layout: Option<Layout>,
}

fn tilde_expand_file_names(file_names: Vec<String>) -> Vec<String> {
    let mut ret = vec![];
    for file_name in file_names {
        if file_name.starts_with('~') {
            ret.push(file_name.replacen(
                '~',
                &std::env::var("HOME").expect("$HOME not defined"),
                1,
            ));
        } else {
            ret.push(file_name)
        }
    }
    ret
}

impl Config {
    pub fn get_menu_executable(&self) -> String {
        self.menu
            .as_ref()
            .and_then(|m| m.executable.clone())
            .or_else(|| Menu::default().executable)
            .expect("No menu.executable defined!")
    }

    pub fn get_menu_args(&self) -> Vec<String> {
        self.menu
            .as_ref()
            .and_then(|m| m.args.clone())
            .or_else(|| Menu::default().args)
            .expect("No menu.args defined.")
    }

    pub fn get_format_output_format(&self) -> String {
        self.format
            .as_ref()
            .and_then(|f| f.output_format.clone())
            .or_else(|| Format::default().output_format)
            .expect("No format.output_format defined.")
    }

    pub fn get_format_workspace_format(&self) -> String {
        self.format
            .as_ref()
            .and_then(|f| f.workspace_format.clone())
            .or_else(|| Format::default().workspace_format)
            .expect("No format.workspace_format defined.")
    }

    pub fn get_format_container_format(&self) -> String {
        self.format
            .as_ref()
            .and_then(|f| f.container_format.clone())
            .or_else(|| Format::default().container_format)
            .expect("No format.container_format defined.")
    }

    pub fn get_format_window_format(&self) -> String {
        self.format
            .as_ref()
            .and_then(|f| f.window_format.clone())
            .or_else(|| Format::default().window_format)
            .expect("No format.window_format defined.")
    }

    pub fn get_format_indent(&self) -> String {
        self.format
            .as_ref()
            .and_then(|f| f.indent.clone())
            .or_else(|| Format::default().indent)
            .expect("No format.indent defined.")
    }

    pub fn get_format_urgency_start(&self) -> String {
        self.format
            .as_ref()
            .and_then(|f| f.urgency_start.clone())
            .or_else(|| Format::default().urgency_start)
            .expect("No format.urgency_start defined.")
    }

    pub fn get_format_urgency_end(&self) -> String {
        self.format
            .as_ref()
            .and_then(|f| f.urgency_end.clone())
            .or_else(|| Format::default().urgency_end)
            .expect("No format.urgency_end defined.")
    }

    pub fn get_format_html_escape(&self) -> bool {
        self.format
            .as_ref()
            .and_then(|f| f.html_escape)
            .or_else(|| Format::default().html_escape)
            .expect("No format.html_escape defined.")
    }

    pub fn get_format_icon_dirs(&self) -> Vec<String> {
        self.format
            .as_ref()
            .and_then(|f| f.icon_dirs.clone())
            .or_else(|| Format::default().icon_dirs)
            .map(tilde_expand_file_names)
            .expect("No format.icon_dirs defined.")
    }

    pub fn get_format_fallback_icon(&self) -> Option<String> {
        self.format
            .as_ref()
            .and_then(|f| f.fallback_icon.clone())
            .or_else(|| Format::default().fallback_icon)
    }

    pub fn is_layout_auto_tile(&self) -> bool {
        self.layout
            .as_ref()
            .and_then(|l| l.auto_tile)
            .or_else(|| Layout::default().auto_tile)
            .expect("No layout.auto_tile defined.")
    }

    pub fn get_layout_auto_tile_min_window_width_per_output_width_as_map(
        &self,
    ) -> HashMap<i32, i32> {
        self.layout.as_ref()
            .and_then(|l|l.auto_tile_min_window_width_per_output_width_as_map())
            .or_else(|| Layout::default().auto_tile_min_window_width_per_output_width_as_map())
            .expect("No layout.auto_tile_min_window_width_per_output_width defined.")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Menu {
    executable: Option<String>,
    args: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Format {
    output_format: Option<String>,
    workspace_format: Option<String>,
    container_format: Option<String>,
    window_format: Option<String>,
    indent: Option<String>,
    urgency_start: Option<String>,
    urgency_end: Option<String>,
    html_escape: Option<bool>,
    icon_dirs: Option<Vec<String>>,
    fallback_icon: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Layout {
    auto_tile: Option<bool>,
    auto_tile_min_window_width_per_output_width: Option<Vec<[i32; 2]>>,
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
                "--height=40%".to_string(),
                "--prompt={prompt}".to_string(),
            ]),
        }
    }
}

impl Default for Format {
    fn default() -> Self {
        Format {
            output_format: Some(
                "{indent}<b>Output {name}</b>    \
                 <span alpha=\"20000\">({id})</span>"
                    .to_string(),
            ),
            workspace_format: Some(
                "{indent}<b>Workspace {name} [{layout}]</b> \
                 on output {output_name}    \
                 <span alpha=\"20000\">({id})</span>"
                    .to_string(),
            ),
            container_format: Some(
                "{indent}<b>Container [{layout}]</b> \
                 <i>{marks}</i> \
                 on workspace {workspace_name}    \
                 <span alpha=\"20000\">({id})</span>"
                    .to_string(),
            ),
            window_format: Some(
                "img:{app_icon}:text:{indent}<i>{app_name}</i> — \
                 {urgency_start}<b>“{title}”</b>{urgency_end} \
                 <i>{marks}</i> \
                 on workspace {workspace_name} / {output_name}    \
                 <span alpha=\"20000\">({id})</span>"
                    .to_string(),
            ),
            indent: Some("    ".to_string()),
            html_escape: Some(true),
            urgency_start: Some(
                "<span background=\"darkred\" foreground=\"yellow\">"
                    .to_string(),
            ),
            urgency_end: Some("</span>".to_string()),
            icon_dirs: Some(vec![
                "/usr/share/icons/hicolor/scalable/apps".to_string(),
                "/usr/share/icons/hicolor/64x64/apps".to_string(),
                "/usr/share/icons/hicolor/48x48/apps".to_string(),
                "/usr/share/icons/Adwaita/64x64/apps".to_string(),
                "/usr/share/icons/Adwaita/48x48/apps".to_string(),
                "/usr/share/pixmaps".to_string(),
            ]),
            fallback_icon: None,
        }
    }
}

impl Default for Layout {
    fn default() -> Layout {
        let resolution_min_width_vec = vec![
            [800, 400],
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
    let user_config_dir = proj_dirs.config_dir();
    if !user_config_dir.exists() {
        let sys_config_file = Path::new("/etc/xdg/swayr/config.toml");
        if sys_config_file.exists() {
            return sys_config_file.into();
        }
        DirBuilder::new()
            .recursive(true)
            .create(user_config_dir)
            .unwrap();
    }
    user_config_dir.join("config.toml").into_boxed_path()
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
