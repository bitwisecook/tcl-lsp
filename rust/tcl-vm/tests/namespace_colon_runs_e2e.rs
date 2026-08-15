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

//! Colon-run (`:::`) namespace-separator regression tests.
//!
//! C Tcl treats a run of **two or more** colons as a single `::` separator
//! (`TclGetNamespaceForQualName`), so `foo:::bar` names `foo::bar`, a trailing
//! run in a *command* name denotes the `{}`-named command (`proc quux::: …`
//! defines `::quux::`), and a trailing run in a *namespace* name is dropped
//! (`namespace eval c::: {}` creates `::c`). The VM's old hand-rolled
//! `rsplit("::")`/`strip_prefix("::")` sites diverged on every one of these
//! (documented drift — `docs/design/family-b-routing.md`); resolution now
//! routes through the canonical splits (`tcl_syntax::naming`,
//! `tcl_cmd_core::namespace`). Every expectation below is pinned against
//! tclsh8.6 (`// tclsh8.6:` comments).

// ---------------------------------------------------------------------------
// Harness — copied verbatim from `run_script.rs` (the `run` helper plus every
// struct/import it needs: `CompilerSvc`, `Capture`, and the `use` lines).
// ---------------------------------------------------------------------------

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

/// A `tcl-compiler`-backed compile service so the VM can resolve runtime
/// `eval` / `[command substitution]` (the injection seam — `tcl-vm` itself
/// never depends on the compiler).
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

/// A `Write` sink backed by a shared buffer the test can read afterwards.
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

/// Compile and run `src`; return `(ok, result-string, captured-stdout)`.
fn run(src: &str) -> (bool, String, String) {
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

    let out = String::from_utf8(buf.borrow().clone()).expect("utf-8 output");
    (
        completion.code.is_ok(),
        completion.result.to_str().to_string(),
        out,
    )
}

// ===========================================================================
// Command resolution through colon runs
// ===========================================================================

/// A colon-run call collapses to the `::` form: `foo:::bar` dispatches
/// `::foo::bar`, and a leading run is the global root.
#[test]
fn colon_run_call_resolves_like_double_colon() {
    // tclsh8.6: `namespace eval foo {proc bar {} {return hi}}; foo:::bar` -> hi
    let (ok, res, _) = run("namespace eval foo {proc bar {} {return hi}}\nfoo:::bar");
    assert!(ok, "got: {res}");
    assert_eq!(res, "hi");
    // tclsh8.6: `::::g` calls the global `g` (leading run = the root).
    let (ok, res, _) = run("proc g {} {return G}\n::::g");
    assert!(ok, "got: {res}");
    assert_eq!(res, "G");
}

/// `proc` with an interior colon run defines the `::`-collapsed name (the old
/// `rsplit_once` derived the bogus namespace `foo:` and errored).
#[test]
fn proc_with_colon_run_defines_collapsed_name() {
    // tclsh8.6: defines ::foo::baz; `foo::baz` -> baz; info commands lists both.
    let (ok, res, _) = run("namespace eval foo {proc bar {} {}}\n\
         proc foo:::baz {} {return baz}\n\
         list [foo::baz] [lsort [info commands ::foo::*]]");
    assert!(ok, "got: {res}");
    assert_eq!(res, "baz {::foo::bar ::foo::baz}");
}

/// A trailing colon run in a *command* name denotes the `{}`-named command in
/// the qualified namespace; both the `:::` and `::` spellings reach it.
#[test]
fn proc_with_trailing_run_defines_empty_named_command() {
    // tclsh8.6: `proc quux::: {} {…}` defines ::quux:: ; `quux:::` and
    // `quux::` both call it; info commands lists `::quux::`.
    let (ok, res, _) = run("namespace eval quux {}\n\
         proc quux::: {} {return trailing}\n\
         list [quux:::] [quux::] [info commands ::quux::*]");
    assert!(ok, "got: {res}");
    assert_eq!(res, "trailing trailing ::quux::");
}

