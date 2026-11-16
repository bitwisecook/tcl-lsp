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

//! Readonly script geometry of one document: commands, script regions, lexical tokens and definition members.

use super::SourceStructure;
use std::sync::Arc;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::segmenter::SegmentedCommand;
use tcl_lexer::{NativeWord, SourceImage, Span, Token};

/// Readonly original script geometry from one complete retained analysis.
/// Source roles, lexical substitutions and declaration bodies supply syntax
/// regions only; no region proves dispatch, execution, a frame or an edit.
pub struct SourceSyntaxStructure {
    pub(super) structure: SourceStructure,
    lexical_regions: Vec<SourceSyntaxRegion>,
    context: Arc<tcl_registry::model::ContextRegistry>,
}

/// One independently selected syntax interior and its original lexical spans.
/// All token offsets address the complete document, including nested regions.
/// Tokens supply syntax classification only, without execution or edit authority.
pub struct SourceSyntaxRegion {
    pub(super) span: Span,
    tokens: Vec<Token>,
}

/// Readonly member selected by the genuine captured definition vocabulary.
/// This owns no global command identity, frame, formal value or edit permission.
pub(crate) struct SourceSyntaxDefinitionMember<'a> {
    pub(super) words: &'a [NativeWord],
    pub(super) bodies:
        Vec<tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberScriptBody>,
    pub(super) region: &'a tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberRegion,
}

/// Missing correspondence within an otherwise genuine member source query.
#[derive(Debug)]
pub(crate) enum SourceSyntaxMemberUnavailable {
    /// More than one selected parent vocabulary reaches this original head.
    AmbiguousVocabulary,
    /// Supplied whole words or body extents differ from the retained member.
    WordCorrespondence,
    /// An actual definition parent cannot supply complete current member layout.
    MemberDisposition,
}

impl SourceSyntaxDefinitionMember<'_> {
    /// Whole original member vector from the selected parent region.
    pub(crate) const fn original_words(&self) -> &[NativeWord] {
        self.words
    }

    /// Selected member scripts retain their original containers and ordinals.
    pub(crate) fn script_bodies(
        &self,
    ) -> &[tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberScriptBody] {
        &self.bodies
    }

    /// The genuine parent vocabulary region that selected this member.
    pub(crate) const fn region(
        &self,
    ) -> &tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberRegion {
        self.region
    }
}

impl SourceSyntaxRegion {
    /// Original complete-document extent of this script interior.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    /// Selected-grammar lexical tokens in the complete-document address space.
    #[must_use]
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }
}

impl SourceSyntaxStructure {
    /// Original readonly command vectors without the independent lexical-token
    /// inventory. The complete current analysis/input/image/configuration owns
    /// the same selected script regions as `capture`; no binding, execution,
    /// completeness or edit authority follows from an individual command.
    #[must_use]
    pub fn capture_commands(
        source: &str,
        analysis: &AnalysisResult,
    ) -> Option<Vec<SegmentedCommand>> {
        let (_, config) = tcl_compiler::source_graph::current_analysis(source, analysis)?;
        Some(SourceStructure::capture(source, Some(analysis), config)?.commands)
    }

    /// Capture the current document using its actual input, realm and grammar.
    /// Stale source or unavailable correspondence declines without recapture.
    #[must_use]
    pub fn capture(source: &str, analysis: &AnalysisResult) -> Option<Self> {
        let config = analysis.body_lexer_config?;
        let structure = SourceStructure::capture(source, Some(analysis), config)?;
        let image = SourceImage::document(source);
        let lexical_regions = structure
            .scripts
            .iter()
            .map(|&(span, depth)| {
                Some(SourceSyntaxRegion {
                    span,
                    tokens: tcl_lexer::source_region_tokens_in(
                        &image,
                        span,
                        config.at_depth(depth),
                    )
                    .ok()?,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            structure,
            lexical_regions,
            context: analysis.resolved_input.as_ref()?.context_registry(),
        })
    }

    /// Original commands in the selected readonly syntax regions.
    #[must_use]
    pub fn commands(&self) -> &[SegmentedCommand] {
        &self.structure.commands
    }

    /// Select a member at its actual original head before any global command
    /// lookup. Ambiguous parent vocabularies decline without nominal fallback.
    pub(crate) fn definition_member_at(
        &self,
        head: u32,
    ) -> Result<Option<SourceSyntaxDefinitionMember<'_>>, SourceSyntaxMemberUnavailable> {
        // naming.editor.original-source-formatting
        // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting.md
        use tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberDisposition as D;
        let region = self
            .structure
            .scripts
            .iter()
            .filter(|(region, _)| region.start() <= head && head < region.end())
            .max_by_key(|(_, depth)| *depth)
            .map(|(region, _)| *region);
        if region.is_some_and(|region| {
            self.structure
                .definition_members_unavailable
                .contains(&region)
        }) {
            return Err(SourceSyntaxMemberUnavailable::MemberDisposition);
        }
        let mut selected = None;
        let mut found_parent = false;
        for selection in &self.structure.definition_members {
            if Some(selection.source_region()) != region {
                continue;
            }
            let words = selection
                .original_command_words_at(head)
                .ok_or(SourceSyntaxMemberUnavailable::WordCorrespondence)?;
            if found_parent {
                return Err(SourceSyntaxMemberUnavailable::AmbiguousVocabulary);
            }
            found_parent = true;
            match selection.script_body_disposition(&self.context, words) {
                D::Unavailable => return Err(SourceSyntaxMemberUnavailable::MemberDisposition),
                D::CompleteNonMember => {}
                D::CompleteMember(bodies) => {
                    selected = Some(SourceSyntaxDefinitionMember {
                        words,
                        bodies,
                        region: selection,
                    });
                }
            }
        }
        Ok(selected)
    }

    /// Original script interiors and their shared lexer nesting depth.
    #[must_use]
    pub fn script_regions(&self) -> &[(Span, u32)] {
        &self.structure.scripts
    }

    /// Original script interiors with already joined lexical source geometry.
    /// Consumers do not offset lexer-local tokens or recapture a child input.
    #[must_use]
    pub fn lexical_regions(&self) -> &[SourceSyntaxRegion] {
        &self.lexical_regions
    }
}
