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
use std::fmt::Write as _;
use tcl_compiler::analyser::Analyser;

fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl8.6").clone()
}

fn logical_procedure_navigation_analysis(source: &str) -> AnalysisResult {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name)
}

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
fn logical_definition_calls_retain_original_alias_move_and_redefinition_headers() {
    let source = "proc join {first second} {}\ninterp alias {} joined {} join captured\njoined value\nrename join archived\nproc join {new} {}\narchived alpha beta\njoin one\n";
    let mut analysis = logical_procedure_navigation_analysis(source);
    assert!(analysis.allows_retained_logical_declaration_advice());
    analysis.all_procs.clear();
    analysis.proc_declaration_sites.clear();
    for (needle, expected_line) in [("joined value", 0), ("archived alpha", 0), ("join one", 4)] {
        let offset = u32::try_from(source.find(needle).unwrap() + 1).unwrap();
        let position = LineIndex::new(source).position_at_utf16(offset, source);
        let call =
            tcl_compiler::registry_invocation::source_structure::source_logical_procedure_call_at(
                source,
                &analysis,
                u32::try_from(source.find(needle).unwrap()).unwrap(),
            )
            .expect("genuine original call survives reporting-map removal");
        assert_eq!(
            call.declaration(&analysis)
                .unwrap()
                .metadata()
                .name_span
                .start(),
            if expected_line == 0 {
                5
            } else {
                u32::try_from(source.find("proc join {new}").unwrap() + 5).unwrap()
            }
        );
        let ranges = definition(source, position.line, position.character.get(), &analysis);
        assert_eq!(
            ranges
                .iter()
                .map(|range| range.start_line)
                .collect::<Vec<_>>(),
            vec![expected_line]
        );
    }
    let offset = u32::try_from(source.find("proc join").unwrap() + 5).unwrap();
    assert_eq!(
        procedure_name_at_cursor(&analysis, offset).unwrap().start(),
        offset
    );
}

#[test]
fn logical_definition_call_refusals_do_not_resurrect_report_procedure_names() {
    let source = "proc helper {value} {}\nrename helper {}\nhelper ignored\ninterp alias {} builtin {} puts\nbuiltin text\nset data helper\n$head data\n";
    let analysis = logical_procedure_navigation_analysis(source);
    for needle in ["helper ignored", "builtin text", "$head data"] {
        let offset = u32::try_from(source.find(needle).unwrap()).unwrap();
        assert!(
            tcl_compiler::registry_invocation::source_structure::source_logical_procedure_call_at(
                source, &analysis, offset,
            )
            .is_none()
        );
        let position = LineIndex::new(source).position_at_utf16(offset + 1, source);
        assert_eq!(
            definition(source, position.line, position.character.get(), &analysis),
            [] as [crate::definition::LspRange; 0]
        );
    }
    let offset = u32::try_from(source.find("data helper").unwrap() + 5).unwrap();
    assert!(
        indirect_definition_target(
            &analysis,
            CallResolution::document_only(),
            source,
            offset,
            "helper"
        )
        .is_none()
    );
    let changed = source.replace("helper ignored", "helper altered");
    assert_eq!(
        definition(&changed, 2, 1, &analysis),
        [] as [crate::definition::LspRange; 0]
    );
}

#[test]
fn logical_definition_call_requires_current_complete_analysis_input() {
    let source = "proc helper {value} {}\nhelper data\n";
    let analysis = logical_procedure_navigation_analysis(source);
    assert_eq!(definition(source, 1, 1, &analysis).len(), 1);
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert_eq!(
        definition(source, 1, 1, &missing),
        [] as [crate::definition::LspRange; 0]
    );
    let mut foreign = analysis.clone();
    foreign.resolved_input = Analyser::new().analyse(source, "tcl8.6").resolved_input;
    assert_eq!(
        definition(source, 1, 1, &foreign),
        [] as [crate::definition::LspRange; 0]
    );
}

fn external_mask_at(
    source: &str,
    analysis: &AnalysisResult,
    selector: &str,
) -> ExternalObjectDispatchMask {
    let cursor = u32::try_from(source.rfind(selector).unwrap()).unwrap();
    let position = LineIndex::new(source).position_at_utf16(cursor, source);
    object_dispatch_mask(analysis, source, position.line, position.character.get())
}

#[test]
fn original_external_visibility_keeps_variable_and_object_names_separate_from_self_dispatch() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    for (construction, receiver) in [
        ("C create my", "my"),
        ("set my [C new]", "$my"),
        ("C create {[self]}", "{[self]}"),
    ] {
        for (method, expected) in [
            ("Hidden", ExternalObjectDispatchMask::Native),
            ("visible", ExternalObjectDispatchMask::Clear),
        ] {
            let source = format!(
                "oo::class create C {{}}; {construction}; oo::objdefine {receiver} {{method {method} {{}} {{}}}}; {receiver} {method}"
            );
            let mut analysis = Analyser::new().analyse(&source, "tcl8.6");
            assert!(
                analysis.original_object_configurations().next().is_some(),
                "{source}"
            );
            analysis.object_member_state.clear();
            analysis.dialect = "presentation label".into();
            assert_eq!(
                external_mask_at(&source, &analysis, method),
                expected,
                "{source}"
            );
            assert_eq!(expected.withholds_workspace_fallback(), method == "Hidden");
        }
    }
}

#[test]
fn original_internal_visibility_uses_the_selected_receiver_local_head() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let source = "proc my args {return DECOY}; oo::class create C {method Hidden {} {}; method run {} {my Hidden}}; C create obj; obj run";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let offset = u32::try_from(source.find("my Hidden").unwrap()).unwrap();
    let selected = analysis
        .retained_command_realm()
        .unwrap()
        .invocation_at_source("my", offset);
    assert!(
        selected
            .receiver_self_method_entry(analysis.resolved_registry().unwrap())
            .is_some()
    );
    assert_eq!(
        external_mask_at(source, &analysis, "Hidden}"),
        ExternalObjectDispatchMask::Clear
    );
    assert!(!external_mask_at(source, &analysis, "Hidden}").withholds_workspace_fallback());
}

#[test]
fn original_external_self_visibility_requires_its_own_entry_after_internal_dispatch() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    // Measured EXTERNAL_SELF_UNEXPORTED in native_method_visibility/v2
    // rejects on C8.6/C9. UNEXPORT_FULL_INTERNAL uses actual my dispatch.
    // C9 EXTERNAL_SELF_PRIVATE is a separate declaring-class scope rule,
    // not evidence that an unexported entry is externally callable.
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for (head, method, expected) in [
            ("my", "Hidden", ExternalObjectDispatchMask::Clear),
            ("[self]", "Hidden", ExternalObjectDispatchMask::Native),
            ("[self]", "visible", ExternalObjectDispatchMask::Clear),
        ] {
            let source = format!(
                "oo::class create C {{method {method} {{}} {{}}; method invoke {{}} {{{head} {method}}}}}; C create c; c invoke"
            );
            let analysis = Analyser::new().analyse(&source, dialect);
            let call = format!("{head} {method}");
            let offset = u32::try_from(source.find(&call).unwrap()).unwrap();
            let binding = analysis
                .retained_command_realm()
                .unwrap()
                .invocation_at_source(head, offset);
            if head == "my" {
                assert!(
                    binding
                        .receiver_self_method_entry(analysis.resolved_registry().unwrap())
                        .is_some()
                );
            } else {
                let selected = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
                    &call,
                    offset,
                    analysis.body_lexer_config.unwrap(),
                )
                .into_iter()
                .next()
                .unwrap();
                let tokens = tcl_compiler::ir::CommandTokens::from_segmented(
                    &tcl_lexer::SourceMap::new(&source),
                    analysis.body_lexer_config.unwrap(),
                    &selected,
                );
                let (_, entry, _) = binding
                    .frozen_object_receiver_method_entry(tokens.word_exprs.first().unwrap())
                    .expect("the reached self head retains an independent external method entry");
                assert_eq!(entry.is_exported(), method == "visible");
            }
            assert_eq!(
                external_mask_at(&source, &analysis, &format!("{method}}}")),
                expected,
                "{dialect}: {source}"
            );
        }
        let deferred = "oo::class create C {method Hidden {} {}; method invoke {} {[self] Hidden}}";
        let analysis = Analyser::new().analyse(deferred, dialect);
        assert_eq!(
            external_mask_at(deferred, &analysis, "Hidden}}"),
            ExternalObjectDispatchMask::Unavailable,
            "source self identity cannot donate a missing actual method entry"
        );
    }
}

#[test]
fn original_source_visibility_keeps_full_lexer_input_and_conditional_mask_purpose() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let source = "\u{feff}oo::class create C {method m {} {}}; set my [C new]; oo::objdefine $my {unexport m}; $my m";
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let config = tcl_lexer::LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        ..tcl_lexer::LexerConfig::for_profile(Some(profile))
    };
    let input =
        tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
    let mut analysis = Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(source, "presentation label");
    assert_eq!(analysis.body_lexer_config, Some(config));
    assert!(
        analysis
            .object_member_state
            .get("my")
            .is_some_and(|states| { states.iter().any(|state| state.unexports.contains("m")) })
    );
    assert!(
        analysis.original_object_configurations().next().is_none(),
        "source advice cannot invent a Native allocation"
    );
    assert_eq!(
        external_mask_at(source, &analysis, "m"),
        ExternalObjectDispatchMask::ConditionalSource
    );
    analysis.dialect = "jim".into();
    assert_eq!(
        external_mask_at(source, &analysis, "m"),
        ExternalObjectDispatchMask::ConditionalSource
    );
    let other_namespace = "oo::class create C {method m {} {}}; set my [C new]; oo::objdefine $my {unexport m}; namespace eval other {set my [C new]; $my m}";
    let other = Analyser::new()
        .with_resolved_input(input)
        .analyse(other_namespace, "presentation label");
    assert!(other.object_member_state.contains_key("my"));
    let mask = external_mask_at(other_namespace, &other, "m}");
    assert_eq!(mask, ExternalObjectDispatchMask::Unavailable);
    assert!(mask.withholds_workspace_fallback());
}

#[test]
fn original_external_visibility_withholds_populated_unavailable_stale_and_foreign_input() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let source = "oo::class create C {}; C create obj; oo::objdefine obj {method visible {} {return VISIBLE}}; obj visible";
    let baseline = Analyser::new().analyse(source, "tcl8.6");
    assert!(baseline.original_object_configurations().next().is_some());
    assert_eq!(
        external_mask_at(source, &baseline, "visible"),
        ExternalObjectDispatchMask::Clear
    );
    let input = baseline.resolved_input.as_ref().unwrap();
    let older =
        tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry();
    let older_same_store = std::sync::Arc::new(
        older.with_command_store(
            input
                .context_registry()
                .commands()
                .snapshot()
                .shared_registry(),
        ),
    );
    for unavailable in vec![
        {
            let mut next = baseline.clone();
            next.resolved_input = None;
            next
        },
        {
            let mut next = baseline.clone();
            next.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
                environment: "tcl8.6".into(),
                overlay: 999,
            });
            next
        },
        {
            let mut next = baseline.clone();
            next.body_lexer_config.as_mut().unwrap().strict_quoting =
                !input.lexer_config().strict_quoting;
            next
        },
        {
            let mut next = baseline.clone();
            next.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                older_same_store,
                input.lexer_config(),
            ));
            next
        },
        {
            let mut next = baseline.clone();
            next.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                older,
                input.lexer_config(),
            ));
            next
        },
    ]
    .into_boxed_slice()
    {
        assert!(
            unavailable
                .original_object_configurations()
                .next()
                .is_some()
        );
        let mask = external_mask_at(source, &unavailable, "visible");
        assert_eq!(mask, ExternalObjectDispatchMask::Unavailable);
        assert!(mask.withholds_workspace_fallback());
    }
    let changed = source.replace("return VISIBLE", "return CHANGED");
    let mask = external_mask_at(&changed, &baseline, "visible");
    assert_eq!(mask, ExternalObjectDispatchMask::Unavailable);
    assert!(mask.withholds_workspace_fallback());
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

