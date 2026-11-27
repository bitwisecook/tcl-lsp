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

//! Bounded readonly original command and body geometry for editor providers.

use std::collections::{BTreeSet, VecDeque};
use std::sync::Arc;
use tcl_compiler::analyser::{Analyser, AnalysisResult, ResolvedAnalysisInput};
use tcl_lexer::{ExecutablePart, LexerConfig, SourceImage, Span};
#[cfg(test)]
use tcl_registry::ArgRole;
use tcl_registry::CommandRegistry;
use tcl_source::segment::SegmentedCommand;
use tcl_source::segment::segment_commands_with_offset_and_config;

mod command_bodies;
mod cursor_syntax;
mod definition_grammar;
mod syntax_coverage;
mod syntax_structure;

use command_bodies::declared_bodies;
#[cfg(test)]
use command_bodies::logical_declared_bodies;
pub(crate) use cursor_syntax::{
    OriginalVariableReferences, SourceVariableReference, original_complete_cursor_word,
    original_cursor_word_value, original_method_site_at, original_variable_cursor_prefix,
    original_variable_reference_at, original_variable_word_end, original_word_cursor_prefix,
    template_variable_reference_at,
};
pub(crate) use definition_grammar::{DefinitionGrammarUnavailable, SourceDefinitionGrammar};
pub(crate) use syntax_coverage::SourcePackagePresenceCensus;
pub use syntax_coverage::{SourceSyntaxCoverage, SourceSyntaxCoverageApplicability};
pub(crate) use syntax_structure::{SourceSyntaxDefinitionMember, SourceSyntaxMemberUnavailable};
pub use syntax_structure::{SourceSyntaxRegion, SourceSyntaxStructure};

/// Explicit compatibility ingress retains the supplied store and full grammar
/// before the analyser sees a presentation label. Existing analyses bypass it.
pub(crate) fn analyse_document(
    source: &str,
    profile: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
    config: LexerConfig,
) -> AnalysisResult {
    document_analyser(profile, registry, config).analyse(source, profile.name)
}

/// A loader index analysed inside the procedure frame its C Tcl loader
/// evaluates it in, when the caller supplies one.
#[must_use]
pub(crate) fn analyse_loader_index(
    source: &str,
    profile: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
    config: LexerConfig,
    frame: Option<Arc<tcl_compiler::analyser::EntryProcedureFrame>>,
    structure_only: bool,
) -> AnalysisResult {
    let generation = crate::context_for_dialect_profile(profile);
    let context = Arc::new(generation.with_command_store(registry.snapshot().shared_registry()));
    let input = ResolvedAnalysisInput::new(profile, profile, context, config);
    let input = match frame {
        Some(frame) => input.with_entry_procedure_frame(frame),
        None => input,
    };
    let analyser = Analyser::new().with_resolved_input(input);
    let mut analyser = if structure_only {
        analyser.structure_only()
    } else {
        analyser
    };
    analyser.analyse(source, profile.name)
}

fn document_analyser(
    profile: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
    config: LexerConfig,
) -> Analyser {
    let generation = crate::context_for_dialect_profile(profile);
    let context = Arc::new(generation.with_command_store(registry.snapshot().shared_registry()));
    let input = ResolvedAnalysisInput::new(profile, profile, context, config);
    Analyser::new().with_resolved_input(input)
}

/// Each region is original source geometry, with independently selected source
/// applicability. None of these spans grants execution, a frame or an edit.
pub(crate) struct SourceStructure {
    /// Commands segmented in the actual document root. Descendant scripts
    /// are separately retained below and must not be reinterpreted as new
    /// document-level definition issuers.
    pub(crate) root_commands: Vec<SegmentedCommand>,
    pub(crate) commands: Vec<SegmentedCommand>,
    pub(crate) bodies: Vec<Span>,
    pub(crate) scripts: Vec<(Span, u32)>,
    pub(crate) compiler_body_syntax:
        Option<tcl_compiler::registry_invocation::source_structure::OriginalBodySyntaxInventory>,
    // Refusal-only source contexts: none establishes an entered or new frame.
    pub(crate) caller_context_unavailable_bodies: Vec<Span>,
    // The subset run during their command's own invocation, in another frame.
    pub(crate) invocation_scoped_bodies: Vec<Span>,
    definition_members:
        Vec<tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberRegion>,
    definition_members_unavailable: Vec<Span>,
    scoped_environment_regions: Vec<(
        tcl_compiler::registry_invocation::OriginalSourceScriptBody,
        u32,
    )>,
    package_census: Option<PackageCensusState>,
    definition_cursor_regions: Vec<DefinitionCursorRegion>,
    definition_cursor_unavailable: Vec<Span>,
    definition_command_parents: Vec<(
        Span,
        tcl_compiler::registry_invocation::OriginalSourceScriptBody,
    )>,
}

