// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original parser-word geometry for native compiler descriptors.

use crate::ir::{CommandTokens, Provenance, WordExpr};
use tcl_lexer::word_parts::NativeWord;
use tcl_lexer::{LexerConfig, SourceImage, SourceMap, Span};
use tcl_registry::native_compilation::{
    NativeCompilationContext, NativeCompilationSelection, NativeCompilationSpec,
};
use tcl_registry::native_compiler_words::NativeCompilerWords;
use tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingCompilation;

#[derive(Clone, Copy)]
pub(crate) struct OriginalNativeCompilerInvocation<'a> {
    pub image: &'a SourceImage,
    pub words: &'a [WordExpr],
    pub offset: u32,
    pub config: LexerConfig,
    pub source_protocol: Option<tcl_syntax::native_string::NativeStringProtocol>,
    pub compiler_dialect: Option<tcl_registry::InvocationDialect>,
    pub context: NativeCompilationContext,
    pub operand_from: usize,
}

/// Purpose-specific preparation from authenticated original compiler words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OriginalNativeCompilerPreparation {
    NamespaceBindings(NativeNamespaceBindingCompilation),
    Switch(tcl_registry::native_switch_compilation::NativeSwitchInstruction),
    Structured(Box<tcl_registry::native_instruction_plan::NativeInstructionPlan>),
}

impl OriginalNativeCompilerPreparation {
    pub(crate) fn namespace_bindings(&self) -> Option<&NativeNamespaceBindingCompilation> {
        match self {
            Self::NamespaceBindings(recipe) => Some(recipe),
            Self::Switch(_) | Self::Structured(_) => None,
        }
    }

    pub(crate) fn switch(
        &self,
    ) -> Option<&tcl_registry::native_switch_compilation::NativeSwitchInstruction> {
        match self {
            Self::Switch(recipe) => Some(recipe),
            Self::NamespaceBindings(_) | Self::Structured(_) => None,
        }
    }

    pub(crate) fn structured(
        &self,
    ) -> Option<&tcl_registry::native_instruction_plan::NativeInstructionPlan> {
        if let Self::Structured(recipe) = self {
            Some(recipe)
        } else {
            None
        }
    }
}

/// Select source-dependent native recipes from the complete original vector.
/// Missing source never manufactures compiler components from argv values.
pub(crate) fn original_native_compilation(
    spec: NativeCompilationSpec,
    invocation: OriginalNativeCompilerInvocation<'_>,
) -> Option<(
    NativeCompilationSelection,
    Option<OriginalNativeCompilerPreparation>,
)> {
    if !spec.requires_original_word_preparation() {
        return None;
    }
    let Some(original) = original_native_compiler_words(
        invocation.image,
        invocation.words,
        invocation.offset,
        invocation.config,
    ) else {
        return Some((NativeCompilationSelection::Unknown, None));
    };
    let Some(protocol) = invocation.source_protocol else {
        return Some((NativeCompilationSelection::Unknown, None));
    };
    let Ok(captured) = NativeCompilerWords::capture(&original, protocol) else {
        return Some((NativeCompilationSelection::Unknown, None));
    };
    let selection = spec.select_native_words(
        &captured,
        invocation.operand_from,
        invocation.compiler_dialect,
        invocation.context,
    );
    let recipe = invocation.compiler_dialect.and_then(|dialect| {
        use tcl_registry::native_instruction_plan::{
            NativeInstructionPlan, native_instruction_plan,
        };
        let plan = native_instruction_plan(
            spec,
            selection,
            &captured,
            invocation.operand_from,
            dialect,
            invocation.context,
        )
        .ok()?;
        Some(match plan {
            NativeInstructionPlan::NamespaceBindings(recipe) => {
                OriginalNativeCompilerPreparation::NamespaceBindings(recipe)
            }
            NativeInstructionPlan::Switch(recipe) => {
                OriginalNativeCompilerPreparation::Switch(recipe)
            }
            plan => OriginalNativeCompilerPreparation::Structured(Box::new(plan)),
        })
    });
    Some((selection, recipe))
}

#[cfg(test)]
fn original_namespace_binding_compilation(
    spec: NativeCompilationSpec,
    invocation: OriginalNativeCompilerInvocation<'_>,
) -> Option<(
    NativeCompilationSelection,
    Option<NativeNamespaceBindingCompilation>,
)> {
    spec.namespace_binding_kind()?;
    original_native_compilation(spec, invocation).map(|(selection, preparation)| {
        (
            selection,
            preparation.and_then(|recipe| recipe.namespace_bindings().cloned()),
        )
    })
}