fn assert_original_class_publications() {
    let source = r"oo::class create c\uD800 {}
oo::class create c\uD801 {}
c\uD800";
    let call = u32::try_from(source.rfind(r"c\uD800").unwrap()).unwrap();
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut analysis = Analyser::new().analyse(source, dialect);
        let span = analysis
            .original_class_declarations()
            .next()
            .unwrap()
            .metadata()
            .name_span;
        let mut counterfeit = analysis.all_classes.values().next().unwrap().clone();
        counterfeit.name_span = tcl_lexer::Span::new(0, 1);
        analysis.all_classes.clear();
        analysis
            .all_classes
            .insert("::counterfactual".to_owned(), counterfeit);
        assert_eq!(
            resolve_class_target_at(
                &analysis,
                source,
                CallResolution::document_only(),
                call,
                "counterfactual",
            )
            .unwrap()
            .1
            .name_span,
            span,
            "{dialect}"
        );
        assert!(
            resolve_class_target_at(
                &analysis,
                &format!("{source}\n# changed"),
                CallResolution::document_only(),
                call,
                "counterfactual",
            )
            .is_none()
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

#[test]
fn constructed_colon_namespace_context_survives_editor_projection() {
    let source = "namespace eval : {namespace eval : {list HERE}}";
    let offset = u32::try_from(source.find("list HERE").unwrap()).unwrap();
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(source, dialect);
        assert_eq!(
            namespace_context_at(&analysis.global_scope, offset, &[]),
            "::::::",
            "{dialect}"
        );
    }
    for namespace in ["::", ":::", "::a:::b", "::::::"] {
        let overrides = [(tcl_lexer::Span::new(0, 100), namespace.to_owned())];
        assert_eq!(
            namespace_context_at(&tcl_compiler::analyser::Scope::global(), 1, &overrides),
            namespace
        );
    }
}

/// The document-only resolution context: no registry, no whole-program
/// export oracle — what a single-document test resolves against.
const DOC: CallResolution<'static> = CallResolution::document_only();

/// Scope extent and source-declaration membership must handle nested
/// namespace scopes without overflowing. 80 nested `namespace eval`
/// levels is past the point (confirmed empirically: 100+) where
/// unguarded namespace-scope recursion overflows `cargo test`'s bare
/// ~2 MiB per-test default. The assertion is that both return at all,
/// not what they return.
#[test]
fn deeply_nested_namespaces_survive_scope_extent_and_alias_scans() {
    const DEPTH: usize = 80;
    let mut source = String::new();
    for i in 0..DEPTH {
        let _ = writeln!(source, "namespace eval ns{i} {{");
    }
    source.push_str("set x 1\nputs $x\n");
    for _ in 0..DEPTH {
        source.push_str("}\n");
    }
    // The analyser runs on the server's worker stack; the scans under
    // test keep the default test stack.
    let analysed = source.clone();
    let analysis = crate::on_worker_stack(move || analyse(&analysed));
    let _ = variable_scope_extent(&analysis, "x", 0);
    let mut scopes = vec![&analysis.global_scope];
    while let Some(scope) = scopes.pop() {
        if let Some(var_def) = scope.variables.values().next() {
            let _ = linked_var_reference_spans(
                &source,
                &analysis,
                var_def,
                var_def.definition_span.start(),
            );
            break;
        }
        scopes.extend(&scope.children);
    }
    let _ = cross_namespace_qualified_vars(&analysis.global_scope, &FxHashSet::default());
}

#[test]
fn jump_to_proc_definition() {
    let src = "proc greet {} {}\ngreet\n";
    let analysis = analyse(src);
    // Cursor on the second `greet` (line 1, char 2).
    let locs = definition(src, 1, 2, &analysis);
    assert_eq!(locs.len(), 1);
    // The proc name span is on line 0 starting at column 5.
    assert_eq!(locs[0].start_line, 0);
    assert_eq!(locs[0].start_character, 5);
}

// foreach multi-list lock-step form: `foreach
// varList1 list1 varList2 list2 ... body` binds every varList, not
// just the first.

#[test]
fn multi_list_foreach_name_resolves_to_its_own_loop_not_an_unrelated_later_one() {
    // The failure mode on the real pix corpus file (docs/pixdoc.tcl) — a
    // *second* `foreach` reusing the bare name `name` (a wholly
    // unrelated, later loop) wins whenever it is the only `VarDef` named
    // `name` in the flat top-level scope, which happens if the first
    // loop's own `name` (the second varList of a `foreach dirName
    // {...} name {...} {...}` multi-list form) is not bound at
    // all. tclsh9.0/8.6 both prove `name` inside the first loop's body
    // is that loop's own variable, never reaching the second loop.
    let src = "foreach dirName {src src {src core}} name {alpha beta gamma} {\n    puts \"$dirName $name\"\n    if {$name eq \"pixutils\"} { puts skip }\n}\nforeach name {examples color changes} {\n    puts $name.ruff\n}\n";
    let analysis = analyse(src);
    // Cursor on the first loop's own `$name` read (line 1, col 21).
    let locs = definition(src, 1, 21, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 0,
        "must resolve to the first loop's own `name` clause (line 0), not the unrelated second loop (line 4): {locs:?}"
    );
    assert_eq!(locs[0].start_character, 37);

    // Same wrong-location bug reproduced from a second usage inside
    // the same loop body (line 2, `if {$name eq ...}`).
    let locs2 = definition(src, 2, 10, &analysis);
    assert_eq!(locs2.len(), 1, "{locs2:?}");
    assert_eq!(locs2[0].start_line, 0, "{locs2:?}");
}

#[test]
fn definition_resolves_a_method_return_captured_dispatch() {
    // `instance_classes` never binds `b` from a method return, so
    // go-to-definition on the consuming `$b greet` has only the lattice's
    // scope-keyed singleton to resolve through — the same accessor every
    // dispatch consumer reads.
    let src = "oo::class create A { method make {} { ::return [::B new] } }\n\
                   oo::class create B { method greet {} { ::return \"hi\" } }\n\
                   set a [A new]\n\
                   set b [$a make]\n\
                   $b greet\n";
    let analysis = analyse(src);
    // Cursor on `greet` at the `$b greet` call site (line 4, col 4).
    let locs = definition(src, 4, 4, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 1,
        "must resolve to ::B::greet's declaration: {locs:?}"
    );
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

// wildcard namespace import bareword resolution:
// `namespace import NS::*` makes every command `NS` has `namespace
// export`ed callable bare wherever the import is in scope — but real
// Tcl only imports *exported* names (`Tcl_Export`), so an unexported
// sibling in `NS` stays unreachable through the import.

#[test]
fn wildcard_namespace_import_resolves_exported_proc() {
    // TP — single command in `Foo`, exported, reached only through the
    // wildcard import (tclsh9.0/8.6-verified: `bar` prints 1).
    let src = "namespace eval Foo {\n    proc bar {} { return 1 }\n    namespace export bar\n}\nnamespace import ::Foo::*\nbar\n";
    let analysis = analyse(src);
    // Cursor on the bareword `bar` call (line 5, col 0).
    let locs = definition(src, 5, 0, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 1,
        "must resolve to Foo::bar's declaration"
    );
    assert_eq!(locs[0].start_character, 9);
}

#[test]
fn wildcard_namespace_import_resolves_exported_proc_alongside_unexported_sibling() {
    // TP — the finding's own headline shape: `Foo` has both an exported
    // `bar` and an unrelated, unexported `other`. Sibling count must not
    // enter into it: there is no "ambiguity threshold" in real Tcl.
    let src = "namespace eval Foo {\n    proc bar {} { return 1 }\n    proc other {} { return 2 }\n    namespace export bar\n}\nnamespace import ::Foo::*\nbar\n";
    let analysis = analyse(src);
    // Cursor on the bareword `bar` call (line 6, col 0).
    let locs = definition(src, 6, 0, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 1,
        "must resolve to Foo::bar's declaration"
    );
    assert_eq!(locs[0].start_character, 9);
}

#[test]
fn wildcard_namespace_import_unexported_sibling_stays_unresolved() {
    // FP guard (CRITICAL) — `other` lives in `Foo` but is never
    // exported, so real Tcl's `namespace import ::Foo::*` never binds
    // it: a bare `other` call errors "invalid command name" at runtime
    // (Tcl manual, `namespace` — "namespace import" only imports
    // exported names). The wildcard-import resolver must not
    // manufacture a resolution the interpreter itself would reject.
    //
    // Exercises `proc_visible_via_wildcard_import` directly rather than
    // the full `definition()` provider: `resolve_called_proc`'s own
    // pre-existing, unrelated lenient fallback
    // (`fallback_proc_by_simple_name`, "any proc with this simple name,
    // anywhere") would otherwise also resolve a bare `other` call
    // regardless of import/export state — a longstanding, documented
    // leniency for procs whose defining namespace isn't statically
    // visible, not the gate this test is proving.
    let src = "namespace eval Foo {\n    proc bar {} { return 1 }\n    proc other {} { return 2 }\n    namespace export bar\n}\nnamespace import ::Foo::*\nother\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "::", "other", u32::MAX);
    assert!(
        hit.is_none(),
        "an unexported sibling must stay unresolved through the wildcard import: {hit:?}"
    );
}

#[test]
fn wildcard_namespace_import_resolves_exported_class() {
    // TP — the class analogue of
    // `wildcard_namespace_import_resolves_exported_proc`:
    // `resolve_class_target_at` is the fixed function directly (the
    // top-level `definition()` provider's own bareword-class match is
    // namespace-blind already and would not exercise this fix).
    let src = "namespace eval Foo {\n    oo::class create Widget {}\n    namespace export Widget\n}\nnamespace import ::Foo::*\nWidget new\n";
    let analysis = analyse(src);
    let line_index = LineIndex::new(src);
    let cursor_off = byte_offset_at(&line_index, src, 5, 0);
    let hit = resolve_class_target_at(&analysis, src, DOC, cursor_off, "Widget");
    assert!(
        hit.is_some(),
        "must resolve Widget through the wildcard import"
    );
    let (qname, _) = hit.unwrap();
    assert_eq!(qname, "::Foo::Widget");
}

#[test]
fn wildcard_namespace_import_unexported_sibling_class_stays_unresolved() {
    // FP guard — the class analogue: `Other` lives in `Foo` but is
    // never exported, so it must stay unresolved through the wildcard
    // import exactly like the proc case.
    let src = "namespace eval Foo {\n    oo::class create Widget {}\n    oo::class create Other {}\n    namespace export Widget\n}\nnamespace import ::Foo::*\nOther new\n";
    let analysis = analyse(src);
    let line_index = LineIndex::new(src);
    let cursor_off = byte_offset_at(&line_index, src, 6, 0);
    let hit = resolve_class_target_at(&analysis, src, DOC, cursor_off, "Other");
    assert!(
        hit.is_none(),
        "an unexported sibling class must stay unresolved: {hit:?}"
    );
}

#[test]
fn wildcard_namespace_import_wins_over_an_unrelated_decoy_with_the_same_simple_name() {
    // TP — a bareword call reached via a wildcard import must consult
    // `namespace_imports`: falling straight to
    // `fallback_proc_by_simple_name`'s lenient "any proc with this simple
    // name, prefer lexicographically smallest qualified name" tie-break
    // silently resolves (and hovers) to an entirely unrelated,
    // never-imported decoy
    // (`::decoyns::helper`, "d" < "r") instead of the real,
    // oracle-proven target (`::realns::helper`, tclsh9.0/8.6-verified:
    // prints `REAL`). The wildcard-import resolution is tried *before*
    // that fallback, so it must win here.
    let src = "namespace eval realns {\n    namespace export helper\n    proc helper {} { return REAL }\n}\nnamespace eval decoyns {\n    proc helper {} { return DECOY }\n}\nnamespace eval userns {\n    namespace import ::realns::*\n    proc run {} {\n        return [helper]\n    }\n}\n";
    let analysis = analyse(src);
    // Cursor on the bareword `helper` call inside `userns::run` (line
    // 10, col 16 — `        return [helper]`).
    let locs = definition(src, 10, 16, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 2,
        "must resolve to realns::helper (the exported, imported target), \
             not the decoy: {locs:?}"
    );
}

// Per-import-site export snapshots. `namespace import`
// binds the names exported *when it runs*; the export list keeps
// changing afterwards and none of it reaches back. Every case below is
// oracle-pinned against tclsh 8.6.14 and 9.0.4.
//
// These drive `proc_visible_via_wildcard_import` rather than the full
// `definition()` provider for the same reason
// `wildcard_namespace_import_unexported_sibling_stays_unresolved` does:
// `resolve_called_proc`'s unrelated lenient `fallback_proc_by_simple_name`
// would answer either way and would prove nothing about this gate.

#[test]
fn import_site_snapshot_survives_a_later_export_clear() {
    // TP, direction A — `namespace export -clear` after the
    // import does NOT revoke the alias the import already installed.
    // Oracle: `namespace eval ::src {proc p {} {return P}; namespace
    // export p}; namespace eval ::dst {namespace import ::src::*};
    // namespace eval ::src {namespace export -clear}; ::dst::p` → `P`,
    // and `info commands ::dst::*` still lists `::dst::p`.
    let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval src {\n    namespace export -clear\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::p"),
        "a `-clear` written after the import must not revoke it: {hit:?}"
    );
}

#[test]
fn import_site_snapshot_ignores_a_later_export() {
    // FP guard (CRITICAL), direction B — exporting a name
    // *after* an import does not add it retroactively. Oracle:
    // `namespace eval ::src {proc p {} {return P}}; namespace eval ::dst
    // {namespace import ::src::*}; namespace eval ::src {namespace export
    // p}; ::dst::p` → `invalid command name "::dst::p"`.
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return P }\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval src {\n    namespace export p\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_none(),
        "an export written after the import must not apply retroactively: {hit:?}"
    );
}

#[test]
fn a_global_rooted_import_is_export_gated_in_document_too() {
    // TN / TP pair — `namespace import ::p` reads as an *empty* source
    // namespace, which is easy to skip and so bypass the gate. Oracle
    // (8.6.14 /
    // 9.0.4): without `namespace export p` at global level the import is a
    // silent no-op and `info commands ::dst::*` stays empty; with it,
    // `::dst::p` binds.
    let ungated =
        "proc p {} { return GLOBAL }\nnamespace eval dst {\n    namespace import ::p\n}\n";
    let analysis = analyse(ungated);
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX).is_none(),
        "an unexported global command installs nothing",
    );
    let gated = "proc p {} { return GLOBAL }\nnamespace export p\nnamespace eval dst {\n    namespace import ::p\n}\n";
    let analysis = analyse(gated);
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX)
            .is_some_and(|p| p.qualified_name == "::p"),
        "with the export it binds",
    );
}

// The same gate, as the **provider** answers it.  Everything above drives
// `proc_visible_via_wildcard_import` directly; these pin that
// `resolve_called_proc`'s lenient tail match does not overrule it.

#[test]
fn single_file_definition_refuses_direction_b_through_the_fallback() {
    // FP guard (CRITICAL) — the whole Direction-B fixture in one
    // document. Oracle (8.6.14 / 9.0.4): the bare call is `invalid command
    // name "p"` and `info commands ::dst::*` is empty, so go-to-definition
    // must not answer `::src::p` through `fallback_proc_by_simple_name`.
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return P }\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval src {\n    namespace export p\n}\nnamespace eval dst {\n    p\n}\n";
    let analysis = analyse(src);
    let locs = definition(src, 10, 4, &analysis);
    assert!(
        locs.is_empty(),
        "the lenient tail match must not overrule the import gate: {locs:?}"
    );
}

#[test]
fn single_file_definition_still_answers_when_the_import_is_live() {
    // TP — the same fixture with the export written *before* the import
    // must still resolve, or the tightening has simply broken the feature.
    let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    p\n}\n";
    let analysis = analyse(src);
    let locs = definition(src, 8, 4, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 1);
}

#[test]
fn the_tail_match_still_answers_where_no_import_names_that_namespace() {
    // Nothing imports `::other` and `::dst` has no path to it: tclsh
    // 8.4-9.1 and jimsh answer `invalid command name "helper"`, so a
    // same-file tail match must not answer.
    let src = "namespace eval other {\n    proc helper {} { return H }\n}\nnamespace eval dst {\n    helper\n}\n";
    let analysis = analyse(src);
    let locs = definition(src, 4, 4, &analysis);
    assert!(
        locs.is_empty(),
        "an unrelated same-file tail match must not answer: {locs:?}"
    );
}

#[test]
fn the_tail_match_still_answers_when_a_gated_import_names_another_namespace() {
    // A dead import of `::src` says nothing about `::other`, and nothing
    // else makes `::other::helper2` visible from `::dst`: tclsh 8.4-9.1
    // answer `invalid command name "helper2"`.
    let src = "namespace eval src {\n    proc helper {} { return S }\n}\nnamespace eval other {\n    proc helper2 {} { return O }\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    helper2\n}\n";
    let analysis = analyse(src);
    let locs = definition(src, 10, 4, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn the_tail_match_refuses_a_forgotten_import_too() {
    // TN — the same tightening covers every way the route can be dead,
    // not just the export snapshot: after `namespace forget ::src::p` the
    // bare call is `invalid command name` (oracle on `alias_live_at`), so
    // the fallback must not answer either.
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    namespace forget ::src::p\n}\nnamespace eval dst {\n    p\n}\n";
    let analysis = analyse(src);
    let locs = definition(src, 11, 4, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn the_tail_match_refuses_a_call_written_before_its_import() {
    // TN — the call-site ordering reaches the provider through the same
    // door: the call above the import resolves
    // nowhere, the one below it resolves. The early call is caught, as
    // tclsh 8.4-9.1 otherwise stop at `invalid command name "p"` before
    // the import runs (with the catch they print `P` for the later call).
    let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    catch {p}\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    p\n}\n";
    let analysis = analyse(src);
    assert!(
        definition(src, 5, 11, &analysis).is_empty(),
        "the call before the import must not resolve",
    );
    assert_eq!(
        definition(src, 11, 4, &analysis).len(),
        1,
        "the call after it must",
    );
}

#[test]
fn a_second_import_after_a_later_export_does_pick_the_name_up() {
    // TP — each import takes its own snapshot, so the *second* import,
    // written after the export, binds `q` even though the first could
    // not. Oracle: `namespace eval ::src {proc p …; proc q …; namespace
    // export p}; namespace eval ::dst {namespace import ::src::*};
    // namespace eval ::src {namespace export -clear p q}; namespace eval
    // ::dst {namespace import ::src::*}` → both `::dst::p` and
    // `::dst::q` run.
    let src = "namespace eval src {\n    proc p {} { return P }\n    proc q {} { return Q }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval src {\n    namespace export -clear p q\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    for name in ["p", "q"] {
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", name, u32::MAX);
        assert!(
            hit.is_some(),
            "the second import must see `{name}`: {hit:?}"
        );
    }
}

#[test]
fn export_clear_before_the_import_does_revoke() {
    // TN — the ordering gate is real in both directions: a `-clear`
    // written *before* the import leaves nothing for it to bind.
    // Oracle: after `namespace export -clear`, `namespace export`
    // returns the empty list, and the later import binds no command.
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return P }\n    namespace export p\n    namespace export -clear\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_none(),
        "a `-clear` before the import leaves nothing exported: {hit:?}"
    );
}

