//! Utility functions including wofi-selection.

use crate::con::DisplayFormat;
use crate::config as cfg;
use std::collections::HashMap;
use std::io::Write;
use std::process as proc;

pub fn get_swayr_socket_path() -> String {
    let wayland_display = std::env::var("WAYLAND_DISPLAY");
    format!(
        "/run/user/{}/swayr-{}.sock",
        users::get_current_uid(),
        match wayland_display {
            Ok(val) => val,
            Err(_e) => {
                eprintln!("Couldn't get WAYLAND_DISPLAY!");
                String::from("unknown")
            }
        }
    )
}

pub fn wofi_select<'a, 'b, TS>(
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

    let default = cfg::Config::default();
    let launcher = cfg
        .launcher
        .as_ref()
        .and_then(|l| l.executable.as_ref())
        .unwrap_or_else(|| {
            default
                .launcher
                .as_ref()
                .unwrap()
                .executable
                .as_ref()
                .unwrap()
        });
    let args: Vec<String> = cfg
        .launcher
        .as_ref()
        .and_then(|l| l.args.as_ref())
        .unwrap_or_else(|| {
            default.launcher.as_ref().unwrap().args.as_ref().unwrap()
        })
        .iter()
        .map(|a| a.replace("{prompt}", prompt))
        .collect();

    let mut wofi = proc::Command::new(launcher)
        .args(args)
        .stdin(proc::Stdio::piped())
        .stdout(proc::Stdio::piped())
        .spawn()
        .expect(&("Error running ".to_owned() + launcher));

    {
        let stdin = wofi.stdin.as_mut().expect("Failed to open wofi stdin");
        let wofi_input = strs.join("\n");
        println!("Wofi input:\n{}", wofi_input);
        stdin
            .write_all(wofi_input.as_bytes())
            .expect("Failed to write to wofi stdin");
    }

    let output = wofi.wait_with_output().expect("Failed to read stdout");
    let choice = String::from_utf8_lossy(&output.stdout);
    let mut choice = String::from(choice);
    choice.pop(); // Remove trailing \n from choice.
    map.get(&choice).copied()
}