/// A colon-run proc name whose namespace does not exist errors exactly as C.
#[test]
fn proc_colon_run_into_missing_namespace_errors() {
    // tclsh8.6: can't create procedure "nosuch:::p": unknown namespace
    let (ok, msg, _) = run("proc nosuch:::p {} {}");
    assert!(!ok);
    assert_eq!(
        msg,
        "can't create procedure \"nosuch:::p\": unknown namespace"
    );
}

/// `rename` onto a colon-run qualified target lands on (and re-homes to) the
/// collapsed namespace — the raw target used to register an `a:::q` key.
#[test]
fn rename_through_colon_run_re_homes_the_proc() {
    // tclsh8.6: `rename p6 dst6:::q` -> ::dst6::q, body runs in ::dst6.
    let (ok, res, _) = run("namespace eval dst6 {}\n\
         proc p6 {} {namespace current}\n\
         rename p6 dst6:::q\n\
         list [dst6::q] [info commands ::dst6::*]");
    assert!(ok, "got: {res}");
    assert_eq!(res, "::dst6 ::dst6::q");
}

// ===========================================================================
// Namespace-name resolution through colon runs
// ===========================================================================

/// `namespace exists` / `namespace parent` collapse colon runs in their
/// argument (the old literal lookup missed `a:::b`).
#[test]
fn namespace_exists_and_parent_collapse_runs() {
    // tclsh8.6: exists a:::b -> 1 ; parent a:::b -> ::a
    let (ok, res, _) = run("namespace eval a::b {}\n\
         list [namespace exists a:::b] [namespace parent a:::b]");
    assert!(ok, "got: {res}");
    assert_eq!(res, "1 ::a");
}

/// A trailing colon run in a *namespace* name is dropped: `namespace eval
/// c::: {}` creates (and reports) `::c`, not a namespace literally named
/// `c::`/`c:::`.
#[test]
fn namespace_eval_trailing_run_creates_plain_namespace() {
    // tclsh8.6: exists ::c9 -> 1 ; exists c9::: -> 1
    let (ok, res, _) = run("namespace eval c9::: {}\n\
         list [namespace exists ::c9] [namespace exists c9:::]");
    assert!(ok, "got: {res}");
    assert_eq!(res, "1 1");
    // tclsh8.6: `namespace eval c9b::: {namespace current}` -> ::c9b
    let (ok, res, _) = run("namespace eval c9b::: {namespace current}");
    assert!(ok, "got: {res}");
    assert_eq!(res, "::c9b");
}

/// `namespace delete` collapses colon runs in its argument.
#[test]
fn namespace_delete_collapses_runs() {
    // tclsh8.6: `namespace delete a:::b` removes ::a::b, keeps ::a.
    let (ok, res, _) = run("namespace eval a::b {}\n\
         namespace delete a:::b\n\
         list [namespace exists ::a::b] [namespace exists ::a]");
    assert!(ok, "got: {res}");
    assert_eq!(res, "0 1");
}

// Tcl 8.6/9.0 (`Tcl_ParseVarName` in `generic/tclParse.c`) consumes the
// complete separator run before variable lookup.  These vectors exercise the
// execution path, rather than only the low-level scanner: `a:::b` and
// `::a:::b` address `::a::b`, while a trailing run is part of the variable
// name.  The array cases ensure the run is consumed before the index opener.
#[test]
fn variable_substitution_consumes_colon_runs() {
    let (ok, res, _) = run("namespace eval a {}\n\
         set ::a::b VALUE\n\
         set ::a::arr(k) ARRAY\n\
         list $a:::b $::a:::b $a:::arr(k) $::a:::arr(k)");
    assert!(ok, "got: {res}");
    assert_eq!(res, "VALUE VALUE ARRAY ARRAY");

    let (ok, res, _) = run("namespace eval foo {}\n\
         set ::foo::: EMPTY\n\
         list $foo:::");
    assert!(ok, "got: {res}");
    assert_eq!(res, "EMPTY");

    let (ok, msg, _) = run("set x $missing:::b");
    assert!(!ok);
    assert_eq!(msg, "can't read \"missing:::b\": no such variable");
}