#[test]
fn export_clear_with_patterns_on_the_same_call_keeps_only_the_new_ones() {
    // TP + TN — `namespace export a b; namespace export -clear p` leaves
    // exactly `p` exported (oracle: `namespace export` → `p`), so an
    // import after it binds `p` and not `a`.
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc a {} { return A }\n    proc p {} { return P }\n    namespace export a\n    namespace export -clear p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX).is_some(),
        "the `-clear` call's own pattern survives it"
    );
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "a", u32::MAX).is_none(),
        "the pattern cleared by the same call must not survive"
    );
}

#[test]
fn separate_export_calls_are_additive() {
    // TP — `namespace export a` then `namespace export b` exports both
    // (oracle: `namespace export` → `a b`); `export` appends, it does
    // not replace. A snapshot model that kept only the newest call's
    // patterns would lose `a`.
    let src = "namespace eval src {\n    proc a {} { return A }\n    proc b {} { return B }\n    namespace export a\n    namespace export b\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    for name in ["a", "b"] {
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "dst", name, u32::MAX).is_some(),
            "`{name}` must stay exported — `namespace export` is additive"
        );
    }
}

#[test]
fn export_glob_patterns_use_tcl_glob_semantics() {
    // TP + TN — export patterns are glob *patterns*: oracle `namespace
    // export get*` with `getX`/`setX` in the namespace imports only
    // `getX` (`::dst::setX` → `invalid command name`).
    let src = "namespace eval src {\n    proc getX {} { return GX }\n    proc setX {} { return SX }\n    namespace export get*\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    assert!(proc_visible_via_wildcard_import(&analysis, DOC, "dst", "getX", u32::MAX).is_some());
    assert!(proc_visible_via_wildcard_import(&analysis, DOC, "dst", "setX", u32::MAX).is_none());
}

#[test]
fn an_export_written_before_the_proc_still_exports_it() {
    // TP — an export pattern is a *name pattern*, not a reference to a
    // command: oracle `namespace eval ::src {namespace export p; proc p
    // {} {return P}}` then importing `::src::*` yields a working
    // `::dst::p`. Nothing in the snapshot may require the proc to have
    // been defined by the time the export ran.
    let src = "namespace eval src {\n    namespace export p\n    proc p {} { return P }\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX).is_some(),
        "`namespace export` names a pattern, not an existing command"
    );
}

#[test]
fn an_import_inside_a_body_sees_a_later_top_level_export() {
    // TP — the same-document half of the ordering rule the workspace tier
    // must match (pinned on both sides so they cannot drift): the import
    // runs only when `setup` is called, by which
    // time the whole file — including the `namespace export` written below
    // it — has loaded. Oracle (tclsh 8.6.14 / 9.0.4): `::app::setup;
    // ::app::run` → `HELP`.
    //
    // `indirection::in_effect` states exactly that; a plain offset compare
    // does not.
    let src = "namespace eval src {\n    proc p {} { return P }\n}\nnamespace eval dst {\n    proc setup {} { namespace import ::src::* }\n}\nnamespace eval src {\n    namespace export p\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::p"),
        "a body-local import observes a top-level export written later in \
             the same file: {hit:?}"
    );
}

#[test]
fn an_export_in_the_importing_body_itself_stays_ordered() {
    // FN guard for the leniency above: the "whole file loads first"
    // exception does not extend to a statement of the *same* body, where
    // the offsets are in genuine execution order. Oracle: `proc setup {}
    // {namespace import ::src::*; namespace eval ::src {namespace export
    // p}}` then `::dst::setup` leaves `::dst::p` an invalid command name.
    let src = "namespace eval src {\n    proc p {} { return P }\n}\nnamespace eval dst {\n    proc setup {} { namespace import ::src::* ; namespace eval ::src { namespace export p } }\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_none(),
        "an export written after the import in the same body has not run \
             yet: {hit:?}"
    );
}

#[test]
fn a_command_named_like_the_export_flag_is_exportable() {
    // TP — the user-visible consequence of consuming only one `-clear`:
    // `namespace export -clear -clear`
    // exports a command genuinely *named* `-clear`, and a wildcard import
    // then binds it. Oracle (tclsh 8.6.14 / 9.0.4): `namespace export`
    // returns `-clear`, and `info commands ::dst::*` lists `::dst::-clear`.
    // Consuming both words as flags would drop the export entirely.
    let src = "namespace eval src {\n    proc -clear {} { return DC }\n    namespace export -clear -clear\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "-clear", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::-clear"),
        "the second `-clear` is an export pattern, not a second flag: {hit:?}"
    );
}

#[test]
fn exact_namespace_import_resolves_source_proc_in_document() {
    // FP guard / bonus TP — a plain (non-glob) `namespace import
    // ::Foo::bar` must keep working through the same in-document path
    // (the wildcard resolver's glob match degrades to exact equality
    // for a literal pattern), matching the already-passing
    // cross-document `namespace_import_call_site_references_the_source_command`
    // (`tcl-lsp-core::workspace_index`) — this exercises the same-file
    // case that resolver never covers.
    let src = "namespace eval Foo {\n    proc bar {} { return 1 }\n    namespace export bar\n}\nnamespace import ::Foo::bar\nbar\n";
    let analysis = analyse(src);
    let locs = definition(src, 5, 0, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 1,
        "must resolve to Foo::bar's declaration"
    );
}

// The import edge's own lifecycle.
//
// The import edge is a *snapshot* fact; these pin it as a **link
// with a lifetime**. Every case is oracle-pinned against tclsh 9.0.4 and
// 8.6.14, byte-identical on both, and drives
// `proc_visible_via_wildcard_import` / `proc_visible_via_forced_import`
// directly for the same reason the export-snapshot block above does: the lenient
// `fallback_proc_by_simple_name` in `resolve_called_proc` would answer
// either way and prove nothing about this gate.

/// Byte offset just past the last occurrence of `needle`.
fn after_last(src: &str, needle: &str) -> u32 {
    u32::try_from(src.rfind(needle).expect("needle present") + needle.len())
        .expect("tiny test source")
}

/// Byte offset of the first occurrence of `needle`.
fn at_first(src: &str, needle: &str) -> u32 {
    u32::try_from(src.find(needle).expect("needle present")).expect("tiny test source")
}

/// Byte offset of the head word of the last call written on its own
/// indented line as `    word`. A call-site query sits on the call's own
/// head word, which is what the original declaration selection reads
/// (test-input correction, user decision 2026-10-18T16:00Z
/// `forced_import_sibling_offsets`; fixtures and expected values are
/// unchanged).
fn last_call_head(src: &str, word: &str) -> u32 {
    let line = format!("    {word}\n");
    u32::try_from(src.rfind(&line).expect("call line present") + 4).expect("tiny test source")
}

// Behaviour 1 — `namespace forget`.

#[test]
fn a_forget_after_the_import_stops_resolving_the_alias() {
    // TN (the bug): oracle
    //   namespace eval ::src { proc p {} {return P}; namespace export p }
    //   namespace eval ::dst { namespace import ::src::* }
    //   namespace eval ::dst { p }                        → P
    //   namespace eval ::dst { namespace forget ::src::p }
    //   namespace eval ::dst { p }                        → invalid command name "p"
    //   info commands ::dst::*                            → (empty)
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    namespace forget ::src::p\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_none(),
        "a forgotten alias must stop resolving after the forget: {hit:?}"
    );
}

#[test]
fn a_call_before_the_forget_still_resolves() {
    // TP — the forget is order-gated, not file-wide: a call written
    // between the import and the forget still reaches `::src::p`.
    let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\np\nnamespace eval dst {\n    namespace forget ::src::p\n}\n";
    let analysis = analyse(src);
    let call = at_first(src, "\np\n") + 1;
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::p"),
        "a call before the forget is unaffected by it: {hit:?}"
    );
}

#[test]
fn a_glob_forget_removes_every_matching_alias() {
    // TN — oracle: `namespace forget ::src::*` empties `info commands
    // ::dst::*` entirely.
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return P }\n    proc q {} { return Q }\n    namespace export *\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    namespace forget ::src::*\n}\n";
    let analysis = analyse(src);
    for name in ["p", "q"] {
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "dst", name, u32::MAX).is_none(),
            "`{name}` must be forgotten by the glob pattern"
        );
    }
}

/// FN guard: a **dynamic** forget pattern
/// revokes nothing, so the alias keeps resolving.
///
/// `namespace forget ::src::$name` really does remove the alias when
/// `$name` is `p` (oracle for the literal spelling is on
/// `a_forget_after_the_import_stops_resolving_the_alias`) — but
/// nothing static says what `$name` holds, and the pattern is a *glob*, so
/// a guess would have to be either "revokes `p`" or "revokes everything".
/// Both silently drop references the program still has, which is the one
/// direction this family never takes: a removal that cannot be proved is
/// not applied. The literal prefix (`::src::`) narrows the *source* but
/// not the name, so it decides nothing on its own.
///
/// The scanner-side twin of this abstention is
/// `signature_scan::handlers::handle_namespace_forget_skips_dynamic_patterns`;
/// this pins the consequence a consumer actually sees.
#[test]
fn a_dynamic_forget_pattern_revokes_nothing() {
    let dynamic = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    namespace forget ::src::$name\n}\n";
    let analysis = analyse(dynamic);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::p"),
        "a dynamic forget pattern must not revoke on a guess: {hit:?}"
    );
    // Control — the same source with the pattern written literally does
    // revoke, so the difference is the substitution and nothing else.
    let literal = dynamic.replace("::src::$name", "::src::p");
    let analysis = analyse(&literal);
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX).is_none(),
        "the literal spelling of the same forget still revokes"
    );
}

/// FN guard for a dynamic export pattern — the export-side twin of
/// [`a_dynamic_forget_pattern_revokes_nothing`], and the same sign
/// argument.
///
/// `namespace export $pat` really does export whatever `$pat` expands to,
/// but nothing static says what that is and the word is a *glob*. The
/// scanner therefore records no pattern at all
/// (`Analyser::handle_namespace_export` skips a word containing `$` or
/// `[`), so the import takes a snapshot that does not cover the name and
/// the resolver abstains. That is the safe direction here precisely
/// because the sign is inverted from the forget case: a *guessed export*
/// can only add names an import never took, inventing a definition the
/// program has no path to.
///
/// The abstention costs a real answer, which is why it is pinned rather
/// than merely documented — a future change that starts guessing here
/// will fail this test, not silently start resolving.
#[test]
fn a_dynamic_export_pattern_exports_nothing() {
    let dynamic = "set pat p\nnamespace eval src {\n    proc p {} { return P }\n    namespace export $pat\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(dynamic);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_none(),
        "a dynamic export pattern must not be guessed into a covering export: {hit:?}"
    );
    // Control — the same source with the pattern written literally does
    // export, so the difference is the substitution and nothing else.
    let literal = dynamic.replace("namespace export $pat", "namespace export p");
    let analysis = analyse(&literal);
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX)
            .is_some_and(|p| p.qualified_name == "::src::p"),
        "the literal spelling of the same export does resolve"
    );
}

/// TN for the interaction the dynamic-export abstention has with a
/// tombstone: a `namespace export -clear` written
/// after a dynamic pattern leaves the snapshot empty either way, so the
/// unrecorded pattern costs nothing there — the abstention is only ever
/// visible *before* a clear, which the test above covers.
#[test]
fn a_clear_after_a_dynamic_export_is_still_empty() {
    let src = "set pat p\nnamespace eval src {\n    proc p {} { return P }\n    namespace export $pat\n    namespace export -clear\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX).is_none(),
        "a `-clear` after any export leaves nothing exported"
    );
}

/// A load-level forget precedes a body-local import, so the call resolves.
/// C Tcl 8.6.14/9.0.4 return `P` for this original ordering.
#[test]
fn a_load_level_forget_before_a_body_local_import_revokes_nothing() {
    let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    proc setup {} { namespace import ::src::* ; p }\n}\nnamespace eval dst {\n    namespace forget ::src::p\n}\n";
    let analysis = analyse(src);
    let call = u32::try_from(src.rfind("; p }").expect("call in source") + 2).expect("tiny source");
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::p"),
        "the body-local import runs after the load-level forget and reinstalls: {hit:?}"
    );
    // FN guard — a forget written *inside that same body*, after the
    // import, is an ordinary later statement and does revoke.
    let inner = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    proc setup {} { namespace import ::src::* ; namespace forget ::src::p ; p }\n}\n";
    let analysis = analyse(inner);
    let call =
        u32::try_from(inner.rfind("; p }").expect("call in source") + 2).expect("tiny source");
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call).is_none(),
        "a forget in the same body before the call does revoke"
    );
}

#[test]
fn a_simple_forget_pattern_removes_the_alias_too() {
    // TN — `Tcl_ForgetImport`'s unqualified branch: the pattern is matched
    // against this namespace's own imported command names, whatever their
    // origin. Oracle: `namespace eval ::dst {namespace forget p}` leaves
    // `info commands ::dst::*` empty and raises no error.
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    namespace forget p\n}\n";
    let analysis = analyse(src);
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX).is_none(),
        "an unqualified forget pattern removes the alias too"
    );
}

#[test]
fn a_forget_of_an_unrelated_source_leaves_the_alias_alone() {
    // FP guard — a qualified forget names the source whose imports it
    // removes; one naming a *different* namespace must not touch this
    // alias.
    let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval other {\n    proc p {} { return O }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    namespace forget ::other::p\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::p"),
        "a forget aimed at another source namespace revokes nothing: {hit:?}"
    );
}

#[test]
fn a_re_import_after_a_forget_resolves_again() {
    // TP — the log is ordered, not a one-way tombstone: re-running the
    // import after the forget installs the alias again (oracle: the bare
    // call works once more).
    let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    namespace forget ::src::p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::p"),
        "a later re-import reinstates the alias: {hit:?}"
    );
}

// Behaviour 1b — the install is order-gated too.
//
// Oracle, byte-identical on tclsh 8.6.14 and 9.0.4:
//
//   namespace eval ::src { proc p {} {return P}; namespace export p }
//   namespace eval ::dst { p }                    → invalid command name "p"
//   namespace eval ::dst { namespace import ::src::* }
//   namespace eval ::dst { p }                    → P
//
//   namespace eval ::app { proc run {} { helper } }
//   namespace eval ::app { namespace import ::src::* }
//   ::app::run                                    → HELP
//
//   namespace eval ::app2 { proc run2 {} {
//       catch {helper} e ; namespace import ::src::* ; return "$e [helper]" } }
//   ::app2::run2       → {invalid command name "helper"} HELP

