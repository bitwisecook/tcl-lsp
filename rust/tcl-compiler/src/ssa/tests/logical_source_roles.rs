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

//! Reads and stores under logical source metadata: literal names, quiet scalar mutations, dictionary scopes and conditional store symbols.

use super::*;

#[test]
fn lowered_literal_read_names_keep_empty_and_dollar_data() {
    // Public native literal-name cases: experiments/name-handling/results/frame-local-name-geometry.
    let (context, unit) = logical_metadata_fixture("info exists {}; info exists {$name}");
    let registry = context.commands();
    let metadata = unit
        .top_level
        .invocation_metadata_context_for_module(registry, &unit.ir_module)
        .expect("actual retained Logical source input");
    let selection = SsaInvocationContext::supplied(registry, Some(metadata));
    let mut scanner = VarReferenceScanner::with_config(
        VarScanOptions::default(),
        unit.top_level.source_lexer_config(),
    );
    let reads: Vec<_> = unit
        .ir_module
        .top_level
        .statements
        .iter()
        .map(|statement| uses_of_classified_in_context(statement, &mut scanner, selection))
        .collect();
    assert!(
        reads[0].contains(&(String::new(), UseClass::Name)),
        "{reads:?}"
    );
    assert!(
        reads[1].contains(&("$name".to_owned(), UseClass::Name)),
        "{reads:?}"
    );
    assert!(
        !reads[1].iter().any(|(name, _)| name == "name"),
        "{reads:?}"
    );
}

#[test]
fn readonly_logical_roles_retain_source_reads_without_normal_or_native_ownership() {
    let (context, unit) = logical_metadata_fixture("info exists {}; info exists {$name}");
    let registry = context.commands();
    let metadata = unit
        .top_level
        .invocation_metadata_context_for_module(registry, &unit.ir_module)
        .unwrap();
    let selection = SsaInvocationContext::supplied(registry, Some(metadata));
    for statement in &unit.ir_module.top_level.statements {
        let tokens = statement.tokens().unwrap();
        assert!(selection.logical_structure(tokens).is_some());
        assert!(selection.normal(tokens).is_none());
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .variable_context
                .interpreter
                .is_none()
        );
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .variable_context
                .execution
                .is_none()
        );
    }
}

#[test]
fn readonly_logical_roles_keep_unknown_read_hazards_after_unclosed_commands() {
    let (context, unit) =
        logical_metadata_fixture("info exists {}; opaque_reader; info exists {$name}");
    let registry = context.commands();
    let metadata = unit
        .top_level
        .invocation_metadata_context_for_module(registry, &unit.ir_module)
        .unwrap();
    let selection = SsaInvocationContext::supplied(registry, Some(metadata));
    let statement = unit.ir_module.top_level.statements.last().unwrap();
    assert!(
        selection
            .logical_structure(statement.tokens().unwrap())
            .is_none()
    );
    let mut scanner = VarReferenceScanner::with_config(
        VarScanOptions::default(),
        unit.top_level.source_lexer_config(),
    );
    assert!(
        uses_of_classified_in_context(statement, &mut scanner, selection)
            .iter()
            .any(|(name, _)| name == "name")
    );
}

#[test]
fn quiet_logical_scalar_mutations_keep_original_names_without_numeric_or_normal_results() {
    let (context, unit) =
        logical_metadata_fixture("incr {}; incr {$counter}; info exists {$counter}");
    let registry = context.commands();
    let metadata = unit
        .top_level
        .invocation_metadata_context_for_module(registry, &unit.ir_module)
        .unwrap();
    let selection = SsaInvocationContext::supplied(registry, Some(metadata));
    let mut scanner = VarReferenceScanner::with_config(
        VarScanOptions::default(),
        unit.top_level.source_lexer_config(),
    );
    for (statement, expected) in unit
        .ir_module
        .top_level
        .statements
        .iter()
        .zip(["", "$counter", "$counter"])
    {
        let tokens = unit
            .ir_module
            .top_level
            .retained_source_tokens_for_statement(statement)
            .expect("original consumed or direct source words");
        assert!(selection.logical_structure(tokens).is_some());
        assert!(selection.normal(tokens).is_none());
        let variables = &tokens.source_binding.as_ref().unwrap().variable_context;
        assert!(variables.interpreter.is_none());
        assert!(variables.execution.is_none());
        assert!(variables.literal_value(expected, registry).is_none());
        let reads =
            uses_of_classified_with_source(statement, &mut scanner, selection, Some(tokens));
        assert!(
            reads.contains(&(expected.to_owned(), UseClass::Name)),
            "{reads:?}"
        );
        assert!(
            !reads.iter().any(|(name, _)| name == "counter"),
            "{reads:?}"
        );
    }
}

