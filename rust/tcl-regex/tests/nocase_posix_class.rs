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

//! under `-nocase`, the POSIX classes `[[:upper:]]` /
//! `[[:lower:]]` must fold to *letters* (C Tcl's `CC_ALPHA`), not `alnum` —
//! so they never match digits.

use tcl_regex::Regex;
use tcl_regex::defs::{REG_ADVANCED, REG_ICASE};

fn cps(s: &str) -> Vec<u32> {
    s.chars().map(|c| c as u32).collect()
}

fn matches(pattern: &str, subject: &str, nocase: bool) -> bool {
    let flags = if nocase {
        REG_ADVANCED | REG_ICASE
    } else {
        REG_ADVANCED
    };
    let re = Regex::compile_str(pattern, flags).expect("pattern compiles");
    re.exec(&cps(subject), 0, 0).is_some()
}

#[test]
fn nocase_upper_class_does_not_match_digit() {
    // (TP) A digit must NOT match `[[:upper:]]` even under -nocase.
    assert!(!matches("[[:upper:]]", "5", true));
    assert!(!matches("[[:lower:]]", "5", true));
    assert!(!matches("^[[:lower:]]+$", "abc9", true));
    // (TN / FP-guard) Letters of either case still match under -nocase.
    assert!(matches("[[:upper:]]", "a", true));
    assert!(matches("[[:upper:]]", "A", true));
    assert!(matches("[[:lower:]]", "Z", true));
    assert!(matches("^[[:lower:]]+$", "abc", true));
    // Case-sensitive behaviour is unchanged.
    assert!(matches("[[:upper:]]", "A", false));
    assert!(!matches("[[:upper:]]", "a", false));
    assert!(!matches("[[:upper:]]", "5", false));
    // `[[:alnum:]]` still matches digits (the remap is class-specific).
    assert!(matches("[[:alnum:]]", "5", true));
}
