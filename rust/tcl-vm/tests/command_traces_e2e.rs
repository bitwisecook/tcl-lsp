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

//! M16.3 — command traces (`rename`/`delete`) and execution traces
//! (`enter`/`leave`/`enterstep`/`leavestep`) fire with C-faithful shapes.
//!
//! Every vector's stdout is compared against the bytecode VM **and** — when
//! installed — real `tclsh8.6` / `tclsh9.0` (identical output on both), so
//! the shapes cannot drift from C Tcl: fully-qualified names in command
//! traces, `{cmd-string op}` / `{cmd-string code result op}` argument forms,
//! an enter-trace error aborting the command, a leave-trace error replacing
//! its result, traces following `rename`, and redefinition firing `delete`.

use std::cell::RefCell;
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
    vm.set_compiler(Box::new(CompilerSvc {
        registry: CommandRegistry::build_default(),
    }));
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

struct Vector {
    name: &'static str,
    script: &'static str,
    want: &'static str,
}

const VECTORS: &[Vector] = &[
    Vector {
        name: "command traces: fully-qualified rename/delete shapes, following the rename",
        script: "proc tracer args { puts \"T:[join $args |]\" }\n\
                 proc victim {} {return V}\n\
                 trace add command victim {rename delete} tracer\n\
                 rename victim victim2\n\
                 rename victim2 {}\n",
        want: "T:::victim|::victim2|rename\nT:::victim2||delete",
    },
    Vector {
        name: "trace add command requires the command to exist",
        script: "proc tracer args {}\n\
                 puts [catch {trace add command missing {delete} tracer} m]:$m\n",
        want: "1:unknown command \"missing\"",
    },
    Vector {
        name: "redefining a traced command fires its delete trace",
        script: "proc tracer args { puts \"T:[join $args |]\" }\n\
                 proc dup {} {}\n\
                 trace add command dup delete tracer\n\
                 proc dup {} {return NEW}\n\
                 puts after-redef:[dup]\n",
        want: "T:::dup||delete\nafter-redef:NEW",
    },
    Vector {
        name: "namespace-qualified names arrive fully qualified",
        script: "proc tracer args { puts \"T:[join $args |]\" }\n\
                 namespace eval ns { proc inner {} {} }\n\
                 trace add command ns::inner rename tracer\n\
                 rename ns::inner ns::inner2\n",
        want: "T:::ns::inner|::ns::inner2|rename",
    },
    Vector {
        name: "execution enter/leave shapes: cmd-string, code, result",
        script: "proc etracer args { puts \"E:[join $args |]\" }\n\
                 proc p {a b} { return [list $a $b] }\n\
                 trace add execution p {enter leave} etracer\n\
                 p one {t w o}\n",
        want: "E:p one {t w o}|enter\nE:p one {t w o}|0|one {t w o}|leave",
    },
    Vector {
        name: "leave fires on error with code 1, and the error still propagates",
        script: "proc etracer args { puts \"E:[join $args |]\" }\n\
                 proc boom {} { error BOOM }\n\
                 trace add execution boom {enter leave} etracer\n\
                 puts catch:[catch {boom} m]:$m\n",
        want: "E:boom|enter\nE:boom|1|BOOM|leave\ncatch:1:BOOM",
    },
    Vector {
        name: "an enter-trace error aborts the traced command",
        script: "proc q {} { return Q }\n\
                 proc failtrace args { error TRACEFAIL }\n\
                 trace add execution q enter failtrace\n\
                 puts qcatch:[catch {q} m]:$m\n",
        want: "qcatch:1:TRACEFAIL",
    },
    Vector {
        name: "execution traces follow a rename and report the invoked name",
        script: "proc etracer args { puts \"E:[join $args |]\" }\n\
                 proc r {} { return R }\n\
                 trace add execution r {enter leave} etracer\n\
                 rename r r2\n\
                 r2\n",
        want: "E:r2|enter\nE:r2|0|R|leave",
    },
    Vector {
        name: "step traces fire per dispatched inner command, and trace info reports pairs",
        script: "proc st args { puts \"S:[join $args |]\" }\n\
                 proc stepped {} { format %s x }\n\
                 trace add execution stepped {enterstep leavestep} st\n\
                 stepped\n\
                 puts info:[trace info execution stepped]\n",
        want: "S:format %s x|enterstep\nS:format %s x|0|x|leavestep\n\
               info:{{enterstep leavestep} st}",
    },
    Vector {
        name: "a leave-trace error replaces the command's result",
        script: "proc q {} { return Q }\n\
                 proc ft args { error LEAVEFAIL }\n\
                 trace add execution q leave ft\n\
                 puts [catch {q} m]:$m\n",
        want: "1:LEAVEFAIL",
    },
    Vector {
        name: "a leavestep-trace error replaces the result too",
        script: "proc r {} { format %s R }\n\
                 proc st args { error STEPFAIL }\n\
                 trace add execution r leavestep st\n\
                 puts [catch {r} m]:$m\n",
        want: "1:STEPFAIL",
    },
    Vector {
        name: "trace remove stops the firing",
        script: "proc s {} { return S }\n\
                 proc et args { puts FIRED }\n\
                 trace add execution s enter et\n\
                 s\n\
                 trace remove execution s enter et\n\
                 s\n\
                 puts removed-ok\n",
        want: "FIRED\nremoved-ok",
    },
];

/// Documented divergence from C: step traces observe **dispatched** commands
/// (procs, non-inlined builtins).  Commands the compiler lowers to inline
/// opcodes (`set`, `incr`, `return`, …) never reach the dispatcher, so they
/// do not step — where C Tcl forces a step-traced proc out of bytecode and
/// fires for every inner command transitively (tclsh8.6-verified: the same
/// script prints `S:helperp|enterstep`, `S:return H|enterstep`,
/// `S:return H|2|H|leavestep`, `S:helperp|0|H|leavestep`).  Pinned here so a
/// future engine change (trace-aware compilation) shows up as a conscious
/// update, not silent drift.
#[test]
fn step_traces_observe_dispatched_commands_only_vm_divergence() {
    let out = vm_output(
        "proc st args { puts \"S:[join $args |]\" }\n\
         proc stepped2 {} { helperp }\n\
         proc helperp {} { return H }\n\
         trace add execution stepped2 {enterstep leavestep} st\n\
         stepped2\n",
    );
    assert_eq!(
        out,
        "S:helperp|enterstep\nS:helperp|0|H|leavestep",
        "the dispatched proc call steps; its inlined `return` body does not"
    );
}

#[test]
fn vm_matches_the_pinned_trace_vectors() {
    for v in VECTORS {
        assert_eq!(vm_output(v.script), v.want, "{}", v.name);
    }
}

/// The table itself is pinned to C Tcl (8.6 and 9.0 agree on every shape).
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
