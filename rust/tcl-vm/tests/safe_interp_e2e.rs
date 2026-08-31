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
use tcl_dialect::{DialectProfile, TclVersion};
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

/// [`run`] with the VM (and its compiler) pinned to a Tcl release, so a
/// release-gated command is genuinely absent from the surface rather than
/// merely unhidden — the shape `tclvm --tcl-version` builds.
fn run_at(version: TclVersion, src: &str) -> (bool, String) {
    let profile = tcl_registry::model::ingress::resolve_environment(version.dialect_profile_name())
        .analyser_profile();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let ir = tcl_compiler::lowering::lower_to_ir_for_bytecode_with_dialect(
        src,
        registry,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        Some(profile),
    );
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, registry);
    let cap = Capture::default();
    let mut vm = Vm::with_output(Box::new(cap.clone()));
    vm.set_dialect_profile(profile);
    vm.set_compiler(Box::new(ProfiledCompilerSvc { profile }));
    let c = vm.run_module(&asm);
    let out = String::from_utf8_lossy(&cap.0.borrow()).trim().to_string();
    (c.code.is_ok(), out)
}

#[test]
fn child_and_safe_platform_schemas_come_from_the_shared_owner() {
    let mut child_keys = tcl_platform::bootstrap::entries()
        .iter()
        .map(|entry| entry.name())
        .collect::<Vec<_>>();
    child_keys.sort_unstable();
    let (ok, out) = run("interp create child\n\
         puts [child eval {lsort [array names ::tcl_platform]}]\n");
    assert!(ok, "normal child schema query failed: {out}");
    assert_eq!(out, child_keys.join(" "));

    let scrubbed = tcl_platform::bootstrap::safe_scrub_keys().collect::<Vec<_>>();
    let mut safe_keys = tcl_platform::bootstrap::entries()
        .iter()
        .map(|entry| entry.name())
        .filter(|name| !scrubbed.contains(name))
        .collect::<Vec<_>>();
    safe_keys.sort_unstable();
    let (ok, out) = run("interp create -safe safe\n\
         puts [safe eval {lsort [array names ::tcl_platform]}]\n");
    assert!(ok, "safe child schema query failed: {out}");
    assert_eq!(out, safe_keys.join(" "));
    assert!(safe_keys.contains(&"threaded"));
}

/// A compile service pinned to one resolved profile, as `tclvm` wires it.
struct ProfiledCompilerSvc {
    profile: &'static DialectProfile,
}

impl CompileService for ProfiledCompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        self.compile_for_profile(src, self.profile)
    }

    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        let ir = tcl_compiler::lowering::lower_to_ir_for_bytecode_with_dialect(
            src,
            registry,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            Some(profile),
        );
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, registry))
    }
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

