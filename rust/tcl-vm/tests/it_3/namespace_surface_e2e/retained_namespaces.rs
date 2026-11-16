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

//! A deleted namespace outlives its last active frame: deferred deletion, dead handles, recreation as a distinct token, and deferred teardown order.

use super::*;

#[test]
fn dying_namespace_handles_are_rejected_during_command_delete_traces() {
    // Tcl marks an ordinary namespace token dead before retiring member
    // commands. Parent/children must therefore use the same lifecycle-aware
    // handle lookup as exists, while the command token remains observable.
    // Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set seen {}
             proc deleted {old new op} {
                 set pc [catch {namespace parent ::N} pm po]
                 set cc [catch {namespace children ::N} cm co]
                 lappend ::seen [list [namespace exists ::N] \
                     [info commands $old] $pc $pm [dict get $po -errorcode] \
                     $cc $cm [dict get $co -errorcode]]
             }
             namespace eval N {proc p {} {}}
             trace add command ::N::p delete deleted
             namespace delete ::N
             set seen"),
        "{0 ::N::p 1 {namespace \"::N\" not found} {TCL LOOKUP NAMESPACE ::N} 1 \
         {namespace \"::N\" not found} {TCL LOOKUP NAMESPACE ::N}}"
    );
}

#[test]
fn retained_namespace_handle_remains_dead_after_deletion_returns() {
    // Deletion unlinks the namespace name but an active proc retains the old
    // token. Its display name survives; name-based lookup through that token
    // must remain dead after teardown has finished. Exact Tcl 9.0.4 oracle.
    assert_eq!(
        run("namespace eval N {
                 proc p {} {
                     namespace delete ::N
                     set code [catch {namespace parent {}} message options]
                     list [namespace current] [namespace exists {}] $code \
                          $message [dict get $options -errorcode]
                 }
                 p
             }"),
        "::N 0 1 {namespace \"\" not found in \"::N\"} {TCL LOOKUP NAMESPACE {}}"
    );
}

#[test]
fn recreated_namespace_does_not_revive_a_retained_dead_token() {
    // A recreated spelling is a fresh namespace token. The old activation
    // continues to name (and reject lookup through) its deleted identity while
    // absolute lookup reaches the new live token. Exact Tcl 9.0.4 oracle.
    assert_eq!(
        run("namespace eval N {
                 proc p {} {
                     namespace delete ::N
                     namespace eval ::N {}
                     set old [list [namespace current] [namespace exists {}] \
                                  [namespace exists ::N]]
                     set old_code [catch {namespace parent {}} old_message old_options]
                     set new_code [catch {namespace parent ::N} new_message]
                     list $old $old_code $old_message \
                          [dict get $old_options -errorcode] $new_code $new_message
                 }
                 p
             }"),
        "{::N 0 1} 1 {namespace \"\" not found in \"::N\"} \
         {TCL LOOKUP NAMESPACE {}} 0 ::"
    );
}

// A deleted namespace outlives its last active frame.

#[test]
fn deleting_the_running_namespace_retains_it_for_the_frame() {
    // `Tcl_DeleteNamespace` takes its
    // `activationCount > (nsPtr == globalNsPtr)` branch: `NS_DYING`, unlink the
    // parent edge, keep the contents for the frames still running in the token.
    // Exact Tcl 9.0.4 oracle result (identical on 8.6.16).
    assert_eq!(
        run(r"set answer [namespace eval N {
                 proc q {} {return Q}
                 proc p {} {
                     namespace delete ::N
                     set code [catch {q} message]
                     list [namespace exists ::N] [info commands ::N::p] \
                          [info commands ::N::q] $code $message [namespace current]
                 }
                 p
             }]
             list $answer [namespace exists ::N] [info commands ::N::*]"),
        r"{0 {} {} 0 Q ::N} 0 {}"
    );
}

#[test]
fn a_namespace_eval_frame_defers_its_own_deletion() {
    // `Tcl_PushCallFrame` counts every frame, `namespace eval`/`inscope` and
    // `apply -ns` included. Exact Tcl 9.0.4 oracle results (identical on
    // 8.6.16).
    assert_eq!(
        run(r"set r [namespace eval N {
                 proc q {} {return Q}
                 namespace delete ::N
                 list [namespace exists ::N] [catch {q} m] $m [namespace current] \
                      [info commands ::N::*]
             }]
             list $r [namespace exists ::N]"),
        r"{0 0 Q ::N {}} 0"
    );
}

