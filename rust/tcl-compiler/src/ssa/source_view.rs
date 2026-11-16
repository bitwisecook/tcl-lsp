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

//! `SsaSourceView`: the positioned queries over a built function (definitions, reaching bindings, reads and their contents) and the original-source selectors at a unit.

use super::{SsaFunction, Symbol};
use crate::cfg::BlockId;
use crate::ir::CommandTokens;
use std::collections::HashMap;

mod analysis_definitions;
mod definitions;
mod expression_reads;
mod reaching_bindings;
mod source_selectors;
mod word_reads;

pub(crate) use analysis_definitions::current_authored_analysis_function;
pub(super) use analysis_definitions::original_written_source_variable_name;
pub use analysis_definitions::{
    OriginalAnalysisSsaDefinition, OriginalAnalysisSsaWords, original_analysis_definition_at_unit,
    original_analysis_words_for_statement,
};
pub use definitions::{
    OriginalFreshScalarAssignmentProposal, OriginalFreshScalarVariableProposal,
    OriginalSsaDefinitionName,
};
pub use expression_reads::SsaIncomingSlotRead;
pub use source_selectors::{
    OriginalSourceSsaDefinition, OriginalSourceSsaRead, original_source_definition_at_unit,
    original_source_read_at_unit,
};
pub(crate) use word_reads::SsaReadRepresentationAdvice;
pub(super) use word_reads::source_read_place;
pub use word_reads::{SsaReadContents, SsaReadPresenceAlternatives};

/// One conditional source definition in its current actual declared body.
/// Its authored SSA slot supplies no physical cell, successful store or edit permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceSsaDefinitionName {
    pub(super) span: tcl_lexer::Span,
    input: crate::signature_scan::scope::SignatureSourceNameInput,
    pub(super) body: crate::ir::ProcedureId,
}

impl OriginalSourceSsaDefinitionName {
    /// Complete original written variable operand extent in the document.
    #[must_use]
    pub const fn span(&self) -> tcl_lexer::Span {
        self.span
    }
    /// Genuine original operand under the retained Native source policy.
    #[must_use]
    pub fn original_name_input(&self) -> &crate::signature_scan::scope::SignatureSourceNameInput {
        &self.input
    }
    /// Current issued body whose conditional source owns this occurrence.
    #[must_use]
    pub const fn body_id(&self) -> crate::ir::ProcedureId {
        self.body
    }
}

/// Read-only SSA source-name lookup at an optional exact execution point.
/// Unpositioned compatibility queries succeed only for a unique proved binding.
#[derive(Clone, Copy)]
pub struct SsaSourceView<'a> {
    pub(super) ssa: &'a SsaFunction,
    point: Option<(BlockId, usize)>,
}

impl<'a> SsaSourceView<'a> {
    /// Whether this query selects one operation rather than compatibility-wide bindings.
    #[must_use]
    pub const fn is_positioned(self) -> bool {
        self.point.is_some()
    }

    /// Query only bindings proved unique over the whole function.
    #[must_use]
    pub const fn unpositioned(ssa: &'a SsaFunction) -> Self {
        Self { ssa, point: None }
    }

    /// Query bindings reaching one statement's evaluation.
    #[must_use]
    pub const fn at_statement(ssa: &'a SsaFunction, block: BlockId, index: usize) -> Self {
        Self {
            ssa,
            point: Some((block, index)),
        }
    }

    /// Query bindings reaching one terminator.
    #[must_use]
    pub const fn at_terminator(ssa: &'a SsaFunction, block: BlockId) -> Self {
        Self::at_statement(ssa, block, usize::MAX)
    }

    /// Return the selected function's SSA value tables.
    #[must_use]
    pub const fn function(self) -> &'a SsaFunction {
        self.ssa
    }

    /// Iterate source spellings with their proved symbols; missing positioned facts yield no names.
    pub fn source_symbols(self) -> impl Iterator<Item = (&'a str, Symbol)> {
        self.source_names()
            .into_iter()
            .flat_map(|names| names.iter().map(|(name, &symbol)| (name.as_str(), symbol)))
            .chain(
                self.point
                    .is_none()
                    .then_some(&self.ssa.var_to_symbol)
                    .into_iter()
                    .flat_map(|names| names.iter().map(|(name, &symbol)| (name.as_str(), symbol))),
            )
    }

    /// Resolve a source spelling using the selected proof context.
    #[must_use]
    pub fn symbol(self, name: &str) -> Option<Symbol> {
        self.point.map_or_else(
            || self.ssa.var_symbol(name),
            |(block, index)| self.ssa.var_symbol_at(block, index, name),
        )
    }

    /// Original words retained at this exact operation; absent provenance stays absent.
    #[must_use]
    pub fn source_tokens(self) -> Option<&'a CommandTokens> {
        let (block, index) = self.point?;
        self.ssa
            .point_contexts
            .as_ref()?
            .source_tokens_at(block, index)
    }

    /// Reaching source spellings for a positioned query; absence is not a fallback.
    #[must_use]
    pub fn source_names(self) -> Option<&'a HashMap<String, Symbol>> {
        self.point
            .and_then(|(block, index)| self.ssa.source_symbols_at(block, index))
    }
}

impl<'a> From<&'a SsaFunction> for SsaSourceView<'a> {
    fn from(ssa: &'a SsaFunction) -> Self {
        Self::unpositioned(ssa)
    }
}
