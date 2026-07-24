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

//! Runtime enforcement of `interp create -safe` command hiding, for every
//! indirection shape the W129 static-lint fix (issue #1001) statically
//! flags — and a couple it deliberately does not (dynamic dispatch), which
//! must still be rejected here since the runtime is the actual enforcement
//! mechanism regardless of what the static lint catches.
//!
//! Before this file, `rust/tcl-vm` had no test at all asserting a hidden
//! command raises `invalid command name` inside a `-safe` interpreter — its
//! only "safe child" vector (`cross_interp_reentry_e2e.rs`,
//! `"TP: a safe child can still call a parent-target alias"`) tests the
//! opposite direction (an explicitly-wired alias still working). This file
//! closes that gap.
//!
//! Also pins the `after`/`vwait` fix: confirmed against real tclsh 8.6.14
//! (`interp create -safe s; s hidden` does not list either; `s eval {info
//! commands after}` returns `after`) that a prior version of `make_safe`'s
//! `UNSAFE` list incorrectly hid both, which would have broken legitimate
//! safe-interp code using `after idle`/`after cancel`.

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
        let ir = lower_to_ir(src, &self.registry);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.registry))
    }
}

#[derive(Clone, Default)]
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

/// Compile + run `src`; return `(ok, stdout)`, mirroring
/// `command_resolution_conformance.rs`'s helper of the same shape.
fn run(src: &str) -> (bool, String) {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);
    let cap = Capture::default();
    let mut vm = Vm::with_output(Box::new(cap.clone()));
    vm.set_compiler(Box::new(CompilerSvc {
        registry: CommandRegistry::build_default(),
    }));
    let c = vm.run_module(&asm);
    let out = String::from_utf8_lossy(&cap.0.borrow()).trim().to_string();
    (c.code.is_ok(), out)
}

/// TP: a hidden command called directly inside a safe child's `interp eval`
/// body raises `invalid command name` — the direct case (no indirection).
#[test]
fn hidden_command_direct_call_raises_invalid_command_name() {
    let (ok, out) = run("interp create -safe s\n\
         set rc [catch {interp eval s { source b.tcl }} err]\n\
         puts \"$rc $err\"\n");
    assert!(ok, "the catch itself must not error: {out}");
    assert!(out.starts_with("1 "), "source must fail: {out}");
    assert!(
        out.contains("invalid command name"),
        "expected the C-Tcl-faithful error text: {out}"
    );
}

/// TP: the same hidden command reached via `{*}[list source ...]` — the
/// runtime must reject this exactly like the direct call, independent of
/// whether the static W129 lint catches the shape.
#[test]
fn hidden_command_via_expand_list_quoting_raises_invalid_command_name() {
    let (ok, out) = run("interp create -safe s\n\
         set rc [catch {interp eval s { {*}[list source b.tcl] }} err]\n\
         puts \"$rc $err\"\n");
    assert!(ok, "the catch itself must not error: {out}");
    assert!(out.starts_with("1 "), "source must fail: {out}");
    assert!(
        out.contains("invalid command name"),
        "expected the C-Tcl-faithful error text: {out}"
    );
}

/// TP: `eval` of a dynamically-built string naming a hidden command — the
/// runtime resolves the evaluated script's command the same way it would
/// resolve any other, so this must fail too.
#[test]
fn hidden_command_via_eval_of_built_string_raises_invalid_command_name() {
    let (ok, out) = run("interp create -safe s\n\
         set rc [catch {interp eval s { eval [list source b.tcl] }} err]\n\
         puts \"$rc $err\"\n");
    assert!(ok, "the catch itself must not error: {out}");
    assert!(out.starts_with("1 "), "source must fail: {out}");
    assert!(
        out.contains("invalid command name"),
        "expected the C-Tcl-faithful error text: {out}"
    );
}

