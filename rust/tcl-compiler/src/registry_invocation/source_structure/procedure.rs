// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original selected procedure signatures before successful argc binding.

use super::{OriginalOperandSource, effective_operand_sources};
use crate::analyser::{AnalysisResult, ProcDef, ResolvedAnalysisInput};
use crate::command_binding::SourceCommandReference;
use crate::registry_invocation::EffectiveInvocationWord;
use crate::signature_scan::formal_parameters::SignatureSourceFormalParameters;
use crate::signature_scan::original_name::SourceDeclarationMetadata;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage};
use tcl_registry::{CommandRegistry, RegistrySemanticKey};

/// Authentic selected procedure declaration and complete original call.
/// Formal syntax is retained before argc binding: this grants no callee entry,
/// values, local cells, successful call, Normal result or edit permission.
pub struct OriginalProcedureSourceDescriptor<'a> {
    pub(super) declaration: &'a SourceDeclarationMetadata<ProcDef>,
    pub(super) formals: SignatureSourceFormalParameters,
    reference: SourceCommandReference,
    original: Vec<NativeWord>,
    input: ResolvedAnalysisInput,
    pub(super) arguments: Vec<EffectiveInvocationWord>,
    pub(super) operands: Vec<Option<OriginalOperandSource>>,
    pub(super) image: SourceImage,
    pub(super) config: LexerConfig,
    pub(super) registry: RegistrySemanticKey,
}
impl<'a> OriginalProcedureSourceDescriptor<'a> {
    /// Exact immutable declaration joined to the current implementation allocation.
    #[must_use]
    pub const fn declaration(&self) -> &'a SourceDeclarationMetadata<ProcDef> {
        self.declaration
    }
    /// Current original reference, independently of eventual argc success.
    #[must_use]
    pub const fn reference(&self) -> &SourceCommandReference {
        &self.reference
    }
    /// Original parsed parameter-list and its independently retained grammar.
    #[must_use]
    pub const fn formals(&self) -> &SignatureSourceFormalParameters {
        &self.formals
    }
    /// Complete original written invocation, including the actual head.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.original
    }
    /// Actual complete interpretation and Registry availability context.
    #[must_use]
    pub const fn resolved_input(&self) -> &ResolvedAnalysisInput {
        &self.input
    }
    /// Effective post-head operands, retaining captured and expanded origins.
    #[must_use]
    pub fn arguments(&self) -> &[EffectiveInvocationWord] {
        &self.arguments
    }
    /// Original written anchors; captured prefix values never borrow call spans.
    #[must_use]
    pub fn operands(&self) -> &[Option<OriginalOperandSource>] {
        &self.operands
    }
    /// Same complete immutable source/channel and full lexer configuration.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        self.image == *image && self.config == config && self.formals.matches_source(image, config)
    }
    /// Same Registry semantic contents used by the retained formal owner.
    #[must_use]
    pub fn matches_registry(&self, registry: &CommandRegistry) -> bool {
        self.registry == registry.snapshot().semantic_key()
    }
}

