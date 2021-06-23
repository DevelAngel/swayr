#[derive(Serialize, Deserialize)]
pub struct Config {
    launcher: Option<Launcher>,
    format: Option<Format>,
}

#[derive(Serialize, Deserialize)]
pub struct Launcher {
    executable: Option<String>,
    args: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
pub struct Format {
    window_format: Option<String>,
    workspace_format: Option<String>,
}
