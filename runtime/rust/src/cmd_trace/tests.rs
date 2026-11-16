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

use crate::counters;
use crate::interp::{Code, Interp};

fn leak_free(body: impl FnOnce(&mut Interp)) {
    leak_free_with(Interp::new, body);
}

fn leak_free_with(create: impl FnOnce() -> Interp, body: impl FnOnce(&mut Interp)) {
    counters::reset();
    {
        let mut interp = create();
        body(&mut interp);
    }
    assert_eq!(
        counters::finalize(),
        0,
        "residual: {} objs, {} bufs",
        counters::live_objs(),
        counters::live_bufs()
    );
    assert_eq!(counters::double_free_count(), 0);
}

fn ok(i: &mut Interp, src: &[u8]) -> Vec<u8> {
    let code = i.eval_str(src);
    assert!(
        !i.host_refusal_pending(),
        "admission {:?}, native access {:?}",
        i.native_compilation_admission_error(),
        i.native_access_refusal()
    );
    assert_eq!(
        code,
        Code::Ok,
        "eval {:?} → {:?}",
        String::from_utf8_lossy(src),
        String::from_utf8_lossy(&i.result_bytes())
    );
    i.result_bytes()
}

#[test]
fn execution_trace_epochs_follow_only_native_hook_boundaries() {
    use tcl_dialect::TclVersion;
    use tcl_runtime_api::native_compilation::NativeCompilerHookPresence;
    fn uncompiled(_: &mut Interp, _: &[*mut crate::obj::TclObj]) -> Code {
        Code::Ok
    }
    for version in [
        TclVersion::V8_4,
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        leak_free_with(
            || {
                Interp::with_native_core(
                    crate::interp::default_host(),
                    crate::environment::profile_for_dialect(version.dialect_name()),
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .unwrap()
            },
            |i| {
                i.register_builtin(b"uncompiled", uncompiled);
                let set = i.resolve_cmd_token(b"set").unwrap();
                let other = i.resolve_cmd_token(b"uncompiled").unwrap();
                assert_eq!(
                    i.namespaces().native_compiler_hook(set),
                    Some(NativeCompilerHookPresence::Present),
                );
                assert_eq!(
                    i.namespaces().native_compiler_hook(other),
                    Some(NativeCompilerHookPresence::Absent),
                );
                let epochs = |i: &Interp| {
                    i.namespaces()
                        .native_compiler_cache_epochs(crate::namespace::GLOBAL)
                        .unwrap()
                };
                let (before, resolver) = epochs(i);
                ok(i, b"trace add command set rename observer");
                assert_eq!(epochs(i), (before, resolver));
                ok(i, b"trace add execution set enter observer");
                assert_eq!(epochs(i), (before + 1, resolver));
                ok(i, b"trace add execution set leave other_observer");
                assert_eq!(epochs(i), (before + 1, resolver));
                ok(i, b"trace remove execution set enter missing_observer");
                assert_eq!(epochs(i), (before + 1, resolver));
                ok(i, b"trace remove execution set enter observer");
                assert_eq!(epochs(i), (before + 1, resolver));
                ok(i, b"trace remove command set rename observer");
                assert_eq!(epochs(i), (before + 1, resolver));
                ok(i, b"trace remove execution set leave other_observer");
                assert_eq!(epochs(i), (before + 2, resolver));
                ok(i, b"trace add execution uncompiled enter observer");
                ok(i, b"trace remove execution uncompiled enter observer");
                assert_eq!(epochs(i), (before + 2, resolver));
            },
        );
    }
}

#[test]
fn write_trace_fires_and_records() {
    leak_free(|i| {
        ok(i, b"set log {}");
        ok(i, b"proc rec {name elem op} {global log; lappend log $op}");
        ok(i, b"trace add variable v write rec");
        ok(i, b"set v 1");
        ok(i, b"set v 2");
        assert_eq!(ok(i, b"set log"), b"write write");
        // info reports the registration.
        assert_eq!(ok(i, b"trace info variable v"), b"{write rec}");
        // remove stops further firing.
        ok(i, b"trace remove variable v write rec");
        ok(i, b"set v 3");
        assert_eq!(ok(i, b"set log"), b"write write");
        i.eval_str(b"unset -nocomplain v log");
    });
}

/// Adding a trace to an array element vivifies the array (undefined), so a
/// later read reports the element-aware miss and array/element existence
/// match C (trace-1.4); tracing an element of an existing scalar errors.
#[test]
fn trace_array_element_vivifies_array() {
    leak_free(|i| {
        ok(i, b"proc foo args {}");
        ok(i, b"trace add variable x(2) read foo");
        assert_eq!(ok(i, b"array exists x"), b"1");
        assert_eq!(ok(i, b"info exists x"), b"1");
        assert_eq!(ok(i, b"info exists x(2)"), b"0");
        assert_eq!(i.eval_str(b"set x(2)"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can't read \"x(2)\": no such element in array"
        );
        i.eval_str(b"unset -nocomplain x");
    });
    leak_free(|i| {
        ok(i, b"set y foo");
        assert_eq!(i.eval_str(b"trace add variable y(2) read foo"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can't trace \"y(2)\": variable isn't array"
        );
        i.eval_str(b"unset -nocomplain y");
    });
}

#[test]
fn trace_registration_vivifies_only_on_add_and_uses_canonical_sets() {
    leak_free(|i| {
        ok(i, b"proc cb args {}");
        // A scalar registration creates Tcl's unset Var, not an empty
        // value. Its callback is not invoked during registration.
        ok(i, b"trace add variable fresh {write read write} cb");
        assert_eq!(ok(i, b"info exists fresh"), b"0");
        assert_eq!(ok(i, b"trace info variable fresh"), b"{{read write} cb}");

        // Identical registrations are distinct; removal consumes exactly
        // one first match and does not materialise a missing variable.
        ok(i, b"trace add variable fresh {read write} cb");
        ok(i, b"trace remove variable fresh {write read} cb");
        assert_eq!(ok(i, b"trace info variable fresh"), b"{{read write} cb}");
        ok(i, b"trace remove variable absent read cb");
        assert_eq!(ok(i, b"info exists absent"), b"0");

        assert_eq!(
            err(i, b"trace add {} fresh read cb"),
            b"ambiguous option \"\": must be execution, command, or variable"
        );
        assert_eq!(
            err(i, b"trace add variable ::missing::x read cb"),
            b"can't trace \"::missing::x\": parent namespace doesn't exist"
        );
    });
}

#[test]
fn active_trace_does_not_suppress_an_unrelated_variable_trace() {
    leak_free(|i| {
        ok(i, b"set x 0");
        ok(i, b"set y 0");
        ok(i, b"set hits 0");
        ok(i, b"proc x_cb args {global y; incr y}");
        ok(i, b"proc y_cb args {global hits; incr hits}");
        ok(i, b"trace add variable x write x_cb");
        ok(i, b"trace add variable y write y_cb");
        ok(i, b"set x 1");
        assert_eq!(ok(i, b"set hits"), b"1");
    });
}

#[test]
fn active_trace_suppresses_every_trace_on_its_variable_cell() {
    leak_free(|i| {
        ok(i, b"set x 0");
        ok(i, b"set hits_one 0");
        ok(i, b"set hits_two 0");
        ok(
            i,
            b"proc one args {global hits_one x; incr hits_one; incr x}",
        );
        ok(i, b"proc two args {global hits_two; incr hits_two}");
        ok(i, b"trace add variable x write one");
        ok(i, b"trace add variable x write two");
        ok(i, b"set x 1");
        // The outer write invokes both registrations. The nested `incr x`
        // sees the same active cell, so it invokes neither.
        assert_eq!(ok(i, b"set hits_one"), b"1");
        assert_eq!(ok(i, b"set hits_two"), b"1");
    });
}

#[test]
fn read_trace_fires() {
    leak_free(|i| {
        ok(i, b"set hits 0");
        ok(i, b"proc bump {args} {global hits; incr hits}");
        ok(i, b"set x 5");
        ok(i, b"trace add variable x read bump");
        ok(i, b"set y $x");
        ok(i, b"set y $x");
        assert_eq!(ok(i, b"set hits"), b"2");
        i.eval_str(b"unset -nocomplain x y hits");
    });
}

fn err(i: &mut Interp, src: &[u8]) -> Vec<u8> {
    assert_eq!(i.eval_str(src), Code::Error, "expected error for {src:?}");
    i.result_bytes()
}

#[test]
fn exec_trace_register_info_remove_roundtrip() {
    leak_free(|i| {
        ok(i, b"proc foo {a} {return $a}");
        ok(i, b"trace add execution foo {enter leave} cb1");
        ok(i, b"trace add execution foo enterstep cb2");
        // Most-recent first; ops in C's fixed print order within each entry.
        assert_eq!(
            ok(i, b"trace info execution foo"),
            b"{enterstep cb2} {{enter leave} cb1}"
        );
        // A command trace does not show under `execution` (category filter).
        ok(i, b"trace add command foo delete cbd");
        assert_eq!(
            ok(i, b"trace info execution foo"),
            b"{enterstep cb2} {{enter leave} cb1}"
        );
        assert_eq!(ok(i, b"trace info command foo"), b"{delete cbd}");
        // Remove the first exact match.
        ok(i, b"trace remove execution foo {enter leave} cb1");
        assert_eq!(ok(i, b"trace info execution foo"), b"{enterstep cb2}");
    });
}

#[test]
fn cmd_trace_info_op_order_is_rename_then_delete() {
    leak_free(|i| {
        ok(i, b"proc foo {} {}");
        ok(i, b"trace add command foo {delete rename} cb");
        assert_eq!(ok(i, b"trace info command foo"), b"{{rename delete} cb}");
    });
}

/// Hidden and visible commands may use the same display spelling at once.
/// Their trace lists follow their command generations independently.
#[test]
fn hidden_and_visible_same_name_traces_remain_isolated() {
    leak_free(|i| {
        assert_eq!(
                ok(
                    i,
                    br#"set log {}
                        proc hc {old new op} {
                            lappend ::log [list hidden $old $new $op]
                        }
                        proc vc {old new op} {
                            lappend ::log [list visible $old $new $op]
                        }
                        proc old {} {return OLD}
                        trace add command old delete hc
                        interp hide {} old shared
                        proc shared {} {return NEW}
                        trace add command shared delete vc
                        set before [trace info command shared]
                        rename shared {}
                        set mid $log
                        interp expose {} shared exposed
                        set after [trace info command exposed]
                        rename exposed {}
                        list $before $mid $after $log"#,
                ),
                br#"{{delete vc}} {{visible ::shared {} delete}} {{delete hc}} {{visible ::shared {} delete} {hidden ::exposed {} delete}}"#,
            );
    });
}

/// `interp invokehidden` dispatches a real command token even though
/// namespace lookup cannot see it. Its execution trace sidecar follows the
/// hidden binding and is selected by that binding's generation.
#[test]
fn invokehidden_runs_the_hidden_commands_execution_traces() {
    leak_free(|i| {
        assert_eq!(
                ok(
                    i,
                    br#"set log {}
                        proc cb args {global log; lappend log $args}
                        proc foo {} {set x 1; return ok}
                        trace add execution foo {enter leave enterstep leavestep} cb
                        interp hide {} foo h
                        set result [interp invokehidden {} h]
                        interp expose {} h visible
                        list $result $log [trace info execution visible]"#,
                ),
                br#"ok {{h enter} {{set x 1} enterstep} {{set x 1} 0 1 leavestep} {{return ok} enterstep} {{return ok} 2 ok leavestep} {h 0 ok leave}} {{{enter leave enterstep leavestep} cb}}"#,
            );
    });
}

#[test]
fn trace_errors_match_c() {
    leak_free(|i| {
        ok(i, b"proc foo {} {}");
        assert_eq!(
            err(i, b"trace add bogus foo enter cb"),
            b"bad option \"bogus\": must be execution, command, or variable"
        );
        assert_eq!(
            err(i, b"trace add execution nosuch enter cb"),
            b"unknown command \"nosuch\""
        );
        assert_eq!(
                err(i, b"trace add execution foo {} cb"),
                b"bad operation list \"\": must be one or more of enter, leave, enterstep, or leavestep"
            );
        assert_eq!(
            err(i, b"trace add execution foo bogus cb"),
            b"bad operation \"bogus\": must be enter, leave, enterstep, or leavestep"
        );
        assert_eq!(
            err(i, b"trace add command foo bogus cb"),
            b"bad operation \"bogus\": must be delete or rename"
        );
        assert_eq!(
            err(i, b"trace info command nosuch"),
            b"unknown command \"nosuch\""
        );
        assert_eq!(
            err(i, b"trace remove command nosuch delete cb"),
            b"unknown command \"nosuch\""
        );
    });
}

#[test]
fn command_trace_fires_on_rename_and_delete() {
    leak_free(|i| {
        ok(i, b"set log {}");
        ok(i, b"proc cb {args} {global log; lappend log $args}");
        ok(i, b"proc foo {} {return hi}");
        ok(i, b"trace add command foo {rename delete} cb");
        ok(i, b"rename foo bar");
        // FQN old/new + op; the trace follows the command to ::bar.
        assert_eq!(ok(i, b"set log"), b"{::foo ::bar rename}");
        assert_eq!(ok(i, b"trace info command bar"), b"{{rename delete} cb}");
        ok(i, b"rename bar {}");
        assert_eq!(ok(i, b"set log"), b"{::foo ::bar rename} {::bar {} delete}");
        // The trace went away with the command.
        assert_eq!(ok(i, b"set log2 [info commands bar]"), b"");
        i.eval_str(b"unset -nocomplain log log2");
    });
}

#[test]
fn command_trace_callback_error_is_ignored() {
    leak_free(|i| {
        ok(i, b"proc boom {args} {error kaboom}");
        ok(i, b"proc q {} {}");
        ok(i, b"trace add command q delete boom");
        // Delete still succeeds; the callback error is swallowed.
        assert_eq!(ok(i, b"rename q {}"), b"");
    });
}

#[test]
fn exec_trace_enter_leave_fire() {
    leak_free(|i| {
        ok(i, b"set log {}");
        ok(i, b"proc cb {args} {global log; lappend log $args}");
        ok(i, b"proc foo {a b} {return $a-$b}");
        ok(i, b"trace add execution foo {enter leave} cb");
        assert_eq!(ok(i, b"foo x y"), b"x-y");
        // enter: {cmd args} enter ; leave: {cmd args} code result leave.
        assert_eq!(
            ok(i, b"set log"),
            &b"{{foo x y} enter} {{foo x y} 0 x-y leave}"[..]
        );
        i.eval_str(b"unset -nocomplain log");
    });
}

#[test]
fn exec_trace_order_and_live_result() {
    leak_free(|i| {
        ok(i, b"set log {}");
        ok(i, b"proc cb {args} {global log; lappend log $args}");
        ok(i, b"proc baz {} {return R}");
        ok(i, b"trace add execution baz {enter leave} {cb 1}");
        ok(i, b"trace add execution baz {enter leave} {cb 2}");
        ok(i, b"baz");
        // enter newest-first (2,1); leave oldest-first (1,2); the 2nd leave
        // sees the 1st leave callback's result (live, not preserved).
        assert_eq!(
                ok(i, b"set log"),
                &b"{2 baz enter} {1 baz enter} {1 baz 0 R leave} {2 baz 0 {{2 baz enter} {1 baz enter} {1 baz 0 R leave}} leave}"[..]
            );
        i.eval_str(b"unset -nocomplain log");
    });
}

#[test]
fn exec_trace_enter_error_aborts_leave_error_overrides() {
    leak_free(|i| {
        ok(i, b"proc bar {} {return ok}");
        ok(i, b"proc deny {args} {error NOPE}");
        ok(i, b"trace add execution bar enter deny");
        assert_eq!(err(i, b"bar"), b"NOPE");
        ok(i, b"trace remove execution bar enter deny");
        // Command runs again cleanly once the enter trace is gone.
        assert_eq!(ok(i, b"bar"), b"ok");
        // A leave-trace error overrides the command's result/code.
        ok(i, b"proc boom {args} {error OVERRIDE}");
        ok(i, b"trace add execution bar leave boom");
        assert_eq!(err(i, b"bar"), b"OVERRIDE");
    });
}

#[test]
fn step_trace_fires_for_body_commands() {
    leak_free(|i| {
        ok(i, b"set log {}");
        ok(i, b"proc tr {args} {global log; lappend log $args}");
        ok(i, b"proc foo {} {set a 1; set b 2}");
        ok(i, b"trace add execution foo {enterstep leavestep} tr");
        ok(i, b"foo");
        // Each body command gets enterstep before and leavestep after, with
        // <code> <result>; foo itself is not stepped by its own trace.
        assert_eq!(
                ok(i, b"set log"),
                &b"{{set a 1} enterstep} {{set a 1} 0 1 leavestep} {{set b 2} enterstep} {{set b 2} 0 2 leavestep}"[..]
            );
        i.eval_str(b"unset -nocomplain log");
    });
}

// Needs the numeric tower: the traced proc body is `if`+`expr` (quoted in the pinned log).
#[cfg(have_tommath)]
#[test]
fn step_trace_recursion_installs_once() {
    leak_free(|i| {
        ok(i, b"set log {}");
        ok(
            i,
            b"proc tr {args} {global log; lappend log [lindex $args 0]}",
        );
        ok(i, b"proc rec {n} {if {$n>0} {rec [expr {$n-1}]}}");
        ok(i, b"trace add execution rec enterstep tr");
        ok(i, b"rec 1");
        // Only the outermost rec installs the step trace; the inner call is
        // stepped by that single active trace (no double-firing).
        assert_eq!(
                ok(i, b"set log"),
                &b"{if {$n>0} {rec [expr {$n-1}]}} {expr {$n-1}} {rec 0} {if {$n>0} {rec [expr {$n-1}]}}"[..]
            );
        i.eval_str(b"unset -nocomplain log");
    });
}

#[test]
fn redefining_a_proc_clears_and_fires_delete_traces() {
    leak_free(|i| {
        ok(i, b"set log {}");
        ok(i, b"proc cb {args} {global log; lappend log $args}");
        ok(i, b"proc foo {} {}");
        ok(i, b"trace add command foo delete cb");
        ok(i, b"trace add execution foo enter cb");
        // Redefining foo deletes the old command: fire its delete trace,
        // drop all its traces (C's Tcl_CreateObjCommand replace).
        ok(i, b"proc foo {} {}");
        assert_eq!(ok(i, b"set log"), b"{::foo {} delete}");
        assert_eq!(ok(i, b"trace info command foo"), b"");
        assert_eq!(ok(i, b"trace info execution foo"), b"");
        i.eval_str(b"unset -nocomplain log");
    });
}

#[test]
fn write_trace_error_propagates() {
    leak_free(|i| {
        ok(i, b"unset -nocomplain z");
        ok(
            i,
            b"trace add variable z write {unset z; error {memory corruption};#}",
        );
        // The set fails with the trace's message; C's TclObjCallVarTraces.
        assert_eq!(err(i, b"set z 1"), b"can't set \"z\": memory corruption");
        i.eval_str(b"unset -nocomplain z");
    });
}

#[test]
fn read_trace_error_propagates() {
    leak_free(|i| {
        ok(i, b"set w 5");
        ok(i, b"trace add variable w read {error boom;#}");
        // Both the `$w` and `set w` read forms fail with `can't read`.
        assert_eq!(err(i, b"set w"), b"can't read \"w\": boom");
        assert_eq!(err(i, b"set q $w"), b"can't read \"w\": boom");
        i.eval_str(b"unset -nocomplain w q");
    });
}

#[test]
fn unset_trace_error_is_ignored() {
    leak_free(|i| {
        ok(i, b"set v 1");
        ok(i, b"trace add variable v unset {error boom;#}");
        // Unset-trace errors are swallowed (C ignores them).
        assert_eq!(ok(i, b"unset v"), b"");
    });
}

#[test]
fn unsetting_a_variable_clears_its_traces() {
    leak_free(|i| {
        ok(i, b"set x 1");
        ok(i, b"proc cb {args} {}");
        ok(i, b"trace add variable x read cb");
        ok(i, b"unset x");
        ok(i, b"set x 2");
        // The trace died with the variable (C frees the Var's trace list).
        assert_eq!(ok(i, b"trace info variable x"), b"");
        i.eval_str(b"unset -nocomplain x");
    });
}

#[test]
fn unset_keeps_traces_on_same_named_variables_in_other_namespaces() {
    leak_free(|i| {
        ok(
            i,
            b"proc record {label args} {lappend ::events $label}\n\
                  set ::events {}\n\
                  namespace eval ::a {variable x 1; trace add variable x write {record scalar-a}}\n\
                  namespace eval ::b {variable x 1; trace add variable x write {record scalar-b}}\n\
                  unset ::a::x\n\
                  set ::b::x 2",
        );
        // TP: deleting `::a::x` removes only its own trace, not the
        // same-bare-name trace attached to `::b::x`.
        assert_eq!(ok(i, b"set ::events"), b"scalar-b");

        ok(
                i,
                b"set ::events {}\n\
                  namespace eval ::c {variable x; set x(k) 1; trace add variable x(k) write {record element-c}}\n\
                  namespace eval ::d {variable x; set x(k) 1; trace add variable x(k) write {record element-d}}\n\
                  unset ::c::x(k)\n\
                  set ::d::x(k) 2",
            );
        // TP: element removal has the same home-namespace identity check.
        assert_eq!(ok(i, b"set ::events"), b"element-d");
    });
}

#[test]
fn unset_keeps_a_callers_local_trace_when_a_callee_uses_the_same_name() {
    leak_free(|i| {
        ok(
                i,
                b"proc record {label args} {lappend ::events $label}\n\
                  proc inner {} {set x inner; trace add variable x write {record inner}; unset x}\n\
                  proc outer {} {set x outer; trace add variable x write {record outer}; inner; set x after}\n\
                  set ::events {}\n\
                  outer",
            );
        // TN: a proc-local trace is scoped by its call frame, so the inner
        // unset cannot delete the caller's trace for its own `x`.
        assert_eq!(ok(i, b"set ::events"), b"outer");
    });
}

/// Every trace list is prepended in C (`TraceVarEx`, tclTrace.c
/// 9.0.4:3090-3092) and walked head→tail, so the newest registration fires
/// first for `read`/`write`/`unset` alike. Pinned against
/// tclsh 8.6.16 and 9.0.4.
#[test]
fn variable_traces_fire_newest_first() {
    for op in [&b"write"[..], b"read", b"unset"] {
        leak_free(|i| {
            ok(i, b"set ::log {}");
            ok(i, b"proc rec {label args} {lappend ::log $label}");
            let mut add = b"trace add variable v ".to_vec();
            add.extend_from_slice(op);
            for label in [&b" {rec one}"[..], b" {rec two}", b" {rec three}"] {
                let mut line = add.clone();
                line.extend_from_slice(label);
                ok(i, &line);
            }
            ok(i, b"set v x");
            ok(i, b"set ignore $v");
            ok(i, b"unset v");
            assert_eq!(ok(i, b"set ::log"), b"three two one");
            i.eval_str(b"unset -nocomplain ignore ::log");
        });
    }
}

/// C walks the containing array's trace list before the element's own
/// (`TclCallVarTraces`' `arrayPtr` loop precedes its `varPtr` loop), so
/// registration order does not decide which fires first.
#[test]
fn whole_array_traces_fire_before_element_traces() {
    for script in [
        &b"trace add variable a write {rec W}\ntrace add variable a(k) write {rec E}"[..],
        b"trace add variable a(k) write {rec E}\ntrace add variable a write {rec W}",
    ] {
        leak_free(|i| {
            ok(i, b"set ::log {}");
            ok(i, b"proc rec {label args} {lappend ::log $label}");
            ok(i, b"array set a {}");
            ok(i, script);
            ok(i, b"set a(k) 1");
            assert_eq!(ok(i, b"set ::log"), b"W E");
            i.eval_str(b"unset -nocomplain a ::log");
        });
    }
}

/// `Tcl_TraceCommand` prepends and `CallCommandTraces` walks head→tail, so
/// `rename`/`delete` callbacks also run newest-first.
#[test]
fn command_traces_fire_newest_first() {
    leak_free(|i| {
        ok(i, b"set ::log {}");
        ok(
            i,
            b"proc rec {label args} {lappend ::log $label-[lindex $args end]}",
        );
        ok(i, b"proc victim {} {}");
        ok(i, b"trace add command victim {rename delete} {rec one}");
        ok(i, b"trace add command victim {rename delete} {rec two}");
        ok(i, b"rename victim victim2");
        ok(i, b"rename victim2 {}");
        assert_eq!(
            ok(i, b"set ::log"),
            b"two-rename one-rename two-delete one-delete"
        );
        i.eval_str(b"unset -nocomplain ::log");
    });
}

/// `trace remove` breaks at the first match walking C's list head→tail, and
/// that head is the newest registration — so among identical duplicates the
/// **newest** goes. Observable in the surviving firing order and in
/// `trace info`. Pinned against tclsh 8.6.16 and 9.0.4.
#[test]
fn trace_remove_drops_the_newest_duplicate() {
    leak_free(|i| {
        ok(i, b"set ::log {}");
        ok(i, b"proc rec {label args} {lappend ::log $label}");
        ok(i, b"trace add variable v write {rec one}");
        ok(i, b"trace add variable v write {rec two}");
        ok(i, b"trace add variable v write {rec one}");
        ok(i, b"trace remove variable v write {rec one}");
        assert_eq!(
            ok(i, b"trace info variable v"),
            b"{write {rec two}} {write {rec one}}"
        );
        ok(i, b"set v 1");
        assert_eq!(ok(i, b"set ::log"), b"two one");
        i.eval_str(b"unset -nocomplain v ::log");
    });
    leak_free(|i| {
        ok(i, b"proc cb1 args {}");
        ok(i, b"proc cb2 args {}");
        ok(i, b"proc p {} {}");
        ok(i, b"trace add command p delete cb1");
        ok(i, b"trace add command p delete cb2");
        ok(i, b"trace add command p delete cb1");
        ok(i, b"trace remove command p delete cb1");
        assert_eq!(ok(i, b"trace info command p"), b"{delete cb2} {delete cb1}");
    });
}

/// Namespace teardown fires each variable's unset traces newest-first, like
/// every other trace list (the order *across* the namespace's variables is
/// C's hash walk and is not pinned).
#[test]
fn namespace_teardown_fires_unset_traces_newest_first() {
    leak_free(|i| {
        ok(i, b"set ::log {}");
        ok(i, b"proc rec {label args} {lappend ::log $label}");
        ok(
            i,
            b"namespace eval ::gone {variable v 1\n\
                  trace add variable v unset {rec first}\n\
                  trace add variable v unset {rec second}}",
        );
        ok(i, b"namespace delete ::gone");
        assert_eq!(ok(i, b"set ::log"), b"second first");
        i.eval_str(b"unset -nocomplain ::log");
    });
}

/// The commands the same teardown deletes fire their `delete` traces
/// newest-first too (`CallCommandTraces` walks head→tail).
/// tclsh 8.6.16 and 9.0.4 both report `second first`.
#[test]
fn a_delete_callbacks_own_trace_dies_with_the_dying_token() {
    // `Tcl_DeleteCommandFromToken` frees the whole `cmdPtr->tracePtr` list
    // after `CallCommandTraces`. A trace the callback registers on the
    // command being deleted attaches to that same dying token, so it never
    // fires — not in this walk (`CallCommandTraces` follows
    // `active.nextTracePtr`), and not for a later command that takes the
    // vacated name. Exact tclsh 9.0.4 oracle results (identical on
    // 8.6.16).
    let recorders: &[u8] = b"set ::log {}
             proc inner {old new op} {lappend ::log inner:$old}
             proc outer {old new op} {lappend ::log outer:$old
                 trace add command $old delete inner}";
    // `rename cmd {}`.
    leak_free(|i| {
        ok(i, recorders);
        ok(i, b"proc p {} {}");
        ok(i, b"trace add command ::p delete outer");
        ok(i, b"rename ::p {}");
        assert_eq!(ok(i, b"set ::log"), b"outer:::p");
        ok(i, b"proc p {} {}");
        ok(i, b"rename ::p {}");
        assert_eq!(ok(i, b"set ::log"), b"outer:::p");
        ok(i, b"unset ::log");
    });
    // Redefinition (`TclCreateObjCommandInNs` deletes the old token first).
    leak_free(|i| {
        ok(i, recorders);
        ok(i, b"proc p {} {}");
        ok(i, b"trace add command ::p delete outer");
        ok(i, b"proc p {} {}");
        assert_eq!(ok(i, b"set ::log"), b"outer:::p");
        ok(i, b"proc p {} {}");
        ok(i, b"rename ::p {}");
        assert_eq!(ok(i, b"set ::log"), b"outer:::p");
        ok(i, b"unset ::log");
    });
    // Namespace teardown.
    leak_free(|i| {
        ok(i, recorders);
        ok(i, b"namespace eval N {proc q {} {}}");
        ok(i, b"trace add command ::N::q delete outer");
        ok(i, b"namespace delete ::N");
        assert_eq!(ok(i, b"set ::log"), b"outer:::N::q");
        ok(i, b"namespace eval N {proc q {} {}}");
        ok(i, b"namespace delete ::N");
        assert_eq!(ok(i, b"set ::log"), b"outer:::N::q");
        ok(i, b"unset ::log");
    });
}

#[test]
fn a_replacements_trace_survives_the_deletion_that_created_it() {
    // The other half of the same rule: a callback that *binds* a
    // replacement at the vacated name registers on that new token, whose
    // own trace list C never touches. Only the dying token's list is
    // freed. Exact tclsh 9.0.4 oracle results (identical on 8.6.16).
    leak_free(|i| {
        ok(i, b"set ::log {}");
        ok(i, b"proc inner {old new op} {lappend ::log inner:$old}");
        ok(
            i,
            b"proc mk {old new op} {lappend ::log mk:$old
                      proc ::p {} {}
                      trace add command ::p delete inner}",
        );
        ok(i, b"proc p {} {}");
        ok(i, b"trace add command ::p delete mk");
        ok(i, b"rename ::p {}");
        assert_eq!(ok(i, b"set ::log"), b"mk:::p");
        assert_eq!(ok(i, b"llength [info commands ::p]"), b"1");
        ok(i, b"rename ::p {}");
        assert_eq!(ok(i, b"set ::log"), b"mk:::p inner:::p");
        assert_eq!(ok(i, b"llength [info commands ::p]"), b"0");
        ok(i, b"unset ::log");
    });
    // A trace the callback adds *before* the replacement still belongs to
    // the dying token and goes with it; only the one added after survives.
    leak_free(|i| {
        ok(i, b"set ::log {}");
        ok(i, b"proc early {old new op} {lappend ::log early:$old}");
        ok(i, b"proc late {old new op} {lappend ::log late:$old}");
        ok(
            i,
            b"proc mk {old new op} {lappend ::log mk:$old
                      trace add command $old delete early
                      proc ::p {} {}
                      trace add command ::p delete late}",
        );
        ok(i, b"proc p {} {}");
        ok(i, b"trace add command ::p delete mk");
        ok(i, b"rename ::p {}");
        assert_eq!(ok(i, b"set ::log"), b"mk:::p");
        ok(i, b"rename ::p {}");
        assert_eq!(ok(i, b"set ::log"), b"mk:::p late:::p");
        ok(i, b"unset ::log");
    });
}

#[test]
fn namespace_teardown_fires_command_delete_traces_newest_first() {
    leak_free(|i| {
        ok(i, b"set ::log {}");
        ok(i, b"proc rec {label args} {lappend ::log $label}");
        ok(i, b"namespace eval ::doomed {proc victim {} {}}");
        ok(i, b"trace add command ::doomed::victim delete {rec first}");
        ok(i, b"trace add command ::doomed::victim delete {rec second}");
        ok(i, b"namespace delete ::doomed");
        assert_eq!(ok(i, b"set ::log"), b"second first");
        i.eval_str(b"unset -nocomplain ::log");
    });
}

#[test]
fn parent_teardown_reaches_each_child_after_parent_commands() {
    // Tcl drains a parent's owned ensembles while both namespace tokens are
    // live, marks only that parent dying for its ordinary command traces,
    // and reaches each child ensemble through a separate recursive delete.
    // Exact Tcl 9.0.4 oracle result.
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"set seen {}
                      proc ensdeleted {tag old new op} {
                          lappend ::seen [list $tag [namespace exists ::P] \
                                               [namespace exists ::P::C]]
                      }
                      proc cmddeleted {old new op} {
                          lappend ::seen [list PC [namespace exists ::P] \
                                               [namespace exists ::P::C] \
                                               [info commands ::CE]]
                      }
                      namespace eval ::P {
                          namespace ensemble create -command ::PE
                          proc p {} {}
                          namespace eval C {
                              namespace ensemble create -command ::CE
                          }
                      }
                      trace add command ::PE delete [list ensdeleted P]
                      trace add command ::P::p delete cmddeleted
                      trace add command ::CE delete [list ensdeleted C]
                      namespace delete ::P
                      set seen"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"{P 1 1} {PC 0 1 ::CE} {C 0 1}");
    });
}

