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

use crate::interprocedural::test_body_identity::fixture_id_if_present;

use super::*;

fn conditional_representation_report(source: &str) -> Arc<DeclarationFlowReport> {
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let input = analysis.resolved_input.as_ref().unwrap();
    let registry = input.borrowed_context_registry().commands();
    let config = input.lexer_config();
    let realm = analysis.retained_command_realm().unwrap();
    let image = tcl_lexer::SourceImage::document(source);
    let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let mut tokens = CommandTokens::from_segmented(&image.source_map(), config, &segments[0]);
    realm
        .source_bindings_ref()
        .stamp_original_tokens(&mut tokens);
    tokens
        .source_binding
        .as_ref()
        .unwrap()
        .declaration_representation_flow_report(&tokens, input, registry, Some(realm))
        .expect("authentic root representation report")
}

#[test]
fn conditional_root_naming_is_scoped_to_the_original_command() {
    let source = "set s abc; set n [string length $s]; set m $n; lindex $m 0; lindex $n 0";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let input = analysis.resolved_input.as_ref().unwrap();
    let registry = input.borrowed_context_registry().commands();
    let config = input.lexer_config();
    let realm = analysis.retained_command_realm().unwrap();
    let image = tcl_lexer::SourceImage::document(source);
    let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let mut anchor = CommandTokens::from_segmented(&image.source_map(), config, &segments[0]);
    realm
        .source_bindings_ref()
        .stamp_original_tokens(&mut anchor);
    let binding = anchor.source_binding.as_ref().unwrap();
    let inventory = binding.declaration_flow_inventory.as_ref().unwrap();
    let root_site = binding.invocation_site().unwrap();
    let observation = super::super::declaration_layout::original_declaration_layouts(
        inventory.layouts.get(root_site).unwrap(),
    )
    .unwrap()
    .next()
    .unwrap();
    let frame = DeclarationFlowFrame::Existing(observation.entry.as_ref().clone());
    let builder = Builder {
        inventory,
        registry,
        config,
        dialect: observation
            .snapshot
            .state
            .source_variables
            .invocation_dialect
            .unwrap(),
        entry: &frame,
        purpose: DeclarationFlowPurpose::Representation,
        representation_realm: Some(realm),
        representation_loops: Vec::new(),
        current_representation_body: None,
        nodes: Vec::new(),
        jobs: Vec::new(),
        conditional_overwrites: Vec::new(),
        read_sites: BTreeSet::new(),
    };
    let earlier = &segments[3];
    let later = &segments[4];
    let earlier_site = CommandAllocationSite {
        source: Arc::clone(frame.origin()),
        offset: earlier.argv[0].span.start(),
    };
    let later_site = CommandAllocationSite {
        source: Arc::clone(frame.origin()),
        offset: later.argv[0].span.start(),
    };
    let earlier_tokens = CommandTokens::from_segmented(&image.source_map(), config, earlier);
    let later_tokens = CommandTokens::from_segmented(&image.source_map(), config, later);
    assert!(
        builder
            .representation_root_naming_scope(&earlier_tokens, &earlier_site, None)
            .is_some()
    );
    assert!(
        builder
            .representation_root_naming_scope(&later_tokens, &later_site, None)
            .is_none()
    );
    let mut changed = earlier_tokens.clone();
    changed.argv_texts[1] = "$foreign".to_owned();
    assert!(
        builder
            .representation_root_naming_scope(&changed, &earlier_site, None)
            .is_none()
    );
}

#[test]
#[allow(clippy::too_many_lines)] // one end-to-end scenario
fn conditional_representation_head_correspondence_checks_original_receipts() {
    let source = "set n 1";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let input = analysis.resolved_input.as_ref().unwrap();
    let registry = input.borrowed_context_registry().commands();
    let config = input.lexer_config();
    let realm = analysis.retained_command_realm().unwrap();
    let bindings = realm.source_bindings_ref();
    let image = tcl_lexer::SourceImage::document(source);
    let segment =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
    let mut tokens = CommandTokens::from_segmented(&image.source_map(), config, &segment);
    bindings.stamp_original_tokens(&mut tokens);
    let binding = tokens.source_binding.as_ref().unwrap();
    let inventory = binding.declaration_flow_inventory.as_ref().unwrap();
    let site = binding.invocation_site().unwrap().clone();
    let observation = super::super::declaration_layout::original_declaration_layouts(
        inventory.layouts.get(&site).unwrap(),
    )
    .unwrap()
    .next()
    .unwrap();
    let frame = DeclarationFlowFrame::Existing(observation.entry.as_ref().clone());
    let builder = Builder {
        inventory,
        registry,
        config,
        dialect: observation
            .snapshot
            .state
            .source_variables
            .invocation_dialect
            .unwrap(),
        entry: &frame,
        purpose: DeclarationFlowPurpose::Representation,
        representation_realm: Some(realm),
        representation_loops: Vec::new(),
        current_representation_body: None,
        nodes: Vec::new(),
        jobs: Vec::new(),
        conditional_overwrites: Vec::new(),
        read_sites: BTreeSet::new(),
    };
    let (observed, _) = builder.original_layout_selection(&tokens, &site).unwrap();
    let words =
        crate::registry_invocation::source_structure::original_registry_words_for_supplied_tokens(
            realm, input, &tokens,
        )
        .unwrap();
    let original = crate::registry_invocation::declaration_original_words_flow(
        registry,
        &tokens,
        words,
        input.borrowed_context_registry(),
        builder.completion_frame(),
        true,
    )
    .unwrap();
    let scope = builder
        .selection_scopes(&tokens, &site)
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    let advice = super::super::original_site_operand_layout_advice(
        &site,
        &tokens,
        scope.snapshot,
        scope.namespace,
        scope.config,
    )
    .unwrap();
    let check = |source: &crate::registry_invocation::DeclarationInvocationFlow,
                 observed: &crate::registry_invocation::DeclarationInvocationFlow| {
        crate::registry_invocation::declaration_source_flow_corresponds(
            registry,
            &tokens,
            &advice,
            source,
            observed,
            input.borrowed_context_registry(),
        )
        .is_some()
    };
    assert_ne!(
        original.effective.words.first(),
        observed.effective.words.first()
    );
    assert!(check(&original, &observed));
    let mut changed = original.clone();
    if let crate::ir::WordExpr::Literal { text, .. } = &mut changed.effective.words[0] {
        *text = "foreign".to_owned();
    } else {
        panic!("expected source head");
    }
    assert!(!check(&changed, &observed));
    let mut changed = original.clone();
    if let crate::ir::WordExpr::Literal { source, .. } = &mut changed.effective.words[0] {
        *source = crate::ir::SourceSite::opaque(tcl_lexer::Span::new(1, 2));
    } else {
        panic!("expected source head");
    }
    assert!(!check(&changed, &observed));
    let mut changed = original.clone();
    changed.effective.words[1] = changed.effective.words[2].clone();
    assert!(!check(&changed, &observed));
    let mut changed = original.clone();
    changed.effective.origins[0] = crate::registry_invocation::InvocationWordOrigin::Written(0);
    assert!(!check(&changed, &observed));
    let mut changed = original.clone();
    changed
        .effective
        .binding_prefix
        .push(crate::registry_invocation::EffectiveInvocationWord::Literal("foreign".to_owned()));
    assert!(!check(&changed, &observed));
    let foreign = crate::analyser::Analyser::new().analyse(source, "tcl9.0");
    let foreign_input = foreign.resolved_input.as_ref().unwrap();
    assert!(!std::ptr::eq(
        input.borrowed_context_registry().context(),
        foreign_input.borrowed_context_registry().context(),
    ));
    assert!(
        crate::registry_invocation::declaration_source_flow_corresponds(
            registry,
            &tokens,
            &advice,
            &original,
            &observed,
            foreign_input.borrowed_context_registry(),
        )
        .is_none()
    );
}

