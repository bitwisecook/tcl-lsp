// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `\z` is Tcl 9.1.0's synonym for the `\Z` end-of-string anchor
//! (`regc_lex.c`); earlier releases reject it. The engine gates it on
//! `REG_ZANCHOR`. Oracle: tclsh 9.1.0 and 9.0.4.

use tcl_regex::defs::{REG_ADVANCED, REG_EXTENDED, REG_NEWLINE, REG_ZANCHOR};
use tcl_regex::{ExecOutcome, Regex};

fn cps(s: &str) -> Vec<u32> {
    s.chars().map(|c| c as u32).collect()
}

fn matches(pattern: &str, subject: &str, cflags: i32) -> bool {
    let re = Regex::compile_str(pattern, cflags).expect("pattern compiles");
    match re.exec(&cps(subject), 0, 0) {
        ExecOutcome::Matched(_) => true,
        ExecOutcome::NoMatch => false,
        ExecOutcome::Stopped(stop) => panic!("`{pattern}` against {subject:?} stopped: {stop:?}"),
    }
}

#[test]
fn z_is_an_end_of_string_anchor_when_enabled() {
    let are = REG_ADVANCED | REG_ZANCHOR;
    // (TP) `regexp {a\z} xa` → 1 on 9.1.0.
    assert!(matches(r"a\z", "xa", are));
    // (TN) End of *string*, not end of line: `a\z` misses "a\nb" even with
    // `-line`, exactly as `\Z` does.
    assert!(!matches(r"a\z", "a\nb", are));
    assert!(!matches(r"a\z", "a\nb", are | REG_NEWLINE));
    assert!(!matches(r"a\Z", "a\nb", are | REG_NEWLINE));
    // Embedded options keep the escape available.
    assert!(matches(r"(?x) a \z", "xa", are));
}

#[test]
fn z_is_an_invalid_escape_without_the_flag() {
    // (TN) tclsh 8.4.20–9.0.4: `invalid escape \ sequence`.
    let Err(err) = Regex::compile_str(r"a\z", REG_ADVANCED) else {
        panic!("`a\\z` must not compile without REG_ZANCHOR");
    };
    assert_eq!(err.message(), r"invalid escape \ sequence");
    // `\Z` is unaffected by the flag's absence.
    assert!(matches(r"a\Z", "xa", REG_ADVANCED));
}

#[test]
fn z_stays_invalid_inside_a_bracket_expression() {
    // (TN) tclsh 9.1.0 still rejects `[\z]`, like `[\Z]`.
    assert!(Regex::compile_str(r"[\z]", REG_ADVANCED | REG_ZANCHOR).is_err());
}

#[test]
fn z_is_a_literal_outside_are_mode() {
    // (TN) `(?e)a\z` matches "az" on both 9.0.4 and 9.1.0: ERE never reaches
    // the ARE escape lexer, so the flag changes nothing there.
    for cflags in [REG_EXTENDED, REG_EXTENDED | REG_ZANCHOR] {
        assert!(matches(r"a\z", "az", cflags));
        assert!(!matches(r"a\z", "a", cflags));
    }
}
