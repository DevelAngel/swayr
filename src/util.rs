use std::io::Write;
use std::process as proc;

pub fn is_debug() -> bool {
    true
}

pub fn get_swayr_socket_path() -> String {
    format!("/run/user/{}/swayr-sock", users::get_current_uid())
}

pub fn wofi_select<'a, 'b, TS>(prompt: &'a str, choices: &'b Vec<TS>) -> Option<&'b TS>
where
    TS: std::fmt::Display + Sized,
{
    let mut map: std::collections::HashMap<String, &TS> = std::collections::HashMap::new();
    for c in choices.iter() {
        map.insert(format!("{}", c), c);
    }

    let mut wofi = proc::Command::new("wofi")
        .arg("--show=dmenu")
        .arg("--prompt")
        .arg(prompt)
        .stdin(proc::Stdio::piped())
        .stdout(proc::Stdio::piped())
        .spawn()
        .expect("Error running wofi!");

    {
        let stdin = wofi.stdin.as_mut().expect("Failed to open wofi stdin");
        for k in map.keys() {
            stdin
                .write_all(format!("{}\n", k).as_bytes())
                .expect("Failed to write to wofi stdin");
        }
    }

    let output = wofi.wait_with_output().expect("Failed to read stdout");
    let choice = String::from_utf8_lossy(&output.stdout);
    // FIXME: Remove trailing \n from choice.
    //println!("choice: {:?}", choice);
    map.get(&*choice).copied()
}

#[test]
fn test_wofi_select() {
    let choices = vec!["a", "b", "c"];
    let choice = wofi_select("Choose wisely", &choices);
    println!("choice: {:?}", choice);
    assert!(choice.is_some());
    assert!(choices.contains(choice.unwrap()));
}