#[test]
fn marktrusted_preserves_a_visible_command_colliding_with_hidden_token() {
    let (ok, out) = run("interp create -safe s\n\
         s eval {proc open args {return mine}}\n\
         interp marktrusted s\n\
         puts [list [s eval {open}] [expr {[lsearch -exact [interp hidden s] open] >= 0}]]\n");
    assert!(ok, "marktrusted collision vector must complete: {out}");
    assert_eq!(out, "mine 1");
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

/// Every command `interp create -safe` hides, per release, measured on the
/// reference interpreters with
/// `interp create -safe s; lsort [interp hidden s]` — top-level command
/// names only. The `tcl:file:*` / `tcl:zipfs:*` / `tcl:clock:*` entries a
/// real 8.6+ interpreter also lists are C's internal rewrite names for the
/// *unsafe subcommands* of an ensemble, not commands a script can name;
/// neither engine models ensembles that way, so they are out of scope here.
///
/// Patch levels measured: 8.4.20, 8.5.19, 8.6.14, 9.0.4, 9.1b0.
const MEASURED_HIDDEN: &[(TclVersion, &[&str])] = &[
    (
        TclVersion::V8_4,
        &[
            "cd",
            "encoding",
            "exec",
            "exit",
            "fconfigure",
            "file",
            "glob",
            "load",
            "open",
            "pwd",
            "socket",
            "source",
        ],
    ),
    // 8.5 adds `unload` (TIP 100).
    (
        TclVersion::V8_5,
        &[
            "cd",
            "encoding",
            "exec",
            "exit",
            "fconfigure",
            "file",
            "glob",
            "load",
            "open",
            "pwd",
            "socket",
            "source",
            "unload",
        ],
    ),
    (
        TclVersion::V8_6,
        &[
            "cd",
            "encoding",
            "exec",
            "exit",
            "fconfigure",
            "file",
            "glob",
            "load",
            "open",
            "pwd",
            "socket",
            "source",
            "unload",
        ],
    ),
    // 9.0 adds `zipfs`.
    (
        TclVersion::V9_0,
        &[
            "cd",
            "encoding",
            "exec",
            "exit",
            "fconfigure",
            "file",
            "glob",
            "load",
            "open",
            "pwd",
            "socket",
            "source",
            "unload",
            "zipfs",
        ],
    ),
    // 9.1 additionally lists `clock`. That is *not* an unsafety fact and is
    // deliberately not in the registry's trait: 9.1 hides the C `clock` and
    // immediately re-provides a safe one, so `clock format 0 -gmt 1` works
    // inside a 9.1 safe child exactly as it does inside an 8.6 one
    // (measured). See `NOT_IMPLEMENTED` below.
    (
        TclVersion::V9_1,
        &[
            "cd",
            "clock",
            "encoding",
            "exec",
            "exit",
            "fconfigure",
            "file",
            "glob",
            "load",
            "open",
            "pwd",
            "socket",
            "source",
            "unload",
            "zipfs",
        ],
    ),
];

/// The measured names this VM cannot hide, with why. Listing them explicitly
/// rather than intersecting silently means implementing any of them forces
/// this test to be revisited.
const NOT_IMPLEMENTED: &[&str] = &[
    // Not commands in this VM at all: `info commands` is empty for each in a
    // plain, non-safe interpreter, so calling one already fails with
    // "invalid command name" for an unrelated reason and there is nothing to
    // park in the hidden table.
    "load", "unload", "socket", "zipfs",
    // `clock` is implemented and stays *visible*, which is what a real 9.1
    // safe child does behaviourally. Only its appearance in `interp hidden`
    // differs, and that is the safe base's hide-then-alias artefact.
    "clock",
];

/// The hidden set under each pinned release is exactly the measured tclsh
/// set, narrowed to the commands this VM implements — no name list in
/// `make_safe`, which is now the registry's `Traits::SAFE_INTERP_HIDDEN`
/// query (ledger row B2).
///
/// The narrowing is not a fudge: it is the mechanism. `unload` (8.5+) and
/// `zipfs` (9.0+) are release-gated commands, so "hide what the trait names,
/// if this interpreter carries it" reproduces the per-release differences
/// with no second availability rule — which is why the 8.4 row below has no
/// `unload` even though the trait does.
#[test]
fn safe_interp_hidden_set_matches_the_measured_tclsh_sets() {
    for &(version, measured) in MEASURED_HIDDEN {
        let expected: Vec<&str> = measured
            .iter()
            .copied()
            .filter(|name| !NOT_IMPLEMENTED.contains(name))
            .collect();
        let (ok, out) = run_at(
            version,
            "interp create -safe s\nputs [lsort [interp hidden s]]\n",
        );
        assert!(ok, "[{version:?}] must not error: {out}");
        let actual: Vec<&str> = out.split_whitespace().collect();
        assert_eq!(actual, expected, "[{version:?}] hidden set");
    }
}

/// FP guard on `NOT_IMPLEMENTED`'s first four entries: they are absent
/// because the VM has no such command, not because `make_safe` skipped
/// them. If one is ever implemented this fails, and the row above becomes
/// the right answer.
#[test]
fn the_unhidden_residue_is_genuinely_unimplemented() {
    let (ok, out) = run_at(
        TclVersion::V9_1,
        "foreach c {load unload socket zipfs} { puts \"$c [info commands $c]\" }\n",
    );
    assert!(ok, "must not error: {out}");
    for line in out.lines() {
        let (name, found) = line.split_once(' ').unwrap_or((line, ""));
        assert!(
            found.trim().is_empty(),
            "{name} is implemented now — move it out of NOT_IMPLEMENTED and \
             expect it in the hidden set"
        );
    }
}

/// TP: `clock` stays callable inside a safe child, on every release —
/// measured on tclsh 8.6.14, 9.0.4 and 9.1b0, where
/// `s eval {clock format 0 -gmt 1}` succeeds even though 9.1 lists `clock`
/// in `interp hidden`.
#[test]
fn clock_remains_callable_in_a_safe_child() {
    for &(version, _) in MEASURED_HIDDEN {
        let (ok, out) = run_at(
            version,
            "interp create -safe s\nputs [s eval {clock format 0 -gmt 1 -format %Y}]\n",
        );
        assert!(ok, "[{version:?}] must not error: {out}");
        assert_eq!(out, "1970", "[{version:?}] safe clock");
    }
}

/// Byte-compare the measured sets against a real interpreter when one is on
/// `PATH`; skips silently otherwise. This is the half that keeps
/// `MEASURED_HIDDEN` honest — the hermetic test above only pins what this
/// engine does with it.
#[test]
fn measured_hidden_sets_match_real_tclsh_when_available() {
    let mut checked = 0usize;
    for &(version, measured) in MEASURED_HIDDEN {
        let bin = format!("tclsh{}", version.version_string());
        let env = format!("TCLSH{}", version.version_string().replace('.', ""));
        let Some(out) = tclsh_output(
            &env,
            &[&bin],
            "interp create -safe s\nputs [lsort [interp hidden s]]\n",
        ) else {
            continue;
        };
        // Top-level command names only: C's ensemble-subcommand rewrite names
        // (`tcl:file:atime`, `tcl:zipfs:mount`, …) are not commands a script
        // can name and are not what this set models.
        let actual: Vec<&str> = out
            .split_whitespace()
            .filter(|name| !name.starts_with("tcl:"))
            .collect();
        assert_eq!(actual, measured, "[{version:?}] real tclsh hidden set");
        checked += 1;
    }
    if checked == 0 {
        eprintln!("no system tclsh found — pinned expectations still verified");
    }
}

/// Run `src` under a real tclsh, or `None` when that binary isn't available.
fn tclsh_output(bin_env: &str, names: &[&str], src: &str) -> Option<String> {
    use std::io::Write as _;
    let mut candidates: Vec<String> = Vec::new();
    if let Ok(explicit) = std::env::var(bin_env) {
        candidates.push(explicit);
    }
    candidates.extend(names.iter().map(ToString::to_string));
    for name in candidates {
        let Ok(mut child) = std::process::Command::new(&name)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
        else {
            continue;
        };
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(src.as_bytes());
        }
        let Ok(out) = child.wait_with_output() else {
            continue;
        };
        return Some(String::from_utf8_lossy(&out.stdout).trim().to_string());
    }
    None
}
