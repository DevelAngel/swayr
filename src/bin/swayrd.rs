//! The `swayrd` binary.

extern crate serde;
extern crate serde_json;

use swayr::demon;

fn main() {
    demon::run_demon();
}
