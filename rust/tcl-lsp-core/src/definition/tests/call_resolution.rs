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

//! The procedure or class a written call denotes: counted class definitions, logical procedure targets, namespace order and `apply` bodies.

use super::*;

#[test]
fn counted_class_body_effects_do_not_prove_a_later_class_binding() {
    for body in [
        "{error nope}",
        "{return}",
        "{unknown_member}",
        "{rename ::C\\uD800 {}}",
    ] {
        let source = format!("oo::class create C\\uD800 {body}\nC\\uD800 new\n");
        let analysis = analyse(&source);
        assert_eq!(
            analysis.original_counted_class_metadata.len(),
            1,
            "{source}"
        );
        let start = u32::try_from(source.find("C\\uD800").unwrap()).unwrap();
        let class = span_to_range(
            &source,
            &LineIndex::new(&source),
            tcl_lexer::Span::new(start, start + 7),
        );
        assert!(
            !definition(&source, 1, 0, &analysis).contains(&class),
            "{source}"
        );
    }
}

#[test]
fn counted_nonempty_class_definition_uses_original_name_after_move() {
    let source = "oo::class create C\\uD800 {method m {} {return body}}\nC\\uD800 new\nrename C\\uD800 moved\nmoved new\n";
    let analysis = analyse(source);
    assert_eq!(analysis.original_counted_class_metadata.len(), 1);
    let start = u32::try_from(source.find("C\\uD800").unwrap()).unwrap();
    let expected = span_to_range(
        source,
        &LineIndex::new(source),
        tcl_lexer::Span::new(start, start + 7),
    );
    assert_eq!(definition(source, 1, 0, &analysis), vec![expected]);
    assert_eq!(definition(source, 3, 0, &analysis), vec![expected]);
    assert_eq!(
        definition(&format!("#{source}"), 1, 0, &analysis),
        [] as [crate::definition::LspRange; 0]
    );
}

#[test]
fn counted_class_definition_navigation_keeps_exact_source_and_horizons() {
    let source = "oo::class create C\\uD800 {}\noo::class create C\\uD801 {}\nC\\uD800 new\nC\\uD801 new\nrename C\\uD800 moved\nmoved new\n";
    let analysis = analyse(source);
    assert_eq!(analysis.original_counted_class_metadata.len(), 2);
    let index = LineIndex::new(source);
    for (line, name) in [(2, "C\\uD800"), (3, "C\\uD801"), (5, "C\\uD800")] {
        let start = u32::try_from(source.find(name).unwrap()).unwrap();
        let span = tcl_lexer::Span::new(start, start + u32::try_from(name.len()).unwrap());
        assert_eq!(
            definition(source, line, 0, &analysis),
            vec![span_to_range(source, &index, span)]
        );
    }
    assert_eq!(
        definition(&format!("#{source}"), 2, 0, &analysis),
        [] as [crate::definition::LspRange; 0]
    );
    assert_eq!(
        definition(source, 2, 7, &analysis),
        [] as [crate::definition::LspRange; 0]
    );
}

