// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The exact single-cut declaration keeps its deferred syntax and world separate.

use super::OriginalSourceCutProcedureDeclaration;
use crate::analyser::AnalysisResult;
use crate::command_binding::{OriginalSingleSourceCutSurvey, OriginalSourceCutBodyEntryProjection};
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
            .as_ref()?
            .stamp(tokens)
    }
}
