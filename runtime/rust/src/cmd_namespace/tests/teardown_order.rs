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

//! `namespace delete`: the hash order commands are torn down in, callbacks that create or delete commands mid-teardown, and subtree removal.

use super::*;

/// Run `script` (which fills `::log` through delete-trace callbacks) and
/// return the resulting log. Every expectation below is an exact tclsh
/// 9.0.4 result, identical on 8.6.16 unless the test says otherwise.
fn teardown_log(script: &str) -> String {
    let mut log = String::new();
    leak_free(|i| {
        let recorder = b"set log {}
                 proc rec {old new op} {lappend ::log [namespace tail $old]}";
        assert_eq!(i.eval_str(recorder), Code::Ok);
        assert_eq!(
            i.eval_str(script.as_bytes()),
            Code::Ok,
            "{}",
            String::from_utf8_lossy(&i.result_bytes())
        );
        assert_eq!(i.eval_str(b"set ::log"), Code::Ok);
        log = String::from_utf8_lossy(&i.result_bytes()).into_owned();
    });
    log
}

/// Define `names` as procs in `::N` and trace each one's deletion.
fn traced_procs(names: &str) -> String {
    format!(
        "namespace eval N {{}}
             foreach n {{{names}}} {{
                 proc ::N::$n {{}} {{}}
                 trace add command ::N::$n delete rec
             }}"
    )
}

#[test]
fn namespace_teardown_follows_command_table_hash_order() {
    // TclTeardownNamespace snapshots cmdTable with Tcl_FirstHashEntry, so
    // the delete traces fire in the retained TCL_STRING_KEYS bucket order,
    // not the definition or lexical order.
    assert_eq!(
        teardown_log(&format!(
            "{}
                 namespace delete ::N",
            traced_procs("one two three four five six seven eight nine ten")
        )),
        "six four three eight seven nine five two one ten"
    );
}

#[test]
fn namespace_teardown_hash_order_records_table_growth() {
    // RebuildTable quadruples the bucket array once entries reach three
    // times the bucket count and re-pushes each chain head-first, which
    // reverses it. Thirteen commands cross the first threshold.
    assert_eq!(
        teardown_log(&format!(
            "{}
                 namespace delete ::N",
            traced_procs("c0 c1 c2 c3 c4 c5 c6 c7 c8 c9 c10 c11 c12")
        )),
        "c5 c6 c7 c8 c9 c0 c1 c10 c2 c11 c12 c3 c4"
    );
}

#[test]
fn namespace_teardown_hash_order_retains_capacity_across_deletions() {
    // Tcl_DeleteHashEntry never shrinks the bucket array, so commands
    // created after a bulk deletion land in the grown table and precede
    // the survivors.
    assert_eq!(
        teardown_log(
            "namespace eval N {}
                 foreach n {a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11} {proc ::N::$n {} {}}
                 foreach i {1 2 4 5 6 7 8 9 10 11} {rename ::N::a$i {}}
                 foreach n {b1 b2 b3} {proc ::N::$n {} {}}
                 foreach n [info commands ::N::*] {trace add command $n delete rec}
                 namespace delete ::N"
        ),
        "b1 b2 b3 a0 a3"
    );
}

#[test]
fn namespace_teardown_hash_order_moves_a_redefined_command() {
    // TclCreateObjCommandInNs deletes the existing hash entry and creates
    // a fresh one, so redefining `one` moves it to its bucket head.
    assert_eq!(
        teardown_log(
            "namespace eval N {}
                 foreach n {one two three four five six seven eight nine ten} {
                     proc ::N::$n {} {}
                 }
                 proc ::N::one {} {return again}
                 foreach n [info commands ::N::*] {trace add command $n delete rec}
                 namespace delete ::N"
        ),
        "six four three eight seven one nine five two ten"
    );
    // TclRenameCommand creates the destination entry the same way.
    assert_eq!(
        teardown_log(
            "namespace eval N {}
                 foreach n {one two three four five six seven eight nine ten} {
                     proc ::N::$n {} {}
                 }
                 rename ::N::one ::N::uno
                 foreach n [info commands ::N::*] {trace add command $n delete rec}
                 namespace delete ::N"
        ),
        "six four three eight seven uno nine five two ten"
    );
}

#[test]
fn namespace_teardown_visits_a_callback_created_command_in_the_next_pass() {
    // TclTeardownNamespace loops while cmdTable is non-empty, so a command
    // a delete callback creates is torn down by a second snapshot, after
    // every entry of the first one.
    assert_eq!(
        teardown_log(&format!(
            "{}
                 proc mk {{old new op}} {{
                     lappend ::log [namespace tail $old]
                     proc ::N::zz {{}} {{}}
                     trace add command ::N::zz delete rec
                 }}
                 trace add command ::N::six delete mk
                 namespace delete ::N",
            traced_procs("one two three four five six seven eight nine ten")
        )),
        "six six four three eight seven nine five two one ten zz"
    );
}