#[test]
fn conditional_loop_indexes_and_appends_retain_source_events() {
    let source = "set ys {}; set pts {}; foreach pt {{1 2} {3 4} {5 6}} {set y [lindex $pt 1]; lappend ys $y; lappend pts $pt}";
    let report = conditional_representation_report(source);
    let offset = |text: &str| u32::try_from(source.find(text).unwrap()).unwrap();
    let nested_context = report
        .conditional_body_contexts
        .iter()
        .find(|context| {
            context.substitution.as_ref().is_some_and(|substitution| {
                substitution.original.command().span.start() == offset("lindex")
            })
        })
        .expect("the nested index retains its original substitution context");
    let substitution = nested_context.substitution.as_ref().unwrap();
    assert_eq!(substitution.parent.site.offset, offset("foreach"));
    assert_eq!(substitution.parent_site.offset, offset("set y ["));
    assert!(
        report
            .conditional_substitution_entries
            .contains(&offset("lindex"))
    );
    representation::assert_body_context_state_laws(nested_context, &nested_context.words);
    assert!(report.conditional_loop_sites.contains(&offset("foreach")));
    assert!(
        report
            .conditional_literal_index_sites
            .contains(&offset("lindex"))
    );
    assert!(
        report
            .conditional_list_append_sites
            .contains(&offset("lappend ys"))
    );
    assert!(
        report
            .conditional_list_append_sites
            .contains(&offset("lappend pts"))
    );
    let index = report
        .conditional_list_events
        .iter()
        .find(|event| {
            event.site == offset("lindex") && event.kind == representation::ListEventKind::Index
        })
        .expect("actual index state");
    assert!(!index.blocked_before && !index.blocked_after, "{index:?}");
    assert!(index.result_representation.is_none() && !index.result_is_integer);
    for text in ["lappend ys", "lappend pts"] {
        let event = report
            .conditional_list_events
            .iter()
            .find(|event| {
                event.site == offset(text) && event.kind == representation::ListEventKind::Append
            })
            .unwrap_or_else(|| panic!("missing {text}: {:?}", report.conditional_list_events));
        assert!(!event.blocked_before && !event.blocked_after, "{event:?}");
        assert_eq!(
            event.stored_representation,
            Some(tcl_registry::TclType::List)
        );
        assert_eq!(
            event.result_representation,
            Some(tcl_registry::TclType::List)
        );
        assert!(!event.result_is_integer);
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one end-to-end scenario
fn conditional_substitution_context_requires_exact_single_child() {
    let source =
        "set marker 0; foreach pt {{1 2} {3 4}} {set y [  lindex $pt 1  ]; set z [lindex $pt 1]}";
    let report = conditional_representation_report(source);
    let first_offset = u32::try_from(source.find("lindex").unwrap()).unwrap();
    let context = report
        .conditional_body_contexts
        .iter()
        .find(|context| {
            context.substitution.as_ref().is_some_and(|substitution| {
                substitution.original.command().span.start() == first_offset
            })
        })
        .expect("the original first substitution context");
    let config = context.input.lexer_config();
    let image = tcl_lexer::SourceImage::document(source);
    let make_child = |offset| {
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            "lindex $pt 1",
            offset,
            config,
        )
        .remove(0);
        (
            CommandTokens::from_segmented(&image.source_map(), config, &segment),
            CommandAllocationSite {
                source: Arc::clone(&context.site.source),
                offset,
            },
        )
    };
    let (child, site) = make_child(first_offset);
    assert!(Builder::conditional_substitution_matches(
        context, &child, &site, config
    ));
    let sibling_offset = u32::try_from(source.rfind("lindex").unwrap()).unwrap();
    let (sibling, sibling_site) = make_child(sibling_offset);
    assert!(!Builder::conditional_substitution_matches(
        context,
        &sibling,
        &sibling_site,
        config
    ));
    let mut changed = child.clone();
    changed.argv_texts[1] = "foreign".to_owned();
    assert!(!Builder::conditional_substitution_matches(
        context, &changed, &site, config
    ));
    let mut wrong_parent = context.as_ref().clone();
    wrong_parent
        .substitution
        .as_mut()
        .unwrap()
        .parent_site
        .offset += 1;
    assert!(!Builder::conditional_substitution_matches(
        &wrong_parent,
        &child,
        &site,
        config
    ));
    let mut wrong_operand = context.as_ref().clone();
    wrong_operand.substitution.as_mut().unwrap().operand += 1;
    assert!(!Builder::conditional_substitution_matches(
        &wrong_operand,
        &child,
        &site,
        config
    ));
    let sibling_context = report
        .conditional_body_contexts
        .iter()
        .find(|candidate| {
            candidate.substitution.as_ref().is_some_and(|substitution| {
                substitution.original.command().span.start() == sibling_offset
            })
        })
        .expect("independently issued sibling substitution");
    let mut sibling_words = context.as_ref().clone();
    sibling_words.substitution.as_mut().unwrap().parent_words =
        Arc::clone(&sibling_context.substitution.as_ref().unwrap().parent_words);
    assert!(!Builder::conditional_substitution_matches(
        &sibling_words,
        &child,
        &site,
        config
    ));
    let foreign_report = conditional_representation_report(source);
    let foreign_context = foreign_report
        .conditional_body_contexts
        .iter()
        .find(|candidate| {
            candidate.substitution.as_ref().is_some_and(|substitution| {
                substitution.original.command().span.start() == first_offset
            })
        })
        .expect("independently issued equal-text substitution");
    assert!(!Arc::ptr_eq(&foreign_context.owner, &context.owner));
    let mut foreign_words = context.as_ref().clone();
    foreign_words.substitution.as_mut().unwrap().parent_words =
        Arc::clone(&foreign_context.substitution.as_ref().unwrap().parent_words);
    assert!(!Builder::conditional_substitution_matches(
        &foreign_words,
        &child,
        &site,
        config
    ));
    let foreign_site = CommandAllocationSite {
        source: Arc::new(site.source.as_ref().clone()),
        offset: site.offset,
    };
    assert!(!Arc::ptr_eq(&foreign_site.source, &site.source));
    assert!(!Builder::conditional_substitution_matches(
        context,
        &child,
        &foreign_site,
        config
    ));
    let mut foreign_config = config;
    foreign_config.expand_syntax = !foreign_config.expand_syntax;
    assert!(!Builder::conditional_substitution_matches(
        context,
        &child,
        &site,
        foreign_config
    ));
    let foreign = crate::analyser::Analyser::new().analyse(source, "tcl9.0");
    let mut foreign_input = context.as_ref().clone();
    foreign_input.input = foreign.resolved_input.unwrap();
    assert!(!Builder::conditional_substitution_matches(
        &foreign_input,
        &child,
        &site,
        config
    ));
    let mut foreign_policy = context.as_ref().clone();
    foreign_policy.policy =
        tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V9_0);
    assert!(!Builder::conditional_substitution_matches(
        &foreign_policy,
        &child,
        &site,
        config
    ));
}

#[test]
fn conditional_substitution_refuses_compound_and_multiple_commands() {
    for source in [
        "set marker 0; foreach pt {{1 2} {3 4}} {set y pre[lindex $pt 1]}",
        "set marker 0; foreach pt {{1 2} {3 4}} {set y [lindex $pt 1; lindex $pt 0]}",
        "set marker 0; foreach pt {{1 2} {3 4}} {set y {*}[lindex $pt 1]}",
        "set marker 0; foreach pt {{1 2} {3 4}} {set y $a([lindex $pt 1])}",
    ] {
        let report = conditional_representation_report(source);
        assert!(
            report
                .conditional_body_contexts
                .iter()
                .all(|context| context.substitution.is_none()),
            "unsupported operand must not acquire a child context: {source}"
        );
    }
}

#[test]
fn conditional_substitution_does_not_reenter_after_unknown_operation() {
    let source = "set marker 0; foreach pt {{1 2} {3 4}} {unknown_command; set y [lindex $pt 1]}";
    let report = conditional_representation_report(source);
    let offset = u32::try_from(source.find("lindex").unwrap()).unwrap();
    assert!(
        report
            .conditional_list_events
            .iter()
            .all(|event| { event.site != offset || event.blocked_before || event.blocked_after }),
        "unknown operation must prevent a later unblocked index event"
    );
}

#[test]
fn conditional_append_reads_the_target_after_nested_operands() {
    use representation::ListEventKind;
    for (source, expected_input, blocked) in [
        (
            "set out \"{\"; lappend out [set out {}]",
            b"".as_slice(),
            false,
        ),
        (
            "set out {}; lappend out [set out \"{\"]",
            b"{".as_slice(),
            true,
        ),
    ] {
        let report = conditional_representation_report(source);
        let offset = u32::try_from(source.find("lappend").unwrap()).unwrap();
        let event = report
            .conditional_list_events
            .iter()
            .find(|event| event.site == offset && event.kind == ListEventKind::Append)
            .unwrap_or_else(|| {
                panic!(
                    "missing append event for {source}: {:?}",
                    report.conditional_list_events
                )
            });
        assert!(!event.blocked_before, "{source}: {event:?}");
        assert_eq!(
            event.input_literal.as_deref(),
            Some(expected_input),
            "{source}: {event:?}"
        );
        assert!(
            event
                .operand_literals
                .iter()
                .any(|(operand, bytes)| { *operand == 1 && bytes == expected_input }),
            "{source}: {event:?}"
        );
        assert_eq!(event.blocked_after, blocked, "{source}: {event:?}");
        if !blocked {
            assert_eq!(
                event.stored_representation,
                Some(tcl_registry::TclType::List)
            );
            assert_eq!(
                event.result_representation,
                Some(tcl_registry::TclType::List)
            );
            assert!(event.result_literal.is_none());
            assert!(!event.result_is_integer);
        }
    }
}

#[test]
fn conditional_literal_index_keeps_unknown_results_distinct() {
    use representation::ListEventKind;
    for (source, input, result) in [
        (
            "set values {a b}; set x [lindex $values 1]",
            Some(b"a b".as_slice()),
            Some(b"b".as_slice()),
        ),
        (
            "set s abc; set values [string length $s]; set x [lindex $values 1]",
            None,
            None,
        ),
    ] {
        let report = conditional_representation_report(source);
        let offset = u32::try_from(source.find("lindex").unwrap()).unwrap();
        let event = report
            .conditional_list_events
            .iter()
            .find(|event| event.site == offset && event.kind == ListEventKind::Index)
            .unwrap_or_else(|| {
                panic!(
                    "missing index event for {source}: {:?}",
                    report.conditional_list_events
                )
            });
        assert!(
            !event.blocked_before && !event.blocked_after,
            "{source}: {event:?}"
        );
        assert_eq!(event.input_literal.as_deref(), input, "{source}: {event:?}");
        assert_eq!(
            event.result_literal.as_deref(),
            result,
            "{source}: {event:?}"
        );
        assert!(event.result_representation.is_none() && !event.result_is_integer);
    }
}