/// TP: a `namespace ensemble` `-map` redirect to a hidden command still
/// dispatches through the same visible-table-only lookup, so it must fail
/// too — the runtime has no ensemble-specific bypass.
#[test]
fn hidden_command_via_ensemble_map_redirect_raises_invalid_command_name() {
    let (ok, out) = run("interp create -safe s\n\
         set rc [catch {interp eval s {\n\
             namespace eval myns { namespace ensemble create -command myens -map {go source} }\n\
             myns::myens go b.tcl\n\
         }} err]\n\
         puts \"$rc $err\"\n");
    assert!(ok, "the catch itself must not error: {out}");
    assert!(
        out.starts_with("1 "),
        "the ensemble redirect must fail: {out}"
    );
    assert!(
        out.contains("invalid command name"),
        "expected the C-Tcl-faithful error text: {out}"
    );
}

/// TN (runtime side of the rename/alias investigation from issue #1001's
/// KCS doc): `rename` can only rename a command already in the *visible*
/// table, so it cannot resurrect a hidden command's callability — attempting
/// to rename an already-hidden `source` fails ("doesn't exist"), it does not
/// silently succeed and leave the renamed target callable.
#[test]
fn rename_cannot_resurrect_a_hidden_command() {
    let (ok, out) = run("interp create -safe s\n\
         set rc [catch {interp eval s { rename source mySource }} err]\n\
         puts \"$rc $err\"\n");
    assert!(ok, "the catch itself must not error: {out}");
    assert!(
        out.starts_with("1 "),
        "renaming an already-hidden command must fail: {out}"
    );
}

/// Regression guard for the `after`/`vwait` fix: both stay present and
/// callable inside a safe interpreter (confirmed against real tclsh
/// 8.6.14 — see this file's module doc) — a prior version of `make_safe`
/// incorrectly hid them.
#[test]
fn after_and_vwait_remain_callable_in_a_safe_interp() {
    let (ok, out) = run("interp create -safe s\n\
         set rc [catch {interp eval s { after idle {} }} err]\n\
         puts \"$rc $err\"\n\
         puts [interp eval s { info commands after }]\n\
         puts [interp eval s { info commands vwait }]\n\
         puts [expr {[lsearch [interp hidden s] after] < 0}]\n\
         puts [expr {[lsearch [interp hidden s] vwait] < 0}]\n");
    assert!(ok, "must not error: {out}");
    let lines: Vec<&str> = out.lines().collect();
    assert!(
        lines.first().is_some_and(|l| l.starts_with("0 ")),
        "`after idle {{}}` must succeed inside a safe interp: {out}"
    );
    assert_eq!(lines.get(1), Some(&"after"), "after must be visible: {out}");
    assert_eq!(lines.get(2), Some(&"vwait"), "vwait must be visible: {out}");
    assert_eq!(
        lines.get(3),
        Some(&"1"),
        "after must not be in the hidden set: {out}"
    );
    assert_eq!(
        lines.get(4),
        Some(&"1"),
        "vwait must not be in the hidden set: {out}"
    );
}

/// The registry's `Traits::SAFE_INTERP_HIDDEN` set is 13 commands (`source`,
/// `load`, `file`, `exec`, `open`, `socket`, `cd`, `pwd`, `glob`, `exit`,
/// `fconfigure`, `encoding`, `unload` — pinned against real tclsh 8.6.14,
/// `interp create -safe s; s hidden`). `tcl-vm` does not implement `load` /
/// `unload` / `socket` as real commands at all (nothing to hide — calling
/// any of the three already fails with "invalid command name" regardless of
/// safe-interp state, for an unrelated reason — confirmed via `info
/// commands` returning empty for each in a plain, non-safe interpreter), so
/// only the 10 it does implement are checked here.
#[test]
fn safe_interp_hides_every_implemented_command_in_the_registry_set() {
    let (ok, out) = run("interp create -safe s\n\
         foreach c {source file exec open cd pwd glob exit fconfigure encoding} {\n\
             puts \"$c [expr {[lsearch [interp hidden s] $c] >= 0}]\"\n\
         }\n");
    assert!(ok, "must not error: {out}");
    for line in out.lines() {
        assert!(
            line.ends_with(" 1"),
            "expected every listed command hidden, got: {line:?} (full: {out:?})"
        );
    }
}
