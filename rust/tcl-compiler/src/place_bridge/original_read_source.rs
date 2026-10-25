// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original argv source geometry, separate from physical invocation reads.

use crate::ir::CommandTokens;
use crate::registry_invocation::{InvocationMetadataContext, OwnedInvocationMetadataContext};
use crate::word_subst::LiftedCall;
use tcl_registry::CommandRegistry;

#[derive(Clone, Copy)]
pub(crate) enum ArgumentReadInput<'a> {
    Standalone,
    Supplied(Option<InvocationMetadataContext<'a>>),
}

impl<'a> ArgumentReadInput<'a> {
    pub(crate) fn for_owner(
        owner: &'a OwnedInvocationMetadataContext,
        registry: &CommandRegistry,
    ) -> Self {
        if owner.is_standalone() {
            Self::Standalone
        } else {
            Self::Supplied(
                owner
                    .metadata_context(registry)
                    .and_then(crate::registry_invocation::InvocationMetadataSelection::context),
            )
        }
    }

    pub(crate) fn metadata(
        self,
        registry: &CommandRegistry,
    ) -> Option<InvocationMetadataContext<'a>> {
        match self {
            Self::Standalone => registry
                .profile()
                .map(tcl_registry::model::semantic::SemanticContext::for_profile)
                .map(Into::into),
            Self::Supplied(metadata) => metadata,
        }
    }
}

/// These producer markers operate on already evaluated values or separate
/// effects. They never substitute the original vector again.
pub(crate) fn evaluates_arguments(tokens: &CommandTokens) -> bool {
    crate::command_binding::original_name_value::OriginalWordSourcePurpose::ArgumentEvaluation
        .accepts(tokens)
}

