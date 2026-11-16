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

use crate::analyser::ResolvedAnalysisInput;
use crate::ir::CommandTokens;

fn original_statement(source: &str, selected: &str) -> (ResolvedAnalysisInput, CommandTokens) {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
            .unwrap()
            .default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let mut lowerer = crate::lowering::Lowerer::with_config(
        input.borrowed_context_registry().commands(),
        input.lexer_config(),
    )
    .with_resolved_analysis_input(input.clone());
    lowerer.lower(source);
    let module = lowerer.finish_module(source);
    assert_eq!(module.source_metadata_input.as_ref(), Some(&input));
    assert_eq!(module.lexer_config, input.lexer_config());
    let offset = u32::try_from(source.rfind(selected).unwrap()).unwrap();
    let mut found = Vec::new();
    let mut collect = |statement: &crate::ir::Statement| {
        if statement.span().start() == offset {
            found.push(
                statement
                    .tokens()
                    .expect("original statement vector")
                    .clone(),
            );
        }
    };
    crate::ir::for_each_statement(&module.top_level, &mut collect);
    for procedure in module.procedures.values() {
        crate::ir::for_each_statement(&procedure.body, &mut collect);
    }
    assert_eq!(found.len(), 1);
    (input, found.pop().unwrap())
}

#[test]
fn original_logical_nested_declaration_retains_availability_without_dispatch() {
    // naming.compiler.original-positioned-callee-body-availability
    // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
    for (source, selected, prefix) in [
        (
            "proc subject {} {package require Tcl}",
            "package require Tcl",
            0,
        ),
        (
            "interp alias {} ensure {} package require; proc subject {} {ensure Tcl}",
            "ensure Tcl",
            1,
        ),
    ] {
        let (input, tokens) = original_statement(source, selected);
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.lookup_state.is_none());
        assert!(binding.unknown);
        assert_eq!(
            binding.targets,
            [] as [crate::command_binding::SourceCommandTarget; 0]
        );
        assert!(binding.original_logical_declaration_targets.is_some());
        let targets = binding
            .original_logical_declaration_call_targets(
                &tokens,
                &input,
                input.borrowed_context_registry().commands(),
            )
            .expect("closed conditional original declaration");
        let [target] = targets.as_slice() else {
            panic!("one selected source target")
        };
        assert!(target.registry_backed);
        assert_eq!(target.prepended.len(), prefix);
        assert!(target.implementation_allocation.is_none());
        assert!(binding.proved_target().is_none());
        assert!(binding.proved_execution_target().is_none());
        assert!(binding.proved_handler_target().is_none());
        assert!(binding.native_compiler_admission.is_none());
        assert!(binding.native_compilation_admission.is_none());
    }
}

fn assert_changed_declared_call_inputs(
    binding: &crate::command_binding::SourceInvocationBinding,
    tokens: &CommandTokens,
    input: &ResolvedAnalysisInput,
    registry: &tcl_registry::CommandRegistry,
) {
    for axis in [0, 1] {
        let mut config = input.lexer_config();
        if axis == 0 {
            config.leading_bom = match config.leading_bom {
                tcl_lexer::LeadingBom::Content => tcl_lexer::LeadingBom::Skip,
                tcl_lexer::LeadingBom::Skip => tcl_lexer::LeadingBom::Content,
            };
        } else {
            config.strict_quoting = !config.strict_quoting;
        }
        let changed = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            config,
        );
        assert!(
            binding
                .original_logical_declaration_call_targets(tokens, &changed, registry)
                .is_none()
        );
    }
}

fn assert_changed_original_call_refused(
    changed: &CommandTokens,
    input: &ResolvedAnalysisInput,
    registry: &tcl_registry::CommandRegistry,
) {
    assert!(
        changed
            .source_binding
            .as_ref()
            .unwrap()
            .original_logical_declaration_call_targets(changed, input, registry,)
            .is_none()
    );
}