#[test]
fn conditional_loop_receipts_require_known_count_and_original_binder() {
    for (source, expected) in [
        ("set x 0; foreach i {1 2 3} {incr x; lindex $x 0}", true),
        ("set x 0; foreach i {} {incr x; lindex $x 0}", false),
        ("set x 0; foreach i {1} {incr x; lindex $x 0}", false),
        ("set x 0; foreach i $unknown {incr x; lindex $x 0}", false),
        (
            "set x 0; foreach {i j} {1 2 3} {incr x; lindex $x 0}",
            false,
        ),
        (
            "set x 0; foreach i {1 2 3} j {4 5} {incr x; lindex $x 0}",
            false,
        ),
    ] {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let input = analysis.resolved_input.as_ref().unwrap();
        let registry = input.borrowed_context_registry().commands();
        let config = input.lexer_config();
        let realm = analysis.retained_command_realm().unwrap();
        let image = tcl_lexer::SourceImage::document(source);
        let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let mut tokens = CommandTokens::from_segmented(&image.source_map(), config, &segments[0]);
        realm
            .source_bindings_ref()
            .stamp_original_tokens(&mut tokens);
        let report = tokens
            .source_binding
            .as_ref()
            .unwrap()
            .declaration_representation_flow_report(&tokens, input, registry, Some(realm))
            .expect("authentic root representation report");
        let offset = u32::try_from(source.find("foreach").unwrap()).unwrap();
        assert_eq!(
            report.conditional_loop_sites.contains(&offset),
            expected,
            "{source}"
        );
        if expected {
            assert_eq!(
                report.conditional_body_contexts.len(),
                1,
                "authentic immediate body context: {source}"
            );
            let context = &report.conditional_body_contexts[0];
            assert_eq!(context.site.offset, offset);
            representation::assert_body_context_state_laws(context, &context.words);
            assert!(
                report.representation_occurrences.iter().any(|observation| {
                    observation
                        .body_contexts()
                        .iter()
                        .any(|held| held == context)
                }),
                "conditional conversion retains its actual body context: {source}"
            );
            let (_, normal, error, returned, other) = report
                .conditional_loop_exits
                .iter()
                .find(|(site, ..)| *site == offset)
                .copied()
                .expect("original loop exits");
            let exits = [normal, error, returned, other];
            for (index, exit) in exits.iter().enumerate() {
                assert!(
                    !exits[..index].contains(exit),
                    "distinct normal/error/return/other exits"
                );
            }
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one end-to-end scenario
fn conditional_body_context_requires_exact_original_loop_and_source() {
    let source = "set x 0; foreach i {1 2 3} {incr x; lindex $x 0; if {1} {incr x}}; foreach j {1 2 3} {incr x}";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let input = analysis.resolved_input.as_ref().unwrap();
    let registry = input.borrowed_context_registry().commands();
    let config = input.lexer_config();
    let realm = analysis.retained_command_realm().unwrap();
    let image = tcl_lexer::SourceImage::document(source);
    let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let mut anchor = CommandTokens::from_segmented(&image.source_map(), config, &segments[0]);
    realm
        .source_bindings_ref()
        .stamp_original_tokens(&mut anchor);
    let binding = anchor.source_binding.as_ref().unwrap();
    let inventory = binding.declaration_flow_inventory.as_ref().unwrap();
    let root_site = binding.invocation_site().unwrap();
    let observation = super::super::declaration_layout::original_declaration_layouts(
        inventory.layouts.get(root_site).unwrap(),
    )
    .unwrap()
    .next()
    .unwrap();
    let frame = DeclarationFlowFrame::Existing(observation.entry.as_ref().clone());
    let report = binding
        .declaration_representation_flow_report(&anchor, input, registry, Some(realm))
        .unwrap();
    let loop_offset = u32::try_from(source.find("foreach i").unwrap()).unwrap();
    let context = Arc::clone(
        report
            .conditional_body_contexts
            .iter()
            .find(|context| context.site.offset == loop_offset)
            .expect("original immediate body receipt"),
    );
    let offset = u32::try_from(source.find("incr x").unwrap()).unwrap();
    let segment =
        crate::segmenter::segment_commands_with_offset_and_config("incr x", offset, config)
            .remove(0);
    let child = CommandTokens::from_segmented(&image.source_map(), config, &segment);
    let site = CommandAllocationSite {
        source: Arc::clone(frame.origin()),
        offset,
    };
    let mut builder = Builder {
        inventory,
        registry,
        config,
        dialect: observation
            .snapshot
            .state
            .source_variables
            .invocation_dialect
            .unwrap(),
        entry: &frame,
        purpose: DeclarationFlowPurpose::Representation,
        representation_realm: Some(realm),
        representation_loops: Vec::new(),
        current_representation_body: Some(Arc::clone(&context)),
        nodes: Vec::new(),
        jobs: Vec::new(),
        conditional_overwrites: Vec::new(),
        read_sites: BTreeSet::new(),
    };
    let selected = builder
        .source_read_selection(&child, &site)
        .expect("original child source selection");
    assert!(
        builder
            .conditional_representation_body_scope(&child, &site, &selected)
            .is_some()
    );
    let mut changed_words = child.clone();
    changed_words.argv_texts[1] = "foreign".to_owned();
    assert!(
        builder
            .conditional_representation_body_scope(&changed_words, &site, &selected)
            .is_none()
    );
    let mut wrong_parent = context.as_ref().clone();
    wrong_parent.site.offset += 1;
    builder.current_representation_body = Some(Arc::new(wrong_parent));
    assert!(
        builder
            .conditional_representation_body_scope(&child, &site, &selected)
            .is_none()
    );
    let foreign_analysis = crate::analyser::Analyser::new().analyse(source, "tcl9.0");
    let mut foreign_input = context.as_ref().clone();
    foreign_input.input = foreign_analysis.resolved_input.unwrap();
    builder.current_representation_body = Some(Arc::new(foreign_input));
    assert!(
        builder
            .conditional_representation_body_scope(&child, &site, &selected)
            .is_none()
    );
    let foreign = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let foreign_realm = foreign.retained_command_realm().unwrap();
    let mut foreign_anchor =
        CommandTokens::from_segmented(&image.source_map(), config, &segments[0]);
    foreign_realm
        .source_bindings_ref()
        .stamp_original_tokens(&mut foreign_anchor);
    let foreign_owner = foreign_anchor
        .source_binding
        .as_ref()
        .unwrap()
        .declaration_flow_inventory
        .as_ref()
        .unwrap()
        .source_seed
        .as_ref()
        .unwrap()
        .analysis_owner
        .as_ref()
        .unwrap();
    assert!(!Arc::ptr_eq(foreign_owner, &context.owner));
    let mut foreign_context = context.as_ref().clone();
    foreign_context.owner = Arc::clone(foreign_owner);
    builder.current_representation_body = Some(Arc::new(foreign_context));
    assert!(
        builder
            .conditional_representation_body_scope(&child, &site, &selected)
            .is_none()
    );
    builder.current_representation_body = Some(Arc::clone(&context));
    let nested = source.find("if {1} {incr x}").unwrap() + "if {1} {".len();
    let sibling = source.rfind("incr x").unwrap();
    for (label, raw_offset) in [("nested", nested), ("sibling", sibling)] {
        let offset = u32::try_from(raw_offset).unwrap();
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config("incr x", offset, config)
                .remove(0);
        let other = CommandTokens::from_segmented(&image.source_map(), config, &segment);
        let other_site = CommandAllocationSite {
            source: Arc::clone(frame.origin()),
            offset,
        };
        let other_flow = builder
            .source_read_selection(&other, &other_site)
            .expect("genuine independently selected source child");
        assert!(
            builder
                .conditional_representation_body_scope(&other, &other_site, &other_flow)
                .is_none(),
            "{label} body cannot borrow the first loop's receipt"
        );
    }
    let mut known_observer = selected.clone();
    known_observer.representation_no_known_counterevidence = false;
    assert!(
        builder
            .conditional_representation_body_scope(&child, &site, &known_observer)
            .is_none()
    );
    builder.current_representation_body = None;
    assert!(
        builder
            .conditional_representation_body_scope(&child, &site, &selected)
            .is_none()
    );
    let mut changed = inventory.layouts.as_ref().clone();
    let alternate =
        tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V9_0);
    assert_ne!(alternate, context.policy);
    for row in changed.get_mut(&site).unwrap() {
        let mut state = row.snapshot.state.clone();
        Arc::make_mut(&mut state.baseline).execution_name_policy = Some(
            tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe(alternate),
        );
        row.snapshot = Arc::new(super::super::SourceLookupSnapshot::in_realm(
            state,
            row.snapshot.realm,
        ));
    }
    let mut changed_inventory = DeclarationFlowInventory::new(
        Arc::new(changed),
        Arc::clone(&inventory.accesses),
        Arc::clone(&inventory.origin_accesses),
        Arc::clone(&inventory.points),
        Arc::clone(&inventory.origin_points),
    );
    changed_inventory.source_seed = inventory.source_seed.clone();
    let changed_builder = Builder {
        inventory: &changed_inventory,
        current_representation_body: Some(Arc::clone(&context)),
        ..builder
    };
    assert!(
        changed_builder
            .source_read_point_scope(&child, &site)
            .is_some()
    );
    assert_eq!(
        changed_builder
            .source_read_selection(&child, &site)
            .as_ref(),
        Some(&selected)
    );
    assert!(
        changed_builder
            .conditional_representation_body_scope(&child, &site, &selected)
            .is_none()
    );
}

#[test]
fn conditional_root_representation_flow_covers_reported_foreach() {
    let source = "set ys {}; set pts {}; foreach pt {{1 2} {3 4} {5 6}} {set y [lindex $pt 1]; if {$y > 2} {lappend ys $y; lappend pts $pt}}";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let input = analysis.resolved_input.as_ref().unwrap();
    let registry = input.borrowed_context_registry().commands();
    let config = input.lexer_config();
    let realm = analysis.retained_command_realm().unwrap();
    let bindings = realm.source_bindings_ref();
    let image = tcl_lexer::SourceImage::document(source);
    let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let mut tokens = CommandTokens::from_segmented(&image.source_map(), config, &segments[0]);
    bindings.stamp_original_tokens(&mut tokens);
    let report = tokens
        .source_binding
        .as_ref()
        .unwrap()
        .declaration_representation_flow_report(&tokens, input, registry, Some(realm))
        .expect("the original root input owns the conditional representation graph");
    for command in [
        "foreach pt",
        "set y [lindex",
        "lindex $pt 1",
        "if {$y",
        "lappend ys $y",
        "lappend pts $pt",
    ] {
        let offset = u32::try_from(source.find(command).unwrap()).unwrap();
        assert!(
            report.conditional_handler_sites.contains(&offset),
            "missing conditional source handler at {command}"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one end-to-end scenario
fn conditional_root_source_scope_uses_its_original_parent_and_input() {
    let source = "set ys {}; foreach pt {{1 2} {3 4}} {lappend ys $pt}";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let input = analysis.resolved_input.as_ref().unwrap();
    let registry = input.borrowed_context_registry().commands();
    let config = input.lexer_config();
    let realm = analysis.retained_command_realm().unwrap();
    let bindings = realm.source_bindings_ref();
    let image = tcl_lexer::SourceImage::document(source);
    let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let mut anchor = CommandTokens::from_segmented(&image.source_map(), config, &segments[0]);
    bindings.stamp_original_tokens(&mut anchor);
    let binding = anchor.source_binding.as_ref().unwrap();
    let inventory = binding.declaration_flow_inventory.as_ref().unwrap();
    let observation = super::super::declaration_layout::original_declaration_layouts(
        inventory
            .layouts
            .get(binding.invocation_site().unwrap())
            .unwrap(),
    )
    .unwrap()
    .next()
    .unwrap();
    let frame = DeclarationFlowFrame::Existing(observation.entry.as_ref().clone());
    assert!(matches!(
        frame,
        DeclarationFlowFrame::Existing(
            super::super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript { .. }
        )
    ));
    let dialect = observation
        .snapshot
        .state
        .source_variables
        .invocation_dialect
        .unwrap();
    let offset = u32::try_from(source.find("lappend").unwrap()).unwrap();
    let segment =
        crate::segmenter::segment_commands_with_offset_and_config("lappend ys $pt", offset, config)
            .remove(0);
    let child = CommandTokens::from_segmented(&image.source_map(), config, &segment);
    let site = CommandAllocationSite {
        source: Arc::clone(frame.origin()),
        offset,
    };
    macro_rules! builder {
        ($purpose:expr, $inventory:expr) => {
            Builder {
                inventory: $inventory,
                registry,
                config,
                dialect,
                entry: &frame,
                purpose: $purpose,
                representation_realm: None,
                representation_loops: Vec::new(),
                current_representation_body: None,
                nodes: Vec::new(),
                jobs: Vec::new(),
                conditional_overwrites: Vec::new(),
                read_sites: BTreeSet::new(),
            }
        };
    }
    assert!(
        builder!(DeclarationFlowPurpose::Reads, inventory)
            .source_read_selection(&child, &site)
            .is_none()
    );
    let root_builder = builder!(DeclarationFlowPurpose::Representation, inventory);
    let related = root_builder
        .source_read_point_scope(&child, &site)
        .expect("actual nested activation is related to the root");
    assert!(
        matches!(related, DeclarationReadPointScope::ConditionalRootRelated { ref bodies, .. }
            if !bodies.is_empty() && bodies.iter().all(|span| *span == tcl_lexer::Span::new(offset, offset + 14)))
    );
    let selected = builder!(DeclarationFlowPurpose::Representation, inventory)
        .source_read_selection(&child, &site)
        .expect("conditional original body schema");
    assert!(selected.accumulator_writes.iter().any(|name| name == "ys"));
    assert!(
        root_builder
            .representation_root_naming_scope(&child, &site, Some(&selected))
            .is_some()
    );
    assert!(
        root_builder
            .representation_root_naming_scope(&child, &site, None)
            .is_none()
    );
    let mut changed_child = child.clone();
    changed_child.argv_texts[1] = "foreign".to_owned();
    assert!(
        root_builder
            .representation_root_naming_scope(&changed_child, &site, Some(&selected))
            .is_none()
    );
    let mut changed_flow = selected.clone();
    changed_flow.effects.unknown_writes = true;
    assert!(
        root_builder
            .representation_root_naming_scope(&child, &site, Some(&changed_flow))
            .is_none()
    );
    let foreign_analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let foreign_realm = foreign_analysis.retained_command_realm().unwrap();
    assert!(
        binding
            .declaration_representation_flow_report(&anchor, input, registry, Some(foreign_realm))
            .is_none(),
        "equal text from a different source owner cannot warm the report cache"
    );
    let report = binding
        .declaration_representation_flow_report(&anchor, input, registry, Some(realm))
        .expect("same original analysis owns the conditional report");
    assert!(
        report.conditional_handler_sites.contains(&offset),
        "the body handler must be modelled"
    );
    assert!(
        binding
            .declaration_representation_flow_report(&anchor, input, registry, Some(realm))
            .is_some(),
        "the original source owner remains usable after a foreign request"
    );
    assert!(
        binding
            .declaration_representation_flow_report(&anchor, input, registry, Some(foreign_realm))
            .is_none(),
        "a cached original report cannot be borrowed by a foreign owner"
    );

    let mut synthetic = child.clone();
    synthetic.synthetic = Some(crate::ir::SyntheticMarker::Condition);
    assert!(
        builder!(DeclarationFlowPurpose::Representation, inventory)
            .source_read_selection(&synthetic, &site)
            .is_none()
    );
    let mut changed = child.clone();
    changed.argv_texts[1] = "foreign".to_owned();
    assert!(
        builder!(DeclarationFlowPurpose::Representation, inventory)
            .source_read_selection(&changed, &site)
            .is_none()
    );

    let mut missing = DeclarationFlowInventory::new(
        Arc::new(BTreeMap::new()),
        Arc::clone(&inventory.accesses),
        Arc::clone(&inventory.origin_accesses),
        Arc::clone(&inventory.points),
        Arc::clone(&inventory.origin_points),
    );
    missing.source_seed = inventory.source_seed.clone();
    assert!(
        builder!(DeclarationFlowPurpose::Representation, &missing)
            .source_read_selection(&child, &site)
            .is_none()
    );
    let mut corrupt = inventory.layouts.as_ref().clone();
    corrupt.get_mut(&site).unwrap()[0].words = Arc::from([]);
    let mut corrupt = DeclarationFlowInventory::new(
        Arc::new(corrupt),
        Arc::clone(&inventory.accesses),
        Arc::clone(&inventory.origin_accesses),
        Arc::clone(&inventory.points),
        Arc::clone(&inventory.origin_points),
    );
    corrupt.source_seed = inventory.source_seed.clone();
    assert!(
        builder!(DeclarationFlowPurpose::Representation, &corrupt)
            .source_read_selection(&child, &site)
            .is_none()
    );
    for axis in 0..3 {
        let mut changed = inventory.layouts.as_ref().clone();
        for row in changed.get_mut(&site).unwrap() {
            let mut state = row.snapshot.state.clone();
            let baseline = Arc::make_mut(&mut state.baseline);
            match axis {
                0 => baseline.unknown_entry = true,
                1 => baseline.registry_snapshot = None,
                _ => {
                    baseline.metadata_context =
                        crate::registry_invocation::OwnedInvocationMetadataContext::Standalone;
                }
            }
            row.snapshot = Arc::new(super::super::SourceLookupSnapshot::in_realm(
                state,
                row.snapshot.realm,
            ));
        }
        let mut changed = DeclarationFlowInventory::new(
            Arc::new(changed),
            Arc::clone(&inventory.accesses),
            Arc::clone(&inventory.origin_accesses),
            Arc::clone(&inventory.points),
            Arc::clone(&inventory.origin_points),
        );
        changed.source_seed = inventory.source_seed.clone();
        assert!(
            builder!(DeclarationFlowPurpose::Representation, &changed)
                .source_read_selection(&child, &site)
                .is_none(),
            "changed child baseline axis {axis}"
        );
    }
    let mut no_parent = inventory.layouts.as_ref().clone();
    let parent_site = CommandAllocationSite {
        source: Arc::clone(&site.source),
        offset: u32::try_from(source.find("foreach").unwrap()).unwrap(),
    };
    assert!(no_parent.remove(&parent_site).is_some());
    let mut no_parent = DeclarationFlowInventory::new(
        Arc::new(no_parent),
        Arc::clone(&inventory.accesses),
        Arc::clone(&inventory.origin_accesses),
        Arc::clone(&inventory.points),
        Arc::clone(&inventory.origin_points),
    );
    no_parent.source_seed = inventory.source_seed.clone();
    assert!(
        builder!(DeclarationFlowPurpose::Representation, &no_parent)
            .source_read_selection(&child, &site)
            .is_none()
    );

    for axis in 0..4 {
        let mut changed = inventory.layouts.as_ref().clone();
        for row in changed.get_mut(&site).unwrap() {
            let mut entry = row.entry.as_ref().clone();
            let super::super::declaration_layout::OriginalDiagnosticFrameEntry::ScriptActivation {
                source: activation,
                frame,
                namespace,
                config,
            } = &mut entry
            else {
                panic!("genuine script activation expected");
            };
            match axis {
                0 => {
                    *activation = ExecutedScriptSource::contiguous_image(
                        Arc::clone(&site.source),
                        site.source.source_image().clone(),
                        0,
                    )
                    .unwrap();
                }
                1 => {
                    *frame = crate::var_resolve::VariableExecutionFrame::Namespace(
                        "::foreign".to_owned(),
                    );
                }
                2 => *namespace = super::super::SourceNamespaceKey::authored("::foreign"),
                _ => config.expand_syntax = !config.expand_syntax,
            }
            row.entry = Arc::new(entry);
        }
        let mut changed = DeclarationFlowInventory::new(
            Arc::new(changed),
            Arc::clone(&inventory.accesses),
            Arc::clone(&inventory.origin_accesses),
            Arc::clone(&inventory.points),
            Arc::clone(&inventory.origin_points),
        );
        changed.source_seed = inventory.source_seed.clone();
        if axis == 0 {
            assert!(
                builder!(DeclarationFlowPurpose::Representation, &changed)
                    .source_read_point_scope(&child, &site)
                    .is_some(),
                "wider source reaches the ancestry gate"
            );
        }
        assert!(
            builder!(DeclarationFlowPurpose::Representation, &changed)
                .source_read_selection(&child, &site)
                .is_none(),
            "changed activation axis {axis}"
        );
    }
    let mut foreign = site.clone();
    foreign.source = Arc::new(super::super::SourceOriginId::authored_image(image.clone()));
    assert!(
        builder!(DeclarationFlowPurpose::Representation, inventory)
            .source_read_selection(&child, &foreign)
            .is_none()
    );
}

#[test]
fn lifecycle_subcommand_schedules_its_original_phase_scripts() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let mut registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
    let mut descriptor = registry.get("tcltest::test").unwrap().clone();
    descriptor.name = "offset-lifecycle";
    descriptor.required_package = None;
    descriptor.subcommands = Box::leak(Box::new([tcl_registry::SubCommand {
        name: "run",
        arity: descriptor.arity,
        arg_role_resolver: descriptor.arg_role_resolver,
        arg_role_resolver_roles: descriptor.arg_role_resolver_roles,
        body_kind: descriptor.body_kind,
        body_execution: descriptor.body_execution,
        completion: descriptor.completion,
        ..tcl_registry::SubCommand::DEFAULT
    }]));
    registry.insert(descriptor);
    let loader = crate::lowering::stock_body_provider_loader(
        tcl_registry::body_execution::TCLTEST_STOCK_PROVIDER,
        None,
    )
    .unwrap();
    let wrapper =
        "offset-lifecycle run n d -cleanup {puts CLEANUP} -body {puts BODY} -setup {puts SETUP}";
    let source = format!("package require tcltest\nproc p {{}} {{{wrapper}}}");
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let bindings = super::super::SourceCommandBindings::analyse_with_options(
        &source,
        config,
        &registry,
        super::super::SourceAnalysisOptions {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            trusted_package_loaders: std::slice::from_ref(&loader),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        },
    );
    let offset = u32::try_from(source.find(wrapper).unwrap()).unwrap();
    let segment =
        crate::segmenter::segment_commands_with_offset_and_config(wrapper, offset, config)
            .remove(0);
    let mut tokens = CommandTokens::from_segmented(
        &tcl_lexer::SourceImage::document(&source).source_map(),
        config,
        &segment,
    );
    bindings.stamp_original_tokens(&mut tokens);
    let binding = tokens.source_binding.as_ref().unwrap();
    let site = binding.invocation_site().unwrap();
    let inventory = binding.declaration_flow_inventory.as_ref().unwrap();
    let observation = super::super::declaration_layout::original_declaration_layouts(
        inventory.layouts.get(site).unwrap(),
    )
    .unwrap()
    .next()
    .unwrap();
    let frame = DeclarationFlowFrame::Existing(observation.entry.as_ref().clone());
    let mut builder = Builder {
        inventory,
        registry: &registry,
        config,
        dialect: tcl_registry::InvocationDialect::of_profile(profile),
        entry: &frame,
        purpose: DeclarationFlowPurpose::Reads,
        representation_realm: None,
        representation_loops: Vec::new(),
        current_representation_body: None,
        nodes: Vec::new(),
        jobs: Vec::new(),
        conditional_overwrites: Vec::new(),
        read_sites: BTreeSet::new(),
    };
    let (selected, lifecycle) = builder.selection(&tokens, site).unwrap();
    let lifecycle = lifecycle.unwrap();
    assert_eq!(lifecycle.invocation.argument_offset, 1);
    let after = builder.placeholder();
    builder.lifecycle_flow(site, &selected, &lifecycle, after, Exits::all(after));
    let scripts = builder
        .jobs
        .iter()
        .map(|job| job.source.text.try_text().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(scripts, ["puts CLEANUP", "puts BODY", "puts SETUP"]);
}

#[test]
fn declaration_selection_keeps_absent_dispatch_separate_from_invalid_reached_owner() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    let source = "set anchor 1; puts $absent";
    let bindings = super::super::SourceCommandBindings::analyse(source, config, registry);
    let segment =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
    let mut tokens = CommandTokens::from_segmented(
        &tcl_lexer::SourceImage::document(source).source_map(),
        config,
        &segment,
    );
    bindings.stamp_original_tokens(&mut tokens);
    let binding = tokens.source_binding.as_ref().unwrap();
    let site = binding.invocation_site().unwrap();
    let inventory = binding.declaration_flow_inventory.as_ref().unwrap();
    let observation = super::super::declaration_layout::original_declaration_layouts(
        inventory.layouts.get(site).unwrap(),
    )
    .unwrap()
    .next()
    .unwrap();
    let dialect = observation
        .snapshot
        .state
        .source_variables
        .invocation_dialect
        .unwrap();
    let frame = DeclarationFlowFrame::Existing(observation.entry.as_ref().clone());
    let selection = |inventory| {
        let builder = Builder {
            inventory,
            registry,
            config,
            dialect,
            entry: &frame,
            purpose: DeclarationFlowPurpose::Reads,
            representation_realm: None,
            representation_loops: Vec::new(),
            current_representation_body: None,
            nodes: Vec::new(),
            jobs: Vec::new(),
            conditional_overwrites: Vec::new(),
            read_sites: BTreeSet::new(),
        };
        let mut raw = tokens.clone();
        raw.source_binding = None;
        builder.selection(&raw, site)
    };
    assert!(
        selection(inventory).is_some(),
        "genuine original reached owner"
    );
    let absent = DeclarationFlowInventory::new(
        Arc::clone(&inventory.layouts),
        Arc::clone(&inventory.accesses),
        Arc::clone(&inventory.origin_accesses),
        Arc::new(Vec::new()),
        Arc::new(BTreeMap::new()),
    );
    assert!(
        selection(&absent).is_some(),
        "sealed raw declaration without dispatch"
    );
    let mut invalid = inventory.points.as_ref().clone();
    assert!(
        invalid
            .iter()
            .any(|point| point.dispatch && point.offset == site.offset)
    );
    for point in &mut invalid {
        point.original_written_words = None;
        point.compiled_execution.original_words = None;
    }
    let invalid = DeclarationFlowInventory::new(
        Arc::clone(&inventory.layouts),
        Arc::clone(&inventory.accesses),
        Arc::clone(&inventory.origin_accesses),
        Arc::new(invalid),
        Arc::new(BTreeMap::new()),
    );
    assert!(
        selection(&invalid).is_none(),
        "invalid reached owner remains terminal"
    );
}

#[test]
fn source_read_scope_preserves_reached_owner_refusal_after_loop() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    let source = "proc p {n} {set anchor 1; while {$n > 0} {set x 1; incr n -1}; puts $x}";
    let bindings = super::super::SourceCommandBindings::analyse(source, config, registry);
    let offset = u32::try_from(source.find("puts $x").unwrap()).unwrap();
    let segment =
        crate::segmenter::segment_commands_with_offset_and_config("puts $x", offset, config)
            .remove(0);
    let mut tokens = CommandTokens::from_segmented(
        &tcl_lexer::SourceImage::document(source).source_map(),
        config,
        &segment,
    );
    bindings.stamp_original_tokens(&mut tokens);
    let binding = tokens.source_binding.as_ref().unwrap();
    let site = binding.invocation_site().unwrap();
    let inventory = binding.declaration_flow_inventory.as_ref().unwrap();
    let observation = super::super::declaration_layout::original_declaration_layouts(
        inventory.layouts.get(site).unwrap(),
    )
    .unwrap()
    .next()
    .unwrap();
    let dialect = observation
        .snapshot
        .state
        .source_variables
        .invocation_dialect
        .unwrap();
    let frame = DeclarationFlowFrame::Existing(observation.entry.as_ref().clone());
    let selected = |inventory| {
        let builder = Builder {
            inventory,
            registry,
            config,
            dialect,
            entry: &frame,
            purpose: DeclarationFlowPurpose::Reads,
            representation_realm: None,
            representation_loops: Vec::new(),
            current_representation_body: None,
            nodes: Vec::new(),
            jobs: Vec::new(),
            conditional_overwrites: Vec::new(),
            read_sites: BTreeSet::new(),
        };
        let mut raw = tokens.clone();
        raw.source_binding = None;
        builder.source_read_selection(&raw, site)
    };
    assert!(selected(inventory).is_some());
    let mut points = inventory.points.as_ref().clone();
    assert!(
        points
            .iter()
            .any(|point| point.dispatch && point.offset == site.offset)
    );
    for point in &mut points {
        point.original_written_words = None;
        point.compiled_execution.original_words = None;
    }
    let mut invalid = DeclarationFlowInventory::new(
        Arc::clone(&inventory.layouts),
        Arc::clone(&inventory.accesses),
        Arc::clone(&inventory.origin_accesses),
        Arc::new(points),
        Arc::new(BTreeMap::new()),
    );
    invalid.source_seed = inventory.source_seed.clone();
    assert!(
        selected(&invalid).is_none(),
        "malformed reached metadata stays terminal"
    );
}

fn report(source: &str) -> Arc<DeclarationFlowReport> {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
    let bindings = super::super::SourceCommandBindings::analyse(source, config, registry);
    let original = "set anchor 1";
    let offset = u32::try_from(source.find(original).unwrap()).unwrap();
    let segment =
        crate::segmenter::segment_commands_with_offset_and_config(original, offset, config)
            .remove(0);
    let mut tokens = CommandTokens::from_segmented(
        &tcl_lexer::SourceImage::document(source).source_map(),
        config,
        &segment,
    );
    bindings.stamp_original_tokens(&mut tokens);
    tokens
        .source_binding
        .as_ref()
        .unwrap()
        .declaration_flow_report(registry)
        .expect("accepted original declaration owns a diagnostic graph")
}

#[test]
fn original_declaration_flow_cache_retains_full_inventory_and_query_identity() {
    // Implementation contract: naming.source.declaration-flow-report-cache (docs/design/analysis/name-resolution-proofs/declaration-flow-report-cache.md).
    let owner = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = owner.commands();
    let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
    let source = "set anchor 1; list $anchor";
    let mut bindings = super::super::SourceCommandBindings::analyse(source, config, registry);
    let call = bindings.invocation_at_source("set", 0);
    let first = call.declaration_flow_report(registry).unwrap();
    let again = bindings
        .invocation_at_source("set", 0)
        .declaration_flow_report(registry)
        .unwrap();
    assert!(Arc::ptr_eq(&first, &again));
    let inventory = call.declaration_flow_inventory.as_ref().unwrap();
    let key = inventory.reports.0.lock().unwrap()[0].0.clone();
    let mut changed = key.clone();
    changed.config.strict_quoting = !changed.config.strict_quoting;
    assert!(inventory.reports.get(&changed).is_none());
    changed = key.clone();
    changed.registry = tcl_registry::model::ingress::static_context_for("tcl8.5")
        .commands()
        .snapshot()
        .semantic_key();
    assert!(inventory.reports.get(&changed).is_none());
    changed = key.clone();
    let DeclarationFlowFrame::Existing(original_entry) = key.entry.as_ref() else {
        panic!("retained original diagnostic frame");
    };
    let super::super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript {
        source,
        frame,
        namespace,
    } = original_entry
    else {
        panic!("original root entry");
    };
    let mut foreign_source = source.clone();
    foreign_source.text = tcl_lexer::SourceImage::native(source.text.bytes());
    foreign_source.origin = Arc::new(super::super::SourceOriginId::authored_image(
        foreign_source.text.clone(),
    ));
    changed.entry = Arc::new(DeclarationFlowFrame::Existing(
        super::super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript {
            source: foreign_source,
            frame: frame.clone(),
            namespace: namespace.clone(),
        },
    ));
    assert!(inventory.reports.get(&changed).is_none());
    // A fresh, unchanged clone shares the reports of the equal inventory.
    let clone = bindings.clone();
    let shared = clone.current_declaration_flow_inventory();
    assert!(Arc::ptr_eq(&inventory.reports.0, &shared.reports.0));
    assert!(!shared.reports.0.lock().unwrap().is_empty());
    let point = bindings.points[0].clone();
    bindings.points.push(point);
    let next = bindings.current_declaration_flow_inventory();
    assert!(!Arc::ptr_eq(inventory, &next));
    assert!(next.reports.0.lock().unwrap().is_empty());
    assert!(Arc::ptr_eq(
        &first,
        &call.declaration_flow_report(registry).unwrap()
    ));
}

#[test]
fn original_diagnostic_frame_keeps_root_values_and_expanded_loop_bindings() {
    let source = "set spec {{1}}; set names {x}; foreach $names {*}$spec {puts $x}";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
    let bindings = super::super::SourceCommandBindings::analyse(source, config, registry);
    let segment =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
    let mut tokens = CommandTokens::from_segmented(
        &tcl_lexer::SourceImage::document(source).source_map(),
        config,
        &segment,
    );
    bindings.stamp_original_tokens(&mut tokens);
    let result = tokens
        .source_binding
        .as_ref()
        .unwrap()
        .declaration_flow_report(registry)
        .expect("original global script owns a diagnostic frame");
    assert!(!result.potential_declared_reads().contains_key("x"));
    assert!(result.has_authored_use("x"));
}

#[test]
fn original_declaration_graph_preserves_nested_missing_local_reads() {
    for (source, name) in [
        (
            "proc f {n} {set anchor 1; switch -- $n a {for {set j 0} {$j < 3} {incr j} {puts $missing}}}",
            "missing",
        ),
        (
            "proc f {dict} {set anchor 1; ::foreach {k v} $dict {puts \"$k=$missing\"}}",
            "missing",
        ),
        (
            "proc f {items} {set anchor 1; foreach x $items {puts $y; set y $x}}",
            "y",
        ),
    ] {
        let result = report(source);
        let span = result
            .potential_declared_reads()
            .get(name)
            .copied()
            .unwrap_or_else(|| panic!("original occurrence missing: {source}"));
        assert!(
            result.allows_declared_absence_warning(name, span),
            "original prefix unavailable: {source}"
        );
    }
}

#[test]
fn conditional_missing_read_warning_keeps_first_entry_and_rejects_unknown_prefixes() {
    let first = report("proc p {flag} {set anchor 1; while {$flag} {puts $missing; operation}}");
    let span = first.declared_absence_warning_occurrences()["missing"];
    assert!(first.allows_declared_absence_warning("missing", span));
    assert!(!first.local_store_may_be_unread(
        &CommandAllocationSite {
            source: Arc::clone(&first.source),
            offset: span.start(),
        },
        "missing"
    ));
    for source in [
        "proc p {} {set anchor 1; operation; puts $missing}",
        "proc p {name} {set anchor 1; set $name 1; puts $missing}",
    ] {
        let blocked = report(source);
        assert!(blocked.potential_declared_reads().contains_key("missing"));
        assert!(
            !blocked
                .declared_absence_warning_occurrences()
                .contains_key("missing"),
            "{source}"
        );
    }
}

#[test]
fn declaration_substitution_keeps_exact_original_component_ranges() {
    let config = tcl_lexer::LexerConfig::default();
    for command in [
        "list [set x] ",
        "list \"[set x]\" ",
        "list \"before[set x]after\" ",
        "list pre[set x] ",
        "list [] ",
        "list [list [set x]] ",
    ] {
        let source = format!("# original prefix\n{command}");
        let base = u32::try_from(source.len() - command.len()).unwrap();
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config(command, base, config)
                .remove(0);
        let tokens =
            CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(&source), config, &segment);
        let parts = match tokens.words().last().unwrap() {
            WordExpr::CommandSubstitution { spelling, source } => {
                vec![(spelling, source.span)]
            }
            WordExpr::Template { parts, .. } => parts
                .iter()
                .filter_map(|part| match part {
                    WordPart::CommandSubstitution { spelling, source } => {
                        Some((spelling, source.span))
                    }
                    _ => None,
                })
                .collect(),
            _ => panic!("original command component missing: {command}"),
        };
        assert_eq!(parts.len(), 1, "{command}");
        let (spelling, span) = parts[0];
        let site = CommandAllocationSite {
            source: Arc::new(super::super::SourceOriginId::authored(&Arc::from(
                source.as_str(),
            ))),
            offset: base,
        };
        let mut events = Vec::new();
        Builder::substitution(spelling, span, &site, &mut events);
        let [Event::Script(script)] = events.as_slice() else {
            panic!("exact original component must retain its script: {command}");
        };
        assert_eq!(script.origin, site.source);
        assert_eq!(script.base(), span.start() + 1);
        assert_eq!(
            script.text.bytes(),
            &spelling.as_bytes()[1..spelling.len() - 1]
        );
        for changed in [
            tcl_lexer::Span::new(span.start(), span.end() + 1),
            tcl_lexer::Span::new(span.start(), span.end() - 1),
            tcl_lexer::Span::new(span.start() + 1, span.end()),
        ] {
            let mut events = Vec::new();
            Builder::substitution(spelling, changed, &site, &mut events);
            assert!(matches!(events.as_slice(), [Event::Unknown]), "{command}");
        }
    }
}

#[test]
fn original_declaration_overwrite_advice_is_conditional_and_ordered() {
    let source =
        "proc p {target} {set anchor 1; set b [set {$target}]; set a 1; set a 2; return \"$a$b\"}";
    let declared = report(source);
    let occurrences = declared
        .conditional_overwrite_occurrences()
        .collect::<Vec<_>>();
    let (first, next, name, target) = occurrences
        .iter()
        .find(|(_, _, name, _)| *name == "a")
        .expect("the declared adjacent setters retain only conditional ordering");
    assert!(first.offset < next.offset);
    assert_eq!(
        &source[target.start() as usize..target.end() as usize],
        *name
    );
    let advice = crate::registry_invocation::conditional_declared_overwrite_advice(&declared)
        .find(|advice| advice.name() == "a")
        .unwrap();
    assert!(advice.owns_source(&tcl_lexer::SourceImage::document(source)));
    assert!(
        !advice.owns_source(&tcl_lexer::SourceImage::document(&format!(
            "# relocated\n{source}"
        ),))
    );
    for source in [
        "proc p {target} {set anchor 1; set b [set $target]; set a 1; set a 2; return \"$a$b\"}",
        "proc p {} {set anchor 1; operation; set a 1; set a 2; return $a}",
        "proc p {} {set anchor 1; set a 1; puts $a; set a 2; return $a}",
        "proc p {} {set anchor 1; upvar 1 remote a; set a 1; set a 2; return $a}",
        "proc p {} {set anchor 1; set ::a 1; set ::a 2; return $::a}",
    ] {
        assert!(
            report(source)
                .conditional_overwrite_occurrences()
                .all(|(_, _, name, _)| name != "a" && name != "::a"),
            "{source}"
        );
    }
}

#[test]
fn original_procedure_store_graph_survives_abrupt_reads_without_unknown_writer_grants() {
    let owner = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = owner.commands();
    let profile = registry.profile().unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    for (source, expected) in [
        (
            "proc f {p} {set b [set {$p}]; set a 1; set a 2; return \"$a$b\"}",
            true,
        ),
        (
            "proc f {p} {set b [set $p]; set a 1; set a 2; return \"$a$b\"}",
            false,
        ),
        (
            "proc f {p} {unknown_host; set b [set {$p}]; set a 1; set a 2; return \"$a$b\"}",
            false,
        ),
        (
            "proc f {p} {set b [set {$p}]; rename set saved; set a 1; set a 2; return $a}",
            false,
        ),
        (
            "proc f {p} {set b [set {$p}]; operation; set a 1; set a 2; return $a}",
            false,
        ),
        (
            "proc f {p} {set b [set {$p}]; set other [operation]; set a 1; set a 2; return $a}",
            false,
        ),
    ] {
        let bindings = super::super::SourceCommandBindings::analyse(source, config, registry);
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            source, registry, false, profile,
        );
        let procedure = fixture_id_if_present(&unit.ir_module.procedures, "::f")
            .and_then(|id| unit.ir_module.procedures.get(&id))
            .unwrap();
        let image = tcl_lexer::SourceImage::document(source);
        let report = bindings
            .original_procedure_declaration_flow(procedure, &image, registry)
            .expect("the exact original declaration owns conditional diagnostics");
        assert_eq!(
            report
                .conditional_overwrite_occurrences()
                .any(|(_, _, name, _)| name == "a"),
            expected,
            "{source}"
        );
        assert!(
            bindings
                .original_procedure_declaration_flow(
                    procedure,
                    &tcl_lexer::SourceImage::document(&format!("{source}\n")),
                    registry,
                )
                .is_none()
        );
        let mut changed = procedure.clone();
        changed.params.push("different".to_owned());
        assert!(
            bindings
                .original_procedure_declaration_flow(&changed, &image, registry)
                .is_none()
        );
    }
}

#[test]
fn unentered_declaration_suffix_does_not_grant_original_dispatch() {
    let source = "proc f {p} {set b [set {$p}]; set a 1; set a 2; return $a}";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let dialect = tcl_registry::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::of_dialect_name(Some("tcl8.6")).unwrap(),
    );
    let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
    let bindings = super::super::SourceCommandBindings::analyse_with_options(
        source,
        config,
        registry,
        super::super::SourceAnalysisOptions {
            invocation_dialect: Some(dialect),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..super::super::SourceAnalysisOptions::default()
        },
    );
    let original = "set a 1";
    let offset = u32::try_from(source.find(original).unwrap()).unwrap();
    let segment =
        crate::segmenter::segment_commands_with_offset_and_config(original, offset, config)
            .remove(0);
    let mut tokens = CommandTokens::from_segmented(
        &tcl_lexer::SourceImage::document(source).source_map(),
        config,
        &segment,
    );
    bindings.stamp_original_tokens(&mut tokens);
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(
        binding.declaration_operand_layout_advice(&tokens).is_some(),
        "site={:?}, layout={:?}",
        binding.invocation_site(),
        binding
            .declaration_layout_observations
            .as_ref()
            .map(|observations| observations
                .iter()
                .map(|observation| (
                    &observation.namespace,
                    &observation.snapshot.state.variable_frame,
                    observation
                        .snapshot
                        .state
                        .targets("set", &observation.namespace)
                        .iter()
                        .map(|target| (target.command.clone(), target.registry_backed))
                        .collect::<Vec<_>>()
                ))
                .collect::<Vec<_>>())
    );
    assert!(!binding.unobserved_native_dispatch());
    assert!(!binding.original_arguments_complete_normally(&tokens));
}

