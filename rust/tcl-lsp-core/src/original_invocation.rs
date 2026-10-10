// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Readonly Registry advice with each effective operand's original producer.

use tcl_compiler::analyser::AnalysisResult;
use tcl_registry::{CommandRegistry, InvocationArguments};

#[derive(Clone)]
pub(crate) struct OriginalOperandSource {
    pub(crate) span: tcl_lexer::Span,
    pub(crate) input: Option<tcl_compiler::signature_scan::scope::SignatureSourceNameInput>,
    pub(crate) word: Option<tcl_lexer::NativeWord>,
}

pub(crate) use tcl_compiler::registry_invocation::source_structure::OriginalRegistrySource;

pub(crate) struct OriginalRegistryWords {
    pub(crate) command: String,
    pub(crate) dialect: Option<tcl_registry::InvocationDialect>,
    pub(crate) arguments: Vec<tcl_compiler::registry_invocation::EffectiveInvocationWord>,
    pub(crate) operands: Vec<Option<OriginalOperandSource>>,
    pub(crate) origins: Vec<tcl_compiler::registry_invocation::InvocationWordOrigin>,
    pub(crate) roles: Option<Vec<(usize, tcl_registry::ArgRole)>>,
    pub(crate) source: OriginalRegistrySource,
    structure: tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
}

impl OriginalRegistryWords {
    /// Compiler-owned written head geometry, independent of the resolved target.
    pub(crate) fn head_source(
        &self,
    ) -> Option<&tcl_compiler::registry_invocation::source_structure::OriginalOperandSource> {
        self.structure.head_source()
    }

    /// Same Compiler-owned source schema; no text head or argv reconstruction.
    pub(crate) fn with_source_schema<'r, T>(
        &'r self,
        context: &'r tcl_registry::model::ContextRegistry,
        project: impl FnOnce(&tcl_registry::ResolvedInvocation<'r, '_>) -> T,
    ) -> Option<T> {
        self.structure.with_source_schema(context, project)
    }

    /// Shared original Body/case grammar, independently of runtime entry.
    pub(crate) fn source_script_bodies(
        &self,
        context: &tcl_registry::model::ContextRegistry,
    ) -> Vec<tcl_compiler::registry_invocation::OriginalSourceScriptBody> {
        self.source_script_bodies_for(
            context,
            tcl_compiler::registry_invocation::OriginalSourceScriptPurpose::Syntax,
        )
    }

    /// Same source inventory, with its consumer's explicit syntax/evaluation purpose.
    pub(crate) fn source_script_bodies_for(
        &self,
        context: &tcl_registry::model::ContextRegistry,
        purpose: tcl_compiler::registry_invocation::OriginalSourceScriptPurpose,
    ) -> Vec<tcl_compiler::registry_invocation::OriginalSourceScriptBody> {
        self.structure.source_script_bodies_for(context, purpose)
    }

    /// Same Compiler-owned original expression command regions and input.
    pub(crate) fn source_expression_script_bodies(
        &self,
        input: &tcl_compiler::analyser::ResolvedAnalysisInput,
    ) -> Option<Vec<tcl_compiler::registry_invocation::OriginalSourceScriptBody>> {
        self.structure.source_expression_script_bodies(input)
    }

    /// Independent Compiler-owned check of the original argument lookup boundary.
    pub(crate) fn operands_preserve_source_lookup(&self) -> bool {
        self.structure.operands_preserve_source_lookup()
    }

    /// Select an effective argument from its own written producer. Expanded
    /// children require their independent original extent; a captured prefix
    /// cannot acquire the cursor's source position.
    pub(crate) fn active_argument_at(&self, written: usize, cursor: u32) -> Option<u32> {
        use tcl_compiler::registry_invocation::InvocationWordOrigin;
        let mut selected = None;
        for (ordinal, origin) in self.origins.iter().enumerate().skip(1) {
            let matches = match origin {
                InvocationWordOrigin::Written(index) => *index == written,
                InvocationWordOrigin::ExpandedElement { written: index, .. } => {
                    *index == written
                        && self
                            .operands
                            .get(ordinal - 1)
                            .and_then(Option::as_ref)
                            .is_some_and(|operand| {
                                operand.span.start() <= cursor && cursor <= operand.span.end()
                            })
                }
                InvocationWordOrigin::BindingPrefix(_) | InvocationWordOrigin::ResolvedHead => {
                    false
                }
            };
            if matches && selected.replace(u32::try_from(ordinal - 1).ok()?).is_some() {
                return None;
            }
        }
        selected
    }
}

