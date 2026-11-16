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

//! Usage and shape lint checks emitted during the command walk.
//!
//! These diagnostics inspect a single command's words and argument tokens
//! for style and shape problems that need no control- or data-flow:
//! unbraced `expr` / `if` / `while` bodies (W100, W105), unbraced `switch`
//! bodies (W106), a redundant nested `expr` (W114), the `string`-vs-`==`
//! comparison rewrite (W110), a `lappend` on a non-list value (W104),
//! name-vs-value confusion in a variable write (W212), a `${name}(...)`
//! array-write that should be `name(...)` (W216), non-ASCII and
//! encoding-mismatch text (W108, W311), invalid `binary format` modifiers
//! (W200), and an invalid subnet mask literal (W121).

mod source_channel;

#[cfg(test)]
use tcl_core_types::DiagCode;

use crate::analyser::state::Analyser;
#[cfg(test)]
use crate::expr_ast::ExprNode;

mod expression_rewrites;
mod literal_contents;
mod unbraced_words;
mod variable_arguments;

pub(in crate::analyser) use expression_rewrites::W110Anchor;
#[cfg(test)]
pub(super) use expression_rewrites::first_nested_expr;
#[cfg(test)]
use expression_rewrites::{find_string_eq_ne_ops, is_expr_atom, node_extent};
#[cfg(test)]
use literal_contents::comment_regions_recursive;
#[cfg(test)]
pub(super) use literal_contents::{
    is_benign_unicode, is_valid_subnet_mask, looks_like_subnet_mask, nearest_valid_mask,
};
#[cfg(test)]
pub(super) use unbraced_words::is_safe_literal;
#[cfg(test)]
pub(super) use unbraced_words::is_safe_literal_expr;
pub(in crate::analyser) use unbraced_words::original_word_has_substitution;

impl Analyser {
    /// Parser topology from the actual source input, independently of handler
    /// success. Complete availability validates the owner without choosing its
    /// expression or variable-reference grammar.
    pub(in crate::analyser) fn original_expression_parser_context(
        &self,
    ) -> Option<tcl_syntax::expr::parser::ExprParseContext> {
        let input = self.result.resolved_input.as_ref()?;
        if !self.head_identities.matches_resolved_analysis_input(input) {
            return None;
        }
        let context = self.head_identities.source_metadata_context()?;
        crate::registry_invocation::InvocationMetadataContext::for_source_input(
            context.commands(),
            input,
            self.file_lexer_config(),
            self.unit_profile,
        )?;
        let mut parser = tcl_registry::InvocationDialect::of_profile(input.unit_profile())
            .expression_parse_context(Some(input.unit_profile()));
        parser.lexer_grammar = input.lexer_config().grammar_over(parser.lexer_grammar);
        Some(parser)
    }
}

#[cfg(test)]
mod issue996_tests;

#[cfg(test)]
mod original_expression_grammar_tests;

#[cfg(test)]
mod original_reference_atom_tests;

#[cfg(test)]
mod original_braced_array_proposal_controls;

#[cfg(test)]
mod selected_nested_expression_controls;

#[cfg(test)]
mod original_w110_operator_geometry_controls;

#[cfg(test)]
mod configured_w104_destination_controls;