#[test]
fn inscope_and_apply_frames_defer_their_own_deletion() {
    // `namespace inscope` and `apply` push the same kind of activation. Exact
    // Tcl 9.0.4 oracle results (identical on 8.6.16).
    assert_eq!(
        run(r"namespace eval N {proc q {} {return Q}}
             set a [namespace inscope N {
                 namespace delete ::N
                 list [q] [namespace current] [namespace exists ::N]
             }]
             list $a [namespace exists ::N]"),
        r"{Q ::N 0} 0"
    );
    assert_eq!(
        run(r"set log {}
             proc rec {old new op} {lappend ::log [list $old $op]}
             namespace eval N {proc q {} {return Q}}
             trace add command ::N::q delete rec
             set a [apply [list {} {namespace delete ::N; list [q] [llength $::log]} ::N]]
             list $a $log"),
        "{Q 0} {{::N::q delete}}"
    );
}

#[test]
fn a_recreated_namespace_is_a_distinct_token_from_the_retained_one() {
    // C nulls `parentPtr` in the deferred branch, so the spelling is free at
    // once: `namespace eval ::N` builds a wholly separate token with its own
    // command table, which the retained one neither revives nor absorbs. The
    // relative `r` is *not* visible from the retained frame — its own table has
    // no such command and the global one has none either. Exact Tcl 9.0.4
    // oracle result (identical on 8.6.16).
    assert_eq!(
        run(r"namespace eval N {
                 proc q {} {return Q}
                 proc p {} {
                     namespace delete ::N
                     namespace eval ::N {proc r {} {return R}}
                     list [q] [catch {r} m] $m [info commands ::N::*] [::N::r] \
                          [namespace current] [namespace exists ::N] \
                          [namespace which q] [namespace which r] [info commands q]
                 }
             }
             set a [::N::p]
             list $a [info commands ::N::*] [namespace exists ::N]"),
        r#"{Q 1 {invalid command name "r"} ::N::r R ::N 1 ::N::q {} q} ::N::r 1"#
    );
}

#[test]
fn a_retained_token_takes_relative_definitions_only() {
    // A qualified definition cannot reach the retained token (its public name
    // is gone — `TclGetNamespaceForQualName` finds no namespace); an
    // unqualified one lands in it through the frame, invisible to every
    // absolute probe. Exact Tcl 9.0.4 oracle result (identical on 8.6.16).
    assert_eq!(
        run(r"namespace eval N {
                 proc p {} {
                     namespace delete ::N
                     proc r {} {return R}
                     list [r] [info commands ::N::r] [namespace which r] \
                          [catch {proc ::N::s {} {}} m] $m [info commands ::N::*] \
                          [namespace exists ::N]
                 }
             }
             set a [::N::p]
             list $a [info commands ::N::*] [namespace exists ::N]"),
        r#"{R {} ::N::r 1 {can't create procedure "::N::s": unknown namespace} {} 0} {} 0"#
    );
}

#[test]
fn an_absolute_proc_definition_targets_the_live_recreation() {
    assert_eq!(
        run(r"namespace eval N {
                 proc p {} {
                     namespace delete ::N
                     namespace eval ::N {}
                     proc ::N::fresh {} {return [namespace current]}
                     list [::N::fresh] [info commands fresh]
                 }
             }
             ::N::p"),
        "::N {}"
    );
}

#[test]
fn a_retained_token_keeps_its_children() {
    // The child stays reachable relatively and by no absolute name at all. The
    // absolute name is built at run time on purpose: a literal would measure
    // tclsh's `ResolvedCmdName` bytecode cache, which only invalidates when the
    // command's *own* namespace is `NS_DYING`. Exact Tcl 9.0.4 oracle result
    // (identical on 8.6.16).
    assert_eq!(
        run(r"namespace eval N {
                 namespace eval C {proc x {} {return X}}
                 proc p {} {
                     namespace delete ::N
                     set n [join [list {} N C x] ::]
                     list [C::x] [namespace exists ::N::C] [catch {$n} m] $m \
                          [namespace children] [catch {namespace children ::N} m2] $m2
                 }
             }
             set a [::N::p]
             list $a [namespace exists ::N::C]"),
        r#"{X 0 1 {invalid command name "::N::C::x"} ::N::C 1 {namespace "::N" not found}} 0"#
    );
}

#[test]
fn a_retained_token_keeps_its_variables() {
    // The token's variables ride with it; every name-addressed probe fails
    // because the name is gone. Exact Tcl 9.0.4 oracle result (identical on
    // 8.6.16). The relative link still addresses the retained token's exact
    // variable cell even though neither absolute glob can reach it.
    assert_eq!(
        run(r"namespace eval N {
                 variable v V
                 proc p {} {
                     namespace delete ::N
                     variable v
                     list [set v] [info vars ::N::*] [info vars [namespace current]::*]
                 }
             }
             ::N::p"),
        r"V {} {}"
    );
}

#[test]
fn retained_and_recreated_namespace_variables_have_distinct_cells() {
    // Exact Tcl 9.0.4 oracle. This pins the variable-table half of
    // stable namespace identity; command-token retirement has separate
    // coverage.
    assert_eq!(
        run(r"set answer [namespace eval N {
                 variable v OLD
                 proc q {} {return OLDQ}
                 proc p {} {
                     namespace delete ::N
                     namespace eval ::N {variable v NEW; proc q {} {return NEWQ}}
                     variable v
                     list [q] [::N::q] $v [set v OLD2] [set ::N::v] \
                          [info commands q] [info commands ::N::q] \
                          [namespace current] [namespace exists {}] [namespace exists ::N]
                 }
                 p
             }]
             list $answer [set ::N::v] [::N::q]"),
        "{OLDQ NEWQ OLD OLD2 NEW q ::N::q ::N 0 1} NEW NEWQ"
    );
}

#[test]
fn colon_edge_command_tokens_stay_distinct() {
    // #1778. `a:`/`p`, `a`/`:p` and `a` -> `:` -> `p` are three distinct
    // commands whose rendered names collide. Measured on tclsh 8.4.20,
    // 8.5.19, 8.6.18, 9.0.4 and 9.1.0: before 8.6 a non-global `:p` cannot be
    // created; from 8.6 each relative spelling reaches its own token, the
    // absolute `::a:::::p` resolves to none of them, and deleting `a:` leaves
    // the other two.
    let src = r#"set r {}
foreach s {
    {namespace eval a: {proc p {} {return from-a-colon}}}
    {namespace eval a {proc :p {} {return from-colon-p}}}
    {namespace eval a {namespace eval : {proc p {} {return nested}}}}
    {namespace eval a: {p}}
    {namespace eval a {:p}}
    {namespace eval a {namespace eval : {p}}}
    {::a:::::p}
    {namespace delete a:}
    {namespace eval a {:p}}
    {namespace eval a {namespace eval : {p}}}
} {
    lappend r [list [catch $s m] [expr {[string match {*invalid command*} $m] ? "invalid" : $m}]]
}
set r"#;
    let legacy = r#"{0 {}} {1 {can't create procedure ":p" in non-global namespace with name starting with ":"}} {0 {}} {1 invalid} {1 invalid} {1 invalid} {0 nested} {0 {}} {1 invalid} {1 invalid}"#;
    let modern = r"{0 {}} {0 {}} {0 {}} {0 from-a-colon} {0 from-colon-p} {0 nested} {1 invalid} {0 {}} {0 from-colon-p} {0 nested}";
    for (release, expected) in [
        ("tcl8.4", legacy),
        ("tcl8.5", legacy),
        ("tcl8.6", modern),
        ("tcl9.0", modern),
        ("tcl9.1", modern),
    ] {
        assert_eq!(run_at(src, release), expected, "{release}");
    }
}

#[test]
fn colon_edge_rename_trace_and_recreation_keep_their_own_tokens() {
    // #1778. Renaming `a`/`:p` leaves `a:`/`p` and its trace alone; deleting
    // `a:` fires only that command's trace, and recreating `a:`/`p` does not
    // disturb the renamed `a`/`:q`. tclsh 8.6.18, 9.0.4 and 9.1.0 agree;
    // 8.4 and 8.5 cannot create the non-global `:p` at all.
    let src = r#"set r {}
set ::log {}
proc rec {args} {lappend ::log [lindex $args end]}
foreach s {
    {namespace eval a: {proc p {} {return A}}}
    {namespace eval a {proc :p {} {return B}}}
    {namespace eval a: {trace add command p delete rec}}
    {namespace eval a {rename :p :q}}
    {namespace eval a: {p}}
    {namespace eval a {:q}}
    {namespace eval a {:p}}
    {namespace delete a:}
    {set ::log}
    {namespace eval a: {proc p {} {return C}}}
    {namespace eval a: {p}}
    {namespace eval a {:q}}
} {
    lappend r [list [catch $s m] [expr {[string match {*invalid command*} $m] ? "invalid" : $m}]]
}
set r"#;
    for release in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        assert_eq!(
            run_at(src, release),
            r"{0 {}} {0 {}} {0 {}} {0 {}} {0 A} {0 B} {1 invalid} {0 {}} {0 delete} {0 {}} {0 C} {0 B}",
            "{release}"
        );
    }
}

