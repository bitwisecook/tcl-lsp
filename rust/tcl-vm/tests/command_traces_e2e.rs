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
use tcl_compiler::lowering::{lower_to_ir, lower_to_ir_traced};
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

    fn compile_traced(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        if let Some(msg) = tcl_compiler::lowering::first_fatal_parse_error(src) {
            return Err(CompileError(msg));
        }
        let ir = lower_to_ir_traced(src, &self.registry);
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
    // Firing order, issue #1440. Every trace list is prepended in C
    // (`TraceVarEx` tclTrace.c:3090-3092, `Tcl_TraceCommand` :1016-1018), and
    // each firing loop walks it head→tail — so the newest registration fires
    // first everywhere except the `leave`/`leavestep` reverse scan.
    Vector {
        name: "command rename/delete traces fire newest-first",
        script: "proc c1 args { puts \"c1:[join $args |]\" }\n\
                 proc c2 args { puts \"c2:[join $args |]\" }\n\
                 proc victim {} {}\n\
                 trace add command victim {rename delete} c1\n\
                 trace add command victim {rename delete} c2\n\
                 rename victim victim2\n\
                 rename victim2 {}\n",
        want: "c2:::victim|::victim2|rename\n\
               c1:::victim|::victim2|rename\n\
               c2:::victim2||delete\n\
               c1:::victim2||delete",
    },
    Vector {
        name: "execution enter fires newest-first, leave oldest-first",
        script: "proc e1 args { puts \"e1:[lindex $args end]\" }\n\
                 proc e2 args { puts \"e2:[lindex $args end]\" }\n\
                 proc target {} { return T }\n\
                 trace add execution target {enter leave} e1\n\
                 trace add execution target {enter leave} e2\n\
                 target\n",
        want: "e2:enter\ne1:enter\ne1:leave\ne2:leave",
    },
    Vector {
        name: "enterstep fires newest-first, leavestep oldest-first",
        script: "proc s1 args { puts \"s1:[lindex $args end]\" }\n\
                 proc s2 args { puts \"s2:[lindex $args end]\" }\n\
                 proc stepped {} { format %s x }\n\
                 trace add execution stepped {enterstep leavestep} s1\n\
                 trace add execution stepped {enterstep leavestep} s2\n\
                 stepped\n",
        want: "s2:enterstep\ns1:enterstep\ns1:leavestep\ns2:leavestep",
    },
    Vector {
        name: "variable write/read/unset traces fire newest-first",
        script: "proc t1 args { puts 1 }\n\
                 proc t2 args { puts 2 }\n\
                 proc t3 args { puts 3 }\n\
                 trace add variable v {read write unset} t1\n\
                 trace add variable v {read write unset} t2\n\
                 trace add variable v {read write unset} t3\n\
                 set v x\n\
                 puts -nonewline \"\"\n\
                 set ignore $v\n\
                 unset v\n",
        want: "3\n2\n1\n3\n2\n1\n3\n2\n1",
    },
    Vector {
        name: "whole-array traces fire before element traces, either registration order",
        script: "proc W args { puts \"W:[join $args |]\" }\n\
                 proc E args { puts \"E:[join $args |]\" }\n\
                 array set a {}\n\
                 trace add variable a write W\n\
                 trace add variable a(k) write E\n\
                 set a(k) 1\n\
                 array set b {}\n\
                 trace add variable b(k) write E\n\
                 trace add variable b write W\n\
                 set b(k) 2\n",
        want: "W:a|k|write\nE:a|k|write\nW:b|k|write\nE:b|k|write",
    },
    // `trace remove` breaks at the first match walking C's list head→tail, and
    // that head is the newest registration — so among identical duplicates the
    // NEWEST goes, which the surviving firing order and `trace info` both show.
    Vector {
        name: "trace remove drops the newest of several identical registrations",
        script: "proc cb1 args { puts c1 }\n\
                 proc cb2 args { puts c2 }\n\
                 trace add variable v write cb1\n\
                 trace add variable v write cb2\n\
                 trace add variable v write cb1\n\
                 trace remove variable v write cb1\n\
                 puts [trace info variable v]\n\
                 set v 1\n",
        want: "{write cb2} {write cb1}\nc2\nc1",
    },
    Vector {
        name: "the same newest-first removal rule for command and execution traces",
        script: "proc d1 args { puts d1 }\n\
                 proc d2 args { puts d2 }\n\
                 proc p {} {}\n\
                 trace add command p delete d1\n\
                 trace add command p delete d2\n\
                 trace add command p delete d1\n\
                 trace remove command p delete d1\n\
                 puts [trace info command p]\n\
                 proc q {} {}\n\
                 trace add execution q enter d1\n\
                 trace add execution q enter d2\n\
                 trace add execution q enter d1\n\
                 trace remove execution q enter d1\n\
                 puts [trace info execution q]\n",
        want: "{delete d2} {delete d1}\n{enter d2} {enter d1}",
    },
    // `trace info` renders the stored op set in the order each C `TRACE_INFO`
    // arm tests the flag bits — `array read write unset` and `rename delete`,
    // neither of which is the `opStrings[]` table order.
    Vector {
        name: "trace info renders ops in C's fixed per-kind order",
        script: "proc cb args {}\n\
                 proc p {} {}\n\
                 trace add command p {delete rename} cb\n\
                 trace add execution p {leavestep leave enterstep enter} cb\n\
                 trace add variable q {unset write read array} cb\n\
                 puts [trace info command p]\n\
                 puts [trace info execution p]\n\
                 puts [trace info variable q]\n",
        want: "{{rename delete} cb}\n\
               {{enter leave enterstep leavestep} cb}\n\
               {{array read write unset} cb}",
    },
];

/// Former divergence from C (issue #946 fault 3), now fixed: step traces used
/// to observe only **dispatched** commands (procs, non-inlined builtins) —
/// commands the compiler lowers to inline opcodes (`set`, `incr`, `return`, …)
/// never reached the dispatcher, so they did not step. C Tcl forces a
/// step-traced proc "out of bytecode" (`DONT_COMPILE_CMDS_INLINE`,
/// `tclTrace.c`) so every inner command fires transitively — this recompiles
/// the traced proc trace-visible on its next entry (and reverts the same way
/// once the last step-capable trace is removed), matching tclsh 8.6/9.0
/// exactly (tclsh9.0.3-verified).
#[test]
fn step_traces_observe_inlined_commands_too() {
    let out = vm_output(
        "proc st args { puts \"S:[join $args |]\" }\n\
         proc stepped2 {} { helperp }\n\
         proc helperp {} { return H }\n\
         trace add execution stepped2 {enterstep leavestep} st\n\
         stepped2\n",
    );
    assert_eq!(
        out,
        "S:helperp|enterstep\n\
         S:return H|enterstep\n\
         S:return H|2|H|leavestep\n\
         S:helperp|0|H|leavestep",
        "the dispatched proc call steps, AND its inlined `return` body steps too"
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