#[test]
fn namespace_teardown_does_not_resurrect_a_dying_namespace() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"set seen {}
                      proc cb {old new op} {
                          lappend ::seen [namespace exists ::N]
                          set c [catch {
                              namespace eval ::N {proc q {} {return Q}}
                          } m]
                          lappend ::seen $c $m [namespace exists ::N] \
                              [info commands ::N::q]
                      }
                      namespace eval N {proc p {} {return P}}
                      trace add command ::N::p delete cb
                      namespace delete ::N
                      list $seen [namespace exists ::N] [info commands ::N::q]"
            ),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"{0 1 {can't create namespace \"::N\": already exists} 0 {}} 0 {}"
        );
    });
}

#[test]
fn retained_dying_namespace_handle_is_not_publicly_live() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"set seen {}
                      proc cb {old new op} {
                          proc ::N::q {} {
                              set c [catch {namespace parent {}} m o]
                              list [namespace current] [namespace exists {}] \
                                  $c $m [dict get $o -errorcode]
                          }
                          set ::seen [::N::q]
                      }
                      namespace eval N {proc p {} {return P}}
                      trace add command ::N::p delete cb
                      namespace delete ::N
                      list $seen [namespace exists ::N] \
                          [info commands ::N::q]"
            ),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"{::N 0 1 {namespace \"\" not found in \"::N\"} \
                  {TCL LOOKUP NAMESPACE {}}} 0 {}"
        );
    });
}

