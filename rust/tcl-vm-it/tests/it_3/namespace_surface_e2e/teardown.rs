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

//! Namespace teardown (`TclTeardownNamespace`): paths and unknown handlers, owned ensembles, hash order of commands and children, and commands created or deleted by callbacks.

use super::*;

// Namespace teardown (`TclTeardownNamespace`, tclNamesp.c:1084)

/// A rename whose destination lands in the caller's *retained* namespace — one
/// the running proc deleted out from under itself — leaves nothing publicly
/// addressable at the new key: `register_command` ends in
/// `absorb_retained_binding`, which moves the fresh binding into the retained
/// namespace's own table. Anything on the rename path that expects to read the
/// command back out of the live map afterwards is therefore wrong. tclsh
/// 8.6.16 and 9.0.4 both complete the rename and invoke the ensemble.
#[test]
fn renaming_an_ensemble_into_a_retained_namespace_still_dispatches() {
    assert_eq!(
        run("namespace eval E { proc sub {} { return S }\n\
             \x20   namespace export sub; namespace ensemble create }\n\
             namespace eval N { proc p {} {\n\
             \x20   namespace delete ::N\n\
             \x20   rename ::E local\n\
             \x20   return [local sub]\n\
             } }\n\
             N::p"),
        "S"
    );
}

#[test]
fn deleting_a_namespace_clears_paths_in_both_directions() {
    // (a) another namespace's path loses the deleted entry, and a recreated
    // namespace does not resurrect it. tclsh 8.6.16 / 9.0.4: {} then the
    // `invalid command name` the empty path implies.
    assert_eq!(
        run("namespace eval a {proc pp {} {return A1}}\n\
             namespace eval b {namespace path ::a}\n\
             namespace delete ::a\n\
             namespace eval b {namespace path}"),
        ""
    );
    assert_eq!(
        run("namespace eval a {proc pp {} {return A1}}\n\
             namespace eval b {namespace path ::a}\n\
             namespace delete ::a\n\
             namespace eval a {proc pp {} {return A2}}\n\
             namespace eval b {catch {pp} m; set m}"),
        "invalid command name \"pp\""
    );
}

#[test]
fn deleting_a_namespace_clears_its_own_path_and_unknown_handler() {
    // (b) a recreated namespace starts clean. tclsh 8.6.16 / 9.0.4: {} / {}.
    assert_eq!(
        run("namespace eval a {}\n\
             namespace eval z {namespace path ::a; namespace unknown {puts UNK}}\n\
             namespace delete ::z\n\
             namespace eval z {}\n\
             namespace eval z {namespace path}"),
        ""
    );
    assert_eq!(
        run("namespace eval a {}\n\
             namespace eval z {namespace path ::a; namespace unknown {puts UNK}}\n\
             namespace delete ::z\n\
             namespace eval z {}\n\
             namespace eval z {namespace unknown}"),
        ""
    );
}

#[test]
fn an_unset_namespace_unknown_is_empty_outside_the_global_namespace() {
    // Only the global namespace carries `::unknown` by default.
    assert_eq!(run("namespace unknown"), "::unknown");
    assert_eq!(
        run("namespace eval q {}\nnamespace eval q {namespace unknown}"),
        ""
    );
    // Resetting a namespace's handler goes back to empty, not to ::unknown.
    assert_eq!(
        run(
            "namespace eval q {namespace unknown {puts A}; namespace unknown {}}\n\
             namespace eval q {namespace unknown}"
        ),
        ""
    );
}

#[test]
fn deleting_a_namespace_deletes_its_ensembles() {
    // (c) `Tcl_DeleteNamespace`'s `while (nsPtr->ensembles != NULL)` loop —
    // the command lives in the global table but belongs to ::ens1.
    assert_eq!(
        run(
            "namespace eval ens1 {namespace export *; proc sub {} {return S}\n\
             namespace ensemble create -command ::myens}\n\
             info commands ::myens"
        ),
        "::myens"
    );
    assert_eq!(
        run(
            "namespace eval ens1 {namespace export *; proc sub {} {return S}\n\
             namespace ensemble create -command ::myens}\n\
             namespace delete ::ens1\n\
             info commands ::myens"
        ),
        ""
    );
}

