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
use crate::analyser::ResolvedAnalysisInput;
use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};

fn logical_source(source: &str) -> (SourceCommandBindings, ResolvedAnalysisInput) {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
            .unwrap()
            .default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        input.lexer_config(),
        input.borrowed_context_registry().commands(),
        SourceAnalysisOptions::for_logical_source(&input).unwrap(),
    );
    (bindings, input)
}

#[test]
fn original_logical_root_procedure_and_alias_select_no_native_compiler_admission() {
    // naming.compiler.original-name-effect-separation
    // docs/design/analysis/name-resolution-proofs/original-compiler-name-effect-separation.md
    for source in [
        "list VALUE",
        "proc selected {value} {list $value}; selected VALUE",
        "proc selected {prefix value} {}; interp alias {} forwarded {} selected PREFIX; forwarded VALUE",
    ] {
        let (bindings, input) = logical_source(source);
        assert_eq!(
            bindings.final_state.logical_source_name_advice_input(),
            Some(&input)
        );
        assert!(bindings.final_state.baseline.native_entry.is_none());
        assert!(
            bindings
                .final_state
                .baseline
                .execution_name_policy
                .is_none()
        );
        assert_eq!(
            bindings.final_state.baseline.native_compilation.mode,
            NativeCompilationMode::Unknown
        );
        let points = bindings
            .points
            .iter()
            .filter(|point| point.dispatch)
            .collect::<Vec<_>>();
        assert_ne!(points.len(), 0);
        for point in points {
            assert!(point.compiled_execution.admission.is_none());
            assert!(point.compiled_execution.policy.is_none());
            assert!(point.compiled_execution.structured.is_none());
            let binding = bindings.invocation_at_source("", point.offset);
            assert!(binding.native_compilation_admission.is_none());
            assert!(binding.proved_execution_target().is_none());
            assert!(binding.proved_handler_target().is_none());
        }
    }
}

#[test]
fn original_logical_unknown_callback_stays_open_without_compiler_admission() {
    // naming.compiler.original-name-effect-separation
    // docs/design/analysis/name-resolution-proofs/original-compiler-name-effect-separation.md
    let source = "proc selected {value} {}; unknown; selected VALUE";
    let (bindings, input) = logical_source(source);
    let offset = u32::try_from(source.rfind("selected VALUE").unwrap()).unwrap();
    let binding = bindings.invocation_at_source("", offset);
    assert!(binding.native_compilation_admission.is_none());
    assert!(binding.unknown);
    assert!(binding.proved_execution_target().is_none());
    assert!(binding.proved_handler_target().is_none());
    let segments = tcl_source::segment::segment_commands_with_offset_and_config(
        source,
        0,
        input.lexer_config(),
    );
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        input.lexer_config(),
        segments.last().unwrap(),
    );
    bindings.stamp_original_tokens(&mut tokens);
    assert!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .original_logical_declaration_call_targets(
                &tokens,
                &input,
                input.borrowed_context_registry().commands(),
            )
            .is_none()
    );
}

#[test]
fn original_native_and_authored_unknown_compiler_purposes_are_not_logical_source() {
    // naming.compiler.original-name-effect-separation
    // docs/design/analysis/name-resolution-proofs/original-compiler-name-effect-separation.md
    let environment = tcl_registry::model::ingress::resolve_known_environment("tcl9.0").unwrap();
    let profile = environment.unit_profile();
    let context = environment.default_context_registry();
    let (owner, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let config = SourceAnalysisOptions {
        native_entry: Some(&entry),
        ..Default::default()
    }
    .native_lexer_config(tcl_lexer::LexerConfig::for_file_grammar(profile.grammar));
    let input = ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
    assert!(SourceAnalysisOptions::for_logical_source(&input).is_none());
    let source = "set checkpoint READY";
    let native = SourceCommandBindings::analyse_with_options(
        source,
        config,
        context.commands(),
        SourceAnalysisOptions {
            native_entry: Some(&entry),
            invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
            metadata_context: crate::registry_invocation::InvocationMetadataInput::SuppliedSource(
                Some(&input),
            ),
            ..Default::default()
        },
    );
    assert!(
        native
            .final_state
            .logical_source_name_advice_input()
            .is_none()
    );
    assert!(native.final_state.baseline.native_entry.is_some());
    assert_eq!(
        native
            .invocation_at_source("", 0)
            .native_compilation_admission,
        Some(NativeCompilationSelection::Unknown)
    );
    assert_eq!(entry.execution_point, environment.point());
    drop(owner);
    let dialect = tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    let authored = SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::for_file_grammar(dialect.lexer_grammar),
        &tcl_registry::CommandRegistry::build_default(),
        SourceAnalysisOptions {
            invocation_dialect: Some(dialect),
            ..Default::default()
        },
    );
    assert!(
        authored
            .final_state
            .logical_source_name_advice_input()
            .is_none()
    );
    assert_eq!(
        authored
            .invocation_at_source("", 0)
            .native_compilation_admission,
        Some(NativeCompilationSelection::Unknown)
    );
}
