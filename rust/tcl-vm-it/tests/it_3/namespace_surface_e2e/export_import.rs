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

//! `namespace export` and `namespace import`: the flag words, imported commands tracking replacement and deletion of their source, delete traces and import cycles.

use super::*;

// `namespace export` / `import` leading-option handling
// (`NamespaceExportCmd`/`Tcl_Export`, `NamespaceImportCmd`/`Tcl_Import`)

#[test]
fn export_query_form_reports_the_pattern_list() {
    // tclsh 8.6.16 / 9.0.4: `a b` (the regression that masked the -clear bug).
    assert_eq!(
        run("namespace eval e {namespace export a b}\nnamespace eval e {namespace export}"),
        "a b"
    );
    // C's `Tcl_Export` skips a pattern already present, so the list is a set.
    assert_eq!(
        run(
            "namespace eval e {namespace export a; namespace export a b}\n\
             namespace eval e {namespace export}"
        ),
        "a b"
    );
}

#[test]
fn export_clear_really_clears() {
    // tclsh 8.6.16 / 9.0.4: {}
    assert_eq!(
        run(
            "namespace eval e {namespace export a b; namespace export -clear}\n\
             namespace eval e {namespace export}"
        ),
        ""
    );
    // The behavioural vector, independent of the query form: after -clear
    // nothing is importable any more.
    assert_eq!(
        run("namespace eval d1 {proc foo {} {}; proc bar {} {}\n\
             namespace export foo bar; namespace export -clear}\n\
             namespace eval d2 {namespace import ::d1::*}\n\
             info commands ::d2::*"),
        ""
    );
    // …and a -clear followed by patterns sets exactly those.
    assert_eq!(
        run(
            "namespace eval e {namespace export a b; namespace export -clear c}\n\
             namespace eval e {namespace export}"
        ),
        "c"
    );
}

#[test]
fn only_the_first_word_is_the_export_flag() {
    // The registry pins `max_leading_option_words: Some(1)`; C tests only
    // objv[1] (tclNamesp.c:3552). tclsh 8.6.16 / 9.0.4: `-clear x`.
    assert_eq!(
        run("namespace eval e {namespace export -clear -clear x}\n\
             namespace eval e {namespace export}"),
        "-clear x"
    );
    // A pattern may not carry a namespace qualifier (`Tcl_Export`).
    assert_eq!(
        run("catch {namespace eval e {namespace export ::foo}} m; set m"),
        "invalid export pattern \"::foo\": pattern can't specify a namespace"
    );
}

#[test]
fn only_the_first_word_is_the_import_flag() {
    let setup = "namespace eval s3 {namespace export q; proc q {} {return Q}}\n";
    // A trailing `-force` is an ordinary pattern, and an unqualified pattern
    // names no source namespace (tclsh 8.6.16 / 9.0.4).
    assert_eq!(
        run(&format!(
            "{setup}namespace eval t3 {{catch {{namespace import ::s3::q -force}} m; set m}}"
        )),
        "no namespace specified in import pattern \"-force\""
    );
    // The import that preceded the bad pattern still happened, as in C.
    assert_eq!(
        run(&format!(
            "{setup}namespace eval t3 {{catch {{namespace import ::s3::q -force}}}}\n\
             info commands ::t3::*"
        )),
        "::t3::q"
    );
    // A *leading* -force is the flag.
    assert_eq!(
        run(&format!(
            "{setup}namespace eval t4 {{namespace import -force ::s3::q}}\n\
             info commands ::t4::*"
        )),
        "::t4::q"
    );
    // The introspection form lists the current namespace's imports.
    assert_eq!(
        run(&format!(
            "{setup}namespace eval t5 {{namespace import ::s3::q; namespace import}}"
        )),
        "q"
    );
}

