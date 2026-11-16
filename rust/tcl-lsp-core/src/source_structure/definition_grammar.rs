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

//! The definition-body vocabulary selected at a cursor, and the opaque script operands open to authoring.

use super::{CapturePurpose, DefinitionCursorRegion, SourceStructure};
use tcl_compiler::analyser::{AnalysisResult, ResolvedAnalysisInput};
use tcl_lexer::{SourceImage, Span, WordKind};

impl DefinitionCursorRegion {
    pub(super) fn vocabulary(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        cursor: u32,
        current: &crate::original_context::CurrentSourceContext<'_>,
    ) -> SourceDefinitionGrammar {
        use DefinitionGrammarUnavailable as Unavailable;
        use SourceDefinitionGrammar as Selection;
        let Some(input) = analysis.resolved_input.as_ref() else {
            return Selection::Unavailable(Unavailable::CurrentSource);
        };
        let contains = |span: Span| span.start() <= cursor && cursor <= span.end();
        let Some(plan) = &self.plan else {
            return Selection::Unavailable(Unavailable::MalformedRegion);
        };
        if plan
            .fatal_tail
            .is_some_and(|tail| cursor >= tail.command_start)
        {
            return Selection::Unavailable(Unavailable::MalformedRegion);
        }
        let image = SourceImage::document(source);
        let root = |nested_unavailable| {
            let grammar = current.registry().document_grammar()?;
            Some(Selection::StandaloneRoot {
                vocabulary: SourceDefinitionVocabulary {
                    grammar,
                    image: image.clone(),
                    input: input.clone(),
                    parent: None,
                    members: None,
                },
                nested_unavailable,
            })
        };
        // An operand interior is not a nested script unless the genuine body
        // issuer already queued it. Root authoring is a separate purpose and
        // retains this unavailable-child debt instead of selecting prior scope.
        let unrepresented = plan
            .commands
            .iter()
            .flat_map(|command| &command.words)
            .any(|word| {
                matches!(word.group().kind, WordKind::Braced | WordKind::Quoted)
                    && word.content_span().is_ok_and(contains)
            });
        if unrepresented {
            return if self.depth == 0 {
                root(Some(Unavailable::UnrepresentedInterior))
                    .unwrap_or(Selection::Unavailable(Unavailable::UnrepresentedInterior))
            } else {
                Selection::Unavailable(Unavailable::UnrepresentedInterior)
            };
        }
        let Some(parent) = &self.parent else {
            return if self.depth == 0 {
                root(None).unwrap_or(Selection::OrdinaryScript)
            } else {
                Selection::OrdinaryScript
            };
        };
        let context = current.context();
        if !parent.matches_source(&image, current.config()) || !parent.matches_context(&context) {
            return Selection::Unavailable(Unavailable::ParentCorrespondence);
        }
        let Some(members) = &self.members else {
            return Selection::Unavailable(Unavailable::ParentCorrespondence);
        };
        let Some(grammar) = parent
            .source_words()
            .with_source_schema(&context, |schema| {
                schema.authored_source_definition_body_grammar()
            })
            .flatten()
        else {
            return Selection::Unavailable(Unavailable::ParentCorrespondence);
        };
        Selection::Selected(SourceDefinitionVocabulary {
            grammar,
            image,
            input: input.clone(),
            parent: Some(parent.clone()),
            members: Some(members.clone()),
        })
    }
}

/// A completion-only opaque operand under an authentic selected definition
/// parent. Source/input currency is retained independently of script traversal.
pub(crate) struct OpaqueScriptAuthoringContext {
    pub(super) region: tcl_compiler::registry_invocation::OpaqueScriptAuthoringRegion,
    pub(super) image: SourceImage,
    pub(super) input: ResolvedAnalysisInput,
}

impl OpaqueScriptAuthoringContext {
    /// Select lexical command-position geometry solely inside the retained
    /// opaque operand. No descendant script, scope or variable advice follows.
    pub(crate) fn command_prefix(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        cursor: u32,
    ) -> Option<tcl_lexer::word_parts::WordCursorPrefix> {
        let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
        if self.image != SourceImage::document(source)
            || analysis.resolved_input.as_ref() != Some(&self.input)
        {
            return None;
        }
        let content =
            self.region
                .content_for_authoring(&self.image, current.config(), &current.context())?;
        let prefix = tcl_lexer::word_parts::word_cursor_prefix_in(
            &self.image,
            content,
            cursor,
            current.config(),
        )
        .ok()??;
        (prefix.word_index() == 0).then_some(prefix)
    }
}

