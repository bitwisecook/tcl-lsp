// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional cross-document procedure roles with original formal ownership.

use super::{OriginalOperandSource, effective_operand_sources};
use crate::analyser::types::LogicalSourceProcedureDeclaration;
use crate::analyser::{AnalysisResult, ProcDef, ResolvedAnalysisInput};
use crate::registry_invocation::EffectiveInvocationWord;
use crate::signature_scan::formal_parameters::SignatureSourceFormalParameters;
use crate::signature_scan::original_name::SourceDeclarationMetadata;
use crate::signature_scan::scope::SignatureSourceCommand;
use tcl_lexer::{LexerConfig, SourceImage};

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProcedureHeader {
    name: SignatureSourceCommand,
    logical_slot: Option<tcl_core_types::ByteCommandSlot>,
    metadata: Option<SourceDeclarationMetadata<ProcDef>>,
    logical: Option<LogicalSourceProcedureDeclaration>,
    native_formals: Option<SignatureSourceFormalParameters>,
}

/// Genuine source declarations and their original formal topology. Class rows
/// retain blockers. No project loading, dispatch or entered callee follows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalProjectProcedureRoles {
    input: ResolvedAnalysisInput,
    image: SourceImage,
    realm: std::sync::Arc<crate::realm::CommandBindingRealm>,
    headers: Vec<ProcedureHeader>,
}

impl OriginalProjectProcedureRoles {
    /// Capture from the declaring analysis's actual immutable source and input.
    /// Missing formal ownership retains a blocking declaration, never roles.
    #[must_use]
    pub fn capture(analysis: &AnalysisResult) -> Option<Self> {
        let input = analysis.resolved_input.as_ref()?;
        let realm = analysis.retained_command_realm()?;
        let image = realm.original_source_image()?;
        if !analysis.matches_original_source_image(image, input.lexer_config())
            || !realm.matches_resolved_analysis_input(input)
            || !analysis.command_aliases.is_empty()
        {
            return None;
        }
        let registry = analysis.resolved_registry()?;
        let mut headers = Vec::new();
        for record in analysis.original_procedure_declarations() {
            if record.name_input().source_image() != image
                || record.name_input().lexer_config() != input.lexer_config()
            {
                return None;
            }
            let logical = analysis
                .logical_source_procedure_declaration(record.metadata())
                .cloned();
            let logical_slot = if input.has_logical_source_name_context() {
                Some(logical.as_ref()?.source_candidate_coordinate()?)
            } else {
                None
            };
            headers.push(ProcedureHeader {
                name: record.name().clone(),
                logical_slot,
                metadata: Some(record.clone()),
                logical,
                native_formals: analysis.original_procedure_formals(record, registry),
            });
        }
        for record in analysis.original_class_declarations() {
            let logical_slot = if input.has_logical_source_name_context() {
                Some(
                    analysis
                        .logical_source_class_declaration(record.metadata())?
                        .source_candidate_coordinate()?
                        .clone(),
                )
            } else {
                None
            };
            headers.push(ProcedureHeader {
                name: record.name().clone(),
                logical_slot,
                metadata: None,
                logical: None,
                native_formals: None,
            });
        }
        Some(Self {
            input: input.clone(),
            image: image.clone(),
            realm: std::sync::Arc::clone(analysis.retained_command_realm_arc()?),
            headers,
        })
    }

    /// The declaring input and immutable interpretation remain independent of
    /// the caller. Distinct packs/availability may infer different source roles;
    /// the caller's own lookup still selects policy and source applicability.
    fn matches_declaring_owner(&self) -> bool {
        self.realm.matches_resolved_analysis_input(&self.input)
            && self
                .realm
                .matches_original_source_image(&self.image, self.input.lexer_config())
            && self.headers.iter().all(|header| {
                header.metadata.as_ref().is_none_or(|record| {
                    record.name_input().source_image() == &self.image
                        && record.name_input().lexer_config() == self.input.lexer_config()
                }) && header
                    .logical
                    .as_ref()
                    .is_none_or(|logical| logical.matches_source(&self.image, &self.input))
                    && header.native_formals.as_ref().is_none_or(|formals| {
                        formals.matches_source(&self.image, self.input.lexer_config())
                    })
            })
    }