#[test]
fn namespace_import_rejects_self_and_unknown_sources() {
    assert_eq!(
        run("namespace eval self {catch {namespace import ::self::*} m; set m}"),
        "import pattern \"::self::*\" tries to import from namespace \"self\" into itself"
    );
    assert_eq!(
        run("namespace eval dest {catch {namespace import ::nosuch::*} m; set m}"),
        "unknown namespace in import pattern \"::nosuch::*\""
    );
}

/// Configuring an ensemble through a `namespace import` alias configures the
/// ORIGIN: both spellings then observe the one config, and the alias stays an
/// alias so `namespace origin` still answers the source (tclsh 9.0.4-pinned).
#[test]
fn configuring_an_imported_ensemble_updates_the_origin() {
    let setup = "namespace eval S {namespace export ens\n\
                 proc impl {} {return ORIG}\n\
                 proc impl2 {} {return NEW}\n\
                 namespace ensemble create -command ::S::ens -map {go impl}}\n\
                 namespace eval T {namespace import ::S::ens}\n\
                 namespace eval S {namespace ensemble configure ::T::ens -map {go impl2}}\n";
    // Both spellings dispatch the new target.
    assert_eq!(run(&format!("{setup}::T::ens go")), "NEW");
    assert_eq!(run(&format!("{setup}::S::ens go")), "NEW");
    // Both spellings read back the new config.
    assert_eq!(
        run(&format!(
            "{setup}namespace ensemble configure ::T::ens -map"
        )),
        "go ::S::impl2"
    );
    assert_eq!(
        run(&format!(
            "{setup}namespace ensemble configure ::S::ens -map"
        )),
        "go ::S::impl2"
    );
    // The alias is still an alias — configuring it did not fork a new ensemble.
    assert_eq!(
        run(&format!("{setup}namespace origin ::T::ens")),
        "::S::ens"
    );
}

#[test]
fn imported_ensemble_tracks_atomic_replacement_but_not_true_deletion() {
    // Tcl_CreateObjCommand-style replacement keeps the source table token, so
    // an existing import observes the new implementation and retains its
    // provenance. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("proc tgt_old args {return OLD}
             proc tgt_new args {return NEW}
             namespace eval S {
                 namespace export E
                 namespace ensemble create -command ::S::E -map {x ::tgt_old}
             }
             namespace eval I {namespace import ::S::E}
             namespace eval S {
                 namespace ensemble create -command ::S::E -map {x ::tgt_new}
             }
             list [::I::E x] [namespace origin ::I::E] \
                  [namespace ensemble configure ::I::E -map]"),
        "NEW ::S::E {x ::tgt_new}"
    );

    // A true source deletion removes the import. Recreating an unrelated
    // command at the old source name does not resurrect it; configure and
    // invocation retain their distinct Tcl LOOKUP messages. Exact 9.0.4.
    assert_eq!(
        run("proc tgt_old args {return OLD}
             proc tgt_new args {return NEW}
             namespace eval S {
                 namespace export E
                 namespace ensemble create -command ::S::E -map {x ::tgt_old}
             }
             namespace eval I {namespace import ::S::E}
             rename ::S::E {}
             set before [list [info commands ::I::E] \
                              [namespace ensemble exists ::I::E]]
             set cc [catch {namespace ensemble configure ::I::E} cm co]
             set ic [catch {::I::E x} im io]
             namespace eval S {
                 namespace ensemble create -command ::S::E -map {x ::tgt_new}
             }
             list $before $cc $cm [dict get $co -errorcode] \
                  $ic $im [dict get $io -errorcode] \
                  [info commands ::I::E] [namespace ensemble exists ::I::E]"),
        "{{} 0} 1 {unknown command \"::I::E\"} {TCL LOOKUP COMMAND ::I::E} \
         1 {invalid command name \"::I::E\"} {TCL LOOKUP COMMAND ::I::E} {} 0"
    );
}

