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

//! XC-series translatability diagnostics for inline editor feedback.
//!
//! Walks the same IR as the translator (via [`translate_irule`]) but
//! produces ranged [`XcDiagnostic`]s for the LSP diagnostics pipeline.

use tcl_lexer::LineIndex;

use crate::model::TranslationItem;
use crate::translator::translate_irule;

/// Severity of an XC diagnostic. The XC-series codes only ever emit
/// `Hint` (`XC1xx` — translatable) or `Info` (`XC2xx` / `XC3xx` — partial /
/// untranslatable).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XcSeverity {
    /// Advisory hint (`XC1xx` — translatable).
    Hint,
    /// Informational (`XC2xx` / `XC3xx` — partial / untranslatable).
    Info,
}

/// A 0-based source position (LSP convention: `character` in UTF-16 code
/// units).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// 0-based line.
    pub line: u32,
    /// 0-based UTF-16 column.
    pub character: u32,
}

/// A source range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    /// Start position.
    pub start: Position,
    /// End position.
    pub end: Position,
}

/// One XC translatability diagnostic, ready to lift into an LSP
/// `Diagnostic`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XcDiagnostic {
    /// XC-series code (e.g. `"XC100"`).
    pub code: String,
    /// Human-readable message (`xc_description`, plus `note` after `—`).
    pub message: String,
    /// Severity (by code prefix).
    pub severity: XcSeverity,
    /// Source range.
    pub range: Range,
}

/// Severity from a diagnostic-code prefix: `XC1xx` → Hint,
/// `XC2xx`/`XC3xx` → Info (default Info).
fn severity_for_code(code: &str) -> XcSeverity {
    if code.starts_with("XC1") {
        XcSeverity::Hint
    } else {
        // XC2 / XC3 (and any other) → Info (the default).
        XcSeverity::Info
    }
}

/// Convert a [`TranslationItem`] to an [`XcDiagnostic`], resolving its byte
/// span to line/UTF-16-column positions via `line_index` / `source`.
/// Returns `None` when the item carries no range or no code.
fn item_to_diagnostic(
    item: &TranslationItem,
    line_index: &LineIndex,
    source: &str,
) -> Option<XcDiagnostic> {
    let span = item.irule_range?;
    if item.diagnostic_code.is_empty() {
        return None;
    }
    let severity = severity_for_code(&item.diagnostic_code);
    let mut message = item.xc_description.clone();
    if !item.note.is_empty() {
        message.push_str(" — ");
        message.push_str(&item.note);
    }
    let start = line_index.position_at_utf16(span.start(), source);
    let end = line_index.position_at_utf16(span.end(), source);
    Some(XcDiagnostic {
        code: item.diagnostic_code.clone(),
        message,
        severity,
        range: Range {
            start: Position {
                line: start.line,
                character: start.character.get(),
            },
            end: Position {
                line: end.line,
                character: end.character.get(),
            },
        },
    })
}

/// Analyse an iRule and return XC translatability diagnostics, suitable for
/// the LSP diagnostics pipeline.
#[must_use]
pub fn get_xc_diagnostics(source: &str) -> Vec<XcDiagnostic> {
    let result = translate_irule(source);
    let line_index = LineIndex::new(source);
    result
        .items
        .iter()
        .filter_map(|item| item_to_diagnostic(item, &line_index, source))
        .collect()
}