#[test]
fn original_logical_declaration_receipt_rejects_changed_inputs_rows_and_world() {
    // naming.compiler.original-positioned-callee-body-availability
    // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
    let (input, tokens) = original_statement(
        "proc subject {} {package require Tcl}",
        "package require Tcl",
    );
    let registry = input.borrowed_context_registry().commands();
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(
        binding
            .original_logical_declaration_call_targets(&tokens, &input, registry)
            .is_some()
    );
    assert_changed_declared_call_inputs(binding, &tokens, &input, registry);
    let other_input = ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::resolve_known_environment("tcl9.0")
            .unwrap()
            .default_context_registry(),
        input.lexer_config(),
    );
    assert!(
        binding
            .original_logical_declaration_call_targets(
                &tokens,
                &other_input,
                other_input.borrowed_context_registry().commands(),
            )
            .is_none()
    );
    let (_, foreign) = original_statement(
        "proc another {} {package require Tcl}",
        "package require Tcl",
    );
    let mut changed = tokens.clone();
    changed
        .source_binding
        .as_mut()
        .unwrap()
        .declaration_layout_observations = foreign
        .source_binding
        .as_ref()
        .unwrap()
        .declaration_layout_observations
        .clone();
    assert_changed_original_call_refused(&changed, &input, registry);
    let mut changed = tokens.clone();
    changed.source_binding.as_mut().unwrap().unknown = false;
    assert_changed_original_call_refused(&changed, &input, registry);
    let mut changed = tokens.clone();
    changed.source_binding.as_mut().unwrap().may_be_absent = true;
    assert_changed_original_call_refused(&changed, &input, registry);
    let mut changed = tokens.clone();
    changed
        .source_binding
        .as_mut()
        .unwrap()
        .join(&super::super::SourceInvocationBinding::default());
    assert!(
        changed
            .source_binding
            .as_ref()
            .unwrap()
            .original_logical_declaration_targets
            .is_none()
    );
    assert_changed_original_call_refused(&changed, &input, registry);
    let mut changed = tokens.clone();
    changed.synthetic = Some(crate::ir::SyntheticMarker::EvaluatedArguments);
    assert!(
        binding
            .original_logical_declaration_call_targets(&changed, &input, registry)
            .is_none()
    );
    let mut changed = tokens.clone();
    changed.word_exprs.pop();
    assert!(
        binding
            .original_logical_declaration_call_targets(&changed, &input, registry)
            .is_none()
    );
}

#[test]
fn original_logical_declaration_receipt_keeps_reached_and_opaque_guards() {
    // naming.compiler.original-positioned-callee-body-availability
    // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
    let (input, tokens) = original_statement("package require Tcl", "package require Tcl");
    let registry = input.borrowed_context_registry().commands();
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(binding.lookup_state.is_some());
    assert!(binding.original_logical_declaration_targets.is_none());
    assert!(
        binding
            .original_logical_declaration_call_targets(&tokens, &input, registry)
            .is_some()
    );
    for axis in 0..3 {
        let mut changed = tokens.clone();
        let binding = changed.source_binding.as_mut().unwrap();
        match axis {
            0 => binding.unknown = true,
            1 => binding.may_be_absent = true,
            _ => binding.targets.clear(),
        }
        assert!(
            changed
                .source_binding
                .as_ref()
                .unwrap()
                .original_logical_declaration_call_targets(&changed, &input, registry,)
                .is_none()
        );
    }
    let (input, tokens) = original_statement(
        "proc subject {} {$callback; package require Tcl}",
        "package require Tcl",
    );
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(binding.original_logical_declaration_targets.is_none());
    assert!(
        binding
            .original_logical_declaration_call_targets(
                &tokens,
                &input,
                input.borrowed_context_registry().commands(),
            )
            .is_none()
    );
    assert!(binding.proved_execution_target().is_none());
}