#[test]
fn counted_class_definition_navigation_refuses_observer_shadow_and_forged_rows() {
    for source in [
        "oo::class create C\\uD800 {}\nrename C\\uD800 {}\nC\\uD800 new\n",
        "oo::class create C\\uD800 {}\nrename C\\uD800 {}\nproc C\\uD800 args {}\nC\\uD800 new\n",
        "oo::class create C\\uD800 {}\nproc change args {rename C\\uD800 {}}\ntrace add execution C\\uD800 enter change\nC\\uD800 new\n",
    ] {
        let analysis = analyse(source);
        let line = u32::try_from(source.lines().count() - 1).unwrap();
        let start = u32::try_from(source.find("C\\uD800").unwrap()).unwrap();
        let class = span_to_range(
            source,
            &LineIndex::new(source),
            tcl_lexer::Span::new(start, start + 7),
        );
        assert!(
            !definition(source, line, 0, &analysis).contains(&class),
            "{source}"
        );
    }
    let source =
        "oo::class create C\\uD800 {}\noo::class create C\\uD801 {}\nC\\uD800 new\nC\\uD801 new\n";
    let analysis = analyse(source);
    let mut duplicate = analysis.clone();
    duplicate
        .original_counted_class_metadata
        .push(duplicate.original_counted_class_metadata[0].clone());
    assert_eq!(
        definition(source, 2, 0, &duplicate),
        [] as [crate::definition::LspRange; 0]
    );
    let mut swapped = analysis.clone();
    let reference = swapped
        .command_invocations
        .iter()
        .find(|call| {
            call.range.start() == u32::try_from(source.rfind("C\\uD801 new").unwrap()).unwrap()
                && call.lookup.is_execution_site()
        })
        .unwrap()
        .resolved_command_reference
        .clone();
    assert!(reference.is_some());
    let first = swapped
        .command_invocations
        .iter_mut()
        .find(|call| {
            call.range.start() == u32::try_from(source.find("C\\uD800 new").unwrap()).unwrap()
                && call.lookup.is_execution_site()
        })
        .unwrap();
    first.resolved_command_reference = reference;
    assert_eq!(
        definition(source, 2, 0, &swapped),
        [] as [crate::definition::LspRange; 0]
    );
    let mut foreign = analysis.clone();
    foreign.original_counted_class_metadata = Analyser::new()
        .analyse(source, "tcl9.0")
        .original_counted_class_metadata;
    assert_eq!(
        definition(source, 2, 0, &foreign),
        [] as [crate::definition::LspRange; 0]
    );
}

#[test]
fn logical_proc_targets_keep_displaced_original_declaration_allocations() {
    let source = "proc repeated {first} {return $first}\nproc repeated {second} {return $second}\n";
    let mut analysis = logical_procedure_navigation_analysis(source);
    analysis.all_procs.clear();
    analysis.proc_declaration_sites.clear();
    for offset in [
        source.find("repeated").unwrap(),
        source.rfind("repeated").unwrap(),
    ] {
        let cursor = u32::try_from(offset + 1).unwrap();
        let declaration = logical_procedure_header_at(&analysis, cursor).unwrap();
        let target = resolve_proc_target_at(
            &analysis,
            source,
            cursor,
            "repeated",
            CallResolution::document_only(),
        )
        .unwrap();
        assert!(std::ptr::eq(target.1, declaration.metadata()));
        assert_eq!(target.1.name_span.start(), u32::try_from(offset).unwrap());
        assert!(std::ptr::eq(
            proc_declaration_at(&analysis, cursor).unwrap(),
            target.1
        ));
    }
}

#[test]
fn logical_proc_targets_retain_alias_move_and_redefinition_call_owners() {
    let source = "proc join {first second} {}\ninterp alias {} joined {} join captured\njoined value\nrename join archived\nproc join {new} {}\narchived alpha beta\njoin one\n";
    let mut analysis = logical_procedure_navigation_analysis(source);
    analysis.all_procs.clear();
    analysis.proc_declaration_sites.clear();
    for needle in ["joined value", "archived alpha", "join one"] {
        let start = u32::try_from(source.find(needle).unwrap()).unwrap();
        let call =
            tcl_compiler::registry_invocation::source_structure::source_logical_procedure_call_at(
                source, &analysis, start,
            )
            .unwrap();
        let header = call.declaration(&analysis).unwrap();
        let target = resolve_proc_target_at(
            &analysis,
            source,
            start + 1,
            "unrelated-display",
            CallResolution::document_only(),
        )
        .unwrap();
        assert!(std::ptr::eq(target.1, header.metadata()));
        assert!(std::ptr::eq(
            resolve_called_proc(
                &analysis,
                source,
                "::unrelated",
                "unrelated-display",
                start + 1,
                CallResolution::document_only()
            )
            .unwrap(),
            header.metadata(),
        ));
    }
}

