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

//! Backslash-only `subst` against Tcl 9's `TclParseBackslash`.

use crate::prelude::*;

/// `subst` with only backslash substitution enabled.
fn subst_backslashes(template: &str) -> String {
    let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("tcl9.0"));
    let completion = crate::fixture::call(
        &mut vm,
        "subst",
        &[
            Value::string("-nocommands"),
            Value::string("-novariables"),
            Value::string(template),
        ],
    );
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    completion.result.to_str().to_string()
}

#[test]
fn subst_hex_escape_consumes_exactly_two_digits() {
    // Tcl 9 caps `\x` at two hex digits (`TclParseBackslash`): `\x41BC`
    // is `A` + literal `BC`, never a wider code point or the 8.x
    // last-two-digits reading.
    assert_eq!(subst_backslashes(r"\x41BC"), "ABC");
    assert_eq!(subst_backslashes(r"\x4142"), "A42");
}

#[test]
fn subst_backslash_continuation_accepts_only_raw_lf() {
    // TclParseBackslash recognises raw LF. A channel may translate CRLF
    // before this parser seam, but raw CR and CRLF remain data.
    assert_eq!(subst_backslashes("a\\\n   b"), "a b");
    assert_eq!(subst_backslashes("a\\\r\n\t b"), "a\r\n\t b");
    assert_eq!(subst_backslashes("a\\\rb"), "a\rb");
    // FP guard: `\\` is an escaped backslash, so the newline after it is
    // real content, not a continuation.
    assert_eq!(subst_backslashes("x\\\\\r\ny"), "x\\\r\ny");
}