/// Map a procedure argument through the same original expansion and captured
/// prefix geometry used by Registry advice. This issues only a cursor ordinal.
pub(crate) fn effective_argument_at(
    tokens: &tcl_compiler::ir::CommandTokens,
    effective: &tcl_compiler::registry_invocation::EffectiveCommandWords,
    written: usize,
    cursor: u32,
) -> Option<u32> {
    use tcl_compiler::registry_invocation::InvocationWordOrigin;
    let binding = tokens.source_binding.as_ref()?;
    let arguments = tcl_compiler::registry_invocation::frozen_argument_words(tokens, effective);
    let words = arguments
        .iter()
        .map(|word| word.as_registry_word())
        .collect::<Vec<_>>();
    InvocationArguments::structured(&words).exact_argv_len()?;
    if effective.origins.len() != arguments.len().checked_add(1)? {
        return None;
    }
    let mut selected = None;
    for (ordinal, origin) in effective.origins.iter().enumerate().skip(1) {
        let matches = match origin {
            InvocationWordOrigin::Written(index) => *index == written,
            InvocationWordOrigin::ExpandedElement {
                written: index,
                element,
            } => {
                let parent = binding.original_written_name_input(tokens, *index)?;
                let children = parent.original_list_elements_with_source_spans()?;
                let (input, span) = children.get(*element)?;
                if arguments.get(ordinal - 1)?.literal_bytes()? != input.bytes() {
                    return None;
                }
                *index == written
                    && span.is_some_and(|span| span.start() <= cursor && cursor <= span.end())
            }
            InvocationWordOrigin::BindingPrefix(_) | InvocationWordOrigin::ResolvedHead => false,
        };
        if matches && selected.replace(u32::try_from(ordinal - 1).ok()?).is_some() {
            return None;
        }
    }
    selected
}

fn from_source_words(
    words: tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
) -> OriginalRegistryWords {
    OriginalRegistryWords {
        command: words.command().to_owned(),
        dialect: words.dialect(),
        arguments: words.arguments().to_vec(),
        operands: words
            .operands()
            .iter()
            .map(|operand| {
                operand.as_ref().map(|operand| OriginalOperandSource {
                    span: operand.span(),
                    input: operand.input().cloned(),
                    word: operand.word().cloned(),
                })
            })
            .collect(),
        origins: words.origins().to_vec(),
        roles: words.roles().map(<[_]>::to_vec),
        source: words.source().clone(),
        structure: words,
    }
}

/// Thin adapter over the shared Compiler source argv/role owner.
pub(crate) fn selected_registry_words(
    source: &str,
    analysis: &AnalysisResult,
    segment: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
) -> Option<OriginalRegistryWords> {
    tcl_compiler::registry_invocation::source_structure::selected_registry_words(
        source, analysis, segment, registry,
    )
    .map(from_source_words)
}

/// Shared candidate-only structure, retaining all conditional applicability.
pub(crate) fn source_registry_words(
    source: &str,
    analysis: &AnalysisResult,
    segment: &tcl_compiler::segmenter::SegmentedCommand,
) -> Option<OriginalRegistryWords> {
    tcl_compiler::registry_invocation::source_structure::source_registry_words(
        source, analysis, segment,
    )
    .map(from_source_words)
}

/// Original list-built prefix syntax; every source and context premise stays
/// owned by the Compiler's deferred parent and complete builder receipts.
pub(crate) fn source_produced_command_prefix_words(
    source: &str,
    analysis: &AnalysisResult,
    producer: &tcl_compiler::segmenter::SegmentedCommand,
) -> Option<OriginalRegistryWords> {
    tcl_compiler::registry_invocation::source_structure::source_produced_command_prefix_words(
        source, analysis, producer,
    )
    .map(from_source_words)
}

