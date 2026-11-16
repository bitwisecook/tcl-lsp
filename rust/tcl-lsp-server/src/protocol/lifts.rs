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

//! Conversions from core results into LSP types.

use tcl_lsp_core::completion::{
    CompletionItem as CoreCompletionItem, CompletionKind as CoreCompletionKind,
};
use tcl_lsp_core::definition::LspRange as CoreLspRange;
use tcl_lsp_core::document_symbols::SymbolKind as CoreSymbolKind;
use tcl_lsp_core::folding::FoldKind;
use tcl_lsp_core::hover::{Hover as CoreHover, HoverKind as CoreHoverKind};
use tcl_lsp_core::signature_help::{
    ParameterInformation as CoreParameterInformation, SignatureHelp as CoreSignatureHelp,
    SignatureInformation as CoreSignatureInformation,
};
use tcl_lsp_core::{document_symbols as core_symbols, selection_range as core_selection_range};
use tower_lsp_server::ls_types::{
    CompletionItem, CompletionItemKind, DocumentSymbol, Documentation, FoldingRange,
    FoldingRangeKind, Hover, HoverContents, MarkupContent, MarkupKind, ParameterInformation,
    ParameterLabel, Position, Range, SelectionRange, SignatureHelp, SignatureInformation,
    SymbolKind,
};

fn lift_completion_kind(k: CoreCompletionKind) -> CompletionItemKind {
    match k {
        CoreCompletionKind::Variable => CompletionItemKind::VARIABLE,
        CoreCompletionKind::Function => CompletionItemKind::FUNCTION,
        CoreCompletionKind::EnumValue => CompletionItemKind::ENUM_MEMBER,
        CoreCompletionKind::Snippet => CompletionItemKind::SNIPPET,
    }
}

pub(in super::super) fn lift_completion_item(
    item: CoreCompletionItem,
    line: u32,
) -> CompletionItem {
    // An explicit single-line replacement edit (var / switch / array) — the
    // editor applies it verbatim so the `$`/`-` prefix isn't dropped/duplicated.
    let text_edit = item.text_edit.map(|e| {
        tower_lsp_server::ls_types::CompletionTextEdit::Edit(tower_lsp_server::ls_types::TextEdit {
            range: tower_lsp_server::ls_types::Range {
                start: tower_lsp_server::ls_types::Position::new(line, e.start_char),
                end: tower_lsp_server::ls_types::Position::new(line, e.end_char),
            },
            new_text: e.new_text,
        })
    });
    let documentation = item
        .documentation
        .map(tower_lsp_server::ls_types::Documentation::String);
    CompletionItem {
        label: item.label,
        kind: Some(lift_completion_kind(item.kind)),
        insert_text: Some(item.insert_text),
        detail: item.detail,
        documentation,
        sort_text: item.sort_text,
        // Snippet items carry VS Code tabstop syntax and
        // filter on their `tcl-…` prefix.
        insert_text_format: item
            .is_snippet
            .then_some(tower_lsp_server::ls_types::InsertTextFormat::SNIPPET),
        filter_text: item.filter_text,
        text_edit,
        ..CompletionItem::default()
    }
}

/// Materialise the `tcl-lsp-core::selection_range` flat-vector
/// representation into an `ls_types` `SelectionRange` tree.
///
/// The chain in `core` is innermost first, with each link
/// pointing at its parent index; the LSP wire shape recurses
/// outward via `parent: Option<Box<SelectionRange>>`.
pub(in super::super) fn materialise_selection_range(
    chain: &[core_selection_range::SelectionRange],
) -> Option<SelectionRange> {
    if chain.is_empty() {
        return None;
    }
    // Build LSP records bottom-up so the parent box for index
    // `i` is already constructed when we wrap index `i-1`.
    let mut wrapped: Vec<Option<SelectionRange>> = chain.iter().map(|_| None).collect();
    // Process in reverse (outermost first) so each link's
    // parent — at a higher index — is already populated.
    for (i, link) in chain.iter().enumerate().rev() {
        let parent_box = link
            .parent_index
            .and_then(|p| wrapped[p].clone().map(Box::new));
        wrapped[i] = Some(SelectionRange {
            range: lift_lsp_range(link.range),
            parent: parent_box,
        });
    }
    wrapped.into_iter().next().flatten()
}

pub(in super::super) fn lift_lsp_range(r: CoreLspRange) -> Range {
    Range {
        start: Position {
            line: r.start_line,
            character: r.start_character,
        },
        end: Position {
            line: r.end_line,
            character: r.end_character,
        },
    }
}

/// Lift a [`tcl_bigip::Range`] (UTF-16 columns, **inclusive** end — the last
/// covered code unit) to an LSP [`Range`] (exclusive end). BIG-IP object
/// stanzas and reference spans are single-line tokens, so the exclusive end
/// column is `end.character + 1` (the same convention
/// [`lift_config_diagnostic`] uses for validator ranges).
pub(in super::super) fn lift_bigip_range(r: tcl_bigip::Range) -> Range {
    Range {
        start: Position {
            line: r.start.line,
            character: r.start.character,
        },
        end: Position {
            line: r.end.line,
            character: r.end.character + 1,
        },
    }
}

