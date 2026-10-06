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

//! Step-trace (`enterstep`/`leavestep`) parity for opcode-inlined commands.
//!
//! C Tcl forces a step-traced proc "out of bytecode"
//! (`iPtr->flags |= DONT_COMPILE_CMDS_INLINE`, `tclTrace.c`
//! `Tcl_CreateObjTrace2`) the moment the first `enterstep`/`leavestep`-capable
//! execution trace exists anywhere in the interp, so every inner command —
//! including ones the compiler would otherwise inline as raw opcodes
//! (`set`, `incr`, `return`, `if`, `while`, …) — is individually dispatched
//! and observed. It reverts (recompiling back to fast/inlined form) once the
//! last such trace is removed.
//!
//! The VM's fix mirrors this: `Vm::step_trace_active` tracks whether any
//! step-capable trace exists; `Vm::ensure_proc_ready`/`ensure_proc_traced`
//! recompile a proc's body from its retained source text
//! (`ProcDef::body_src`) via `CompileService::compile_traced` — a compiler
//! pass with every registry-driven inline/structured lowering hook
//! suppressed (`Lowerer::trace_visible`) — on next entry, and revert the
//! same way when tracing stops. `Vm::compile_cached`/`eval_source` apply the
//! same choice to runtime-compiled bodies (`if`/`while`/`foreach`/`eval`/
//! `uplevel`/`catch`/`try`/… all evaluate their bodies through this path via
//! their runtime builtins), so the whole traced call tree — not just the
//! directly-traced proc — becomes trace-visible.
//!
//! Every vector's stdout is compared against the matching physical-core VM and
//! both pinned C8.6/C9.0 engines through the shared strict oracle runner. Older C
//! releases retain their measured step-trace and absent-core-feature controls;
//! Jim's unsupported execution-trace surface is checked independently.

mod common;

use tcl_dialect::TclVersion;
use tcl_test_support::{JimCapability, require_jimsh, required_tclshs};

fn vm_output(source: &str) -> String {
    common::vm_output(source, "tcl9.0")
}

/// One behaviour vector: the script prints its observations; `want` is the
/// full expected stdout (identical under 8.6 and 9.0 unless noted).
struct Vector {
    name: &'static str,
    script: &'static str,
    want: &'static str,
}