#[test]
fn retained_namespace_handle_remains_dead_after_deletion_returns() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"namespace eval N {
                          proc p {} {
                              namespace delete ::N
                              set code [catch {namespace parent {}} message options]
                              list [namespace current] [namespace exists {}] \
                                  $code $message [dict get $options -errorcode]
                          }
                          p
                      }"
            ),
            Code::Ok
        );
        // Exact Tcl 9.0.4 oracle: the retained token still names `::N`,
        // but relative public lookup cannot treat it as a live namespace.
        assert_eq!(
            i.result_bytes(),
            b"::N 0 1 {namespace \"\" not found in \"::N\"} \
                  {TCL LOOKUP NAMESPACE {}}"
        );
    });
}

#[test]
fn recreated_namespace_does_not_revive_a_retained_dead_token() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"namespace eval N {
                          proc p {} {
                              namespace delete ::N
                              namespace eval ::N {}
                              set old [list [namespace current] \
                                  [namespace exists {}] [namespace exists ::N]]
                              set old_code [catch {namespace parent {}} \
                                  old_message old_options]
                              set new_code [catch {namespace parent ::N} new_message]
                              list $old $old_code $old_message \
                                  [dict get $old_options -errorcode] \
                                  $new_code $new_message
                          }
                          p
                      }"
            ),
            Code::Ok
        );
        // Exact Tcl 9.0.4 oracle: the recreated spelling is live through
        // absolute lookup, but the old activation retains its dead token.
        assert_eq!(
            i.result_bytes(),
            b"{::N 0 1} 1 {namespace \"\" not found in \"::N\"} \
                  {TCL LOOKUP NAMESPACE {}} 0 ::"
        );
    });
}

