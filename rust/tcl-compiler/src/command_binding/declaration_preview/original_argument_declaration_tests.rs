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

fn logical(source: &str) -> SourceCommandBindings {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
            .unwrap()
            .default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    SourceCommandBindings::analyse_with_options(
        source,
        input.lexer_config(),
        input.borrowed_context_registry().commands(),
        super::super::SourceAnalysisOptions::for_logical_source(&input).unwrap(),
    )
}

#[test]
fn logical_called_procedure_requires_its_current_source_allocation() {
    let source = "namespace eval n {proc format {args} {return NS}; format %b 5}\nformat %d 5\n";
    let bindings = logical(source);
    let call = u32::try_from(source.find("format %b").unwrap()).unwrap();
    let point = bindings
        .points
        .iter()
        .find(|point| point.offset == call && point.dispatch)
        .unwrap();
    let binding = point.lookup_binding("format");
    let target = binding.proved_target().unwrap().clone();
    let registry = point
        .state
        .logical_source_name_advice_input()
        .unwrap()
        .borrowed_context_registry()
        .commands();
    assert!(
        bindings
            .logical_called_procedure_body(&target, &point.state, registry)
            .is_some()
    );
    let mut changed = target.clone();
    changed
        .implementation_allocation
        .as_mut()
        .unwrap()
        .incarnation = super::super::AllocationIncarnation::Second;
    assert!(
        bindings
            .logical_called_procedure_body(&changed, &point.state, registry)
            .is_none()
    );
    let mut changed = target.clone();
    changed
        .implementation_allocation
        .as_mut()
        .unwrap()
        .namespace = super::super::SourceNamespaceKey::authored("::other");
    assert!(
        bindings
            .logical_called_procedure_body(&changed, &point.state, registry)
            .is_none()
    );
    let mut changed = target.clone();
    changed.identity.as_mut().unwrap().allocation = None;
    assert!(
        bindings
            .logical_called_procedure_body(&changed, &point.state, registry)
            .is_none()
    );
    let mut changed = target.clone();
    changed.implementation_allocation = None;
    assert!(
        bindings
            .logical_called_procedure_body(&changed, &point.state, registry)
            .is_none()
    );
    let foreign = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    assert!(
        bindings
            .logical_called_procedure_body(&target, &point.state, foreign)
            .is_none()
    );
}

#[test]
fn logical_evaluated_declaration_body_retains_derived_source_without_native_receipts() {
    let source = "set name {LiteralChild}; set body {return original}; proc $name {} $body";
    let bindings = logical(source);
    let (site, rows) = bindings
        .declaration_layouts
        .iter()
        .find(|(site, _)| site.offset as usize == source.find("proc $name").unwrap())
        .expect("actual original declaration point");
    let row = original_declaration_layouts(rows).unwrap().next().unwrap();
    let input = row
        .snapshot
        .state
        .logical_source_name_advice_input()
        .unwrap();
    let registry = input.borrowed_context_registry().commands();
    let tokens = declaration_tokens(site, row).unwrap();
    let recipe = bindings
        .original_logical_procedure_declaration(&tokens, registry, input)
        .expect("quiet original values retain the conditional body recipe");
    assert_eq!(recipe.qualified_name(), Some("::LiteralChild"));
    assert_eq!(recipe.source().try_text().unwrap(), "return original");
    assert_eq!(
        recipe.source().mapping,
        super::super::ExecutedScriptMapping::Materialised
    );
    assert_ne!(recipe.source().origin, site.source);
    assert_eq!(recipe.source().base(), 0);
    assert!(recipe.body.original_parameters.is_none());
    assert!(row.snapshot.state.baseline.native_entry.is_none());
    let original = crate::registry_invocation::original_native_compiler_words(
        site.source.source_image(),
        tokens.words(),
        site.offset,
        row.config,
    )
    .unwrap();
    assert!(super::super::logical_definition::static_value(&original[1]).is_none());
    assert!(super::super::logical_definition::static_value(&original[3]).is_none());
    let mut changed = tokens.clone();
    changed.word_exprs.swap(1, 3);
    assert!(
        bindings
            .original_logical_procedure_declaration(&changed, registry, input)
            .is_none()
    );
}

#[test]
fn logical_evaluated_declaration_body_refuses_unknown_array_and_effectful_values() {
    for source in [
        "set name {LiteralChild}; proc $name {} $missing",
        "set name {LiteralChild}; set body {return original}; proc $name {} $body(index)",
        "set name {LiteralChild}; set body {return original}; unknown_writer; proc $name {} $body",
        "set name {LiteralChild}; set body {return original}; proc $name {} [opaque]",
    ] {
        let bindings = logical(source);
        let offset = source.find("proc $name").unwrap();
        if let Some((site, rows)) = bindings
            .declaration_layouts
            .iter()
            .find(|(site, _)| site.offset as usize == offset)
            && let Some(row) = original_declaration_layouts(rows).and_then(|mut rows| rows.next())
        {
            let input = row
                .snapshot
                .state
                .logical_source_name_advice_input()
                .unwrap();
            let registry = input.borrowed_context_registry().commands();
            let tokens = declaration_tokens(site, row).unwrap();
            assert!(
                bindings
                    .original_logical_procedure_declaration(&tokens, registry, input)
                    .is_none(),
                "{source}"
            );
        }
    }
}

