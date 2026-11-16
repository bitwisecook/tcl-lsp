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

//! A running coroutine or traced command that hides, exposes, renames or deletes itself, checked against real `tclsh` when it is available.

use super::*;

const SELF_HIDING_COROUTINE: &str = r"
set first [coroutine c apply {{} {
    interp hide {} [info coroutine] held
    yield first
    interp expose {} held d
    yield second
    return done
}}]
puts [list $first [interp invokehidden {} held] [d] [interp hidden {}]]
";

// A deleted coroutine binding and a later command at the same spelling are
// distinct lifecycles.  The original driver is still unwinding after it
// deletes `c`; the replacement is then created at `c` and renamed to `d`.
// Its completion must not retire `d` through the stale active sidecar handle.
const SELF_DELETING_COROUTINE_REUSES_AND_RENAMES_BINDING: &str = r"
proc fresh {} {yield fresh-one; yield fresh-two; return fresh-done}
proc stale {} {
    rename [info coroutine] {}
    coroutine c fresh
    rename c d
    return stale-done
}
set start [coroutine c stale]
puts [list $start [d] [d] [catch {d} message] $message]
";

const SELF_HIDING_TRACED_COMMAND: &str = r"
set log {}
proc tr args {lappend ::log $args}
proc p {} {interp hide {} p held; return ok}
trace add execution p {enter leave} tr
set result [p]
puts [list $result $log [interp hidden {}]]
";

const SELF_EXPOSING_TRACED_COMMAND: &str = r"
set log {}
proc tr args {lappend ::log $args}
proc p {} {interp expose {} held q; return ok}
trace add execution p {enter leave} tr
interp hide {} p held
set result [interp invokehidden {} held]
puts [list $result $log [info commands q] [interp hidden {}]]
";

const SELF_RENAMING_TRACED_COMMAND: &str = r"
set log {}
proc tr args {lappend ::log $args}
proc p {} {rename p q; return ok}
trace add execution p {enter leave} tr
set result [p]
puts [list $result $log [info commands q]]
";

const SELF_HIDING_TRACED_ERROR: &str = r"
set log {}
proc tr args {lappend ::log $args}
proc p {} {interp hide {} p held; error boom}
trace add execution p {enter leave} tr
set code [catch {p} result]
puts [list $code $result $log [interp hidden {}]]
";

// A leave callback can delete the running binding and install a replacement.
// Tcl stops the cloned old trace list at that lifecycle boundary: later old
// callbacks must not run against the replacement generation.
const LEAVE_TRACE_DELETE_RECREATE_STOPS_OLD_LIST: &str = r"
set log {}
proc a {} {return old}
proc first args {lappend ::log A; rename a {}; proc a {} {return new}}
proc second args {lappend ::log B}
trace add execution a leave first
trace add execution a leave second
set old [a]
set new [a]
puts [list $old $log $new]
";

// Tcl runs enter callbacks newest-first, then leave callbacks oldest-first.
// Each successful leave result feeds the next callback, but never replaces
// the traced command's completion.
const EXECUTION_TRACE_ORDER_AND_LEAVE_RESULT_CHAIN: &str = r"
set log {}
proc p {} {return ORIG}
proc a args {lappend ::log [list A {*}$args]; return R1}
proc b args {lappend ::log [list B {*}$args]; return R2}
trace add execution p {enter leave} a
trace add execution p {enter leave} b
set result [p]
puts [list $result $log]
";

// Removing one pending registration skips only that entry; later entries in
// the captured Tcl trace list still fire.
const EXECUTION_TRACE_REMOVAL_CONTINUES_SNAPSHOT: &str = r"
set log {}
proc p {} {return ok}
proc a args {lappend ::log A}
proc b args {lappend ::log B}
proc c args {lappend ::log C; trace remove execution p enter b}
trace add execution p enter a
trace add execution p enter b
trace add execution p enter c
p
puts $log
";

