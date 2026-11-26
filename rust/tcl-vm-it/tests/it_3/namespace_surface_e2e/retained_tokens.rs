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

//! A retained token is entered by identity, never by spelling: procedures, paths, renames and imports on the retained token's own tables.

use super::*;

// A retained token is entered by identity, never by spelling.

#[test]
fn a_retained_procedure_runs_in_its_own_token_not_the_recreations() {
    // `TclProcInterpProc` hands `Tcl_PushCallFrame` the procedure's own
    // `cmdPtr->nsPtr`, so a retained `::N::q` called after `::N` has been
    // recreated still runs in the retained token — the recreation's `r` is not
    // in its table, and its own table has no `r` either. Exact Tcl 9.0.4 oracle
    // result (identical on 8.6.16).
    assert_eq!(
        run(r"namespace eval N {
                 proc q {} {list [catch {r} m] $m [namespace current]}
                 proc p {} {
                     namespace delete ::N
                     namespace eval ::N {proc r {} {return NEW-R}}
                     list [q] [namespace current]
                 }
             }
             set a [::N::p]
             list $a [namespace exists ::N] [info commands ::N::*]"),
        r#"{{1 {invalid command name "r"} ::N} ::N} 1 ::N::r"#
    );
}

#[test]
fn a_relative_namespace_eval_enters_the_retained_child() {
    // `TclGetNamespaceForQualName` walks a *relative* name from the frame's own
    // namespace, so `namespace eval C` from a retained `::N` enters the
    // retained `::N::C` and republishes neither name. An *absolute*
    // `namespace eval ::N::C` is rooted at the global namespace instead, and
    // builds a fresh pair that has none of the retained commands (the name is
    // joined at run time so this measures the semantics, not tclsh's
    // `ResolvedCmdName` cache). Exact Tcl 9.0.4 oracle results (identical on
    // 8.6.16).
    assert_eq!(
        run(r"namespace eval N {
                 namespace eval C {proc x {} {return X}}
                 proc p {} {
                     namespace delete ::N
                     set r [namespace eval C {list [x] [namespace current]}]
                     list $r [namespace exists ::N] [namespace exists ::N::C] [namespace current]
                 }
             }
             set a [::N::p]
             list $a [namespace exists ::N] [namespace exists ::N::C]"),
        r"{{X ::N::C} 0 0 ::N} 0 0"
    );
    assert_eq!(
        run(r"namespace eval N {
                 namespace eval C {proc x {} {return X}}
                 proc p {} {
                     namespace delete ::N
                     set abs [join [list {} N C] ::]
                     set r [catch {namespace eval $abs {
                         list [catch {x} m] $m [namespace current]
                     }} out]
                     list $r $out [namespace exists ::N] [namespace exists ::N::C]
                 }
             }
             set a [::N::p]
             list $a [namespace exists ::N] [namespace exists ::N::C]"),
        r#"{0 {1 {invalid command name "x"} ::N::C} 1 1} 1 1"#
    );
}

#[test]
fn retained_teardown_does_not_unlink_a_recreated_namespace_path_target() {
    // The old ::N token returns only after ::U has linked its path to the new
    // ::N token. Final teardown must unlink by token, not by the shared display
    // spelling. Exact Tcl 9.0.4 result.
    assert_eq!(
        run(r"namespace eval N {
                 proc p {} {
                     namespace delete ::N
                     namespace eval ::N {proc mark {} {return NEW}}
                     namespace eval ::U {namespace path ::N}
                     list [namespace eval ::U {namespace path}] [namespace eval ::U {mark}]
                 }
             }
             set during [::N::p]
             list $during [namespace eval ::U {namespace path}] [namespace eval ::U {mark}]"),
        "{::N NEW} ::N NEW"
    );
}