#[test]
fn original_declaration_graph_keeps_zero_trip_and_no_match_successors() {
    for (source, missing) in [
        (
            "proc p {mode} {set anchor 1; switch -- $mode {a {set x 1}}; puts $x}",
            true,
        ),
        (
            "proc p {mode} {set anchor 1; switch -- $mode {a {set x 1} default {set x 2}}; puts $x}",
            false,
        ),
        (
            "proc p {mode} {set anchor 1; switch -- $mode {a {return} default {error STOP}}; puts $x}",
            false,
        ),
        (
            "proc p {rows} {set anchor 1; foreach i $rows {set x 1}; puts $x}",
            true,
        ),
        (
            "proc p {} {set anchor 1; while {0} {set x 1}; puts $x}",
            true,
        ),
        (
            "proc p {} {set anchor 1; if {0} {puts $x}; set x 1; puts $x}",
            false,
        ),
    ] {
        let result = report(source);
        assert_eq!(
            result.potential_missing_reads().contains_key("x"),
            missing,
            "{source}"
        );
    }
}

#[test]
fn conditional_reachability_preserves_terminal_and_unavailable_paths() {
    for (source, expected) in [
        ("proc p {} {set anchor 1; return; puts AFTER}", Some(false)),
        (
            "proc p {} {set anchor 1; set x [operation]; puts AFTER}",
            None,
        ),
        ("proc p {} {set anchor 1; set x 1; puts AFTER}", Some(true)),
    ] {
        let result = report(source);
        let site = CommandAllocationSite {
            source: Arc::clone(&result.source),
            offset: u32::try_from(source.find("puts AFTER").unwrap()).unwrap(),
        };
        assert_eq!(
            result.invocation_may_be_reached(&site),
            expected,
            "{source}"
        );
        let other = CommandAllocationSite {
            source: Arc::new(super::super::SourceOriginId::authored(&"puts AFTER".into())),
            offset: 0,
        };
        assert_eq!(result.invocation_may_be_reached(&other), None);
        let unrepresented = CommandAllocationSite {
            source: Arc::clone(&result.source),
            offset: 1,
        };
        assert_eq!(result.invocation_may_be_reached(&unrepresented), None);
    }
}