#[test]
fn compiled_descendant_command_survives_activation_delayed_parent_deletion() {
    // #1755. Measured on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0: from
    // 8.5 the precompiled relative and absolute references both keep the
    // retained old child token while the deleting activation is live; 8.4
    // re-resolves the absolute spelling through the recreated namespace.
    let src = r"set answer [namespace eval ::N {
                    namespace eval C {proc q {} {return OLDQ}}
                    proc p {} {
                        namespace delete ::N
                        namespace eval ::N::C {proc q {} {return NEWQ}}
                        list [C::q] [::N::C::q]
                    }
                    p
                }]
                list $answer [::N::C::q]";
    for (release, expected) in [
        ("tcl8.4", "{OLDQ NEWQ} NEWQ"),
        ("tcl8.5", "{OLDQ OLDQ} NEWQ"),
        ("tcl8.6", "{OLDQ OLDQ} NEWQ"),
        ("tcl9.0", "{OLDQ OLDQ} NEWQ"),
        ("tcl9.1", "{OLDQ OLDQ} NEWQ"),
    ] {
        assert_eq!(run_at(src, release), expected, "{release}");
    }
}

#[test]
fn retained_and_recreated_namespace_variable_traces_are_isolated() {
    // Exact Tcl 9.0.4 oracle. Final teardown of the old token fires only its
    // old trace; its callback sees the same-spelled recreation as live, and
    // that new token keeps both its value and trace.
    assert_eq!(
        run(r"proc rec {tag n1 n2 op} {
                 lappend ::events [list $tag [namespace exists ::N] $n1 $n2 $op]
             }
             set events {}
             namespace eval N {
                 variable v old
                 trace add variable v unset {::rec old}
                 proc p {} {
                     namespace delete ::N
                     namespace eval ::N {
                         variable v new
                         trace add variable v unset {::rec new}
                     }
                     variable v
                     list [namespace exists {}] $v [set ::N::v]
                 }
             }
             set inside [::N::p]
             set after [list [namespace exists ::N] [set ::N::v] $events \
                             [trace info variable ::N::v]]
             namespace delete ::N
             list $inside $after $events"),
        "{0 old new} {1 new {{old 1 ::N::v {} unset}} {{unset {::rec new}}}} {{old 1 ::N::v {} unset} {new 0 ::N::v {} unset}}"
    );
}