#[test]
fn namespace_teardown_sweeps_callback_created_commands_and_imports() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"set seen {}
                      proc qdel {old new op} {
                          set c [catch {$old} result]
                          lappend ::seen [list qdel $old $c $result]
                      }
                      proc cb {old new op} {
                          proc ::N::q {} {return Q}
                          trace add command ::N::q delete qdel
                          namespace eval ::I {namespace import ::N::q}
                          lappend ::seen [list callback [namespace exists ::N] \
                              [::N::q] [::I::q] [namespace origin ::I::q]]
                      }
                      namespace eval I {}
                      namespace eval N {
                          namespace export q
                          proc p {} {return P}
                      }
                      trace add command ::N::p delete cb
                      namespace delete ::N
                      set first $seen
                      set absent [list [namespace exists ::N] \
                          [info commands ::N::q] [info commands ::I::q] \
                          [namespace eval I {namespace import}]]
                      namespace eval N {proc q {} {return NEW}}
                      rename ::N::q {}
                      list $first $absent $seen [namespace exists ::N] \
                          [info commands ::I::q]"
            ),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"{{callback 0 Q Q ::N::q} {qdel ::N::q 0 Q}} \
                  {0 {} {} {}} \
                  {{callback 0 Q Q ::N::q} {qdel ::N::q 0 Q}} 1 {}"
        );
    });
}