#[test]
fn active_coroutine_and_execution_trace_follow_self_hide() {
    for profile in [
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    ] {
        assert_eq!(
            profile_output(SELF_HIDING_COROUTINE, profile),
            "first second done {}"
        );
        assert_eq!(
            profile_output(SELF_DELETING_COROUTINE_REUSES_AND_RENAMES_BINDING, profile),
            "stale-done fresh-two fresh-done 1 {invalid command name \"d\"}"
        );
        assert_eq!(
            profile_output(SELF_HIDING_TRACED_COMMAND, profile),
            "ok {{p enter} {p 0 ok leave}} held"
        );
        assert_eq!(
            profile_output(SELF_EXPOSING_TRACED_COMMAND, profile),
            "ok {{held enter} {held 0 ok leave}} q {}"
        );
        assert_eq!(
            profile_output(SELF_RENAMING_TRACED_COMMAND, profile),
            "ok {{p enter} {p 0 ok leave}} q"
        );
        assert_eq!(
            profile_output(SELF_HIDING_TRACED_ERROR, profile),
            "1 boom {{p enter} {p 1 boom leave}} held"
        );
        assert_eq!(
            profile_output(LEAVE_TRACE_DELETE_RECREATE_STOPS_OLD_LIST, profile),
            "old A new"
        );
        assert_eq!(
            profile_output(EXECUTION_TRACE_ORDER_AND_LEAVE_RESULT_CHAIN, profile),
            "ORIG {{B p enter} {A p enter} {A p 0 ORIG leave} {B p 0 R1 leave}}"
        );
        assert_eq!(
            profile_output(EXECUTION_TRACE_REMOVAL_CONTINUES_SNAPSHOT, profile),
            "C A"
        );
    }
}

#[test]
fn self_hide_vectors_match_real_tcl_when_available() {
    for (env, names) in [
        ("TCLSH86", &["tclsh8.6", "tclsh"][..]),
        ("TCLSH90", &["tclsh9.0"][..]),
    ] {
        for (script, expected) in [
            (SELF_HIDING_COROUTINE, "first second done {}"),
            (
                SELF_DELETING_COROUTINE_REUSES_AND_RENAMES_BINDING,
                "stale-done fresh-two fresh-done 1 {invalid command name \"d\"}",
            ),
            (
                SELF_HIDING_TRACED_COMMAND,
                "ok {{p enter} {p 0 ok leave}} held",
            ),
            (
                SELF_EXPOSING_TRACED_COMMAND,
                "ok {{held enter} {held 0 ok leave}} q {}",
            ),
            (
                SELF_RENAMING_TRACED_COMMAND,
                "ok {{p enter} {p 0 ok leave}} q",
            ),
            (
                SELF_HIDING_TRACED_ERROR,
                "1 boom {{p enter} {p 1 boom leave}} held",
            ),
            (LEAVE_TRACE_DELETE_RECREATE_STOPS_OLD_LIST, "old A new"),
            (
                EXECUTION_TRACE_ORDER_AND_LEAVE_RESULT_CHAIN,
                "ORIG {{B p enter} {A p enter} {A p 0 ORIG leave} {B p 0 R1 leave}}",
            ),
            (EXECUTION_TRACE_REMOVAL_CONTINUES_SNAPSHOT, "C A"),
        ] {
            if let Some(actual) = tclsh_output(env, names, script) {
                assert_eq!(actual, expected, "{env}");
            }
        }
    }
}

/// An enter trace may delete the command currently being invoked while also
/// installing a replacement under its former visible name.  Tcl treats the
/// in-flight command token as deleted; it neither invokes nor resurrects it,
/// and the callback-created replacement remains authoritative.
#[test]
fn deleted_hidden_enter_trace_matches_real_tclsh() {
    let src = concat!(
        "proc p {} {return hidden}\n",
        "proc replace args {\n",
        "  rename p {}\n",
        "  interp expose {} held p\n",
        "  rename p {}\n",
        "  proc p {} {return replacement}\n",
        "}\n",
        "trace add execution p enter replace\n",
        "interp hide {} p held\n",
        "proc p {} {return initial}\n",
        "set rc [catch {interp invokehidden {} held} msg]\n",
        "puts [list $rc $msg [p] [interp hidden {}]]\n",
    );
    let want = "1 {attempt to invoke a deleted command} replacement {}";
    for (env, names) in [
        ("TCLSH86", &["tclsh8.6"][..]),
        ("TCLSH90", &["tclsh9.0"][..]),
    ] {
        if let Some(out) = tclsh_output(env, names, src) {
            assert_eq!(out, want, "[{env}] hidden enter-trace lifecycle");
        }
    }
}