const VECTORS: &[Vector] = &[
    // TP: opcode-inlined forms now step.
    Vector {
        name: "TP: set/incr/return inside a step-traced proc all step",
        script: "proc helperp {} { set a 1; incr a; return H }\n\
                 proc st {cmd op} { puts \"S:$cmd|$op\" }\n\
                 proc st2 {cmd code result op} { puts \"S:$cmd|$code|$result|$op\" }\n\
                 trace add execution helperp enterstep st\n\
                 trace add execution helperp leavestep st2\n\
                 puts \"R:[helperp]\"\n",
        want: "S:set a 1|enterstep\n\
               S:set a 1|0|1|leavestep\n\
               S:incr a|enterstep\n\
               S:incr a|0|2|leavestep\n\
               S:return H|enterstep\n\
               S:return H|2|H|leavestep\n\
               R:H",
    },
    Vector {
        name: "TP: nested proc bodies step across the call boundary",
        script: "proc inner {} { set x 5; return I }\n\
                 proc outer {} { inner ; return O }\n\
                 proc st {cmd op} { puts \"S:$cmd|$op\" }\n\
                 trace add execution outer enterstep st\n\
                 puts \"R:[outer]\"\n",
        want: "S:inner|enterstep\n\
               S:set x 5|enterstep\n\
               S:return I|enterstep\n\
               S:return O|enterstep\n\
               R:O",
    },
    Vector {
        name: "TP: if/while control-flow headers AND their inlined bodies step",
        script: "proc p {} { if {1} { set y 2 } ; while {$y > 0} { incr y -1 } ; return done }\n\
                 proc st {cmd op} { puts \"S:$cmd|$op\" }\n\
                 trace add execution p enterstep st\n\
                 puts \"R:[p]\"\n",
        want: "S:if 1 { set y 2 }|enterstep\n\
               S:set y 2|enterstep\n\
               S:while {$y > 0} { incr y -1 }|enterstep\n\
               S:incr y -1|enterstep\n\
               S:incr y -1|enterstep\n\
               S:return done|enterstep\n\
               R:done",
    },
    Vector {
        name: "TP: eval/uplevel/subst bodies compile trace-visible too",
        script: "proc p {} { eval {set e 1} ; uplevel 0 {set u 2} ; subst {[set s 3]} ; return done }\n\
                 proc st {cmd op} { puts \"S:$cmd|$op\" }\n\
                 trace add execution p enterstep st\n\
                 puts \"R:[p]\"\n",
        want: "S:eval {set e 1}|enterstep\n\
               S:set e 1|enterstep\n\
               S:uplevel 0 {set u 2}|enterstep\n\
               S:set u 2|enterstep\n\
               S:subst {[set s 3]}|enterstep\n\
               S:set s 3|enterstep\n\
               S:return done|enterstep\n\
               R:done",
    },
    Vector {
        name: "TP: nested command substitution reports the substituted (not raw) command text",
        script: "proc helperp {} { set y [expr {1+1}]; set z $y; return H }\n\
                 proc st {cmd op} { puts \"S:$cmd|$op\" }\n\
                 trace add execution helperp enterstep st\n\
                 puts \"R:[helperp]\"\n",
        want: "S:expr 1+1|enterstep\n\
               S:set y 2|enterstep\n\
               S:set z 2|enterstep\n\
               S:return H|enterstep\n\
               R:H",
    },
    Vector {
        name: "TP: a step-traced proc's own dispatch reports both enterstep and leavestep",
        script: "proc helperp {} { set a 1; incr a; return H }\n\
                 proc st {args} { puts \"S:[join $args |]\" }\n\
                 trace add execution helperp {enterstep leavestep} st\n\
                 puts \"R:[helperp]\"\n",
        want: "S:set a 1|enterstep\n\
               S:set a 1|0|1|leavestep\n\
               S:incr a|enterstep\n\
               S:incr a|0|2|leavestep\n\
               S:return H|enterstep\n\
               S:return H|2|H|leavestep\n\
               R:H",
    },
    // TP: coroutine suspension keeps stepping after resume.
    Vector {
        // Creation and resume are nested inside one top-level `puts […]` (not
        // two separate top-level statements): a script piped to tclsh over
        // stdin runs in interactive mode, which calls `::tcl::history add`
        // for each *top-level* command it reads. Since `helperp`'s step scope
        // stays open across the coroutine suspension (`info coroutine`
        // exhibits the same span, verified separately), a second top-level
        // statement between creation and resume would also observe that
        // internal history bookkeeping — a real but harness-induced
        // artifact, not a semantic one, so the vector avoids it structurally.
        name: "TP: a step trace survives a yield/resume and observes post-resume inlined commands",
        script: "proc helperp {} { set a 1 ; yield ; incr a ; return H }\n\
                 proc st {cmd op} { puts \"S:$cmd|$op\" }\n\
                 trace add execution helperp enterstep st\n\
                 puts \"R:[list [coroutine k helperp] [k]]\"\n",
        want: "S:set a 1|enterstep\n\
               S:yield|enterstep\n\
               S:k|enterstep\n\
               S:incr a|enterstep\n\
               S:return H|enterstep\n\
               R:{} H",
    },
    // TP: TclOO method bodies step.
    Vector {
        // An explicitly-named object (`K create myobj`, not `K new`) keeps the
        // step-traced command string deterministic — `K new`'s auto-generated
        // object name is a counter the VM and tclsh do not share.
        name: "TP: a TclOO method body's inlined commands step via the calling proc's trace",
        script: "oo::class create K { method m {} { set mm 3 ; return M } }\n\
                 K create myobj\n\
                 proc p {} { myobj m }\n\
                 proc st {cmd op} { puts \"S:$cmd|$op\" }\n\
                 trace add execution p enterstep st\n\
                 puts \"R:[p]\"\n",
        want: "S:myobj m|enterstep\n\
               S:set mm 3|enterstep\n\
               S:return M|enterstep\n\
               R:M",
    },
    // TP: callback effects on inlined commands.
    Vector {
        name: "TP: an enterstep error on an inlined command aborts it",
        script: "proc p {} { set a 1 ; return $a }\n\
                 proc stE {cmd op} { if {[lindex $cmd 0] eq \"set\"} { error abort-set } }\n\
                 trace add execution p enterstep stE\n\
                 puts [catch {p} m]:$m\n",
        want: "1:abort-set",
    },
    Vector {
        name: "TP: a leavestep callback replaces an inlined command's result",
        script: "proc p {} { set a 1 ; return $a }\n\
                 proc stR {cmd code result op} { return -level 0 -code error replaced-by-leavestep }\n\
                 trace add execution p leavestep stR\n\
                 puts [catch {p} m]:$m\n",
        want: "1:replaced-by-leavestep",
    },
    Vector {
        name: "TP: a step callback that mutates a global variable is visible after the traced call",
        script: "proc p {} { set a 1 ; return $a }\n\
                 proc stMut {cmd op} { if {[lindex $cmd 0] eq \"set\"} { global sideeffect; set sideeffect yes } }\n\
                 trace add execution p enterstep stMut\n\
                 p\n\
                 puts side=$::sideeffect\n",
        want: "side=yes",
    },
    // TP: trace add/remove mid-execution + rename.
    Vector {
        name: "TP: adding a step trace to a callee mid-call still traces it once entered",
        script: "proc q {} { set b 9 ; return Q }\n\
                 proc addtrace {} { trace add execution q enterstep st ; return added }\n\
                 proc st {cmd op} { puts \"S:$cmd|$op\" }\n\
                 proc p {} { addtrace ; q }\n\
                 puts \"R:[p]\"\n",
        want: "S:set b 9|enterstep\n\
               S:return Q|enterstep\n\
               R:Q",
    },
    Vector {
        name: "TP: removing the last step trace reverts a proc to fast (unobserved) execution",
        script: "proc q {} { set b 9 ; return Q }\n\
                 proc st {cmd op} { puts \"S:$cmd|$op\" }\n\
                 trace add execution q enterstep st\n\
                 puts \"R1:[q]\"\n\
                 trace remove execution q enterstep st\n\
                 puts \"R2:[q]\"\n",
        want: "S:set b 9|enterstep\n\
               S:return Q|enterstep\n\
               R1:Q\n\
               R2:Q",
    },
    Vector {
        // Redefining a command drops every trace registered on it — C's
        // `Tcl_CreateObjCommand` overwrite semantics (tclsh9.0.3-verified:
        // the second call produces no `S:` output at all, matching a plain
        // `enter`/`leave` trace's identical drop-on-redefine behaviour).
        // The compiled-epoch recompile machinery must not resurrect a
        // dropped trace's deopt effect either — the new body compiles fast.
        name: "TP: redefining a step-traced proc drops the trace; the new body runs fast/unobserved",
        script: "proc q {} { set b 9 ; return Q1 }\n\
                 proc st {cmd op} { puts \"S:$cmd|$op\" }\n\
                 trace add execution q enterstep st\n\
                 puts \"R1:[q]\"\n\
                 proc q {} { set c 5 ; return Q2 }\n\
                 puts \"R2:[q]\"\n",
        want: "S:set b 9|enterstep\n\
               S:return Q1|enterstep\n\
               R1:Q1\n\
               R2:Q2",
    },
    Vector {
        name: "TP: a step-traced proc still steps after being renamed",
        script: "proc q {} { set b 9 ; return Q }\n\
                 proc st {cmd op} { puts \"S:$cmd|$op\" }\n\
                 trace add execution q enterstep st\n\
                 rename q q2\n\
                 puts \"R:[q2]\"\n",
        want: "S:set b 9|enterstep\n\
               S:return Q|enterstep\n\
               R:Q",
    },
    // FP: a plain enter/leave (no step) trace does not force deopt.
    Vector {
        name: "FP: a plain enter/leave trace (no step) never sees inlined set/incr — same as before the fix",
        script: "proc helperp {} { set a 1; incr a; return H }\n\
                 proc et {args} { puts \"E:[join $args |]\" }\n\
                 trace add execution helperp {enter leave} et\n\
                 puts \"R:[helperp]\"\n",
        want: "E:helperp|enter\n\
               E:helperp|0|H|leave\n\
               R:H",
    },
    // TN: an untraced proc runs unaffected.
    Vector {
        name: "TN: an untraced proc with the same shape produces no step output",
        script: "proc helperp2 {} { set a 1; incr a; return H }\n\
                 puts \"R:[helperp2]\"\n",
        want: "R:H",
    },
    // -- TN: step-tracing one proc does not affect an unrelated proc's output
    Vector {
        name: "TN: tracing proc A does not alter proc B's own (untraced) execution or result",
        script: "proc a_traced {} { set x 1 ; return A }\n\
                 proc b_untraced {} { set y 2 ; return B }\n\
                 proc st {cmd op} { puts \"S:$cmd|$op\" }\n\
                 trace add execution a_traced enterstep st\n\
                 puts \"RB:[b_untraced]\"\n\
                 puts \"RA:[a_traced]\"\n",
        want: "RB:B\n\
               S:set x 1|enterstep\n\
               S:return A|enterstep\n\
               RA:A",
    },
];

