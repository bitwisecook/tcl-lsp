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

//! A document's top level that one command's compiler refuses is still
//! compiled whole (plan item 423).
//!
//! Tcl 8.4 hands a sourced file to `Tcl_EvalEx`, so the commands before a
//! malformed `expr {0o17}` run and the error is that command's. A compiled
//! unit must do the same: every command compiled, the refused one failing at
//! run time, never an empty unit. Each case compiles as `tcl dis` does and
//! runs the module; when the release's tclsh is on `PATH` its file-mode
//! output is the reference.

use std::cell::RefCell;
use std::rc::Rc;

use tcl_codegen::codegen::codegen_module;
use tcl_codegen::compile_service::BytecodeCompileService;
use tcl_compiler::cfg_builder::build_cfg_codegen_with_registry;
use tcl_compiler::lowering::lower_to_ir_for_bytecode_with_dialect;
use tcl_dialect::TclVersion;
use tcl_vm::Code;

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl std::io::Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// What running a script shows: its `puts` output and, when it fails, the
/// `errorInfo` up to the frame naming the file.
#[derive(Debug, PartialEq, Eq)]
struct Run {
    stdout: String,
    error: Option<String>,
}

/// Compile `src` as `tcl dis` does under `version`, check every command made
/// it into the top level, and run it.
fn compiled_run(src: &str, version: TclVersion) -> Run {
    let environment =
        tcl_registry::model::ingress::resolve_environment(version.dialect_profile_name());
    let profile = environment.analyser_profile();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let ir = lower_to_ir_for_bytecode_with_dialect(
        src,
        registry,
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        Some(profile),
    );
    let cfg = build_cfg_codegen_with_registry(&ir, false, registry);
    let asm = codegen_module(&cfg, &ir, registry).expect("original source namespace");
    assert!(
        asm.top_level.native_compilation_failure.is_none(),
        "{version:?}: {src:?} compiled to a refusal of the whole unit"
    );
    let listing = tcl_bytecode::format::format_module_asm(&asm);
    for literal in ["\"a\"", "\"b\""] {
        assert!(
            listing.contains(literal),
            "{version:?}: {src:?} lost a command:\n{listing}"
        );
    }

    let out = Capture::default();
    let mut vm = tcl_vm::test_support::native_vm_for_release(version, Box::new(out.clone()));
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    let completion = vm.run_module(&asm);
    let stdout = String::from_utf8_lossy(&out.0.borrow()).into_owned();
    Run {
        stdout,
        error: (completion.code == Code::Error).then(|| {
            vm.get_var("::errorInfo")
                .map_or_else(String::new, |info| info.to_str().to_string())
        }),
    }
}

/// `src` run as a file by the release's tclsh, or `None` without one.
fn tclsh_run(src: &str, version: TclVersion) -> Option<Run> {
    let tclsh = tcl_test_support::witness_tclsh(version)?;
    let outcome =
        tcl_test_support::run_script_fixture(&tclsh.path, src, &[]).expect("the oracle runs");
    let stderr = String::from_utf8_lossy(&outcome.stderr).into_owned();
    Some(Run {
        stdout: String::from_utf8_lossy(&outcome.stdout).into_owned(),
        error: (!outcome.success()).then(|| {
            stderr
                .split_once("\n    (file ")
                .map_or(stderr.as_str(), |(info, _)| info)
                .to_owned()
        }),
    })
}

/// Run `src` compiled for `version`: its output must be `stdout`, and its
/// `errorInfo` must open with `error` when one is expected; the release's
/// tclsh, where there is one, must agree exactly.
fn check(src: &str, version: TclVersion, stdout: &str, error: Option<&str>) {
    let compiled = compiled_run(src, version);
    assert_eq!(compiled.stdout, stdout, "{version:?}: {src:?}");
    match (error, &compiled.error) {
        (None, None) => {}
        (Some(expected), Some(info)) => assert!(
            info.starts_with(expected),
            "{version:?}: {src:?} failed elsewhere:\n{info}"
        ),
        _ => panic!("{version:?}: {src:?} gave {compiled:?}, expected {error:?}"),
    }
    if let Some(reference) = tclsh_run(src, version) {
        assert_eq!(compiled, reference, "{version:?}: {src:?} against tclsh");
    }
}

const NOT_OCTAL: &str = "syntax error in expression \"0o17\": extra tokens at end of expression";

#[test]
fn octal_prefix_fails_at_its_command_under_8_4() {
    let src = "puts a; expr {0o17}; puts b";
    let error = format!("{NOT_OCTAL}\n    while executing\n\"expr {{0o17}}\"");
    check(src, TclVersion::V8_4, "a\n", Some(&error));
    check(src, TclVersion::V8_6, "a\nb\n", None);
    check(src, TclVersion::V9_0, "a\nb\n", None);
}

#[test]
fn mixed_octal_spellings_fail_at_their_command_under_8_4() {
    let src = "puts a; puts [expr {017 + 0o17}]; puts b";
    let error = "syntax error in expression \"017 + 0o17\": extra tokens at end of expression";
    check(src, TclVersion::V8_4, "a\n", Some(error));
    check(src, TclVersion::V8_6, "a\n30\nb\n", None);
    check(src, TclVersion::V9_0, "a\n32\nb\n", None);
}

#[test]
fn malformed_expression_fails_at_its_command_under_8_4() {
    check(
        "puts a; expr {1 +}; puts b",
        TclVersion::V8_4,
        "a\n",
        Some(
            "syntax error in expression \"1 +\": premature end of expression\n    while executing\n",
        ),
    );
}

/// A script a command runs is compiled when it runs, so its refusal is that
/// command's error, raised `while compiling`, and a `catch` sees it.
#[test]
fn refused_bodies_fail_when_their_command_runs_under_8_4() {
    let compiling = format!("{NOT_OCTAL}\n    while compiling\n\"expr {{0o17}}\"");
    check(
        "puts a; if 1 {expr {0o17}}; puts b",
        TclVersion::V8_4,
        "a\n",
        Some(&compiling),
    );
    check(
        "proc p {} {expr {0o17}}; puts a; p; puts b",
        TclVersion::V8_4,
        "a\n",
        Some(&compiling),
    );
    check(
        "puts a; catch {expr {0o17}} m; puts $m; puts b",
        TclVersion::V8_4,
        &format!("a\n{NOT_OCTAL}\nb\n"),
        None,
    );
}
