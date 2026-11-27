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

//! Standalone-runtime coverage for the shared Tcl 9 command-head matrix.

use std::path::{Path, PathBuf};

use tcl_dialect::TclVersion;
use tcl_runtime::interp::{Code, Interp};
use tcl_test_support::{locate_source_tree, run_script_from_source_tree};

const EXPECTED: &str = "statement 2 \
head-ordinary-expanded {head ordinary tail1 tail2} empty {} multi {a b} \
substitutions seed value {value position} malformed 1 \
{unmatched open brace in list} {TCL VALUE LIST BRACE}";

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn standalone_and_tcl_9_0_4_share_command_head_expansion_semantics() {
    let source = std::fs::read_to_string(
        repository_root().join("tests/fixtures/tcl9/expanded-command-head.tcl"),
    )
    .expect("read expanded-command-head fixture");

    let mut interp = Interp::new();
    interp.set_runtime_version(TclVersion::V9_0);
    let code = interp.eval_str(source.as_bytes());
    let result = String::from_utf8(interp.result_bytes()).expect("runtime result is UTF-8");
    assert_eq!(code, Code::Ok, "standalone fixture failed: {result}");
    assert_eq!(result, EXPECTED);

    let Some(tree) = locate_source_tree(&repository_root(), TclVersion::V9_0, None)
        .expect("locate Tcl 9.0.4 source tree")
    else {
        eprintln!("skipping oracle: Tcl 9.0.4 source tree is not installed");
        return;
    };
    assert_eq!(tree.patchlevel, "9.0.4", "exact Tcl oracle pin");
    if !tree.root.join("unix/tclsh").is_file() {
        eprintln!("skipping oracle: Tcl 9.0.4 source tree has no built unix/tclsh");
        return;
    }
    let oracle_source = format!("{source}\nputs -nonewline [set ::out]\n");
    let oracle = run_script_from_source_tree(&tree, TclVersion::V9_0, oracle_source.as_bytes())
        .expect("run Tcl 9.0.4 oracle")
        .strict_text()
        .expect("Tcl 9.0.4 oracle succeeds");
    assert_eq!(result, oracle);
}

/// `TclParseCommand` flattens a pure-TEXT `{*}` literal into simple words
/// before the `list` and `dict create` compilers run, inside procedures and
/// in a compiled branch that never executes. Expected: tclsh 8.6.18, 9.0.4
/// and 9.1.0.
#[test]
fn a_flattened_literal_expansion_compiles_as_its_spelled_out_words() {
    const SCRIPT: &str = r#"set out {}
proc p1 {q} { list p {*}{a {b c}} $q {*}"d e" }
proc p2 {} { dict create {*}{a 1 b 2} }
proc p3 {q} { dict create k $q {*}{a 1} }
proc p4 {} { list {*}{} }
proc p5 {} { list {*}{a \x41} }
proc p6 {} { catch {dict create {*}{a b c}} m; set m }
set v 0
if {$v} { set never [dict create {*}{a b c}] }
lappend out [p1 x] [p2] [p3 x] [p4] [p5] [p6]
"#;
    const FLATTENED: &str = "{p a {b c} x d e} {a 1 b 2} {k x a 1} {} {a A} \
{wrong # args: should be \"dict create ?key value ...?\"}";
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        let mut interp = Interp::new();
        interp.set_runtime_version(version);
        let code = interp.eval_str(SCRIPT.as_bytes());
        let result = String::from_utf8(interp.result_bytes()).expect("runtime result is UTF-8");
        assert_eq!(
            (code, result.as_str()),
            (Code::Ok, FLATTENED),
            "{version:?}"
        );
    }
}
