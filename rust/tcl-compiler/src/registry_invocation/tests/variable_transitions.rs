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

//! Possible variable name operands: alias prefixes, expansions and method names.

use super::*;

#[test]
fn possible_name_operands_preserve_alias_prefix_and_missing_binding_residual() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut tokens = last_source_tokens("interp alias {} store {} set target; store VALUE");
    let possible = possible_variable_name_operands(registry, None, &tokens).unwrap();
    assert!(
        possible
            .operands()
            .any(|(role, word)| role == tcl_registry::ArgRole::VarWrite
                && word == &EffectiveInvocationWord::Literal("target".to_owned()))
    );
    let mut unknown_binding = crate::command_binding::SourceInvocationBinding::default();
    unknown_binding.unknown = true;
    tokens.source_binding = Some(unknown_binding);
    let unknown = possible_variable_name_operands(registry, None, &tokens).unwrap();
    assert!(unknown.unknown_residual());
    assert!(unknown.operands().next().is_none());
    tokens.source_binding = Some(crate::command_binding::SourceInvocationBinding::default());
    let closed = possible_variable_name_operands(registry, None, &tokens).unwrap();
    assert!(!closed.unknown_residual());
    assert!(closed.operands().next().is_none());
    tokens.source_binding = None;
    assert!(possible_variable_name_operands(registry, None, &tokens).is_none());
}

#[test]
fn possible_name_operands_keep_expansion_cardinality_separate_from_name_values() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    for (command, dynamic, expected_roles) in [
        ("set {*}$n VALUE", true, 2),
        ("set fixed {*}$n", false, 2),
        ("set {*}{fixed} VALUE", false, 1),
        ("set fixed VALUE EXTRA", false, 0),
    ] {
        let source = format!("proc f {{n}} {{{command}; return ok}}");
        let bindings =
            crate::command_binding::SourceCommandBindings::analyse(&source, config, registry);
        let offset = source.find(command).unwrap();
        let offset_u32 = u32::try_from(offset).unwrap();
        let segment = tcl_source::segment::segment_commands_with_offset_and_config(
            &source[offset..],
            offset_u32,
            config,
        )
        .remove(0);
        let tokens =
            CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(&source), config, &segment)
                .with_source_binding(
                    bindings.invocation_at_source("unused-written-head", offset_u32),
                );
        let possible = possible_variable_name_operands(registry, None, &tokens).unwrap();
        let mut operands = Vec::new();
        for operand in possible.operands() {
            if !operands.contains(&operand) {
                operands.push(operand);
            }
        }
        assert_eq!(operands.len(), expected_roles, "{command}");
        assert_eq!(
            operands
                .iter()
                .any(|(_, word)| matches!(word, EffectiveInvocationWord::Dynamic)),
            dynamic,
            "{command}"
        );
        if dynamic {
            assert!(normal_transfer_invocation(registry, None, &tokens).is_none());
        }
    }
}

#[test]
fn evaluated_wrapper_retains_only_possible_output_names_and_lookup_phase() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let mut tokens = f5_source_tokens("catch {foo} ::err");
    tokens.synthetic = Some(crate::ir::SyntheticMarker::EvaluatedWrapper);
    let possible = possible_variable_name_operands(registry, None, &tokens).unwrap();
    assert!(possible.unknown_residual());
    assert!(possible.phased_operands().any(|(role, word, phase, destroys)| {
            role == tcl_registry::ArgRole::VarWrite
                && !destroys
                && word == &EffectiveInvocationWord::Literal("::err".to_owned())
                && phase
                    == tcl_registry::native_compilation::VariableOperandBindingPhase::NormalContinuation
        }));
    assert!(normal_transfer_invocation(registry, None, &tokens).is_none());
    assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
    tokens.synthetic = Some(crate::ir::SyntheticMarker::EvaluatedArguments);
    assert!(possible_variable_name_operands(registry, None, &tokens).is_none());

    let mut replaced = f5_source_tokens("proc catch args {return CUSTOM}; catch BODY ::err");
    replaced.synthetic = Some(crate::ir::SyntheticMarker::EvaluatedWrapper);
    assert!(
        possible_variable_name_operands(registry, None, &replaced)
            .unwrap()
            .operands()
            .next()
            .is_none()
    );
}

#[test]
fn possible_method_name_operands_do_not_create_normal_store_proof() {
    use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};
    let source = "oo::class create C {method m {} {set state VALUE}}";
    let config = tcl_lexer::LexerConfig::default();
    let registry = CommandRegistry::build_default();
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        config,
        &registry,
        SourceAnalysisOptions {
            invocation_dialect: Some(tcl_native_model::InvocationDialect::for_version(
                tcl_dialect::TclVersion::V9_1,
            )),
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                ..Default::default()
            },
            ..SourceAnalysisOptions::default()
        },
    );
    let offset = u32::try_from(source.find("set state").unwrap()).unwrap();
    let segment = tcl_source::segment::segment_commands_with_offset_and_config(
        "set state VALUE",
        offset,
        config,
    )
    .pop()
    .unwrap();
    let tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment)
            .with_source_binding(bindings.invocation_at_source("set", offset));
    let possible = possible_variable_name_operands(&registry, None, &tokens).unwrap();
    assert!(possible.unknown_residual());
    assert!(
        possible
            .operands()
            .any(|(role, word)| role == tcl_registry::ArgRole::VarWrite
                && word == &EffectiveInvocationWord::Literal("state".to_owned()))
    );
    assert!(normal_transfer_invocation(&registry, None, &tokens).is_none());
}