/// Why current source cannot select a nested definition vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DefinitionGrammarUnavailable {
    CurrentSource,
    DepthLimit,
    AmbiguousRegion,
    ParentCorrespondence,
    MalformedRegion,
    UnrepresentedInterior,
}

/// Genuine selected source vocabulary, separate from an entered definition frame.
/// A standalone document root has no fabricated body or parent declaration.
pub(crate) struct SourceDefinitionVocabulary {
    grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
    pub(super) image: SourceImage,
    pub(super) input: ResolvedAnalysisInput,
    pub(super) parent: Option<tcl_compiler::registry_invocation::OriginalSourceScriptBody>,
    pub(super) members:
        Option<tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberRegion>,
}

impl SourceDefinitionVocabulary {
    pub(crate) fn grammar(
        &self,
        source: &str,
        analysis: &AnalysisResult,
    ) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
        let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
        if self.image != SourceImage::document(source)
            || analysis.resolved_input.as_ref() != Some(&self.input)
        {
            return None;
        }
        let context = current.context();
        let actual = if let Some(parent) = &self.parent {
            if !parent.matches_source(&self.image, current.config())
                || !parent.matches_context(&context)
                || self.members.is_none()
            {
                return None;
            }
            parent
                .source_words()
                .with_source_schema(&context, |schema| {
                    schema.authored_source_definition_body_grammar()
                })??
        } else {
            current.registry().document_grammar()?
        };
        std::ptr::eq(actual, self.grammar).then_some(actual)
    }
}

/// Cursor vocabulary purpose and unavailable nested ownership stay distinct.
pub(crate) enum SourceDefinitionGrammar {
    Selected(SourceDefinitionVocabulary),
    /// Root authoring suggestions remain valid for a declaration document even
    /// when a written operand is not a selected child script. The debt cannot
    /// supply that child's grammar or edit candidates.
    StandaloneRoot {
        vocabulary: SourceDefinitionVocabulary,
        nested_unavailable: Option<DefinitionGrammarUnavailable>,
    },
    OrdinaryScript,
    Unavailable(DefinitionGrammarUnavailable),
}

impl SourceDefinitionGrammar {
    pub(crate) const fn unavailable(&self) -> Option<DefinitionGrammarUnavailable> {
        match self {
            Self::Unavailable(reason) => Some(*reason),
            _ => None,
        }
    }

    /// Candidate edits require a selected nested vocabulary or an unobstructed
    /// actual standalone root, independently of the caller's edit authority.
    pub(crate) fn for_selected_source(
        &self,
        source: &str,
        analysis: &AnalysisResult,
    ) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
        match self {
            Self::Selected(vocabulary)
            | Self::StandaloneRoot {
                vocabulary,
                nested_unavailable: None,
            } => SourceDefinitionVocabulary::grammar(vocabulary, source, analysis),
            _ => None,
        }
    }

    /// Explicit declaration-document authoring projection only. In particular,
    /// a root suggestion with nested debt does not select a child grammar.
    pub(crate) fn for_authoring(
        &self,
        source: &str,
        analysis: &AnalysisResult,
    ) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
        match self {
            Self::Selected(vocabulary) | Self::StandaloneRoot { vocabulary, .. } => {
                SourceDefinitionVocabulary::grammar(vocabulary, source, analysis)
            }
            _ => None,
        }
    }

    /// Scoped source documentation at an exact member head in a genuine
    /// selected definition body. This supplies no entered command or dispatch.
    pub(crate) fn selected_scoped_head(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        cursor: u32,
    ) -> Option<(
        &'static tcl_registry::scoped::ScopedCommandEnv,
        &'static tcl_registry::scoped::ScopedCommand,
    )> {
        let Self::Selected(vocabulary) = self else {
            return None;
        };
        vocabulary.grammar(source, analysis)?;
        let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
        let parent = vocabulary.parent.as_ref()?;
        let members = vocabulary.members.as_ref()?;
        let context = current.context();
        let environment = parent
            .source_words()
            .with_source_schema(&context, |schema| {
                schema.authored_source_descriptors().command.body_scope
            })??;
        let commands = members.complete_original_commands()?;
        let mut found = commands.iter().filter_map(|words| {
            let head = words.first()?;
            if head.group().expand || !head.span().as_range().contains(&(cursor as usize)) {
                return None;
            }
            members.member_keyword(words)?;
            let name = tcl_syntax::word_rules::original_static_word_ascii_presentation(head)?;
            let name = std::str::from_utf8(&name).ok()?;
            environment.command(name)
        });
        let command = found.next()?;
        found.next().is_none().then_some((environment, command))
    }

    #[cfg(test)]
    pub(crate) fn is_none(&self) -> bool {
        matches!(self, Self::OrdinaryScript | Self::Unavailable(_))
    }
}

