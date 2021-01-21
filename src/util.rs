use crate::window;
use std::io::Write;
use std::process as proc;

pub fn is_debug() -> bool {
    true
}

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

pub fn swaymsg(args: Vec<&str>) -> String {
    let mut cmd = proc::Command::new("swaymsg");
    for a in args {
        cmd.arg(a);
    }

    let output = cmd.output().expect("Error running swaymsg!");
    String::from_utf8(output.stdout).unwrap()
}

pub fn select_window<'a>(windows: &'a Vec<window::Window>) -> Option<&'a window::Window<'a>> {
    wofi_select("Select window", windows)
}

pub fn wofi_select<'a, 'b, TS>(prompt: &'a str, choices: &'b Vec<TS>) -> Option<&'b TS>
where
    TS: std::fmt::Display + Sized,
{
    let mut map: std::collections::HashMap<String, &TS> = std::collections::HashMap::new();
    for c in choices {
        map.insert(format!("{}", c), c);
    }

    let mut wofi = proc::Command::new("wofi")
        .arg("--show=dmenu")
        .arg("--allow-markup")
        .arg("--allow-images")
        .arg("--insensitive")
        .arg("--prompt")
        .arg(prompt)
        .stdin(proc::Stdio::piped())
        .stdout(proc::Stdio::piped())
        .spawn()
        .expect("Error running wofi!");

    {
        let stdin = wofi.stdin.as_mut().expect("Failed to open wofi stdin");
        for c in choices {
            stdin
                .write_all(format!("{}\n", c).as_bytes())
                .expect("Failed to write to wofi stdin");
        }
    }

    let output = wofi.wait_with_output().expect("Failed to read stdout");
    let choice = String::from_utf8_lossy(&output.stdout);
    let mut choice = String::from(choice);
    choice.pop(); // Remove trailing \n from choice.
    map.get(&choice).copied()
}

#[test]
#[ignore = "interactive test requiring user input"]
fn test_wofi_select() {
    let choices = vec!["a", "b", "c"];
    let choice = wofi_select("Choose wisely", &choices);
    assert!(choice.is_some());
    assert!(choices.contains(choice.unwrap()));
}
