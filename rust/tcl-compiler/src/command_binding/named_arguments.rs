// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! One original-word projection for a compiler-selected private handler.

use super::{Arc, SourceExecutionContext};
use crate::registry_invocation::EffectiveInvocationWord;

pub(super) struct ProjectedNamedArguments {
    pub(super) words: Vec<crate::ir::WordExpr>,
    pub(super) effective: Vec<EffectiveInvocationWord>,
    original: Option<OriginalNamedWords>,
    arguments: Option<Vec<EffectiveInvocationWord>>,
    values: Option<Vec<Option<Arc<super::native_result::EvaluatedSourceValue>>>>,
    name_values: Option<Vec<Option<Arc<super::original_name_value::OriginalProducedNameValue>>>>,
    representations: Option<Vec<Option<super::source_representation::FrozenSourceRepresentation>>>,
    objects: Option<Vec<Option<Arc<super::SourceObjectInstanceProof>>>>,
    prefixes: Option<Vec<Option<Arc<super::SourceCapturedMethodPrefix>>>>,
    reads: Option<Vec<Option<Arc<super::argument_reads::FrozenSourceArgumentRead>>>>,
}

/// Exact source words inherited from a complete original invocation before
/// the selected private handler consumes ensemble selectors.
pub(super) struct OriginalNamedWords {
    words: Vec<crate::ir::WordExpr>,
    native: Vec<tcl_lexer::NativeWord>,
}

impl OriginalNamedWords {
    fn capture(
        image: &tcl_lexer::SourceImage,
        words: &[crate::ir::WordExpr],
        context: &SourceExecutionContext<'_>,
        consumed: usize,
    ) -> Option<Self> {
        let native = if let Some(original) = context.original_written_projection {
            original.words_for(image, words, context.config)?
        } else {
            return Self::from_complete(image, words, context.config, consumed);
        };
        Some(Self {
            words: project(words, consumed).ok()?,
            native: project(native, consumed).ok()?,
        })
    }

    fn from_complete(
        image: &tcl_lexer::SourceImage,
        words: &[crate::ir::WordExpr],
        config: tcl_lexer::LexerConfig,
        consumed: usize,
    ) -> Option<Self> {
        let native = crate::registry_invocation::original_native_compiler_words(
            image,
            words,
            words.first()?.source().span.start(),
            config,
        )?;
        Some(Self {
            words: project(words, consumed).ok()?,
            native: project(&native, consumed).ok()?,
        })
    }