/// Shared hosted source selector; no Native recipe or installer context donor.
pub(crate) fn selected_vendor_registry_words_at(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
) -> Option<(
    tcl_compiler::registry_invocation::OriginalConditionalVendorRegistryMetadata,
    tcl_compiler::segmenter::SegmentedCommand,
)> {
    tcl_compiler::registry_invocation::source_structure::selected_vendor_registry_words_at(
        source, analysis, cursor,
    )
}

/// Available Registry source advice at authentic full command vectors. The
/// traversal enters only independently retained script/body roles; omitted
/// unknown commands do not imply complete execution or reference coverage.
pub(crate) fn registry_commands_in_source(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<
    Vec<(
        tcl_compiler::segmenter::SegmentedCommand,
        OriginalRegistryWords,
    )>,
> {
    let walk = crate::refactor::FrameWalk::new(source, analysis)?;
    let selector = RegistrySourceWalk {
        source,
        analysis,
        walk,
    };
    let mut out = Vec::new();
    selector.in_region(source, 0, 0, &mut out);
    out.sort_by_key(|(command, _)| command.span.start());
    out.dedup_by_key(|(command, _)| command.span.start());
    Some(out)
}
struct RegistrySourceWalk<'a> {
    source: &'a str,
    analysis: &'a AnalysisResult,
    walk: crate::refactor::FrameWalk<'a>,
}
impl RegistrySourceWalk<'_> {
    fn in_region(
        &self,
        text: &str,
        base: u32,
        depth: u32,
        out: &mut Vec<(
            tcl_compiler::segmenter::SegmentedCommand,
            OriginalRegistryWords,
        )>,
    ) {
        if crate::references::MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) {
            return;
        }
        for command in self.walk.segment(text, base) {
            let regions = self
                .walk
                .same_frame_regions(self.source, &command)
                .into_iter()
                .chain(self.walk.frame_shifted_regions(self.source, &command));
            for (start, end) in regions {
                let Ok(start_offset) = u32::try_from(start) else {
                    continue;
                };
                if let Some(text) = self.source.get(start..end) {
                    self.in_region(text, start_offset, depth + 1, out);
                }
            }
            if let Some(words) = source_registry_words(self.source, self.analysis, &command) {
                out.push((command, words));
            }
        }
    }
}

#[cfg(test)]
mod source_words_tests {
    use super::*;

    #[test]
    fn original_source_words_share_roles_and_original_geometry() {
        // naming.core.original-command-source-schema
        // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
        let source = "set first 1\nset second 2\n";
        let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        analysis.command_invocations.clear();
        analysis.all_procs.clear();
        let config = analysis.body_lexer_config.unwrap();
        let commands =
            tcl_compiler::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let words = source_registry_words(source, &analysis, &commands[1]).unwrap();
        assert_eq!(words.arguments.len(), 2);
        assert!(
            words
                .roles
                .as_ref()
                .unwrap()
                .contains(&(0, tcl_registry::ArgRole::VarWrite))
        );
        assert_eq!(
            words.operands[0].as_ref().unwrap().span,
            commands[1].argv[1].span
        );
        assert_eq!(
            words.operands[0]
                .as_ref()
                .unwrap()
                .word
                .as_ref()
                .unwrap()
                .image(),
            &tcl_lexer::SourceImage::document(source)
        );
        match &words.source {
            OriginalRegistrySource::Selected => {}
            OriginalRegistrySource::Conditional(metadata) => {
                assert_eq!(metadata.original_words().len(), 3)
            }
            OriginalRegistrySource::SourceTransitions(_) => {
                panic!("direct literal borrowed transition advice")
            }
            OriginalRegistrySource::ProducedPrefix(_) => {
                panic!("direct invocation acquired produced-prefix syntax")
            }
            OriginalRegistrySource::Vendor(_) => panic!("Native input acquired hosted policy"),
            OriginalRegistrySource::Scoped(_) => {
                panic!("root Native input acquired scoped body policy")
            }
        }
        assert!(
            source_registry_words(&source.replace("second", "absent"), &analysis, &commands[1])
                .is_none()
        );
    }
}
