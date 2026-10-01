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

//! `switch -integer` (TIP 730) is a Tcl 9.1 match mode: the value and every
//! reached pattern are wide integers compared by value, and the option does
//! not exist on earlier releases. Vectors are tclsh 9.1.0 / 9.0.4 output.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;
use tcl_vm::{CompileService, Vm};

#[derive(Clone)]
struct Capture(Rc<RefCell<Vec<u8>>>);
impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn run_for_version(src: &str, version: TclVersion) -> (bool, String) {
    let profile = tcl_registry::model::ingress::resolve_environment(version.dialect_name())
        .analyser_profile();
    let service = BytecodeCompileService::for_profile(profile);
    let asm = service
        .compile_for_profile(src, profile)
        .expect("test script compiles for its selected profile");
    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_runtime_version(version);
    vm.set_compiler(Box::new(service));
    let c = vm.run_module(&asm);
    (c.code.is_ok(), c.result.to_str().to_string())
}

/// `[catch]` the script, yielding `code result errorcode`.
fn caught(script: &str, version: TclVersion) -> String {
    let src = format!(
        "list [catch {{{script}}} r o] $r [expr {{[dict exists $o -errorcode] ? [dict get $o -errorcode] : {{}}}}]"
    );
    let (ok, out) = run_for_version(&src, version);
    assert!(ok, "{script}: {out}");
    out
}

#[test]
fn integer_mode_matches_by_value_on_tcl91() {
    let cases: &[(&str, &str)] = &[
        // Tcl 9 reads `010` as decimal.
        ("switch -integer 010 {8 {set _ a} 10 {set _ b}}", "b"),
        ("switch -int 0x10 {16 {set _ hex}}", "hex"),
        ("switch -integer { 16 } {0x10 {set _ ws}}", "ws"),
        ("switch -integer 1_000 {1000 {set _ us}}", "us"),
        ("switch -integer 2 {1 {set _ a} default {set _ d}}", "d"),
        ("switch -integer 3 {1 {set _ a} 2 {set _ b}}", ""),
        ("switch -integer 1 {1 {set _ a} abc {set _ b}}", "a"),
        ("switch -integer 1 1 {set _ inline} 2 {set _ two}", "inline"),
        ("switch -integer 2 {1 - 2 {set _ ft}}", "ft"),
        ("set n 7; switch -integer -- $n {7 {set _ var}}", "var"),
    ];
    for &(src, want) in cases {
        let (ok, got) = run_for_version(src, TclVersion::V9_1);
        assert!(ok, "{src}: {got}");
        assert_eq!(got, want, "{src}");
    }
}

#[test]
fn integer_mode_errors_match_tcl91() {
    let cases: &[(&str, &str)] = &[
        (
            "switch -integer abc {1 {set _ a}}",
            r#"1 {expected integer but got "abc"} {TCL VALUE NUMBER}"#,
        ),
        (
            "switch -integer 2 {1 {set _ a} abc {set _ b} default {set _ d}}",
            r#"1 {expected integer but got "abc"} {TCL VALUE NUMBER}"#,
        ),
        (
            "switch -integer 2 {default {set _ d} 2 {set _ two}}",
            r#"1 {expected integer but got "default"} {TCL VALUE NUMBER}"#,
        ),
        (
            "switch -integer 99999999999999999999 {1 {set _ a}}",
            "1 {integer value too large to represent} \
             {ARITH IOVERFLOW {integer value too large to represent}}",
        ),
        (
            "switch -integer -nocase 1 {1 {set _ a}}",
            "1 {-nocase option cannot be used with -integer option} \
             {TCL OPERATION SWITCH MODERESTRICTION}",
        ),
        (
            "switch -glob -integer 1 {1 {set _ a}}",
            r#"1 {bad option "-integer": -glob option already found} {TCL OPERATION SWITCH DOUBLEOPT}"#,
        ),
        (
            "switch -in 1 {1 {set _ a}}",
            r#"1 {ambiguous option "-in": must be -exact, -glob, -indexvar, -integer, -matchvar, -nocase, -regexp, or --} {TCL LOOKUP INDEX option -in}"#,
        ),
    ];
    for &(src, want) in cases {
        assert_eq!(caught(src, TclVersion::V9_1), want, "{src}");
    }
}

#[test]
fn integer_option_is_refused_before_tcl91() {
    let want = r#"1 {bad option "-integer": must be -exact, -glob, -indexvar, -matchvar, -nocase, -regexp, or --} {TCL LOOKUP INDEX option -integer}"#;
    for version in [TclVersion::V8_5, TclVersion::V8_6, TclVersion::V9_0] {
        assert_eq!(
            caught("switch -integer 1 {1 {set _ a}}", version),
            want,
            "{version:?}"
        );
    }
}
