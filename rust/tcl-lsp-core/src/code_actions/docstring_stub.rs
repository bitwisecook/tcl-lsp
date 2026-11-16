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

//! The action that inserts a generated docstring stub for a procedure.

use super::{ActionKind, CodeAction};
use crate::definition::LspRange;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

// Generate docstring (source action).

pub(super) fn docstring_actions(
    source: &str,
    range: LspRange,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
    docstring_style: crate::formatting::DocstringStyle,
) -> Vec<CodeAction> {
    // `None` — "do not generate or reformat docstrings" — offers no action
    // at all, matching the setting's documented (and default) meaning.
    if docstring_style == crate::formatting::DocstringStyle::None {
        return Vec::new();
    }
    let mut out = Vec::new();
    let Some(declarations) = crate::procedure_symbol::declarations(source, analysis) else {
        return out;
    };
    for proc_def in declarations {
        let decl = line_index.position_at_utf16(proc_def.name_span.start(), source);
        if decl.line != range.start_line {
            continue;
        }
        // Skip procs that already carry a doc-comment.
        if !proc_def.doc.is_empty() {
            continue;
        }
        let edit = match docstring_style {
            crate::formatting::DocstringStyle::Body => {
                let Some(content) = original_literal_procedure_body(source, analysis, proc_def)
                else {
                    continue;
                };
                body_docstring_edit(source, line_index, content)
            }
            // `Preceding` (and unreachable `None`, filtered above).
            _ => preceding_docstring_edit(decl.line),
        };
        // The DOXYGEN stub (a `# @brief` placeholder plus one `# @param`
        // line per parameter) is rendered by the shared docstring generator.
        let indent = edit.indent;
        let doc = crate::formatting::generate_stub_for_proc(
            proc_def,
            crate::formatting::DocstringTagStyle::Doxygen,
            false,
            '.',
            70,
            &indent,
        );
        out.push(CodeAction {
            title: format!("Generate docstring for '{}'", proc_def.name),
            edits: vec![crate::rename::TextEdit {
                range: edit.range,
                new_text: format!("{}{doc}{}", edit.prefix, edit.suffix),
            }],
            kind: ActionKind::GenerateDocstring,
            command: None,
            data_group_definition: None,
            disabled: None,
        });
    }
    out
}

// Editing inside a body needs its actual grouped word. The declaration
// inventory supplies the selected ProcDef; body text/ranges alone cannot turn
// a computed script into writable original source.
fn original_literal_procedure_body(
    source: &str,
    analysis: &AnalysisResult,
    proc_def: &tcl_compiler::analyser::ProcDef,
) -> Option<tcl_lexer::Span> {
    let config = analysis.body_lexer_config?;
    let image = tcl_lexer::SourceImage::document(source);
    let input = analysis.retained_command_realm().and_then(|realm| {
        realm.original_written_name_input_at_span_in_source(&image, proc_def.body_span, config)
    })?;
    let key = input.original_word_key()?;
    let word = key.original_word();
    (word.image() == &image
        && word.config() == config
        && word.group().kind == tcl_lexer::WordKind::Braced
        && !word.group().expand
        && word
            .tokens()
            .first()
            .is_some_and(|token| token.span == proc_def.body_span))
    .then_some(())?;
    word.content_span().ok()
}

/// Where + how to insert a generated docstring stub, and the indent its
/// lines should carry.
struct DocstringInsertion {
    pub(super) range: LspRange,
    /// Text emitted before the rendered stub (e.g. nothing, or a leading
    /// newline when inserting mid-line).
    prefix: String,
    /// Text emitted after the rendered stub — a newline separating it from
    /// what follows, omitted when the insertion point is already followed
    /// by one (so `Body` placement never leaves a spurious blank line).
    suffix: String,
    indent: String,
}

/// [`crate::formatting::DocstringStyle::Preceding`]: insert a zero-indent
/// comment block on its own line directly above the `proc` declaration.
fn preceding_docstring_edit(decl_line: u32) -> DocstringInsertion {
    DocstringInsertion {
        range: LspRange {
            start_line: decl_line,
            start_character: 0,
            end_line: decl_line,
            end_character: 0,
        },
        prefix: String::new(),
        suffix: "\n".to_owned(),
        indent: String::new(),
    }
}

/// [`crate::formatting::DocstringStyle::Body`]: insert the comment block as
/// the first line inside the `proc` body, indented to match the body's
/// existing content (or four spaces — the formatter's default indent size —
/// when the body has no other indented line to match, e.g. an empty or
/// single-line proc).
fn body_docstring_edit(
    source: &str,
    line_index: &LineIndex,
    content: tcl_lexer::Span,
) -> DocstringInsertion {
    let body_start = content.start();
    let body_start_idx = body_start as usize;
    let body_end_idx = content.end() as usize;
    let body_text = source.get(body_start_idx..body_end_idx).unwrap_or("");
    // The authenticated content starts after its genuine opening delimiter.
    // Read indentation from the first new content line; a single-line body
    // uses the formatter's default below.
    let indent = body_text
        .lines()
        .skip(1)
        .map(str::trim_start)
        .zip(body_text.lines().skip(1))
        .find(|(trimmed, _)| !trimmed.is_empty())
        .map_or_else(
            || "    ".to_owned(),
            |(trimmed, line)| line[..line.len() - trimmed.len()].to_owned(),
        );
    // The body already starts with its own newline (the common multi-line
    // shape) — reuse it rather than inserting a second one, which would
    // leave a blank line between the stub and the body's first statement.
    let suffix = if body_text.starts_with('\n') {
        String::new()
    } else {
        "\n".to_owned()
    };
    let pos = line_index.position_at_utf16(body_start, source);
    DocstringInsertion {
        range: LspRange {
            start_line: pos.line,
            start_character: pos.character.get(),
            end_line: pos.line,
            end_character: pos.character.get(),
        },
        prefix: "\n".to_owned(),
        suffix,
        indent,
    }
}
