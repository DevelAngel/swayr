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

//! Implementation of sway's criteria API.

use regex::Regex;

use crate::{shared::ipc::NodeMethods, tree as t};

#[derive(Debug)]
pub enum RegexOrFocused {
    Regex(Regex),
    Focused,
}

#[derive(Debug)]
pub enum I64OrFocused {
    I64(i64),
    Focused,
}

#[derive(Debug)]
pub enum Criterion {
    AppId(RegexOrFocused),
    Class(RegexOrFocused),
    Instance(RegexOrFocused),
    /// Not specified by sway: matched against either app_id or class,
    /// depending on if the window is a wayland or X11 window.
    AppName(RegexOrFocused),
    Title(RegexOrFocused),
    ConMark(Regex),
    ConId(I64OrFocused),
    Pid(i32),
    Floating,
    Tiling,
    // TODO: There are more...
}

fn regex_from_str(s: &str) -> Regex {
    match Regex::new(s) {
        Ok(rx) => rx,
        Err(err) => {
            log::error!("Invalid regex {:?}: {}", s, err);
            Regex::new("^__I_WONT_MATCH_A_💩__$").unwrap()
        }
    }
}

peg::parser! {
    grammar criteria_parser() for str {
        rule space() -> () = [' ' | '\t']* {}
        rule i32_literal() -> i32 =
            n:$(['-']?['0'..='9']+) {? n.parse().or(Err("i32")) }
        rule i64_literal() -> i64 =
            n:$(['-']?['0'..='9']+) {? n.parse().or(Err("i64")) }
        rule string_literal() -> String =
            "\"" s:[^'"']* "\"" { s.into_iter().collect() }

        rule rx_focused() -> RegexOrFocused = "__focused__" {RegexOrFocused::Focused}

        rule regex_or_focused() -> RegexOrFocused =
            rx_focused()
            / s:string_literal() {RegexOrFocused::Regex(regex_from_str(&s))}

        rule i64_focused() -> I64OrFocused = "__focused__" {I64OrFocused::Focused}
        rule i64_or_focused() -> I64OrFocused =
            i64_focused() / n:i64_literal() {I64OrFocused::I64(n)}

        rule tiling() -> Criterion = "tiling" {Criterion::Tiling}
        rule floating() -> Criterion = "floating" {Criterion::Floating}
        rule app_id() -> Criterion = "app_id" space() "=" space()
            rof:regex_or_focused() {Criterion::AppId(rof)}
        rule app_name() -> Criterion = "app_name" space() "=" space()
            rof:regex_or_focused() {Criterion::AppName(rof)}
        rule class() -> Criterion = "class" space() "=" space()
            rof:regex_or_focused() {Criterion::Class(rof)}
        rule instance() -> Criterion = "instance" space() "=" space()
            rof:regex_or_focused() {Criterion::Instance(rof)}
        rule title() -> Criterion = "title" space() "=" space()
            rof:regex_or_focused() {Criterion::Title(rof)}
        rule con_mark() -> Criterion = "con_mark" space() "=" space()
            s:string_literal() {Criterion::ConMark(regex_from_str(&s))}
        rule con_id() -> Criterion = "con_id" space() "=" space()
            i:i64_or_focused() {Criterion::ConId(i)}
        rule pid() -> Criterion = "pid" space() "=" space()
            n:i32_literal() {Criterion::Pid(n)}

        rule criterion() -> Criterion =
            tiling() / floating()
            / app_id() / class() / instance() / app_name() / title()
            / con_mark()
            / con_id()
            / pid()

        pub rule criteria() -> Criteria =
            "[" space() l:(criterion() ** space()) space() "]" { l }
  }
}

pub fn parse_criteria(criteria: &str) -> Option<Criteria> {
    match criteria_parser::criteria(criteria) {
        Ok(c) => Some(c),
        Err(err) => {
            log::error!("Could not parse criteria query {}: {}", criteria, err);
            None
        }
    }
}