#[test]
fn descendant_delete_callback_creates_a_fresh_visible_namespace_tree() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"proc cb {old new op} {
                          namespace eval ::N::C::X {
                              proc p {} {return P}
                          }
                      }
                      namespace eval ::N::C {
                          namespace export q
                          proc q {} {return Q}
                      }
                      namespace eval ::I {namespace import ::N::C::q}
                      trace add command ::N::C::q delete cb
                      namespace delete ::N
                      list [namespace exists ::N] \
                          [info commands ::N::C::X::p] [::N::C::X::p] \
                          [info commands ::I::q] \
                          [namespace eval I {namespace import}]"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"1 ::N::C::X::p P {} {}");
    });
}

/// C tears a namespace down one entity at a time, completing each
/// variable's whole trace list before the next, so **interleaved**
/// registrations still fire as contiguous per-variable groups. This is the
/// shape a flat reverse gets wrong: `A1 B1 A2 B2` must fire `A2 A1 B2 B1`,
/// not `B2 A2 B1 A1`. tclsh 8.6.16 and 9.0.4 agree.
#[test]
fn namespace_teardown_groups_interleaved_variable_traces() {
    leak_free(|i| {
        ok(i, b"set ::log {}");
        ok(i, b"proc rec {label args} {lappend ::log $label}");
        ok(i, b"namespace eval ::nsv {variable a 1\nvariable b 2}");
        ok(i, b"trace add variable ::nsv::a unset {rec A1}");
        ok(i, b"trace add variable ::nsv::b unset {rec B1}");
        ok(i, b"trace add variable ::nsv::a unset {rec A2}");
        ok(i, b"trace add variable ::nsv::b unset {rec B2}");
        ok(i, b"namespace delete ::nsv");
        assert_eq!(ok(i, b"set ::log"), b"A2 A1 B2 B1");
        i.eval_str(b"unset -nocomplain ::log");
    });
}

