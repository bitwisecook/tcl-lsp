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

//! The checked inventory of selected script syntax, how complete it is and the package census read from it.

use super::{CapturePurpose, SourceStructure};
use tcl_compiler::analyser::{AnalysisResult, ResolvedAnalysisInput};
use tcl_lexer::{SourceImage, Span};
use tcl_source::segment::SegmentedCommand;

/// Completeness of independently selected original script syntax only.
/// Complete supplies no dispatch/call-graph closure, execution or Normal result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceSyntaxCoverageApplicability {
    /// All independently selected script syntax is represented.
    Complete,
    /// Some selected script syntax is unavailable or unrepresented.
    Incomplete,
}

/// Checked original syntax inventory tied to one retained whole input.
/// This is separate from the ordinary readonly partial source inventory.
pub struct SourceSyntaxCoverage {
    pub(super) image: SourceImage,
    pub(super) input: ResolvedAnalysisInput,
    pub(super) structure: SourceStructure,
}

impl SourceSyntaxCoverage {
    /// Inspect genuine selected body roles, including deferred definition
    /// syntax, and preserve all unavailable/malformed/unrepresented regions.
    pub fn capture(source: &str, analysis: &AnalysisResult) -> Option<Self> {
        let config = analysis.body_lexer_config?;
        let input = analysis.resolved_input.as_ref()?;
        let realm = analysis.retained_command_realm()?;
        if config != input.lexer_config() || !realm.matches_resolved_analysis_input(input) {
            return None;
        }
        let structure = SourceStructure::capture_with_purpose(
            source,
            Some(analysis),
            config,
            CapturePurpose::CheckedSyntax,
        )?;
        Some(Self {
            image: SourceImage::document(source),
            input: input.clone(),
            structure,
        })
    }

    pub(super) fn matches(&self, source: &str, analysis: &AnalysisResult) -> bool {
        self.image == SourceImage::document(source)
            && analysis.resolved_input.as_ref() == Some(&self.input)
            && analysis.matches_original_source_image(&self.image, self.input.lexer_config())
    }

    /// Captured commands remain useful even when selected syntax is incomplete.
    /// Callers retain independent actual call/declaration applicability checks.
    pub fn commands<'a>(
        &'a self,
        source: &str,
        analysis: &AnalysisResult,
    ) -> Option<&'a [SegmentedCommand]> {
        self.matches(source, analysis)
            .then_some(self.structure.commands.as_slice())
    }

    /// Actual selected script regions and their retained nesting depths.
    /// This is source geometry only, for consumers with independent budgets;
    /// unavailable/foreign source cannot borrow another capture's depths.
    pub fn script_regions<'a>(
        &'a self,
        source: &str,
        analysis: &AnalysisResult,
    ) -> Option<&'a [(Span, u32)]> {
        self.matches(source, analysis)
            .then_some(self.structure.scripts.as_slice())
    }

    /// Candidate-only scoped vocabulary from genuine original Body/schema
    /// owners. Inner frame boundaries stop inheritance; no report span can
    /// manufacture a selected scoped parent or complete command table.
    pub fn logical_scoped_environment_at(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        cursor: u32,
    ) -> Option<tcl_compiler::registry_invocation::OriginalLogicalScopedEnvironment> {
        self.matches(source, analysis).then_some(())?;
        let context = analysis.resolved_input.as_ref()?.context_registry();
        let mut regions = self
            .structure
            .scoped_environment_regions
            .iter()
            .filter(|(body, _)| {
                body.content_span().start() <= cursor && cursor < body.content_span().end()
            })
            .collect::<Vec<_>>();
        regions.sort_by_key(|(body, depth)| {
            (
                std::cmp::Reverse(*depth),
                body.content_span().end() - body.content_span().start(),
            )
        });
        for (body, _) in regions {
            if let Some(environment) = body.logical_scoped_environment(&context) {
                return Some(environment);
            }
            if !body.inherits_immediate_caller_source_context(&context) {
                return None;
            }
        }
        None
    }

    /// [`Self::applicability`] where the caller independently proved each
    /// command whose schema is missing; any other such command stays open.
    pub(crate) fn complete_with_proved_commands(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        proved: impl Fn(Span) -> bool,
    ) -> bool {
        self.matches(source, analysis)
            && self.structure.package_census.as_ref().is_some_and(|state| {
                state
                    .unavailable
                    .iter()
                    .chain(&state.syntax_unavailable)
                    .all(|span| proved(*span))
            })
    }

    /// Complete means every represented selected script syntax body was
    /// inspected; missing schemas, including absent heads, stay incomplete.
    pub fn applicability(
        &self,
        source: &str,
        analysis: &AnalysisResult,
    ) -> Option<SourceSyntaxCoverageApplicability> {
        if !self.matches(source, analysis) {
            return None;
        }
        let state = self.structure.package_census.as_ref()?;
        Some(
            if state.unavailable.is_empty() && state.syntax_unavailable.is_empty() {
                SourceSyntaxCoverageApplicability::Complete
            } else {
                SourceSyntaxCoverageApplicability::Incomplete
            },
        )
    }
}

/// Source-only selected package syntax with explicit incomplete coverage.
/// A complete negative says nothing about installed/runtime package state.
pub(crate) struct SourcePackagePresenceCensus {
    coverage: SourceSyntaxCoverage,
}

impl SourcePackagePresenceCensus {
    pub(crate) fn capture(source: &str, analysis: &AnalysisResult) -> Option<Self> {
        Some(Self {
            coverage: SourceSyntaxCoverage::capture(source, analysis)?,
        })
    }

    /// Explicit authoring-value comparison, separate from Native package keys.
    /// Positive original references survive other incomplete regions; a negative
    /// requires every retained reference to support this exact source purpose.
    pub(crate) fn already_names_unicode_source_value(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        package: &str,
    ) -> Option<bool> {
        if !self.coverage.matches(source, analysis) {
            return None;
        }
        let config = analysis.body_lexer_config?;
        let image = SourceImage::document(source);
        let state = self.coverage.structure.package_census.as_ref()?;
        let mut unavailable = false;
        for reference in &state.references {
            match reference.matches_unicode_source_value(&image, config, package) {
                Some(true) => return Some(true),
                Some(false) => {}
                None => unavailable = true,
            }
        }
        (!unavailable && state.unavailable.is_empty() && state.package_unavailable.is_empty())
            .then_some(false)
    }

    /// Positive genuine selected references survive incomplete other regions.
    /// Incomplete/no-representation is None, never a complete source negative.
    pub(crate) fn already_names(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        package: &str,
    ) -> Option<bool> {
        if !self.coverage.matches(source, analysis) {
            return None;
        }
        let state = self.coverage.structure.package_census.as_ref()?;
        if state
            .references
            .iter()
            .any(|reference| reference.matches_ascii(package))
        {
            return Some(true);
        }
        (state.unavailable.is_empty() && state.package_unavailable.is_empty()).then_some(false)
    }
}