#[test]
fn a_suspended_coroutine_releases_its_retained_namespace_on_completion() {
    // A parked coroutine owns its exact namespace activation. Deletion
    // unpublishes that token without tearing its command table down; ordinary
    // completion later pops the parked frame and performs deferred teardown.
    // Exact Tcl 9.0.4 oracle result (identical on 8.6.16).
    assert_eq!(
        run(r"set log {}
             proc rec {old new op} {lappend ::log [list $old $op]}
             namespace eval N {
                 proc q {} {return Q}
                 trace add command q delete ::rec
                 coroutine ::co apply {{} {
                     yield ready
                     list [q] [namespace exists ::N] [namespace current]
                 } ::N}
             }
             namespace delete ::N
             set before [list [namespace exists ::N] [llength $::log]]
             set resumed [co]
             list $before $resumed $log"),
        "{0 0} {Q 0 ::N} {{::N::q delete}}"
    );
}

#[test]
fn namespace_teardown_fires_trace_only_undefined_cells() {
    // Tcl 9.0.4 trace-18.3: registration itself materialises an undefined
    // namespace cell, which still participates in namespace destruction.
    assert_eq!(
        run(r"set events {}
             proc rec {n1 n2 op} {lappend ::events [list $n1 $n2 $op]}
             namespace eval N {}
             trace add variable ::N::var unset rec
             namespace delete ::N
             set events"),
        "{::N::var {} unset}"
    );
}

#[test]
fn namespace_teardown_removes_one_cell_then_purges_its_recreation() {
    // Exact Tcl 9.0.4 oracle. Each callback sees its own cell absent and the
    // namespace already unpublished. The remaining-count multiset proves the
    // cells are removed one at a time without pinning the hash-table order.
    // Recreating either removed name and attaching a new trace does not give it
    // a second callback during the dying token's fixed-point purge.
    assert_eq!(
        run(r"set events {}
             set remaining {}
             proc R {n1 n2 op} {
                 lappend ::events [list $n1 [info exists $n1] [namespace exists ::N]]
                 lappend ::remaining [expr {[info exists ::N::a] + [info exists ::N::b]}]
                 set $n1 revived
                 trace add variable $n1 unset {lappend ::events NEW;#}
             }
             namespace eval N {
                 variable a A
                 variable b B
                 trace add variable a unset R
                 trace add variable b unset R
             }
             namespace delete ::N
             list [lsort $events] [lsort -integer $remaining] \
                  [namespace exists ::N] [info exists ::N::a] [info exists ::N::b]"),
        "{{::N::a 0 0} {::N::b 0 0}} {0 1} 0 0 0"
    );
}

#[test]
fn a_retained_token_reports_its_own_name_and_commands() {
    // The current-namespace probes from inside the frame. Exact Tcl 9.0.4
    // oracle result (identical on 8.6.16).
    assert_eq!(
        run(r"namespace eval N {
                 variable v 1
                 proc p {} {
                     namespace delete ::N
                     list [namespace exists [namespace current]] \
                          [info commands [namespace current]::*] [info procs] \
                          [namespace which -variable v]
                 }
             }
             ::N::p"),
        r"0 {} p ::N::v"
    );
}

#[test]
fn an_active_child_defers_while_its_parent_tears_down() {
    // `TclDeleteNamespaceChildren` re-enters `Tcl_DeleteNamespace` per child,
    // so the parent (no frame of its own) tears down at once while the child
    // holding the frame is retained until it pops. Exact Tcl 9.0.4 oracle
    // result (identical on 8.6.16).
    assert_eq!(
        run(r"set log {}
             proc rec {old new op} {lappend ::log [list $old $op]}
             namespace eval N {
                 proc np {} {return NP}
                 namespace eval C {
                     proc cq {} {return CQ}
                     proc p {} {
                         namespace delete ::N
                         set abs [join [list {} N C cq] ::]
                         list [namespace exists ::N] [namespace exists ::N::C] \
                              [namespace current] [cq] [catch {$abs} m] $m [llength $::log]
                     }
                 }
             }
             trace add command ::N::np delete rec
             trace add command ::N::C::cq delete rec
             set a [::N::C::p]
             list $a $log"),
        r#"{0 0 ::N::C CQ 1 {invalid command name "::N::C::cq"} 1} {{::N::np delete} {::N::C::cq delete}}"#
    );
}

#[test]
fn command_delete_traces_wait_for_the_last_activation() {
    // Nothing in the retained token fires until the last frame pops, and the
    // enclosing `namespace eval N` is a second activation, so returning from
    // the proc is not enough. Exact Tcl 9.0.4 oracle result (identical on
    // 8.6.16).
    assert_eq!(
        run(r"set log {}
             proc rec {old new op} {lappend ::log [list $old $op]}
             namespace eval N {
                 proc q {} {return Q}
                 proc p {} {namespace delete ::N; set ::mid $::log; return [q]}
             }
             trace add command ::N::p delete rec
             trace add command ::N::q delete rec
             set r [namespace eval N {set inner [p]; set ::after_p $::log; set inner}]
             list $r $mid $after_p $log"),
        r"Q {} {} {{::N::p delete} {::N::q delete}}"
    );
}

#[test]
fn a_deferred_namespace_is_unpublished_for_every_caller() {
    // The frame that defers need not be the one running the delete, and the
    // retained token refuses a second deletion — its name is already gone.
    // Exact Tcl 9.0.4 oracle results (identical on 8.6.16).
    assert_eq!(
        run(r"set log {}
             proc rec {old new op} {lappend ::log [list $old $op]}
             namespace eval N {
                 proc q {} {return Q}
                 proc p2 {} {namespace delete ::N}
                 proc p {} {p2; list [q] [llength $::log]}
             }
             trace add command ::N::q delete rec
             set a [::N::p]
             list $a [llength $log]"),
        r"{Q 0} 1"
    );
    assert_eq!(
        run(r"namespace eval N {
                 proc p {} {namespace delete ::N; list [catch {namespace delete ::N} m] $m}
             }
             ::N::p"),
        r#"1 {unknown namespace "::N" in namespace delete command}"#
    );
}

#[test]
fn an_owned_ensemble_dies_while_its_namespace_is_retained() {
    // `Tcl_DeleteNamespace` pops `nsPtr->ensembles` before it looks at the
    // activation count, so an owned ensemble dies at once. Exact Tcl 9.0.4
    // oracle result (identical on 8.6.16).
    assert_eq!(
        run(r"set log {}
             proc rec {old new op} {lappend ::log [list $old $op]}
             namespace eval N {
                 namespace ensemble create -command ::E
                 proc p {} {namespace delete ::N; list [info commands ::E] $::log}
             }
             trace add command ::E delete rec
             ::N::p"),
        r"{} {{::E delete}}"
    );
}

#[test]
fn a_deferred_teardown_keeps_tcl_hash_order() {
    // The deferred teardown is the ordinary one run later, so the command table
    // still empties in `Tcl_FirstHashEntry` order — imports at their own hash
    // position included. Exact Tcl 9.0.4 oracle results (identical on
    // 8.6.16).
    assert_eq!(
        run(r"set log {}
             proc rec2 {old new op} {lappend ::log [namespace tail $old]}
             namespace eval N {}
             foreach n {one two three four five six seven eight nine ten} {
                 proc ::N::$n {} {}
             }
             proc ::N::p {} {namespace delete ::N; return [llength $::log]}
             foreach n [info commands ::N::*] {trace add command $n delete rec2}
             set r [::N::p]
             list $r $log"),
        r"0 {p six four three eight seven nine five two one ten}"
    );
    assert_eq!(
        run(r"set log {}
             proc rec2 {old new op} {lappend ::log [namespace tail $old]}
             namespace eval S {proc s1 {} {}; namespace export s*}
             namespace eval N {
                 namespace import ::S::s1
                 proc p {} {namespace delete ::N; return}
             }
             foreach n {one two three four five six seven eight nine ten} {
                 proc ::N::$n {} {}
             }
             foreach n [info commands ::N::*] {trace add command $n delete rec2}
             ::N::p
             set log"),
        "p three seven one two four eight five nine s1 six ten"
    );
}

#[test]
fn a_command_deleted_from_a_retained_frame_reports_its_own_namespace() {
    // `namespace current` answers from the token's retained name, so a proc the
    // retained frame calls still reports `::N`. Exact Tcl 9.0.4 oracle result
    // (identical on 8.6.16).
    assert_eq!(
        run(r"set log {}
             proc rec {old new op} {lappend ::log [list $old $op]}
             namespace eval N {
                 proc q {} {return [namespace current]}
                 proc p {} {namespace delete ::N; set a [q]; set b $::log; list $a $b}
             }
             trace add command ::N::q delete rec
             set a [::N::p]
             list $a $log"),
        r"{::N {}} {{::N::q delete}}"
    );
}
