extern crate serde;
extern crate serde_json;

use serde_json::Deserializer;
use std::collections::HashMap;
use std::process as proc;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use swayr::ipc;

fn main() {
    let win_props: Arc<RwLock<HashMap<ipc::Id, ipc::WindowProps>>> =
        Arc::new(RwLock::new(HashMap::new()));

    let subscriber_handle = thread::spawn(|| {
        let x = proc::Command::new("swaymsg")
            .arg("-t")
            .arg("subscribe")
            .arg("[window]")
            .output()
            .expect("Failed to subscribe to window events");
        println!(
            "{}",
            String::from_utf8(x.stdout).expect("Wrong string data!")
        );
        let child = proc::Command::new("swaymsg")
            .arg("--monitor")
            .arg("-t")
            .arg("subscribe")
            .arg("'[window]'")
            .stdout(proc::Stdio::piped())
            .spawn()
            .expect("Failed to subscribe to window events");
        let stdout: std::process::ChildStdout = child.stdout.unwrap();
        // TODO: Before the WindowEvents, there's one Reply.  How to read that?
        let stream = Deserializer::from_reader(stdout).into_iter::<ipc::WindowEvent>();
        for res in stream {
            match res {
                Ok(msg) => println!("Got msg: {:?}", msg),
                Err(err) => panic!("{:?}", err),
            }
        }
    });

    subscriber_handle.join();
}