/// Select the genuine current procedure allocation and original formals before
/// argument binding. Replacements, foreign inputs, lost projection correspondence
/// and unknown lookup remain terminal; failing argc cannot fabricate a schema.
#[must_use]
pub fn original_procedure_source_descriptor<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
) -> Option<OriginalProcedureSourceDescriptor<'a>> {
    // naming.source.original-procedure-argument-topology
    // docs/design/analysis/name-resolution-proofs/original-procedure-argument-topology.md
    let config = analysis.body_lexer_config?;
    let input = analysis.resolved_input.as_ref()?;
    (input.lexer_config() == config).then_some(())?;
    let image = tcl_lexer::SourceImage::document(source);
    analysis
        .matches_original_source_image(&image, config)
        .then_some(())?;
    let registry = analysis.resolved_registry()?;
    let realm = analysis.retained_command_realm()?;
    realm.matches_resolved_analysis_input(input).then_some(())?;
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let binding = tokens.source_binding.as_ref()?;
    let head = binding.original_head_name_input(&tokens)?;
    let lookup = binding.original_command_lookup(&tokens, &head)?;
    let reference = binding.original_command_reference(&tokens)?;
    if lookup.name_input() != &head
        || lookup.site().source.source_image() != &image
        || !reference.matches_original_invocation_site(lookup.site())
    {
        return None;
    }
    // A present analysis projection cannot lose its original correspondence
    // and silently borrow the realm's other receipt. Reporting rows may be
    // absent entirely; the retained binding then remains the genuine owner.
    for invocation in analysis
        .command_invocations
        .iter()
        .filter(|invocation| invocation.range.start() == lookup.site().offset)
    {
        let projected_input = invocation.original_name_input.as_ref()?;
        let projected_lookup = invocation.original_lookup.as_ref()?;
        let projected_reference = invocation.resolved_command_reference.as_ref()?;
        if projected_input != &head
            || projected_lookup.name_input() != projected_input
            || projected_lookup.site() != lookup.site()
            || projected_reference != &reference
        {
            return None;
        }
    }
    let definition = reference
        .linked_definition()
        .or_else(|| reference.definition())?;
    if definition.kind() != crate::command_binding::SourceCommandDefinitionKind::Procedure {
        return None;
    }
    let mut declarations = analysis
        .original_procedure_declarations()
        .filter(|row| row.declaration_site() == &definition.allocation().site);
    let declaration = declarations.next()?;
    if declarations.any(|other| other != declaration)
        || declaration.name().policy() != head.policy()
    {
        return None;
    }
    let formals = analysis.original_procedure_formals(declaration, registry)?;
    formals.matches_source(&image, config).then_some(())?;
    let effective = crate::registry_invocation::effective_command_words(&tokens)?;
    let arguments = crate::registry_invocation::frozen_argument_words(&tokens, &effective);
    let native = crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        segment.argv.first()?.span.start(),
        config,
    )?;
    let operands =
        effective_operand_sources(segment, &tokens, &effective, &arguments, Some(&native))?;
    Some(OriginalProcedureSourceDescriptor {
        declaration,
        reference,
        original: native,
        input: input.clone(),
        formals,
        arguments,
        operands,
        image,
        config,
        registry: registry.snapshot().semantic_key(),
    })
}

/// Conditional Logical procedure-role geometry at one original call. Agreement
/// supplies source-display ordinals, never dispatch, activation or edit authority.
pub struct OriginalLogicalProcedureArguments<'a> {
    declaration: &'a crate::analyser::types::LogicalSourceProcedureDeclaration,
    candidates: crate::command_binding::OriginalLogicalCommandCandidates,
    target: crate::command_binding::SourceCommandTarget,
    bindings: Vec<tcl_syntax::formal_params::FormalByteArgumentBinding>,
    arguments: Vec<EffectiveInvocationWord>,
    origins: Vec<crate::registry_invocation::InvocationWordOrigin>,
    operands: Vec<Option<OriginalOperandSource>>,
    input: ResolvedAnalysisInput,
    image: SourceImage,
    config: LexerConfig,
    registry: RegistrySemanticKey,
}

