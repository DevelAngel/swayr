// Copyright (C) 2022  Tassilo Horn <tsdh@gnu.org>
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

//! Provides runtime formatting of strings since our format strings are read
//! from the swayr config, not specified as literals, e.g., `{name:{:.30}}` in
//! `format.window_format`.

use rt_format::{
    Format, FormatArgument, NoNamedArguments, ParsedFormat, Specifier,
};
use std::fmt;

enum FmtArg<'a> {
    Str(&'a str),
}

impl<'a> FormatArgument for FmtArg<'a> {
    fn supports_format(&self, spec: &Specifier) -> bool {
        spec.format == Format::Display
    }

    fn fmt_display(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Str(val) => fmt::Display::fmt(&val, f),
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

impl<'a> std::convert::TryInto<usize> for &FmtArg<'a> {
    type Error = ();
    fn try_into(self) -> Result<usize, Self::Error> {
        Err(())
    }
}

pub fn format(fmt: &str, arg: &str, clipped_str: &str) -> String {
    let args = &[FmtArg::Str(arg)];

    if let Ok(pf) = ParsedFormat::parse(fmt, args, &NoNamedArguments) {
        let mut s = format!("{}", pf);

        if !clipped_str.is_empty() && !s.contains(arg) {
            remove_last_n_chars(&mut s, clipped_str.chars().count());
            s.push_str(clipped_str);
        }
        s
    } else {
        format!("Invalid format string: {}", fmt)
    }
}

fn remove_last_n_chars(s: &mut String, n: usize) -> () {
    match s.char_indices().nth_back(n) {
        Some((pos, ch)) => s.truncate(pos + ch.len_utf8()),
        None => s.clear(),
    }
}

#[test]
fn test_format() {
    assert_eq!(format("{:.10}", "sway", ""), "sway");
    assert_eq!(format("{:.10}", "sway", "…"), "sway");
    assert_eq!(format("{:.4}", "𝔰𝔴𝔞𝔶", "……"), "𝔰𝔴𝔞𝔶");

    assert_eq!(format("{:.3}", "sway", ""), "swa");
    assert_eq!(format("{:.3}", "sway", "…"), "sw…");
    assert_eq!(format("{:.5}", "𝔰𝔴𝔞𝔶 𝔴𝔦𝔫𝔡𝔬𝔴", "…?"), "𝔰𝔴𝔞…?");
    assert_eq!(format("{:.5}", "sway window", "..."), "sw...");
    assert_eq!(format("{:.2}", "sway", "..."), "...");
}