/// Capture an unchanged original command, including its delimiters and input
/// channel. Source positions alone cannot authenticate transformed argv.
/// Original traversal supplies a complete segmented vector; later consumers
/// first match the binding's retained complete vector before using this adapter.
/// This readonly lexical projection supplies executable component geometry,
/// not native compilation, local-table layout, lookup or Normal permission.
#[must_use]
pub fn original_native_compiler_words(
    image: &SourceImage,
    words: &[WordExpr],
    offset: u32,
    config: LexerConfig,
) -> Option<Vec<NativeWord>> {
    OriginalSourceCommandProjection::capture(image, words, offset, config)
        .map(|original| original.native)
}

/// Derived lexical geometry for one unchanged complete selected source vector.
/// The original parser and IR-vector checks own this immutable result. It
/// carries no selected implementation, metadata, native entry or completion.
#[derive(Debug)]
pub(crate) struct OriginalSourceCommandProjection {
    native: Vec<NativeWord>,
    command: crate::segmenter::SegmentedCommand,
}

impl OriginalSourceCommandProjection {
    pub(crate) fn capture(
        image: &SourceImage,
        words: &[WordExpr],
        offset: u32,
        config: LexerConfig,
    ) -> Option<Self> {
        if words.is_empty()
            || words
                .iter()
                .any(|word| word.source().provenance != Provenance::Source)
        {
            return None;
        }
        let text = image.try_text().ok()?;
        let spans: Vec<_> = words
            .iter()
            .map(|word| tcl_lexer::word_span_at(text, word.source().span))
            .collect();
        if spans.first()?.start() != offset {
            return None;
        }
        let region = Span::new(offset, spans.last()?.end());
        let mut parsed = tcl_lexer::native_script_words_in(image.clone(), region, config).ok()?;
        if parsed.fatal_tail.is_some() || parsed.commands.len() != 1 {
            return None;
        }
        let native = parsed.commands.pop()?.words;
        if native.len() != words.len()
            || native
                .iter()
                .zip(&spans)
                .any(|(word, span)| word.span() != *span)
        {
            return None;
        }
        // Compare the existing source IR owner too: an edit retaining old offsets
        // cannot borrow the compiler capabilities of the source it replaced.
        let original =
            SourceImage::from_bytes(image.bytes().get(region.as_range())?, image.channel());
        let segments = crate::segmenter::segment_commands_image_with_offset_and_config(
            &original, offset, config,
        )?;
        let [segment] = segments.as_slice() else {
            return None;
        };
        if segment.is_partial
            || CommandTokens::from_segmented(&SourceMap::from_image(image), config, segment).words()
                != words
        {
            return None;
        }
        Some(Self {
            native,
            command: segment.clone(),
        })
    }

    pub(crate) fn native_words(&self) -> &[NativeWord] {
        &self.native
    }

