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

//! Evaluated reports: retained inventory, missing reads, overwrites and zero-trip paths.

use super::*;

#[test]
fn original_declaration_flow_cache_retains_full_inventory_and_query_identity() {
    // Implementation contract: naming.source.declaration-flow-report-cache (docs/design/analysis/name-resolution-proofs/declaration-flow-report-cache.md).
    let owner = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = owner.commands();
    let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
    let source = "set anchor 1; list $anchor";
    let mut bindings =
        super::super::super::SourceCommandBindings::analyse(source, config, registry);
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
    let super::super::super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript {
        source,
        frame,
        namespace,
    } = original_entry
    else {
        panic!("original root entry");
    };
    let mut foreign_source = source.clone();
    foreign_source.text = tcl_lexer::SourceImage::native(source.text.bytes());
    foreign_source.origin = Arc::new(super::super::super::SourceOriginId::authored_image(
        foreign_source.text.clone(),
    ));
    changed.entry = Arc::new(DeclarationFlowFrame::Existing(
        super::super::super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript {
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
    let bindings = super::super::super::SourceCommandBindings::analyse(source, config, registry);
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
            source: Arc::new(super::super::super::SourceOriginId::authored(&Arc::from(
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
        let bindings =
            super::super::super::SourceCommandBindings::analyse(source, config, registry);
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
    let bindings = super::super::super::SourceCommandBindings::analyse_with_options(
        source,
        config,
        registry,
        super::super::super::SourceAnalysisOptions {
            invocation_dialect: Some(dialect),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..super::super::super::SourceAnalysisOptions::default()
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
            source: Arc::new(super::super::super::SourceOriginId::authored(
                &"puts AFTER".into(),
            )),
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
    // An embedding host's object types may carry hooks, so the condition's
    // conversion leaves the successor's entry unavailable.
    let source = "proc p {n} {set anchor 1; while {$n > 0} {set x 1; incr n -1}; puts $x}";
    let result = report_in_extensible_host(source);
    let site = CommandAllocationSite {
        source: Arc::clone(&result.source),
        offset: u32::try_from(source.rfind("puts").unwrap()).unwrap(),
    };
    assert!(result.declared_argument_entry(&site));
    assert_eq!(result.invocation_may_be_reached(&site), None);
    assert!(!result.local_store_may_be_unread(&site, "x"));
    // A fresh interpreter's stock types run no script on conversion.
    let fresh = report(source);
    let site = CommandAllocationSite {
        source: Arc::clone(&fresh.source),
        offset: site.offset,
    };
    assert_eq!(fresh.invocation_may_be_reached(&site), Some(true));
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
        source: Arc::new(super::super::super::SourceOriginId::authored(
            &source.into()
        )),
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
