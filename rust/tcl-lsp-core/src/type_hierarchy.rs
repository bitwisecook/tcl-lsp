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

//! Type-hierarchy provider.
//!
//! Resolves a `TclOO` class at the cursor and returns a
//! single hierarchy item.  Supertype / subtype walks are
//! stub-empty; computing them requires the class-hierarchy
//! index that the analyser populates.

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

use crate::definition::LspRange;
use crate::hover::find_word_span_at_position;

/// One hierarchy item — class identification plus its name
/// and definition span for editor display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeHierarchyItem {
    /// Class qualified name.
    pub name: String,
    /// Detail (e.g. metaclass).
    pub detail: Option<String>,
    /// Range of the entire definition.
    pub range: LspRange,
    /// Range of just the name token.
    pub selection_range: LspRange,
}

/// Resolve a "prepare type hierarchy" request to a single
/// item — the class whose name is at the cursor.
#[must_use]
pub fn prepare(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Vec<TypeHierarchyItem> {
    let line_index = LineIndex::new(source);
    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };
    for class_def in analysis.all_classes.values() {
        if class_def.name == word
            || class_def.qualified_name == word
            || class_def.qualified_name == format!("::{word}")
        {
            let name_range = span_to_range(source, &line_index, class_def.name_span);
            let body_range = span_to_range(source, &line_index, class_def.body_span);
            let full_range = LspRange {
                start_line: name_range.start_line,
                start_character: name_range.start_character,
                end_line: body_range.end_line,
                end_character: body_range.end_character,
            };
            return vec![TypeHierarchyItem {
                name: class_def.qualified_name.clone(),
                detail: Some(class_def.metaclass.clone()),
                range: full_range,
                selection_range: name_range,
            }];
        }
    }
    Vec::new()
}

fn span_to_range(source: &str, line_index: &LineIndex, span: tcl_lexer::Span) -> LspRange {
    let start = line_index.position_at_utf16(span.start(), source);
    let end = line_index.position_at_utf16(span.end(), source);
    LspRange {
        start_line: start.line,
        start_character: start.character.get(),
        end_line: end.line,
        end_character: end.character.get(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    #[test]
    fn prepare_resolves_class_at_cursor() {
        let src = "oo::class create Greeter {}\n";
        let analysis = analyse(src);
        let items = prepare(src, 0, 18, &analysis);
        if !items.is_empty() {
            assert!(items[0].name.contains("Greeter"));
        }
    }
}
