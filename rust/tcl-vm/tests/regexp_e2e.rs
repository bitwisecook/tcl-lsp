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

//! `regexp` / `regsub` command-level behaviour, covering Tcl 9's
//! `regexp.test` / `regexpComp.test` cases, run end-to-end through the VM (which
//! drives the pure-Rust `tcl-regex` ARE engine). Each expected value was
//! captured from real `tclsh` 9.0.3, so this exercises the full command stack
//! — option parsing, the match/advance loop, `-all`/`-inline`/`-indices`/
//! `-start`/`-line`/`-nocase`, submatch-variable assignment, and the `regsub`
//! substitution-spec expansion — over the faithful engine.

// The case table keeps uniform `r#"…"#` delimiters so patterns with and
// without embedded quotes line up; a few don't strictly need the hashes.
#![allow(clippy::needless_raw_string_hashes)]

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::Vm;

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

/// Compile + run `src`; return `(ok, result)`.
fn run(src: &str) -> (bool, String) {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);
    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    let c = vm.run_module(&asm);
    (c.code.is_ok(), c.result.to_str().to_string())
}

/// `(Tcl source, expected result)` — every expected value verified against
/// `tclsh` 9.0.3.
const CASES: &[(&str, &str)] = &[
    (r#"regexp ab*c aaabbbccc"#, "1"),
    (r#"regexp -inline {a(b*)c} xabbbcx"#, "abbbc bbb"),
    (r#"regexp -indices -inline {a(b*)c} xabbbcx"#, "{1 5} {2 4}"),
    (r#"regexp -all -inline {[0-9]+} "a12b345c""#, "12 345"),
    (
        r#"list [regexp {(a)(b)?(c)} ac m a b c] $m $a $b $c"#,
        "1 ac a {} c",
    ),
    (r#"regexp -nocase {abc} XABCY"#, "1"),
    (r#"regexp -- -foo -foo"#, "1"),
    (r#"set s "a\nb"; regexp -line {^b} $s"#, "1"),
    (r#"regexp {^b} "a\nb""#, "0"),
    (r#"regexp -all -inline {a.} "axaybz""#, "ax ay"),
    (r#"regexp -start 3 {a} "aaaaa""#, "1"),
    (r#"regsub {b+} aabbbcc X"#, "aaXcc"),
    (r#"regsub -all {b} aabbbcc X"#, "aaXXXcc"),
    (r#"regsub {(a)(b)} ab {\2\1}"#, "ba"),
    (r#"regsub -all {[abc]} "abcd" {[&]}"#, "[a][b][c]d"),
    (r#"regsub -nocase {ABC} xabcy Q"#, "xQy"),
    // Word edges and classes — not expressible in the old approximate engine.
    (r#"regexp {\mfoo\M} "a foo b""#, "1"),
    (r#"regexp {\w+} "  hi_there! ""#, "1"),
    // POSIX longest-match submatch semantics (differs from Perl leftmost-first).
    (r#"regexp -inline {(a*)(a*)} aaa"#, "aaa aaa {}"),
    (r#"regexp {(a|ab)(c|bcd)(d*)} abcd"#, "1"),
    (r#"regexp -inline {(a|ab)(c|bcd)(d*)} abcd"#, "abcd ab c d"),
    (r#"regexp {[[:digit:]]+} "ab123""#, "1"),
    (r#"regsub -all {(\w)(\w)} abcd {\2\1}"#, "badc"),
    (r#"regexp -inline {(\w+)@(\w+)} foo@bar"#, "foo@bar foo bar"),
    (r#"regsub {^} abc >"#, ">abc"),
    (r#"regexp -all -inline -indices {a} aXa"#, "{0 0} {2 2}"),
    (r#"regexp (?n)^b "a\nb""#, "1"),
    (
        r#"catch {regexp (*) x} e; set e"#,
        "cannot compile regular expression pattern: invalid quantifier operand",
    ),
];

#[test]
fn regexp_command_corpus() {
    let mut failures = Vec::new();
    for (src, expected) in CASES {
        let (ok, result) = run(src);
        if !ok || &result != expected {
            failures.push(format!(
                "  src=`{src}`\n    got ok={ok} result={result:?}, expected {expected:?}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} regexp/regsub command cases failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