    /// Whether any genuine procedure carries inferred source-display traits.
    #[must_use]
    pub fn has_roles(&self) -> bool {
        self.headers.iter().any(|header| {
            header.metadata.as_ref().is_some_and(|row| {
                use crate::analyser::types::ProcArgTrait;
                row.metadata().param_traits.values().any(|traits| {
                    traits.iter().any(|kind| {
                        matches!(
                            kind,
                            ProcArgTrait::VarWrite
                                | ProcArgTrait::VarRead
                                | ProcArgTrait::DynamicNameLocal
                                | ProcArgTrait::Command
                        )
                    })
                })
            })
        })
    }
}

/// One original call and uniquely selected foreign declaration/formal owner.
/// These source-display ordinals supply no Native command, activation or cell.
pub struct OriginalProjectProcedureArguments<'a> {
    declaration: &'a SourceDeclarationMetadata<ProcDef>,
    bindings: Vec<tcl_syntax::formal_params::FormalByteArgumentBinding>,
    arguments: Vec<EffectiveInvocationWord>,
    operands: Vec<Option<OriginalOperandSource>>,
    input: ResolvedAnalysisInput,
    image: SourceImage,
    config: LexerConfig,
}

impl OriginalProjectProcedureArguments<'_> {
    #[must_use]
    pub fn declaration(&self) -> &SourceDeclarationMetadata<ProcDef> {
        self.declaration
    }
    #[must_use]
    pub fn bindings(&self) -> &[tcl_syntax::formal_params::FormalByteArgumentBinding] {
        &self.bindings
    }
    #[must_use]
    pub fn arguments(&self) -> &[EffectiveInvocationWord] {
        &self.arguments
    }
    #[must_use]
    pub fn operands(&self) -> &[Option<OriginalOperandSource>] {
        &self.operands
    }
    /// Exact current caller interpretation and immutable source.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, input: &ResolvedAnalysisInput) -> bool {
        image == &self.image && input == &self.input && input.lexer_config() == self.config
    }
}

/// Select a foreign source header through genuine original occurrence occupancy,
/// then bind only its retained formal topology to the unchanged written argv.
#[must_use]
pub fn original_project_procedure_arguments<'a>(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    providers: &'a [OriginalProjectProcedureRoles],
) -> Option<OriginalProjectProcedureArguments<'a>> {
    let input = analysis.resolved_input.as_ref()?;
    let config = analysis.body_lexer_config?;
    let image = SourceImage::document(source);
    if config != input.lexer_config()
        || !analysis.matches_original_source_image(&image, config)
        || segment.expand_word.iter().any(|expanded| *expanded)
    {
        return None;
    }
    let realm = analysis.retained_command_realm()?;
    realm.matches_resolved_analysis_input(input).then_some(())?;
    analysis.resolved_registry()?;
    let offset = segment.argv.first()?.span.start();
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let binding = tokens.source_binding.as_ref()?;
    let projection = binding.invocation_site()?.source.original_lexical_command(
        tokens.words(),
        offset,
        config,
    )?;
    let actual = projection.command();
    if actual.span != segment.span
        || actual.argv != segment.argv
        || actual.texts != segment.texts
        || actual.word_fragments != segment.word_fragments
        || actual.single_token_word != segment.single_token_word
        || actual.expand_word != segment.expand_word
        || actual.is_partial != segment.is_partial
        || actual.partial_delimiter != segment.partial_delimiter
    {
        return None;
    }
    if projection.native_words().iter().any(|word| {
        word.executable_parts()
            .all_parts()
            .any(|part| !matches!(part.part, tcl_lexer::ExecutablePart::Text(_)))
    }) {
        return None;
    }
    let selected = select_project_procedure_header(
        (source, analysis, offset),
        &tokens,
        projection.native_words(),
        providers,
    )?;
    let declaration = selected.metadata.as_ref()?;
    let effective = crate::registry_invocation::effective_command_words(&tokens)?;
    let arguments = crate::registry_invocation::frozen_argument_words(&tokens, &effective);
    let bindings = if input.has_logical_source_name_context() {
        selected
            .logical
            .as_ref()?
            .source_argument_bindings(arguments.len())?
    } else {
        let formals = selected.native_formals.as_ref()?;
        if formals
            .parameters()
            .iter()
            .enumerate()
            .any(|(index, parameter)| {
                formals.parameters()[..index]
                    .iter()
                    .any(|earlier| earlier.name == parameter.name)
            })
        {
            return None;
        }
        formals.bindings(arguments.len()).ok()?
    };
    let operands = effective_operand_sources(
        segment,
        &tokens,
        &effective,
        &arguments,
        Some(projection.native_words()),
    )?;
    Some(OriginalProjectProcedureArguments {
        declaration,
        bindings,
        arguments,
        operands,
        input: input.clone(),
        image,
        config,
    })
}