/// The same grouping for the command `delete` traces the teardown collects
/// alongside them: `X1 Y1 X2 Y2` fires `X2 X1 Y2 Y1`.
#[test]
fn namespace_teardown_groups_interleaved_command_traces() {
    leak_free(|i| {
        ok(i, b"set ::log {}");
        ok(i, b"proc rec {label args} {lappend ::log $label}");
        ok(i, b"namespace eval ::nsc {proc x {} {}\nproc y {} {}}");
        ok(i, b"trace add command ::nsc::x delete {rec X1}");
        ok(i, b"trace add command ::nsc::y delete {rec Y1}");
        ok(i, b"trace add command ::nsc::x delete {rec X2}");
        ok(i, b"trace add command ::nsc::y delete {rec Y2}");
        ok(i, b"namespace delete ::nsc");
        assert_eq!(ok(i, b"set ::log"), b"X2 X1 Y2 Y1");
        i.eval_str(b"unset -nocomplain ::log");
    });
}

/// Grouping holds when a teardown callback re-enters the interpreter — the
/// callbacks run after the namespace is already gone, so a callback that
/// reads or writes global state must not disturb the remaining groups.
#[test]
fn namespace_teardown_grouping_survives_a_re_entrant_callback() {
    leak_free(|i| {
        ok(i, b"set ::log {}");
        ok(i, b"set ::side 0");
        ok(
            i,
            b"proc rec {label args} {lappend ::log $label; incr ::side; \
                  catch {namespace exists ::nsr}}",
        );
        ok(i, b"namespace eval ::nsr {variable a 1\nvariable b 2}");
        ok(i, b"trace add variable ::nsr::a unset {rec A1}");
        ok(i, b"trace add variable ::nsr::b unset {rec B1}");
        ok(i, b"trace add variable ::nsr::a unset {rec A2}");
        ok(i, b"trace add variable ::nsr::b unset {rec B2}");
        ok(i, b"namespace delete ::nsr");
        assert_eq!(ok(i, b"set ::log"), b"A2 A1 B2 B1");
        assert_eq!(ok(i, b"set ::side"), b"4");
        i.eval_str(b"unset -nocomplain ::log ::side");
    });
}

