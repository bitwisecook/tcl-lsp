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

use crate::interprocedural::test_body_identity::fixture_id;

use super::*;
use tcl_registry::StateTransition;

use crate::ir::{SourceSite, WordOpacity};
use crate::segmenter::segment_commands;

mod body_assistance;
mod expression_operand_advice;
mod invocation_assistance;
mod metadata_context;
mod normal_representation;
mod normal_transfer;
mod structured_invocation;
mod variable_transitions;
mod word_resolution;

fn logical_source_tokens(
    source: &str,
    registry: &CommandRegistry,
    input: &crate::analyser::ResolvedAnalysisInput,
) -> CommandTokens {
    let image = tcl_lexer::SourceImage::document(source);
    let config = input.lexer_config();
    let bindings =
        crate::command_binding::SourceCommandBindings::analyse_image_in_frame_with_options(
            &image,
            &crate::var_resolve::VariableExecutionFrame::Unknown,
            config,
            registry,
            crate::command_binding::SourceAnalysisOptions::for_logical_source(input).unwrap(),
        )
        .unwrap();
    let segment =
        crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)
            .unwrap()
            .pop()
            .unwrap();
    let mut tokens = CommandTokens::from_segmented(&image.source_map(), config, &segment);
    bindings.stamp_original_tokens(&mut tokens);
    tokens
}

fn test_context() -> SemanticContext {
    SemanticContext::for_environment("tcl8.6")
}

fn literal(text: &str) -> WordExpr {
    WordExpr::Literal {
        text: text.to_owned(),
        source: SourceSite::source(tcl_lexer::Span::new(0, 0)),
    }
}

fn last_source_tokens(source: &str) -> CommandTokens {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    let bindings = crate::command_binding::SourceCommandBindings::analyse(source, config, registry);
    let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
        .pop()
        .unwrap();
    CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment)
        .with_source_binding(
            bindings.invocation_at_source("ignored-authored-head", segment.span.start()),
        )
}

#[test]
fn native_operation_plan_retains_original_words_and_reports_missing_source() {
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    let rules = WordValueRules::from_config(&config);
    let source = r"se\x74 value 7";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let bindings = crate::command_binding::SourceCommandBindings::analyse_with_options(
        source,
        config,
        registry,
        crate::command_binding::SourceAnalysisOptions {
            invocation_dialect: Some(crate::environment_ingress::authoring_invocation_dialect(
                registry,
                registry.profile(),
                config,
            )),
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                ..crate::environment_ingress::authoring_native_compilation()
            },
            ..crate::command_binding::SourceAnalysisOptions::default()
        },
    );
    let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
        .pop()
        .unwrap();
    let mut tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment)
            .with_source_binding(bindings.invocation_at_source("set", 0));
    let plan = native_operation_selection_plan(&tokens, config.escapes, rules)
        .unwrap()
        .unwrap();
    assert_eq!(plan.source, r"se\x74 value 7");
    assert_eq!(plan.requirements[0].name, "set");
    assert_eq!(
        plan.guard,
        tcl_runtime_api::CommandBindingGuard::BeforeArguments
    );
    tokens.argv_texts.pop();
    assert_eq!(
        native_operation_selection_plan(&tokens, config.escapes, rules),
        Err(NativeOperationSelectionDecline::MissingOriginalSource),
    );
    let generic = last_source_tokens("pid");
    assert_eq!(
        native_operation_selection_plan(&generic, config.escapes, rules),
        Ok(None)
    );
}

fn f5_source_tokens(source: &str) -> CommandTokens {
    f5_source_tokens_in_realm(source, tcl_dialect::model::InvocationRealm::RuleLoader)
}

fn f5_source_tokens_in_realm(
    source: &str,
    realm: tcl_dialect::model::InvocationRealm,
) -> CommandTokens {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let config = tcl_lexer::LexerConfig::for_dialect("f5-irules");
    let analysis = crate::command_binding::SourceCommandBindings::analyse_with_options(
        source,
        config,
        registry,
        crate::command_binding::SourceAnalysisOptions {
            invocation_realm: realm,
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                tcl_dialect::DialectProfile::find("f5-irules").expect("selected fixture entry"),
            )),
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                catch_depth: Some(0),
                loop_depth: 0,
            },
            ..crate::command_binding::SourceAnalysisOptions::default()
        },
    );
    let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
        .pop()
        .unwrap();
    CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment)
        .with_source_binding(analysis.invocation_at_source("ignored", segment.span.start()))
}