#[test]
fn test_criteria_parser() {
    match criteria_parser::criteria(
        "[tiling floating app_id=__focused__ app_id=\"foot\" class=\"emacs\" instance = \"the.instance\" title=\"something with :;&$\" con_mark=\"^.*foo$\"\tapp_name=\"Hugo\" con_id = __focused__ con_id=17 pid=23223]",
    ) {
        Ok(c) => println!("Criteria: {:?}", c),
        Err(err) => println!("Could not parse: {}", err),
    }
}

pub type Criteria = Vec<Criterion>;

fn is_some_and_rx_matches(s: Option<&String>, rx: &Regex) -> bool {
    s.is_some() && rx.is_match(s.unwrap())
}

fn are_some_and_equal<T: std::cmp::PartialEq>(
    a: Option<T>,
    b: Option<T>,
) -> bool {
    a.is_some() && b.is_some() && a.unwrap() == b.unwrap()
}

pub fn criteria_to_predicate<'a>(
    criteria: Criteria,
    all_windows: &'a [t::DisplayNode],
) -> impl Fn(&t::DisplayNode) -> bool + 'a {
    let focused = all_windows.iter().find(|x| x.node.focused);

    move |w: &t::DisplayNode| {
        for c in &criteria {
            let result = match c {
                Criterion::AppId(val) => match val {
                    RegexOrFocused::Regex(rx) => {
                        is_some_and_rx_matches(w.node.app_id.as_ref(), rx)
                    }
                    RegexOrFocused::Focused => match focused {
                        Some(win) => are_some_and_equal(
                            w.node.app_id.as_ref(),
                            win.node.app_id.as_ref(),
                        ),
                        None => false,
                    },
                },
                Criterion::AppName(val) => match val {
                    RegexOrFocused::Regex(rx) => {
                        rx.is_match(w.node.get_app_name())
                    }
                    RegexOrFocused::Focused => match focused {
                        Some(win) => {
                            w.node.get_app_name() != win.node.get_app_name()
                        }
                        None => false,
                    },
                },
                Criterion::Class(val) => match val {
                    RegexOrFocused::Regex(rx) => is_some_and_rx_matches(
                        w.node
                            .window_properties
                            .as_ref()
                            .and_then(|wp| wp.class.as_ref()),
                        rx,
                    ),
                    RegexOrFocused::Focused => match focused {
                        Some(win) => are_some_and_equal(
                            w.node
                                .window_properties
                                .as_ref()
                                .and_then(|p| p.class.as_ref()),
                            win.node
                                .window_properties
                                .as_ref()
                                .and_then(|p| p.class.as_ref()),
                        ),
                        None => false,
                    },
                },
                Criterion::Instance(val) => match val {
                    RegexOrFocused::Regex(rx) => is_some_and_rx_matches(
                        w.node
                            .window_properties
                            .as_ref()
                            .and_then(|wp| wp.instance.as_ref()),
                        rx,
                    ),
                    RegexOrFocused::Focused => match focused {
                        Some(win) => are_some_and_equal(
                            w.node
                                .window_properties
                                .as_ref()
                                .and_then(|p| p.instance.as_ref()),
                            win.node
                                .window_properties
                                .as_ref()
                                .and_then(|p| p.instance.as_ref()),
                        ),
                        None => false,
                    },
                },
                Criterion::ConId(val) => match val {
                    I64OrFocused::I64(id) => w.node.id == *id,
                    I64OrFocused::Focused => w.node.focused,
                },
                Criterion::ConMark(rx) => {
                    w.node.marks.iter().any(|m| rx.is_match(m))
                }
                Criterion::Pid(pid) => w.node.pid == Some(*pid),
                Criterion::Floating => w.node.is_floating(),
                Criterion::Tiling => !w.node.is_floating(),
                Criterion::Title(val) => match val {
                    RegexOrFocused::Regex(rx) => {
                        is_some_and_rx_matches(w.node.name.as_ref(), rx)
                    }
                    RegexOrFocused::Focused => match focused {
                        Some(win) => are_some_and_equal(
                            w.node.name.as_ref(),
                            win.node.name.as_ref(),
                        ),
                        None => false,
                    },
                },
            };
            if !result {
                return false;
            }
        }
        true
    }
}