#[test]
fn namespace_teardown_defers_a_callback_recreated_command_to_the_next_pass() {
    // A redefinition inside the delete callback unlinks the dying token's
    // entry itself (Tcl_DeleteCommandFromToken's CMD_DYING branch), so the
    // replacement is a distinct token the current snapshot no longer names.
    // Native proof: naming.namespace.original-command-holder-routing
    // docs/design/analysis/name-resolution-proofs/namespace-original-command-holder-routing.md
    // The original C86, C90 and C91 controls complete with this order.
    assert_eq!(
        teardown_log(&format!(
            "{}
                 proc remake {{old new op}} {{
                     proc ::N::two {{}} {{}}
                     trace add command ::N::two delete rec
                     lappend ::log remade
                 }}
                 trace add command ::N::two delete remake
                 namespace delete ::N",
            traced_procs("one two three four five six seven eight nine ten")
        )),
        "six four three eight seven nine five remade two one ten two"
    );
}

#[test]
fn namespace_teardown_skips_a_command_a_callback_already_deleted() {
    // The snapshot entry for `one` is gone by the time the loop reaches
    // it, and Tcl_DeleteCommandFromToken returns early rather than firing
    // a second time.
    assert_eq!(
        teardown_log(&format!(
            "{}
                 proc killone {{old new op}} {{rename ::N::one {{}}; lappend ::log killed-one}}
                 trace add command ::N::six delete killone
                 namespace delete ::N",
            traced_procs("one two three four five six seven eight nine ten")
        )),
        "one killed-one six four three eight seven nine five two ten"
    );
}

#[test]
fn namespace_teardown_places_imports_at_their_hash_positions() {
    // An imported redirect occupies an ordinary cmdTable entry, so it is
    // retired at its own hash position rather than as a separate bulk pass
    // over the namespace's imports.
    let expected = "six four three s1 eight seven s2 five two one";
    assert_eq!(
        teardown_log(
            "namespace eval S {proc s1 {} {}; proc s2 {} {}; namespace export s*}
                 namespace eval N {namespace import ::S::s1 ::S::s2}
                 foreach n {one two three four five six seven eight} {proc ::N::$n {} {}}
                 foreach n [info commands ::N::*] {trace add command $n delete rec}
                 namespace delete ::N"
        ),
        expected
    );
    // `namespace forget` deletes the entry and the re-import creates a new
    // one at the same bucket head, so the order is unchanged.
    assert_eq!(
        teardown_log(
            "namespace eval S {proc s1 {} {}; proc s2 {} {}; namespace export s*}
                 namespace eval N {
                     namespace import ::S::s1 ::S::s2
                     namespace forget ::S::s1
                     namespace import ::S::s1
                 }
                 foreach n {one two three four five six seven eight} {proc ::N::$n {} {}}
                 foreach n [info commands ::N::*] {trace add command $n delete rec}
                 namespace delete ::N"
        ),
        expected
    );
}

#[test]
fn namespace_teardown_retires_each_sources_import_tree_before_the_next_entry() {
    // Tcl_DeleteCommandFromToken walks the deleted command's ImportRef
    // list depth-first straight after its own delete trace, so a source's
    // whole import tree fires before the next cmdTable entry.
    let mut log = String::new();
    leak_free(|i| {
        let script = b"set log {}
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
                 set ::log";
        assert_eq!(
            i.eval_str(script),
            Code::Ok,
            "{}",
            String::from_utf8_lossy(&i.result_bytes())
        );
        log = String::from_utf8_lossy(&i.result_bytes()).into_owned();
    });
    assert_eq!(log, "::N::six ::N::two ::N::one ::I::one ::J::one");
}