struct DefinitionCursorRegion {
    span: Span,
    depth: u32,
    parent: Option<tcl_compiler::registry_invocation::OriginalSourceScriptBody>,
    members: Option<tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberRegion>,
    plan: Option<tcl_lexer::NativeScriptWordsPlan>,
}

#[derive(Default)]
struct PackageCensusState {
    references:
        Vec<tcl_compiler::registry_invocation::source_structure::OriginalSourcePackageReference>,
    unavailable: Vec<Span>,
    syntax_unavailable: Vec<Span>,
    package_unavailable: Vec<Span>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CapturePurpose {
    Readonly,
    SyntaxCompaction,
    CheckedSyntax,
    DefinitionCursor,
}

type PendingSourceRegion = (
    Span,
    u32,
    Option<tcl_compiler::registry_invocation::OriginalSourceScriptBody>,
);

struct SourceScriptContext<'a> {
    analysis: &'a AnalysisResult,
    depth: u32,
    purpose: CapturePurpose,
    definition_parent: Option<&'a tcl_compiler::registry_invocation::OriginalSourceScriptBody>,
    definition_members:
        Option<&'a tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberRegion>,
}

struct SourceCaptureContext<'a> {
    source: &'a str,
    analysis: Option<&'a AnalysisResult>,
    config: LexerConfig,
    purpose: CapturePurpose,
    image: SourceImage,
}

impl SourceStructure {
    pub(crate) fn capture(
        source: &str,
        analysis: Option<&AnalysisResult>,
        config: LexerConfig,
    ) -> Option<Self> {
        Self::capture_with_purpose(source, analysis, config, CapturePurpose::Readonly)
    }

    /// Syntax compaction enters only independently closed original script
    /// roles. Readonly declarations and conditional schemas cannot authorise
    /// changes to descendant operand bytes.
    pub(crate) fn capture_for_syntax_compaction(
        source: &str,
        analysis: &AnalysisResult,
        config: LexerConfig,
    ) -> Option<Self> {
        Self::capture_with_purpose(
            source,
            Some(analysis),
            config,
            CapturePurpose::SyntaxCompaction,
        )
    }

    fn capture_with_purpose(
        source: &str,
        analysis: Option<&AnalysisResult>,
        config: LexerConfig,
        purpose: CapturePurpose,
    ) -> Option<Self> {
        // Implementation contract: naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md

        let image = SourceImage::document(source);
        let end = u32::try_from(source.len()).ok()?;
        if let Some(analysis) = analysis
            && (analysis.body_lexer_config != Some(config)
                || !analysis.matches_original_source_image(&image, config))
        {
            return None;
        }
        let mut structure = Self {
            root_commands: Vec::new(),
            commands: Vec::new(),
            compiler_body_syntax: (purpose == CapturePurpose::SyntaxCompaction)
                .then(|| analysis.and_then(|analysis| {
                    tcl_compiler::registry_invocation::source_structure::OriginalBodySyntaxInventory::capture(source, analysis)
                })).flatten(),
            bodies: Vec::new(),
            scripts: Vec::new(),
            caller_context_unavailable_bodies: Vec::new(),
            invocation_scoped_bodies: Vec::new(),
            definition_members: Vec::new(),
            definition_members_unavailable: Vec::new(),
            scoped_environment_regions: Vec::new(),
            definition_cursor_regions: Vec::new(),
            definition_cursor_unavailable: Vec::new(),
            definition_command_parents: Vec::new(),
            package_census: (purpose == CapturePurpose::CheckedSyntax)
                .then(PackageCensusState::default),
        };
        let mut queue: VecDeque<PendingSourceRegion> =
            VecDeque::from([(Span::new(0, end), 0, None)]);
        if purpose == CapturePurpose::Readonly
            && let Some(analysis) = analysis
        {
            for body in declared_bodies(source, analysis, &image, config)? {
                structure.bodies.push(body.extent);
                structure
                    .caller_context_unavailable_bodies
                    .push(body.content);
                queue.push_back((body.content, 1, None));
            }
            if let Some(realm) = analysis.retained_command_realm()
                && let Some(registry) = analysis.resolved_registry()
                && let Some(content) =
                    realm.original_incomplete_body_region_at(&image, config, end, registry)
            {
                structure.bodies.push(content);
                structure.caller_context_unavailable_bodies.push(content);
                queue.push_back((content, 1, None));
            }
        }
        let capture = SourceCaptureContext {
            source,
            analysis,
            config,
            purpose,
            image,
        };
        let mut visited = BTreeSet::new();
        while let Some((region, depth, definition_parent)) = queue.pop_front() {
            let parent_span = definition_parent
                .as_ref()
                .map(|parent| (parent.content_span().start(), parent.content_span().end()));
            if purpose == CapturePurpose::DefinitionCursor && depth > 32 {
                structure.definition_cursor_unavailable.push(region);
                continue;
            }
            if depth > 256 {
                if let Some(census) = &mut structure.package_census {
                    census.unavailable.push(region);
                }
                continue;
            }
            if !visited.insert((region.start(), region.end(), parent_span)) {
                continue;
            }
            structure.capture_region(
                &capture,
                region,
                depth,
                definition_parent.as_ref(),
                &mut queue,
            )?;
        }
        structure
            .scripts
            .sort_by_key(|(span, depth)| (span.start(), span.end(), *depth));
        structure.scripts.dedup_by(|left, right| left.0 == right.0);
        structure
            .bodies
            .sort_by_key(|span| (span.start(), span.end()));
        structure.bodies.dedup();
        structure
            .commands
            .sort_by_key(|command| (command.span.start(), command.span.end()));
        structure
            .commands
            .dedup_by(|a, b| a.span == b.span && a.all_tokens == b.all_tokens);
        Some(structure)
    }