#[test]
fn namespace_owned_ensemble_trace_precedes_the_dying_marker() {
    // Tcl_DeleteNamespace drains nsPtr->ensembles before setting NS_DYING.
    // A global command owned by ::N therefore remains visible as an ensemble
    // while its delete trace also sees the namespace token as live; both are
    // absent once namespace deletion returns. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set seen {}
             proc observe {old new op} {
                 lappend ::seen [list [namespace exists ::N] $old \
                     [info commands $old] [namespace ensemble exists $old]]
             }
             namespace eval N {namespace ensemble create -command ::E}
             trace add command ::E delete observe
             namespace delete ::N
             list $seen [namespace exists ::N] [info commands ::E]"),
        "{{1 ::E ::E 1}} 0 {}"
    );
}

#[test]
fn namespace_owned_ensembles_retire_live_heads_in_creation_order() {
    // Tcl_CreateEnsemble pushes each token at the head of nsPtr->ensembles and
    // Tcl_DeleteNamespace removes one live head at a time. A delete callback's
    // newly-created ensemble is consequently the next head, ahead of the older
    // token that was already waiting. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set seen {}
             proc deleted {old new op} {
                 lappend ::seen [namespace tail $old]
                 if {[namespace tail $old] eq \"E2\"} {
                     namespace eval ::N {
                         namespace ensemble create -command ::E3
                     }
                     trace add command ::E3 delete deleted
                 }
             }
             namespace eval ::N {
                 namespace ensemble create -command ::E1
                 namespace ensemble create -command ::E2
             }
             trace add command ::E1 delete deleted
             trace add command ::E2 delete deleted
             namespace delete ::N
             set seen"),
        "E2 E3 E1"
    );
}

#[test]
fn parent_teardown_reaches_each_child_after_parent_commands() {
    // Tcl drains a parent's owned ensembles while both namespace tokens are
    // live, then marks only the parent dying and retires its ordinary command
    // table. The child remains live until its own recursive ensemble phase.
    // Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set seen {}
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
             set seen"),
        "{P 1 1} {PC 0 1 ::CE} {C 0 1}"
    );
}

#[test]
fn child_namespace_teardown_uses_tcl_string_hash_order() {
    // TclDeleteNamespaceChildren snapshots the parent's childTable with
    // Tcl_FirstHashEntry/Tcl_NextHashEntry, not namespace creation order.
    // Each child's owned ensemble makes that recursive order observable. Exact
    // Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set seen {}
             proc deleted {tag old new op} {lappend ::seen $tag}
             foreach n {one two three four five six seven eight nine ten} {
                 namespace eval ::P::$n \
                     [list namespace ensemble create -command ::E_$n]
                 trace add command ::E_$n delete [list deleted $n]
             }
             namespace delete ::P
             set seen"),
        "six four three eight seven nine five two one ten"
    );
}

#[test]
fn namespace_teardown_uses_command_table_tcl_string_hash_order() {
    // TclTeardownNamespace snapshots nsPtr->cmdTable with
    // Tcl_FirstHashEntry/Tcl_NextHashEntry before deleting each token, so the
    // delete traces fire in the retained TCL_STRING_KEYS bucket order rather
    // than definition or lexical order. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set log {}
             proc rec {old new op} {lappend ::log [namespace tail $old]}
             namespace eval N {}
             foreach n {one two three four five six seven eight nine ten} {
                 proc ::N::$n {} {}
                 trace add command ::N::$n delete rec
             }
             namespace delete ::N
             set log"),
        "six four three eight seven nine five two one ten"
    );
}