#[test]
fn logical_simple_name_assistance_does_not_supply_call_target_authority() {
    let source = "namespace eval other {proc helper {value} {return $value}}\nhelper data\n";
    let mut analysis = logical_procedure_navigation_analysis(source);
    analysis.all_procs.clear();
    analysis.proc_declaration_sites.clear();
    let suggestion = fallback_proc_by_simple_name(&analysis, source, "helper").unwrap();
    assert_eq!(suggestion.name, "helper");
    let cursor = u32::try_from(source.rfind("helper data").unwrap() + 1).unwrap();
    assert!(
        resolve_proc_target_at(
            &analysis,
            source,
            cursor,
            "helper",
            CallResolution::document_only()
        )
        .is_none()
    );
    assert!(
        resolve_called_proc(
            &analysis,
            source,
            "::",
            "helper",
            cursor,
            CallResolution::document_only()
        )
        .is_none()
    );
    assert!(
        fallback_proc_by_simple_name(&analysis, &format!("{source}# changed"), "helper").is_none()
    );
}

#[test]
fn logical_proc_targets_refuse_deleted_dynamic_argument_and_stale_input() {
    let source = "proc helper {value} {}\nhelper data\nrename helper {}\nhelper ignored\ninterp alias {} builtin {} puts\nbuiltin text\nset data helper\n$head data\n";
    let analysis = logical_procedure_navigation_analysis(source);
    let call = u32::try_from(source.find("helper data").unwrap() + 1).unwrap();
    assert!(
        resolve_proc_target_at(
            &analysis,
            source,
            call,
            "helper",
            CallResolution::document_only()
        )
        .is_some()
    );
    for offset in [
        source.find("helper ignored").unwrap(),
        source.find("builtin text").unwrap(),
        source.find("$head data").unwrap(),
        source.find("data helper").unwrap() + 5,
    ] {
        assert!(
            resolve_proc_target_at(
                &analysis,
                source,
                u32::try_from(offset + 1).unwrap(),
                "helper",
                CallResolution::document_only()
            )
            .is_none()
        );
    }
    assert!(
        resolve_proc_target_at(
            &analysis,
            &source.replace("helper data", "helper other"),
            call,
            "helper",
            CallResolution::document_only()
        )
        .is_none()
    );
    let mut stale = analysis.clone();
    stale.resolved_input = None;
    assert!(
        resolve_proc_target_at(
            &stale,
            source,
            call,
            "helper",
            CallResolution::document_only()
        )
        .is_none()
    );
    let mut stale = analysis.clone();
    stale.body_lexer_config = None;
    assert!(
        resolve_proc_target_at(
            &stale,
            source,
            call,
            "helper",
            CallResolution::document_only()
        )
        .is_none()
    );
    let mut stale = analysis.clone();
    stale.resolved_input = Analyser::new().analyse(source, "tcl8.6").resolved_input;
    assert!(
        resolve_proc_target_at(
            &stale,
            source,
            call,
            "helper",
            CallResolution::document_only()
        )
        .is_none()
    );
}

#[test]
fn original_declaration_advice_cannot_choose_between_redefinitions() {
    // Implementation contract: naming.consumer.original-declaration-advice-ambiguity
    // docs/design/analysis/name-resolution-proofs/original-declaration-advice-ambiguity.md
    let source = "proc p {} {}\nproc p {} {}\np";
    let analysis = analyse(source);
    let offset = u32::try_from(source.rfind('p').unwrap()).unwrap();
    let invocation = invocation_head_at(&analysis, offset).expect("actual original call");
    let input = invocation
        .original_name_input
        .as_ref()
        .expect("retained operand");
    let scope = analysis
        .original_namespace_scope_at(offset)
        .expect("original namespace");
    assert_eq!(analysis.procedures_for_original_name(input, scope).len(), 2);
    assert!(proc_visible_from_namespace(&analysis, "::", "counterfactual label", offset).is_none());
}

