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

//! Conditional representation plans: naming, substitutions, loop receipts and body contexts.

use super::*;

#[test]
fn conditional_root_naming_is_scoped_to_the_original_command() {
    let source = "set s abc; set n [string length $s]; set m $n; lindex $m 0; lindex $n 0";
    // An embedding host's object types may carry hooks, so the first
    // `lindex` ends what the root may name.
    let analysis = crate::command_binding::SourceAnalysisEntry::analyse_in_extensible_host_for_test(
        source, "tcl8.6",
    );
    let input = analysis.resolved_input.as_ref().unwrap();
    let registry = input.borrowed_context_registry().commands();
    let config = input.lexer_config();
    let realm = analysis.retained_command_realm().unwrap();
    let image = tcl_lexer::SourceImage::document(source);
    let segments = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
    let mut anchor = CommandTokens::from_segmented(&image.source_map(), config, &segments[0]);
    realm
        .source_bindings_ref()
        .stamp_original_tokens(&mut anchor);
    let binding = anchor.source_binding.as_ref().unwrap();
    let inventory = binding.declaration_flow_inventory.as_ref().unwrap();
    let root_site = binding.invocation_site().unwrap();
    let observation = super::super::super::declaration_layout::original_declaration_layouts(
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
        tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config).remove(0);
    let mut tokens = CommandTokens::from_segmented(&image.source_map(), config, &segment);
    bindings.stamp_original_tokens(&mut tokens);
    let binding = tokens.source_binding.as_ref().unwrap();
    let inventory = binding.declaration_flow_inventory.as_ref().unwrap();
    let site = binding.invocation_site().unwrap().clone();
    let observation = super::super::super::declaration_layout::original_declaration_layouts(
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
    let advice = super::super::super::original_site_operand_layout_advice(
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
    if let tcl_source::words::WordExpr::Literal { text, .. } = &mut changed.effective.words[0] {
        *text = "foreign".to_owned();
    } else {
        panic!("expected source head");
    }
    assert!(!check(&changed, &observed));
    let mut changed = original.clone();
    if let tcl_source::words::WordExpr::Literal { source, .. } = &mut changed.effective.words[0] {
        *source = tcl_source::words::SourceSite::opaque(tcl_lexer::Span::new(1, 2));
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
        let segment = tcl_source::segment::segment_commands_with_offset_and_config(
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
        let segments =
            tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
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
    let segments = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
    let mut anchor = CommandTokens::from_segmented(&image.source_map(), config, &segments[0]);
    realm
        .source_bindings_ref()
        .stamp_original_tokens(&mut anchor);
    let binding = anchor.source_binding.as_ref().unwrap();
    let inventory = binding.declaration_flow_inventory.as_ref().unwrap();
    let root_site = binding.invocation_site().unwrap();
    let observation = super::super::super::declaration_layout::original_declaration_layouts(
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
        tcl_source::segment::segment_commands_with_offset_and_config("incr x", offset, config)
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
            tcl_source::segment::segment_commands_with_offset_and_config("incr x", offset, config)
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
        row.snapshot = Arc::new(super::super::super::SourceLookupSnapshot::in_realm(
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
    let segments = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
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