#[test]
fn a_top_level_call_before_its_own_import_resolves_nothing() {
    // TN: both statements are top level, so
    // the offsets are in genuine execution order.
    let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\np\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    let call = at_first(src, "\np\n") + 1;
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call);
    assert!(
        hit.is_none(),
        "a call written before its own import reaches nothing: {hit:?}"
    );
    // TP — the same call after the import does resolve.
    let after = after_last(src, "namespace import ::src::*\n}\n");
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", after)
            .is_some_and(|p| p.qualified_name == "::src::p"),
    );
}

#[test]
fn a_body_local_call_sees_an_import_written_later_in_the_file() {
    // TP (CRITICAL) — the gate is `in_effect`, not `at < call_off`: the
    // whole file loads, imports included, before any body runs. This is
    // the shape `tcllib`'s `modules/uev/uevent.tcl` is written in (procs
    // first, `namespace import` at the bottom); a plain-offset gate broke
    // every one of its call sites.
    let src = "namespace eval src {\n    proc helper {} { return HELP }\n    namespace export helper\n}\nnamespace eval app {\n    proc run {} { helper }\n}\nnamespace eval app {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    let call = at_first(src, "helper }");
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "app", "helper", call);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::helper"),
        "a body-local call observes its own file's later top-level import: {hit:?}"
    );
}

#[test]
fn a_call_before_an_import_in_that_same_body_resolves_nothing() {
    // TN — the FN guard: an import written in the *same* body after the
    // call is an ordinary later statement of the running script, exactly
    // as the export snapshot already treats one.
    let src = "namespace eval src {\n    proc helper {} { return HELP }\n    namespace export helper\n}\nnamespace eval app {\n    proc run {} { helper ; namespace import ::src::* ; helper }\n}\n";
    let analysis = analyse(src);
    let first = at_first(src, "helper ;");
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "app", "helper", first).is_none(),
        "the first call runs before the same body's import",
    );
    let second = after_last(src, "namespace import ::src::* ; ");
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "app", "helper", second)
            .is_some_and(|p| p.qualified_name == "::src::helper"),
        "the second call runs after it",
    );
}

#[test]
fn a_call_before_a_forced_import_is_not_shadowed_yet() {
    // TP — the same gate through `ImportQuery::ForcedShadow`: a `-force`
    // import has not replaced the local command until it runs, so a call
    // written before it still reaches the local definition (oracle:
    // `::dst::p` → LOCAL before the forced import, → SRC after).
    let src = "namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    proc p {} { return LOCAL }\n}\np\nnamespace eval dst {\n    namespace import -force ::src::*\n}\n";
    let analysis = analyse(src);
    let before = at_first(src, "\np\n") + 1;
    assert!(
        !forced_import_shadows(&analysis, DOC, "dst", "p", before),
        "the shadow is not in effect before the forced import runs",
    );
    let after = after_last(src, "namespace import -force ::src::*\n}\n");
    assert!(forced_import_shadows(&analysis, DOC, "dst", "p", after));
}

#[test]
fn a_later_conflicting_import_does_not_decide_an_earlier_call() {
    // TP — the winning source is chosen from the installs already in
    // effect, so a `-force` re-import from a different namespace written
    // *after* the call cannot retarget it (oracle: with `::A` imported
    // first, a call between the two imports runs `::A::p`).
    let src = "namespace eval A {\n    proc p {} { return A }\n    namespace export p\n}\nnamespace eval B {\n    proc p {} { return B }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::A::*\n}\np\nnamespace eval dst {\n    namespace import -force ::B::*\n}\n";
    let analysis = analyse(src);
    let call = at_first(src, "\np\n") + 1;
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::A::p"),
        "the call between the two imports takes the first edge: {hit:?}"
    );
    let after = after_last(src, "namespace import -force ::B::*\n}\n");
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", after)
            .is_some_and(|p| p.qualified_name == "::B::p"),
        "…and a call after the forced re-import takes the second",
    );
}

// Behaviour 2 — `-force` / import-conflict semantics.

#[test]
fn a_forced_import_shadows_the_local_command() {
    // TP (the bug): oracle
    //   namespace eval ::src { proc p {} {return SRC}; namespace export p }
    //   namespace eval ::dst { proc p {} {return LOCAL} }
    //   ::dst::p                                   → LOCAL
    //   namespace eval ::dst { namespace import -force ::src::* }
    //   ::dst::p                                   → SRC
    //   namespace origin ::dst::p                  → ::src::p
    let src = "namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    proc p {} { return LOCAL }\n}\nnamespace eval dst {\n    namespace import -force ::src::*\n}\nnamespace eval dst {\n    p\n}\n";
    let analysis = analyse(src);
    let call = last_call_head(src, "p");
    let hit = resolve_called_proc(&analysis, src, "::dst", "p", call, DOC);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::p"),
        "a `-force` import replaces the local command: {hit:?}"
    );
}

#[test]
fn an_unforced_import_onto_an_existing_command_installs_nothing() {
    // TN (the other bug): the same program without `-force` raises `can't
    // import command "p": already exists`, the local survives, and
    // `namespace origin ::dst::p` answers `::dst::p`. The import must not
    // make the call a reference to `::src::p`.
    let src = "namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    proc p {} { return LOCAL }\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX).is_none(),
        "a conflicting non-`-force` import installs nothing"
    );
    // …and the ordinary namespace lookup still answers the local.
    let hit = resolve_called_proc(&analysis, src, "::dst", "p", u32::MAX, DOC);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::dst::p"),
        "the local definition survives the failed import: {hit:?}"
    );
}

#[test]
fn a_local_defined_after_the_import_is_not_a_conflict_but_does_end_the_alias() {
    // The three-phase ordering. The conflict is
    // judged at the *import's* position, so a `proc` written after it does
    // not retroactively make the import a no-op — a call between the two
    // still reaches `::src::p`. But the `proc` *does* recreate
    // `::dst::p` as an ordinary command, silently, so a call after it
    // reaches the local one. Oracle (9.0.4 / 8.6.14, byte-identical):
    //   namespace eval ::dst {namespace import ::src::*}
    //   ::dst::p                    → SRC   ; origin → ::src::p
    //   namespace eval ::dst {proc p {} {return LOCAL}}    (rc 0, silent)
    //   ::dst::p                    → LOCAL ; origin → ::dst::p
    //   ::src::p                    → SRC   (untouched)
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\np\nnamespace eval dst {\n    proc p {} { return LOCAL }\n}\n";
    let analysis = analyse(src);
    // TP — between the import and the redefinition, the alias is live.
    let between = at_first(src, "\np\n") + 1;
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", between);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::p"),
        "a later local definition is not a conflict at the import: {hit:?}"
    );
    // TN — after it, the alias is gone and the local command is what the
    // ordinary namespace lookup answers.
    let after = after_last(src, "return LOCAL }");
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", after).is_none(),
        "redefining the imported name ends the alias"
    );
    let hit = resolve_called_proc(&analysis, src, "::dst", "p", after, DOC);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::dst::p"),
        "after the redefinition the call reaches the local proc: {hit:?}"
    );
}

#[test]
fn a_redefinition_ends_a_forced_import_shadow() {
    // TN — the same rule reached through `forced_import_shadows`: with the
    // shadow stuck on, `definition` / `hover` / `signature_help` would skip
    // the valid local definition and answer the import source forever.
    // Oracle: `proc ::dst::p` after `namespace import -force ::src::*`
    // returns LOCAL2 with `namespace origin ::dst::p` → `::dst::p`.
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    proc p {} { return LOCAL }\n}\nnamespace eval dst {\n    namespace import -force ::src::*\n}\nnamespace eval dst {\n    proc p {} { return LOCAL2 }\n}\n";
    let analysis = analyse(src);
    let between = at_first(src, "namespace import -force")
        + u32::try_from("namespace import -force ::src::*".len()).expect("tiny");
    let hit = resolve_called_proc(&analysis, src, "::dst", "p", between, DOC);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::p"),
        "between the forced import and the redefinition the source wins: {hit:?}"
    );
    let after = after_last(src, "return LOCAL2 }");
    let hit = resolve_called_proc(&analysis, src, "::dst", "p", after, DOC);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::dst::p"),
        "the redefinition ends the forced shadow: {hit:?}"
    );
}

#[test]
fn a_live_alias_is_an_import_conflict_for_a_different_source() {
    // TN (CRITICAL) — with `::dst` already
    // importing `::A::*`, a later *unforced* `namespace import ::B::*`
    // fails and changes nothing. Oracle (9.0.4 / 8.6.14):
    //   namespace eval ::dst {namespace import ::A::*}   ; origin → ::A::p
    //   namespace eval ::dst {namespace import ::B::*}
    //     → can't import command "p": already exists
    //   ::dst::p → AP ; origin → ::A::p
    // Before this, both imports entered the install set and the *newest*
    // won, resolving later calls to `::B::p`.
    let src = "namespace eval A {\n    proc p {} { return AP }\n    namespace export p\n}\nnamespace eval B {\n    proc p {} { return BP }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::A::*\n}\nnamespace eval dst {\n    namespace import ::B::*\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::A::p"),
        "the failed second import leaves the first alias in place: {hit:?}"
    );
}

#[test]
fn a_forced_import_from_a_second_source_replaces_the_first_alias() {
    // TP, the other half of the conflict rule — with `-force` the second import
    // wins (oracle: `::dst::p` → BP, `namespace origin` → `::B::p`).
    let src = "namespace eval A {\n    proc p {} { return AP }\n    namespace export p\n}\nnamespace eval B {\n    proc p {} { return BP }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::A::*\n}\nnamespace eval dst {\n    namespace import -force ::B::*\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::B::p"),
        "a `-force` import replaces a live alias too: {hit:?}"
    );
}

#[test]
fn a_body_local_import_conflicts_with_a_later_top_level_import() {
    // TN (CRITICAL) — the in-document twin of the workspace tier's own
    // check: which import installs is decided in *execution* order, not
    // written order. Oracle, byte-identical on tclsh 8.6.14 and 9.0.4:
    //
    //   namespace eval ::dst { proc p {} { namespace import ::B::x } }
    //   namespace eval ::dst { namespace import ::A::* }
    //   namespace origin ::dst::x  -> ::A::x
    //   ::dst::p                   -> can't import command "x": already exists
    //   namespace origin ::dst::x  -> ::A::x   (unchanged)
    //
    // The whole file loads before any body runs, so the top-level `::A`
    // import owns the name and the body-local `::B` one installs nothing.
    // Folding the slot log in raw offset order got both halves wrong: it
    // installed `::B` and then conflicted `::A` away.
    let src = "namespace eval A {\n    proc p {} { return AP }\n    namespace export p\n}\nnamespace eval B {\n    proc p {} { return BP }\n    namespace export p\n}\nnamespace eval dst {\n    proc runner {} { namespace import ::B::p }\n}\nnamespace eval dst {\n    namespace import ::A::*\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::A::p"),
        "the top-level import runs first and the body-local one installs \
             nothing: {hit:?}"
    );
}

#[test]
fn a_later_import_in_the_same_body_still_loses_to_the_earlier_one() {
    // TN guard — inside one body the offsets *are* execution order again.
    // Oracle: `proc q {} { namespace import ::A::* ; namespace import
    // ::B::p }` → the first succeeds, the second raises `already exists`,
    // `namespace origin` stays `::A::p`.
    let src = "namespace eval A {\n    proc p {} { return AP }\n    namespace export p\n}\nnamespace eval B {\n    proc p {} { return BP }\n    namespace export p\n}\nnamespace eval dst {\n    proc q {} { namespace import ::A::* ; namespace import ::B::p }\n}\n";
    let analysis = analyse(src);
    let call = after_last(src, "namespace import ::B::p");
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::A::p"),
        "the second import of the same body installs nothing: {hit:?}"
    );
}

#[test]
fn a_forget_lets_the_next_unforced_import_install() {
    // FN guard for the conflict rule — the conflict is the *live* alias, not the
    // fact that one was ever installed. Oracle: forgetting `::A::p` first
    // makes the unforced `::B::*` import succeed (`origin` → `::B::p`).
    let src = "rename auto_import savedPrelude; namespace eval A {\n    proc p {} { return AP }\n    namespace export p\n}\nnamespace eval B {\n    proc p {} { return BP }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::A::*\n}\nnamespace eval dst {\n    namespace forget ::A::p\n}\nnamespace eval dst {\n    namespace import ::B::*\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::B::p"),
        "after the forget the next unforced import installs: {hit:?}"
    );
}

#[test]
fn a_re_import_from_the_same_source_is_not_a_conflict() {
    // FN guard — re-importing the *same* thing is a silent no-op, not the
    // `already exists` error (oracle: rc 0, `origin` still `::A::p`), so
    // the second import must not be treated as conflicting with the alias
    // it would reinstall.
    let src = "namespace eval A {\n    proc p {} { return AP }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::A::*\n}\nnamespace eval dst {\n    namespace import ::A::*\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::A::p"),
        "a same-source re-import stays live: {hit:?}"
    );
}

#[test]
fn an_unforced_import_without_a_conflict_still_installs() {
    // FN guard — the conflict rule must not fire when the importing
    // namespace holds nothing of that name (the overwhelmingly common
    // case, oracle `H` row: `::dst::p` → `SRC`).
    let src = "namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX).is_some(),
        "an unconflicted import still installs"
    );
}

#[test]
fn a_forced_import_before_the_call_does_not_shadow_a_later_call_site() {
    // The forced import replaces the local `::dst::p`, and forgetting it
    // removes the only `::dst::p` left, so the local definition does not
    // come back. Oracle (tclsh 8.4.20 / 8.5.19 / 8.6.18 / 9.0.4 / 9.1.0):
    // the forced import makes `namespace origin ::dst::p` `::src::p`, the
    // forget leaves no `::dst::p`, and the call raises
    // `invalid command name "p"`. The query sits on the call's own head
    // word, which is what the original declaration selection reads.
    let src = "namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    proc p {} { return LOCAL }\n}\nnamespace eval dst {\n    namespace import -force ::src::*\n}\nnamespace eval dst {\n    namespace forget ::src::p\n}\nnamespace eval dst {\n    p\n}\n";
    let analysis = analyse(src);
    let call = last_call_head(src, "p");
    let hit = resolve_called_proc(&analysis, src, "::dst", "p", call, DOC);
    assert!(
        hit.is_none(),
        "after the forget no `p` is left for the call to reach: {hit:?}"
    );
}

// Behaviour 2b — what an *absent* export means for `-force`.

#[test]
fn a_forced_import_shadows_when_this_file_holds_no_export_for_the_source() {
    // TP (CRITICAL) — the partly-observable source. `::src`'s proc is in
    // this document, its `namespace export` is not, so a
    // "does this file hold *anything* of `::src`" test reads the gap as
    // "not exported" and single-file go-to-definition kept answering the
    // local `::app::helper`. Oracle (8.6.14 / 9.0.4) with the export
    // present anywhere in the program: `::app::helper` → SRC and
    // `namespace origin ::app::helper` → `::src::helper`.
    let src = "namespace eval src {\n    proc helper {} { return SRC }\n}\nnamespace eval app {\n    proc helper {} { return LOCAL }\n}\nnamespace eval app {\n    namespace import -force ::src::*\n}\nnamespace eval app {\n    helper\n}\n";
    let analysis = analyse(src);
    let call = last_call_head(src, "helper");
    assert!(
        forced_import_shadows(&analysis, DOC, "::app", "helper", call),
        "a `-force` import of a namespace whose exports this file cannot \
             see must presume the shadow",
    );
    assert!(
        resolve_called_proc(&analysis, src, "::app", "helper", call, DOC).is_none(),
        "…and the local definition the import deleted must not be the answer",
    );
}