#[test]
fn original_vendor_definition_uses_current_direct_source_headers() {
    // Implementation contract: naming.vendor.original-source-declaration-consumers
    // docs/design/analysis/name-resolution-proofs/vendor-original-source-declaration-consumers.md
    for (dialect, source, line, character) in [
        (
            "f5-iapps",
            "proc helper {argument} {return $argument}\nhelper 1",
            0,
            7,
        ),
        ("f5-iapps", r"proc {p\uD800} {} {}", 0, 9),
        ("f5-irules", "when HTTP_REQUEST {set local 1}", 0, 9),
    ] {
        let mut analysis = Analyser::new().analyse(source, dialect);
        analysis.all_procs.clear();
        analysis.global_scope.procs.clear();
        analysis.all_defined_symbols.clear();
        analysis.global_scope.defined_symbols.clear();
        let offset = byte_offset_at(&LineIndex::new(source), source, line, character);
        let std::ops::ControlFlow::Break(Some(selected)) =
            crate::vendor_declaration::select_at_offset(source, &analysis, offset)
        else {
            panic!("actual selected hosted header must retain its source card");
        };
        assert_eq!(
            definition(source, line, character, &analysis),
            vec![span_to_range(
                source,
                &LineIndex::new(source),
                selected.span()
            )]
        );
        assert_eq!(
            definition(
                &format!("# displaced\n{source}"),
                line,
                character,
                &analysis
            ),
            [] as [crate::definition::LspRange; 0]
        );
        assert!(
            analysis.original_procedure_declarations().next().is_none(),
            "source-card selection cannot create a native publication"
        );
    }
}

#[test]
fn original_proc_and_class_selection_uses_current_typed_publications() {
    // Implementation contract: naming.editor.original-declaration-wire-identity
    // docs/design/analysis/name-resolution-proofs/original-declaration-wire-identity.md
    let source = r"proc p\uD800 {} {}
proc p\uD801 {} {}
p\uD800";
    let call = u32::try_from(source.rfind(r"p\uD800").unwrap()).unwrap();
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut analysis = Analyser::new().analyse(source, dialect);
        let span = analysis
            .original_procedure_declarations()
            .next()
            .unwrap()
            .metadata()
            .name_span;
        let mut counterfeit = analysis.all_procs.values().next().unwrap().clone();
        counterfeit.name_span = tcl_lexer::Span::new(0, 1);
        analysis.all_procs.clear();
        analysis
            .all_procs
            .insert("::counterfactual".to_owned(), counterfeit);
        assert_eq!(
            resolve_called_proc(
                &analysis,
                source,
                "::counterfactual",
                "counterfactual",
                call,
                CallResolution::document_only(),
            )
            .unwrap()
            .name_span,
            span,
            "{dialect}"
        );
        assert_eq!(
            resolve_proc_target_at(
                &analysis,
                source,
                call,
                "counterfactual",
                CallResolution::document_only(),
            )
            .unwrap()
            .1
            .name_span,
            span,
            "{dialect}"
        );
        assert!(
            resolve_proc_target_at(
                &analysis,
                &format!("{source}\n# changed"),
                call,
                "counterfactual",
                CallResolution::document_only(),
            )
            .is_none()
        );
        let config = analysis.body_lexer_config.as_mut().unwrap();
        config.strict_quoting = !config.strict_quoting;
        assert!(
            resolve_proc_target_at(
                &analysis,
                source,
                call,
                "counterfactual",
                CallResolution::document_only(),
            )
            .is_none()
        );
    }
    assert_original_class_publications();
}

// namespace-aware proc resolution (C Tcl `Tcl_FindCommand` order)

#[test]
fn unqualified_call_resolves_in_callers_namespace_first() {
    // Two namespaces each define `helper`; the unqualified call inside
    // ::b must land on ::b::helper — C Tcl resolves the current
    // namespace before global, never a sibling namespace.
    let src = "namespace eval a {\n    proc helper {} { return 1 }\n}\nnamespace eval b {\n    proc helper {} { return 2 }\n    helper\n}\n";
    let analysis = analyse(src);
    // Cursor on the bare `helper` call (line 5).
    let locs = definition(src, 5, 6, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 4, "must resolve to ::b::helper");
    assert_eq!(locs[0].start_character, 9);
}

// apply namespace override: a bareword called
// inside `apply {{params} body ns}`'s body resolves against `ns`, not
// wherever the `apply` call is lexically written.

