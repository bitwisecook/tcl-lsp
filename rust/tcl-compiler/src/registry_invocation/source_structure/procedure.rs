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
