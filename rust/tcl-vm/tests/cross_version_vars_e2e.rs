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

//! M11 — cross-version variable semantics, the only resolution-semantics
//! change across 8.4→9.1: Tcl 8.x resolves an unqualified variable at
//! **namespace scope** to the global variable when the namespace has none but
//! the global namespace does (reads *and* writes); Tcl 9.0 removed the
//! fallback (TIP 278 — 8.6 `tclVar.c:757` keeps it, 9.0 forces
//! `TCL_NAMESPACE_ONLY`, `tclVar.c:935`).
//!
//! Each vector runs through the bytecode VM **twice** — once at
//! `TclVersion::V8_6` and once at `V9_0` — and, when the matching real
//! tclsh is installed (`tclsh8.6` / `tclsh9.0` on PATH, or
//! `TCL_LSP_TCLSH86` / `TCL_LSP_TCLSH90`), the same script is executed under
//! it and must agree — so the table can never drift from C Tcl.

use std::cell::RefCell;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir;
use tcl_dialect::TclVersion;
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

/// Run `src` in the VM at `version`; the script's `puts` output is returned
/// (the vectors communicate through stdout so the tclsh leg is directly
/// comparable).
fn vm_output(src: &str, version: TclVersion) -> String {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);

    let cap = Capture::default();
    let mut vm = Vm::with_output(Box::new(cap.clone()));
    vm.set_compiler(Box::new(CompilerSvc {
        registry: CommandRegistry::build_default(),
    }));
    vm.set_runtime_version(version);
    let _ = vm.run_module(&asm);
    String::from_utf8_lossy(&cap.0.borrow()).trim().to_string()
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
        child
            .stdin
            .as_mut()
            .expect("stdin")
            .write_all(src.as_bytes())
            .expect("write");
        let out = child.wait_with_output().expect("run");
        if out.status.success() {
            return Some(String::from_utf8_lossy(&out.stdout).trim().to_string());
        }
    }
    None
}

/// One behaviour vector: the script prints its observations; `want_8x` and
/// `want_90` are the full expected stdout under each semantic.
struct Vector {
    name: &'static str,
    script: &'static str,
    want_8x: &'static str,
    want_90: &'static str,
}

const VECTORS: &[Vector] = &[
    Vector {
        name: "read falls back to global only in 8.x",
        script: "set g GLOBAL\nnamespace eval foo { puts [catch {set g} m]:$m }\n",
        want_8x: "0:GLOBAL",
        want_90: "1:can't read \"g\": no such variable",
    },
    Vector {
        name: "write reaches the global in 8.x, creates in the namespace in 9.0",
        script: "set g OLD\nnamespace eval foo { set g NEW }\nputs [info exists ::foo::g]:[set ::g]\n",
        want_8x: "0:NEW",
        want_90: "1:OLD",
    },
    Vector {
        name: "a declared-but-unset `variable` blocks the fallback",
        script: "set v GLOBALV\nnamespace eval bar { variable v; puts [catch {set v}] }\n",
        want_8x: "1",
        want_90: "1",
    },
    Vector {
        name: "with neither cell, a write creates in the namespace",
        script: "namespace eval foo { set fresh NS }\nputs [info exists ::foo::fresh]:[info exists ::fresh]\n",
        want_8x: "1:0",
        want_90: "1:0",
    },
    Vector {
        name: "info exists agrees with the read rule",
        script: "set g G\nnamespace eval foo { puts [info exists g] }\n",
        want_8x: "1",
        want_90: "0",
    },
    Vector {
        name: "unset reaches the global in 8.x",
        script: "set g G\nputs [catch {namespace eval foo { unset g }}]:[info exists ::g]\n",
        want_8x: "0:0",
        want_90: "1:1",
    },
    Vector {
        name: "incr through the fallback mutates the global in 8.x",
        script: "set ctr 5\nnamespace eval foo { incr ctr }\nputs [set ::ctr]:[info exists ::foo::ctr]\n",
        want_8x: "6:0",
        want_90: "5:1",
    },
    Vector {
        name: "array element reads fall back in 8.x",
        script: "set arr(k) AV\nnamespace eval foo { puts [catch {set arr(k)} m]:$m }\n",
        want_8x: "0:AV",
        want_90: "1:can't read \"arr(k)\": no such variable",
    },
    Vector {
        name: "procs never use the namespace-scope fallback",
        script: "set g G\nnamespace eval foo { proc p {} { catch {set g} m; return $m } }\nputs [foo::p]\n",
        want_8x: "can't read \"g\": no such variable",
        want_90: "can't read \"g\": no such variable",
    },
];

#[test]
fn vm_matches_the_pinned_cross_version_vectors() {
    for v in VECTORS {
        assert_eq!(
            vm_output(v.script, TclVersion::V8_6),
            v.want_8x,
            "[8.6] {}",
            v.name,
        );
        assert_eq!(
            vm_output(v.script, TclVersion::V9_0),
            v.want_90,
            "[9.0] {}",
            v.name,
        );
    }
}

/// The table itself is pinned to C Tcl: every vector's `want` must match what
/// the matching real tclsh prints.  Skips silently per-binary when not
/// installed (CI / dev machines with `make ensure-test-deps` have both).
#[test]
fn vectors_match_real_tclsh() {
    let mut ran = 0;
    for v in VECTORS {
        if let Some(got) = tclsh_output("TCL_LSP_TCLSH86", &["tclsh8.6"], v.script) {
            assert_eq!(got, v.want_8x, "[tclsh8.6] {}", v.name);
            ran += 1;
        }
        if let Some(got) = tclsh_output("TCL_LSP_TCLSH90", &["tclsh9.0"], v.script) {
            assert_eq!(got, v.want_90, "[tclsh9.0] {}", v.name);
            ran += 1;
        }
    }
    if ran == 0 {
        eprintln!("skipping: neither tclsh8.6 nor tclsh9.0 found");
    }
}

/// M10.1 — the command-resolution `namespace path` tier is a Tcl 8.5 feature
/// (TIP 181): under an 8.4 runtime a bare call resolves current-namespace →
/// global only, never through the path.  (Real tclsh8.4 would reject the
/// `namespace path` subcommand outright — the VM models the *resolution*
/// semantics per version, not the per-version command surface, so the
/// recorded path is simply not consulted.)  The analyser applies the same
/// gate at its recording site (`bare_call_honours_namespace_path_only_from_8_5`).
#[test]
fn namespace_path_resolution_tier_is_8_5_plus_m10() {
    let script = "proc ::helper {} { return GLOBAL }\n\
         namespace eval ::mymod { proc helper {} { return MYMOD } }\n\
         namespace eval ::app { namespace path ::mymod }\n\
         namespace eval ::app { puts [helper] }\n";
    assert_eq!(
        vm_output(script, TclVersion::V8_4),
        "GLOBAL",
        "8.4 has no path tier"
    );
    for v in [TclVersion::V8_5, TclVersion::V8_6, TclVersion::V9_0] {
        assert_eq!(
            vm_output(script, v),
            "MYMOD",
            "8.5+ resolves through the namespace path"
        );
    }
}
