// Copyright (C) 2022-2023  Tassilo Horn <tsdh@gnu.org>
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

use once_cell::sync::Lazy;
use regex::Regex;
use rt_format::{
    Format, FormatArgument, NoNamedArguments, ParsedFormat, Specifier,
};
use std::fmt::{self, Display};

pub enum FmtArg {
    I64(i64),
    I32(i32),
    U8(u8),
    F64(f64),
    F32(f32),
    String(String),
}

impl From<i64> for FmtArg {
    fn from(x: i64) -> FmtArg {
        FmtArg::I64(x)
    }
}

impl From<i32> for FmtArg {
    fn from(x: i32) -> FmtArg {
        FmtArg::I32(x)
    }
}

impl From<u8> for FmtArg {
    fn from(x: u8) -> FmtArg {
        FmtArg::U8(x)
    }
}

impl From<f64> for FmtArg {
    fn from(x: f64) -> FmtArg {
        FmtArg::F64(x)
    }
}

impl From<f32> for FmtArg {
    fn from(x: f32) -> FmtArg {
        FmtArg::F32(x)
    }
}

impl From<&str> for FmtArg {
    fn from(x: &str) -> FmtArg {
        FmtArg::String(x.to_string())
    }
}

impl From<String> for FmtArg {
    fn from(x: String) -> FmtArg {
        FmtArg::String(x)
    }
}

impl fmt::Display for FmtArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FmtArg::String(x) => x.fmt(f),
            FmtArg::I64(x) => x.fmt(f),
            FmtArg::I32(x) => x.fmt(f),
            FmtArg::U8(x) => x.fmt(f),
            FmtArg::F64(x) => x.fmt(f),
            FmtArg::F32(x) => x.fmt(f),
        }
    }
}

impl FormatArgument for FmtArg {
    fn supports_format(&self, spec: &Specifier) -> bool {
        spec.format == Format::Display
    }

    fn fmt_display(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::String(val) => val.fmt(f),
            Self::I64(val) => val.fmt(f),
            Self::I32(val) => val.fmt(f),
            Self::U8(val) => val.fmt(f),
            Self::F64(val) => val.fmt(f),
            Self::F32(val) => val.fmt(f),
        }
    }

    fn fmt_debug(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        Err(fmt::Error)
    }

    fn fmt_octal(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        Err(fmt::Error)
    }

    fn fmt_lower_hex(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        Err(fmt::Error)
    }

    fn fmt_upper_hex(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        Err(fmt::Error)
    }

    fn fmt_binary(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        Err(fmt::Error)
    }

    fn fmt_lower_exp(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        Err(fmt::Error)
    }

    fn fmt_upper_exp(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        Err(fmt::Error)
    }
}

pub fn rt_format(fmt: &str, arg: FmtArg, clipped_str: &str) -> String {
    let arg_string = arg.to_string();

    if let Ok(pf) = ParsedFormat::parse(fmt, &[arg], &NoNamedArguments) {
        let mut s = format!("{pf}");

        if !clipped_str.is_empty() && !s.contains(arg_string.as_str()) {
            remove_last_n_chars(&mut s, clipped_str.chars().count());
            s.push_str(clipped_str);
        }
        s
    } else {
        format!("Invalid format string: {fmt}")
    }
}

fn remove_last_n_chars(s: &mut String, n: usize) {
    match s.char_indices().nth_back(n) {
        Some((pos, ch)) => s.truncate(pos + ch.len_utf8()),
        None => s.clear(),
    }
}

#[test]
fn test_format() {
    assert_eq!(rt_format("{:.10}", FmtArg::from("sway"), ""), "sway");
    assert_eq!(rt_format("{:.10}", FmtArg::from("sway"), "…"), "sway");
    assert_eq!(rt_format("{:.4}", FmtArg::from("𝔰𝔴𝔞𝔶"), "……"), "𝔰𝔴𝔞𝔶");
    assert_eq!(rt_format("{:.3}", FmtArg::from("sway"), ""), "swa");
    assert_eq!(rt_format("{:.3}", FmtArg::from("sway"), "…"), "sw…");
    assert_eq!(
        rt_format("{:.5}", FmtArg::from("𝔰𝔴𝔞𝔶 𝔴𝔦𝔫𝔡𝔬𝔴"), "…?"),
        "𝔰𝔴𝔞…?"
    );
    assert_eq!(
        rt_format("{:.5}", FmtArg::from("sway window"), "..."),
        "sw..."
    );
    assert_eq!(rt_format("{:.2}", FmtArg::from("sway"), "..."), "...");
}

pub static PLACEHOLDER_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"\{(?P<name>[^}:]+)(?::(?P<fmtstr>\{[^}]*\})(?P<clipstr>[^}]*))?\}",
    )
    .unwrap()
});

#[test]
fn test_placeholder_rx() {
    let caps = PLACEHOLDER_RX.captures("Hello, {place}!").unwrap();
    assert_eq!(caps.name("name").unwrap().as_str(), "place");
    assert_eq!(caps.name("fmtstr"), None);
    assert_eq!(caps.name("clipstr"), None);

    let caps = PLACEHOLDER_RX.captures("Hi, {place:{:>10.10}}!").unwrap();
    assert_eq!(caps.name("name").unwrap().as_str(), "place");
    assert_eq!(caps.name("fmtstr").unwrap().as_str(), "{:>10.10}");
    assert_eq!(caps.name("clipstr").unwrap().as_str(), "");

    let caps = PLACEHOLDER_RX.captures("Hello, {place:{:.5}…}!").unwrap();
    assert_eq!(caps.name("name").unwrap().as_str(), "place");
    assert_eq!(caps.name("fmtstr").unwrap().as_str(), "{:.5}");
    assert_eq!(caps.name("clipstr").unwrap().as_str(), "…");

    let caps = PLACEHOLDER_RX.captures("Hello, {place:{:.5}...}!").unwrap();
    assert_eq!(caps.name("name").unwrap().as_str(), "place");
    assert_eq!(caps.name("fmtstr").unwrap().as_str(), "{:.5}");
    assert_eq!(caps.name("clipstr").unwrap().as_str(), "...");
}

pub fn maybe_html_escape(do_it: bool, text: String) -> String {
    if do_it {
        text.replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('&', "&amp;")
    } else {
        text
    }
}

macro_rules! subst_placeholders {
    ( $fmt_str:expr, $html_escape:expr,
      { $( $($pat:pat_param)|+ => $exp:expr, )+ }
    ) => {
        $crate::shared::fmt::PLACEHOLDER_RX
            .replace_all($fmt_str, |caps: &regex::Captures| {
                let value: String = match &caps["name"] {
                    $(
                        $( $pat )|+ => {
                            let val = $crate::shared::fmt::FmtArg::from($exp);
                            let fmt_str = caps.name("fmtstr")
                                .map_or("{}", |m| m.as_str());
                            let clipped_str = caps.name("clipstr")
                                .map_or("", |m| m.as_str());
                            $crate::shared::fmt::maybe_html_escape(
                                $html_escape,
                                $crate::shared::fmt::rt_format(fmt_str, val, clipped_str),
                            )
                        }
                    )+
                        _ => caps[0].to_string(),
                };
                value
            }).into()
    };
}

pub(crate) use subst_placeholders;

#[test]
fn test_subst_placeholders() {
    let fmt_str = "{a}, {b} = {d}";
    let html_escape = true;
    let x: String = subst_placeholders!(fmt_str, html_escape, {
        "a" => "1".to_string(),
        "b" | "d" =>  "2".to_string(),
        "c" => "3".to_owned(),
    });

    assert_eq!("1, 2 = 2", x);
}