fn puts_parent(
    bindings: &SourceCommandBindings,
) -> (
    CommandAllocationSite,
    crate::ir::CommandTokens,
    Vec<DeclarationLayoutObservation>,
) {
    let (site, rows) = bindings
        .declaration_layouts
        .iter()
        .find(|(_, rows)| {
            rows.first().is_some_and(|row| {
                row.words
                    .first()
                    .is_some_and(|word| word.legacy_text() == "puts")
            })
        })
        .unwrap();
    let rows = original_declaration_layouts(rows)
        .unwrap()
        .cloned()
        .collect::<Vec<_>>();
    let tokens = declaration_tokens(site, &rows[0]).unwrap();
    (site.clone(), tokens, rows)
}

#[test]
fn original_argv_declaration_retains_real_child_world_without_dispatch_or_frame() {
    // naming.compiler.original-positioned-callee-body-availability
    // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
    let source = "proc subject {} {puts [list FIRST] [package require Tcl]}";
    let bindings = logical(source);
    let (site, mut tokens, _) = puts_parent(&bindings);
    bindings.stamp_original_tokens(&mut tokens);
    let config = bindings.lexer_config.unwrap();
    let calls = crate::word_subst::checked_argument_lifted_calls(&tokens, config).unwrap();
    assert_eq!(calls.len(), 2);
    let input = bindings
        .original_logical_source_name_advice_input()
        .unwrap();
    for call in calls {
        let child = call.tokens.unwrap();
        let binding = child.source_binding.as_ref().unwrap();
        assert!(binding.original_logical_declaration_targets.is_some());
        let targets = binding
            .original_logical_declaration_call_targets(
                &child,
                input,
                input.borrowed_context_registry().commands(),
            )
            .unwrap();
        assert_eq!(targets.len(), 1);
        assert!(targets[0].registry_backed);
        assert!(targets[0].implementation_allocation.is_none());
        assert!(binding.lookup_state.is_none());
        assert!(binding.unknown);
        assert_eq!(
            binding.targets,
            [] as [crate::command_binding::SourceCommandTarget; 0]
        );
        assert!(binding.proved_execution_target().is_none());
        assert!(binding.proved_handler_target().is_none());
        assert_ne!(
            binding.runtime_reachability(),
            super::super::SourceRuntimeReachability::Reached
        );
    }
    assert!(!bindings.has_actual_procedure_entry_at(&site.source, site.offset));
}

#[test]
fn original_argv_declaration_stops_after_unknown_callback_or_variable_prefix() {
    // naming.compiler.original-positioned-callee-body-availability
    // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
    for source in [
        "proc subject {value} {puts $value [package require Tcl]}",
        "proc subject {} {puts [unknown] [package require Tcl]}",
    ] {
        let bindings = logical(source);
        let (_, mut tokens, _) = puts_parent(&bindings);
        bindings.stamp_original_tokens(&mut tokens);
        let package = crate::word_subst::checked_argument_lifted_calls(
            &tokens,
            bindings.lexer_config.unwrap(),
        )
        .unwrap()
        .into_iter()
        .find(|call| call.command == "package")
        .unwrap();
        let child = package.tokens.unwrap();
        let binding = child.source_binding.as_ref().unwrap();
        assert!(
            binding.original_logical_declaration_targets.is_none(),
            "{source}"
        );
        assert!(binding.lookup_state.is_none());
        assert!(binding.proved_execution_target().is_none());
        assert!(binding.proved_handler_target().is_none());
    }
}

#[test]
fn original_argv_declaration_requires_original_parent_input_source_config_and_vector() {
    // naming.compiler.original-positioned-callee-body-availability
    // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
    let bindings = logical("proc subject {} {puts [package require Tcl]}");
    let (site, tokens, rows) = puts_parent(&bindings);
    let calls =
        crate::word_subst::checked_argument_lifted_calls(&tokens, bindings.lexer_config.unwrap())
            .unwrap();
    let child_offset = calls[0].tokens.as_ref().unwrap().words()[0]
        .source()
        .span
        .start();
    let child_site = CommandAllocationSite {
        source: Arc::clone(&site.source),
        offset: child_offset,
    };
    let registry = bindings
        .original_logical_source_name_advice_input()
        .unwrap()
        .context_registry();
    for axis in 0..5 {
        let mut changed = bindings.clone();
        changed.declaration_layouts.remove(&child_site);
        let mut parent = tokens.clone();
        let mut originals = rows.clone();
        match axis {
            0 => changed.lexer_config = None,
            1 => {
                let mut config = changed.lexer_config.unwrap();
                config.strict_quoting = !config.strict_quoting;
                changed.lexer_config = Some(config);
            }
            2 => {
                parent.word_exprs.pop();
            }
            3 => {
                changed.root_origin =
                    logical("proc foreign {} {puts [package require Tcl]}").root_origin;
            }
            4 => {
                let snapshot = Arc::make_mut(&mut originals[0].snapshot);
                Arc::make_mut(&mut snapshot.state.baseline).logical_source_input = None;
            }
            _ => unreachable!(),
        }
        changed.retain_original_argument_layouts(
            &site,
            &parent,
            &originals,
            registry.commands(),
            &mut Vec::new(),
            &super::nested_layouts::ScopeAliasOwnershipShare::default(),
        );
        assert!(
            !changed.declaration_layouts.contains_key(&child_site),
            "axis {axis}"
        );
    }
}