#[test]
fn bareword_inside_apply_body_resolves_against_the_lambdas_own_namespace() {
    // TP — the finding's own literal repro: `real` and `lexical` both
    // define `cleanup`; `apply {{} {cleanup done} ::real}` must
    // resolve the bareword `cleanup` inside its body to `::real`'s
    // copy, not `::lexical`'s (tclsh9.0/8.6-verified: prints "real
    // done").  A lexical-nearest-definition guess answers
    // `::lexical::cleanup` instead.
    let src = "namespace eval real {\n    proc cleanup {tag} { puts \"real $tag\" }\n}\nnamespace eval lexical {\n    proc cleanup {tag} { puts \"lexical $tag\" }\n}\napply {{} {cleanup done} ::real}\n";
    let analysis = analyse(src);
    // Cursor on `cleanup` inside the apply body (line 6, col 11).
    let locs = definition(src, 6, 11, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 1,
        "must resolve to real::cleanup (line 1), not lexical::cleanup (line 4): {locs:?}"
    );
    assert_eq!(locs[0].start_character, 9);
}

#[test]
fn bareword_inside_var_indirected_apply_body_resolves_against_the_lambdas_namespace() {
    // TP — one-hop `$var` indirection to a literal lambda triple:
    // `set lambda {{} {cleanup done} ::real}` then `apply $lambda`.
    // tclsh9.0/8.6-verified: prints "real done". Exercises
    // `resolve_dynamic_apply_lambda`'s `TokenType::Var` branch.
    let src = "namespace eval real {\n    proc cleanup {tag} { puts \"real $tag\" }\n}\nnamespace eval lexical {\n    proc cleanup {tag} { puts \"lexical $tag\" }\n}\nset lambda {{} {cleanup done} ::real}\napply $lambda\n";
    let analysis = analyse(src);
    // Cursor on `cleanup` inside the `set`'s braced value (line 6, col 16).
    let locs = definition(src, 6, 16, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 1,
        "must resolve to real::cleanup (line 1), not lexical::cleanup (line 4): {locs:?}"
    );
}

#[test]
fn bareword_inside_list_and_qualified_var_apply_resolves_against_the_lambdas_namespace() {
    // Original list components retain the qualified scalar value and
    // the lambda's independent namespace.
    let src = "namespace eval real {\n    proc cleanup {tag} { puts \"real $tag\" }\n}\nnamespace eval lexical {\n    proc cleanup {tag} { puts \"lexical $tag\" }\n    set body {cleanup done}\n}\napply [list {} $lexical::body ::real]\n";
    let analysis = analyse(src);
    // Cursor on `cleanup` inside `set body {cleanup done}` (line 5, col 14).
    let locs = definition(src, 5, 14, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 1,
        "must resolve to real::cleanup (line 1), not lexical::cleanup (line 4): {locs:?}"
    );
}

#[test]
fn two_hop_apply_var_indirection_stays_unresolved() {
    // FN — deliberately out of scope: `set a $lambda; apply $a` is an
    // extra level of $var-to-$var forwarding beyond the one hop
    // `resolve_dynamic_apply_lambda` follows from `apply`'s own
    // argument (`set a $lambda`'s value is itself a `$`-prefixed Var
    // token, which `set`'s binding never records as a constant
    // string at all, so `apply $a` can't resolve even one hop back to
    // the literal lambda). Two conflicting `cleanup` procs, matching
    // the other scenarios' shape, so a wrong resolution would be
    // visible rather than accidentally right via a single-candidate
    // fallback. It stays unresolved — a documented depth bound.
    let src = "namespace eval real {\n    proc cleanup {tag} { puts \"real $tag\" }\n}\nnamespace eval lexical {\n    proc cleanup {tag} { puts \"lexical $tag\" }\n}\nset lambda {{} {cleanup done} ::real}\nset a $lambda\napply $a\n";
    let analysis = analyse(src);
    // Cursor on `cleanup` inside the `set lambda`'s braced value (line 6, col 16).
    let locs = definition(src, 6, 16, &analysis);
    // Since `apply $a` never resolves (not even one hop), this text is
    // never walked as a script body at all, so `namespace_overrides`
    // gets no entry for it — whatever answer comes back is from the
    // SAME pre-existing, coincidental lexical-nearest fallback that
    // would fire with no `apply`/`namespace_overrides` mechanism in
    // the picture at all (confirmed: resolves to lexical::cleanup,
    // line 4 — the nearer declaration, not real::cleanup on line 1).
    // The one assertion that actually matters: the override must NOT
    // have kicked in and pointed this at real::cleanup.
    assert!(
        locs.iter().all(|l| l.start_line != 1),
        "two-hop indirection is a deliberate depth bound — must not resolve via the \
             override to real::cleanup: {locs:?}"
    );
}

