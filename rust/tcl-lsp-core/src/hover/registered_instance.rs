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

//! Method hover on a registered instance the walk proved (a Tk widget path or
//! a handle variable bound by its constructor). The instance's own factory
//! descriptor supplies the method, under the document's availability floor.

use super::Hover;
use tcl_compiler::analyser::AnalysisResult;

pub(super) fn method_hover(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
    profile: &'static tcl_dialect::DialectProfile,
) -> Option<Hover> {
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
                .is_some_and(|head| head.span.start() <= cursor)
        })
        .max_by_key(|command| command.argv[0].span.start())?;
    let head = command.argv.first()?;
    let words = crate::registered_instance::words_at(source, analysis, head.span.start())?;
    let selector = words.argument_word(0)?;
    if !(selector.word_span().start() <= cursor && cursor <= selector.word_span().end()) {
        return None;
    }
    let (spec, method) = crate::registered_instance::selected_method(analysis, &words, profile)?;
    let mut text = format!("**{} {}**", spec.name, method.name);
    if !method.detail.is_empty() {
        text.push_str("\n\n");
        text.push_str(method.detail);
    }
    Some(Hover::markdown(text))
}