#[test]
fn command_table_hash_order_records_table_growth() {
    // RebuildTable quadruples the bucket array once numEntries reaches three
    // times numBuckets and re-pushes each chain head-first, reversing it.
    // Thirteen commands cross the first threshold. Exact Tcl 9.0.4 oracle
    // result.
    assert_eq!(
        run("set log {}
             proc rec {old new op} {lappend ::log [namespace tail $old]}
             namespace eval N {}
             for {set i 0} {$i < 13} {incr i} {
                 proc ::N::c$i {} {}
                 trace add command ::N::c$i delete rec
             }
             namespace delete ::N
             set log"),
        "c5 c6 c7 c8 c9 c0 c1 c10 c2 c11 c12 c3 c4"
    );
}

#[test]
fn command_table_hash_order_retains_capacity_across_deletions() {
    // Tcl_DeleteHashEntry never shrinks the bucket array, so commands created
    // after a bulk deletion land in the grown table and precede the survivors.
    // Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set log {}
             proc rec {old new op} {lappend ::log [namespace tail $old]}
             namespace eval N {}
             foreach n {a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11} {proc ::N::$n {} {}}
             foreach i {1 2 4 5 6 7 8 9 10 11} {rename ::N::a$i {}}
             foreach n {b1 b2 b3} {proc ::N::$n {} {}}
             foreach n [info commands ::N::*] {trace add command $n delete rec}
             namespace delete ::N
             set log"),
        "b1 b2 b3 a0 a3"
    );
}

#[test]
fn command_table_hash_order_moves_a_redefined_command() {
    // TclCreateObjCommandInNs deletes the existing hash entry and creates a
    // fresh one, so redefining `one` moves it to its bucket head. Exact Tcl
    // 9.0.4 oracle result.
    assert_eq!(
        run("set log {}
             proc rec {old new op} {lappend ::log [namespace tail $old]}
             namespace eval N {}
             foreach n {one two three four five six seven eight nine ten} {proc ::N::$n {} {}}
             proc ::N::one {} {return again}
             foreach n [info commands ::N::*] {trace add command $n delete rec}
             namespace delete ::N
             set log"),
        "six four three eight seven one nine five two ten"
    );
    // TclRenameCommand creates the destination entry the same way, before it
    // deletes the source's. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set log {}
             proc rec {old new op} {lappend ::log [namespace tail $old]}
             namespace eval N {}
             foreach n {one two three four five six seven eight nine ten} {proc ::N::$n {} {}}
             rename ::N::one ::N::uno
             foreach n [info commands ::N::*] {trace add command $n delete rec}
             namespace delete ::N
             set log"),
        "six four three eight seven uno nine five two ten"
    );
}

#[test]
fn namespace_teardown_visits_a_callback_created_command_in_the_next_pass() {
    // TclTeardownNamespace loops while cmdTable is non-empty, so a command a
    // delete callback creates is torn down by a second snapshot, after every
    // entry of the first one. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set log {}
             proc rec {old new op} {lappend ::log [namespace tail $old]}
             proc mk {old new op} {
                 lappend ::log [namespace tail $old]
                 proc ::N::zz {} {}
                 trace add command ::N::zz delete rec
             }
             namespace eval N {}
             foreach n {one two three four five six seven eight nine ten} {
                 proc ::N::$n {} {}
                 trace add command ::N::$n delete rec
             }
             trace add command ::N::six delete mk
             namespace delete ::N
             set log"),
        "six six four three eight seven nine five two one ten zz"
    );
}

