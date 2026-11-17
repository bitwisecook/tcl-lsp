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

//! The workspace factory index read under a pinned release.

use super::super::super::original_definer::logical_arguments_match;
use super::super::super::state::Analyser;
use super::super::super::types::ClassFactory;
use tcl_lexer::Token;

impl Analyser {
    /// The workspace factory a head names under a pinned release: the head
    /// is absent from this document's closed command world, and the first of
    /// its ordered lookup candidates the workspace index carries is a
    /// metaclass another document publishes. A head this document may bind,
    /// an unknown command world or a computed word abstain.
    pub(super) fn workspace_class_factory_at_release(
        &self,
        args: &[String],
        arg_tokens: &[Token],
        cmd_tok: Token,
    ) -> Option<(ClassFactory, Vec<tcl_lexer::NativeWord>)> {
        let index = self.workspace_class_factories.as_ref()?;
        let call_tokens = std::iter::once(cmd_tok)
            .chain(arg_tokens.iter().copied())
            .collect::<Vec<_>>();
        let tokens = self.retained_invocation_tokens(cmd_tok.span.start(), &call_tokens)?;
        let binding = tokens.source_binding.as_ref()?;
        (binding.targets.is_empty() && binding.may_be_absent).then_some(())?;
        let input = binding.original_head_name_input(&tokens)?;
        matches!(
            input,
            crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(_)
        )
        .then_some(())?;
        let lookup = binding.original_command_lookup(&tokens, &input)?;
        let [candidates] = lookup.candidates() else {
            return None;
        };
        let recipe = lookup.policy().recipe();
        let foreign = candidates.iter().find_map(|slot| {
            index.get(&tcl_syntax::naming::native_command_source_spelling(
                recipe, slot,
            )?)
        })?;
        let image = tcl_lexer::SourceImage::document(&self.source);
        let config = self.file_lexer_config();
        let words = call_tokens
            .iter()
            .map(|token| {
                self.head_identities
                    .original_written_word_at_span_in_source(&image, token.span, config)
                    .map(|(word, _)| word)
            })
            .collect::<Option<Vec<_>>>()?;
        logical_arguments_match(&words, args, arg_tokens).then_some(())?;
        Some((
            self.foreign_factory_at_call(foreign, arg_tokens)?,
            words.into_iter().skip(1).collect(),
        ))
    }
}
