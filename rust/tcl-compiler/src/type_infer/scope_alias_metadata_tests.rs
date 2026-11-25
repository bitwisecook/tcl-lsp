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

use crate::ir::Statement;
#[test]
fn original_alias_type_widening_requires_the_actual_invocation_and_input() {
    // naming.analysis.selected-scope-alias-role-metadata
    // docs/design/analysis/name-resolution-proofs/selected-scope-alias-role-metadata.md
    // Genuine compiled source carriers; values/links and actual execution remain separate.
    let source = "interp alias {} selectedGlobal {} global; selectedGlobal original";
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let entry = crate::command_binding::SourceAnalysisEntry::for_logical_source(&input).unwrap();
    let module = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config: input.lexer_config(),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        &input,
    )
    .ir_module;
    let registry = context.commands();
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, &module)
            .unwrap();
    let statement = module.top_level.statements.iter()
            .find(|statement| matches!(statement, Statement::Call { command, .. } if command == "selectedGlobal"))
            .expect("original Logical aliased invocation");
    assert_eq!(
        super::original_scope_alias_call(registry, Some(metadata), statement),
        Some(true)
    );
    assert!(super::original_scope_alias_call(registry, None, statement).is_none());
    let older = tcl_registry::model::ingress::static_context_for("tcl8.4");
    assert!(super::original_scope_alias_call(registry, Some(older.into()), statement).is_none());
    let mut missing = statement.clone();
    missing.tokens_mut().unwrap().source_binding = None;
    assert!(super::original_scope_alias_call(registry, Some(metadata), &missing).is_none());
    let mut changed = statement.clone();
    changed.tokens_mut().unwrap().word_exprs.pop();
    assert!(super::original_scope_alias_call(registry, Some(metadata), &changed).is_none());
}
#[test]
fn native_jim_alias_metadata_cannot_use_raw_logical_name_eligibility() {
    // naming.analysis.selected-scope-alias-role-metadata
    // docs/design/analysis/name-resolution-proofs/selected-scope-alias-role-metadata.md
    // Real software entry and original source only; no external Jim execution.
    let profile = tcl_dialect::DialectProfile::find("jimtcl").unwrap();
    let context =
        tcl_registry::model::ingress::resolve_environment("jimtcl").default_context_registry();
    let registry = context.commands();
    let (_owner, captured) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    assert!(captured.command_name_policy().is_none());
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(captured)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let unit = crate::compilation_unit::CompilationUnit::build_with_context_registry(
        "global imported",
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_dialect(profile.name),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        std::sync::Arc::clone(&context),
    );
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_module(
        registry,
        &unit.ir_module,
    )
    .unwrap();
    assert!(metadata.permits_logical_source_names());
    let statement = unit.ir_module.top_level.statements.first().unwrap();
    let tokens = statement.tokens().unwrap();
    assert_eq!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .original_invocation_source_purpose(tokens, metadata, registry,),
        Some(crate::command_binding::OriginalInvocationSourcePurpose::Execution)
    );
    assert!(super::original_scope_alias_call(registry, Some(metadata), statement).is_none());
}

#[test]
fn dialect_variable_writes_retain_the_selected_types_and_frame_policy() {
    use crate::analyser::ResolvedAnalysisInput;
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
    use crate::types::{TclType, TypeLattice};
    use crate::var_escape::{EscapeTag, analyse_var_escape_cu};
    use std::sync::Arc;
    use tcl_core_types::BodyFrameId;

    let jim_loop = "proc examine {} {loop item 0 {}; return $item}";
    let shared = "when CLIENT_DATA {set item 1; sharedvar item; return $item}";
    for (dialect, source, name, expected_types) in [
        (
            "jimtcl",
            jim_loop,
            "::examine",
            vec![TypeLattice::overdefined()],
        ),
        ("tcl8.6", jim_loop, "::examine", vec![]),
        (
            "f5-irules",
            shared,
            "::when::CLIENT_DATA",
            vec![TypeLattice::of(TclType::Int), TypeLattice::overdefined()],
        ),
    ] {
        let environment = tcl_registry::model::ingress::resolve_environment(dialect);
        let profile = environment.unit_profile();
        let context = environment.default_context_registry();
        let input = ResolvedAnalysisInput::new(
            environment.analyser_profile(),
            profile,
            Arc::clone(&context),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let unit = CompilationUnit::build_with_analysis_input(
            source,
            UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        );
        let function = unit.function(name).unwrap();
        let mut types: Vec<_> = function
            .types
            .iter()
            .filter(|((symbol, _), _)| function.ssa.var_name(*symbol) == "item")
            .map(|((_, version), inferred)| (*version, inferred.clone()))
            .collect();
        types.sort_unstable_by_key(|(version, _)| *version);
        assert_eq!(
            types
                .into_iter()
                .map(|(_, inferred)| inferred)
                .collect::<Vec<_>>(),
            expected_types,
            "{dialect}"
        );

        let summaries = analyse_var_escape_cu(&unit, false);
        let summary = &summaries[&BodyFrameId::Body(function.body_id.unwrap())];
        assert!(!summary.local_slots.contains_key("item"), "{dialect}");
        if dialect == "f5-irules" {
            assert_eq!(summary.tags.get("item"), Some(&EscapeTag::Frame));
        }
    }
}