#[test]
fn array_index_effects_do_not_become_scalar_read_permission() {
    let source = "proc p {} {set anchor 1; puts $a([operation]); puts $x}";
    let result = report(source);
    assert!(!result.potential_missing_reads().contains_key("x"));
    assert_eq!(
        result.invocation_may_be_reached(&CommandAllocationSite {
            source: Arc::clone(&result.source),
            offset: u32::try_from(source.rfind("puts").unwrap()).unwrap(),
        }),
        None
    );
}

#[test]
fn declared_loop_first_entry_does_not_supply_runtime_zero_trip_proof() {
    for (source, missing) in [
        (
            "proc p {n} {set anchor 1; while {$n > 0} {set x 1; incr n -1}; puts $x}",
            false,
        ),
        (
            "proc p {} {set anchor 1; foreach i {a b} {set x $i}; puts $x}",
            false,
        ),
        (
            "proc p {} {set anchor 1; foreach i {} {set x $i}; puts $x}",
            true,
        ),
        (
            "proc p {rows} {set anchor 1; foreach i $rows {set x $i}; puts $x}",
            true,
        ),
        (
            "proc p {} {set anchor 1; for {set i 0} {$i < 3} {incr i} {set x $i}; puts $x}",
            false,
        ),
        (
            "proc p {} {set anchor 1; for {set i 5} {$i < 3} {incr i} {set x $i}; puts $x}",
            true,
        ),
        (
            "proc p {} {set anchor 1; for {set i 0; incr i 5} {$i < 3} {incr i} {set x $i}; puts $x}",
            true,
        ),
        (
            "proc p {n} {set anchor 1; for {set i 0; set i $n} {$i < 3} {incr i} {set x $i}; puts $x}",
            false,
        ),
        (
            "proc p {} {set anchor 1; foreach i {1} {continue; set x 1}; puts $x}",
            true,
        ),
        (
            "proc p {rows} {set anchor 1; foreach i $rows {puts $x; set x $i}}",
            true,
        ),
        (
            "proc p {rows cols} {set anchor 1; foreach r $rows {foreach c $cols {lappend x [list $r $c]}}; puts $x}",
            false,
        ),
        (
            "proc p {rows cols} {set anchor 1; foreach r $rows {foreach c $cols {if {$c} {lappend x $c}}}; puts $x}",
            true,
        ),
        (
            "proc p {rows} {set anchor 1; foreach r $rows {puts $x; lappend x $r}}",
            true,
        ),
    ] {
        let result = report(source);
        assert_eq!(
            result.potential_declared_reads().contains_key("x"),
            missing,
            "{source}"
        );
    }
    let source = "proc p {n} {set anchor 1; while {$n > 0} {set x 1; incr n -1}; puts $x}";
    let result = report(source);
    let site = CommandAllocationSite {
        source: Arc::clone(&result.source),
        offset: u32::try_from(source.rfind("puts").unwrap()).unwrap(),
    };
    assert!(result.declared_argument_entry(&site));
    assert_eq!(result.invocation_may_be_reached(&site), None);
    assert!(!result.local_store_may_be_unread(&site, "x"));
}