#[test]
fn vm_matches_the_pinned_step_trace_vectors() {
    for version in [TclVersion::V8_6, TclVersion::V9_0] {
        for vector in VECTORS {
            assert_eq!(
                common::vm_output(vector.script, version.dialect_name()),
                vector.want,
                "{version:?}: {}",
                vector.name
            );
        }
    }
}

/// The table is pinned to C Tcl: every vector's `want` must match what the
/// exact pinned C8.6 and C9.0 interpreters print; both are required.
#[test]
fn step_trace_vectors_match_real_tclsh() {
    let oracles = required_tclshs(&[TclVersion::V8_6, TclVersion::V9_0])
        .expect("both pinned step-trace engines");
    for oracle in oracles {
        for vector in VECTORS {
            assert_eq!(
                common::oracle_output(&oracle.path, vector.script),
                vector.want,
                "{}: {}",
                oracle.patchlevel,
                vector.name
            );
        }
    }
}

#[test]
fn step_trace_operand_substitution_matches_all_five_native_engines() {
    const SOURCE: &str = "proc p {} {set x 1; incr x; return $x}\nproc step {cmd op} {puts [list $cmd $op]}\ntrace add execution p enterstep step\nputs [p]\n";
    const EXPECTED: &str = "{set x 1} enterstep\n{incr x} enterstep\n{return 2} enterstep\n2";
    for oracle in required_tclshs(&TclVersion::ALL).expect("all five pinned step-trace engines") {
        assert_eq!(
            common::oracle_output(&oracle.path, SOURCE),
            EXPECTED,
            "{}",
            oracle.patchlevel
        );
        assert_eq!(
            common::vm_output(SOURCE, oracle.version.dialect_name()),
            EXPECTED,
            "{:?}",
            oracle.version
        );
    }
}