impl SourceStructure {
    /// Distinct authoring-only selection; opaque operands never enter the
    /// ordinary script inventory or inherit a definition vocabulary.
    pub(crate) fn opaque_script_authoring_at(
        source: &str,
        analysis: &AnalysisResult,
        cursor: u32,
    ) -> Option<OpaqueScriptAuthoringContext> {
        let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
        let image = SourceImage::document(source);
        let input = analysis.resolved_input.as_ref()?;
        let context = current.context();
        let structure = Self::capture_with_purpose(
            source,
            Some(analysis),
            current.config(),
            CapturePurpose::DefinitionCursor,
        )?;
        let mut selected = None;
        for members in structure
            .definition_cursor_regions
            .iter()
            .filter_map(|region| region.members.as_ref())
        {
            for words in members.original_commands() {
                let Some(regions) = members.opaque_script_authoring_regions(&context, words) else {
                    continue;
                };
                for region in regions {
                    let body_content =
                        region.content_for_authoring(&image, current.config(), &context)?;
                    if body_content.start() <= cursor && cursor <= body_content.end() {
                        if selected.is_some() {
                            return None;
                        }
                        selected = Some(OpaqueScriptAuthoringContext {
                            region,
                            image: image.clone(),
                            input: input.clone(),
                        });
                    }
                }
            }
        }
        selected
    }

    /// One purpose-specific cursor query over the existing genuine body walk.
    /// No display head, detached body or report scope selects a vocabulary.
    pub(crate) fn definition_grammar_at(
        source: &str,
        analysis: &AnalysisResult,
        cursor: u32,
    ) -> SourceDefinitionGrammar {
        use DefinitionGrammarUnavailable as Unavailable;
        use SourceDefinitionGrammar as Selection;
        let Some(current) =
            crate::original_context::CurrentSourceContext::capture(source, analysis)
        else {
            return Selection::Unavailable(Unavailable::CurrentSource);
        };
        let Some(structure) = Self::capture_with_purpose(
            source,
            Some(analysis),
            current.config(),
            CapturePurpose::DefinitionCursor,
        ) else {
            return Selection::Unavailable(Unavailable::CurrentSource);
        };
        if !source.is_char_boundary(cursor as usize) {
            return Selection::Unavailable(Unavailable::CurrentSource);
        }
        let contains = |span: Span| span.start() <= cursor && cursor <= span.end();
        if structure
            .definition_cursor_unavailable
            .iter()
            .copied()
            .any(contains)
        {
            return Selection::Unavailable(Unavailable::DepthLimit);
        }
        let Some(region) = structure
            .definition_cursor_regions
            .iter()
            .filter(|region| contains(region.span))
            .max_by_key(|region| region.depth)
        else {
            return Selection::Unavailable(Unavailable::CurrentSource);
        };
        if structure.definition_cursor_regions.iter().any(|other| {
            contains(other.span) && other.depth == region.depth && !std::ptr::eq(other, region)
        }) {
            return Selection::Unavailable(Unavailable::AmbiguousRegion);
        }
        DefinitionCursorRegion::vocabulary(region, source, analysis, cursor, &current)
    }
}
