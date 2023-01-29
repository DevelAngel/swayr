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

//! Utility functions including selection between choices using a menu program.

use once_cell::sync::Lazy;
use regex::Regex;

use crate::daemon::CONFIG;
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path as p;
use std::process as proc;

pub fn get_swayr_socket_path() -> String {
    // We prefer checking the env variable instead of
    // directories::BaseDirs::new().unwrap().runtime_dir().unwrap() because
    // directories errors if the XDG_RUNTIME_DIR isn't set or set to a relative
    // path which actually works fine for sway & swayr.
    let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR");
    let wayland_display = std::env::var("WAYLAND_DISPLAY");
    format!(
        "{}/swayr-{}.sock",
        match xdg_runtime_dir {
            Ok(val) => val,
            Err(_e) => {
                log::error!("Couldn't get XDG_RUNTIME_DIR!");
                String::from("/tmp")
            }
        },
        match wayland_display {
            Ok(val) => val,
            Err(_e) => {
                log::error!("Couldn't get WAYLAND_DISPLAY!");
                String::from("unknown")
            }
        }
    )
}

fn desktop_entry_folders() -> Vec<Box<p::Path>> {
    let mut dirs: Vec<Box<p::Path>> = vec![];

    // XDG_DATA_HOME/applications
    if let Some(dd) = directories::BaseDirs::new() {
        let mut pb = dd.data_local_dir().to_path_buf();
        pb.push("applications/");
        dirs.push(pb.into_boxed_path());
    }

    let default_dirs =
        ["/usr/local/share/applications/", "/usr/share/applications/"];
    for dir in default_dirs {
        dirs.push(p::Path::new(dir).to_path_buf().into_boxed_path());
    }

    if let Ok(xdg_data_dirs) = std::env::var("XDG_DATA_DIRS") {
        for mut dir in std::env::split_paths(&xdg_data_dirs) {
            dir.push("applications/");
            dirs.push(dir.into_boxed_path());
        }
    }

    dirs.sort();
    dirs.dedup();

    for path in &dirs {
        log::debug!("found desktop entry folder: {}", path.display());
    }

    dirs
}

fn desktop_entries() -> Vec<Box<p::Path>> {
    let mut entries = vec![];
    for dir in desktop_entry_folders() {
        if let Ok(readdir) = dir.read_dir() {
            for entry in readdir.flatten() {
                let path = entry.path();
                if path.is_file()
                    && path.extension().map(|ext| ext == "desktop")
                        == Some(true)
                {
                    entries.push(path.to_path_buf().into_boxed_path());
                }
            }
        }
    }
    entries
}

fn find_icon(icon_name: &str, icon_dirs: &[String]) -> Option<p::PathBuf> {
    let p = p::Path::new(icon_name);
    if p.is_file() {
        log::debug!("(1) Icon name '{}' -> {}", icon_name, p.display());
        return Some(p.to_path_buf());
    }

    for dir in icon_dirs {
        for ext in &["png", "svg"] {
            let mut pb = p::PathBuf::from(dir);
            pb.push(icon_name.to_owned() + "." + ext);
            let icon_file = pb.as_path();
            if icon_file.is_file() {
                log::debug!(
                    "(2) Icon name '{}' -> {}",
                    icon_name,
                    icon_file.display()
                );
                return Some(icon_file.to_path_buf());
            }
        }
    }

    log::debug!("(3) No icon for name {}", icon_name);
    None
}

static WM_CLASS_OR_ICON_RX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(StartupWMClass|Icon)=(.+)").unwrap());
static REV_DOMAIN_NAME_RX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(?:[a-zA-Z0-9-]+\.)+([a-zA-Z0-9-]+)$").unwrap());