    pub(super) fn words_for(
        &self,
        image: &tcl_lexer::SourceImage,
        words: &[crate::ir::WordExpr],
        config: tcl_lexer::LexerConfig,
    ) -> Option<&[tcl_lexer::NativeWord]> {
        (words == self.words.as_slice()
            && words.len() == self.native.len()
            && self
                .native
                .iter()
                .all(|word| word.image() == image && word.config() == config))
        .then_some(self.native.as_slice())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProjectionError {
    MissingHead,
    ConsumedWordsUnavailable,
    SidecarMisaligned,
    EffectiveWordsMisaligned,
}

fn project<T: Clone>(words: &[T], consumed: usize) -> Result<Vec<T>, ProjectionError> {
    let first = words.first().ok_or(ProjectionError::MissingHead)?;
    let start = consumed
        .checked_add(1)
        .ok_or(ProjectionError::ConsumedWordsUnavailable)?;
    let tail = words
        .get(start..)
        .ok_or(ProjectionError::ConsumedWordsUnavailable)?;
    Ok(std::iter::once(first.clone())
        .chain(tail.iter().cloned())
        .collect())
}

fn project_optional<T: Clone>(
    words: Option<&[T]>,
    count: usize,
    consumed: usize,
) -> Result<Option<Vec<T>>, ProjectionError> {
    match words {
        Some(words) if words.len() == count => Ok(Some(project(words, consumed)?)),
        Some(_) => Err(ProjectionError::SidecarMisaligned),
        None => Ok(None),
    }
}

impl ProjectedNamedArguments {
    /// Retain original sites and frozen receipts together. Selection consumes
    /// written words; known expansion is flattened only after that projection.
    #[inline(never)]
    pub(super) fn capture(
        words: &[crate::ir::WordExpr],
        effective: &[EffectiveInvocationWord],
        incoming: &super::ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        consumed: usize,
    ) -> Result<Box<Self>, ProjectionError> {
        let count = words.len();
        let arguments = project_optional(context.written_arguments, count, consumed)?;
        let effective = match &arguments {
            Some(arguments) => arguments
                .iter()
                .flat_map(super::frozen_arguments::runtime_words)
                .collect(),
            None if effective.len() == count => project(effective, consumed)?,
            None => return Err(ProjectionError::EffectiveWordsMisaligned),
        };
        Ok(Box::new(Self {
            words: project(words, consumed)?,
            effective,
            original: incoming.current_source_origin.as_ref().and_then(|origin| {
                OriginalNamedWords::capture(origin.source_image(), words, context, consumed)
            }),
            arguments,
            values: project_optional(context.written_values, count, consumed)?,
            name_values: project_optional(context.written_name_values, count, consumed)?,
            representations: project_optional(context.written_representations, count, consumed)?,
            objects: project_optional(context.written_objects, count, consumed)?,
            prefixes: project_optional(context.written_method_prefixes, count, consumed)?,
            reads: project_optional(context.written_variable_reads, count, consumed)?,
        }))
    }

    pub(super) fn context<'a>(
        &'a self,
        original: &SourceExecutionContext<'a>,
    ) -> Box<SourceExecutionContext<'a>> {
        Box::new(SourceExecutionContext {
            original_variable_compilation: None,
            selected_compilation: Some(
                &tcl_registry::native_compilation::NativeCompilationSelection::Generic,
            ),
            original_written_projection: self.original.as_ref(),
            written_arguments: self.arguments.as_deref(),
            written_values: self.values.as_deref(),
            written_name_values: self.name_values.as_deref(),
            written_representations: self.representations.as_deref(),
            written_objects: self.objects.as_deref(),
            written_method_prefixes: self.prefixes.as_deref(),
            written_variable_reads: self.reads.as_deref(),
            ..*original
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_projection_distinguishes_absence_alignment_and_rejection() {
        assert_eq!(project_optional::<u8>(None, 3, 1), Ok(None));
        assert_eq!(
            project_optional(Some(&[1, 2, 3][..]), 3, 1),
            Ok(Some(vec![1, 3]))
        );
        assert_eq!(
            project_optional(Some(&[1, 2][..]), 3, 1),
            Err(ProjectionError::SidecarMisaligned)
        );
        assert_eq!(project(&[1, 2][..], 1), Ok(vec![1]));
        assert_eq!(
            project(&[1, 2][..], 2),
            Err(ProjectionError::ConsumedWordsUnavailable)
        );
        assert_eq!(project::<u8>(&[], 0), Err(ProjectionError::MissingHead));
    }

    #[test]
    fn original_named_projection_keeps_complete_source_ownership_and_selected_ordinals() {
        // Implementation contract: naming.compiler.original-named-operand-projection
        // docs/design/analysis/name-resolution-proofs/original-named-operand-projection.md
        let source = r"namespace export p\uD800";
        let image = tcl_lexer::SourceImage::document(source);
        let config = tcl_lexer::LexerConfig::default();
        let segments =
            crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)
                .unwrap();
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_image(&image),
            config,
            &segments[0],
        );
        let original = OriginalNamedWords::from_complete(&image, tokens.words(), config, 1)
            .expect("complete original vector projects one consumed selector");
        let projected = project(tokens.words(), 1).unwrap();
        assert!(
            crate::registry_invocation::original_native_compiler_words(
                &image, &projected, 0, config,
            )
            .is_none(),
            "projected argv must not masquerade as a complete source command"
        );
        let words = original.words_for(&image, &projected, config).unwrap();
        assert_eq!(words.len(), 2);
        assert_eq!(
            words[1].span().start(),
            tokens.words()[2].source().span.start()
        );
        assert_eq!(&image.bytes()[words[1].span().as_range()], br"p\uD800");
        assert!(original.words_for(&image, tokens.words(), config).is_none());
        assert!(
            original
                .words_for(
                    &tcl_lexer::SourceImage::document(&format!("{source} ")),
                    &projected,
                    config,
                )
                .is_none()
        );
        let mut changed = config;
        changed.strict_quoting = !changed.strict_quoting;
        assert!(original.words_for(&image, &projected, changed).is_none());
        assert!(
            original
                .words_for(
                    &tcl_lexer::SourceImage::native(image.bytes()),
                    &projected,
                    config,
                )
                .is_none()
        );
    }

    #[test]
    fn original_named_namespace_helper_retains_counted_operand_input() {
        // Implementation contract: naming.compiler.original-named-operand-projection
        // docs/design/analysis/name-resolution-proofs/original-named-operand-projection.md
        for name in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(name).unwrap();
            let generation = tcl_registry::model::ingress::context_for_profile(profile);
            let registry = generation.commands();
            let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
            let source = r"namespace export p\uD800";
            let bindings = super::super::SourceCommandBindings::analyse_with_options(
                source,
                config,
                registry,
                super::super::SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                        mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                        ..crate::environment_ingress::authoring_native_compilation()
                    },
                    ..Default::default()
                },
            );
            let binding = bindings.invocation_at_source("namespace", 0);
            let operands = binding
                .original_variable_operands
                .as_ref()
                .unwrap_or_else(|| panic!("{name}: original selected helper operands"));
            assert_eq!(
                operands.argument_count(),
                1,
                "{name}: consumed ensemble selector"
            );
            let input = operands
                .input(0, &binding.variable_context)
                .unwrap_or_else(|| panic!("{name}: counted original export pattern"));
            let native =
                crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                    &tcl_lexer::native_script_words_in(
                        tcl_lexer::SourceImage::document(source),
                        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                        config,
                    )
                    .unwrap()
                    .commands[0]
                        .words[2],
                    tcl_syntax::word_rules::WordValueRules::from_config(&config),
                    input.policy(),
                )
                .unwrap();
            assert_eq!(input.bytes(), native.bytes(), "{name}");
            assert!(
                bindings.original_completed_command_world().is_some(),
                "{name}: independently completed selected helper preserves the root table"
            );
        }
    }
}