/// The wire severity for a shared [`tcl_core_types::Severity`], as every
/// lift publishes it: `Hint` and `Suggestion` both render as `HINT`.
pub(in super::super) fn lsp_severity(
    severity: tcl_core_types::Severity,
) -> tower_lsp_server::ls_types::DiagnosticSeverity {
    use tower_lsp_server::ls_types::DiagnosticSeverity;
    match severity {
        tcl_core_types::Severity::Error => DiagnosticSeverity::ERROR,
        tcl_core_types::Severity::Warning => DiagnosticSeverity::WARNING,
        tcl_core_types::Severity::Info => DiagnosticSeverity::INFORMATION,
        tcl_core_types::Severity::Hint | tcl_core_types::Severity::Suggestion => {
            DiagnosticSeverity::HINT
        }
    }
}

pub(in super::super) fn lift_span(
    source: &str,
    line_index: &tcl_lexer::LineIndex,
    span: tcl_lexer::Span,
) -> Range {
    let start = line_index.position_at_utf16(span.start(), source);
    let end = line_index.position_at_utf16(span.end(), source);
    Range {
        start: Position {
            line: start.line,
            character: start.character.get(),
        },
        end: Position {
            line: end.line,
            character: end.character.get(),
        },
    }
}

fn lift_parameter_information(p: CoreParameterInformation) -> ParameterInformation {
    ParameterInformation {
        label: ParameterLabel::Simple(p.label),
        documentation: None,
    }
}

fn lift_signature_information(s: CoreSignatureInformation) -> SignatureInformation {
    SignatureInformation {
        label: s.label,
        documentation: s.documentation.map(|d| {
            Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: d,
            })
        }),
        parameters: Some(
            s.parameters
                .into_iter()
                .map(lift_parameter_information)
                .collect(),
        ),
        active_parameter: None,
    }
}

pub(in super::super) fn lift_signature_help(h: CoreSignatureHelp) -> SignatureHelp {
    SignatureHelp {
        signatures: h
            .signatures
            .into_iter()
            .map(lift_signature_information)
            .collect(),
        active_signature: Some(h.active_signature),
        active_parameter: Some(h.active_parameter),
    }
}

pub(in super::super) fn lift_hover(h: CoreHover) -> Hover {
    let kind = match h.kind {
        CoreHoverKind::Markdown => MarkupKind::Markdown,
    };
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind,
            value: h.value,
        }),
        range: None,
    }
}

fn lift_line_range(r: core_symbols::LineRange) -> Range {
    Range {
        start: Position {
            line: r.start_line,
            character: r.start_character,
        },
        end: Position {
            line: r.end_line,
            character: r.end_character,
        },
    }
}

pub(in super::super) fn lift_symbol_kind(k: CoreSymbolKind) -> SymbolKind {
    match k {
        // A tcltest case has no dedicated LSP kind; it lists as a function.
        CoreSymbolKind::Function | CoreSymbolKind::Test => SymbolKind::FUNCTION,
        CoreSymbolKind::Method => SymbolKind::METHOD,
        CoreSymbolKind::Class => SymbolKind::CLASS,
        CoreSymbolKind::Property => SymbolKind::PROPERTY,
        CoreSymbolKind::Constructor => SymbolKind::CONSTRUCTOR,
        CoreSymbolKind::Namespace => SymbolKind::NAMESPACE,
        CoreSymbolKind::Variable => SymbolKind::VARIABLE,
        // tcltest constraints / custom-match modes.
        CoreSymbolKind::Constant => SymbolKind::CONSTANT,
        CoreSymbolKind::Operator => SymbolKind::OPERATOR,
        CoreSymbolKind::Module => SymbolKind::MODULE,
        CoreSymbolKind::Event => SymbolKind::EVENT,
    }
}

pub(in super::super) fn lift_document_symbol(s: core_symbols::DocumentSymbol) -> DocumentSymbol {
    let children = if s.children.is_empty() {
        None
    } else {
        Some(s.children.into_iter().map(lift_document_symbol).collect())
    };
    #[allow(deprecated)] // `deprecated` is required by lsp-types but unused.
    DocumentSymbol {
        name: s.name,
        detail: s.detail,
        kind: lift_symbol_kind(s.kind),
        tags: None,
        deprecated: None,
        range: lift_line_range(s.range),
        selection_range: lift_line_range(s.selection_range),
        children,
    }
}

pub(in super::super) fn lift_folding_range(r: tcl_lsp_core::folding::FoldingRange) -> FoldingRange {
    FoldingRange {
        start_line: r.start_line,
        start_character: None,
        end_line: r.end_line,
        end_character: None,
        kind: Some(match r.kind {
            FoldKind::Region => FoldingRangeKind::Region,
            FoldKind::Comment => FoldingRangeKind::Comment,
        }),
        collapsed_text: None,
    }
}