#[test]
fn a_forced_import_does_not_shadow_when_this_file_holds_the_export_list() {
    // TN / FP guard — with an export declaration for `::src` in this
    // document the gap *is* evidence: `helper` is not among what `::src`
    // exports, so the forced import installs nothing and the local
    // definition is still what the call reaches (oracle: with only
    // `namespace export other`, `::app::helper` → LOCAL).
    let src = "namespace eval src {\n    proc helper {} { return SRC }\n    proc other {} { return O }\n    namespace export other\n}\nnamespace eval app {\n    proc helper {} { return LOCAL }\n}\nnamespace eval app {\n    namespace import -force ::src::*\n}\nnamespace eval app {\n    helper\n}\n";
    let analysis = analyse(src);
    let call = last_call_head(src, "helper");
    assert!(!forced_import_shadows(
        &analysis, DOC, "::app", "helper", call
    ));
    let hit = resolve_called_proc(&analysis, src, "::app", "helper", call, DOC);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::app::helper"),
        "the local definition survives an import that binds nothing: {hit:?}"
    );
}

// Behaviour 2c — the whole-program export oracle.
//
// The single document [`PARTLY_OBSERVABLE`] is the *same bytes* in every
// test below. Only the program around it changes, and with it the correct
// answer — which is the whole reason this tier needed an oracle. Oracle
// transcript (tclsh 8.6.14 and 9.0.4, byte-identical), the file loaded
// from a two-line loader that does or does not also source an
// `exports.tcl` holding `namespace eval ::src {namespace export helper}`:
//
//   with exports.tcl:      call -> SRC     origin -> ::src::helper
//   without exports.tcl:   call -> LOCAL   origin -> ::app::helper

/// The pinned two-file shape's single document, byte-for-byte.
///
/// `::src` is *partly* observable here: its procs and one of its exports
/// (`other`) are in this file. Whether it also exports `helper` is decided
/// somewhere this document cannot see.
const PARTLY_OBSERVABLE: &str = "namespace eval src {\n    proc helper {} { return SRC }\n    proc other {} { return O }\n    namespace export other\n}\nnamespace eval app {\n    proc helper {} { return LOCAL }\n}\nnamespace eval app {\n    namespace import -force ::src::*\n}\nnamespace eval app {\n    helper\n}\n";

/// A stand-in [`NamespaceExportOracle`] over an explicit table: a
/// namespace absent from it is [`ExportVerdict::Unknown`], one present
/// answers from its listed names.
///
/// Deliberately position-blind: what this fixture isolates is the
/// three-way verdict itself. Ordering a real workspace's export events
/// against an import site is `WorkspaceIndex`'s job, pinned against the
/// real snapshot in
/// [`the_real_oracle_does_not_resurrect_an_export_written_after_the_import`]
/// and in `workspace_index`'s own both-tiers tests.
struct FakeExports(&'static [(&'static str, &'static [&'static str])]);

impl NamespaceExportOracle for FakeExports {
    fn exported_at(
        &self,
        source_ns: &str,
        name: &str,
        _import_site: RunPoint<'_>,
    ) -> ExportVerdict {
        let bare = tcl_syntax::naming::unroot_rooted_key(source_ns).unwrap_or(source_ns);
        match self.0.iter().find(|(ns, _)| *ns == bare) {
            None => ExportVerdict::Unknown,
            Some((_, names)) if names.contains(&name) => ExportVerdict::Exported,
            Some(_) => ExportVerdict::NotExported,
        }
    }
}

/// [`PARTLY_OBSERVABLE`] resolved against `oracle`.
fn partly_observable_target(oracle: &dyn NamespaceExportOracle) -> Option<String> {
    let analysis = analyse(PARTLY_OBSERVABLE);
    let call = last_call_head(PARTLY_OBSERVABLE, "helper");
    let ctx = DOC.in_program(ProgramExports {
        uri: "file:///main.tcl",
        oracle,
    });
    let shadow = forced_import_shadows(&analysis, ctx, "::app", "helper", call);
    let hit = resolve_called_proc(&analysis, PARTLY_OBSERVABLE, "::app", "helper", call, ctx);
    // The two halves must not contradict: a live shadow means the local
    // command is gone, so the resolver may never answer with it.
    assert!(
        !(shadow && hit.is_some_and(|p| p.qualified_name == "::app::helper")),
        "shadow fired yet the resolver answered the deleted local command",
    );
    hit.map(|p| p.qualified_name.clone())
}

#[test]
fn a_forced_import_shadows_when_another_file_holds_the_covering_export() {
    // TP (the bug, CRITICAL) — `::src` exports `helper` somewhere else in
    // the program. The `-force` import therefore *did* delete
    // `::app::helper`, and the call reaches `::src::helper` — which this
    // document happens to hold, so the in-document tier can answer it
    // outright. The document-only rule reads "this file has an export
    // record for ::src and none of them covers helper" as a fact and keeps
    // answering the deleted local definition.
    assert_eq!(
        partly_observable_target(&FakeExports(&[("src", &["other", "helper"])])),
        Some("::src::helper".to_owned()),
    );
}

#[test]
fn a_forced_import_does_not_shadow_when_the_whole_program_exports_nothing() {
    // TN, byte-identical input to the test above — nothing anywhere in the
    // program exports `helper`, so the `-force` import binds only `other`
    // and the local definition survives (oracle: LOCAL / ::app::helper).
    // The document-only rule gets this one right too; it is the pinned
    // true negative
    // `a_forced_import_does_not_shadow_when_this_file_holds_the_export_list`
    // covers without an oracle.
    assert_eq!(
        partly_observable_target(&FakeExports(&[("src", &["other"])])),
        Some("::app::helper".to_owned()),
    );
}

#[test]
fn a_forced_import_from_a_namespace_the_program_cannot_see_still_shadows() {
    // TP / abstention — `::src` is in no indexed file (an installed
    // package, a document outside the workspace), so the oracle answers
    // `Unknown`. `-force` deletes the local command whether or not
    // anything here can prove the export, and answering with a command the
    // import may have removed is worse than answering nothing: the
    // resolver abstains and the cross-document tier takes over.
    assert_eq!(partly_observable_target(&FakeExports(&[])), None);
}

#[test]
fn the_same_document_without_an_oracle_keeps_the_document_only_answer() {
    // FP guard on the plumbing itself: the oracle is *optional*. A caller
    // with no workspace index — a single-document unit test, the `tcl`
    // CLI, a buffer the workspace has not indexed — must get exactly the
    // document-only answer rather than a panic or the abstention above.
    let analysis = analyse(PARTLY_OBSERVABLE);
    let call = last_call_head(PARTLY_OBSERVABLE, "helper");
    assert!(!forced_import_shadows(
        &analysis, DOC, "::app", "helper", call
    ));
    let hit = resolve_called_proc(&analysis, PARTLY_OBSERVABLE, "::app", "helper", call, DOC);
    assert_eq!(
        hit.map(|p| p.qualified_name.clone()),
        Some("::app::helper".to_owned()),
    );
}

#[test]
fn the_real_oracle_does_not_resurrect_an_export_written_after_the_import() {
    // FP guard, an export written after its import under the oracle, and the reason
    // [`ProgramExports`] pairs the oracle with the document's **real**
    // URI. The export covering the name is in this very document but
    // written *after* the import, so it is not retroactive (oracle:
    // `invalid command name "p"`). The workspace oracle holds that same
    // row, tagged with the same URI, so it ranks it against the import by
    // offset and answers `NotExported`. Handed a placeholder URI instead
    // it could not rank the row at all, and
    // `exported_at_import_site`'s abstain-toward-exported rule would have
    // installed an import Tcl never made.
    //
    // Deliberately run against the real `WorkspaceIndex` snapshot rather
    // than [`FakeExports`], which is position-blind by design.
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return SRC }\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval src {\n    namespace export p\n}\nnamespace eval dst {\n    p\n}\n";
    let analysis = analyse(src);
    let call = last_call_head(src, "p");
    let index =
        crate::workspace_index::WorkspaceIndex::from_documents([("file:///main.tcl", &analysis)]);
    let exports = index.export_snapshot();
    let ctx = DOC.in_program(ProgramExports {
        uri: "file:///main.tcl",
        oracle: exports.as_ref(),
    });
    assert!(
        proc_visible_via_wildcard_import(&analysis, ctx, "::dst", "p", call).is_none(),
        "an export written after the import must not install it retroactively",
    );
    // …and a *second* import written after the export does pick it up,
    // so the guard above is the ordering rule and not a blanket refusal.
    let src2 = format!("{src}namespace eval dst {{\n    namespace import ::src::*\n}}\n");
    let analysis2 = analyse(&src2);
    let index2 =
        crate::workspace_index::WorkspaceIndex::from_documents([("file:///main.tcl", &analysis2)]);
    let exports2 = index2.export_snapshot();
    let ctx2 = DOC.in_program(ProgramExports {
        uri: "file:///main.tcl",
        oracle: exports2.as_ref(),
    });
    let after = u32::try_from(src2.len()).expect("tiny test source");
    assert!(
        proc_visible_via_wildcard_import(&analysis2, ctx2, "::dst", "p", after)
            .is_some_and(|p| p.qualified_name == "::src::p"),
        "each import site takes its own export snapshot",
    );
}

// Behaviour 3 — source rename / delete / redefine after the import.

#[test]
fn deleting_the_source_command_kills_the_alias() {
    // TN (the bug): oracle
    //   namespace eval ::src { proc p {} {return P}; namespace export p }
    //   namespace eval ::dst { namespace import ::src::* }
    //   ::dst::p              → P
    //   rename ::src::p {}
    //   ::dst::p              → invalid command name "::dst::p"
    //   info commands ::dst::* → (empty)
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nrename ::src::p {}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_none(),
        "the alias holds the command object, so deleting it kills the alias: {hit:?}"
    );
}

#[test]
fn renaming_the_source_command_leaves_the_alias_alive() {
    // TP — the crucial asymmetry: oracle
    //   rename ::src::p ::src::pp
    //   ::dst::p                  → P          (still works)
    //   namespace origin ::dst::p → ::src::pp  (only the origin moved)
    //   info commands ::dst::*    → ::dst::p
    // A model that treated any `rename` of the source as a removal would
    // lose a genuinely-live command here.
    let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nrename ::src::p ::src::pp\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::p"),
        "a rename moves the origin but keeps the alias: {hit:?}"
    );
}

#[test]
fn a_call_before_the_source_deletion_still_resolves() {
    // FP guard — the deletion is order-gated like every other lifecycle
    // event, so a call written before it is unaffected.
    let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\np\nrename ::src::p {}\n";
    let analysis = analyse(src);
    let call = at_first(src, "\np\n") + 1;
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call).is_some(),
        "a call before the deletion still reaches the source"
    );
}

#[test]
fn a_redefinition_of_the_source_is_seen_through_the_link() {
    // TP — oracle: redefining `::src::p` after the import makes `::dst::p`
    // run the *new* body (`SECOND`), with `namespace origin ::dst::p`
    // still `::src::p`. The alias re-reads the source at call time, so the
    // definition a call reaches is the one in effect at the *call*, not
    // the one in effect at the import.
    let src = "namespace eval src {\n    proc p {} { return FIRST }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval src {\n    proc p {} { return SECOND }\n}\ndst::p\n";
    let analysis = analyse(src);
    let call = after_last(src, "dst::p");
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call).expect("alias");
    let captured = analysis
        .proc_def_in_effect_at(&hit.qualified_name, call)
        .expect("definition in effect at the call");
    assert_eq!(
        captured.name_span.start(),
        at_first(src, "p {} { return SECOND }"),
        "the redefinition is seen through the link"
    );
}

// Behaviour 4 — import chains.

#[test]
fn an_import_chain_follows_to_the_original_source() {
    // TP (the bug): oracle
    //   namespace eval ::C { proc p {} {return CP}; namespace export p }
    //   namespace eval ::B { namespace import ::C::*; namespace export p }
    //   namespace eval ::A { namespace import ::B::* }
    //   ::A::p                    → CP
    //   namespace origin ::A::p   → ::C::p
    // Statically `::B::p` is in no `all_procs`, so a single-hop walk found
    // nothing and navigation abstained.
    let src = "namespace eval C {\n    proc p {} { return CP }\n    namespace export p\n}\nnamespace eval B {\n    namespace import ::C::*\n    namespace export p\n}\nnamespace eval A {\n    namespace import ::B::*\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "A", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::C::p"),
        "the chain must follow ::A → ::B → ::C: {hit:?}"
    );
}

#[test]
fn a_forget_in_the_middle_of_a_chain_kills_the_whole_chain() {
    // TN — oracle: `namespace eval ::B {namespace forget ::C::p}` makes
    // `::A::p` an `invalid command name` too, because deleting an imported
    // command deletes the commands imported *from* it. Both `info commands
    // ::A::*` and `::B::*` come back empty.
    let src = "rename auto_import savedPrelude; namespace eval C {\n    proc p {} { return CP }\n    namespace export p\n}\nnamespace eval B {\n    namespace import ::C::*\n    namespace export p\n}\nnamespace eval A {\n    namespace import ::B::*\n}\nnamespace eval B {\n    namespace forget ::C::p\n}\n";
    let analysis = analyse(src);
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "A", "p", u32::MAX).is_none(),
        "a forget in the middle hop kills the outer alias too"
    );
}

#[test]
fn a_chain_hop_that_was_never_exported_does_not_resolve() {
    // FN guard — each hop keeps its own export gate: with `::B` importing
    // `::C::*` but never re-exporting `p`, `::A`'s import of `::B::*`
    // binds nothing (oracle: `::A::p` → invalid command name).
    let src = "namespace eval C {\n    proc p {} { return CP }\n    namespace export p\n}\nnamespace eval B {\n    namespace import ::C::*\n}\nnamespace eval A {\n    namespace import ::B::*\n}\n";
    let analysis = analyse(src);
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "A", "p", u32::MAX).is_none(),
        "the middle hop must have exported the name for the chain to run"
    );
}

