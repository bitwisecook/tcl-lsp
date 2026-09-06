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

//! M16.1 + M16.2 — `interp alias` loop prevention (C `TclPreventAliasLoop`,
//! 8.6 `tclInterp.c`) and cross-interp aliases between a parent and its
//! direct children, in both directions.
//!
//! Every vector is a complete script whose stdout is compared against the
//! bytecode VM **and** — when installed — real `tclsh8.6` / `tclsh9.0`
//! (identical expectations across versions; `TCL_LSP_TCLSH86` /
//! `TCL_LSP_TCLSH90` override the binary), so the table cannot drift from
//! C Tcl.  Key pinned facts:
//!
//! * a refused self-alias **destroys** the proc it clobbered (C creates the
//!   command first and does not restore it on rollback);
//! * `rename` of an alias onto a loop-forming name is refused with the
//!   command **restored** under its old name;
//! * a child→parent alias target runs at the parent's *current* frame
//!   (`info level` stacks on the pending frame, `uplevel 1` sees its
//!   locals), and its errors propagate catchably into the child;
//! * loops are detected across the interp boundary in both directions,
//!   with the error naming the *defining* command.

use std::cell::RefCell;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::Vm;

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

/// Run `src` in the VM; the script's `puts` output is returned.
fn vm_output(src: &str) -> String {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);

    let cap = Capture::default();
    let mut vm = Vm::with_output(Box::new(cap.clone()));
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
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

/// One behaviour vector: the script prints its observations; `want` is the
/// full expected stdout (identical under 8.6 and 9.0).
struct Vector {
    name: &'static str,
    script: &'static str,
    want: &'static str,
}

const VECTORS: &[Vector] = &[
    // -- M16.1: TclPreventAliasLoop --------------------------------------
    Vector {
        name: "a self-alias is refused AND destroys the proc it clobbered",
        script: "proc x {} {return REAL}\n\
                 puts [catch {interp alias {} x {} x} m]:$m\n\
                 puts [catch {x} m2]:$m2\n",
        want: "1:cannot define or rename alias \"x\": would create a loop\n\
               1:invalid command name \"x\"",
    },
    Vector {
        name: "a mutual pair is refused at the closing alias, which is removed",
        script: "interp alias {} a {} b\n\
                 puts [catch {interp alias {} b {} a} m]:$m\n\
                 puts [info commands b]\n",
        want: "1:cannot define or rename alias \"b\": would create a loop",
    },
    Vector {
        name: "rename onto a loop-forming name is refused and rolled back",
        script: "interp alias {} a {} b\n\
                 puts [catch {rename a b} m]:$m\n\
                 puts a=[info commands a],b=[info commands b]\n",
        want: "1:cannot define or rename alias \"b\": would create a loop\n\
               a=a,b=",
    },
    Vector {
        name: "a multi-hop chain with baked args still closes into a loop",
        script: "interp alias {} c {} d\n\
                 interp alias {} d {} e\n\
                 puts [catch {interp alias {} e {} c extra} m]:$m\n",
        want: "1:cannot define or rename alias \"e\": would create a loop",
    },
    Vector {
        name: "a qualified self-spelling is caught by resolution, not string compare",
        script: "puts [catch {interp alias {} q {} ::q} m]:$m\n",
        want: "1:cannot define or rename alias \"q\": would create a loop",
    },
    Vector {
        name: "an alias to an undefined target is legal (aliases late-bind)",
        script: "interp alias {} lb {} no_such_command_yet\n\
                 proc no_such_command_yet {} {return LATE}\n\
                 puts [lb]\n",
        want: "LATE",
    },
    // -- M16.2: child → parent -------------------------------------------
    Vector {
        name: "a child→parent target stacks on the parent's pending frame",
        script: "proc probe {} { return [info level] }\n\
                 proc probe2 {} { uplevel 1 {set marker} }\n\
                 proc runner {} {\n\
                     set marker M\n\
                     interp create c\n\
                     interp alias c pcall {} probe\n\
                     interp alias c pcall2 {} probe2\n\
                     set r1 [interp eval c { pcall }]\n\
                     set r2 [interp eval c { pcall2 }]\n\
                     interp delete c\n\
                     return \"$r1 $r2\"\n\
                 }\n\
                 puts [runner]\n",
        want: "2 M",
    },
    Vector {
        name: "a parent-side error propagates catchably into the child",
        script: "interp create c2\n\
                 interp alias c2 boom {} error KABOOM\n\
                 puts [catch {interp eval c2 { boom }} m]:$m\n\
                 puts [interp eval c2 { catch { boom } im; set im }]\n\
                 interp delete c2\n",
        want: "1:KABOOM\nKABOOM",
    },
    Vector {
        name: "baked args and call args concatenate across the boundary",
        script: "interp create c3\n\
                 interp alias c3 lister {} list A B\n\
                 puts [interp eval c3 { lister C D }]\n\
                 interp delete c3\n",
        want: "A B C D",
    },
    // -- M16.2: parent → child -------------------------------------------
    Vector {
        name: "a parent-side alias dispatches into the child",
        script: "interp create c4\n\
                 interp eval c4 { proc inchild {} { return CHILD } }\n\
                 interp alias {} fwd c4 inchild\n\
                 puts [fwd]\n\
                 interp delete c4\n",
        want: "CHILD",
    },
    // -- M16.1 × M16.2: loops across the interp boundary ------------------
    Vector {
        name: "a child→parent / parent→child pair is a loop (closed parent-side)",
        script: "interp create c5\n\
                 puts [interp alias c5 pcall {} fwd5]\n\
                 puts [catch {interp alias {} fwd5 c5 pcall} m]:$m\n\
                 puts [info commands fwd5]\n\
                 interp delete c5\n",
        want: "pcall\n\
               1:cannot define or rename alias \"fwd5\": would create a loop",
    },
    Vector {
        name: "the same loop closed child-side is caught too",
        script: "interp create c6\n\
                 interp alias {} fw2 c6 ptgt\n\
                 puts [catch {interp alias c6 ptgt {} fw2} m]:$m\n\
                 interp delete c6\n",
        want: "1:cannot define or rename alias \"ptgt\": would create a loop",
    },
];

#[test]
fn vm_matches_the_pinned_cross_interp_alias_vectors() {
    for v in VECTORS {
        assert_eq!(vm_output(v.script), v.want, "{}", v.name);
    }
}

/// The table itself is pinned to C Tcl: every vector's `want` must match what
/// the real tclsh prints (8.6 and 9.0 agree on all of these).  Skips
/// per-binary when not installed.
#[test]
fn vectors_match_real_tclsh() {
    let mut ran = 0;
    for v in VECTORS {
        for (env, names) in [
            ("TCL_LSP_TCLSH86", &["tclsh8.6"][..]),
            ("TCL_LSP_TCLSH90", &["tclsh9.0"][..]),
        ] {
            if let Some(got) = tclsh_output(env, names, v.script) {
                assert_eq!(got, v.want, "[{env}] {}", v.name);
                ran += 1;
            }
        }
    }
    if ran == 0 {
        eprintln!("skipping: neither tclsh8.6 nor tclsh9.0 found");
    }
}
