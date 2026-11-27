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

//! Private first-entry command advice from the original selected Jim wrapper.

use super::{Arc, ResolvedAnalysisInput, SourceCommandBindings};

impl SourceCommandBindings {
    /// Readonly first-head advice from the fresh source stock-call recipe.
    /// Scope coordinates only join the original body; complete current source,
    /// factory, world and inert-argv checks remain separate proof obligations.
    pub(crate) fn first_jim_method_presence_in_scope(
        &self,
        document: &str,
        input: &ResolvedAnalysisInput,
        scope_body: tcl_lexer::Span,
        offset: u32,
    ) -> Option<super::super::SourceCommandSlotPresence> {
        let image = self.root_origin.as_ref()?.source_image();
        if image.channel() != tcl_lexer::SourceChannel::Document
            || image.bytes() != document.as_bytes()
            || self.dispatch_points.contains_key(&offset)
        {
            return None;
        }
        let mut wrappers = self.future_body_inventories().iter().filter_map(|future| {
            let wrapper = future.original_jim_wrapper()?;
            (wrapper.input == *input
                && wrapper.body.tokens().iter().any(|token| {
                    token.kind == tcl_lexer::TokenType::Str && token.span == scope_body
                }))
            .then_some(wrapper)
        });
        let wrapper = wrappers.next()?;
        if wrappers.next().is_some()
            || !wrapper.call.matches_source_context(
                image,
                input.lexer_config(),
                input.borrowed_context_registry(),
            )
        {
            return None;
        }
        let entry = super::super::jim_stock_class::OriginalJimStockMethodEntry::capture(
            self,
            Arc::clone(&self.final_state),
            &wrapper.published,
            wrapper.call.original_words(),
        )?;
        if entry.input() != Some(input) || !entry.owns_world(&self.final_state) {
            return None;
        }
        let (first_offset, presence) = wrapper.first_stock_method_presence(&entry)?;
        (first_offset == offset).then_some(presence)
    }
}
