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

//! Method completion on a registered instance the walk proved: a Tk widget
//! path or a handle variable bound by its constructor. The instance's own
//! factory descriptor supplies the methods, so this holds under a pinned
//! release as well as in the lenient environment.

use super::{CompletionItem, CompletionKind, FilteredCandidates, filter_candidates};
use tcl_compiler::analyser::AnalysisResult;

/// The factory's methods for the registered-instance call whose method word
/// the cursor is on, filtered by `partial`.
pub(super) fn registered_instance_method_items(
    analysis: &AnalysisResult,
    source: &str,
    cursor: u32,
    partial: &str,
    profile: &'static tcl_dialect::DialectProfile,
) -> Option<Vec<CompletionItem>> {
    let config = analysis.body_lexer_config?;
    let structure =
        crate::source_structure::SourceStructure::capture(source, Some(analysis), config)?;
    let command = structure
        .commands
        .iter()
        .filter(|command| {
            command
                .argv
                .first()
                .is_some_and(|head| head.span.start() < cursor)
        })
        .max_by_key(|command| command.argv[0].span.start())?;
    let head = command.argv.first()?;
    let words = crate::registered_instance::words_at(source, analysis, head.span.start())?;
    let in_method_word = match words.argument_word(0) {
        Some(selector) if selector.word_span().start() <= cursor => {
            cursor <= selector.word_span().end()
        }
        _ => crate::original_oo::instance_completion::completion_gap(
            source,
            head.span.end(),
            cursor,
            config,
        ),
    };
    if !in_method_word {
        return None;
    }
    let (spec, methods) = crate::registered_instance::methods(analysis, &words, profile)?;
    let mut items: Vec<CompletionItem> = methods
        .into_iter()
        .filter_map(|method| {
            let insert_text = super::methods::source_method_name_word(analysis, method.name)?;
            Some(CompletionItem {
                label: method.name.to_owned(),
                insert_text,
                kind: CompletionKind::Function,
                detail: Some(format!("method — {}", spec.name)),
                documentation: (!method.detail.is_empty()).then(|| method.detail.to_owned()),
                ..CompletionItem::default()
            })
        })
        .collect();
    items.sort_by(|a, b| a.label.cmp(&b.label));
    let FilteredCandidates { candidates, fuzzy } =
        filter_candidates(partial, items, |item| item.label.as_str());
    (!fuzzy && !candidates.is_empty()).then_some(candidates)
}
