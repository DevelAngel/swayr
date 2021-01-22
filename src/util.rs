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

pub fn swaymsg(args: Vec<&str>) -> String {
    let mut cmd = proc::Command::new("swaymsg");
    for a in args {
        cmd.arg(a);
    }

    let output = cmd.output().expect("Error running swaymsg!");
    String::from_utf8(output.stdout).unwrap()
}

pub fn wofi_select<'a, 'b, TS>(
    prompt: &'a str,
    choices: &'b [TS],
) -> Option<&'b TS>
where
    TS: std::fmt::Display + Sized,
{
    let mut map: HashMap<String, &TS> = HashMap::new();
    let mut strs: Vec<String> = vec![];
    for c in choices {
        let s = format!("{}", c);
        strs.push(String::from(s.as_str()));
        map.insert(s, c);
    }

    let mut wofi = proc::Command::new("wofi")
        .arg("--show=dmenu")
        .arg("--allow-markup")
        .arg("--allow-images")
        .arg("--insensitive")
        .arg("--cache-file=/dev/null")
        .arg("--prompt")
        .arg(prompt)
        .stdin(proc::Stdio::piped())
        .stdout(proc::Stdio::piped())
        .spawn()
        .expect("Error running wofi!");

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

#[test]
#[ignore = "interactive test requiring user input"]
fn test_wofi_select() {
    let choices = vec!["a", "b", "c"];
    let choice = wofi_select("Choose wisely", &choices);
    assert!(choice.is_some());
    assert!(choices.contains(choice.unwrap()));
}