impl<'a> OriginalLogicalProcedureArguments<'a> {
    /// Current original header metadata selected by its actual allocation card.
    #[must_use]
    pub const fn declaration(
        &self,
    ) -> &'a crate::analyser::types::LogicalSourceProcedureDeclaration {
        self.declaration
    }
    /// Conditional argv ordinals on which both formal grammars agree.
    #[must_use]
    pub fn bindings(&self) -> &[tcl_syntax::formal_params::FormalByteArgumentBinding] {
        &self.bindings
    }
    /// Original written and separately captured prefix values.
    #[must_use]
    pub fn arguments(&self) -> &[EffectiveInvocationWord] {
        &self.arguments
    }
    /// Original written positions and separately retained captured-prefix slots.
    #[must_use]
    pub fn origins(&self) -> &[crate::registry_invocation::InvocationWordOrigin] {
        &self.origins
    }
    /// Captured prefix operands have no borrowed call-site source anchor.
    #[must_use]
    pub fn operands(&self) -> &[Option<OriginalOperandSource>] {
        &self.operands
    }
    /// Complete original invocation, including the original selector.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        self.candidates.original_words()
    }
    /// Actual source interpretation, without a Native execution grant.
    #[must_use]
    pub const fn resolved_input(&self) -> &ResolvedAnalysisInput {
        &self.input
    }
    /// Current source and full configuration of both call and selected header.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        self.image == *image
            && self.config == config
            && self.candidates.matches_source(image, &self.input)
            && self.declaration.matches_source(image, &self.input)
            && self.target.implementation_allocation.is_some()
    }
    /// Registry semantic contents retained with the source-role interpretation.
    #[must_use]
    pub fn matches_registry(&self, registry: &CommandRegistry) -> bool {
        self.registry == registry.snapshot().semantic_key()
    }
}

fn original_procedure_segment_geometry_matches(
    actual: &crate::segmenter::SegmentedCommand,
    segment: &crate::segmenter::SegmentedCommand,
) -> bool {
    actual.span == segment.span
        && actual.argv == segment.argv
        && actual.texts == segment.texts
        && actual.word_fragments == segment.word_fragments
        && actual.single_token_word == segment.single_token_word
        && actual.expand_word == segment.expand_word
        && actual.is_partial == segment.is_partial
        && actual.partial_delimiter == segment.partial_delimiter
}