#[test]
fn transitive_import_tracks_an_intermediate_replacement_and_deletion() {
    // Replacing an imported binding turns that name into a real command token.
    // Its downstream imports immediately observe the new implementation and
    // origin; deleting that new source token then removes them. Exact Tcl
    // 9.0.4 oracle result.
    assert_eq!(
        run("namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval A {namespace import ::S::p; namespace export p}
             namespace eval B {namespace import ::A::p}
             proc ::A::p {} {return A}
             set before [list [::B::p] [namespace origin ::B::p]]
             rename ::A::p {}
             list $before [info commands ::B::p]"),
        "{A ::A::p} {}"
    );
}

#[test]
fn deleting_an_imported_intermediate_retires_its_downstream_imports() {
    // A::p is still an imported command token here. Deleting it retires B::p,
    // which imports that immediate token, without retiring the upstream S::p
    // implementation. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval A {namespace import ::S::p; namespace export p}
             namespace eval B {namespace import ::A::p}
             rename ::A::p {}
             list [info commands ::A::p] [info commands ::B::p] \
                  [::S::p] [namespace origin ::S::p]"),
        "{} {} S ::S::p"
    );
}

#[test]
fn import_delete_trace_can_reimport_the_same_origin_at_the_same_name() {
    // The callback creates a new imported command token whose name and origin
    // equal the dying token's. Stable binding identity, rather than provenance
    // equality alone, keeps that replacement alive. Exact Tcl 9.0.4 oracle.
    assert_eq!(
        run("namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval A {namespace import ::S::p}
             proc reimport {old new op} {
                 namespace eval ::A {namespace import -force ::S::p}
             }
             trace add command ::A::p delete reimport
             rename ::A::p {}
             list [info commands ::A::p] [::A::p] \
                  [namespace origin ::A::p]"),
        "::A::p S ::S::p"
    );
}

#[test]
fn import_delete_retires_the_exact_generation_after_trace_relocation() {
    // The delete callback can move the dying imported token. Retirement
    // follows its stable generation, including relocated trace sidecars, and
    // leaves neither spelling callable. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("proc move {old new op} {rename $old ::I::q}
             namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval I {namespace import ::S::p}
             trace add command ::I::p delete move
             rename ::S::p {}
             namespace eval I {
                 set code [catch {q} message options]
                 list [info commands ::I::p] [info commands ::I::q] \
                      $code $message [dict get $options -errorcode]
             }"),
        "{} {} 1 {invalid command name \"q\"} {TCL LOOKUP COMMAND q}"
    );

    // Hidden and exposed domains carry that same generation. The callback's
    // double relocation leaves neither a hidden token nor the exposed global
    // command behind. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("proc move {old new op} {
                 interp hide {} $old held
                 interp expose {} held q
             }
             namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval I {namespace import ::S::p}
             trace add command ::I::p delete move
             rename ::S::p {}
             set code [catch {q} message options]
             list [info commands ::I::p] [info commands ::q] \
                  [interp hidden {}] $code $message \
                  [dict get $options -errorcode]"),
        "{} {} {} 1 {invalid command name \"q\"} {TCL LOOKUP COMMAND q}"
    );
}