#[test]
fn indexed_conditions_preserve_declared_zero_trip_reads() {
    let source = "proc p {key} {set anchor 1; while {$a($key)} {set x 1}; puts $x}";
    let result = report(source);
    assert!(result.has_authored_use("a"));
    assert!(result.has_authored_use("key"));
    assert!(result.potential_declared_reads().contains_key("a"));
    assert!(result.potential_declared_reads().contains_key("x"));
    let after_loop = CommandAllocationSite {
        source: Arc::clone(&result.source),
        offset: u32::try_from(source.rfind("puts").unwrap()).unwrap(),
    };
    assert_eq!(result.invocation_may_be_reached(&after_loop), None);
    assert!(!result.local_store_may_be_unread(&after_loop, "x"));
}

#[test]
fn c86_indexed_conditions_preserve_original_terminal_scripts() {
    // C8.6 executes the index before reading the array and propagates its
    // return. Jim's index completion contract differs from this fixture.
    let source = "proc p {} {set anchor 1; while {$a([return DONE])} {set x 1}; puts $x}";
    let result = report(source);
    assert!(!result.has_authored_use("x"));
    assert!(!result.potential_declared_reads().contains_key("x"));
    let after_loop = CommandAllocationSite {
        source: Arc::clone(&result.source),
        offset: u32::try_from(source.rfind("puts").unwrap()).unwrap(),
    };
    assert_eq!(result.invocation_may_be_reached(&after_loop), Some(false));
    assert!(!result.declared_argument_entry(&after_loop));
}

