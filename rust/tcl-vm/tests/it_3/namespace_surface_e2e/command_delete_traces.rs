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

//! Command delete and rename traces during teardown: nested traces, errors, and callbacks that delete or recreate child interpreters.

use super::*;

#[test]
fn nested_command_delete_traces_preserve_deferred_result() {
    // Tcl 9.0.4: each nested trace chain restores its caller's result.
    assert_eq!(
        run(r"set log {}
              proc inner {args} {lappend ::log inner; return inner-result}
              proc outer {args} {rename target2 {}; lappend ::log outer; return outer-result}
              proc target1 {} {}; proc target2 {} {}
              trace add command target1 delete outer
              trace add command target2 delete inner
              namespace eval N {proc p {} {namespace delete ::N; return keep}}
              trace add command ::N::p delete {apply {{args} {rename ::target1 {}}}}
              set result [::N::p]
              list $result $log"),
        "keep {inner outer}"
    );
}

#[test]
fn command_delete_trace_error_preserves_result_and_continues_chain() {
    // Tcl 9.0.4 ignores callback errors and restores the pending result.
    assert_eq!(
        run(r"set log {}
              proc cb {name args} {
                  lappend ::log $name
                  if {$name eq {first}} {error boom}
                  return ignored
              }
              namespace eval N {proc p {} {namespace delete ::N; return keep}}
              trace add command ::N::p delete [list cb second]
              trace add command ::N::p delete [list cb first]
              list [::N::p] $log"),
        "keep {first second}"
    );
}

#[test]
fn command_trace_unmatched_operation_does_not_fire_callback() {
    assert_eq!(
        run(r"set log {}
              proc cb {args} {lappend ::log fired}
              namespace eval N {proc p {} {namespace delete ::N; return keep}}
              trace add command ::N::p rename cb
              list [::N::p] $log"),
        "keep {}"
    );
}

#[test]
fn command_trace_error_preserves_pending_error_code() {
    // Tcl 9.0.4 preserves the pending error across ignored callback errors.
    assert_eq!(
        run(r"set log {}
              proc cb {name args} {
                  lappend ::log $name
                  if {$name eq {first}} {error boom}
                  return ignored
              }
              namespace eval N {
                  proc p {} {
                      namespace delete ::N
                      lappend ::log before_return
                      return -code error -errorcode {ORIGINAL CODE} original
                  }
              }
              trace add command ::N::p delete [list cb second]
              trace add command ::N::p delete [list cb first]
              set status [catch {::N::p} result opts]
              list $status $result [dict get $opts -errorcode] $log"),
        "1 original {ORIGINAL CODE} {before_return first second}"
    );
}

#[test]
fn command_trace_retains_active_interpreter_during_callback_deletion() {
    // Tcl 9.0.4 keeps the active child alive until its evaluation unwinds.
    assert_eq!(
        run(r"interp create i
              proc retire args {interp delete i}
              interp alias i retire {} retire
              set status [catch {
                  interp eval i {proc victim {} {}; trace add command victim delete retire; rename victim {}}
              } message options]
              list $status $message [interp exists i] [dict get $options -code]"),
        "0 {} 0 0"
    );
}

#[test]
fn command_trace_alias_sweep_rejects_deleted_interpreter_reentry() {
    // Tcl 9.0.4: the alias trace sees a deleted target and unlinked child.
    assert_eq!(
        run(r"interp create i
              interp eval i {set x old}
              interp alias {} a i set x
              set log {}
              proc cb args {
                  lappend ::log [interp exists i]
                  lappend ::log [catch {a} m] $m
                  lappend ::log [catch {interp eval i {set x}} n] $n
              }
              trace add command a delete cb
              set status [catch {interp delete i} message]
              list $status $message $log [info commands a]"),
        r#"0 {} {0 1 {attempt to call eval in deleted interpreter} 1 {could not find interpreter "i"}} {}"#
    );
}

#[test]
fn command_trace_deleted_child_recreation_keeps_generations_distinct() {
    // Tcl 9.0.4: the old active child finishes while its replacement stays live.
    assert_eq!(
        run(r"interp create i
              proc retire args {
                  interp delete i
                  set ::after [catch {interp eval i {set x}} msg]
                  interp create i
                  interp eval i {set marker new}
              }
              interp alias i retire {} retire
              set status [catch {
                  interp eval i {proc victim {} {}; trace add command victim delete retire; rename victim {}}
              } message options]
              list $status $message $::after [interp eval i {set marker}] [dict get $options -code]"),
        "0 {} 1 new 0"
    );
}

#[test]
fn command_trace_nested_child_delete_preserves_name_until_callback_returns() {
    // Tcl 9.0.4: nested deletion succeeds but cannot recreate the name yet.
    assert_eq!(
        run(r"interp create i
              set log {}
              proc cb args {
                  lappend ::log [interp exists i] [info commands i]
                  lappend ::log [catch {interp delete i} m] $m
                  lappend ::log [interp exists i] [info commands i]
                  lappend ::log [catch {interp create i} m] $m
              }
              trace add command i delete cb
              set status [catch {interp delete i} message]
              list $status $message $log [interp exists i] [info commands i]"),
        r#"0 {} {1 i 0 {} 1 {} 1 {interpreter named "i" already exists, cannot create}} 0 {}"#
    );
}

#[test]
fn command_trace_rename_callback_can_delete_child_interpreter() {
    // Tcl 9.0.4: a rename trace does not suppress the nested deletion.
    assert_eq!(
        run(r"interp create i
              set log {}
              proc cb args {
                  lappend ::log [catch {interp delete i} m] $m
                  lappend ::log [interp exists i]
              }
              trace add command i rename cb
              set status [catch {rename i j} message]
              list $status $message $log [interp exists i] [info commands j]"),
        "0 {} {0 {} 0} 0 {}"
    );
}

#[test]
fn command_trace_repeated_nested_child_delete_keeps_original_token() {
    // Tcl 9.0.4: the second nested delete still addresses the unlinked token.
    for (outer, expected) in [
        ("rename i {}", "0 {} {1 i 0 {} 1 {} 0 {} 1 {}} 0 {}"),
        (
            "proc i {} {return replacement}",
            "0 {} {1 i 0 {} 1 {} 0 {} 1 {}} 0 replacement",
        ),
    ] {
        let script = format!(
            r"interp create i
              set log {{}}
              proc cb args {{
                  lappend ::log [interp exists i] [info commands i]
                  lappend ::log [catch {{interp delete i}} m] $m [interp exists i] [info commands i]
                  lappend ::log [catch {{interp delete i}} n] $n [interp exists i] [info commands i]
              }}
              trace add command i delete cb
              set status [catch {{{outer}}} message]
              list $status $message $log [interp exists i] {}",
            if outer.starts_with("rename") {
                "[info commands i]"
            } else {
                "[i]"
            },
        );
        assert_eq!(run(&script), expected, "outer operation: {outer}");
    }
}

#[test]
fn command_trace_replacement_repeated_nested_child_delete_keeps_original_token() {
    // Tcl 9.0.4: replacing the child command retains its deleting token.
    assert_eq!(
        run(r"interp create i
              set log {}
              proc cb args {
                  lappend ::log [interp exists i] [info commands i]
                  lappend ::log [catch {interp delete i} m] $m [interp exists i] [info commands i]
                  lappend ::log [catch {interp delete i} n] $n [interp exists i] [info commands i]
              }
              trace add command i delete cb
              set status [catch {proc i {} {return replacement}} message]
              list $status $message $log [interp exists i] [i]"),
        "0 {} {1 i 0 {} 1 {} 0 {} 1 {}} 0 replacement"
    );
}