fn select_project_procedure_header<'a>(
    call: (&str, &AnalysisResult, u32),
    tokens: &crate::ir::CommandTokens,
    native: &[tcl_lexer::NativeWord],
    providers: &'a [OriginalProjectProcedureRoles],
) -> Option<&'a ProcedureHeader> {
    let (source, analysis, offset) = call;
    let input = analysis.resolved_input.as_ref()?;
    let registry = analysis.resolved_registry()?;
    let binding = tokens.source_binding.as_ref()?;
    if providers
        .iter()
        .any(|provider| !provider.matches_declaring_owner())
    {
        return None;
    }
    let headers = providers
        .iter()
        .filter(|provider| provider.image != SourceImage::document(source))
        .flat_map(|provider| &provider.headers)
        .collect::<Vec<_>>();
    let selected = if input.has_logical_source_name_context() {
        let candidates = super::source_logical_command_candidates_at(source, analysis, offset)?;
        if candidates.has_earlier_unknown_mutations()
            || candidates.has_lookup_operand_effects()
            || candidates.original_words() != native
        {
            return None;
        }
        let mut selected = None;
        for slot in candidates.external_header_coordinates()? {
            let matches = headers
                .iter()
                .copied()
                .filter(|header| header.logical_slot.as_ref() == Some(slot))
                .collect::<Vec<_>>();
            if !matches.is_empty() {
                selected = Some(matches);
                break;
            }
        }
        selected?
    } else {
        let lookup = binding.original_static_head_lookup_for_tokens(tokens)?;
        lookup.matching_external_source_advice_publications(
            registry,
            headers.iter().map(|header| (&header.name, *header)),
        )?
    };
    let [selected] = selected.as_slice() else {
        return None;
    };
    Some(*selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;

    fn last_command(source: &str, analysis: &AnalysisResult) -> crate::segmenter::SegmentedCommand {
        crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            analysis.body_lexer_config.unwrap(),
        )
        .pop()
        .unwrap()
    }

    #[test]
    fn project_procedure_arguments_refuse_mismatched_or_stale_declaring_owner() {
        for dialect in ["tcl", "tcl8.6", "jim"] {
            let library = Analyser::new().analyse(
                "proc reader {name} {upvar 1 $name cell; set cell 1}",
                dialect,
            );
            let source = "reader retained";
            let caller = Analyser::new().analyse(source, dialect);
            let command = last_command(source, &caller);
            let provider = OriginalProjectProcedureRoles::capture(&library).unwrap();
            assert!(
                original_project_procedure_arguments(
                    source,
                    &caller,
                    &command,
                    std::slice::from_ref(&provider)
                )
                .is_some()
            );
            let mut wrong_input = provider.clone();
            let other = if dialect == "tcl8.6" {
                "tcl9.0"
            } else {
                "tcl8.6"
            };
            wrong_input.input = Analyser::new().analyse("", other).resolved_input.unwrap();
            assert!(
                original_project_procedure_arguments(source, &caller, &command, &[wrong_input])
                    .is_none(),
                "{dialect}: independent input mismatch"
            );
            let mut stale_image = provider;
            stale_image.image = SourceImage::document("proc reader {other} {};");
            assert!(
                original_project_procedure_arguments(source, &caller, &command, &[stale_image])
                    .is_none(),
                "{dialect}: stale declaring image"
            );
        }
    }

    #[test]
    fn project_procedure_arguments_keep_declaring_formals_and_actual_call_geometry() {
        for dialect in ["tcl", "tcl8.6", "jim"] {
            let library = Analyser::new().analyse(
                "proc reader {name} {upvar 1 $name cell; set cell 1}",
                dialect,
            );
            let providers = vec![OriginalProjectProcedureRoles::capture(&library).unwrap()];
            let source = "reader retained";
            let mut caller = Analyser::new().analyse(source, dialect);
            let command = last_command(source, &caller);
            let roles = original_project_procedure_arguments(source, &caller, &command, &providers)
                .expect(dialect);
            assert_eq!(roles.declaration().metadata().params[0].name, "name");
            assert_eq!(
                roles.bindings(),
                &[
                    tcl_syntax::formal_params::FormalByteArgumentBinding::Value {
                        parameter: 0,
                        argument: 0
                    }
                ]
            );
            assert!(roles.matches_source(
                &SourceImage::document(source),
                caller.resolved_input.as_ref().unwrap()
            ));
            let mut changed = command.clone();
            changed.texts[1] = "forged".to_owned();
            assert!(
                original_project_procedure_arguments(source, &caller, &changed, &providers)
                    .is_none()
            );
            assert!(
                original_project_procedure_arguments(
                    &(source.to_owned() + " "),
                    &caller,
                    &command,
                    &providers
                )
                .is_none()
            );
            caller.resolved_input = None;
            assert!(
                original_project_procedure_arguments(source, &caller, &command, &providers)
                    .is_none()
            );
        }
    }

    #[test]
    fn project_procedure_arguments_preserve_stronger_occupants_and_barriers() {
        for dialect in ["tcl", "tcl8.6", "jim"] {
            let library = Analyser::new().analyse("proc reader {name} {upvar 1 $name cell; set cell 1}; proc puts {name} {upvar 1 $name cell; set cell 1}", dialect);
            let providers = vec![OriginalProjectProcedureRoles::capture(&library).unwrap()];
            for source in [
                "puts retained",
                "proc reader {value} {}; reader retained",
                "interp alias {} reader {} puts; reader retained",
                "proc reader {value} {}; rename reader {}; reader retained",
                "unknown_mutation; reader retained",
                "reader [unknown_operand]",
                "reader {*}{retained}",
                "reader ONE TWO",
            ] {
                let caller = Analyser::new().analyse(source, dialect);
                let command = last_command(source, &caller);
                assert!(
                    original_project_procedure_arguments(source, &caller, &command, &providers)
                        .is_none(),
                    "{dialect}: {source}"
                );
            }
        }
    }

    #[test]
    fn project_procedure_arguments_keep_priority_ambiguity_and_class_blockers() {
        for dialect in ["tcl", "tcl8.6"] {
            let library = Analyser::new().analyse("namespace eval N {proc reader {name} {upvar 1 $name cell; set cell 1}}; proc reader {name} {upvar 1 $name cell; set cell 1}", dialect);
            let providers = vec![OriginalProjectProcedureRoles::capture(&library).unwrap()];
            let source = "namespace eval N {proc reader {value} {}; rename reader {}; proc caller {} {reader retained}}";
            let caller = Analyser::new().analyse(source, dialect);
            let offset = u32::try_from(source.rfind("reader retained").unwrap()).unwrap();
            let command = crate::segmenter::segment_commands_with_offset_and_config(
                &source[offset as usize..source.len() - 2],
                offset,
                caller.body_lexer_config.unwrap().nested(),
            )
            .remove(0);
            let selected =
                original_project_procedure_arguments(source, &caller, &command, &providers)
                    .expect("genuine root fallback after local deletion");
            assert!(selected.declaration().name().slot().namespace.is_root());
            let caller_source = "reader retained";
            let caller = Analyser::new().analyse(caller_source, dialect);
            let command = last_command(caller_source, &caller);
            let duplicate = vec![providers[0].clone(), providers[0].clone()];
            assert!(
                original_project_procedure_arguments(caller_source, &caller, &command, &duplicate)
                    .is_none()
            );
        }
    }
}