/// `trace info` renders the stored op set in the order each C `TRACE_INFO`
/// arm tests the flag bits — `array read write unset` and `rename delete`,
/// neither of which is the `opStrings[]` table order the bad-operation
/// error enumerates.
#[test]
fn trace_info_renders_ops_in_cs_fixed_order() {
    leak_free(|i| {
        ok(i, b"proc cb args {}");
        ok(i, b"proc p {} {}");
        ok(i, b"trace add command p {delete rename} cb");
        ok(
            i,
            b"trace add execution p {leavestep leave enterstep enter} cb",
        );
        ok(i, b"trace add variable q {unset write read array} cb");
        assert_eq!(ok(i, b"trace info command p"), b"{{rename delete} cb}");
        assert_eq!(
            ok(i, b"trace info execution p"),
            b"{{enter leave enterstep leavestep} cb}"
        );
        assert_eq!(
            ok(i, b"trace info variable q"),
            b"{{array read write unset} cb}"
        );
        i.eval_str(b"unset -nocomplain q");
    });
}

/// The deprecated 8.x forms: `rwua`-only validation with C's
/// error text, duplicate letters collapsed, a set the modern `trace
/// remove variable` matches (and vice versa), `trace vinfo` rendering the
/// letters in C's fixed `r`,`w`,`u`,`a` order, and the callback receiving
/// the single letter rather than the operation name
/// (`TCL_TRACE_OLD_STYLE`). Every expectation is tclsh 8.6.16-pinned; the
/// 9.0 side lives in the `dialect_gate` test below.
#[test]
fn legacy_variable_trace_forms_match_c() {
    leak_free(|i| {
        i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        ok(i, b"set ::log {}");
        ok(i, b"proc cb args {lappend ::log [lindex $args end]}");
        ok(i, b"proc cb2 args {}");
        assert_eq!(
            err(i, b"trace variable x q cb"),
            b"bad operations \"q\": should be one or more of rwua"
        );
        assert_eq!(
            err(i, b"trace vdelete x {read write} cb"),
            b"bad operations \"read write\": should be one or more of rwua"
        );
        assert_eq!(
            err(i, b"trace variable x"),
            b"wrong # args: should be \"trace variable name ops command\""
        );
        assert_eq!(
            err(i, b"trace vdelete x"),
            b"wrong # args: should be \"trace vdelete name ops command\""
        );
        assert_eq!(
            err(i, b"trace vinfo x y"),
            b"wrong # args: should be \"trace vinfo name\""
        );

        // Repeated letters collapse, and `trace var` abbreviates.
        ok(i, b"trace variable x rrw cb");
        assert_eq!(ok(i, b"trace vinfo x"), b"{rw cb}");
        assert_eq!(ok(i, b"trace info variable x"), b"{{read write} cb}");
        ok(i, b"trace var x w cb2");
        assert_eq!(ok(i, b"trace vinfo x"), b"{w cb2} {rw cb}");

        // Letter order does not matter to the removal match, and the two
        // spellings remove each other's registrations.
        ok(i, b"trace vdelete x wr cb");
        assert_eq!(ok(i, b"trace vinfo x"), b"{w cb2}");
        ok(i, b"trace add variable y {write read} cb");
        ok(i, b"trace vdelete y rw cb");
        assert_eq!(ok(i, b"trace vinfo y"), b"");

        // An old-style callback is invoked with the `rwua` letter.
        ok(i, b"trace variable z rwua cb");
        assert_eq!(ok(i, b"trace vinfo z"), b"{rwua cb}");
        ok(i, b"set z 1");
        ok(i, b"set ignore $z");
        assert_eq!(ok(i, b"set ::log"), b"w r");
        // …while a modern registration still gets the full word.
        ok(i, b"set ::log {}");
        ok(i, b"trace add variable m write cb");
        ok(i, b"set m 1");
        assert_eq!(ok(i, b"set ::log"), b"write");
        i.eval_str(b"unset -nocomplain x y z m ignore ::log");
    });
}