#[test]
fn a_refused_alias_rename_keeps_the_command_tables_growth() {
    // `TclRenameCommand` creates the destination hash entry *before*
    // `TclPreventAliasLoop` and deletes it again on a refusal, so the
    // transient twelfth entry rebuilds the eleven-command table from 4
    // buckets to 16 and the grown array outlives the rejected rename.
    let eleven = "namespace eval N {}
             foreach n {one two three four five six seven eight nine ten eleven} {
                 proc ::N::$n {} {}
             }";
    let trace_and_delete = "
             foreach n [info commands ::N::*] {trace add command $n delete rec}
             namespace delete ::N";
    assert_eq!(
        teardown_log(&format!("{eleven}{trace_and_delete}")),
        "six four three eight seven nine five two one eleven ten"
    );
    let mut refusal = String::new();
    let mut alias_survives = String::new();
    leak_free(|i| {
        let recorder = b"set log {}
                 proc rec {old new op} {lappend ::log [namespace tail $old]}";
        assert_eq!(i.eval_str(recorder), Code::Ok);
        assert_eq!(i.eval_str(eleven.as_bytes()), Code::Ok);
        assert_eq!(i.eval_str(b"interp alias {} ::a {} ::N::b"), Code::Ok);
        assert_eq!(i.eval_str(b"rename ::a ::N::b"), Code::Error);
        refusal = String::from_utf8_lossy(&i.result_bytes()).into_owned();
        assert_eq!(i.eval_str(b"llength [info commands ::a]"), Code::Ok);
        alias_survives = String::from_utf8_lossy(&i.result_bytes()).into_owned();
        assert_eq!(i.eval_str(trace_and_delete.as_bytes()), Code::Ok);
        assert_eq!(i.eval_str(b"set ::log"), Code::Ok);
        assert_eq!(
            String::from_utf8_lossy(&i.result_bytes()),
            "three seven one two four eleven eight five nine six ten"
        );
        assert_eq!(i.eval_str(b"unset ::log"), Code::Ok);
    });
    assert_eq!(
        refusal,
        "cannot define or rename alias \"b\": would create a loop"
    );
    assert_eq!(alias_survives, "1");
}

#[test]
fn namespace_delete_removes_subtree() {
    leak_free(|i| {
        i.eval_str(b"namespace eval foo { proc p {} {return P} ; namespace eval bar {} }");
        assert_eq!(i.eval_str(b"namespace exists ::foo::bar"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
        assert_eq!(i.eval_str(b"namespace delete ::foo"), Code::Ok);
        // The namespace, its child, and its commands are gone.
        assert_eq!(i.eval_str(b"namespace exists ::foo"), Code::Ok);
        assert_eq!(i.result_bytes(), b"0");
        assert_eq!(i.eval_str(b"namespace exists ::foo::bar"), Code::Ok);
        assert_eq!(i.result_bytes(), b"0");
        assert_eq!(i.eval_str(b"::foo::p"), Code::Error);
        // Deleting a missing namespace errors (tclsh message).
        assert_eq!(i.eval_str(b"namespace delete ::nope"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"unknown namespace \"::nope\" in namespace delete command"
        );
    });
}

#[test]
fn mutually_entangled_deletion_preserves_the_global_namespace() {
    for version in tcl_dialect::TclVersion::ALL {
        leak_free_with(
            || native_namespace_interp(version),
            |interp| {
                assert_eq!(
                    interp.eval_str(
                        br#"namespace eval ::testing {
                            namespace eval abc {proc xyz {} {}}
                            namespace eval def {proc xyz {} {}}
                            trace add command abc::xyz delete "namespace delete ::testing::def {}; #"
                            trace add command def::xyz delete "namespace delete ::testing::abc {}; #"
                        }
                        namespace delete ::testing
                        list [namespace exists ::testing] [info commands set]"#,
                    ),
                    Code::Ok,
                    "{version:?}: {:?}",
                    interp.result_bytes(),
                );
                assert_eq!(interp.result_bytes(), b"0 set", "{version:?}");
            },
        );
    }
}

#[test]
fn child_teardown_distinguishes_cached_names_from_fresh_lookup() {
    for version in tcl_dialect::TclVersion::ALL
        .into_iter()
        .filter(|version| *version >= tcl_dialect::TclVersion::V8_5)
    {
        leak_free_with(
            || native_namespace_interp(version),
            |interp| {
                assert_eq!(
                    interp.eval_str(
                        br#"set seen {}
                        proc observe {tag old new op} {
                            set fresh ::P
                            append fresh ::C
                            lappend ::seen [list $tag [namespace exists ::P::C] \
                                [namespace exists $fresh]]
                        }
                        namespace eval ::P {
                            namespace ensemble create -command ::PE
                            namespace eval C {namespace ensemble create -command ::CE}
                        }
                        trace add command ::PE delete [list observe parent]
                        trace add command ::CE delete [list observe child]
                        namespace delete ::P
                        set seen"#,
                    ),
                    Code::Ok,
                    "{version:?}: {:?}",
                    interp.result_bytes(),
                );
                assert_eq!(
                    interp.result_bytes(),
                    b"{parent 1 1} {child 1 0}",
                    "{version:?}",
                );
            },
        );
    }
}