/// None means missing/incompatible source ownership or an incomplete walk.
/// Some(empty) means complete absence, including a non-evaluating phase.
pub(crate) fn argument_calls(
    tokens: &CommandTokens,
    registry: &CommandRegistry,
    input: ArgumentReadInput<'_>,
) -> Option<Vec<LiftedCall>> {
    if !evaluates_arguments(tokens) {
        return Some(Vec::new());
    }
    match input {
        ArgumentReadInput::Standalone => {
            let config = tokens
                .source_binding
                .as_ref()
                .and_then(|binding| binding.original_argument_lexer_config(tokens))
                .unwrap_or_else(|| super::invocation_read_grammar(tokens, registry));
            crate::word_subst::checked_argument_lifted_calls(tokens, config)
        }
        ArgumentReadInput::Supplied(Some(metadata)) if metadata.is_standalone() => {
            argument_calls(tokens, registry, ArgumentReadInput::Standalone)
        }
        ArgumentReadInput::Supplied(Some(metadata)) => {
            crate::word_subst::checked_original_argument_lifted_calls(tokens, registry, metadata)
        }
        ArgumentReadInput::Supplied(None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::{Analyser, ResolvedAnalysisInput};
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
    use crate::ir::SyntheticMarker;
    use crate::registry_invocation::OwnedInvocationMetadataContext;
    use std::sync::Arc;

    fn source_input(config: tcl_lexer::LexerConfig) -> ResolvedAnalysisInput {
        let mut analyser = Analyser::new();
        // Logical source naming and the supplied lexical policy are independent.
        let original = analyser.prepare_analysis_input("tcl").unwrap();
        ResolvedAnalysisInput::new(
            original.analyser_profile(),
            original.unit_profile(),
            original.context_registry(),
            config,
        )
    }

    /// Native CFG controls retain actual selected availability independently
    /// from their complete caller-supplied lexical configuration.
    fn native_source_input(config: tcl_lexer::LexerConfig) -> ResolvedAnalysisInput {
        let environment = tcl_registry::model::ingress::resolve_known_environment("tcl9.1")
            .expect("selected native CFG source environment");
        ResolvedAnalysisInput::new(
            environment.analyser_profile(),
            environment.unit_profile(),
            environment.default_context_registry(),
            config,
        )
    }

    fn original_tokens(source: &str, input: &ResolvedAnalysisInput) -> CommandTokens {
        let image = tcl_lexer::SourceImage::document(source);
        let registry = input.borrowed_context_registry().commands();
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
        let segments =
            crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)
                .unwrap();
        let [command] = segments.as_slice() else {
            panic!("one original command")
        };
        let mut tokens = CommandTokens::from_segmented(&image.source_map(), config, command);
        bindings.stamp_original_tokens(&mut tokens);
        tokens
    }

    fn supplied_unit(
        source: &str,
        input: &ResolvedAnalysisInput,
        entry: Option<&crate::command_binding::SourceAnalysisEntry>,
    ) -> CompilationUnit {
        CompilationUnit::build_with_analysis_input(
            source,
            UnitBuildOptions {
                registry: input.borrowed_context_registry().commands(),
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(input.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            entry,
            input,
        )
    }

    #[test]
    fn original_argument_inventory_keeps_full_config_and_excludes_enclosing_roles() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Source argv inventory only; no Expr/body entry, handler or Native read grant.
        for leading_bom in [tcl_lexer::LeadingBom::Skip, tcl_lexer::LeadingBom::Content] {
            let config = tcl_lexer::LexerConfig {
                leading_bom,
                strict_quoting: true,
                braced_var: tcl_lexer::BracedVarStyle::FirstClose,
                ..tcl_lexer::LexerConfig::from_grammar(
                    tcl_dialect::DialectProfile::find("tcl9.1").unwrap().grammar,
                )
            };
            let input = source_input(config);
            let registry = input.borrowed_context_registry().commands();
            let owner = OwnedInvocationMetadataContext::for_source_input(Some(&input));
            for source in ["expr {[dict size DATA]}", "eval {[puts VALUE]}"] {
                let tokens = original_tokens(source, &input);
                assert!(
                    argument_calls(
                        &tokens,
                        registry,
                        ArgumentReadInput::for_owner(&owner, registry)
                    )
                    .unwrap()
                    .is_empty(),
                    "{source}"
                );
                let binding = tokens.source_binding.as_ref().unwrap();
                assert_eq!(
                    binding.original_argument_lexer_config(&tokens),
                    Some(config)
                );
                let mut boundary = tokens.clone();
                boundary.synthetic = Some(SyntheticMarker::EvaluatedArguments);
                assert!(
                    binding
                        .original_lexer_config_for_tokens(&boundary)
                        .is_none()
                );
                assert!(
                    argument_calls(
                        &boundary,
                        registry,
                        ArgumentReadInput::for_owner(&owner, registry)
                    )
                    .unwrap()
                    .is_empty()
                );
                assert!(
                    crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
                        registry,
                        owner.metadata_context(registry).and_then(
                            crate::registry_invocation::InvocationMetadataSelection::context
                        ),
                        &boundary
                    )
                    .is_none()
                );
            }
            let tokens = original_tokens("list [dict size DATA]", &input);
            let calls = argument_calls(
                &tokens,
                registry,
                ArgumentReadInput::for_owner(&owner, registry),
            )
            .unwrap();
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].command, "dict");
            assert_eq!(
                calls[0]
                    .tokens
                    .as_ref()
                    .unwrap()
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .original_argument_lexer_config(calls[0].tokens.as_ref().unwrap()),
                Some(config)
            );
            let bom = original_tokens("list [\u{FEFF}dict size DATA]", &input);
            let lexical = crate::word_subst::checked_argument_lifted_calls(&bom, config).unwrap();
            assert_eq!(lexical.len(), 1);
            assert_eq!(lexical[0].command, "\u{FEFF}dict");
            assert_eq!(
                lexical[0].tokens.as_ref().unwrap().words()[0]
                    .source()
                    .span
                    .start(),
                6
            );
            assert_eq!(
                bom.source_binding
                    .as_ref()
                    .unwrap()
                    .original_argument_lexer_config(&bom),
                Some(config)
            );
        }
    }

    fn assert_changed_argument_inventory(
        tokens: CommandTokens,
        input: &ResolvedAnalysisInput,
        registry: &CommandRegistry,
        owner: &OwnedInvocationMetadataContext,
    ) {
        let mut changed = tokens.clone();
        changed.word_exprs = original_tokens("list [dict size OTHER]", input).word_exprs;
        assert!(
            argument_calls(
                &changed,
                registry,
                ArgumentReadInput::for_owner(owner, registry)
            )
            .is_none()
        );
        let mut detached = tokens;
        detached.source_binding = None;
        assert!(
            argument_calls(
                &detached,
                registry,
                ArgumentReadInput::for_owner(owner, registry)
            )
            .is_none()
        );
    }

    #[test]
    fn original_argument_inventory_refuses_changed_missing_and_foreign_owners() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let input = source_input(tcl_lexer::LexerConfig::default());
        let registry = input.borrowed_context_registry().commands();
        let tokens = original_tokens("list [dict size DATA]", &input);
        let owner = OwnedInvocationMetadataContext::for_source_input(Some(&input));
        assert_eq!(
            argument_calls(
                &tokens,
                registry,
                ArgumentReadInput::for_owner(&owner, registry)
            )
            .unwrap()
            .len(),
            1
        );
        for unavailable in [
            OwnedInvocationMetadataContext::Unavailable,
            OwnedInvocationMetadataContext::Supplied(input.context_registry()),
        ] {
            assert!(
                argument_calls(
                    &tokens,
                    registry,
                    ArgumentReadInput::for_owner(&unavailable, registry)
                )
                .is_none()
            );
        }
        let changed_input = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            tcl_lexer::LexerConfig {
                strict_quoting: true,
                ..input.lexer_config()
            },
        );
        let changed_owner = OwnedInvocationMetadataContext::for_source_input(Some(&changed_input));
        assert!(
            argument_calls(
                &tokens,
                registry,
                ArgumentReadInput::for_owner(&changed_owner, registry)
            )
            .is_none()
        );
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(input.borrowed_context_registry().commands())),
        );
        let older_input = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            older,
            input.lexer_config(),
        );
        let older_owner = OwnedInvocationMetadataContext::for_source_input(Some(&older_input));
        assert!(
            argument_calls(
                &tokens,
                registry,
                ArgumentReadInput::for_owner(&older_owner, registry)
            )
            .is_none()
        );
        let foreign_context = Arc::new(input.borrowed_context_registry().with_command_store(
            Arc::new(CommandRegistry::build_default().project_for_profile(input.unit_profile())),
        ));
        let foreign = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            foreign_context,
            input.lexer_config(),
        );
        let foreign_owner = OwnedInvocationMetadataContext::for_source_input(Some(&foreign));
        assert!(
            argument_calls(
                &tokens,
                foreign.borrowed_context_registry().commands(),
                ArgumentReadInput::for_owner(
                    &foreign_owner,
                    foreign.borrowed_context_registry().commands()
                )
            )
            .is_none()
        );
        assert_changed_argument_inventory(tokens, &input, registry, &owner);
    }

    #[test]
    fn original_argument_inventory_separates_incomplete_from_cached_phases_and_standalone() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let input = source_input(tcl_lexer::LexerConfig::default());
        let registry = input.borrowed_context_registry().commands();
        let owner = OwnedInvocationMetadataContext::for_source_input(Some(&input));
        let tokens = original_tokens("list $a([dict size DATA])", &input);
        assert!(
            argument_calls(
                &tokens,
                registry,
                ArgumentReadInput::for_owner(&owner, registry)
            )
            .is_none()
        );
        for phase in [
            SyntheticMarker::EvaluatedWrapper,
            SyntheticMarker::DictionaryScopeEntry,
            SyntheticMarker::IterationBindings(None),
            SyntheticMarker::CapturedCatchOutputs,
        ] {
            let mut cached = tokens.clone();
            cached.synthetic = Some(phase);
            assert!(
                argument_calls(&cached, registry, ArgumentReadInput::Supplied(None))
                    .unwrap()
                    .is_empty()
            );
        }
        let mut lossless = original_tokens("list {[opaque]}", &input);
        lossless.synthetic = Some(SyntheticMarker::EvaluatedArguments);
        lossless.source_binding = None;
        assert!(argument_calls(&lossless, registry, ArgumentReadInput::Supplied(None)).is_none());
        assert!(
            argument_calls(&lossless, registry, ArgumentReadInput::Standalone)
                .unwrap()
                .is_empty()
        );
        let deep = format!("list {}0{}", "[list ".repeat(12), "]".repeat(12));
        let deep = original_tokens(&deep, &input);
        assert!(
            argument_calls(
                &deep,
                registry,
                ArgumentReadInput::for_owner(&owner, registry)
            )
            .is_none()
        );
    }

    #[test]
    fn original_cfg_argument_phase_retains_owner_and_missing_input_clobbers_all_consumers() {
        // naming.variable.invocation-caller-ssa-frame
        // docs/design/analysis/name-resolution-proofs/variable-invocation-caller-ssa-frame.md
        // Genuine CFG argv producer; unknown liveness is Quoted, never a definite read.
        let config = tcl_lexer::LexerConfig::from_grammar(
            tcl_dialect::DialectProfile::find("tcl9.1").unwrap().grammar,
        );
        let input = native_source_input(config);
        let registry = input.borrowed_context_registry().commands();
        let profile = input.unit_profile();
        let (_native_owner, captured) =
            crate::environment_ingress::captured_native_entry_with_owner(profile);
        let entry = crate::command_binding::SourceAnalysisEntry {
            native_entry: Some(Arc::new(captured)),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        };
        let unit = supplied_unit(
            "set retained LIVE; foreach x {A} {puts $x}",
            &input,
            Some(&entry),
        );
        let function = &unit.top_level;
        let (&block, index, statement) = function
            .cfg
            .blocks
            .iter()
            .find_map(|(block, body)| {
                body.statements
                    .iter()
                    .enumerate()
                    .find_map(|(index, statement)| {
                        statement
                            .tokens()
                            .is_some_and(|tokens| {
                                tokens.synthetic == Some(SyntheticMarker::EvaluatedArguments)
                            })
                            .then_some((block, index, statement))
                    })
            })
            .expect("actual foreach argv boundary");
        let points = function.ssa.point_contexts.as_ref().unwrap();
        assert_eq!(
            points.source_metadata_owner().source_analysis_input(),
            Some(&input)
        );
        let tokens = points.source_tokens_at(block, index).unwrap();
        assert!(
            argument_calls(
                tokens,
                registry,
                ArgumentReadInput::for_owner(points.source_metadata_owner(), registry)
            )
            .unwrap()
            .is_empty()
        );
        assert!(!crate::memory_ssa::is_clobber_with_metadata_context(
            statement,
            registry,
            points
                .source_metadata_owner()
                .metadata_context(registry)
                .and_then(crate::registry_invocation::InvocationMetadataSelection::context)
        ));
        let mut withdrawn = function.cfg.clone();
        withdrawn.metadata_context = OwnedInvocationMetadataContext::Unavailable;
        let points = crate::variable_bindings::build_point_resolve_contexts(
            &withdrawn,
            &withdrawn.name,
            registry,
        );
        assert!(points.source_arguments_unknown_at(block, index, registry));
        assert!(
            super::super::read_places_at(statement, block, index, &points, registry)
                .iter()
                .any(|place| place.kind == crate::place::PlaceKind::Unknown)
        );
        let ssa = crate::ssa::build_ssa_with_config(&withdrawn, registry, config);
        assert!(ssa.has_opaque_native_accesses());
        let scalar = &ssa.blocks[&block].statements[index];
        let retained = ssa
            .var_symbol("retained")
            .expect("original reaching variable");
        assert!(scalar.uses.contains_key(&retained));
        assert!(scalar.quoted_uses.contains(&retained));
        assert!(
            crate::memory_ssa::build_memory_ssa_with_metadata_context(&ssa, registry, None)
                .has_wildcard_aliasing
        );
        assert!(crate::memory_ssa::is_clobber_with_metadata_context(
            statement, registry, None
        ));
        let cells =
            crate::cell_state_ssa::build_cell_state_ssa(&withdrawn, &points, registry).unwrap();
        assert!(cells.operations().iter().any(|operation| matches!(
            operation,
            crate::state_ssa::StateOp::Clobber(_)
        ) && operation.location().kind
            == crate::place::PlaceKind::Unknown));
    }
    #[test]
    fn original_return_argument_source_is_retained_while_unpositioned_payload_stays_unknown() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let config = tcl_lexer::LexerConfig::from_grammar(
            tcl_dialect::DialectProfile::find("tcl9.1").unwrap().grammar,
        );
        let input = native_source_input(config);
        let registry = input.borrowed_context_registry().commands();
        let profile = input.unit_profile();
        let (_native_owner, captured) =
            crate::environment_ingress::captured_native_entry_with_owner(profile);
        let entry = crate::command_binding::SourceAnalysisEntry {
            native_entry: Some(Arc::new(captured)),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        };
        let unit = supplied_unit("proc p {} {return [list VALUE]}; p", &input, Some(&entry));
        let function = unit.function("::p").expect("genuine procedure body");
        let (block, tokens) = function
            .cfg
            .blocks
            .iter()
            .find_map(|(&block, body)| match &body.terminator {
                Some(crate::cfg::Terminator::Return {
                    tokens: Some(tokens),
                    value: Some(_),
                    ..
                }) => Some((block, tokens)),
                _ => None,
            })
            .expect("retained original return payload");
        let points = function.ssa.point_contexts.as_ref().unwrap();
        assert_eq!(
            points.source_tokens_at(block, usize::MAX),
            Some(tokens.as_ref())
        );
        let calls = argument_calls(
            tokens,
            registry,
            ArgumentReadInput::for_owner(points.source_metadata_owner(), registry),
        )
        .unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].command, "list");
        let mut missing = CommandTokens::marker(SyntheticMarker::EvaluatedArguments);
        missing.argv_texts = tokens.argv_texts.clone();
        missing.argv = tokens.argv.clone();
        missing.word_exprs = tokens.word_exprs.clone();
        assert!(
            argument_calls(
                &missing,
                registry,
                ArgumentReadInput::for_owner(points.source_metadata_owner(), registry)
            )
            .is_none()
        );
    }
}
