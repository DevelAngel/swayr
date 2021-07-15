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

//! Utility functions including selection between choices using a menu program.

use crate::config as cfg;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path as p;
use std::process as proc;

pub fn get_swayr_socket_path() -> String {
    let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR");
    let wayland_display = std::env::var("WAYLAND_DISPLAY");
    format!(
        "{}/swayr-{}.sock",
        match xdg_runtime_dir {
            Ok(val) => val,
            Err(_e) => {
                eprintln!("Couldn't get XDG_RUNTIME_DIR!");
                String::from("/tmp")
            }
        },
        match wayland_display {
            Ok(val) => val,
            Err(_e) => {
                eprintln!("Couldn't get WAYLAND_DISPLAY!");
                String::from("unknown")
            }
        }
    )
}

fn desktop_entry_folders() -> Vec<Box<p::Path>> {
    let mut dirs: Vec<Box<p::Path>> = vec![];

    // XDG_DATA_HOME/applications
    if let Some(dd) = directories::BaseDirs::new() {
        dirs.push(dd.data_local_dir().to_path_buf().into_boxed_path());
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

fn find_icon(icon_name: &str, icon_dirs: &[String]) -> Option<Box<p::Path>> {
    let p = p::Path::new(icon_name);
    if p.is_file() {
        println!("(1) Icon name '{}' -> {}", icon_name, p.display());
        return Some(p.to_path_buf().into_boxed_path());
    }

    for dir in icon_dirs {
        for ext in &["png", "svg"] {
            let mut pb = p::PathBuf::from(dir);
            pb.push(icon_name.to_owned() + "." + ext);
            let icon_file = pb.as_path();
            if icon_file.is_file() {
                println!(
                    "(2) Icon name '{}' -> {}",
                    icon_name,
                    icon_file.display()
                );
                return Some(icon_file.to_path_buf().into_boxed_path());
            }
        }
    }

    println!("(3) No icon for name {}", icon_name);
    None
}

lazy_static! {
    static ref WM_CLASS_OR_ICON_RX: regex::Regex =
        regex::Regex::new("(StartupWMClass|Icon)=(.+)").unwrap();
    static ref REV_DOMAIN_NAME_RX: regex::Regex =
        regex::Regex::new(r"^(?:[a-zA-Z0-9-]+\.)+([a-zA-Z0-9-]+)$").unwrap();
}

fn get_app_id_to_icon_map(
    icon_dirs: &[String],
) -> HashMap<String, Box<p::Path>> {
    let mut map: HashMap<String, Box<p::Path>> = HashMap::new();

    for e in desktop_entries() {
        if let Ok(f) = std::fs::File::open(&e) {
            let buf = std::io::BufReader::new(f);
            let mut wm_class: Option<String> = None;
            let mut icon: Option<Box<p::Path>> = None;

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

    println!(
        "Desktop entries to icon files ({} entries):\n{:#?}",
        map.len(),
        map
    );
    map
}

lazy_static! {
    static ref APP_ID_TO_ICON_MAP: std::sync::Mutex<Option<HashMap<String, Box<p::Path>>>> =
        std::sync::Mutex::new(None);
}

pub fn get_icon(app_id: &str, icon_dirs: &[String]) -> Option<Box<p::Path>> {
    let mut opt = APP_ID_TO_ICON_MAP.lock().unwrap();

    if opt.is_none() {
        opt.replace(get_app_id_to_icon_map(icon_dirs));
    }

    opt.as_ref().unwrap().get(app_id).map(|i| i.to_owned())
}

#[test]
fn test_icon_stuff() {
    let icon_dirs = vec![
        String::from("/usr/share/icons/hicolor/scalable/apps"),
        String::from("/usr/share/icons/hicolor/48x48/apps"),
        String::from("/usr/share/icons/Adwaita/48x48/apps"),
        String::from("/usr/share/pixmaps"),
    ];
    let m = get_app_id_to_icon_map(&icon_dirs);
    println!("Found {} icon entries:\n{:#?}", m.len(), m);

    let apps = vec!["Emacs", "Alacritty", "firefoxdeveloperedition", "gimp"];
    for app in apps {
        println!("Icon for {}: {:?}", app, get_icon(app, &icon_dirs))
    }
}

pub trait DisplayFormat {
    fn format_for_display(&self, config: &cfg::Config) -> String;
}

pub fn select_from_menu<'a, 'b, TS>(
    prompt: &'a str,
    choices: &'b [TS],
) -> Option<&'b TS>
where
    TS: DisplayFormat + Sized,
{
    let mut map: HashMap<String, &TS> = HashMap::new();
    let mut strs: Vec<String> = vec![];
    let cfg = cfg::load_config();
    for c in choices {
        let s = c.format_for_display(&cfg);
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

    let menu_exec = cfg.get_menu_executable();
    let args: Vec<String> = cfg
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
        println!("Menu program {} input:\n{}", menu_exec, input);
        stdin
            .write_all(input.as_bytes())
            .expect("Failed to write to the menu program's stdin");
    }

    let output = menu.wait_with_output().expect("Failed to read stdout");
    let choice = String::from_utf8_lossy(&output.stdout);
    let mut choice = String::from(choice);
    choice.pop(); // Remove trailing \n from choice.
    map.get(&choice).copied()
}