#[test]
fn apply_namespace_matching_lexical_nesting_still_resolves() {
    // TN — the "control" shape: apply's namespace override happens to
    // equal its lexical home. Must resolve correctly both before and
    // after the fix (before: coincidentally, via the lexical fallback;
    // after: genuinely, via the override) — proves the fix doesn't
    // disturb the already-working common case.
    let src = "namespace eval app {\n    proc cleanup {tag} { puts $tag }\n    apply {{} {cleanup done} ::app}\n}\n";
    let analysis = analyse(src);
    // Cursor on `cleanup` inside the apply body (line 2, col 15).
    let locs = definition(src, 2, 15, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 1);
}

#[test]
fn apply_namespace_override_does_not_leak_past_its_own_span() {
    // FP guard — a bareword with the SAME text, immediately outside
    // the apply body's span, must resolve exactly as it would with no
    // apply call in the file at all (i.e. to the lexically-nearest
    // `lexical::cleanup`, not real::cleanup). Catches an off-by-one in
    // the recorded span or a "last override wins globally" bug.
    let src = "namespace eval real {\n    proc cleanup {tag} { puts $tag }\n}\nnamespace eval lexical {\n    proc cleanup {tag} { puts $tag }\n    apply {{} {cleanup done} ::real}\n    cleanup done\n}\n";
    let analysis = analyse(src);
    // Cursor on the second `cleanup` (line 6, the bare call OUTSIDE the
    // apply body, col 4).
    let locs = definition(src, 6, 4, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 4,
        "must resolve to lexical::cleanup, the override must not leak past its span: {locs:?}"
    );
}

#[test]
fn two_apply_calls_with_different_namespace_overrides_never_cross_resolve() {
    // FP guard — two independent, non-overlapping `apply` calls with
    // different namespace overrides in the same file must each
    // resolve to their own namespace, never the other's (catches an
    // implementation that only tracks a single override, or picks the
    // wrong entry when more than one exists).
    let src = "namespace eval a {\n    proc cleanup {tag} { puts $tag }\n}\nnamespace eval b {\n    proc cleanup {tag} { puts $tag }\n}\napply {{} {cleanup 1} ::a}\napply {{} {cleanup 2} ::b}\n";
    let analysis = analyse(src);
    // First apply body's `cleanup` (line 6, col 11) -> ::a::cleanup (line 1).
    let locs_a = definition(src, 6, 11, &analysis);
    assert_eq!(locs_a.len(), 1, "{locs_a:?}");
    assert_eq!(locs_a[0].start_line, 1);
    // Second apply body's `cleanup` (line 7, col 11) -> ::b::cleanup (line 4).
    let locs_b = definition(src, 7, 11, &analysis);
    assert_eq!(locs_b.len(), 1, "{locs_b:?}");
    assert_eq!(locs_b[0].start_line, 4);
}

#[test]
fn unqualified_call_at_global_prefers_global_proc() {
    // A global ::helper exists alongside ::a::helper; a global-scope
    // call resolves the global proc (the only candidate C Tcl tries).
    let src = "namespace eval a {\n    proc helper {} { return 1 }\n}\nproc helper {} { return 0 }\nhelper\n";
    let analysis = analyse(src);
    let locs = definition(src, 4, 2, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 3, "must resolve to ::helper");
}

