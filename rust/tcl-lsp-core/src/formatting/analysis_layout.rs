// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Presentation coordinates joined to one supplied complete source analysis.

use std::collections::BTreeMap;

use tcl_compiler::{analyser::AnalysisResult, segmenter::SegmentedCommand};
use tcl_lexer::{DocumentLineEndingProjection, LexerConfig, NativeWord, Span};

pub(super) struct AnalysisFormattingLayout {
    projection: DocumentLineEndingProjection,
    config: LexerConfig,
    syntax: crate::source_structure::SourceSyntaxStructure,
    commands: BTreeMap<u32, (SegmentedCommand, Vec<NativeWord>)>,
}

impl AnalysisFormattingLayout {
    pub(super) fn capture(source: &str, analysis: &AnalysisResult) -> Option<Self> {
        // naming.editor.original-source-formatting
        // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting.md
        let analysis_source = tcl_lexer::normalise_lone_cr(source);
        crate::original_context::CurrentSourceContext::capture(&analysis_source, analysis)?;
        let image = analysis
            .retained_command_realm()?
            .original_source_image()?
            .clone();
        let projection = DocumentLineEndingProjection::new(image)?;
        let syntax =
            crate::source_structure::SourceSyntaxStructure::capture(&analysis_source, analysis)?;
        let config = analysis.body_lexer_config?;
        let commands = projected_commands(&projection, &syntax, config)?;
        Some(Self {
            projection,
            config,
            syntax,
            commands,
        })
    }

    pub(super) fn text(&self) -> &str {
        self.projection.text()
    }

    pub(super) fn presentation_span(&self, original: Span) -> Option<Span> {
        self.projection.normalised_span(original)
    }

    pub(super) fn original_offset(&self, presentation: u32) -> Option<u32> {
        self.projection.original_offset(presentation)
    }

    pub(super) fn source_region(&self, text: &str, start: u32) -> Option<Span> {
        let end = start.checked_add(u32::try_from(text.len()).ok()?)?;
        let presentation = Span::new(start, end);
        (self.text().get(presentation.as_range()) == Some(text)).then_some(())?;
        self.projection.original_span(presentation)
    }

    /// Join every complete word before any source schema or member-body query.
    /// Presentation equality alone cannot issue a new entry or source owner.
    pub(super) fn command(
        &self,
        text: &str,
        start: u32,
        head: u32,
        words: usize,
        config: LexerConfig,
    ) -> Option<(&SegmentedCommand, Vec<NativeWord>)> {
        (config == self.config).then_some(())?;
        let original_region = self.source_region(text, start)?;
        let original_head = self.original_offset(head)?;
        let (command, original) = self.commands.get(&original_head)?;
        if original.len() != words
            || command.argv.len() != words
            || original.first()?.span().start() < original_region.start()
            || original.last()?.span().end() > original_region.end()
            || self
                .projection
                .normalised_offset(original.first()?.span().start())
                != Some(head)
        {
            return None;
        }
        Some((command, original.clone()))
    }

    pub(super) fn member_body_arguments(
        &self,
        head: u32,
        original: &[NativeWord],
    ) -> Result<Option<Vec<usize>>, crate::source_structure::SourceSyntaxMemberUnavailable> {
        use crate::source_structure::SourceSyntaxMemberUnavailable::WordCorrespondence;
        let head = self.original_offset(head).ok_or(WordCorrespondence)?;
        let Some(member) = self.syntax.definition_member_at(head)? else {
            return Ok(None);
        };
        if member.original_words() != original {
            return Err(WordCorrespondence);
        }
        let bodies = member
            .script_bodies()
            .ok_or(WordCorrespondence)?
            .into_iter()
            .map(|body| {
                let written = body.argument().checked_add(1).ok_or(WordCorrespondence)?;
                if original.get(written) != Some(body.original_container())
                    || !self.has_script(body.content_span())
                {
                    return Err(WordCorrespondence);
                }
                Ok(written)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(bodies))
    }

    /// These interiors already come from the selected readonly syntax issuer,
    /// including actual parent-owned declaration vocabulary. They prove no
    /// executed body, closed lookup, native frame or expression equivalence.
    pub(super) fn has_script(&self, original: Span) -> bool {
        self.syntax
            .script_regions()
            .iter()
            .any(|&(region, _)| region == original)
    }
}

/// Build the derived geometric join once per genuine syntax region. No query
/// reanalyses the document or reparses all its commands for a single head.
fn projected_commands(
    projection: &DocumentLineEndingProjection,
    syntax: &crate::source_structure::SourceSyntaxStructure,
    config: LexerConfig,
) -> Option<BTreeMap<u32, (SegmentedCommand, Vec<NativeWord>)>> {
    let segments: BTreeMap<_, _> = syntax
        .commands()
        .iter()
        .filter_map(|command| Some((command.argv.first()?.span.start(), command)))
        .collect();
    let mut result: BTreeMap<u32, (SegmentedCommand, Vec<NativeWord>)> = BTreeMap::new();
    let presentation_image = tcl_lexer::SourceImage::document(projection.text());
    for &(region, _) in syntax.script_regions() {
        let original =
            tcl_lexer::native_script_words_in(projection.original().clone(), region, config)
                .ok()?;
        let presentation = tcl_lexer::native_script_words_in(
            presentation_image.clone(),
            projection.normalised_span(region)?,
            config,
        )
        .ok()?;
        let presentation: BTreeMap<_, _> = presentation
            .commands
            .iter()
            .filter_map(|command| Some((command.words.first()?.span().start(), command)))
            .collect();
        for original in &original.commands {
            let Some(first) = original.words.first() else {
                continue;
            };
            let head = first.span().start();
            let Some(segment) = segments.get(&head) else {
                continue;
            };
            let Some(presentation) = presentation.get(&projection.normalised_offset(head)?) else {
                continue;
            };
            if original.words.len() != segment.argv.len()
                || !same_word_projection(projection, &original.words, &presentation.words)
            {
                continue;
            }
            if let Some((_, earlier)) = result.get(&head) {
                if earlier != &original.words {
                    return None;
                }
            } else {
                result.insert(head, ((**segment).clone(), original.words.clone()));
            }
        }
    }
    Some(result)
}

fn same_word_projection(
    projection: &DocumentLineEndingProjection,
    original: &[NativeWord],
    presentation: &[NativeWord],
) -> bool {
    original.len() == presentation.len()
        && original
            .iter()
            .zip(presentation)
            .all(|(original, presentation)| {
                projection.original_span(presentation.span()) == Some(original.span())
                    && original.group().kind == presentation.group().kind
                    && original.group().expand == presentation.group().expand
            })
}