#[test]
fn authored_read_occurrences_preserve_unknown_runtime_and_array_effects() {
    let source = "proc p {} {set anchor 1; puts $a([operation]); puts $x}";
    let result = report(source);
    assert!(result.potential_declared_reads().contains_key("a"));
    assert!(result.potential_declared_reads().contains_key("x"));
    assert!(!result.allows_declared_absence_warning("x", result.potential_declared_reads()["x"]));
    assert!(!result.potential_missing_reads().contains_key("x"));
    let site = CommandAllocationSite {
        source: Arc::clone(&result.source),
        offset: u32::try_from(source.rfind("puts").unwrap()).unwrap(),
    };
    assert!(result.owns_invocation(&site));
    assert_eq!(result.invocation_may_be_reached(&site), None);
    assert!(!result.local_store_may_be_unread(&site, "x"));
    assert!(!result.owns_invocation(&CommandAllocationSite {
        source: Arc::new(super::super::SourceOriginId::authored(&source.into())),
        offset: 1,
    }));
}

#[test]
fn conditional_output_candidates_do_not_become_physical_stores() {
    for pattern in ["[a-z]", "^"] {
        let source = format!("proc p {{}} {{set anchor 1; regexp {{{pattern}}} x v; puts $v}}");
        let result = report(&source);
        assert!(!result.potential_declared_reads().contains_key("v"));
        assert!(!result.local_store_may_be_unread(
            &CommandAllocationSite {
                source: Arc::clone(&result.source),
                offset: u32::try_from(source.find("regexp").unwrap()).unwrap(),
            },
            "v"
        ));
    }
    let result = report("proc p {} {set anchor 1; regexp x y v; puts $v}");
    assert!(result.potential_declared_reads().contains_key("v"));
    let result = report("proc p {} {set anchor 1; operation; puts $other}");
    let span = result.potential_declared_reads()["other"];
    assert!(result.has_authored_use("other"));
    assert!(!result.allows_declared_absence_warning("other", span));
    let result = report("proc p {} {set anchor 1; error stop; puts $other}");
    assert!(!result.has_authored_use("other"));
}