#[test]
fn dynamic_scalar_targets_keep_later_unknown_read_hazards() {
    let (context, unit) = logical_metadata_fixture("incr {}; incr $name; incr {$counter}");
    let registry = context.commands();
    let metadata = unit
        .top_level
        .invocation_metadata_context_for_module(registry, &unit.ir_module)
        .unwrap();
    let selection = SsaInvocationContext::supplied(registry, Some(metadata));
    let statement = unit.ir_module.top_level.statements.last().unwrap();
    assert!(
        selection
            .logical_structure(statement.tokens().unwrap())
            .is_none()
    );
    let mut scanner = VarReferenceScanner::with_config(
        VarScanOptions::default(),
        unit.top_level.source_lexer_config(),
    );
    assert!(
        uses_of_classified_in_context(statement, &mut scanner, selection)
            .iter()
            .any(|(name, _)| name == "counter")
    );
}

#[test]
fn quiet_logical_scalar_mutation_withdraws_prior_literal_contents() {
    let (context, unit) =
        logical_metadata_fixture("set counter 9; incr counter; info exists counter");
    let registry = context.commands();
    let statements = &unit.ir_module.top_level.statements;
    let before_update = &unit
        .ir_module
        .top_level
        .retained_source_tokens_for_statement(&statements[1])
        .expect("original scalar update source words")
        .source_binding
        .as_ref()
        .unwrap()
        .variable_context;
    assert_eq!(before_update.literal_value("counter", registry), Some("9"));
    let after_update = &unit
        .ir_module
        .top_level
        .retained_source_tokens_for_statement(&statements[2])
        .expect("original post-update source words")
        .source_binding
        .as_ref()
        .unwrap()
        .variable_context;
    assert!(after_update.literal_value("counter", registry).is_none());
    assert!(!after_update.dynamic_bindings);
    assert!(!after_update.dynamic_traces);
}

#[test]
fn empty_dictionary_scope_source_phases_keep_original_names_and_unknown_alias_contents() {
    let (context, unit) = logical_metadata_fixture(
        "set protected VALUE; dict with {} {}; dict with {$dict} {# comment only\n}; info exists {$dict}",
    );
    let registry = context.commands();
    let metadata = unit
        .top_level
        .invocation_metadata_context_for_module(registry, &unit.ir_module)
        .unwrap();
    let selection = SsaInvocationContext::supplied(registry, Some(metadata));
    let script = &unit.ir_module.top_level;
    let before = script
        .retained_source_tokens_for_statement(&script.statements[1])
        .unwrap();
    assert_eq!(
        before
            .source_binding
            .as_ref()
            .unwrap()
            .variable_context
            .literal_value("protected", registry),
        Some("VALUE")
    );
    let mut scanner = VarReferenceScanner::with_config(
        VarScanOptions::default(),
        unit.top_level.source_lexer_config(),
    );
    for (statement, expected) in script.statements[1..].iter().zip(["", "$dict", "$dict"]) {
        let tokens = script
            .retained_source_tokens_for_statement(statement)
            .unwrap();
        assert!(selection.logical_structure(tokens).is_some());
        assert!(selection.normal(tokens).is_none());
        let reads = uses_of_classified_in_context(statement, &mut scanner, selection);
        assert!(
            reads.contains(&(expected.to_owned(), UseClass::Name)),
            "{reads:?}"
        );
        assert!(!reads.iter().any(|(name, _)| name == "dict"));
    }
    let tokens = script
        .retained_source_tokens_for_statement(script.statements.last().unwrap())
        .unwrap();
    let variables = &tokens.source_binding.as_ref().unwrap().variable_context;
    assert!(variables.literal_value("protected", registry).is_none());
    assert!(!variables.dynamic_bindings);
    assert!(!variables.dynamic_traces);
    assert!(variables.interpreter.is_none());
    assert!(variables.execution.is_none());
}

