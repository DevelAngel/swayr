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

pub fn format(fmt: &str, arg: &str, ellipsis: bool) -> String {
    let args = &[FmtArg::Str(arg)];

    if let Ok(pf) = ParsedFormat::parse(fmt, args, &NoNamedArguments) {
        let mut s = format!("{}", pf);

        if ellipsis && !s.contains(arg) {
            s.pop();
            s.push('…');
        }
        s
    } else {
        format!("Invalid format string: {}", fmt)
    }
}

#[test]
fn test_format() {
    assert_eq!(format("{:.10}", "sway", false), "sway");
    assert_eq!(format("{:.10}", "sway", true), "sway");
    assert_eq!(format("{:.4}", "𝔰𝔴𝔞𝔶", true), "𝔰𝔴𝔞𝔶");

    assert_eq!(format("{:.3}", "sway", false), "swa");
    assert_eq!(format("{:.3}", "sway", true), "sw…");
    assert_eq!(format("{:.3}", "𝔰𝔴𝔞𝔶", true), "𝔰𝔴…");
}
