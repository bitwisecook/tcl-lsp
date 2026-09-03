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

//! Oracle-pinned coverage for the VM's adoption of the shared word-component
//! owner (`tcl_lexer::word_parts`, bucket R10).
//!
//! Every expectation below was taken verbatim from `tclsh9.0` (9.0.4) and
//! `tclsh8.6` (8.6.16), which agree on all of them:
//!
//! ```text
//! % set x(k) v
//! % subst {[list {a]b}]}   => a\]b
//! % subst {[list "a]b"]}   => a\]b
//! % subst "\[list a\n# ]\nb]"
//!                          => invalid command name "b"
//! % subst {$x(}            => missing )
//! % subst {x[b}            => missing close-bracket
//! % subst {$x({k})}        => 9.0: invalid character in array index
//! %                           8.6: can't read "x({k})": no such element…
//! ```

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

struct CompilerSvc {
    registry: CommandRegistry,
}

impl CompileService for CompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        if let Some(msg) = tcl_compiler::lowering::first_fatal_parse_error(src) {
            return Err(CompileError(msg));
        }
        let ir = lower_to_ir(src, &self.registry);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.registry))
    }
}

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

/// Compile and run `src`; return `(ok, result-string)`.
fn run(src: &str) -> (bool, String) {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);

    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_compiler(Box::new(CompilerSvc {
        registry: CommandRegistry::build_default(),
    }));
    let completion = vm.run_module(&asm);
    (
        completion.code.is_ok(),
        completion.result.to_str().to_string(),
    )
}

/// The `]` that closes a command substitution is found by the shared owner
/// (`tcl_lexer::command_substitution_end`), which knows the substituted text
/// is a *script*: a `]` inside a braced word, inside a quoted word, or after a
/// `#` at command position is inert.
///
/// The VM's private `command_end` handled braces only, so the quoted case
/// stopped at the wrong bracket and the comment case ran a truncated script.
#[test]
fn the_bracket_close_respects_braces_quotes_and_comments() {
    assert_eq!(run(r"subst {[list {a]b}]}"), (true, r"a\]b".to_owned()));
    assert_eq!(run(r#"subst {[list "a]b"]}"#), (true, r"a\]b".to_owned()));
    // `# ]` is a comment line of the substituted script, so the `]` in it does
    // not close the substitution; the script runs on to the undefined `b`.
    assert_eq!(
        run("subst \"\\[list a\n# ]\nb]\""),
        (false, "invalid command name \"b\"".to_owned())
    );
}

/// Every unterminated `$`-reference spelling reports C's message, through the
/// one owner rather than each engine's recovery.
#[test]
fn unterminated_references_report_c_tcls_messages() {
    assert_eq!(run("subst {$x(}"), (false, "missing )".to_owned()));
    assert_eq!(
        run("subst {x[b}"),
        (false, "missing close-bracket".to_owned())
    );
    assert_eq!(
        run("subst {${a{b}"),
        (false, "missing close-brace for variable name".to_owned())
    );
}

/// A well-formed template is unchanged by the adoption: nested brackets,
/// nested array indices, and the release-aware `${…}` close rule all still
/// resolve.
#[test]
fn well_formed_templates_are_unchanged() {
    assert_eq!(run("set a hi\nsubst {x${a}y}"), (true, "xhiy".to_owned()));
    assert_eq!(
        run("set c(1) inner\nset b(inner) mid\nset a(mid) outer\nsubst {$a($b($c(1)))}"),
        (true, "outer".to_owned())
    );
    assert_eq!(
        run("subst {[list a [list b] c]}"),
        (true, "a b c".to_owned())
    );
}

/// Issue #1646 is **not** in the word decomposer, and this records the
/// measurement rather than asserting the bug is gone.
///
/// The divergence is in the compiler's literal emission for a word nested in
/// a bracket word, not in how the VM splits a word into parts, and the shape
/// of the evidence is what says so:
///
/// | vector | oracle | this VM |
/// |---|---|---|
/// | `string length "x\$y"` (top level) | 3 | 3 |
/// | `set n [string length "x\$y"]` | 3 | **4** |
/// | `set n [list "x\$y"]` | `{x$y}` | **`{x\$y}`** |
///
/// The same word reads correctly at the top level and wrongly one bracket
/// deep, so the decomposition is not what differs — what the compiler pushes
/// is. `tcl explore --show asm` confirms it: the nested form emits the literal
/// `x\$y` where the word's value is `x$y`.
///
/// A blanket decode in the VM would "fix" the nested vectors by breaking the
/// `set body` one below, whose literal legitimately *does* contain
/// backslashes. The VM's rule — an emitted `PUSH` literal is already its value
/// — is the correct one; the emission is not. Fixing it means changing
/// `rust/tcl-compiler`, which the native-lowering lane owns.
#[test]
fn issue_1646_is_a_compiler_literal_emission_gap_not_a_decomposition_one() {
    // The VM's rule, which is right: an emitted literal is already the value.
    // `set body "list e\\n} f\\$} "` is 15 characters on both oracles.
    assert_eq!(
        run("set body \"list e\\\\n} f\\\\$} \"\nstring length $body"),
        (true, "15".to_owned())
    );
    // The same word, decomposed correctly at the top level.
    assert_eq!(run(r#"string length "x\$y""#), (true, "3".to_owned()));
    // …and the compiler's gap, one bracket deep. If either of these now reads
    // the oracle value, the compiler-side fix for #1646 landed: delete this
    // test and pin the oracle values instead.
    let (ok, result) = run(r#"set n [string length "x\$y"]"#);
    assert!(ok);
    assert_eq!(result, "4", "oracle says 3 — see this test's doc comment");
    let (ok, result) = run(r#"set n [list "x\$y"]"#);
    assert!(ok);
    assert_eq!(
        result, r"{x\$y}",
        "oracle says {{x$y}} — see this test's doc comment"
    );
}