fn dictionary_effect_fixture(
    access: tcl_registry::world_effect::StaticEffectAccess,
) -> (
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    crate::compilation_unit::CompilationUnit,
) {
    use std::sync::Arc;
    use tcl_registry::world_effect::{
        CallbackEffect, StaticEffectFootprint, WorldEffectDescriptor,
    };
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut registry = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut info = registry.get("info").unwrap().clone();
    info.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    registry.insert(info);
    let mut dictionary = registry.get("dict").unwrap().clone();
    let mut subcommands = dictionary.subcommands.to_vec();
    subcommands
        .iter_mut()
        .find(|subcommand| subcommand.name == "with")
        .unwrap()
        .world_effects = Some(WorldEffectDescriptor {
        static_footprint: StaticEffectFootprint {
            accesses: Box::leak(vec![access].into_boxed_slice()),
            callback: CallbackEffect::NONE,
        },
        ..WorldEffectDescriptor::EMPTY
    });
    dictionary.subcommands = Box::leak(subcommands.into_boxed_slice());
    registry.insert(dictionary);
    let context = Arc::new(baseline.with_command_store(Arc::new(registry)));
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let entry = crate::command_binding::SourceAnalysisEntry::for_logical_source(&input).unwrap();
    let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        "dict with {} {}; info exists {$name}",
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        &input,
    );
    (context, unit)
}

#[test]
fn dictionary_source_phases_refuse_unrepresented_explicit_writes() {
    use tcl_registry::world_effect::{
        EffectAccessMode, StaticEffectAccess, StaticInterpreterScope, StaticNamespaceScope,
        StaticSubjectScope, WorldStateDomain,
    };
    for (domain, mode, namespace, subject) in [
        (
            WorldStateDomain::VariableTraces,
            EffectAccessMode::Write,
            StaticNamespaceScope::Current,
            StaticSubjectScope::Wildcard,
        ),
        (
            WorldStateDomain::VariableTraces,
            EffectAccessMode::ReadWrite,
            StaticNamespaceScope::Current,
            StaticSubjectScope::Wildcard,
        ),
        (
            WorldStateDomain::VariableStore,
            EffectAccessMode::Write,
            StaticNamespaceScope::Named("::foreign"),
            StaticSubjectScope::Named("extra"),
        ),
        (
            WorldStateDomain::VariableStore,
            EffectAccessMode::Write,
            StaticNamespaceScope::Any,
            StaticSubjectScope::Wildcard,
        ),
        (
            WorldStateDomain::VariableStore,
            EffectAccessMode::Write,
            StaticNamespaceScope::Current,
            StaticSubjectScope::Wildcard,
        ),
    ] {
        let (context, unit) = dictionary_effect_fixture(StaticEffectAccess::new(
            domain,
            mode,
            StaticInterpreterScope::Current,
            namespace,
            subject,
        ));
        let registry = context.commands();
        let metadata = unit
            .top_level
            .invocation_metadata_context_for_module(registry, &unit.ir_module)
            .unwrap();
        let selection = SsaInvocationContext::supplied(registry, Some(metadata));
        let script = &unit.ir_module.top_level;
        let before = script
            .retained_source_tokens_for_statement(&script.statements[0])
            .unwrap();
        assert!(
            selection.logical_structure(before).is_some(),
            "{domain:?} {namespace:?} {subject:?}"
        );
        let after = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        assert!(
            selection.logical_structure(after).is_none(),
            "{domain:?} {namespace:?} {subject:?}"
        );
        assert!(selection.normal(after).is_none());
    }
}

#[test]
fn dictionary_scope_bodies_and_variable_observers_keep_lookup_hazards() {
    for source in [
        "dict with {} {opaque_body}; info exists {$name}",
        "trace add variable protected write observer; dict with {} {}; info exists {$name}",
        "dict with $target {}; info exists {$name}",
    ] {
        let (context, unit) = logical_metadata_fixture(source);
        let registry = context.commands();
        let metadata = unit
            .top_level
            .invocation_metadata_context_for_module(registry, &unit.ir_module)
            .unwrap();
        let selection = SsaInvocationContext::supplied(registry, Some(metadata));
        let script = &unit.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        assert!(selection.logical_structure(tokens).is_none(), "{source}");
        assert!(selection.normal(tokens).is_none(), "{source}");
    }
}

