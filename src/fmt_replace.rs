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

use once_cell::sync::Lazy;
use regex::Regex;

pub static PLACEHOLDER_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"\{(?P<name>[^}:]+)(?::(?P<fmtstr>\{[^}]*\})(?P<clipstr>[^}]*))?\}",
    )
    .unwrap()
});

pub fn maybe_html_escape(do_it: bool, text: String) -> String {
    if do_it {
        text.replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('&', "&amp;")
    } else {
        text
    }
}

macro_rules! fmt_replace {
    ( $fmt_str:expr, $html_escape:ident,
      { $( $($pat:pat)|+ => $exp:expr, )+ }
    ) => {
        $crate::fmt_replace::PLACEHOLDER_RX
            .replace_all($fmt_str, |caps: &regex::Captures| {
                let value: String = match &caps["name"] {
                    $(
                        $( $pat )|+ => {
                            let val = $crate::rtfmt::FmtArg::from($exp);
                            let fmt_str = caps.name("fmtstr")
                                .map_or("{}", |m| m.as_str());
                            let clipped_str = caps.name("clipstr")
                                .map_or("", |m| m.as_str());
                            $crate::fmt_replace::maybe_html_escape(
                                $html_escape,
                                $crate::rtfmt::format(fmt_str, val, clipped_str),
                            )
                        }
                    )+
                        _ => caps[0].to_string(),
                };
                value
            }).into()
    };
}

pub(crate) use fmt_replace;

#[test]
fn foo() {
    let foo = "{a}, {b}";
    let html_escape = true;
    let x: String = fmt_replace!(foo, html_escape, {
        "a" => "1".to_string(),
        "b" =>  "2".to_string(),
        "c" => "3".to_owned(),
    });
}
