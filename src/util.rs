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

use crate::con::DisplayFormat;
use crate::config as cfg;
use std::collections::HashMap;
use std::io::Write;
use std::process as proc;

pub fn get_swayr_socket_path() -> String {
    let wayland_display = std::env::var("WAYLAND_DISPLAY");
    let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR");
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
        map.insert(s, c);
    }

    let menu_default = cfg::Menu::default();
    let menu_exec = cfg
        .menu
        .as_ref()
        .and_then(|l| l.executable.as_ref())
        .unwrap_or_else(|| menu_default.executable.as_ref().unwrap());
    let args: Vec<String> = cfg
        .menu
        .as_ref()
        .and_then(|l| l.args.as_ref())
        .unwrap_or_else(|| menu_default.args.as_ref().unwrap())
        .iter()
        .map(|a| a.replace("{prompt}", prompt))
        .collect();

    let mut menu = proc::Command::new(menu_exec)
        .args(args)
        .stdin(proc::Stdio::piped())
        .stdout(proc::Stdio::piped())
        .spawn()
        .expect(&("Error running ".to_owned() + menu_exec));

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
