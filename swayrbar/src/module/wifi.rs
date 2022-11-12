use super::RefreshReason;
use crate::config;
use crate::module::BarModuleFn;
use crate::shared::fmt::subst_placeholders;
use once_cell::sync::Lazy;
use regex::Regex;
use std::sync::Mutex;
use swaybar_types as s;

struct State {
    cached_text: String,
    signal: Option<String>,
    name: Option<String>,
    bars: Option<String>,
}

pub enum WifiTool {
    Nmcli,
    Iwctl,
}

static IWCTL_CONN_NETWORK: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\s*Connected network\s+(.*?)\s*$").unwrap());
static IWCTL_RSSI: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\s*RSSI\s+(-\d+) dBm\s*$").unwrap());

impl WifiTool {
    fn run(&self) -> Result<String, String> {
        let cmd;
        let args;
        match self {
            WifiTool::Nmcli => {
                cmd = "nmcli";
                args = "-c no -g IN-USE,SSID,SIGNAL,BARS dev wifi".split(' ');
            }
            WifiTool::Iwctl => {
                cmd = "iwctl";
                args = "station wlan0 show".split(' ');
            }
        }
        let output = std::process::Command::new(cmd)
            .args(args)
            .output()
            .map_err(|e| {
                format!("Failed to run {}: {}", self.to_string(), e)
            })?;

        if !output.status.success() {
            return Err(format!(
                "{} failed with status code {}",
                self.to_string(),
                output.status.code().unwrap_or(-1)
            ));
        }

        Ok(String::from_utf8(output.stdout).unwrap())
    }

    fn run_and_set_state(&self, state: &mut State) {
        state.name = None;
        state.signal = None;
        state.bars = None;
        if let Ok(output) = self.run() {
            match self {
                WifiTool::Nmcli => {
                    if let Some(line) =
                        output.lines().find(|line| line.starts_with('*'))
                    {
                        let mut parts = line.split(':');
                        parts.next();
                        state.name = Some(parts.next().unwrap().to_string());
                        state.signal = Some(parts.next().unwrap().to_string());
                        state.bars = Some(parts.next().unwrap().to_string());
                    }
                }
                WifiTool::Iwctl => {
                    let mut signal = -100;
                    for line in output.lines() {
                        if let Some(c) = IWCTL_CONN_NETWORK.captures(line) {
                            state.name =
                                c.get(1).map(|m| m.as_str().to_owned());
                        } else if let Some(c) = IWCTL_RSSI.captures(line) {
                            if let Some(s) = c.get(1).map(|m| m.as_str()) {
                                signal = s.parse::<i32>().unwrap();
                                state.signal = Some(s.to_owned());
                            }
                        }
                    }

                    state.bars = if signal > -45 {
                        Some("▂▄▆█".to_owned())
                    } else if signal > -60 {
                        Some("▂▄▆_".to_owned())
                    } else if signal > -70 {
                        Some("▂▄__".to_owned())
                    } else if signal > -80 {
                        Some("▂___".to_owned())
                    } else {
                        Some("____".to_owned())
                    };
                }
            }
        }
    }

    fn get_signal_unit(&self) -> &str {
        match self {
            WifiTool::Nmcli => "%",
            WifiTool::Iwctl => "dBm",
        }
    }
}

impl ToString for WifiTool {
    fn to_string(&self) -> String {
        match self {
            WifiTool::Nmcli => String::from("nmcli"),
            WifiTool::Iwctl => String::from("iwctl"),
        }
    }
}

pub struct BarModuleWifi {
    tool: WifiTool,
    config: config::ModuleConfig,
    state: Mutex<State>,
}

fn subst_placeholders(
    fmt: &str,
    html_escape: bool,
    state: &State,
    unit: &str,
) -> String {
    subst_placeholders!(fmt, html_escape, {
        "name" => {
            match &state.name {
                None => "No wi-fi",
                Some(name) => name,
            }
        },
        "signal" => {
            match &state.signal {
                None => "".to_owned(),
                Some(signal) => " ".to_owned() + signal + unit,
            }
        },
        "bars" => {
            match &state.bars {
                None => "".to_owned(),
                Some(bars) => " ".to_owned() + bars,
            }
        },
    })
}

fn refresh_state(
    tool: &WifiTool,
    state: &mut State,
    fmt_str: &str,
    html_escape: bool,
) {
    tool.run_and_set_state(state);
    state.cached_text =
        subst_placeholders(fmt_str, html_escape, state, tool.get_signal_unit());
}

pub fn create(
    tool: WifiTool,
    config: config::ModuleConfig,
) -> Box<dyn BarModuleFn> {
    Box::new(BarModuleWifi {
        tool,
        config,
        state: Mutex::new(State {
            cached_text: String::new(),
            signal: None,
            name: None,
            bars: None,
        }),
    })
}

impl BarModuleFn for BarModuleWifi {
    fn default_config(instance: String) -> config::ModuleConfig
    where
        Self: Sized,
    {
        config::ModuleConfig {
            name: "nmcli or iwctl, choose one".to_owned(),
            instance,
            format: "📡 Wi-fi: {name}{bars}{signal}".to_owned(),
            html_escape: Some(false),
            on_click: None,
        }
    }

    fn get_config(&self) -> &config::ModuleConfig {
        &self.config
    }

    fn build(&self, nai: &Option<super::NameInstanceAndReason>) -> s::Block {
        let mut state = self.state.lock().expect("Could not lock state.");

        if self.should_refresh(nai, true, &[RefreshReason::ClickEvent]) {
            refresh_state(
                &self.tool,
                &mut state,
                &self.config.format,
                self.config.is_html_escape(),
            );
        }

        s::Block {
            name: Some(self.tool.to_string()),
            instance: Some(self.config.instance.clone()),
            full_text: state.cached_text.to_owned(),
            align: Some(s::Align::Left),
            markup: Some(s::Markup::Pango),
            short_text: None,
            color: None,
            background: None,
            border: None,
            border_top: None,
            border_bottom: None,
            border_left: None,
            border_right: None,
            min_width: None,
            urgent: None,
            separator: Some(true),
            separator_block_width: None,
        }
    }

    fn subst_args<'a>(&'a self, cmd: &'a [String]) -> Option<Vec<String>> {
        let state = self.state.lock().expect("Could not lock state.");
        Some(
            cmd.iter()
                .map(|arg| subst_placeholders(arg, false, &state, ""))
                .collect(),
        )
    }
}