pub fn get_app_id_to_icon_map(
    icon_dirs: &[String],
) -> HashMap<String, p::PathBuf> {
    let mut map: HashMap<String, p::PathBuf> = HashMap::new();

    for e in desktop_entries() {
        if let Ok(f) = std::fs::File::open(&e) {
            let buf = std::io::BufReader::new(f);
            let mut wm_class: Option<String> = None;
            let mut icon: Option<p::PathBuf> = None;

            // Get App-Id and Icon from desktop file.
            for line in buf.lines() {
                if wm_class.is_some() && icon.is_some() {
                    break;
                }
                if let Ok(line) = line {
                    if let Some(cap) = WM_CLASS_OR_ICON_RX.captures(&line) {
                        if "StartupWMClass" == cap.get(1).unwrap().as_str() {
                            wm_class.replace(
                                cap.get(2).unwrap().as_str().to_string(),
                            );
                        } else if let Some(icon_file) =
                            find_icon(cap.get(2).unwrap().as_str(), icon_dirs)
                        {
                            icon.replace(icon_file);
                        }
                    }
                }
            }

            if let Some(icon) = icon {
                // Sometimes the StartupWMClass is the app_id, e.g. FF Dev
                // Edition has StartupWMClass firefoxdeveloperedition although
                // the desktop file is named firefox-developer-edition.
                if let Some(wm_class) = wm_class {
                    map.insert(wm_class, icon.clone());
                }

                // Some apps have a reverse domain name desktop file, e.g.,
                // org.gnome.eog.desktop but reports as just eog.
                let desktop_file_name = String::from(
                    e.with_extension("").file_name().unwrap().to_string_lossy(),
                );
                if let Some(caps) =
                    REV_DOMAIN_NAME_RX.captures(&desktop_file_name)
                {
                    map.insert(
                        caps.get(1).unwrap().as_str().to_string(),
                        icon.clone(),
                    );
                }

                // The usual case is that the app with foo.desktop also has the
                // app_id foo.
                map.insert(desktop_file_name.clone(), icon);
            }
        }
    }

    log::debug!(
        "Desktop entries to icon files ({} entries):\n{:#?}",
        map.len(),
        map
    );
    map
}

pub trait DisplayFormat {
    fn format_for_display(&self) -> String;
    fn get_indent_level(&self) -> usize;
}

pub fn select_from_menu<'b, TS>(
    prompt: &str,
    choices: &'b [TS],
) -> Result<&'b TS, String>
where
    TS: DisplayFormat + Sized,
{
    let mut map: HashMap<String, &TS> = HashMap::new();
    let mut strs: Vec<String> = vec![];
    for c in choices {
        let s = c.format_for_display();
        strs.push(s.clone());

        // Workaround: rofi has "\u0000icon\u001f/path/to/icon.png" as image
        // escape sequence which comes after the actual text but returns only
        // the text, not the escape sequence.
        if s.contains('\0') {
            if let Some(prefix) = s.split('\0').next() {
                map.insert(prefix.to_string(), c);
            }
        }

        map.insert(s, c);
    }

    let menu_exec = CONFIG.get_menu_executable();
    let args: Vec<String> = CONFIG
        .get_menu_args()
        .iter()
        .map(|a| a.replace("{prompt}", prompt))
        .collect();

    let mut menu = proc::Command::new(&menu_exec)
        .args(args)
        .stdin(proc::Stdio::piped())
        .stdout(proc::Stdio::piped())
        .spawn()
        .expect(&("Error running ".to_owned() + &menu_exec));

    {
        let stdin = menu
            .stdin
            .as_mut()
            .expect("Failed to open the menu program's stdin");
        let input = strs.join("\n");
        //log::debug!("Menu program {} input:\n{}", menu_exec, input);
        stdin
            .write_all(input.as_bytes())
            .expect("Failed to write to the menu program's stdin");
    }

    let output = menu.wait_with_output().expect("Failed to read stdout");
    let choice = String::from_utf8_lossy(&output.stdout);
    let mut choice = String::from(choice);
    choice.pop(); // Remove trailing \n from choice.
    map.get(&choice).copied().ok_or(choice)
}