#[test]
fn a_retained_token_keeps_its_namespace_path() {
    // `commandPathArray` hangs off the `Namespace`, and its entries point at
    // namespaces that are still live, so a retained token keeps resolving
    // through its own path. A retained *child* keeps its own (empty) one.
    // Exact Tcl 9.0.4 oracle results (identical on 8.6.16).
    assert_eq!(
        run(r"namespace eval L {proc lc {} {return LC}}
             namespace eval N {
                 namespace path ::L
                 proc p {} {
                     namespace delete ::N
                     list [namespace path] [catch {lc} m] $m [namespace current]
                 }
             }
             ::N::p"),
        r"::L 0 LC ::N"
    );
    assert_eq!(
        run(r"namespace eval L {proc lc {} {return LC}}
             namespace eval N {
                 namespace path ::L
                 namespace eval C {proc cx {} {list CX [namespace path]}}
                 proc p {} {
                     namespace delete ::N
                     list [namespace path] [lc] [C::cx]
                 }
             }
             ::N::p"),
        "::L LC {CX {}}"
    );
}

#[test]
fn a_live_path_keeps_an_exact_retained_target_token() {
    // A suspended coroutine retains ::B after its public namespace spelling is
    // deleted. ::U's path entry is the old namespace token, not a name to
    // resolve again. Exact Tcl 9.0.4 oracle.
    assert_eq!(
        run(r"namespace eval B {
                 proc b {} {return B}
                 coroutine ::cb apply {{} {yield ready; return done} ::B}
             }
             namespace eval U {namespace path ::B}
             namespace delete ::B
             namespace eval ::U {list [namespace path] [catch {b} m] $m}"),
        "::B 0 B"
    );
}

#[test]
fn a_retained_path_reaches_its_retained_child_target() {
    // Both the path owner and its relative target are retained by the active
    // procedure frame. Resolution walks their arena edge instead of reparsing
    // the now-unpublished display name. Exact Tcl 9.0.4 oracle.
    assert_eq!(
        run(r"namespace eval N {
                 namespace eval P {proc q {} {return OLD}}
                 namespace path P
                 proc hold {} {
                     namespace delete ::N
                     list [namespace path] [q]
                 }
             }
             ::N::hold"),
        "::N::P OLD"
    );
}

#[test]
fn finalising_a_path_target_unlinks_live_and_retained_owners() {
    // ::A and ::B are retained independently. Finishing ::B removes that exact
    // target from ::A's retained path table; finishing ::A must therefore see
    // no path and no callable `b`. Exact Tcl 9.0.4 oracle.
    assert_eq!(
        run(r"namespace eval B {
                 proc b {} {return B}
                 coroutine ::cb apply {{} {yield ready; return done} ::B}
             }
             namespace eval A {
                 namespace path ::B
                 coroutine ::ca apply {{} {
                     yield ready
                     list [namespace path] [catch {b} m] $m
                 } ::A}
             }
             namespace delete ::A
             namespace delete ::B
             set bdone [cb]
             set answer [ca]
             list $bdone $answer"),
        r#"done {{} 1 {invalid command name "b"}}"#
    );
}

#[test]
fn a_retained_token_loses_its_unknown_handler_and_keeps_its_exports() {
    // `Tcl_DeleteNamespace` frees `unknownHandlerPtr` before it looks at the
    // activation count, so an unresolvable command in the retained frame
    // reaches the interpreter default rather than the handler the namespace
    // had. Its `namespace export` patterns, which live on the `Namespace`, do
    // ride with the token — and are not the recreation's. Exact Tcl 9.0.4
    // oracle results (identical on 8.6.16).
    assert_eq!(
        run(r"proc myunk {args} {list UNK $args}
             namespace eval N {
                 namespace unknown ::myunk
                 proc p {} {
                     namespace delete ::N
                     list [namespace unknown] [catch {nosuchcommand a b} m] $m
                 }
             }
             ::N::p"),
        r#"{} 1 {invalid command name "nosuchcommand"}"#
    );
    assert_eq!(
        run(r"namespace eval N {
                 proc e1 {} {}
                 namespace export e*
                 proc p {} {
                     namespace delete ::N
                     namespace eval ::N {}
                     list [namespace export] [namespace eval ::N {namespace export}]
                 }
             }
             ::N::p"),
        "e* {}"
    );
}

#[test]
fn relative_qualified_rename_projects_the_resolved_procedure_slot() {
    // Tcl 9.0.4: `b::y` is resolved relative to the current `::a` token. The
    // relocated proc's execution namespace and public name both come from that
    // destination slot, never from the raw rename word.
    assert_eq!(
        run(r"namespace eval a {
                 namespace eval b {}
                 proc p {} {namespace current}
                 rename p b::y
                 list [info commands b::y] [info commands ::a::b::y] [b::y]
             }"),
        "::a::b::y ::a::b::y ::a::b"
    );
}

#[test]
fn retained_command_rename_mutates_the_exact_token_table() {
    // Exact Tcl 9.0.4 oracle. The active frame continues to mutate its dying
    // namespace's command table; the rename and final delete traces identify
    // that token even after its public namespace name has gone.
    assert_eq!(
        run("set log {}
             proc rec {old new op} {lappend ::log [list $old $new $op]}
             namespace eval N {
                 proc q {} {return old}
                 trace add command q {rename delete} rec
                 proc p {} {
                     namespace delete ::N
                     rename q r
                     list [r] [info commands q] [info commands r] $::log
                 }
             }
             list [::N::p] $log"),
        "{old {} r {{::N::q ::N::r rename}}} {{::N::q ::N::r rename} {::N::r {} delete}}"
    );
}

#[test]
fn retained_command_replacement_retires_the_old_token() {
    // Exact Tcl 9.0.4 oracle. A definition attempted after namespace deletion
    // runs the old binding's lifecycle but cannot publish a fresh command in
    // the dying table.
    assert_eq!(
        run("set log {}
             proc rec {old new op} {lappend ::log [list $old $new $op]}
             namespace eval N {
                 proc q {} {return old}
                 trace add command q delete rec
                 proc p {} {
                     namespace delete ::N
                     proc q {} {return new}
                     list [info commands q] [catch {q} m] $m $::log
                 }
             }
             ::N::p"),
        "{} 1 {invalid command name \"q\"} {{::N::q {} delete}}"
    );
}

#[test]
fn retained_import_query_and_forget_use_the_token_table() {
    assert_eq!(
        run("namespace eval S {proc p {} {}; namespace export p}
             namespace eval N {
                 namespace import ::S::p
                 proc hold {} {
                     namespace delete ::N
                     set before [namespace import]
                     namespace forget p
                     list $before [namespace import] [info commands p]
                 }
             }
             ::N::hold"),
        "p {} {}"
    );
}

#[test]
fn retained_namespace_rejects_new_import_publication_without_panicking() {
    assert_eq!(
        run(
            "namespace eval S {proc p {} {return imported}; namespace export p}
             namespace eval N {
                 proc hold {} {
                     namespace delete ::N
                     set c [catch {namespace import ::S::p} m]
                     list $c $m [info commands p] [catch {p} pm] $pm
                 }
             }
             ::N::hold"
        ),
        "0 {} {} 1 {invalid command name \"p\"}"
    );
}

#[test]
fn retained_import_graph_retargets_with_its_source_token() {
    assert_eq!(
        run(
            "namespace eval S {proc p {} {return old}; namespace export p}
             namespace eval N {
                 namespace import ::S::p
                 proc hold {} {
                     namespace delete ::N
                     rename ::S::p ::S::q
                     list [namespace origin p] [p]
                 }
             }
             ::N::hold"
        ),
        "::S::q old"
    );
}

#[test]
fn retained_import_graph_retires_with_its_source_token() {
    assert_eq!(
        run(
            "namespace eval S {proc p {} {return old}; namespace export p}
             namespace eval N {
                 namespace import ::S::p
                 proc hold {} {
                     namespace delete ::N
                     rename ::S::p {}
                     list [info commands p] [catch {p} m] $m
                 }
             }
             ::N::hold"
        ),
        "{} 1 {invalid command name \"p\"}"
    );
}

#[test]
fn namespace_upvar_keeps_the_resolved_retained_namespace_token() {
    // Exact Tcl 9.0.4 oracle. These two namespace paths render to the same
    // colon-run text, so flattening and reparsing the first token selects the
    // wrong live namespace.
    assert_eq!(
        run("namespace eval a: {
                 namespace eval b {variable v LEFT}
                 proc hold {} {
                     namespace delete ::a:
                     namespace upvar b v x
                     set x
                 }
             }
             namespace eval a {namespace eval :b {variable v RIGHT}}
             namespace eval a: {hold}"),
        "LEFT"
    );
}