#[test]
fn older_step_trace_engines_do_not_supply_coroutine_tailcall_or_core_tcloo() {
    let source = "puts [list [llength [info commands ::coroutine]] [llength [info commands ::tailcall]] [llength [info commands ::oo::class]]]\n";
    for oracle in required_tclshs(&[TclVersion::V8_4, TclVersion::V8_5])
        .expect("both pinned older step-trace engines")
    {
        assert_eq!(common::oracle_output(&oracle.path, source), "0 0 0");
        assert_eq!(
            common::vm_output(source, oracle.version.dialect_name()),
            "0 0 0"
        );
    }
}

#[test]
fn jim_execution_traces_are_explicitly_unsupported() {
    let oracle = require_jimsh().expect("pinned Jim execution-trace surface");
    assert!(!oracle.supports(JimCapability::ExecutionTrace));
    let source = "puts [catch {trace add execution p enterstep list} message]\nputs $message\n";
    let expected = "1\ninvalid command name \"trace\"";
    assert_eq!(common::oracle_output(&oracle.path, source), expected);
    assert_eq!(common::vm_output(source, "jim"), expected);
}

/// Documented divergence from C, narrow in scope: C's interp-wide step trace
/// is torn down when the traced proc's
/// OWN `leave` event matches the level/command-string it was registered at
/// (`tcmdPtr->startLevel`/`startCmd`, `tclTrace.c`). A `tailcall` replaces
/// the traced proc's activation at the SAME level, which C's bookkeeping
/// treats as "the traced proc has left" — so the tail target's body runs
/// UNOBSERVED (tclsh9.0.4-verified: only `tailcall tgt|enterstep` prints,
/// nothing from `tgt`'s own body). The VM's step-scope model is a stack tied
/// to the traced proc's own *activation frame*, not a level/command-string
/// match, so it keeps observing through the tail-call replacement — a
/// broader (never a narrower) observation than C. Pinned here, as the
/// codebase's convention for a residual divergence, so a future engine
/// change shows up as a conscious update rather than silent drift.
#[test]
fn step_trace_over_observes_a_tailcall_target_vm_divergence() {
    let out = vm_output(
        "proc tgt {} { set t 1 ; return T }\n\
         proc p {} { tailcall tgt }\n\
         proc st {cmd op} { puts \"S:$cmd|$op\" }\n\
         trace add execution p enterstep st\n\
         puts \"R:[p]\"\n",
    );
    assert_eq!(
        out,
        "S:tailcall tgt|enterstep\n\
         S:tgt|enterstep\n\
         S:set t 1|enterstep\n\
         S:return T|enterstep\n\
         R:T",
        "the VM keeps observing tgt's body after the tailcall; C Tcl stops at the tailcall itself"
    );
}