#[test]
fn a_mutually_importing_pair_terminates() {
    // FP/hang guard — two namespaces importing each other's `*` form a
    // cycle with no definition at either end; the walk is bounded by
    // `indirection::MAX_COMMAND_NAME_HOPS` and must simply abstain.
    let src = "namespace eval A {\n    namespace import ::B::*\n    namespace export *\n}\nnamespace eval B {\n    namespace import ::A::*\n    namespace export *\n}\n";
    let analysis = analyse(src);
    assert!(proc_visible_via_wildcard_import(&analysis, DOC, "A", "p", u32::MAX).is_none());
    assert!(proc_visible_via_wildcard_import(&analysis, DOC, "B", "p", u32::MAX).is_none());
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

#[test]
fn jump_to_next_super_method() {
    // `next` inside B::greet jumps to the overridden A::greet.
    let src = "oo::class create A {\n    method greet {} { return hi }\n}\noo::class create B {\n    superclass A\n    method greet {} { next }\n}\n";
    let analysis = analyse(src);
    // Cursor on `next` (line 5). `    method greet {} { next }`
    let locs = definition(src, 5, 23, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // A::greet's name is on line 1 at column 11.
    assert_eq!(locs[0].start_line, 1);
    assert_eq!(locs[0].start_character, 11);
}

#[test]
fn jump_to_nextto_named_class_method() {
    // `nextto A` inside C::greet jumps to A::greet (skipping B).
    let src = "oo::class create A {\n    method greet {} { return hi }\n}\noo::class create B {\n    superclass A\n    method greet {} { next }\n}\noo::class create C {\n    superclass B\n    method greet {} { nextto A }\n}\n";
    let analysis = analyse(src);
    // Cursor on `nextto` (line 9). `    method greet {} { nextto A }`
    let locs = definition(src, 9, 22, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 1, "should land on A::greet");
    assert_eq!(locs[0].start_character, 11);
}

#[test]
fn jump_to_nextto_namespaced_class_method() {
    // `nextto A` names a namespaced sibling bare from within `::Ns::C`.
    // A method body resolves `A` through the object namespace, its
    // `::oo::Helpers` path, then `::` — never `::Ns` — so tclsh 8.6/9.0
    // raise "A does not refer to an object" and there is no target.
    let src = "namespace eval Ns {\n    oo::class create A {\n        method greet {} { return hi }\n    }\n    oo::class create B {\n        superclass A\n        method greet {} { next }\n    }\n    oo::class create C {\n        superclass B\n        method greet {} { nextto A }\n    }\n}\n";
    let analysis = analyse(src);
    // Cursor on `nextto` inside C::greet (line 10).
    let locs = definition(src, 10, 28, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn jump_to_nextto_relative_qualified_class_method() {
    // `nextto Ns::A` inside `::Ns::C` falls back to `::` and reaches
    // `::Ns::A` (tclsh 8.6/9.0 dispatch `::Ns::A::greet`).
    let src = "namespace eval Ns {\n    oo::class create A {\n        method greet {} { return hi }\n    }\n    oo::class create B {\n        superclass A\n        method greet {} { next }\n    }\n    oo::class create C {\n        superclass B\n        method greet {} { nextto Ns::A }\n    }\n}\n";
    let analysis = analyse(src);
    // Cursor on `nextto` inside C::greet (line 10).
    let locs = definition(src, 10, 28, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // ::Ns::A::greet's name token is on line 2.
    assert_eq!(locs[0].start_line, 2, "should land on ::Ns::A::greet");
}

#[test]
fn jump_to_nextto_fully_qualified_class_method() {
    // `nextto ::Ns::A` inside `::Ns::C` (tclsh 8.6/9.0 dispatch
    // `::Ns::A::greet`).
    let src = "namespace eval Ns {\n    oo::class create A {\n        method greet {} { return hi }\n    }\n    oo::class create B {\n        superclass A\n        method greet {} { next }\n    }\n    oo::class create C {\n        superclass B\n        method greet {} { nextto ::Ns::A }\n    }\n}\n";
    let analysis = analyse(src);
    // Cursor on `nextto` inside C::greet (line 10).
    let locs = definition(src, 10, 28, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // ::Ns::A::greet's name token is on line 2.
    assert_eq!(locs[0].start_line, 2, "should land on ::Ns::A::greet");
}

#[test]
fn nextto_refuses_a_class_name_reached_through_the_helpers_path() {
    // Bare `A` reaches `::oo::Helpers::A` through the method body's
    // namespace path before `::A`: tclsh 8.6/9.0 dispatch
    // `::oo::Helpers::A::greet` (line 2), never `::A::greet` (line 6).
    // The run-time object namespace is not modelled, so the answer is
    // refused rather than guessed.
    let src = "namespace eval ::oo::Helpers {\n    oo::class create A {\n        method greet {} { return helpers }\n    }\n}\noo::class create A {\n    method greet {} { return root }\n}\noo::class create C {\n    superclass ::A ::oo::Helpers::A\n    method greet {} { nextto A }\n}\n";
    let analysis = analyse(src);
    // Cursor on `nextto` inside C::greet (line 10).
    let locs = definition(src, 10, 22, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn nextto_picks_occurrence_at_cursor_not_first_on_line() {
    // The line has a decoy `nextto` earlier (inside a string) before the
    // real `nextto A`.  With the cursor on the real one, resolution must
    // read the class after *that* occurrence (A), not the first match.
    let src = "oo::class create A {\n    method greet {} { return hi }\n}\noo::class create B {\n    superclass A\n    method greet {} { next }\n}\noo::class create C {\n    superclass B\n    method greet {} { set x \"nextto Z\" ; nextto A }\n}\n";
    let analysis = analyse(src);
    // Cursor inside the real `nextto` (line 9, col 44).
    let locs = definition(src, 9, 44, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 1,
        "should land on A::greet, not the string decoy"
    );
    assert_eq!(locs[0].start_character, 11);
}

#[test]
fn jump_to_var_definition() {
    let src = "set greeting hi\nputs $greeting\n";
    let analysis = analyse(src);
    // Cursor inside `$greeting` on line 1.
    let locs = definition(src, 1, 8, &analysis);
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].start_line, 0, "{:?}", locs[0]);
}

#[test]
fn jump_to_proc_param_bareword_declaration_resolves_to_itself() {
    // TP — a cursor placed directly on a proc parameter's own bareword
    // name (not a `$`-prefixed read) must resolve, even though
    // `scope_chain_at` never reaches the proc's own scope for a byte
    // offset inside the parameter list, which sits textually *before*
    // that scope's `body_span` even starts.
    let src = "proc greet {name} { return $name }\n";
    let analysis = analyse(src);
    // Cursor on `name` inside the parameter list (col 12-16).
    let locs = definition(src, 0, 13, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 0, "{:?}", locs[0]);
    assert_eq!(locs[0].start_character, 12, "{:?}", locs[0]);
    assert_eq!(locs[0].end_character, 16, "{:?}", locs[0]);
}

#[test]
fn jump_to_catch_resultvar_bareword_resolves_to_the_reused_variable() {
    // TP — the finding's other confirmed shape: `catch script name`
    // reuses (writes back into) an *existing* variable — its own
    // bareword token is recorded in `VarDef.references`, not
    // `definition_span` (that stays pinned to the variable's original
    // declaration) — so a cursor placed directly on it must still
    // resolve, to the *original* declaration site.
    let src = "proc resolveSwitch {name def} {\n    catch {foo} name\n    return $name\n}\n";
    let analysis = analyse(src);
    // Cursor on the catch result-var `name` (line 1, col 16-20).
    let locs = definition(src, 1, 17, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 0,
        "must resolve to the original param declaration: {:?}",
        locs[0]
    );
    assert_eq!(locs[0].start_character, 20, "{:?}", locs[0]);
    assert_eq!(locs[0].end_character, 24, "{:?}", locs[0]);
}

#[test]
fn bareword_that_is_not_any_variable_occurrence_does_not_resolve_as_a_variable() {
    // FP guard — an ordinary bareword that never appears as any
    // variable's declaration or reference span (an unrelated command's
    // plain argument) must not be mistaken for a variable declaration
    // site, even in a file containing unrelated variables.
    let src = "proc greet {name} { return $name }\nputs literal\n";
    let analysis = analyse(src);
    // Cursor on `literal` (line 1, col 5-12) — not a variable anywhere.
    let locs = definition(src, 1, 7, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn jump_to_absolutely_qualified_var_definition() {
    // TP — the shape defer.tcl's `$::defer::idVar` and uri.tcl's
    // namespace-current pattern both reduce to: a `variable` declared at
    // namespace-eval scope, referenced elsewhere via its fully
    // `::`-qualified name.
    let src = "namespace eval ::simple {\n    variable v \"hello\"\n}\nputs $::simple::v\n";
    let analysis = analyse(src);
    // Cursor on `v` inside `$::simple::v` (line 3, col 16).
    let locs = definition(src, 3, 16, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 1, "{:?}", locs[0]);
    assert_eq!(locs[0].start_character, 13, "{:?}", locs[0]);
    assert_eq!(locs[0].end_character, 14, "name-sized span: {:?}", locs[0]);
}

#[test]
fn jump_to_relatively_qualified_var_definition_from_sibling_namespace() {
    // TP — a relative qualifier (`B::v`, no leading `::`) resolves under
    // the *cursor's* command-resolution namespace (tclsh 9.0.4/8.6.16
    // -verified: identical rule to what an unqualified command there
    // would resolve against), not the file's top level. `show` is
    // defined in `::A`, so `B::v` must reach `::A::B::v`, never a
    // top-level `::B::v`.
    let src = "namespace eval ::A {\n    namespace eval ::A::B {\n        variable v \"nested\"\n    }\n    proc show {} {\n        puts $B::v\n    }\n}\n";
    let analysis = analyse(src);
    // Cursor on `v` inside `$B::v` (line 5, col 17).
    let locs = definition(src, 5, 17, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 2, "{:?}", locs[0]);
    assert_eq!(locs[0].start_character, 17, "{:?}", locs[0]);
}

#[test]
fn qualified_var_definition_for_var_declared_in_a_reopened_namespace() {
    // TP — a namespace `namespace eval`-reopened in two unrelated
    // places: the qualified reference must find the declaration
    // regardless of which block declares it (no single lexical block
    // may be assumed to hold every variable a qualified reference
    // might mean).
    let src = "namespace eval ::A {\n    proc noop {} {}\n}\nnamespace eval ::A {\n    variable v 1\n}\nputs $::A::v\n";
    let analysis = analyse(src);
    let locs = definition(src, 6, 11, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 4, "{:?}", locs[0]);
}

#[test]
fn qualified_var_definition_abstains_for_unknown_namespace() {
    // TN — a qualified reference into a namespace that's never declared
    // anywhere in the file must abstain (empty), not guess at an
    // unrelated variable.
    let src = "namespace eval ::real {\n    variable v 1\n}\nputs $::nonexistent::v\n";
    let analysis = analyse(src);
    let locs = definition(src, 3, 20, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn qualified_var_definition_does_not_match_a_same_named_proc_local() {
    // FP guard — a proc's local variable must never satisfy a
    // namespace-qualified lookup even though the proc's own
    // command-resolution namespace coincides with the target: `f`'s
    // local `v` is unrelated to `::A::v`, which is never declared here.
    let src = "namespace eval ::A {\n    proc f {} {\n        set v 1\n    }\n}\nputs $::A::v\n";
    let analysis = analyse(src);
    let locs = definition(src, 5, 11, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

/// Go-to-definition of a formal-parameter use must resolve to
/// the parameter *name* in the declaration, not the proc name (proc) or the
/// whole method/constructor body (`TclOO`). The returned range must be a
/// single-line, name-sized span over `arg1`.
#[test]
fn param_definition_points_at_the_name_not_the_body() {
    let cases = [
        // (source, usage line, usage char, expected decl line, decl start col)
        (
            "proc greet {arg1 arg2} {\n    puts $arg1\n}\n",
            1,
            11,
            0,
            12,
        ),
        (
            "oo::class create C {\n    method m {arg1 arg2} {\n        puts $arg1\n    }\n}\n",
            2,
            15,
            1,
            14,
        ),
        (
            "oo::class create C {\n    constructor {arg1 arg2} {\n        puts $arg1\n    }\n}\n",
            2,
            15,
            1,
            17,
        ),
    ];
    for (src, ul, uc, dl, dc) in cases {
        let analysis = analyse(src);
        let locs = definition(src, ul, uc, &analysis);
        assert_eq!(locs.len(), 1, "one def for {src:?}: {locs:?}");
        let r = locs[0];
        assert_eq!(r.start_line, dl, "decl line for {src:?}: {r:?}");
        assert_eq!(r.start_character, dc, "decl col for {src:?}: {r:?}");
        // Name-sized: same line, spans the 4-char `arg1`.
        assert_eq!(r.end_line, dl, "decl is single-line for {src:?}: {r:?}");
        assert_eq!(
            r.end_character - r.start_character,
            4,
            "decl spans `arg1` (4 chars) for {src:?}: {r:?}",
        );
    }
}

#[test]
fn no_definition_for_unknown_word() {
    let src = "puts hello\n";
    let analysis = analyse(src);
    assert_eq!(
        definition(src, 0, 6, &analysis),
        [] as [crate::definition::LspRange; 0]
    );
}

#[test]
fn jump_to_class_definition() {
    let src = "oo::class create Greeter {}\nGreeter\n";
    let analysis = analyse(src);
    // Cursor on `Greeter` on line 1.
    let locs = definition(src, 1, 2, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 0);
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

// scope-chain $var descent

#[test]
fn proc_local_var_jumps_to_proc_scope_definition() {
    // `local` is defined inside `proc f`.  Cursor on
    // `$local` inside `f`'s body must jump to the
    // proc-local definition, not the global scope
    // (which has none).
    let src = "proc f {} {\n    set local 1\n    puts $local\n}\n";
    let analysis = analyse(src);
    // Cursor inside `$local` on line 2.
    let locs = definition(src, 2, 12, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // The proc-local `set local 1` is on line 1.
    assert_eq!(locs[0].start_line, 1, "{:?}", locs[0]);
}

#[test]
fn byte_offset_at_handles_newlines() {
    let src = "abc\ndef\nghi\n";
    let line_index = LineIndex::new(src);
    assert_eq!(byte_offset_at(&line_index, src, 0, 0), 0);
    assert_eq!(byte_offset_at(&line_index, src, 0, 3), 3);
    assert_eq!(byte_offset_at(&line_index, src, 1, 0), 4);
    assert_eq!(byte_offset_at(&line_index, src, 2, 2), 10);
}

#[test]
fn byte_offset_at_uses_lsp_utf16_columns() {
    let src = "a😀b";
    let line_index = LineIndex::new(src);
    assert_eq!(byte_offset_at(&line_index, src, 0, 0), 0);
    assert_eq!(byte_offset_at(&line_index, src, 0, 1), 1);
    assert_eq!(byte_offset_at(&line_index, src, 0, 3), 5);
    assert_eq!(byte_offset_at(&line_index, src, 0, 4), 6);
}

#[test]
fn span_to_range_translates_offsets() {
    let src = "abc\ndef\n";
    let line_index = LineIndex::new(src);
    // Span covering `def` (offsets 4..7).
    let span = tcl_lexer::Span::new(4, 7);
    let range = span_to_range(src, &line_index, span);
    assert_eq!(range.start_line, 1);
    assert_eq!(range.start_character, 0);
    assert_eq!(range.end_line, 1);
    assert_eq!(range.end_character, 3);
}

#[test]
fn span_to_range_uses_lsp_utf16_columns() {
    let src = "é😀x\n";
    let line_index = LineIndex::new(src);
    let range = span_to_range(src, &line_index, tcl_lexer::Span::new(6, 7));
    assert_eq!(range.start_line, 0);
    assert_eq!(range.start_character, 3);
    assert_eq!(range.end_line, 0);
    assert_eq!(range.end_character, 4);
}

// class-member lookup

#[test]
fn definition_bare_sibling_method_call_without_link_abstains() {
    // FP — a bareword sibling method call is NOT
    // actually reachable from another method's body unless `link`
    // exposed it that way; real tclsh: "invalid command name". Must
    // abstain rather than falsely resolve.
    let src =
        "oo::class create C {\n    method greet {} {}\n    method twice {} { greet ; greet }\n}\n";
    let analysis = analyse(src);
    // Cursor on the first `greet` in the `twice` body.
    // Line 2: `    method twice {} { greet ; greet }`
    // Col 22 lands on the `g` of the first `greet`.
    let locs = definition(src, 2, 22, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn definition_linked_sibling_method_call_resolves() {
    // TP — `link greet` (called from the constructor) makes `greet`
    // genuinely bareword-callable from every method body of this class, so
    // the bare call resolves to the method's declaration.
    let src = "oo::class create C {\n    constructor {} { link greet }\n    method greet {} {}\n    method twice {} { greet ; greet }\n}\n";
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    // Line 3: `    method twice {} { greet ; greet }` — same text/col
    // as the un-linked test above, one line further down.
    let locs = definition(src, 3, 22, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // The method declaration is now on line 2.
    assert_eq!(locs[0].start_line, 2);
}

#[test]
fn definition_my_dispatch_resolves_when_class_extended_via_separate_oo_define() {
    // `Gadget` is
    // created via `oo::class create` with no body, then every method —
    // including the `my Helper` call site itself — is added via a
    // *separate*, later `oo::define Gadget { ... }` block. This is
    // exactly the real corpus shape (`ticklecharts::chart`: `oo::class
    // create` at one line, all methods added via a later `oo::define`).
    // tclsh9.0/8.6 both prove `my Helper` genuinely dispatches to
    // `Helper` here — go-to-definition must resolve it, not abstain.
    let src = "oo::class create Gadget {\n    variable _x\n}\noo::define Gadget {\n    method Helper {} { return hi }\n    method Caller {} { my Helper }\n}\n";
    let analysis = analyse(src);
    // Line 5: `    method Caller {} { my Helper }` — cursor on the
    // `Helper` word inside `my Helper` (col 26).
    let locs = definition(src, 5, 26, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // The `Helper` method declaration is on line 4.
    assert_eq!(locs[0].start_line, 4);
}

/// `[self] m` / `[self object] m` is `TclOO`'s own
/// same-object dispatch idiom — `self`, called with no argument (or the
/// explicit `object` word), returns the current object's own command
/// name. It reaches the enclosing class exactly like `my` does, but
/// through a bracketed command substitution rather than a bareword
/// dispatch keyword. Shared by both spellings' tests below.
fn assert_self_bracket_dispatch_resolves_like_my(head: &str) {
    let src = format!(
        "oo::class create C {{\n    method animTick {{}} {{ return 1 }}\n    method anim {{}} {{ {head} animTick }}\n}}\n"
    );
    let call_col = u32::try_from(src.lines().nth(2).unwrap().find("animTick").unwrap()).unwrap();
    let locs = definition(&src, 2, call_col, &analyse(&src));
    assert_eq!(locs.len(), 1, "{locs:?} (head={head:?})");
    assert_eq!(locs[0].start_line, 1);
}

#[test]
fn definition_bare_self_bracket_dispatch_resolves_like_my() {
    assert_self_bracket_dispatch_resolves_like_my("[self]");
}

#[test]
fn definition_self_object_bracket_dispatch_resolves_like_my() {
    assert_self_bracket_dispatch_resolves_like_my("[self object]");
}

#[test]
fn definition_bare_sibling_classmethod_call_without_link_abstains() {
    // FP — same shape as the method case above,
    // for `classmethod`.
    let src =
        "oo::class create C {\n    classmethod factory {} {}\n    method use {} { factory }\n}\n";
    let analysis = analyse(src);
    // Line 2: `    method use {} { factory }`
    // Cursor on `factory` (col 20).
    let locs = definition(src, 2, 20, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn definition_linked_sibling_classmethod_call_resolves() {
    // TP — `link factory` makes the classmethod
    // call resolve.
    let src = "oo::class create C {\n    constructor {} { link factory }\n    classmethod factory {} {}\n    method use {} { factory }\n}\n";
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    // Line 3: `    method use {} { factory }` — same text/col as the
    // un-linked test above, one line further down.
    let locs = definition(src, 3, 20, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // The classmethod declaration is now on line 2.
    assert_eq!(locs[0].start_line, 2);
}

/// The line each version resolves the `nth` occurrence of `word` to.
fn method_body_definition_lines(src: &str, word: &str, nth: usize) -> Vec<Vec<u32>> {
    let offset = src.match_indices(word).nth(nth).unwrap().0;
    let line = u32::try_from(src[..offset].matches('\n').count()).unwrap();
    let column = u32::try_from(offset - src[..offset].rfind('\n').map_or(0, |at| at + 1)).unwrap();
    ["tcl8.6", "tcl9.0", "tcl9.1"]
        .into_iter()
        .map(|version| {
            let analysis = Analyser::new().analyse(src, version).clone();
            definition(src, line, column, &analysis)
                .iter()
                .map(|range| range.start_line)
                .collect()
        })
        .collect()
}

/// tclsh 8.6, 9.0, 9.1: a method body runs in its object's namespace with
/// `namespace path ::oo::Helpers`. A method declared inside
/// `namespace eval ::app` calls `::helper` (`ns_method.tcl`: `global`),
/// and `::oo::Helpers::helper`, even when defined after the class,
/// shadows `::helper` (`helpers_shadow.tcl`: `helpers`).
#[test]
fn definition_in_a_method_body_follows_the_object_namespace_lookup() {
    let ns = "proc helper {} {return global}\nnamespace eval ::app {\n    proc helper {} {return app}\n    oo::class create C {\n        method use {} { helper }\n    }\n}\n";
    assert_eq!(
        method_body_definition_lines(ns, "helper", 2),
        [[0], [0], [0]]
    );
    let helpers = "proc helper {} {return global}\noo::class create C {\n    method use {} { helper }\n}\nproc ::oo::Helpers::helper {} {return helpers}\n";
    assert_eq!(
        method_body_definition_lines(helpers, "helper", 1),
        [[4], [4], [4]]
    );
    // `self` only reads the call context and `my` dispatches to a method
    // whose own body is inspected, so neither leaves the lookup unproved.
    let introspecting = "proc helper {} {return global}\noo::class create C {\n    method other {} {}\n    method use {} { my other; set me [self]; helper }\n}\n";
    assert_eq!(
        method_body_definition_lines(introspecting, "helper", 1),
        [[0], [0], [0]]
    );
}

/// `my` lives in the object namespace (tclsh 8.6, 9.0, 9.1: `namespace
/// which my` is `::oo::Obj12::my`; `d_my.tcl` prints `method-q` despite a
/// global `proc my`), so it never names the global procedure; nor does
/// `self`, which `::oo::Helpers` holds (`self_shadow.tcl`: `1`). A method
/// that defines `helper` relative to the object namespace shadows the
/// global one for later calls (`relproc_shadow.tcl`: `global local`), so a
/// document with such a body leaves member-body heads unresolved.
#[test]
fn definition_in_a_method_body_demurs_on_object_namespace_commands() {
    let my = "proc my {args} {return global-my}\noo::class create C {\n    method m {} {return [my q]}\n    method q {} {return method-q}\n}\n";
    assert_eq!(
        method_body_definition_lines(my, "my", 1),
        [[], [], []] as [[u32; 0]; 3]
    );
    let own_self = "proc self {args} {return global-self}\noo::class create C {\n    method use {} { self }\n}\n";
    assert_eq!(
        method_body_definition_lines(own_self, "self", 1),
        [[], [], []] as [[u32; 0]; 3]
    );
    // tclsh 9.0, 9.1 (`link_shadow.tcl`: `global method`): `link` makes a
    // method callable as a command in the object namespace, so the call
    // never names the global procedure; the linked method answers.
    let linked = "proc helper {} {return global}\noo::class create C {\n    method helper {} {return method}\n    method mk {} { link helper }\n    method use {} { helper }\n}\n";
    assert_eq!(
        method_body_definition_lines(linked, "helper", 3)[1..],
        [[2], [2]]
    );
    let relproc = "proc helper {} {return global}\noo::class create C {\n    method mk {} { proc helper {} {return local} }\n    method use {} { helper }\n}\n";
    assert_eq!(
        method_body_definition_lines(relproc, "helper", 2),
        [[], [], []] as [[u32; 0]; 3]
    );
}

#[test]
fn definition_jumps_to_constructor_keyword() {
    // Bare `constructor` inside a class body jumps to the
    // constructor's declaration.
    let src = "oo::class create C {\n    constructor {arg} {}\n    method touch_ctor {} { constructor }\n}\n";
    let analysis = analyse(src);
    // Cursor on `constructor` in the `touch_ctor` body
    // (line 2 col 27).
    let locs = definition(src, 2, 27, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // The analyser anchors the constructor's ``name_span``
    // on the ``constructor`` keyword token (declared on
    // line 1).  The provider keeps a body-span fallback
    // for the empty-span case, but the keyword span is
    // populated here, so the jump lands on line 1.
    assert_eq!(locs[0].start_line, 1);
}

#[test]
fn definition_bare_sibling_property_call_without_link_abstains() {
    // FP — same rationale as the method/classmethod
    // cases above, for a Tcl 9.0+ `oo::configurable` `property`
    // accessor bareword call: `[length]` is not actually reachable
    // without a `link`; real tclsh: "invalid command name".
    let src = "oo::configurable create Widget {\n    property length -get {return 42}\n    method use {} { return [length] }\n}\n";
    let analysis = analyse(src);
    // Line 2: `    method use {} { return [length] }` — col 27 lands
    // on the `l` of `length`.
    let locs = definition(src, 2, 27, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn definition_member_lookup_skipped_outside_class_body() {
    // Same word outside the class body must not surface
    // the method definition.
    let src = "oo::class create C {\n    method greet {} {}\n}\ngreet\n";
    let analysis = analyse(src);
    // Cursor on the bare `greet` on line 3.
    let locs = definition(src, 3, 2, &analysis);
    // No proc / class / member named `greet` is in scope
    // here — the class-member lookup must not leak.
    assert!(locs.is_empty(), "{locs:?}");
}

// $obj method dispatch

#[test]
fn definition_resolves_obj_method_call() {
    // `set d [Dog new]` then `$d bark` — cursor on `bark`
    // jumps to the method declaration on Dog.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\n";
    let analysis = analyse(src);
    // Line 4 `$d bark` — `bark` starts at col 3.
    let locs = definition(src, 4, 3, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // `method bark` is on line 1.
    assert_eq!(locs[0].start_line, 1);
}

#[test]
fn definition_resolves_a_literal_foreach_installed_method() {
    // `alpha` has no written `method alpha` word anywhere —
    // its only textual appearance is inside the loop's own literal list
    // — but go-to-definition must still land somewhere real rather than
    // finding nothing, exactly as hover/outline/completion do.
    let src = concat!(
        "oo::class create Widget {\n",
        "    foreach m {alpha beta gamma} {\n",
        "        method $m {args} { return $args }\n",
        "    }\n",
        "}\n",
        "set d [Widget new]\n",
        "$d alpha\n",
    );
    let analysis = analyse(src);
    // Line 6 `$d alpha` — `alpha` starts at col 3.
    let locs = definition(src, 6, 3, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // The list `{alpha beta gamma}` is on line 1.
    assert_eq!(locs[0].start_line, 1, "{locs:?}");
}

#[test]
fn definition_resolves_obj_method_in_bracket() {
    // `[$d bark]` bracketed form.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nputs [$d bark]\n";
    let analysis = analyse(src);
    // Line 4 `puts [$d bark]` — `bark` starts at col 9.
    let locs = definition(src, 4, 9, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 1);
}

#[test]
fn definition_obj_method_unknown_instance_falls_through() {
    // `$x bark` where `x` has no recorded class — no
    // instance-method resolution, falls through (and finds
    // nothing here).
    let src = "oo::class create Dog {\n    method bark {} {}\n}\n$x bark\n";
    let analysis = analyse(src);
    let locs = definition(src, 3, 3, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn definition_resolves_bare_created_instance_command_method() {
    // `Dog create rex` then `rex bark` — cursor on the bare
    // `bark` jumps to the method declaration, mirroring `$obj bark`.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nDog create rex\nrex bark\n";
    let analysis = analyse(src);
    // Line 4 `rex bark` — `bark` starts at col 4.
    let locs = definition(src, 4, 4, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 1);
}

#[test]
fn definition_bare_var_receiver_without_dollar_does_not_resolve() {
    // `set d [Dog new]` binds `d` as a variable; a bare `d bark` (no `$`)
    // is not a valid dispatch, so it must not resolve to the method.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nd bark\n";
    let analysis = analyse(src);
    let locs = definition(src, 4, 2, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn instance_method_at_cursor_detects_dollar_head() {
    let src = "$d bark\n";
    let got = instance_method_at_cursor(src, 0, 4, tcl_lexer::LexerConfig::default());
    assert_eq!(got, Some(("d".to_string(), "bark".to_string(), true)));
}

#[test]
fn instance_method_var_decoration_vectors_keep_unclosed_braces() {
    assert_eq!(
        instance_method_at_cursor("${x} bark\n", 0, 5, tcl_lexer::LexerConfig::default()),
        Some(("x".to_string(), "bark".to_string(), true))
    );
    assert_eq!(
        instance_method_at_cursor("$x bark\n", 0, 3, tcl_lexer::LexerConfig::default()),
        Some(("x".to_string(), "bark".to_string(), true))
    );
    assert_eq!(
        instance_method_at_cursor("$arr(idx) bark\n", 0, 10, tcl_lexer::LexerConfig::default()),
        Some(("arr(idx)".to_string(), "bark".to_string(), true))
    );
    assert_eq!(
        instance_method_at_cursor("$::a:::b bark\n", 0, 9, tcl_lexer::LexerConfig::default()),
        Some(("::a:::b".to_string(), "bark".to_string(), true))
    );
    assert_eq!(
        instance_method_at_cursor("$foo::: bark\n", 0, 9, tcl_lexer::LexerConfig::default()),
        Some(("foo:::".to_string(), "bark".to_string(), true))
    );
    // The malformed source is tokenised before this feature sees it;
    // assert the shared codec's contract directly, while the following
    // feature-level test proves it cannot resolve the real `x` object.
    assert_eq!(tcl_syntax::naming::var_reference("${x"), "{x");
}

#[test]
fn definition_does_not_resolve_an_unclosed_braced_instance_head() {
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset x [Dog new]\n${x bark\n";
    let analysis = analyse(src);
    let locs = definition(src, 4, 4, &analysis);
    assert!(
        locs.is_empty(),
        "unclosed `${{x` must not dispatch through real `x`: {locs:?}"
    );
}

#[test]
fn instance_method_at_cursor_reports_bare_head_as_non_dollar() {
    // `foo bark` — a bare-word receiver (an object command); reported
    // with `is_dollar == false`.  Whether it actually resolves to a
    // class is decided later by `logical_receiver_candidate_at`.
    let src = "foo bark\n";
    assert_eq!(
        instance_method_at_cursor(src, 0, 5, tcl_lexer::LexerConfig::default()),
        Some(("foo".to_string(), "bark".to_string(), false))
    );
}

#[test]
fn instance_method_at_cursor_extracts_a_substituted_head_verbatim() {
    // `[x] bark` — a command-substitution head is not a bare object
    // command, but this function only extracts the receiver text;
    // whether `[x]` names anything a caller can resolve is a semantic
    // question answered downstream by the original selected descriptor, not here. Rejecting every bracketed head
    // outright would also reject `[self]` / `[self object]`, TclOO's own
    // same-object dispatch spelling.
    let src = "[x] bark\n";
    assert_eq!(
        instance_method_at_cursor(src, 0, 5, tcl_lexer::LexerConfig::default()),
        Some(("[x]".to_string(), "bark".to_string(), false))
    );
}

#[test]
fn instance_method_at_cursor_extracts_a_self_receiver_head_with_brackets_intact() {
    // `[self] m`. The brackets must survive
    // extraction: `parse_command_substitution` (and so
    // `original_internal_dispatch`) requires them.
    let src = "[self] mrun\n";
    assert_eq!(
        instance_method_at_cursor(src, 0, 8, tcl_lexer::LexerConfig::default()),
        Some(("[self]".to_string(), "mrun".to_string(), false))
    );
}

#[test]
fn instance_method_at_cursor_still_rejects_a_malformed_bracket_head() {
    // A stray, unmatched `]` with no opening `[` in scope is not a
    // well-formed command substitution and not a bare identifier
    // either, so it is rejected.
    let src = "x] bark\n";
    assert_eq!(
        instance_method_at_cursor(src, 0, 4, tcl_lexer::LexerConfig::default()),
        None
    );
}

// Method names with non-identifier characters.
// Oracle (tclsh 8.6.14 + 9.0.4): `method with-dash`, `method a.b`, and
// TIP 558's `<ReadProp-x>` / `<WriteProp-x>` all dispatch for real.

#[test]
fn instance_method_at_cursor_keeps_a_hyphenated_method_name_whole() {
    // TP — `-` is part of the method word, not a boundary.
    let src = "$d with-dash\n";
    assert_eq!(
        instance_method_at_cursor(src, 0, 8, tcl_lexer::LexerConfig::default()),
        Some(("d".to_string(), "with-dash".to_string(), true))
    );
}

#[test]
fn instance_method_at_cursor_keeps_an_angle_bracketed_method_name_whole() {
    // TP — the TIP 558 property-accessor shape, cursor inside the name.
    let src = "my <ReadProp-colour>\n";
    assert_eq!(
        instance_method_at_cursor(src, 0, 8, tcl_lexer::LexerConfig::default()),
        Some(("my".to_string(), "<ReadProp-colour>".to_string(), false))
    );
}

#[test]
fn instance_method_at_cursor_keeps_a_dotted_method_name_whole() {
    // TP — `method a.b` dispatches in real Tcl; `.` is not a boundary.
    let src = "$d a.b\n";
    assert_eq!(
        instance_method_at_cursor(src, 0, 4, tcl_lexer::LexerConfig::default()),
        Some(("d".to_string(), "a.b".to_string(), true))
    );
}

#[test]
fn instance_method_at_cursor_rejects_subtraction_in_an_expression() {
    // TN — `$x-1` is arithmetic. The whole `x-1` run is one word whose
    // head slice is a bare `$`, so nothing dispatches.
    let src = "expr {$x-1}\n";
    assert_eq!(
        instance_method_at_cursor(src, 0, 8, tcl_lexer::LexerConfig::default()),
        None
    );
    assert_eq!(
        instance_method_at_cursor(src, 0, 9, tcl_lexer::LexerConfig::default()),
        None
    );
}

#[test]
fn instance_method_at_cursor_rejects_a_bare_comparison_operator() {
    // TN — `expr {$a < $b}` must not read as `$a` dispatching `<`:
    // the word carries no alphanumeric content.
    let src = "expr {$a < $b}\n";
    assert_eq!(
        instance_method_at_cursor(src, 0, 9, tcl_lexer::LexerConfig::default()),
        None
    );
}

#[test]
fn hyphenated_and_angle_bracketed_methods_resolve_from_a_call_site() {
    // TP — end-to-end. `with-dash` starts with an ASCII lowercase
    // letter so it is exported and dispatches externally; the TIP 558
    // accessor `<ReadProp-x>` starts with `<` so it is *unexported* and
    // only reachable from inside the class via `my` (oracle: tclsh
    // 9.0.4 answers `unknown method "<foo>"` for an external call but
    // runs `my <ReadProp-x>` fine).
    let src = concat!(
        "oo::class create C {\n",
        "    method with-dash {} { return 1 }\n",
        "    method <ReadProp-x> {} { return 2 }\n",
        "    method probe {} { my <ReadProp-x> }\n",
        "}\n",
        "C create rex\n",
        "rex with-dash\n",
    );
    let analysis = analyse(src);
    let dash = definition(src, 6, 6, &analysis);
    assert_eq!(dash.len(), 1, "{dash:?}");
    assert_eq!(dash[0].start_line, 1);
    assert_eq!(dash[0].start_character, 11);
    let prop = definition(src, 3, 28, &analysis);
    assert_eq!(prop.len(), 1, "{prop:?}");
    assert_eq!(prop[0].start_line, 2);
    assert_eq!(prop[0].start_character, 11);
}

#[test]
fn unexported_angle_bracketed_method_is_not_externally_dispatchable() {
    // TN — the export rule still applies to the widened word: an
    // external `rex <ReadProp-x>` is `unknown method` in real Tcl
    // (verified, tclsh 8.6.14 + 9.0.4), so it must resolve to nothing
    // rather than to the declaration.
    let src = concat!(
        "oo::class create C {\n",
        "    method <ReadProp-x> {} { return 2 }\n",
        "}\n",
        "C create rex\n",
        "rex <ReadProp-x>\n",
    );
    let analysis = analyse(src);
    assert_eq!(
        definition(src, 4, 8, &analysis),
        [] as [crate::definition::LspRange; 0]
    );
}

#[test]
fn logical_receiver_candidates_keep_instance_and_class_buckets_together() {
    // naming.core.original-source-instance-completion
    // docs/design/analysis/name-resolution-proofs/core-original-source-instance-completion.md
    let source = "oo::class create Bar {method get {} {}}; Bar create rex; set b [Bar new]";
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::context_for_profile(profile),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let analysis = Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name);
    assert!(analysis.allows_lexical_declaration_advice());
    assert!(matches!(
        logical_receiver_candidate_at(&analysis, "rex", false, 0),
        Some((_, MethodBucket::Instance))
    ));
    assert!(logical_receiver_candidate_at(&analysis, "b", false, 0).is_none());
    assert!(matches!(
        logical_receiver_candidate_at(&analysis, "Bar", false, 0),
        Some((_, MethodBucket::Class))
    ));
    let native = analyse(source);
    for receiver in ["rex", "b", "Bar"] {
        assert!(logical_receiver_candidate_at(&native, receiver, false, 0).is_none());
    }
}

// Class-command dispatch: `CLASS method` for a
// classmethod / `self method`, distinct from `$obj method` / `NAME
// method` instance dispatch above.

#[test]
fn classmethod_call_on_its_own_class_resolves() {
    // Native C9 classmethod on its original declaring class.
    let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\nActiveRecord find foo bar\n";
    let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
    let locs = definition(src, 3, 14, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 1);
    assert_eq!(locs[0].start_character, 16);
}

#[test]
fn classmethod_call_on_an_inheriting_subclass_resolves() {
    // Native C9 delegates inherit the original declaring-class method.
    let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\noo::class create Table {\n    superclass ActiveRecord\n}\nTable find foo bar\n";
    let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
    let locs = definition(src, 6, 7, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 1);
    assert_eq!(locs[0].start_character, 16);
}

#[test]
fn self_method_call_on_its_own_class_resolves() {
    // TP — requires BOTH Part 1 (self recognised at all) and Part 2
    // (the class-command receiver path).
    let src = "oo::class create Widget {\n    self method make {n} { return \"made $n\" }\n}\nWidget make gadget\n";
    let analysis = analyse(src);
    let locs = definition(src, 3, 8, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 1);
    assert_eq!(locs[0].start_character, 16);
}

#[test]
fn self_method_not_inherited_by_a_non_overriding_subclass() {
    // TN — the critical precision test for `is_self_method`: unlike
    // `ooutil`'s `classmethod`, a plain `self method` is NOT inherited
    // by a subclass's own bound command (confirmed against tclsh:
    // `Gadget make` raises `unknown method "make"`). A naive reuse of
    // the ordinary MRO walk (which DOES include Widget in Gadget's
    // MRO) would incorrectly resolve this.
    let src = "oo::class create Widget {\n    self method make {n} { return \"made $n\" }\n}\noo::class create Gadget {\n    superclass Widget\n}\nGadget make foo\n";
    let analysis = analyse(src);
    let locs = definition(src, 6, 8, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn instance_calling_a_classmethod_does_not_resolve() {
    // C8.6 provides no native classmethod worker or package extension.
    // C9's instance-side delegate is a distinct native forwarding protocol.
    let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\nActiveRecord create rec1\nrec1 find foo bar\n";
    let analysis = analyse(src);
    let locs = definition(src, 4, 5, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn native_classmethod_requires_the_selected_engine_or_a_provider() {
    let source = "oo::class create C {classmethod find {} {return OK}}\nC find\n";
    let unavailable = Analyser::new().analyse(source, "tcl8.6").clone();
    assert_eq!(
        definition(source, 1, 2, &unavailable),
        [] as [crate::definition::LspRange; 0]
    );
    let provided = Analyser::new().analyse(source, "tcl9.1").clone();
    let locations = definition(source, 1, 2, &provided);
    assert_eq!(locations.len(), 1, "{locations:?}");
    assert_eq!(locations[0].start_line, 0);
    assert_eq!(locations[0].start_character, 32);
}

#[test]
fn bare_var_receiver_without_a_bound_command_does_not_resolve_class_dispatch() {
    // TN — mirrors
    // `logical_receiver_candidates_keep_instance_and_class_buckets_together`: `d` is
    // a plain variable (never bound as a command by `create`), so
    // neither the instance-command branch nor the
    // class-command branch may fire for it.
    let src = "oo::class create ActiveRecord {\n    classmethod find {args} {}\n}\nset d [ActiveRecord new]\nd find foo bar\n";
    let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
    let locs = definition(src, 4, 2, &analysis);
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

// tcl::OptProc — the `opt` package's automatic-option-parsing proc
// definer: a call site must resolve to the real
// `tcl::OptProc` declaration, never a stale stub.

#[test]
fn opt_proc_call_site_resolves_to_its_declaration() {
    let src = "::tcl::OptProc greet {child -use -display} { return $child }\ngreet foo\n";
    let analysis = analyse(src);
    // Line 1: `greet foo` — cursor on "greet".
    let locs = definition(src, 1, 0, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 0, "{locs:?}");
    assert_eq!(locs[0].start_character, 15, "{locs:?}");
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

#[test]
fn tn_a_proc_declaration_name_still_resolves_to_itself() {
    // TN — a declaration's own name token is an *argument* of `proc`,
    // so the head gate would refuse it; the declaration-site check runs
    // first and must keep answering.
    let src = "proc greet {name} { puts $name }\n";
    let analysis = analyse(src);
    let locs = definition(src, 0, 6, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 0, "{locs:?}");
}

#[test]
fn tp_definition_on_an_implicit_parent_namespace_answers_the_covering_prefix() {
    // `namespace eval ::p::q::r {}` really creates
    // `::p::q` (tclsh 8.6.16 / 9.0.4: `namespace exists ::p::q` is 1),
    // but the name is written nowhere on its own.  The answer is the
    // prefix of the deeper word that spells exactly it.
    let src = "namespace eval ::p::q::r {}\nnamespace children ::p::q\n";
    let analysis = analyse(src);
    // Line 1, col 20 — inside the `::p::q` reference.
    let locs = definition(src, 1, 20, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 0, "{locs:?}");
    assert_eq!(locs[0].start_character, 15, "{locs:?}");
    assert_eq!(
        locs[0].end_character, 21,
        "the answer must stop at `::p::q`, not cover `::p::q::r`: {locs:?}",
    );
}

#[test]
fn tn_a_declared_namespace_still_answers_its_own_block() {
    // TN — the implicit fallback only runs when nothing declares the
    // namespace outright.
    let src = "namespace eval ::p::q {}\nnamespace eval ::p::q::r {}\nnamespace children ::p::q\n";
    let analysis = analyse(src);
    let locs = definition(src, 2, 20, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 0, "{locs:?}");
}
#[test]
fn variable_receiver_never_promotes_a_filewide_candidate_to_runtime_proof() {
    let mut analysis = analyse("");
    analysis
        .instance_classes
        .insert("receiver".to_owned(), "::Stale".to_owned());
    assert!(logical_receiver_candidate_at(&analysis, "receiver", true, 0).is_none());
    analysis
        .created_instance_commands
        .insert("receiver".to_owned());
    assert!(logical_receiver_candidate_at(&analysis, "receiver", false, 0).is_none());
}

#[test]
fn constructed_itcl_member_projection_preserves_colon_tail_and_actual_family_guard() {
    // naming.analysis.constructed-key-consumer-projections
    // docs/design/analysis/name-resolution-proofs/constructed-key-consumer-projections.md
    // Explicit descriptive rows; no native class or reached dispatch issuer.
    use tcl_compiler::analyser::types::{ClassDef, MethodDef};
    let mut analysis = Analyser::new().analyse("", "tcl");
    let member = |name: &str| MethodDef {
        name: name.to_owned(),
        params: Vec::new(),
        formal_count: tcl_compiler::signature_scan::formal_count::SourceFormalCount::Unknown,
        params_computed: true,
        name_span: tcl_lexer::Span::new(0, 0),
        body_span: tcl_lexer::Span::new(0, 0),
        kind: "classmethod".to_owned(),
        is_self_method: false,
        visibility: "public".to_owned(),
        doc: String::new(),
        forward_target: None,
    };
    let mut class = ClassDef {
        qualified_name: "::茶".to_owned(),
        metaclass: "itcl::class".to_owned(),
        ..ClassDef::default()
    };
    class.class_methods.insert(":".to_owned(), member(":"));
    class
        .class_methods
        .insert("méthode".to_owned(), member("méthode"));
    class
        .methods
        .insert("instance".to_owned(), member("instance"));
    analysis
        .all_classes
        .insert(class.qualified_name.clone(), class);
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let registry = crate::registry_for_dialect_profile(profile);
    assert_eq!(
        itcl_class_proc_at_in_registry(&analysis, registry, "::茶:::"),
        Some((&"::茶".to_owned(), ":".to_owned()))
    );
    assert_eq!(
        itcl_class_proc_at_in_registry(&analysis, registry, "::茶::méthode"),
        Some((&"::茶".to_owned(), "méthode".to_owned()))
    );
    for key in [
        "méthode",
        "::茶::",
        "::茶::missing",
        "::茶::instance",
        "::other::méthode",
    ] {
        assert!(
            itcl_class_proc_at_in_registry(&analysis, registry, key).is_none(),
            "{key}"
        );
    }
    analysis.all_classes.get_mut("::茶").unwrap().metaclass = "oo::class".to_owned();
    assert!(itcl_class_proc_at_in_registry(&analysis, registry, "::茶:::").is_none());
}