#[test]
fn global_call_with_only_namespaced_procs_falls_back_deterministically() {
    // No ::helper exists, so no candidate resolves. tclsh 8.4-9.1 and
    // jimsh answer `invalid command name "helper"`, so neither
    // `::z::helper` nor `::a::helper` is the target, on every repeat.
    let src = "namespace eval z {\n    proc helper {} { return 26 }\n}\nnamespace eval a {\n    proc helper {} { return 1 }\n}\nhelper\n";
    let analysis = analyse(src);
    for attempt in 0..8 {
        let locs = definition(src, 6, 2, &analysis);
        assert!(
            locs.is_empty(),
            "attempt {attempt}: no namespaced tail may answer: {locs:?}"
        );
    }
}

#[test]
fn builtin_named_namespace_proc_resolves_inside_its_namespace_only() {
    // `proc set` inside ::ns shadows the builtin *within* the namespace;
    // a global-scope `set` reaches the builtin, so no user-proc jump.
    let src = "namespace eval ns {\n    proc set {key value} { return $value }\n    set x 1\n}\nset y 2\n";
    let analysis = analyse(src);
    // Inside the namespace: the proc wins.
    let inside = definition(src, 2, 5, &analysis);
    assert_eq!(inside.len(), 1, "{inside:?}");
    assert_eq!(inside[0].start_line, 1);
    // At global scope the builtin wins — no definition to jump to.
    let global = definition(src, 4, 1, &analysis);
    assert!(global.is_empty(), "{global:?}");
}

#[test]
fn nested_shadow_of_builtin_never_outranks_the_builtin() {
    // TP — the "rename the builtin away, install a same-named shadow,
    // restore it" idiom (`rest.tcl`'s `describe`, tcllib): a `proc ::set
    // {...}` written *inside* another proc's body exists only
    // conditionally (when and if the enclosing proc runs), and is
    // renamed back off again before the enclosing proc returns. A call
    // site that runs strictly after the enclosing proc returns is
    // unambiguously the real builtin — the shadow definition existing
    // *somewhere in the file* must not make it win.
    let src = "proc outer {} {\n    rename ::set ::_set\n    proc ::set {var val} { ::_set $var $val }\n    rename ::set {}\n    rename ::_set ::set\n}\nouter\nset final restored\n";
    let analysis = analyse(src);
    // `set` on the last line, well after `outer` (and its nested shadow)
    // has returned.
    let after_restore = definition(src, 7, 0, &analysis);
    assert!(
        after_restore.is_empty(),
        "a call after the shadow is restored must resolve to the \
             builtin (no jump target), not the nested shadow: {after_restore:?}",
    );
}

#[test]
fn ordinary_nested_proc_still_resolves_when_not_shadowing_a_builtin() {
    // TN — the nested-shadow gate is scoped to names that collide with a
    // real registry builtin; an ordinary nested helper proc (no such
    // collision) still resolves.
    let src = "proc outer {} {\n    proc helper {} { return 1 }\n    return [helper]\n}\n";
    let analysis = analyse(src);
    let locs = definition(src, 2, 13, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 1,
        "must still resolve to the nested helper"
    );
}

#[test]
fn top_level_builtin_override_still_resolves_to_itself() {
    // FN guard — a proc that shadows a builtin at *file* (top-level)
    // scope is not a conditional nested definition; it unconditionally
    // replaces the builtin for the rest of the file, exactly as real
    // Tcl's `proc puts {args} {...}` would, and must keep resolving to
    // itself rather than being caught by the nested-shadow gate.
    let src = "proc puts {args} { return }\nputs hello\n";
    let analysis = analyse(src);
    let locs = definition(src, 1, 0, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 0,
        "must resolve to the top-level override"
    );
}

#[test]
fn qualified_relative_call_prefers_current_namespace_then_global() {
    // `sub::p` written inside ::ns resolves ::ns::sub::p before
    // ::sub::p; the absolute `::sub::p` resolves exactly (confirmed
    // against tclsh — see `bareword_resolution_candidates`).
    let src = "namespace eval ns {\n    namespace eval sub {\n        proc p {} { return 1 }\n    }\n    sub::p\n}\nnamespace eval sub {\n    proc p {} { return 2 }\n}\n::sub::p\n";
    let analysis = analyse(src);
    let relative = definition(src, 4, 5, &analysis);
    assert_eq!(relative.len(), 1, "{relative:?}");
    assert_eq!(relative[0].start_line, 2, "must prefer ::ns::sub::p");
    let absolute = definition(src, 9, 4, &analysis);
    assert_eq!(absolute.len(), 1, "{absolute:?}");
    assert_eq!(absolute[0].start_line, 7, "::sub::p is absolute");
}

