// Copyright (C) 2021-2023  Tassilo Horn <tsdh@gnu.org>
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

/// Config file loading stuff.
use directories::ProjectDirs;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs::{DirBuilder, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

pub fn get_config_file_path(project: &str) -> Box<Path> {
    let proj_dirs = ProjectDirs::from("", "", project).expect("");
    let user_config_dir = proj_dirs.config_dir();
    if !user_config_dir.exists() {
        let sys_path = format!("/etc/xdg/{project}/config.toml");
        let sys_config_file = Path::new(sys_path.as_str());
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

pub fn save_config<T>(project: &str, cfg: T)
where
    T: Serialize,
{
    let path = get_config_file_path(project);
    let content =
        toml::to_string_pretty::<T>(&cfg).expect("Cannot serialize config.");
    let mut file = OpenOptions::new()
        .read(false)
        .write(true)
        .create(true)
        .open(path)
        .unwrap();
    file.write_all(content.as_str().as_bytes()).unwrap();
}

pub fn load_config<T>(project: &str) -> T
where
    T: Serialize + DeserializeOwned + Default,
{
    let path = get_config_file_path(project);
    if !path.exists() {
        save_config(project, T::default());
        // Tell the user that a fresh default config has been created.
        std::process::Command::new("swaynag")
            .arg("--background")
            .arg("00FF44")
            .arg("--text")
            .arg("0000CC")
            .arg("--message")
            .arg(
                if project == "swayr" {
                    "Welcome to swayr! ".to_owned()
                    + "I've created a fresh config for use with wofi for you in "
                    + &path.to_string_lossy()
                        + ". Adapt it to your needs."
                } else {
                    "Welcome to swayrbar! ".to_owned()
                    + "I've created a fresh config for for you in "
                    + &path.to_string_lossy()
                        + ". Adapt it to your needs."
                },
            )
            .arg("--type")
            .arg("warning")
            .arg("--dismiss-button")
            .arg("Thanks!")
            .spawn()
            .ok();
        log::debug!("Created new config in {}.", path.to_string_lossy());
    }

    load_config_file(&path)
}

pub fn load_config_file<T>(config_file: &Path) -> T
where
    T: Serialize + DeserializeOwned + Default,
{
    if !config_file.exists() {
        panic!(
            "Config file {} does not exist.",
            config_file.to_string_lossy()
        );
    } else {
        log::debug!("Loading config from {}.", config_file.to_string_lossy());
    }
    let mut file = OpenOptions::new()
        .read(true)
        .write(false)
        .create(false)
        .open(config_file)
        .unwrap();
    let mut buf: String = String::new();
    file.read_to_string(&mut buf).unwrap();
    match toml::from_str::<T>(&buf) {
        Ok(cfg) => cfg,
        Err(err) => {
            log::error!("Invalid config: {err}");
            log::error!("Using default configuration.");
            T::default()
        }
    }
}