// ===========================================================================
// namespace import / forget through colon runs
// ===========================================================================

/// `namespace import`/`forget` split the pattern at the last separator *run*
/// (the old `rsplit_once` produced the source namespace `src7:` — import
/// matched nothing and forget errored `unknown namespace`).
#[test]
fn import_and_forget_collapse_runs_in_patterns() {
    // tclsh8.6: import ::src7:::im* imports imp; forget ::src7:::im* removes it.
    let (ok, res, _) = run(
        "namespace eval src7 {namespace export im*; proc imp {} {return I}}\n\
         namespace eval use7 {namespace import ::src7:::im*}\n\
         set r [use7::imp]\n\
         namespace eval use7 {namespace forget ::src7:::im*}\n\
         list $r [info commands ::use7::*]",
    );
    assert!(ok, "got: {res}");
    assert_eq!(res, "I {}");
}

/// A single-segment qualified forget pattern (`namespace forget ::x`) is a
/// quiet no-op when no import's *origin* matches — the old split panicked on
/// this shape (`rsplit_once("::").unwrap()` over `x`).
#[test]
fn forget_with_root_qualified_pattern_is_a_quiet_no_op() {
    // tclsh8.6: imp stays imported (its origin is ::src7::imp, not ::imp).
    let (ok, res, _) = run(
        "namespace eval src7 {namespace export im*; proc imp {} {return I}}\n\
         namespace import ::src7::imp\n\
         namespace forget ::imp\n\
         info commands imp",
    );
    assert!(ok, "got: {res}");
    assert_eq!(res, "imp");
}

// ===========================================================================
// Qualified variable names
// ===========================================================================

/// Qualified variable names use the same colon-run and namespace-parent
/// contract as command names (`TclGetNamespaceForQualName`, `tclNamesp.c`).
#[test]
fn qualified_variable_names_canonicalise_runs_and_require_parent_namespaces() {
    // tclsh8.6/9.0: absolute and relative forms address the same canonical
    // variable; a missing parent is an error; a trailing run names the empty
    // variable in its parent namespace.
    let (ok, res, _) = run("namespace eval a {}\n\
         set ::a:::b one\n\
         set a::b two\n\
         set ::a::b");
    assert!(ok, "got: {res}");
    assert_eq!(res, "two");

    let (ok, res, _) = run("namespace eval n { namespace eval a {}\n\
             namespace eval foo {}\n\
             set a:::b three\n\
             set foo::: four\n\
             list [set ::n::a::b] [set ::n::foo::] }");
    assert!(ok, "got: {res}");
    assert_eq!(res, "three four");
    let (ok, msg, _) = run("set ::missing:::b 1");
    assert!(!ok);
    assert_eq!(
        msg,
        "can't set \"::missing:::b\": parent namespace doesn't exist"
    );

    let (ok, msg, _) = run("namespace eval n {set a:::b 1}");
    assert!(!ok);
    assert_eq!(msg, "can't set \"a:::b\": parent namespace doesn't exist");
}

/// A rooted single-segment import names the global namespace, not the current
/// namespace.  This is the VM-side regression for #1493.
#[test]
fn import_global_builtin_keeps_absolute_marker() {
    // tclsh8.6/9.0: the exact reproducer succeeds without an explicit export
    // (global builtins are not exported by default).  A second run exports it
    // so the resulting redirect is observable; `namespace qualifiers
    // ::lassign` itself remains `{}`.
    let (ok, res, _) = run("namespace eval n {namespace import ::lassign}");
    assert!(ok, "got: {res}");
    assert_eq!(res, "");
    let (ok, res, _) = run("namespace export lassign\n\
         namespace eval n {namespace import ::lassign}\n\
         n::lassign {a b} x y; list $x $y");
    assert!(ok, "got: {res}");
    assert_eq!(res, "a b");
}
