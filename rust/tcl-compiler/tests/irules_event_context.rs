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

//! W142 (`return_context_gate`) tracks `Analyser::current_event`: bare
//! `return` is mandatory *directly* inside a `when EVENT { … }` body, but the
//! full Tcl `-code`/`-level`/`-errorcode` form is unrestricted inside any
//! `proc` — see `return_.rs`'s FORMS doc comment. `current_event` must
//! therefore never leak from an enclosing `when` into a `proc` body, even
//! when that `proc` is (invalidly) written lexically inside the `when` block
//! itself — a placement `handle_proc_command`'s W142 check does not own and
//! must not duplicate/corrupt (that's `Irule5006`'s job).

use tcl_compiler::analyser::Analyser;

fn codes(source: &str, dialect: &str) -> Vec<String> {
    let mut a = Analyser::new();
    let result = a.analyse(source, dialect);
    result
        .diagnostics
        .iter()
        .map(|d| format!("{:?}", d.code))
        .collect()
}

#[test]
fn bare_return_with_args_directly_in_event_body_still_warns() {
    // Regression guard for the positive case: `return` with arguments
    // written *directly* inside a `when` body (no intervening `proc`) is
    // exactly what W142 exists to catch.
    let src = "when HTTP_REQUEST {\n    return \"x\"\n}\n";
    let got = codes(src, "f5-irules");
    assert!(
        got.iter().any(|c| c == "W142"),
        "a `return` with args directly in an event body must draw W142; got {got:?}"
    );
}

#[test]
fn bare_return_without_args_directly_in_event_body_never_warns() {
    // The mandatory bare form itself must never warn.
    let src = "when HTTP_REQUEST {\n    return\n}\n";
    let got = codes(src, "f5-irules");
    assert!(
        !got.iter().any(|c| c == "W142"),
        "a bare `return` (no args) directly in an event body must never draw W142; got {got:?}"
    );
}

#[test]
fn return_inside_a_top_level_proc_called_from_event_body_never_warns() {
    // The normal, structurally valid iRules idiom: `proc` defined at the top
    // level, invoked (via `call`) from inside a `when` body. The full
    // `-code`/`-errorinfo`/`-errorcode` form must be unrestricted here.
    let src = "proc helper {} {\n    return -code error \"boom\"\n}\nwhen HTTP_REQUEST {\n    call helper\n}\n";
    let got = codes(src, "f5-irules");
    assert!(
        !got.iter().any(|c| c == "W142"),
        "`return` inside a normally-placed proc must never draw W142; got {got:?}"
    );
}

#[test]
fn return_inside_a_proc_nested_in_an_event_body_never_warns() {
    // The bug: a `proc` written lexically *inside* a `when` body is itself
    // an invalid placement (Irule5006 — "'proc' is only valid at the top
    // level of an iRule") — but `current_event` must still not leak into
    // its body. A `return` inside that nested proc's own body is exactly
    // the form every proc uses (`return_.rs`'s FORMS doc comment) and must
    // never *also* draw the unrelated, misleading W142 event-body message
    // ("wrap the call in a proc" when it already is one).
    let src =
        "when HTTP_REQUEST {\n    proc helper {} {\n        return \"x\"\n    }\n    helper\n}\n";
    let got = codes(src, "f5-irules");
    assert!(
        got.iter().any(|c| c == "Irule5006"),
        "a proc nested in a when body must still draw Irule5006 (placement); got {got:?}"
    );
    assert!(
        !got.iter().any(|c| c == "W142"),
        "`return` inside a proc's own body must never draw W142, however that proc is placed; got {got:?}"
    );
}
