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

//! The exact single-cut declaration keeps its deferred syntax and world separate.

use super::OriginalSourceCutProcedureDeclaration;
use crate::analyser::AnalysisResult;
use crate::command_binding::{
    OriginalSingleSourceCutSurvey, OriginalSourceCutBodyEntryProjection, SourceCommandReference,
};
use crate::ir::CommandTokens;
use std::sync::OnceLock;

/// Deferred syntax under one genuine cut and its completed caller world.
/// This is no invocation, entered frame, Normal, Native or edit receipt.
pub struct OriginalSourceCutBodyLayout<'a> {
    declaration: OriginalSourceCutProcedureDeclaration<'a>,
    projection: OriginalSourceCutBodyEntryProjection,
    rows: OnceLock<Option<crate::command_binding::OriginalSourceCutBodyRows>>,
}

impl<'a> OriginalSourceCutBodyLayout<'a> {
    pub(super) fn capture(
        declaration: OriginalSourceCutProcedureDeclaration<'a>,
        survey: &OriginalSingleSourceCutSurvey<'_>,
    ) -> Option<Self> {
        if !declaration.is_for_analysis(declaration.analysis)
            || !survey.owns_declaration_command(declaration.analysis, &declaration.command)
        {
            return None;
        }
        let projection = survey.conditional_body_projection(declaration.analysis)?;
        Some(Self {
            declaration,
            projection,
            rows: OnceLock::new(),
        })
    }

    /// Reborrow only this same original body/analysis and complete vector.
    /// Existing point owners remain terminal; absent body proof has no fallback.
    #[must_use]
    pub fn stamp_original_tokens(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        tokens: &mut CommandTokens,
    ) -> Option<()> {
        self.current_rows(source, analysis)?.stamp(tokens)
    }

    /// A direct source reference from this exact conditional body row. The
    /// complete literal argv has no substitution effects on its captured table.
    /// Ordinary dispatch, entered frames and completion remain absent.
    #[must_use]
    pub fn source_command_reference<'b>(
        &'b self,
        source: &str,
        analysis: &AnalysisResult,
        tokens: &CommandTokens,
    ) -> Option<OriginalSourceCutBodyReference<'b, 'a>> {
        let reference = self
            .current_rows(source, analysis)?
            .source_reference(tokens)?;
        Some(OriginalSourceCutBodyReference {
            layout: self,
            reference,
        })
    }

    /// Result-argument coverage selected by this same captured cut-body row.
    /// This source-only purpose grants no inherited invocation or Normal fact.
    #[must_use]
    pub fn result_completion_coverage<'b>(
        &'b self,
        source: &str,
        analysis: &AnalysisResult,
        tokens: &CommandTokens,
    ) -> Option<OriginalSourceCutBodyResultCoverage<'b, 'a>> {
        let input = analysis.resolved_input.as_ref()?;
        let rows = self.current_rows(source, analysis)?;
        let registry = analysis.resolved_registry()?;
        let realm = analysis.retained_command_realm()?;
        let image = realm.original_source_image()?;
        let offset = tokens.words().first()?.source().span.start();
        let recorded =
            realm.retained_original_tokens_for_command(image, input.lexer_config(), offset, tokens);
        let lookups = rows.registry_lookups(
            tokens,
            input,
            recorded
                .as_ref()
                .and_then(|tokens| tokens.source_binding.as_ref()),
        )?;
        let mut agreed = None;
        for lookup in lookups {
            let context = lookup
                .original_declaration_metadata_context(registry, tokens)
                .ok()??;
            if context.source_analysis_input() != Some(input) {
                return None;
            }
            let selected =
                super::resolve_original_declared_layout(registry, context, tokens, &lookup)?;
            super::original_structured_operands_are_supported(registry, &selected, None, tokens)?;
            if !selected
                .with_metadata_schema(registry, context, lookup.realm(), |schema| {
                    Some(super::registered_result_completion_schema_is_supported(
                        &selected, input, schema,
                    ))
                })
                .is_some_and(|supported| supported)
                || agreed
                    .as_ref()
                    .is_some_and(|previous| previous != &selected)
            {
                return None;
            }
            agreed = Some(selected);
        }
        Some(OriginalSourceCutBodyResultCoverage {
            layout: self,
            selected: agreed?,
        })
    }

    fn current_rows(
        &self,
        source: &str,
        analysis: &AnalysisResult,
    ) -> Option<&crate::command_binding::OriginalSourceCutBodyRows> {
        let declaration = &self.declaration;
        let input = analysis.resolved_input.as_ref()?;
        let realm = analysis.retained_command_realm()?;
        let image = realm.original_source_image()?;
        if !declaration.is_for_analysis(analysis)
            || image.bytes() != source.as_bytes()
            || !analysis.matches_original_source_image(image, input.lexer_config())
            || !realm.matches_resolved_analysis_input(input)
            || !declaration.body.matches_source(image, input.lexer_config())
            || !declaration
                .body
                .matches_context(input.borrowed_context_registry())
        {
            return None;
        }
        self.rows
            .get_or_init(|| {
                crate::command_binding::OriginalSourceCutBodyRows::capture(
                    realm.source_bindings_ref(),
                    &declaration.header,
                    &declaration.body,
                    &self.projection,
                    input,
                )
            })
            .as_ref()
    }
}

/// A conditional source-only direct reference under the actual cut body owner.
/// This does not supply an ordinary lookup state, invocation, Native dispatch,
/// entered body, completion or permission to edit source.
pub struct OriginalSourceCutBodyReference<'layout, 'analysis> {
    layout: &'layout OriginalSourceCutBodyLayout<'analysis>,
    reference: SourceCommandReference,
}

impl OriginalSourceCutBodyReference<'_, '_> {
    /// Retain the exact declaration analysis; equal text is insufficient.
    #[must_use]
    pub fn is_for_analysis(&self, analysis: &AnalysisResult) -> bool {
        self.layout.declaration.is_for_analysis(analysis)
    }

    /// Navigation-only allocation selected by the captured conditional row.
    /// The enclosing cut and physical coverage guards remain independent.
    #[must_use]
    pub const fn source_reference(&self) -> &SourceCommandReference {
        &self.reference
    }
}

/// Selected Registry result syntax from an exact captured source-cut body row.
/// Completion remains abrupt source metadata, never executed Normal or Native.
pub struct OriginalSourceCutBodyResultCoverage<'layout, 'analysis> {
    layout: &'layout OriginalSourceCutBodyLayout<'analysis>,
    selected: super::ResolvedStatementInvocation,
}

impl OriginalSourceCutBodyResultCoverage<'_, '_> {
    /// Exact declaration analysis; source equality alone cannot attach it.
    #[must_use]
    pub fn is_for_analysis(&self, analysis: &AnalysisResult) -> bool {
        self.layout.declaration.is_for_analysis(analysis)
    }

    /// Registry-selected operation and roles, retaining independent link checks.
    #[must_use]
    pub const fn invocation(&self) -> &super::ResolvedStatementInvocation {
        &self.selected
    }
}