#[test]
fn jump_to_proc_definition_in_two_level_nested_namespace_via_qualified_call() {
    // Go-to-definition on a fully-qualified call to a proc
    // nested two `namespace eval` levels deep must land on its own decl.
    let src = concat!(
        "namespace eval modelTestVerTool {\n",
        "    namespace eval gui {\n",
        "        proc specAddButtonPopUp {x y} { return \"$x $y\" }\n",
        "    }\n",
        "}\n",
        "::modelTestVerTool::gui::specAddButtonPopUp 1 2\n",
    );
    let analysis = analyse(src);
    // Cursor on the qualified call (line 5).
    let locs = definition(src, 5, 30, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 2, "should land on the decl");
}

#[test]
fn jump_to_proc_definition_disambiguates_same_named_procs_by_cursor() {
    // Two procs share the simple name `helper` in different two-level
    // nested namespaces. Go-to-definition from each one's own
    // declaration (a no-op jump, but exercises the lookup) must resolve
    // to *that* proc, not whichever `all_procs` entry a HashMap happens
    // to iterate first.
    let src = concat!(
        "namespace eval a {\n",
        "    namespace eval b {\n",
        "        proc helper {} { return \"a-b\" }\n",
        "    }\n",
        "}\n",
        "namespace eval c {\n",
        "    namespace eval d {\n",
        "        proc helper {} { return \"c-d\" }\n",
        "    }\n",
        "}\n",
    );
    let analysis = analyse(src);
    // Cursor on ::a::b::helper's own decl (line 2).
    let locs_ab = definition(src, 2, 14, &analysis);
    assert_eq!(locs_ab.len(), 1, "{locs_ab:?}");
    assert_eq!(locs_ab[0].start_line, 2, "must resolve to ::a::b::helper");
    // Cursor on ::c::d::helper's own decl (line 7).
    let locs_cd = definition(src, 7, 14, &analysis);
    assert_eq!(locs_cd.len(), 1, "{locs_cd:?}");
    assert_eq!(locs_cd[0].start_line, 7, "must resolve to ::c::d::helper");
}

#[test]
fn jump_to_class_definition_disambiguates_same_named_classes_by_cursor() {
    // Class analogue of the proc test above.
    let src = concat!(
        "namespace eval a {\n",
        "    namespace eval b {\n",
        "        oo::class create Widget {}\n",
        "    }\n",
        "}\n",
        "namespace eval c {\n",
        "    namespace eval d {\n",
        "        oo::class create Widget {}\n",
        "    }\n",
        "}\n",
    );
    let analysis = analyse(src);
    assert!(
        analysis.all_classes.len() >= 2,
        "both positive class definitions must be present in the analysis"
    );
    let locs_ab = definition(src, 2, 26, &analysis);
    assert_eq!(locs_ab.len(), 1, "{locs_ab:?}");
    assert_eq!(locs_ab[0].start_line, 2, "must resolve to ::a::b::Widget");
    let locs_cd = definition(src, 7, 26, &analysis);
    assert_eq!(locs_cd.len(), 1, "{locs_cd:?}");
    assert_eq!(locs_cd[0].start_line, 7, "must resolve to ::c::d::Widget");
}

// tcl::OptProc — the `opt` package's automatic-option-parsing proc
// definer: a call site must resolve to the real
// `tcl::OptProc` declaration, never a stale stub.

#[test]
fn opt_proc_call_site_resolves_to_its_declaration() {
    // tclsh 8.4.20-9.1.0: ::tcl::OptProc needs `package require opt`.
    let src = "package require opt; ::tcl::OptProc greet {child -use -display} { return $child }\ngreet foo\n";
    let analysis = analyse(src);
    // Line 1: `greet foo` — cursor on "greet".
    let locs = definition(src, 1, 0, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 0, "{locs:?}");
    assert_eq!(locs[0].start_character, 15, "{locs:?}");
}