#[test]
fn source_deletion_refollows_generation_after_import_callbacks() {
    // The imported binding's delete callback can rename the real source while
    // its import tree is retiring. Final source unlink resolves that original
    // generation again, so the moved command cannot survive. Exact Tcl 9.0.4
    // oracle result.
    assert_eq!(
        run("set log {}
             proc cb {old new op} {
                 lappend ::log [list $old $new $op]
                 rename ::S::p ::S::q
             }
             namespace eval S {proc p {} {return OLD}; namespace export p}
             namespace eval N {
                 namespace import ::S::p
                 trace add command p delete ::cb
             }
             rename ::S::p {}
             list [info commands ::S::p] [info commands ::S::q] \
                  [catch {::S::q} m] $m [info commands ::N::p] $log"),
        "{} {} 1 {invalid command name \"::S::q\"} {} {{::N::p {} delete}}"
    );
}

#[test]
fn import_tree_refollows_parent_generation_after_child_callbacks() {
    // A transitive child's delete trace can relocate its parent import. The
    // depth-first retirement owner re-resolves the parent's generation after
    // returning from that child before unlinking it. Exact Tcl 9.0.4 oracle.
    assert_eq!(
        run("set log {}
             proc cb {old new op} {
                 lappend ::log [list $old $new $op]
                 rename ::N::p ::N::q
             }
             namespace eval S {proc p {} {return OLD}; namespace export p}
             namespace eval N {namespace import ::S::p; namespace export p}
             namespace eval M {
                 namespace import ::N::p
                 trace add command p delete ::cb
             }
             rename ::S::p {}
             list [info commands ::N::q] [catch {::N::q} m] $m $log"),
        "{} 1 {invalid command name \"::N::q\"} {{::M::p {} delete}}"
    );
}

#[test]
fn real_delete_retires_the_exact_generation_after_trace_relocation() {
    // A delete callback can rename the command token being deleted. C follows
    // that token's updated hash entry and removes the destination, while a
    // same-spelled replacement would remain a distinct generation. Exact Tcl
    // 9.0.4 oracle results for retained-visible and live-hidden locations.
    assert_eq!(
        run("set log {}
             proc cb {old new op} {
                 lappend ::log [list $old $new $op]
                 uplevel 1 {rename q r}
             }
             namespace eval N {
                 proc q {} {return OLD}
                 trace add command q delete ::cb
                 proc hold {} {
                     namespace delete ::N
                     rename q {}
                     list [info commands q] [info commands r] \
                          [catch {r} m] $m $::log
                 }
             }
             ::N::hold"),
        "{} {} 1 {invalid command name \"r\"} {{::N::q {} delete}}"
    );
    assert_eq!(
        run("set log {}
             proc cb {old new op} {
                 lappend ::log [list $old $new $op]
                 interp hide {} x held
             }
             proc x {} {return OLD}
             trace add command x delete cb
             rename x {}
             list [info commands x] [interp hidden {}] \
                  [catch {interp invokehidden {} held} m] $m $log"),
        "{} {} 1 {invalid hidden command name \"held\"} {{::x {} delete}}"
    );
}

#[test]
fn import_cascade_retires_the_relocated_generation_once() {
    // Source deletion owns the complete import tree. An imported token moved
    // by its delete callback still dies at the new location, and its trace
    // fires only once. Exact Tcl 9.0.4 oracle results for retained-visible and
    // hidden-to-visible relocation.
    assert_eq!(
        run("set log {}
             proc cb {old new op} {
                 lappend ::log [list $old $new $op]
                 uplevel 1 {rename p r}
             }
             namespace eval S {proc p {} {return old}; namespace export p}
             namespace eval N {
                 namespace import ::S::p
                 trace add command p delete ::cb
                 proc hold {} {
                     namespace delete ::N
                     rename ::S::p {}
                     list [info commands p] [info commands r] \
                          [catch {r} m] $m $::log
                 }
             }
             ::N::hold"),
        "{} {} 1 {invalid command name \"r\"} {{::N::p {} delete}}"
    );
    assert_eq!(
        run("set log {}
             proc cb {old new op} {
                 lappend ::log [list $old $new $op]
                 interp expose {} held q
             }
             namespace eval S {proc p {} {return old}; namespace export p}
             namespace import ::S::p
             trace add command p delete cb
             interp hide {} p held
             rename ::S::p {}
             list [info commands q] [interp hidden {}] [catch {q} m] $m $log"),
        "{} {} 1 {invalid command name \"q\"} {{::held {} delete}}"
    );
}

#[test]
fn imported_binding_delete_trace_runs_before_unlink_and_is_reentrant() {
    // The imported binding remains in its namespace table while its delete
    // trace runs. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set seen {}
             proc observed {old new op} {lappend ::seen [info commands $old]}
             namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval I {namespace import ::S::p}
             trace add command ::I::p delete observed
             rename ::S::p {}
             set seen"),
        "::I::p"
    );

    // A delete trace may replace the imported binding. The outer source
    // deletion must not unlink that newly-created real command. Exact Tcl
    // 9.0.4 oracle result.
    assert_eq!(
        run("proc replace {old new op} {proc $old {} {return REPLACED}}
             namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval I {namespace import ::S::p}
             trace add command ::I::p delete replace
             rename ::S::p {}
             list [::I::p] [namespace origin ::I::p]"),
        "REPLACED ::I::p"
    );
}

#[test]
fn import_delete_traces_are_depth_first_with_visible_ancestors() {
    // Deleting imported A::p traces A first, then recursively B, while both
    // bindings remain visible through B's callback. Exact Tcl 9.0.4 oracle.
    assert_eq!(
        run("set seen {}
             proc observed {old new op} {
                 lappend ::seen [list $old [info commands ::A::p] \
                                      [info commands ::B::p]]
             }
             namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval A {namespace import ::S::p; namespace export p}
             namespace eval B {namespace import ::A::p}
             trace add command ::A::p delete observed
             trace add command ::B::p delete observed
             rename ::A::p {}
             set seen"),
        "{::A::p ::A::p ::B::p} {::B::p ::A::p ::B::p}"
    );

    // A real source trace runs before its import cascade, with source/import
    // bindings still visible. Exact Tcl 9.0.4 oracle.
    assert_eq!(
        run("set seen {}
             proc observed {old new op} {
                 lappend ::seen [list $old [info commands ::S::p] \
                                      [info commands ::A::p]]
             }
             namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval A {namespace import ::S::p}
             trace add command ::S::p delete observed
             trace add command ::A::p delete observed
             rename ::S::p {}
             set seen"),
        "{::S::p ::S::p ::A::p} {::A::p ::S::p ::A::p}"
    );

    // Direct ImportRefs are linked newest-first, independent of map iteration.
    assert_eq!(
        run("set seen {}
             proc observed {old new op} {lappend ::seen $old}
             namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval A {namespace import ::S::p}
             namespace eval B {namespace import ::S::p}
             trace add command ::A::p delete observed
             trace add command ::B::p delete observed
             rename ::S::p {}
             set seen"),
        "::B::p ::A::p"
    );
}

#[test]
fn namespace_import_rejects_a_cycle_before_mutating_the_graph() {
    // Without -force, Tcl diagnoses the occupied destination before walking
    // the prospective source chain. The import graph is unchanged.
    assert_eq!(
        run("namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval A {namespace import ::S::p; namespace export p}
             namespace eval B {namespace import ::A::p; namespace export p}
             set code [catch {
                 namespace eval S {namespace import ::B::p}
             } message options]
             list $code $message [dict get $options -errorcode] \
                  [namespace origin ::S::p] [namespace origin ::A::p] \
                  [namespace origin ::B::p]"),
        "1 {can't import command \"p\": already exists} \
         {TCL IMPORT OVERWRITE} ::S::p ::S::p ::S::p"
    );

    // Tcl_Import walks the source import chain before a forced overwrite and
    // rejects an edge back to the destination token. Exact Tcl 9.0.4 oracle:
    // the error is structured and all three pre-existing bindings survive.
    assert_eq!(
        run("namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval A {namespace import ::S::p; namespace export p}
             namespace eval B {namespace import ::A::p; namespace export p}
             set code [catch {
                 namespace eval S {namespace import -force ::B::p}
             } message options]
             list $code $message [dict get $options -errorcode] \
                  [namespace origin ::S::p] [namespace origin ::A::p] \
                  [namespace origin ::B::p]"),
        "1 {import pattern \"::B::p\" would create a loop containing command \"::S::p\"} \
         {TCL IMPORT LOOP} ::S::p ::S::p ::S::p"
    );

    // Retirement is consequently finite and removes only the selected import.
    assert_eq!(
        run("namespace eval S {proc p {} {return S}; namespace export p}
             namespace eval A {namespace import ::S::p; namespace export p}
             namespace eval B {namespace import ::A::p}
             rename ::B::p {}
             list [info commands ::S::p] [info commands ::A::p] \
                  [info commands ::B::p]"),
        "::S::p ::A::p {}"
    );
}

#[test]
fn import_created_by_a_source_delete_trace_joins_the_old_token_cascade() {
    // The source remains importable during its delete trace. The resulting
    // alias references the dying token and is recursively retired afterwards.
    // Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("proc create_import {old new op} {
                 namespace eval ::A {namespace import ::S::p}
             }
             namespace eval S {proc p {} {return S}; namespace export p}
             trace add command ::S::p delete create_import
             rename ::S::p {}
             list [info commands ::S::p] [info commands ::A::p]"),
        "{} {}"
    );
}

#[test]
fn imported_ensemble_keeps_source_identity_across_hide_and_expose() {
    let setup = "proc tgt_old args {return OLD}
                 namespace eval S {
                     namespace ensemble create -command ::E -map {x ::tgt_old}
                 }
                 namespace export E
                 namespace eval I {namespace import ::E}
                 interp hide {} ::E held
                 proc ::E args {return REPLACEMENT}\n";
    // A visible same-name replacement must not capture the import whose real
    // source is the hidden token. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run(&format!(
            "{setup}list [::I::E x] [namespace origin ::I::E] \
             [namespace ensemble configure ::I::E -map] \
             [namespace ensemble exists ::I::E]"
        )),
        "OLD ::held {x ::tgt_old} 1"
    );
    assert_eq!(
        run(&format!(
            "{setup}interp expose {{}} held E2
             list [::I::E x] [namespace origin ::I::E] \
                  [namespace ensemble configure ::I::E -map] [::E2 x]"
        )),
        "OLD ::E2 {x ::tgt_old} OLD"
    );
}

/// Reconfiguring an ensemble reaches every importer in the chain, not just the
/// direct ones. C hands each import the *source's* command token, so a
/// two-hop chain (`::S::e` imported and re-exported by `::A`, imported again
/// by `::B`) observes one shared definition; the VM clones the dispatcher, so
/// the refresh has to follow provenance to its ultimate origin. Every
/// expectation below is tclsh 9.0.4- and 8.6.16-pinned.
#[test]
fn reconfiguring_an_ensemble_reaches_transitive_importers() {
    let setup = "namespace eval S {\n\
                     namespace export e\n\
                     proc impl1 {args} {return OLD}\n\
                     proc impl2 {args} {return NEW}\n\
                     namespace ensemble create -command ::S::e -map {go ::S::impl1}}\n\
                 namespace eval A {namespace import ::S::e; namespace export e}\n\
                 namespace eval B {namespace import ::A::e}\n\
                 namespace ensemble configure ::S::e -map {go ::S::impl2}\n";
    // Dispatch: all three spellings run the new target.
    assert_eq!(run(&format!("{setup}::S::e go")), "NEW");
    assert_eq!(run(&format!("{setup}::A::e go")), "NEW");
    assert_eq!(run(&format!("{setup}::B::e go")), "NEW");
    // Config query agrees with dispatch at every spelling — the pair that
    // diverged while only direct edges were refreshed.
    for spelling in ["::S::e", "::A::e", "::B::e"] {
        assert_eq!(
            run(&format!(
                "{setup}namespace ensemble configure {spelling} -map"
            )),
            "go ::S::impl2",
            "{spelling}"
        );
    }
    // Provenance is untouched: both imports still answer the original source.
    assert_eq!(run(&format!("{setup}namespace origin ::A::e")), "::S::e");
    assert_eq!(run(&format!("{setup}namespace origin ::B::e")), "::S::e");
}