/// Where the deprecated letter convention meets the teardown path: a trace
/// installed the deprecated way still receives the `rwua` **letter** when
/// it is fired by `namespace delete` rather than an explicit `unset`.
/// Teardown collects callbacks into a reduced list, so the flag has to be
/// carried through that reduction. tclsh 8.6.16 gives `L:u` for the legacy
/// registration, `M:unset` for a modern one, and — with both on one
/// variable, newest-first — `Mz:unset Lz:u`.
#[test]
fn legacy_letter_survives_namespace_teardown() {
    leak_free(|i| {
        i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        ok(i, b"set ::log {}");
        ok(i, b"proc rec {label n1 n2 op} {lappend ::log $label:$op}");

        ok(
            i,
            b"namespace eval ::legacy {variable x 1\ntrace variable x u {rec L}}",
        );
        ok(i, b"namespace delete ::legacy");
        assert_eq!(ok(i, b"set ::log"), b"L:u");

        ok(i, b"set ::log {}");
        ok(
            i,
            b"namespace eval ::modern {variable y 1\ntrace add variable y unset {rec M}}",
        );
        ok(i, b"namespace delete ::modern");
        assert_eq!(ok(i, b"set ::log"), b"M:unset");

        // Both conventions on one variable: each callback keeps its own.
        ok(i, b"set ::log {}");
        ok(
                i,
                b"namespace eval ::both {variable z 1\ntrace variable z u {rec Lz}\ntrace add variable z unset {rec Mz}}",
            );
        ok(i, b"namespace delete ::both");
        assert_eq!(ok(i, b"set ::log"), b"Mz:unset Lz:u");

        // An array element registered the legacy way behaves the same.
        ok(i, b"set ::log {}");
        ok(
            i,
            b"namespace eval ::arr {variable a\nset a(k) 1\ntrace variable a(k) u {rec A}}",
        );
        ok(i, b"namespace delete ::arr");
        assert_eq!(ok(i, b"set ::log"), b"A:u");
        i.eval_str(b"unset -nocomplain ::log");
    });
}

/// The 9.x side of the same seam: the legacy form does not exist there, so
/// a modern registration fired by teardown must still say `unset`.
#[test]
fn teardown_op_word_is_the_full_word_at_9x() {
    leak_free(|i| {
        i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
        ok(i, b"set ::log {}");
        ok(i, b"proc rec {label n1 n2 op} {lappend ::log $label:$op}");
        ok(
            i,
            b"namespace eval ::modern {variable y 1\ntrace add variable y unset {rec M}}",
        );
        ok(i, b"namespace delete ::modern");
        assert_eq!(ok(i, b"set ::log"), b"M:unset");
        assert_eq!(
            err(i, b"trace variable q u {rec L}"),
            b"bad option \"variable\": must be add, info, or remove"
        );
        i.eval_str(b"unset -nocomplain ::log");
    });
}

/// The registry retires the three legacy forms at 9.0, and the runtime
/// reads that rather than carrying its own list — so the same script is a
/// working trace at 8.x and `bad option` at 9.x, with the option
/// enumeration following too.
#[test]
fn legacy_variable_trace_forms_follow_the_release() {
    leak_free(|i| {
        i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        assert_eq!(
            err(i, b"trace zzz"),
            b"bad option \"zzz\": must be add, info, remove, variable, vdelete, or vinfo"
        );
        assert_eq!(
            err(i, b"trace v x"),
            b"ambiguous option \"v\": must be add, info, remove, variable, vdelete, or vinfo"
        );
    });
    leak_free(|i| {
        i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
        for word in [&b"variable"[..], b"vdelete", b"vinfo", b"var"] {
            let mut line = b"trace ".to_vec();
            line.extend_from_slice(word);
            line.extend_from_slice(b" x w cb");
            let mut want = b"bad option \"".to_vec();
            want.extend_from_slice(word);
            want.extend_from_slice(b"\": must be add, info, or remove");
            assert_eq!(err(i, &line), want);
        }
        assert_eq!(
            err(i, b"trace zzz"),
            b"bad option \"zzz\": must be add, info, or remove"
        );
    });
}

fn declared_trace_option(profile: &'static tcl_dialect::DialectProfile, form: &str) -> String {
    const TABLE: &[&[u8]] = &[
        b"add",
        b"info",
        b"remove",
        b"variable",
        b"vdelete",
        b"vinfo",
    ];
    let visible = crate::environment::release_subcommands(profile.name, "trace", TABLE);
    let names = visible
        .iter()
        .map(|name| std::str::from_utf8(name).unwrap())
        .collect::<Vec<_>>();
    match tcl_cmd_core::trace::resolve_option(form.split_whitespace().next().unwrap(), &names) {
        Ok(_) => "0:".to_owned(),
        Err(error) => format!("1:{}", error.message().unwrap()),
    }
}

/// The `trace` option table is the pinned profile's, not the plain release
/// its runtime version names: an iRules interpreter emulates 8.4 but the
/// TMM's Tcl has only the three legacy forms, so `trace add` — which
/// `tcl8.4` has — is a bad option there and `trace variable` is not. The
/// permissive fallback states no table of its own, so an interpreter pinned
/// to nothing keeps answering for the release it emulates, Tcl 9, which has
/// lost the legacy forms.
#[test]
fn the_trace_gate_reads_the_pinned_profile_not_the_release_name() {
    let probe = |profile: Option<&'static tcl_dialect::DialectProfile>, form: &str| {
        let mut answer = Vec::new();
        leak_free(|i| {
            if let Some(profile) = profile {
                i.set_dialect_profile(profile);
            }
            ok(
                i,
                format!("set r \"[catch {{trace {form}}} m]:$m\"").as_bytes(),
            );
            answer = i.result_bytes();
            if answer.starts_with(b"0:") {
                assert_eq!(i.traces.borrow().traces.len(), 1);
            }
        });
        String::from_utf8(answer).expect("utf-8")
    };
    let resolve = |name: &str| tcl_registry::model::resolve_environment(name).analyser_profile();
    let add = "add variable x write cb";
    let legacy = "variable x w cb";

    // The hosted profile declares a table, not a physical TMM trace/Index provider.
    // Keep declaration outcomes separate from the actual Tcl registration checks below.
    leak_free(|i| {
        i.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        assert!(i.name_policy_protocol().is_none());
        assert!(i
            .native_invocation_dialect()
            .native_index_lookup_protocol()
            .is_none());
        let _ = i.eval_str(b"trace variable x w cb");
        assert!(i.host_refusal_pending());
    });
    assert_eq!(
        declared_trace_option(tcl_dialect::DialectProfile::irules(), add),
        "1:bad option \"add\": must be variable, vdelete, or vinfo"
    );
    assert_eq!(
        declared_trace_option(tcl_dialect::DialectProfile::irules(), legacy),
        "0:"
    );
    assert_eq!(probe(Some(resolve("tcl8.4")), add), "0:");
    assert_eq!(probe(Some(resolve("tcl8.4")), legacy), "0:");
    for profile in [Some(resolve("tcl9.0")), None] {
        assert_eq!(probe(profile, add), "0:");
        assert_eq!(
            probe(profile, legacy),
            "1:bad option \"variable\": must be add, info, or remove"
        );
    }
}
