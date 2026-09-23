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

//! A runtime `expr` substitutes a `"…"` operand and leaves a `{…}` one as
//! written. tclsh 8.4.20 through 9.1b0 all return `[id 9]`, `0` and `pre5`
//! for the rows below; the evaluator substituted both spellings (#2227).

#[cfg(have_tommath)]
use tcl_runtime::interp::{Code, Interp};

#[cfg(have_tommath)]
fn eval(script: &str) -> (Code, String) {
    let mut interp = Interp::new();
    let code = interp.eval_str(script.as_bytes());
    let result = String::from_utf8_lossy(&interp.result_bytes()).into_owned();
    (code, result)
}

#[cfg(have_tommath)]
#[test]
fn a_braced_expr_operand_is_literal_and_a_quoted_one_substitutes() {
    for (script, expected) in [
        (
            "proc id {v} { return $v }; set e {{[id 9]}}; expr $e",
            "[id 9]",
        ),
        ("set x 5; set e {{pre$x} eq \"pre$x\"}; expr $e", "0"),
        ("set x 5; set e {\"pre$x\"}; expr $e", "pre5"),
    ] {
        assert_eq!(eval(script), (Code::Ok, expected.to_owned()), "{script}");
    }
}