/// Join an authentic Logical occurrence to its selected source allocation and
/// same current declaration card. Missing selection, effects, expansion or formal
/// disagreement withholds source roles without changing Native lookup purposes.
#[must_use]
pub fn original_logical_procedure_arguments<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
) -> Option<OriginalLogicalProcedureArguments<'a>> {
    let config = analysis.body_lexer_config?;
    let input = analysis.resolved_input.as_ref()?;
    (input.has_logical_source_name_context() && input.lexer_config() == config).then_some(())?;
    let image = SourceImage::document(source);
    analysis
        .matches_original_source_image(&image, config)
        .then_some(())?;
    let registry = analysis.resolved_registry()?;
    let realm = analysis.retained_command_realm()?;
    realm.matches_resolved_analysis_input(input).then_some(())?;
    let offset = segment.argv.first()?.span.start();
    let candidates = super::source_logical_command_candidates_at(source, analysis, offset)?;
    if candidates.has_earlier_unknown_mutations()
        || candidates.has_lookup_operand_effects()
        || !candidates.matches_source(&image, input)
    {
        return None;
    }
    candidates.argument_count()?;
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
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
    if !original_procedure_segment_geometry_matches(actual, segment) {
        return None;
    }
    let native = projection.native_words();
    (native == candidates.original_words()).then_some(())?;
    let roles = binding.original_source_procedure_argument_roles(&tokens, input, registry)?;
    let target = roles.target()?.clone();
    let allocation = target.implementation_allocation.as_ref()?;
    let cards = realm.original_logical_procedure_body_cards(input)?;
    let mut selected = cards.iter().filter(|card| card.site() == &allocation.site);
    let body = selected.next()?;
    if selected.next().is_some()
        || !body.matches_source(&image, input)
        || body.has_earlier_unknown_mutations()
    {
        return None;
    }
    let declaration = analysis.logical_source_procedure_declaration_for_body(body)?;
    if !target.prepended.is_empty() {
        crate::registry_invocation::original_logical_prefix_for_tokens(&tokens, &target, input)?;
    }
    let effective = crate::registry_invocation::compose_original_effective_words(
        &tokens,
        &target.command,
        &target.prepended,
    )?;
    let arguments = effective
        .origins
        .iter()
        .skip(1)
        .map(|origin| match origin {
            crate::registry_invocation::InvocationWordOrigin::Written(written) => binding
                .original_written_name_input(&tokens, *written)
                .map_or(EffectiveInvocationWord::Dynamic, |input| {
                    EffectiveInvocationWord::from_bytes(input.bytes())
                }),
            crate::registry_invocation::InvocationWordOrigin::BindingPrefix(index) => target
                .prepended
                .get(*index)
                .cloned()
                .unwrap_or(EffectiveInvocationWord::Dynamic),
            _ => EffectiveInvocationWord::Dynamic,
        })
        .collect::<Vec<_>>();
    let operands =
        effective_operand_sources(segment, &tokens, &effective, &arguments, Some(native))?;
    let bindings = crate::signature_scan::formal_count::logical_formal_argument_bindings(
        body.parameters_text(),
        body.parameters(),
        arguments.len(),
    )?;
    Some(OriginalLogicalProcedureArguments {
        declaration,
        candidates,
        target,
        bindings,
        arguments,
        origins: effective.origins.into_iter().skip(1).collect(),
        operands,
        input: input.clone(),
        image,
        config,
        registry: registry.snapshot().semantic_key(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;
    use crate::registry_invocation::source_structure::original_procedure_arguments;

    fn commands(
        source: &str,
        analysis: &AnalysisResult,
    ) -> Vec<crate::segmenter::SegmentedCommand> {
        crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            analysis.body_lexer_config.unwrap(),
        )
    }

    #[test]
    fn logical_procedure_roles_join_current_card_and_complete_original_argv() {
        let source = "proc café {value} {}; café {literal α}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let commands = commands(source, &analysis);
        let call = commands.last().unwrap();
        let roles = original_logical_procedure_arguments(source, &analysis, call)
            .expect("actual Logical call, allocation and current header card");
        assert!(roles.resolved_input().has_logical_source_name_context());
        assert_eq!(roles.declaration().metadata().params[0].name, "value");
        assert_eq!(
            roles.arguments()[0].literal_bytes(),
            Some("literal α".as_bytes())
        );
        assert_eq!(
            roles.bindings(),
            &[
                tcl_syntax::formal_params::FormalByteArgumentBinding::Value {
                    parameter: 0,
                    argument: 0,
                }
            ]
        );
        assert_eq!(
            roles.operands()[0].as_ref().unwrap().word(),
            roles.original_words().get(1)
        );
        assert!(roles.matches_source(
            &SourceImage::document(source),
            analysis.body_lexer_config.unwrap()
        ));
        assert!(roles.matches_registry(analysis.resolved_registry().unwrap()));
        assert!(original_procedure_arguments(source, &analysis, call).is_none());
        assert!(
            original_logical_procedure_arguments(
                &source.replace("literal α", "different"),
                &analysis,
                call,
            )
            .is_none()
        );
        for axis in 0..4 {
            let mut changed_call = call.clone();
            match axis {
                0 => changed_call.texts[0] = "other".into(),
                1 => changed_call.texts[1] = "other".into(),
                2 => changed_call.argv[1].span = changed_call.argv[0].span,
                3 => changed_call.word_fragments[1][0].token.span = changed_call.argv[0].span,
                _ => unreachable!(),
            }
            assert!(
                original_logical_procedure_arguments(source, &analysis, &changed_call).is_none()
            );
        }
        let mut reporting = analysis.clone();
        reporting.all_procs.get_mut("::café").unwrap().params[0].name = "display-only".into();
        let original = original_logical_procedure_arguments(source, &reporting, call)
            .expect("original card signature remains independent of reporting rows");
        assert_eq!(original.declaration().metadata().params[0].name, "value");
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(original_logical_procedure_arguments(source, &missing, call).is_none());
    }

    #[test]
    fn logical_procedure_roles_keep_captured_prefix_separate_from_written_anchors() {
        let source = "proc target {prefix value} {}; interp alias {} forwarded {} target PREFIX; forwarded LITERAL";
        let analysis = Analyser::new().analyse(source, "tcl");
        let commands = commands(source, &analysis);
        let roles =
            original_logical_procedure_arguments(source, &analysis, commands.last().unwrap())
                .expect("genuine captured prefix and selected Logical target");
        assert_eq!(roles.arguments().len(), 2);
        assert_eq!(
            roles.arguments()[0].literal_bytes(),
            Some(b"PREFIX".as_slice())
        );
        assert!(roles.operands()[0].is_none());
        assert_eq!(
            roles.operands()[1].as_ref().unwrap().span(),
            commands.last().unwrap().argv[1].span
        );
        assert_eq!(roles.bindings().len(), 2);
    }

    #[test]
    fn logical_procedure_roles_refuse_unknown_effects_shadowing_and_formal_disagreement() {
        for source in [
            "proc target {value} {}; unknown_mutation; target LITERAL",
            "proc target {value} {}; target [unknown_operand]",
            "proc target {value} {}; target {*}{LITERAL}",
            "proc target {value} {}; rename target {}; target LITERAL",
            "proc target {value} {}; interp alias {} target {} puts; target LITERAL",
            "proc target {{optional DEFAULT} required} {}; target ONLY",
            "proc target {&value} {}; target LITERAL",
            "proc target {value value} {}; target FIRST SECOND",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl");
            let commands = commands(source, &analysis);
            assert!(
                original_logical_procedure_arguments(source, &analysis, commands.last().unwrap())
                    .is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn original_procedure_descriptor_precedes_selected_dialect_argument_binding() {
        // naming.source.original-procedure-argument-topology
        // docs/design/analysis/name-resolution-proofs/original-procedure-argument-topology.md
        let source = "proc target {{optional DEFAULT} required} {}; target ONLY";
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let analysis = Analyser::new().analyse(source, profile);
            let commands = commands(source, &analysis);
            let call = commands.last().unwrap();
            let descriptor =
                original_procedure_source_descriptor(source, &analysis, call).expect(profile);
            assert_eq!(descriptor.arguments().len(), 1);
            assert_eq!(descriptor.formals().parameters().len(), 2);
            assert_eq!(descriptor.formals().bindings(1).is_ok(), profile == "jim");
            assert_eq!(
                original_procedure_arguments(source, &analysis, call).is_some(),
                profile == "jim"
            );
            assert_eq!(descriptor.original_words()[0].span(), call.argv[0].span);
            assert_eq!(
                descriptor.operands()[0].as_ref().unwrap().word(),
                descriptor.original_words().get(1)
            );
            assert!(descriptor.matches_registry(analysis.resolved_registry().unwrap()));
            assert!(descriptor.matches_source(
                &SourceImage::document(source),
                analysis.body_lexer_config.unwrap()
            ));
            assert_eq!(
                descriptor.resolved_input(),
                analysis.resolved_input.as_ref().unwrap()
            );
        }
    }

    #[test]
    fn original_procedure_descriptor_keeps_invalid_count_and_unknown_lookup_separate() {
        // naming.source.original-procedure-argument-topology
        // docs/design/analysis/name-resolution-proofs/original-procedure-argument-topology.md
        let source = "proc target {value} {}; target ONE TWO";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let commands = commands(source, &analysis);
        let call = commands.last().unwrap();
        let descriptor = original_procedure_source_descriptor(source, &analysis, call).unwrap();
        assert_eq!(descriptor.arguments().len(), 2);
        assert!(descriptor.formals().bindings(2).is_err());
        assert!(original_procedure_arguments(source, &analysis, call).is_none());
        assert!(
            original_procedure_source_descriptor(&(source.to_owned() + " "), &analysis, call)
                .is_none()
        );
        analysis
            .command_invocations
            .iter_mut()
            .find(|row| row.range.start() == call.argv[0].span.start())
            .unwrap()
            .original_lookup = None;
        assert!(original_procedure_source_descriptor(source, &analysis, call).is_none());
        assert!(original_procedure_arguments(source, &analysis, call).is_none());
    }
}