#[test]
fn conditional_header_receipt_keeps_withdrawn_layout_and_original_mutation_debt() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let source = "rename $old decl\nproc kept {arg} {list $arg}";
    let module = crate::lowering::lower_to_ir(source, registry);
    assert!(
        module
            .original_authored_declaration_body_units
            .values()
            .any(|card| card.name() == "kept")
    );
    let header = module
        .top_level
        .retained_source_tokens_for_statement(module.top_level.statements.last().unwrap())
        .expect("actual retained declaration vector");
    let binding = header.source_binding.as_ref().unwrap();
    let geometry = binding
        .original_authored_procedure_source_geometry(header, registry)
        .expect("transition-selected declaration geometry");
    assert!(geometry.has_unclosed_source_coverage());
    assert_ne!(geometry.declaration.advice.uncertain_operations(), []);
    assert!(
        geometry
            .snapshot
            .state
            .source_variables
            .execution_name_policy
            .is_none()
    );
    assert_eq!(
        geometry.snapshot.state.variable_frame,
        crate::var_resolve::VariableExecutionFrame::Unknown
    );
    let row =
        original_declaration_layouts(binding.declaration_layout_observations.as_deref().unwrap())
            .unwrap()
            .next()
            .unwrap();
    assert!(declared_body(binding.invocation_site().unwrap(), header, row, registry).is_none());
    assert_eq!(
        geometry
            .declaration
            .advice
            .original_words()
            .first()
            .unwrap()
            .image(),
        &module.source
    );
    assert_eq!(geometry.body_word().image(), &module.source);
}

#[test]
fn conditional_header_receipt_does_not_resurrect_known_shadow_or_deleted_descriptor() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for source in [
        "proc proc args {}; rename $old decl; proc kept {} {list retained}",
        "rename proc {}; rename $old decl; proc kept {} {list retained}",
    ] {
        let module = crate::lowering::lower_to_ir(source, registry);
        assert!(
            !module
                .original_authored_declaration_body_units
                .values()
                .any(|card| card.name() == "kept"),
            "{source}"
        );
    }
}

#[test]
fn conditional_header_geometry_cannot_borrow_a_foreign_present_point() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let source = "rename $old decl\nproc kept {} {list retained}";
    let module = crate::lowering::lower_to_ir(source, registry);
    let header = module
        .top_level
        .retained_source_tokens_for_statement(module.top_level.statements.last().unwrap())
        .unwrap();
    assert!(
        header
            .source_binding
            .as_ref()
            .unwrap()
            .original_authored_procedure_source_geometry(header, registry)
            .is_some()
    );
    let foreign = SourceCommandBindings::analyse(source, module.native_lexer_config(), registry);
    let offset = header
        .source_binding
        .as_ref()
        .unwrap()
        .invocation_site()
        .unwrap()
        .offset;
    let site = CommandAllocationSite {
        source: Arc::clone(foreign.source_origin().unwrap()),
        offset,
    };
    let row = original_declaration_layouts(foreign.declaration_layouts.get(&site).unwrap())
        .unwrap()
        .next()
        .unwrap();
    let mut altered = header.clone();
    altered.source_binding.as_mut().unwrap().lookup_state = Some(Arc::clone(&row.snapshot));
    assert!(
        altered
            .source_binding
            .as_ref()
            .unwrap()
            .original_authored_procedure_source_geometry(&altered, registry)
            .is_none()
    );
    let row = original_declaration_layouts(
        header
            .source_binding
            .as_ref()
            .unwrap()
            .declaration_layout_observations
            .as_deref()
            .unwrap(),
    )
    .unwrap()
    .next()
    .unwrap();
    let mut unavailable_entry = row.snapshot.state.clone();
    Arc::make_mut(&mut unavailable_entry.baseline).unknown_entry = true;
    altered.source_binding.as_mut().unwrap().lookup_state = Some(Arc::new(
        SourceLookupSnapshot::in_realm(unavailable_entry, row.snapshot.realm),
    ));
    assert!(
        altered
            .source_binding
            .as_ref()
            .unwrap()
            .original_authored_procedure_source_geometry(&altered, registry)
            .is_none()
    );
}