/// A step-capable execution trace forces every procedure to recompile at its
/// next entry, and the refreshed body is memoised back into the table its
/// binding lives in. A retained namespace's procedure is not in the flat
/// command map at all, so memoising it there would republish it under a
/// spelling a recreation already owns. Exact tclsh 9.0.4 oracle results
/// (identical on 8.6.16).
#[test]
fn a_recompiled_retained_procedure_is_not_republished() {
    assert_eq!(
        vm_output(
            r"proc step {args} {}
namespace eval N {
    proc q {} {return Q}
    proc p {} {
        namespace delete ::N
        namespace eval ::N {proc other {} {return O}}
        list [q] [info commands ::N::*]
    }
}
trace add execution ::N::p enterstep step
set r [::N::p]
puts [list $r [info commands ::N::*] [namespace exists ::N]]"
        ),
        "{Q ::N::other} ::N::other 1"
    );
    assert_eq!(
        vm_output(
            r"proc step {args} {}
namespace eval N {
    proc q {} {return Q}
    proc p {} {
        namespace delete ::N
        namespace eval ::N {proc q {} {return NEW}}
        list [q] [namespace current]
    }
}
trace add execution ::N::p enterstep step
set r [::N::p]
puts [list $r [info commands ::N::*] [namespace exists ::N]]"
        ),
        "{Q ::N} ::N::q 1"
    );
}
