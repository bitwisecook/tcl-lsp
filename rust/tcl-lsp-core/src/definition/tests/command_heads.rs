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

//! Command heads at the cursor: autoload requests, aliases, ensemble subcommands and resolved indirect heads.

use super::*;

#[test]
fn logical_autoload_head_uses_the_retained_source_namespace_and_refuses_stale_heads() {
    let source = "namespace eval ::scope {proc p {} {}; p 1}";
    let analysis = logical_procedure_navigation_analysis(source);
    assert!(analysis.allows_retained_logical_declaration_advice());
    let cursor = source.rfind("p 1").unwrap();
    assert_eq!(
        command_head_and_namespace_at(source, &analysis, 0, u32::try_from(cursor).unwrap(),),
        Some(("p".to_owned(), "::scope".to_owned()))
    );
    assert!(
        command_head_and_namespace_at(
            &format!("{source} "),
            &analysis,
            0,
            u32::try_from(cursor).unwrap(),
        )
        .is_none()
    );
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert!(
        command_head_and_namespace_at(source, &missing, 0, u32::try_from(cursor).unwrap(),)
            .is_none()
    );

    let computed = "namespace eval ::scope {set head p; $head}";
    let computed_analysis = logical_procedure_navigation_analysis(computed);
    let computed_cursor = computed.rfind("$head").unwrap() + 1;
    assert!(
        command_head_and_namespace_at(
            computed,
            &computed_analysis,
            0,
            u32::try_from(computed_cursor).unwrap(),
        )
        .is_none()
    );
}

// alias resolution