    pub(crate) const fn command(&self) -> &crate::segmenter::SegmentedCommand {
        &self.command
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::native_compilation::{NativeCompilationFrame, NativeCompilationMode};
    use tcl_syntax::native_string::NativeStringProtocol;

    fn source_words(image: &SourceImage, config: LexerConfig, offset: u32) -> Vec<WordExpr> {
        let segments =
            crate::segmenter::segment_commands_image_with_offset_and_config(image, 0, config)
                .unwrap();
        let segment = segments
            .iter()
            .find(|segment| segment.span.start() == offset)
            .unwrap();
        CommandTokens::from_segmented(&SourceMap::from_image(image), config, segment)
            .words()
            .to_vec()
    }

    #[test]
    fn original_compiler_geometry_preserves_delimiters_channels_and_provenance() {
        let config = LexerConfig::default();
        for image in [
            SourceImage::native(br#"variable {} "" {a\}}"#.as_slice()),
            SourceImage::document("variable {} \"\" {a\\}}"),
        ] {
            let words = source_words(&image, config, 0);
            let original = original_native_compiler_words(&image, &words, 0, config).unwrap();
            assert_eq!(original.len(), 4);
            assert!(original.iter().all(|word| word.image() == &image));
            assert_eq!(&image.bytes()[original[1].span().as_range()], b"{}");
            assert_eq!(&image.bytes()[original[2].span().as_range()], b"\"\"");
            assert_eq!(&image.bytes()[original[3].span().as_range()], br"{a\}}");

            let mut changed = words.clone();
            let WordExpr::BracedLiteral { text, .. } = &mut changed[1] else {
                panic!("braced original")
            };
            *text = "new".to_owned();
            assert!(original_native_compiler_words(&image, &changed, 0, config).is_none());
            let mut derived = words.clone();
            let WordExpr::Literal { source, .. } = &mut derived[0] else {
                panic!("literal original")
            };
            source.provenance = Provenance::Opaque;
            assert!(original_native_compiler_words(&image, &derived, 0, config).is_none());
            assert!(original_native_compiler_words(&image, &words, 1, config).is_none());
        }
    }

    #[test]
    fn original_quoted_namespace_operands_retain_native_parser_closers() {
        let source = r#"variable _tmm_operator_re "\\m([join $_gen_all_operators |])\\M""#;
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let profile =
                tcl_registry::model::ingress::resolve_environment(name).analyser_profile();
            let config = LexerConfig::from_grammar(profile.grammar);
            for image in [
                SourceImage::native(source.as_bytes()),
                SourceImage::document(source),
            ] {
                let words = source_words(&image, config, 0);
                let original = original_native_compiler_words(&image, &words, 0, config)
                    .unwrap_or_else(|| panic!("{name}: complete original namespace operand"));
                assert_eq!(original.len(), 3);
                assert_eq!(original[2].span().end() as usize, source.len());
                assert_eq!(original[2].bytes(), &source.as_bytes()[26..]);
                let mut changed = words;
                let WordExpr::Template { parts, .. } = &mut changed[2] else {
                    panic!("original quoted template")
                };
                parts.clear();
                assert!(original_native_compiler_words(&image, &changed, 0, config).is_none());
            }
        }
    }

    #[test]
    fn original_switch_compiler_words_keep_exact_vector_and_native_provider() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let spec = registry
            .native_compilation_for_registration(
                "switch",
                tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_1),
            )
            .unwrap();
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let profile =
                tcl_registry::model::ingress::resolve_environment(name).analyser_profile();
            let dialect = tcl_registry::InvocationDialect::of_profile(profile);
            let config = LexerConfig::from_grammar(profile.grammar);
            let image = SourceImage::document(
                "switch -- miss {x {set kept 1} x {set masked 2} default {set done 3}}",
            );
            let words = source_words(&image, config, 0);
            let invocation = OriginalNativeCompilerInvocation {
                image: &image,
                words: &words,
                offset: 0,
                config,
                source_protocol: dialect.native_source_string_protocol(),
                compiler_dialect: Some(dialect),
                context: NativeCompilationContext {
                    mode: NativeCompilationMode::BytecodeObject,
                    frame: NativeCompilationFrame::ProcedureCode,
                    ..Default::default()
                },
                operand_from: 1,
            };
            let (selection, preparation) = original_native_compilation(spec, invocation).unwrap();
            if matches!(name, "tcl8.4" | "jim") {
                assert_eq!(selection, NativeCompilationSelection::Generic, "{name}");
                assert!(preparation.is_none());
            } else {
                assert!(
                    matches!(selection, NativeCompilationSelection::Inline { .. }),
                    "{name}"
                );
                let recipe = preparation.as_ref().unwrap().switch().unwrap();
                assert_eq!(
                    recipe
                        .arms
                        .iter()
                        .map(|arm| arm.compile_body)
                        .collect::<Vec<_>>(),
                    vec![true, false, true]
                );
                for arm in &recipe.arms {
                    let span = arm.body.unwrap();
                    assert!(image.bytes()[span.as_range()].starts_with(b"set "));
                }
            }
            for missing in [
                OriginalNativeCompilerInvocation {
                    source_protocol: None,
                    ..invocation
                },
                OriginalNativeCompilerInvocation {
                    compiler_dialect: None,
                    ..invocation
                },
            ] {
                assert_eq!(
                    original_native_compilation(spec, missing).unwrap().0,
                    NativeCompilationSelection::Unknown,
                    "{name}"
                );
            }
            let mut changed = words.clone();
            let WordExpr::BracedLiteral { text, .. } = &mut changed[3] else {
                panic!("original arm list")
            };
            *text = "x {set changed 1}".to_owned();
            assert_eq!(
                original_native_compilation(
                    spec,
                    OriginalNativeCompilerInvocation {
                        words: &changed,
                        ..invocation
                    }
                )
                .unwrap(),
                (NativeCompilationSelection::Unknown, None)
            );
        }
    }

    #[test]
    fn original_list_preparations_keep_selected_target_evaluation_order() {
        // Native proof: naming.list.assignment-literal-versus-alias-target-evaluation
        // docs/design/analysis/name-resolution-proofs/assignment-literal-versus-alias-target-evaluation.md
        use tcl_registry::native_compilation::{NativeBodyCompilation, NativeCompilationGrammar};
        use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand;
        use tcl_registry::native_instruction_plan::NativeInstructionPlan;
        use tcl_registry::native_list_operations_compilation::{
            NativeListOperationInstruction, NativeListVariableOperand,
        };
        use tcl_syntax::native_variable_words::NativeVariableWordOperand;

        for version in tcl_dialect::TclVersion::ALL {
            let dialect = tcl_registry::InvocationDialect::for_version(version);
            let config = LexerConfig::from_grammar(dialect.lexer_grammar);
            for (source, grammar, operation, inline) in [
                (
                    "lassign {one two} first [set first]",
                    NativeCompilationGrammar::ListAssignment,
                    tcl_registry::IntrinsicId::ListAssign,
                    version >= tcl_dialect::TclVersion::V8_5,
                ),
                (
                    "lrange {one two} 0 end",
                    NativeCompilationGrammar::ListRange,
                    tcl_registry::IntrinsicId::ListRange,
                    version >= tcl_dialect::TclVersion::V8_6,
                ),
            ] {
                let spec = tcl_registry::native_compilation::NativeCompilationSpec {
                    grammar,
                    operation: tcl_registry::SemanticOperationId::Intrinsic(operation),
                    body: NativeBodyCompilation::Inherit,
                };
                let image = SourceImage::document(source);
                let words = source_words(&image, config, 0);
                let invocation = OriginalNativeCompilerInvocation {
                    image: &image,
                    words: &words,
                    offset: 0,
                    config,
                    source_protocol: dialect.native_source_string_protocol(),
                    compiler_dialect: Some(dialect),
                    context: NativeCompilationContext {
                        mode: NativeCompilationMode::BytecodeObject,
                        frame: NativeCompilationFrame::ScriptCode,
                        ..Default::default()
                    },
                    operand_from: 1,
                };
                let (selection, preparation) = original_native_compilation(spec, invocation)
                    .expect("implemented original list compiler preparation");
                assert_eq!(
                    matches!(selection, NativeCompilationSelection::Inline { .. }),
                    inline
                );
                if inline {
                    let NativeInstructionPlan::ListOperations(plan) = preparation
                        .as_ref()
                        .and_then(OriginalNativeCompilerPreparation::structured)
                        .expect("selected original list recipe")
                    else {
                        panic!("list operation preparation")
                    };
                    if let NativeListOperationInstruction::Assign { list, targets } = plan {
                        assert_eq!(list, &NativeCompilerWordOperand::Original(1));
                        assert_eq!(targets.len(), 2);
                        assert!(matches!(
                            &targets[1],
                            NativeListVariableOperand::Original {
                                operand: NativeCompilerWordOperand::Original(3),
                                variable: NativeVariableWordOperand::DynamicWord,
                            }
                        ));
                    }
                } else {
                    assert_eq!(selection, NativeCompilationSelection::Generic);
                    assert!(preparation.is_none());
                }
                let direct = OriginalNativeCompilerInvocation {
                    context: NativeCompilationContext {
                        mode: NativeCompilationMode::Direct,
                        ..invocation.context
                    },
                    ..invocation
                };
                assert_eq!(
                    original_native_compilation(spec, direct),
                    Some((NativeCompilationSelection::Generic, None))
                );
                assert_eq!(
                    original_native_compilation(
                        spec,
                        OriginalNativeCompilerInvocation {
                            source_protocol: None,
                            ..invocation
                        }
                    ),
                    Some((NativeCompilationSelection::Unknown, None))
                );
            }
        }
    }

    #[test]
    fn original_namespace_compiler_words_preserve_release_and_missing_issuer() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let spec = registry
            .native_compilation_for_registration(
                "variable",
                tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_1),
            )
            .unwrap();
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let profile =
                tcl_registry::model::ingress::resolve_environment(name).analyser_profile();
            let dialect = tcl_registry::InvocationDialect::of_profile(profile);
            let config = LexerConfig::from_grammar(profile.grammar);
            let image = SourceImage::document("variable ${ns}::v");
            let words = source_words(&image, config, 0);
            let invocation = OriginalNativeCompilerInvocation {
                image: &image,
                words: &words,
                offset: 0,
                config,
                source_protocol: dialect.native_source_string_protocol(),
                compiler_dialect: Some(dialect),
                context: NativeCompilationContext {
                    mode: NativeCompilationMode::BytecodeObject,
                    frame: NativeCompilationFrame::ProcedureCode,
                    ..Default::default()
                },
                operand_from: 1,
            };
            let (selection, recipe) =
                original_namespace_binding_compilation(spec, invocation).unwrap();
            assert_eq!(
                matches!(selection, NativeCompilationSelection::Inline { .. }),
                matches!(name, "tcl8.6" | "tcl9.0" | "tcl9.1"),
                "{name}"
            );
            if name == "jim" {
                assert_eq!(selection, NativeCompilationSelection::Generic);
                assert!(recipe.is_none());
            }
            assert_eq!(
                original_namespace_binding_compilation(
                    spec,
                    OriginalNativeCompilerInvocation {
                        source_protocol: None,
                        ..invocation
                    }
                )
                .unwrap(),
                (NativeCompilationSelection::Unknown, None)
            );
            let wrong_compiler = OriginalNativeCompilerInvocation {
                source_protocol: Some(NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1)),
                compiler_dialect: None,
                ..invocation
            };
            assert_eq!(
                original_namespace_binding_compilation(spec, wrong_compiler).unwrap(),
                (NativeCompilationSelection::Unknown, None)
            );
        }
    }
}