    fn capture_region(
        &mut self,
        capture: &SourceCaptureContext<'_>,
        region: Span,
        depth: u32,
        definition_parent: Option<&tcl_compiler::registry_invocation::OriginalSourceScriptBody>,
        queue: &mut VecDeque<PendingSourceRegion>,
    ) -> Option<()> {
        let SourceCaptureContext {
            source,
            analysis,
            config,
            purpose,
            ..
        } = *capture;
        let script = source.get(region.as_range())?;
        self.scripts.push((region, depth));
        let commands =
            segment_commands_with_offset_and_config(script, region.start(), config.at_depth(depth));
        if depth == 0 {
            self.root_commands.extend(commands.iter().cloned());
        }
        let original_plan =
            tcl_lexer::native_script_words_in(capture.image.clone(), region, config);
        if let Some(census) = &mut self.package_census
            && !matches!(&original_plan, Ok(plan) if plan.fatal_tail.is_none())
        {
            census.unavailable.push(region);
        }
        if purpose == CapturePurpose::DefinitionCursor {
            let members = analysis.and_then(|analysis| {
                let input = analysis.resolved_input.as_ref()?;
                definition_parent?.definition_member_region(&input.context_registry(), region)
            });
            self.definition_cursor_regions.push(DefinitionCursorRegion {
                span: region,
                depth,
                parent: definition_parent.cloned(),
                members,
                plan: original_plan.as_ref().ok().cloned(),
            });
        }
        if let Ok(plan) = original_plan {
            for command in &plan.commands {
                for word in &command.words {
                    for part in word.executable_parts().all_parts() {
                        if matches!(part.part, ExecutablePart::ParseError(_))
                            && let Some(census) = &mut self.package_census
                        {
                            census.unavailable.push(word.span());
                        }
                        if let ExecutablePart::Command { body } = part.part {
                            queue.push_back((body, depth + 1, definition_parent.cloned()));
                        }
                    }
                }
            }
        }
        if let Some(analysis) = analysis {
            let definition_members = analysis.resolved_input.as_ref().and_then(|input| {
                definition_parent?.definition_member_region(&input.context_registry(), region)
            });
            if purpose == CapturePurpose::Readonly
                && definition_parent.is_some()
                && definition_members.is_none()
            {
                self.definition_members_unavailable.push(region);
            }
            let context = SourceScriptContext {
                analysis,
                depth,
                purpose,
                definition_parent,
                definition_members: definition_members.as_ref(),
            };
            for command in &commands {
                self.command_bodies(source, command, &context, queue);
            }
            if purpose == CapturePurpose::Readonly
                && let Some(members) = definition_members
            {
                self.definition_members.push(members);
            }
            if purpose == CapturePurpose::Readonly
                && let Some(parent) = definition_parent
            {
                self.definition_command_parents.extend(
                    commands
                        .iter()
                        .map(|command| (command.span, parent.clone())),
                );
            }
        }
        self.commands.extend(commands);
        Some(())
    }

    /// The definition body that directly holds `command`, when it was reached
    /// as a member body of a definition parent.
    pub(crate) fn definition_parent_of(
        &self,
        command: Span,
    ) -> Option<&tcl_compiler::registry_invocation::OriginalSourceScriptBody> {
        let mut parents = self
            .definition_command_parents
            .iter()
            .filter(|(span, _)| *span == command);
        let (_, parent) = parents.next()?;
        parents.all(|(_, other)| other == parent).then_some(parent)
    }

    pub(crate) fn command_at(&self, cursor: u32) -> Option<Span> {
        self.commands
            .iter()
            .map(|command| command.span)
            .filter(|span| span.start() <= cursor && cursor < span.end())
            .min_by_key(|span| span.end() - span.start())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod syntax_compaction_tests;

#[cfg(test)]
mod original_hover_word_geometry_tests;

#[cfg(test)]
mod opaque_script_authoring_tests;