#[test]
fn jump_to_alias_target_proc() {
    // `interp alias {} mycmd {} greet` aliases `mycmd` to
    // the user proc `greet`.  Cursor on `mycmd` should
    // resolve to `greet`'s definition.
    let src = "proc greet {} {}\ninterp alias {} mycmd {} greet\nmycmd\n";
    let analysis = analyse(src);
    let call_offset = u32::try_from(src.rfind("mycmd").unwrap()).unwrap();
    let reference = invocation_head_at(&analysis, call_offset)
        .and_then(|invocation| invocation.resolved_command_reference.as_ref())
        .expect("actual alias slot reference");
    assert!(reference.definition().is_none());
    assert_eq!(
        reference.linked_definition().unwrap().kind(),
        tcl_compiler::command_binding::SourceCommandDefinitionKind::Procedure
    );
    // Cursor on `mycmd` invocation on line 2.
    let locs = definition(src, 2, 2, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // The target's name span is on line 0 starting at
    // column 5 (after `proc `).
    assert_eq!(locs[0].start_line, 0);
    assert_eq!(locs[0].start_character, 5);
}

#[test]
fn alias_with_no_user_proc_returns_empty() {
    // `mycmd` aliases to a built-in (`puts`).  No user
    // proc; the provider returns empty rather than
    // pretending to know the location.
    let src = "interp alias {} mycmd {} puts\nmycmd\n";
    let analysis = analyse(src);
    let locs = definition(src, 1, 2, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn alias_lookup_accepts_qualified_form() {
    // The alias's qualified form (`::mycmd`) should also
    // resolve.
    let src = "proc greet {} {}\ninterp alias {} mycmd {} greet\n::mycmd\n";
    let analysis = analyse(src);
    let locs = definition(src, 2, 2, &analysis);
    // Whether find_word_span_at_position includes leading
    // `::` depends on the implementation; both should
    // jump to the target proc when matched.
    assert!(
        !locs.is_empty(),
        "qualified alias form should resolve to the target proc"
    );
    assert_eq!(locs[0].start_line, 0);
}

#[test]
fn ensemble_map_subcommand_jumps_to_target_proc() {
    // TP — `namespace ensemble create -map {foo ::e::Foo}` inside `::e`, then
    // `e foo bar` at the call site. Cursor on the call-site "foo" must
    // resolve to ::e::Foo's declaration.
    let src = "namespace eval ::e {\n    namespace ensemble create -map {\n        foo ::e::Foo\n    }\n}\nproc ::e::Foo {args} { return \"foo: $args\" }\n\nputs [e foo bar]\n";
    let analysis = analyse(src);
    // Cursor on "foo" in `puts [e foo bar]` (0-based line 7, col 8).
    let locs = definition(src, 7, 8, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 5);
}

#[test]
fn ensemble_map_subcommand_resolves_forward_declared_differently_cased_target() {
    // TP — the subcommand's spelling ("make") differs from the target's
    // simple name ("Make") in case, and the target is declared AFTER
    // the ensemble and the call site. A fix matching on literal name
    // equality (rather than the resolved qualified name) would fail
    // this the same way it would fail the real widget.tcl repro. The
    // call runs only after the target exists: dispatched at top level
    // before it, tclsh 8.5-9.1 stop at `invalid command name
    // "::widget::Make"`; deferred into `main`, they print `make: hello`.
    let src = "namespace eval ::widget {\n    namespace ensemble create -map {\n        make ::widget::Make\n    }\n}\nproc main {} { puts [widget make hello] }\nproc ::widget::Make {args} { return \"make: $args\" }\nmain\n";
    let analysis = analyse(src);
    let locs = definition(src, 5, 30, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 6);
}

/// A real ensemble abbreviates, so navigation on the abbreviated word
/// must land where the dispatch would. Oracle (identical on tclsh 8.6.16
/// and 9.0.4), for `namespace ensemble create -command ::e -map {foo
/// ::e::Foo}`: `e fo` returns `FOO`.
#[test]
fn ensemble_unique_prefix_subcommand_jumps_to_target_proc() {
    let src = "namespace eval ::e {\n    namespace ensemble create -map {\n        foo ::e::Foo\n    }\n}\nproc ::e::Foo {args} { return \"foo: $args\" }\n\nputs [e fo bar]\n";
    let analysis = analyse(src);
    // Cursor on the abbreviated "fo" in `puts [e fo bar]`.
    let locs = definition(src, 7, 8, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 5);
}

/// An *ambiguous* prefix is an error at runtime, so there is no target to
/// point at and navigation must keep abstaining. Oracle: with `-map {foo
/// … fob …}`, `f fo` errors `unknown or ambiguous subcommand "fo": must
/// be fob, or foo`.
#[test]
fn ensemble_ambiguous_prefix_subcommand_abstains() {
    let src = "namespace eval ::f {\n    namespace ensemble create -map {\n        foo ::f::Foo\n        fob ::f::Fob\n    }\n}\nproc ::f::Foo {args} { return F }\nproc ::f::Fob {args} { return B }\n\nputs [f fo bar]\n";
    let analysis = analyse(src);
    let locs = definition(src, 9, 8, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

/// `-prefixes 0` turns prefix matching off, so the abbreviation is a
/// plain unknown subcommand and navigation must not follow it — while the
/// exact word still resolves. Oracle: `g fo` errors `unknown subcommand
/// "fo": must be foo`; `g foo` returns `FOO`.
#[test]
fn prefixless_ensemble_resolves_only_the_exact_subcommand() {
    // tclsh 8.5-9.1 reject `g fo bar` (`unknown subcommand "fo": must be
    // foo`), so the abbreviated call is caught for the exact one to run.
    let src = "namespace eval ::g {\n    namespace ensemble create -map {\n        foo ::g::Foo\n    } -prefixes 0\n}\nproc ::g::Foo {args} { return G }\n\ncatch {g fo bar} m; puts $m\nputs [g foo bar]\n";
    let analysis = analyse(src);
    // The abbreviation must NOT resolve…
    let abbreviated = definition(src, 7, 9, &analysis);
    assert!(abbreviated.is_empty(), "{abbreviated:?}");
    // …while the exact spelling still does.
    let exact = definition(src, 8, 8, &analysis);
    assert_eq!(exact.len(), 1, "{exact:?}");
    assert_eq!(exact[0].start_line, 5);
}

#[test]
fn ensemble_subcommands_form_jumps_to_target_proc() {
    // TP — the `-subcommands` sibling form (not `-map`), the folded-in
    // secondary fix from the same research plan.
    let src = "namespace eval ::box {\n    proc show {} { return \"shown\" }\n    namespace ensemble create -subcommands {show}\n}\nbox show\n";
    let analysis = analyse(src);
    let locs = definition(src, 4, 5, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 1);
}

#[test]
fn ensemble_dynamic_map_mutation_still_abstains() {
    // TN — the correct abstention: a subcommand added only via a runtime
    // `namespace ensemble configure -map $var` dict-set mutation (never
    // a literal `-map {...}` list) is not statically resolvable —
    // `ensemble_subcommand_targets` never gets an entry for it.
    let src = "namespace eval ::widget {\n    namespace ensemble create -map {\n        make ::widget::Make\n    }\n}\nproc ::widget::addType {t} {\n    set m [namespace ensemble configure ::widget -map]\n    dict set m dyn ::widget::Dyn\n    namespace ensemble configure ::widget -map $m\n}\nproc ::widget::Dyn {args} { return dyn }\nwidget addType foo\nputs [widget dyn hello]\n";
    let analysis = analyse(src);
    // Cursor on "dyn" in `puts [widget dyn hello]` (0-based line 12).
    let locs = definition(src, 12, 13, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn two_ensembles_with_shared_subcommand_name_do_not_cross_contaminate() {
    // TN — validates the outer table is keyed per-ensemble, not a
    // single flat map (the reason `command_aliases` can't be reused
    // directly for this fix).
    let src = "namespace eval ::a {\n    namespace ensemble create -map {go ::a::Go}\n}\nnamespace eval ::b {\n    namespace ensemble create -map {go ::b::Go}\n}\nproc ::a::Go {} { return a }\nproc ::b::Go {} { return b }\nputs [a go]\nputs [b go]\n";
    let analysis = analyse(src);
    // `puts [a go]` is 0-based line 8; `puts [b go]` is line 9.
    let locs_a = definition(src, 8, 8, &analysis);
    assert_eq!(locs_a.len(), 1, "{locs_a:?}");
    assert_eq!(locs_a[0].start_line, 6);
    let locs_b = definition(src, 9, 8, &analysis);
    assert_eq!(locs_b.len(), 1, "{locs_b:?}");
    assert_eq!(locs_b[0].start_line, 7);
}

#[test]
fn ensemble_unmapped_subcommand_abstains_rather_than_guessing() {
    // A genuinely-unmapped subcommand silently abstains rather than
    // crashing or guessing. `zzz` is not a prefix of anything the
    // ensemble maps, so the ensemble itself would error here.
    let src = "namespace eval ::widget {\n    namespace ensemble create -map {\n        make ::widget::Make\n    }\n}\nproc ::widget::Make {args} {}\nputs [widget zzz hello]\n";
    let analysis = analyse(src);
    // Cursor on "zzz" in `puts [widget zzz hello]` (0-based line 6).
    let locs = definition(src, 6, 13, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn ensemble_abbreviated_subcommand_resolves_like_the_dispatch_does() {
    // `mak` is an unambiguous prefix of `make` and `-prefixes` defaults to
    // true, so tclsh dispatches it (oracle 8.6.16 / 9.0.4); navigation
    // follows by routing the lookup through
    // `tcl_cmd_core::ensemble::resolve_subcommand`.
    let src = "namespace eval ::widget {\n    namespace ensemble create -map {\n        make ::widget::Make\n    }\n}\nproc ::widget::Make {args} {}\nputs [widget mak hello]\n";
    let analysis = analyse(src);
    let locs = definition(src, 6, 13, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 5);
}

#[test]
fn tk_ensemble_configure_splice_resolves_the_subcommand_to_its_real_target_not_a_decoy() {
    // TP — `tk`'s built-in ensemble is extended at runtime via `namespace
    // ensemble configure tk -map [dict merge [namespace ensemble
    // configure tk -map] {systray ::tk::systray}]`, the real
    // `tk/library/systray.tcl` idiom. tclsh9.0/8.6 both confirm `tk
    // systray` really dispatches to `::tk::systray`, never a
    // same-tail-name decoy proc in an unrelated namespace.  Falling
    // through to `fallback_proc_by_simple_name` here lands on the decoy
    // (or, with no decoy present, abstains despite `systray ->
    // ::tk::systray` being a literal, static fact).
    let src = "namespace eval ::decoy {\n    proc systray {args} { return \"DECOY\" }\n}\nproc ::tk::systray {args} { return \"real systray: $args\" }\nnamespace ensemble configure tk -map [dict merge [namespace ensemble configure tk -map] {systray ::tk::systray}]\ntk systray create -image book\n";
    let analysis = analyse(src);
    // Cursor on "systray" in the final `tk systray create ...` call
    // (0-based line 5, "tk " is 3 chars so "systray" starts at col 3).
    let locs = definition(src, 5, 5, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // Resolves to the REAL `proc ::tk::systray` (line 3), not the
    // same-tail-name decoy inside `::decoy` (line 1).
    assert_eq!(locs[0].start_line, 3);
}

#[test]
fn ensemble_wholly_dynamic_map_value_abstains() {
    // FN (accepted) — the whole `-map` value itself is
    // a variable, not a literal list; `ensemble_subcommand_targets`
    // never gets an entry for `::widget` at all.
    let src = "namespace eval ::widget {\n    variable dynamicMap {make ::widget::Make}\n    namespace ensemble create -map $dynamicMap\n}\nproc ::widget::Make {args} {}\nputs [widget make hello]\n";
    let analysis = analyse(src);
    // Cursor on "make" in `puts [widget make hello]` (0-based line 5).
    let locs = definition(src, 5, 13, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn foreach_rename_reinstall_call_site_resolves_to_the_wrapper_not_the_stale_original() {
    // TP — the `tk/library/accessibility.tcl` shape: a literal-list
    // `foreach` renames
    // each classic widget command away and reinstalls a wrapper proc
    // under the same original name. tclsh9.0/8.6 both prove `button`
    // is the *new* wrapper proc after the loop runs — the old body is
    // reachable only via `::tk::accessible::orig_button`. Go-to
    // -definition on a `button` call made after the loop must resolve
    // to the wrapper's own declaration (inside the loop), not the
    // stale, pre-rename `proc button` at the top of the file.
    let src = "proc button {args} {return orig_button}\n\
                   proc entry {args} {return orig_entry}\n\
                   namespace eval ::tk::accessible {\n    \
                   foreach wtype {button entry} {\n        \
                   rename ::$wtype ::tk::accessible::orig_$wtype\n        \
                   proc ::$wtype {args} {return wrapped}\n    \
                   }\n\
                   }\n\
                   set r1 [button .b1]\n\
                   set r2 [entry .e1]\n";
    let analysis = analyse(src);
    // Line 8: `set r1 [button .b1]` — cursor on "button".
    let locs = definition(src, 8, 9, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // Resolves to the wrapper's own `proc ::$wtype ...` declaration
    // (line 5), not the stale original `proc button` (line 0).
    assert_eq!(locs[0].start_line, 5);
}

#[test]
fn foreach_rename_reinstall_second_element_also_resolves_to_its_own_wrapper() {
    // TP sibling — proves the fix isn't overfit to the *first* literal
    // list element: `entry` (the second element) resolves too.
    let src = "proc button {args} {return orig_button}\n\
                   proc entry {args} {return orig_entry}\n\
                   namespace eval ::tk::accessible {\n    \
                   foreach wtype {button entry} {\n        \
                   rename ::$wtype ::tk::accessible::orig_$wtype\n        \
                   proc ::$wtype {args} {return wrapped}\n    \
                   }\n\
                   }\n\
                   set r1 [button .b1]\n\
                   set r2 [entry .e1]\n";
    let analysis = analyse(src);
    // Line 9: `set r2 [entry .e1]` — cursor on "entry".
    let locs = definition(src, 9, 9, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 5);
}

// Command-position gate + resolved-indirect-head preference.  Both land on
// the same seam: the "otherwise it is a CALL" fallback, which must not hand
// whatever word the text scan returned straight to `resolve_called_proc`.

#[test]
fn fp_argument_word_sharing_a_procs_name_is_not_resolved_as_a_command() {
    // FP guard — the reduced control with zero indirection.
    // `dump` is the *first argument* of `anotherproc`, not
    // a command; tclsh never looks it up as one, so answering with the
    // same-named proc is a wrong answer, not a miss. Must abstain.
    let src = "proc anotherproc {a b} { return $a }\nproc dump {x} { return $x }\nproc caller {} {\n    return [anotherproc dump 5]\n}\n";
    let analysis = analyse(src);
    // Line 3: `    return [anotherproc dump 5]` — cursor on `dump`.
    let locs = definition(src, 3, 24, &analysis);
    assert!(
        locs.is_empty(),
        "an argument word must not resolve as a command: {locs:?}"
    );
}

#[test]
fn tp_the_head_beside_that_argument_still_resolves() {
    // TP — the very same call's real head must keep resolving, so the
    // gate above narrows nothing it should not.
    let src = "proc anotherproc {a b} { return $a }\nproc dump {x} { return $x }\nproc caller {} {\n    return [anotherproc dump 5]\n}\n";
    let analysis = analyse(src);
    // Line 3, col 12 — inside `anotherproc`.
    let locs = definition(src, 3, 12, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 0, "{locs:?}");
}

#[test]
fn tn_a_command_head_in_every_registry_known_script_argument_still_resolves() {
    // TN — the gate reads the analyser's recorded invocations, which
    // cover every registry-declared script argument. A call written in
    // an `if` body, a `switch` arm, a `foreach` body, and a command
    // -prefix callback must all still resolve.
    let src = "proc helper {} {}\nif {1} {\n    helper\n}\nswitch x {\n    a { helper }\n}\nforeach i {1} {\n    helper\n}\nlsort -command helper {1 2}\n";
    let analysis = analyse(src);
    for (line, col) in [(2, 4), (5, 8), (8, 4), (10, 15)] {
        let locs = definition(src, line, col, &analysis);
        assert_eq!(locs.len(), 1, "line {line} col {col}: {locs:?}");
        assert_eq!(locs[0].start_line, 0, "line {line} col {col}: {locs:?}");
    }
}

#[test]
fn tp_resolved_indirect_head_wins_over_a_same_named_decoy() {
    // TP — the analyser settles `set ns ::tc;
    // ${ns}::setdef` to `::tc::setdef` and records it as an `indirect`
    // invocation.  Ignoring that record and falling through to the
    // bareword lookup answers the unrelated `::other::setdef` instead.
    let src = "namespace eval ::tc { proc setdef {} { return 1 } }\nnamespace eval ::other { proc setdef {} { return 2 } }\nset ns ::tc\n${ns}::setdef\n";
    let analysis = analyse(src);
    // Line 3, col 7 — inside the `${ns}::setdef` head word.
    let locs = definition(src, 3, 7, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 0,
        "must reach ::tc::setdef (line 0), not the ::other decoy (line 1): {locs:?}"
    );
}

#[test]
fn consumed_command_names_navigate_without_becoming_execution_heads() {
    let source = "proc target {} {return OK}\ninfo args target\nrename target moved\nmoved\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
    for (written, prefix, line, column) in [
        ("info args target", "info args ", 1, 10),
        ("rename target moved", "rename ", 2, 7),
    ] {
        let offset = u32::try_from(source.find(written).unwrap() + prefix.len()).unwrap();
        let invocation = invocation_reference_at(&analysis, offset).expect("consumed name");
        assert!(!invocation.lookup.is_execution_site());
        assert!(invocation.resolved_command_reference.is_some());
        assert!(invocation_head_at(&analysis, offset).is_none());
        assert!(!offset_is_command_head(&analysis, offset));
        let locations = definition(source, line, column, &analysis);
        assert_eq!(locations.len(), 1, "{written}: {locations:?}");
        assert_eq!(locations[0].start_line, 0);
    }
    let offset = u32::try_from(source.rfind("moved").unwrap()).unwrap();
    assert!(invocation_head_at(&analysis, offset).is_some());
    assert!(offset_is_command_head(&analysis, offset));
    let locations = definition(source, 3, 1, &analysis);
    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0].start_line, 0);
}

#[test]
fn retained_loaded_indirect_definition_does_not_use_a_local_qname_decoy() {
    use std::sync::Arc;
    use tcl_compiler::command_binding::{
        SourceAnalysisEntry, SourceOriginKind, TrustedSourceModuleLoader,
    };
    let source = "proc greet {} { return local }\nsource provider.tcl\nset cmd greet\n$cmd\n";
    let entry = Arc::new(SourceAnalysisEntry {
        trusted_source_modules: vec![TrustedSourceModuleLoader::new(
            tcl_dialect::model::Family::Tcl,
            "provider.tcl".to_owned(),
            None,
            "loaded-provider".to_owned(),
            &Arc::from("proc greet {} {return loaded}"),
        )],
        invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
            tcl_dialect::TclVersion::V8_6,
        )),
        native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
            ..Default::default()
        },
        ..Default::default()
    });
    let analysis = Analyser::new()
        .with_source_analysis_entry(entry)
        .analyse(source, "tcl8.6")
        .clone();
    let invocation = analysis
        .command_invocations
        .iter()
        .find(|invocation| invocation.indirect)
        .expect("positioned indirect call");
    let offset = invocation.range.start();
    let definition = invocation
        .resolved_definition
        .as_ref()
        .expect("actual loaded definition receipt");
    assert!(matches!(
        definition.allocation().site.source.kind(),
        SourceOriginKind::Loaded { .. }
    ));
    let reference = invocation
        .resolved_command_reference
        .as_ref()
        .expect("full actual loaded reference");
    assert_eq!(reference.definition(), Some(definition));
    if let Some(linked) = reference.linked_definition() {
        assert!(
            matches!(
                linked.allocation().site.source.kind(),
                SourceOriginKind::Loaded { .. }
            ),
            "linked current definition must retain its foreign implementation: {linked:?}"
        );
    }
    assert!(resolved_indirect_head_proc(&analysis, offset).is_none());
}

#[test]
fn indirect_alias_wrapper_does_not_borrow_a_displaced_class_definition() {
    let source = "oo::class create C {}\nrename C savedClass\ninterp alias {} C {} list ALIAS\nset ns ::\n${ns}C\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
    let invocation = analysis
        .command_invocations
        .iter()
        .find(|invocation| invocation.indirect)
        .expect("retained indirect alias lookup");
    let reference = invocation
        .resolved_command_reference
        .as_ref()
        .unwrap_or_else(|| {
            panic!(
                "actual alias slot reference: {:?}",
                analysis
                    .command_invocations
                    .iter()
                    .map(|call| (
                        call.range,
                        call.name.as_str(),
                        call.resolved_qualified_name.as_deref(),
                        call.resolved_command_reference
                            .as_ref()
                            .and_then(tcl_compiler::command_binding::SourceCommandReference::slot),
                    ))
                    .collect::<Vec<_>>()
            )
        });
    assert!(reference.definition().is_none());
    assert!(reference.linked_definition().is_none());
    assert!(matches!(
        reference.binding(),
        tcl_compiler::command_binding::SourceCommandReferenceBinding::Direct {
            kind: tcl_compiler::command_binding::BindingKind::Alias,
            ..
        }
    ));
    assert!(invocation.resolved_definition.is_none());
    assert!(!invocation.resolved_user_definition);
    assert!(
        resolved_indirect_head_target(&analysis, invocation.range.start()).is_none(),
        "the alias slot has no current class definition"
    );
    assert_eq!(
        definition(source, 4, 5, &analysis),
        [] as [crate::definition::LspRange; 0]
    );
}

#[test]
fn tn_a_bare_dollar_cmd_head_still_answers_the_variable_it_reads() {
    // TN / documented limit — a whole-word `$cmd` head *is* a variable
    // read, and the position tier answers that first (it is asked long
    // before any command resolution). The indirect-head preference
    // therefore covers the spellings whose word is not itself a bare
    // variable reference — `${ns}::setdef`, a dispatch-table literal —
    // and deliberately leaves the plain `$cmd` cursor on its variable.
    let src = "proc greet {} { return hi }\nset cmd greet\n$cmd\n";
    let analysis = analyse(src);
    let locs = definition(src, 2, 1, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 1,
        "a `$cmd` cursor answers the variable declaration: {locs:?}"
    );
}

#[test]
fn fn_unresolved_indirect_head_does_not_block_the_bareword_fallback() {
    // FP guard for the preference itself — an indirect head the
    // analyser could NOT settle must not suppress an answer the
    // ordinary path can still give. Here the neighbouring plain call
    // still resolves.
    let src = "proc greet {} { return hi }\nset cmd $env(WHATEVER)\n$cmd\ngreet\n";
    let analysis = analyse(src);
    let locs = definition(src, 3, 0, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 0, "{locs:?}");
}