#[test]
fn quoted_data_mentions_do_not_supply_executed_reads() {
    for body in [
        "puts {$literal}",
        "foreach item {$literal} {puts $item}",
        "switch -- mode {$literal {puts ARM} default {puts DEFAULT}}",
    ] {
        let source = format!("proc p {{}} {{set anchor 1; {body}}}");
        let result = report(&source);
        assert!(result.has_quoted_use("literal"), "{body}");
        assert!(!result.has_authored_use("literal"), "{body}");
        assert!(
            !result.potential_missing_reads().contains_key("literal"),
            "{body}"
        );
        assert!(
            !result.potential_declared_reads().contains_key("literal"),
            "{body}"
        );
    }
    let result = report("proc p {} {set anchor 1; puts {plain}; return; puts {$dead}}");
    assert!(!result.has_quoted_use("plain"));
    assert!(!result.has_quoted_use("dead"));
    let result = report("proc p {} {set anchor 1; puts $live}");
    assert!(!result.has_quoted_use("live"));
    assert!(result.has_authored_use("live"));
}

#[test]
fn declared_aliases_and_terminal_paths_keep_distinct_read_occurrences() {
    let result = report(
        "proc p {} {set anchor 1; variable ::ns::children; puts $children; puts $other; return; puts $dead}",
    );
    assert!(result.has_declared_alias("children"));
    assert!(!result.potential_declared_reads().contains_key("children"));
    assert!(result.potential_declared_reads().contains_key("other"));
    assert!(!result.has_authored_use("dead"));
    let result = report("proc p {} {set anchor 1; set {$n} 1; set {$n} 2; return [set {$n}]}");
    assert!(result.has_named_use("$n"));
    assert!(!result.has_authored_use("n"));
}

#[test]
fn original_call_entry_precedes_its_unknown_dispatch() {
    let source = "proc p {} {set anchor 1; run notacommand; puts AFTER}";
    let result = report(source);
    for (command, expected) in [("run notacommand", Some(true)), ("puts AFTER", None)] {
        assert_eq!(
            result.invocation_may_be_reached(&CommandAllocationSite {
                source: Arc::clone(&result.source),
                offset: u32::try_from(source.find(command).unwrap()).unwrap(),
            }),
            expected,
            "{command}"
        );
    }
    let source = "proc p {} {set anchor 1; operation; run notacommand}";
    let result = report(source);
    assert_eq!(
        result.invocation_may_be_reached(&CommandAllocationSite {
            source: Arc::clone(&result.source),
            offset: u32::try_from(source.find("run notacommand").unwrap()).unwrap(),
        }),
        None
    );
}