#[test]
fn namespace_teardown_defers_a_callback_recreated_command_to_the_next_pass() {
    // A redefinition inside the delete callback unlinks the dying token's own
    // entry (Tcl_DeleteCommandFromToken's CMD_DYING branch), so the
    // replacement is a distinct token the current snapshot no longer names and
    // the next snapshot picks up. 9.0: exact tclsh 9.0.4 oracle result; 8.6.16
    // segfaults on this script.
    assert_eq!(
        run("set log {}
             proc rec {old new op} {lappend ::log [namespace tail $old]}
             proc remake {old new op} {
                 proc ::N::two {} {}
                 trace add command ::N::two delete rec
                 lappend ::log remade
             }
             namespace eval N {}
             foreach n {one two three four five six seven eight nine ten} {
                 proc ::N::$n {} {}
                 trace add command ::N::$n delete rec
             }
             trace add command ::N::two delete remake
             namespace delete ::N
             set log"),
        "six four three eight seven nine five remade two one ten two"
    );
}

#[test]
fn namespace_teardown_skips_a_command_a_callback_already_deleted() {
    // CMD_TRACE_ACTIVE is per Command, not interpreter-wide: `one` fires its
    // own delete trace nested inside six's callback, and the snapshot entry
    // for it is gone by the time the loop reaches it. Exact Tcl 9.0.4 oracle
    // result.
    assert_eq!(
        run("set log {}
             proc rec {old new op} {lappend ::log [namespace tail $old]}
             proc killone {old new op} {rename ::N::one {}; lappend ::log killed-one}
             namespace eval N {}
             foreach n {one two three four five six seven eight nine ten} {
                 proc ::N::$n {} {}
                 trace add command ::N::$n delete rec
             }
             trace add command ::N::six delete killone
             namespace delete ::N
             set log"),
        "one killed-one six four three eight seven nine five two ten"
    );
}

#[test]
fn namespace_teardown_places_imports_at_their_hash_positions() {
    // An imported redirect occupies an ordinary cmdTable entry, so it retires
    // at its own hash position rather than in a separate pass over the
    // namespace's imports. Exact Tcl 9.0.4 oracle result.
    let imports = "set log {}
             proc rec {old new op} {lappend ::log [namespace tail $old]}
             namespace eval S {proc s1 {} {}; proc s2 {} {}; namespace export s*}
             namespace eval N {namespace import ::S::s1 ::S::s2}
             foreach n {one two three four five six seven eight} {proc ::N::$n {} {}}
             foreach n [info commands ::N::*] {trace add command $n delete rec}
             namespace delete ::N
             set log";
    assert_eq!(
        run(imports),
        "six four three s1 eight seven s2 five two one"
    );
    // `namespace forget` deletes the entry and the re-import creates a new one
    // at the same bucket head, so the order is unchanged.
    assert_eq!(
        run("set log {}
             proc rec {old new op} {lappend ::log [namespace tail $old]}
             namespace eval S {proc s1 {} {}; proc s2 {} {}; namespace export s*}
             namespace eval N {
                 namespace import ::S::s1 ::S::s2
                 namespace forget ::S::s1
                 namespace import ::S::s1
             }
             foreach n {one two three four five six seven eight} {proc ::N::$n {} {}}
             foreach n [info commands ::N::*] {trace add command $n delete rec}
             namespace delete ::N
             set log"),
        "six four three s1 eight seven s2 five two one"
    );
}

#[test]
fn namespace_teardown_retires_each_sources_import_tree_before_the_next_entry() {
    // Tcl_DeleteCommandFromToken walks the deleted command's ImportRef list
    // depth-first straight after its own delete trace, so a source's whole
    // import tree fires before the next cmdTable entry. Exact Tcl 9.0.4 oracle
    // result.
    assert_eq!(
        run("set log {}
             namespace eval N {proc one {} {}; proc two {} {}; proc six {} {}
                               namespace export *}
             namespace eval I {namespace import ::N::one; namespace export *}
             namespace eval J {namespace import ::I::one}
             foreach c {::N::one ::N::two ::N::six ::I::one ::J::one} {
                 trace add command $c delete [list apply {{c old new op} {
                     lappend ::log $c
                 }} $c]
             }
             namespace delete ::N
             set log"),
        "::N::six ::N::two ::N::one ::I::one ::J::one"
    );
}

#[test]
fn a_delete_callbacks_own_trace_dies_with_the_dying_token() {
    // `Tcl_DeleteCommandFromToken` frees the whole `cmdPtr->tracePtr` list
    // after `CallCommandTraces`. A trace the callback registers on the command
    // being deleted attaches to that same dying token, so it never fires — not
    // in this walk (`CallCommandTraces` follows `active.nextTracePtr`), and not
    // for a later command that takes the vacated name. Exact Tcl 9.0.4 oracle
    // results (identical on 8.6.16).
    let recorders = "set log {}
             proc inner {old new op} {lappend ::log inner:$old}
             proc outer {old new op} {lappend ::log outer:$old
                 trace add command $old delete inner}\n";
    // `rename cmd {}`.
    assert_eq!(
        run(&format!(
            "{recorders}
             proc p {{}} {{}}
             trace add command ::p delete outer
             rename ::p {{}}
             set a $log
             proc p {{}} {{}}
             rename ::p {{}}
             list $a $log"
        )),
        "outer:::p outer:::p"
    );
    // Redefinition (`TclCreateObjCommandInNs` deletes the old token first).
    assert_eq!(
        run(&format!(
            "{recorders}
             proc p {{}} {{}}
             trace add command ::p delete outer
             proc p {{}} {{}}
             set a $log
             proc p {{}} {{}}
             set b $log
             rename ::p {{}}
             list $a $b $log"
        )),
        "outer:::p outer:::p outer:::p"
    );
    // Namespace teardown.
    assert_eq!(
        run(&format!(
            "{recorders}
             namespace eval N {{proc q {{}} {{}}}}
             trace add command ::N::q delete outer
             namespace delete ::N
             set a $log
             namespace eval N {{proc q {{}} {{}}}}
             namespace delete ::N
             list $a $log"
        )),
        "outer:::N::q outer:::N::q"
    );
}

#[test]
fn a_replacements_trace_survives_the_deletion_that_created_it() {
    // The other half of the same rule: a callback that *binds* a replacement
    // at the vacated name registers on that new token, whose own trace list C
    // never touches. Only the dying token's list is freed. Exact Tcl 9.0.4
    // oracle results (identical on 8.6.16).
    let recorders = "set log {}
             proc inner {old new op} {lappend ::log inner:$old}
             proc mk {old new op} {lappend ::log mk:$old
                 proc ::p {} {}
                 trace add command ::p delete inner}\n";
    assert_eq!(
        run(&format!(
            "{recorders}
             proc p {{}} {{}}
             trace add command ::p delete mk
             rename ::p {{}}
             set a [list $log [llength [info commands ::p]]]
             rename ::p {{}}
             list $a $log [llength [info commands ::p]]"
        )),
        "{mk:::p 1} {mk:::p inner:::p} 0"
    );
    // A trace the callback adds *before* the replacement still belongs to the
    // dying token and goes with it; only the one added after survives.
    assert_eq!(
        run("set log {}
             proc early {old new op} {lappend ::log early:$old}
             proc late {old new op} {lappend ::log late:$old}
             proc mk {old new op} {lappend ::log mk:$old
                 trace add command $old delete early
                 proc ::p {} {}
                 trace add command ::p delete late}
             proc p {} {}
             trace add command ::p delete mk
             rename ::p {}
             set a $log
             rename ::p {}
             list $a $log"),
        "mk:::p {mk:::p late:::p}"
    );
}

#[test]
fn a_refused_alias_rename_keeps_the_command_tables_growth() {
    // `TclRenameCommand` creates the destination hash entry *before*
    // `TclPreventAliasLoop` and deletes it again on a refusal, so the transient
    // twelfth entry rebuilds the eleven-command table from 4 buckets to 16 and
    // the grown array outlives the rejected rename. Exact Tcl 9.0.4 oracle
    // results (identical on 8.6.16).
    let eleven = "set log {}
             proc rec {old new op} {lappend ::log [namespace tail $old]}
             namespace eval N {}
             foreach n {one two three four five six seven eight nine ten eleven} {
                 proc ::N::$n {} {}
             }\n";
    let trace_and_delete = "foreach n [info commands ::N::*] {trace add command $n delete rec}
             namespace delete ::N\n";
    assert_eq!(
        run(&format!("{eleven}{trace_and_delete} set log")),
        "six four three eight seven nine five two one eleven ten"
    );
    assert_eq!(
        run(&format!(
            "{eleven}
             interp alias {{}} ::a {{}} ::N::b
             set c [catch {{rename ::a ::N::b}} m]
             {trace_and_delete}
             list $c $m [llength [info commands ::a]] $log"
        )),
        "1 {cannot define or rename alias \"b\": would create a loop} 1 \
         {three seven one two four eleven eight five nine six ten}"
    );
}

#[test]
fn deleting_a_namespace_fully_retires_its_visible_ensemble_once() {
    // The ensemble lives in the global command table but is owned by ::N.
    // Namespace teardown fires and removes its delete trace. A later, distinct
    // ::E lifecycle must not inherit that trace. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set hits {}
             proc deleted {old new op} {lappend ::hits [list $old $new $op]}
             namespace eval N {
                 proc x args {return OLD}
                 namespace ensemble create -command ::E -map {x ::N::x}
             }
             trace add command ::E delete deleted
             namespace delete ::N
             set first $hits
             namespace eval N {
                 proc x args {return NEW}
                 namespace ensemble create -command ::E -map {x ::N::x}
             }
             rename ::E {}
             list $first $hits"),
        "{{::E {} delete}} {{::E {} delete}}"
    );
}

#[test]
fn deleting_a_namespace_fully_retires_ordinary_commands_once() {
    // Ordinary namespace members use the same command-token deletion
    // lifecycle as explicitly renamed commands. The trace fires once during
    // teardown and cannot attach to a later command at the vacated name.
    // Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set hits {}
             proc deleted {old new op} {lappend ::hits [list $old $new $op]}
             namespace eval N {proc p {} {}}
             trace add command ::N::p delete deleted
             namespace delete ::N
             set first $hits
             namespace eval N {proc p {} {}}
             rename ::N::p {}
             list $first $hits"),
        "{{::N::p {} delete}} {{::N::p {} delete}}"
    );
}

#[test]
fn namespace_delete_marks_the_namespace_dying_before_command_traces() {
    // The namespace token is already non-existent, but the dying command is
    // still visible during its delete trace. A callback may replace and invoke
    // that command; the replacement remains in the dying table only until
    // teardown finishes. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set seen {}
             proc deleted {old new op} {
                 lappend ::seen [list before [namespace exists ::N] \
                                      [info commands $old]]
                 proc $old {} {return NEW}
                 lappend ::seen [list after [namespace exists ::N] \
                                      [info commands $old] \
                                      [namespace exists ::N] [$old]]
             }
             namespace eval N {proc p {} {return OLD}}
             trace add command ::N::p delete deleted
             namespace delete ::N
             set seen"),
        "{before 0 ::N::p} {after 0 ::N::p 0 NEW}"
    );
    assert_eq!(
        run("set hits 0
             proc deleted {old new op} {incr ::hits; proc $old {} {return NEW}}
             namespace eval N {proc p {} {return OLD}}
             trace add command ::N::p delete deleted
             namespace delete ::N
             set after [info commands ::N::p]
             namespace eval N {proc p {} {return LATER}}
             rename ::N::p {}
             list $after $hits"),
        "{} 1"
    );

    // Re-entering the exact dying namespace is refused: `namespace eval ::N`
    // no longer finds the token (it is `NS_DYING`) and `Tcl_CreateNamespace`
    // then trips over the child entry `TclTeardownNamespace` unlinks only
    // after the command loop, so the callback errors out and appends nothing.
    // Exact Tcl 9.0.4 oracle result (identical on 8.6.16).
    assert_eq!(
        run("set seen {}
             proc deleted {old new op} {
                 namespace eval ::N {lappend ::seen [namespace exists ::N]}
             }
             namespace eval N {proc p {} {}}
             trace add command ::N::p delete deleted
             namespace delete ::N
             list $seen [namespace exists ::N]"),
        "{} 0"
    );
}

#[test]
fn namespace_exists_uses_namespace_name_canonicalisation() {
    // A trailing separator run belongs to namespace-name grammar and drops;
    // it must not be preserved as the empty command tail. Exact Tcl 9.0.4
    // oracle result (and a regression for namespace_colon_runs_e2e).
    assert_eq!(
        run("namespace eval c9::: {}
             list [namespace exists ::c9] [namespace exists c9:::]"),
        "1 1"
    );
}

#[test]
fn namespace_delete_retires_callback_created_hidden_ensemble_imports() {
    // The callback creates a namespace-owned ensemble, imports it, and moves
    // that import to the hidden table. Namespace teardown must revisit the
    // live command/ensemble frontier and retire the hidden import through its
    // source token's normal import graph. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("namespace export E
             proc target {} {return TARGET}
             proc late {old new op} {
                 namespace eval ::N {
                     namespace ensemble create -command ::E -map {x ::target}
                 }
                 namespace eval ::I {namespace import ::E}
                 interp hide {} ::I::E held
             }
             namespace eval ::N {proc p {} {}}
             trace add command ::N::p delete late
             namespace delete ::N
             set c [catch {interp invokehidden {} held x} m o]
             list [info commands ::E] [interp hidden {}] \
                  [info commands ::I::E] $c $m [dict get $o -errorcode]"),
        "{} {} {} 1 {invalid hidden command name \"held\"} \
         {TCL LOOKUP HIDDENTOKEN held}"
    );
}

#[test]
fn namespace_path_checks_that_every_entry_exists() {
    // (d) `NamespacePathCmd` resolves each entry with
    // `TclGetNamespaceFromObj` before installing the path.
    assert_eq!(
        run("namespace eval outer2 {catch {namespace path inner} m; set m}"),
        "namespace \"inner\" not found in \"::outer2\""
    );
    assert_eq!(
        run("namespace eval outer3 {catch {namespace path ::inner} m; set m}"),
        "namespace \"::inner\" not found"
    );
    // A rejected path leaves the previous one installed.
    assert_eq!(
        run("namespace eval a {}\n\
             namespace eval o {namespace path ::a; catch {namespace path {::a ::nope}}\n\
             namespace path}"),
        "::a"
    );
}

#[test]
fn namespace_delete_accepts_the_global_root() {
    // Tcl 9.0.4 accepts the command and tears down the tree.
    assert_eq!(run("namespace delete ::"), "");
    assert_eq!(
        run("namespace delete ::
             puts hi"),
        "invalid command name \"puts\""
    );
}

#[test]
fn namespace_origin_reports_its_own_usage_for_both_bad_arities() {
    let usage = "wrong # args: should be \"namespace origin name\"";
    assert_eq!(run("catch {namespace origin} m; set m"), usage);
    assert_eq!(run("catch {namespace origin set extra} m; set m"), usage);
}

#[test]
fn namespace_children_matches_tcls_string_hash_iteration_order() {
    let setup = "namespace eval order {}
                 foreach n {one two three four five six seven eight nine ten} {
                     namespace eval ::order::$n {}
                 }\n";
    assert_eq!(
        run(&format!("{setup}namespace children ::order")),
        "::order::six ::order::four ::order::three ::order::eight \
         ::order::seven ::order::nine ::order::five ::order::two \
         ::order::one ::order::ten"
    );
    assert_eq!(
        run(&format!("{setup}namespace children ::order ::order::t*")),
        "::order::three ::order::two ::order::ten"
    );
}

#[test]
fn namespace_children_retains_tcls_hash_capacity_after_deletion() {
    assert_eq!(
        run("namespace eval p {}
             foreach n {a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11} {
                 namespace eval ::p::$n {}
             }
             foreach i {1 2 4 5 6 7 8 9 10 11} {namespace delete ::p::a$i}
             namespace children ::p"),
        "::p::a0 ::p::a3"
    );
}
