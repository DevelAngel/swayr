use crate::config;
use crate::module::BarModuleFn;
use crate::shared::fmt::subst_placeholders;
use std::sync::Mutex;
use swaybar_types as s;

use super::RefreshReason;

const NAME: &str = "nmcli";

struct State {
    cached_text: String,
    signal: Option<String>,
    name: Option<String>,
    bars: Option<String>,
}

pub struct BarModuleNmcli {
    config: config::ModuleConfig,
    state: Mutex<State>,
}

fn run_nmcli() -> Result<String, String> {
    let cmd = "nmcli";
    let args = "-c no -g IN-USE,SSID,SIGNAL,BARS dev wifi".split(" ");
    let output = std::process::Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run nmcli: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "nmcli failed with status code {}",
            output.status.code().unwrap_or(-1)
        ));
    }

    Ok(String::from_utf8(output.stdout).unwrap())
}

fn subst_placeholders(fmt: &str, html_escape: bool, state: &State) -> String {
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
                Some(signal) => " ".to_owned() + signal + "%",
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

fn refresh_state(state: &mut State, fmt_str: &str, html_escape: bool) {
    if let Ok(output) = run_nmcli() {
        state.name = None;
        state.signal = None;
        state.bars = None;
        if let Some(line) = output.lines().find(|line| line.starts_with("*")) {
            let mut parts = line.split(":");
            parts.next();
            state.name = Some(parts.next().unwrap().to_string());
            state.signal = Some(parts.next().unwrap().to_string());
            state.bars = Some(parts.next().unwrap().to_string());
        }
    }
    state.cached_text = subst_placeholders(fmt_str, html_escape, state);
}

impl BarModuleFn for BarModuleNmcli {
    fn create(config: config::ModuleConfig) -> Box<dyn BarModuleFn>
    where
        Self: Sized,
    {
        Box::new(BarModuleNmcli {
            config,
            state: Mutex::new(State {
                cached_text: String::new(),
                signal: None,
                name: None,
                bars: None,
            }),
        })
    }

    fn default_config(instance: String) -> config::ModuleConfig
    where
        Self: Sized,
    {
        config::ModuleConfig {
            name: NAME.to_owned(),
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
                &mut state,
                &self.config.format,
                self.config.is_html_escape(),
            );
        }

        s::Block {
            name: Some(NAME.to_owned()),
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
                .map(|arg| subst_placeholders(arg, false, &state))
                .collect(),
        )
    }
}