#[test]
fn conditional_logical_store_symbols_keep_original_empty_and_dollar_names() {
    let (context, unit) = logical_metadata_fixture("set {$literal} 9; set {} 8");
    let registry = context.commands();
    let function = &unit.top_level;
    let points = function.ssa.point_contexts.as_ref().unwrap();
    let mut found = HashSet::new();
    for (&block, data) in &function.cfg.blocks {
        for (index, _) in data.statements.iter().enumerate() {
            let Some(store) = points.conditional_logical_scalar_store_at(block, index, registry)
            else {
                continue;
            };
            let place = store.definition_place(registry).unwrap();
            let key = crate::var_resolve::canonical_binding_value_key(&place).unwrap();
            let source = function.ssa.source_symbols_at(block, index).unwrap();
            let symbol = source
                .get(store.source_name())
                .expect("original conditional store name");
            assert_eq!(function.ssa.cell_key(*symbol), Some(&key));
            assert!(place.cell.as_ref().unwrap().interpreter.is_none());
            assert!(place.cell.as_ref().unwrap().execution.is_none());
            assert!(!source.contains_key("literal"));
            let tokens = points.source_tokens_at(block, index).unwrap();
            let metadata = function
                .invocation_metadata_context_for_module(registry, &unit.ir_module)
                .unwrap();
            assert!(
                SsaInvocationContext::supplied(registry, Some(metadata))
                    .normal(tokens)
                    .is_none()
            );
            found.insert(store.source_name().to_owned());
        }
    }
    assert_eq!(found, HashSet::from(["$literal".to_owned(), String::new()]));
}

#[test]
fn conditional_store_symbol_owner_refuses_after_dynamic_target_and_unknown_command() {
    let (context, unit) =
        logical_metadata_fixture("set {$literal} 9; set $target 8; opaque_writer; set {$later} 7");
    let registry = context.commands();
    let function = &unit.top_level;
    let points = function.ssa.point_contexts.as_ref().unwrap();
    let mut checked = 0;
    for (&block, data) in &function.cfg.blocks {
        for (index, statement) in data.statements.iter().enumerate() {
            let Some(tokens) = statement.tokens() else {
                continue;
            };
            if tokens
                .argv_texts
                .get(1)
                .is_some_and(|name| name == "$later")
            {
                checked += 1;
                assert!(
                    points
                        .conditional_logical_scalar_store_at(block, index, registry)
                        .is_none()
                );
                assert!(
                    !function
                        .ssa
                        .source_symbols_at(block, index)
                        .unwrap()
                        .contains_key("$later")
                );
            }
        }
    }
    assert_eq!(checked, 1);
}

#[test]
fn original_literal_empty_reads_and_mutations_keep_name_data() {
    // Native cases: experiments/name-handling/results/ssa-empty-read-write-names.
    let source = "info exists {}; info exists {$name}; incr {}; incr {$counter}; dict with {} {}; dict with {$dict} {}";
    let (context, unit) = logical_metadata_fixture(source);
    let registry = context.commands();
    let function = &unit.top_level;
    let metadata = function
        .invocation_metadata_context_for_module(registry, &unit.ir_module)
        .expect("actual retained Logical source input");
    let selection = SsaInvocationContext::supplied(registry, Some(metadata));
    let mut scanner =
        VarReferenceScanner::with_config(VarScanOptions::default(), function.source_lexer_config());
    let statements = &unit.ir_module.top_level.statements;
    assert_eq!(statements.len(), 6);
    for (statement, expected) in statements
        .iter()
        .zip(["", "$name", "", "$counter", "", "$dict"])
    {
        let reads = uses_of_classified_in_context(statement, &mut scanner, selection);
        assert!(
            reads.contains(&(expected.to_owned(), UseClass::Name)),
            "{expected:?}: {reads:?}"
        );
        if let Some(unprefixed) = expected.strip_prefix('$') {
            assert!(!reads.iter().any(|(name, _)| name == unprefixed));
        }
    }
}

#[test]
fn original_loop_name_lists_keep_empty_and_dollar_elements() {
    // Native foreach cases: experiments/name-handling/results/ssa-empty-read-write-names.
    let source = "foreach {{} {$name}} $values {}";
    let (context, unit) = logical_metadata_fixture(source);
    let registry = context.commands();
    let function = &unit.top_level;
    let script = &unit.ir_module.top_level;
    let tokens = script
        .retained_source_tokens_for_statement(&script.statements[0])
        .expect("actual original loop words");
    let metadata = function
        .invocation_metadata_context_for_module(registry, &unit.ir_module)
        .expect("actual retained Logical source input");
    let selected = selected_barrier_defs(
        SsaInvocationContext::supplied(registry, Some(metadata)),
        tokens,
    );
    assert!(selected.contains(&String::new()), "{selected:?}");
    assert!(selected.contains(&"$name".to_owned()), "{selected:?}");
    assert!(!selected.contains(&"name".to_owned()), "{selected:?}");
    assert_eq!(
        selected_barrier_defs(SsaInvocationContext::supplied(registry, None), tokens),
        [] as [std::string::String; 0]
    );
}
