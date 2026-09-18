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
//! produces ranged [`XcDiagnostic`]s for the LSP diagnostics pipeline, and
//! converts each into the policy step's [`Finding`].

use core::str::FromStr;

use tcl_core_types::{DiagCode, Severity};
use tcl_lexer::{LineIndex, Span};
use tcl_lsp_core::diagnostic_policy::{Finding, Producer};

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
    /// XC-series code (e.g. `XC100`), as the one catalogue spells it.
    pub code: DiagCode,
    /// Human-readable message (`xc_description`, plus `note` after `—`).
    pub message: String,
    /// Severity (by code prefix).
    pub severity: XcSeverity,
    /// Source range.
    pub range: Range,
    /// The byte span `range` was resolved from.
    pub span: Span,
}

impl From<XcDiagnostic> for Finding {
    /// `Hint` and `Info` are the shared ladder's `Hint` and `Info`; a
    /// translatability note carries no fix and no payload.
    fn from(d: XcDiagnostic) -> Self {
        Self {
            code: d.code,
            span: d.span,
            severity: match d.severity {
                XcSeverity::Hint => Severity::Hint,
                XcSeverity::Info => Severity::Info,
            },
            message: d.message,
            fixes: Vec::new(),
            data: None,
            producer: Producer::Xc,
        }
    }
}

/// Severity from a diagnostic-code prefix: `XC1xx` → Hint,
/// `XC2xx`/`XC3xx` → Info (default Info).
fn severity_for_code(code: DiagCode) -> XcSeverity {
    if code.as_str().starts_with("XC1") {
        XcSeverity::Hint
    } else {
        // XC2 / XC3 (and any other) → Info (the default).
        XcSeverity::Info
    }
}

/// Convert a [`TranslationItem`] to an [`XcDiagnostic`], resolving its byte
/// span to line/UTF-16-column positions via `line_index` / `source`.
/// Returns `None` when the item carries no range, or a code the catalogue
/// does not spell (every code the translator emits is catalogued; see
/// `emitted_codes_are_catalogued`).
fn item_to_diagnostic(
    item: &TranslationItem,
    line_index: &LineIndex,
    source: &str,
) -> Option<XcDiagnostic> {
    let span = item.irule_range?;
    let code = DiagCode::from_str(&item.diagnostic_code).ok()?;
    let severity = severity_for_code(code);
    let mut message = item.xc_description.clone();
    if !item.note.is_empty() {
        message.push_str(" — ");
        message.push_str(&item.note);
    }
    let start = line_index.position_at_utf16(span.start(), source);
    let end = line_index.position_at_utf16(span.end(), source);
    Some(XcDiagnostic {
        code,
        message,
        severity,
        span,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `diagnostic_code` spelling the translator writes onto a
    /// [`TranslationItem`]. A spelling missing from the catalogue would make
    /// [`item_to_diagnostic`] drop the item, so the set is pinned here; the
    /// reverse direction (every catalogued XC code has an emission site) is
    /// `cargo xtask diag-emission-check`.
    const EMITTED: &[&str] = &[
        "XC100", "XC101", "XC102", "XC103", "XC105", "XC106", "XC107", "XC200", "XC201", "XC203",
        "XC250", "XC300", "XC301",
    ];

    #[test]
    fn emitted_codes_are_catalogued() {
        for code in EMITTED {
            assert!(DiagCode::from_str(code).is_ok(), "{code} is not catalogued");
        }
        let translator = include_str!("translator.rs");
        for code in EMITTED {
            assert!(
                translator.contains(&format!("\"{code}\"")),
                "{code} is pinned here but the translator no longer emits it"
            );
        }
    }

    #[test]
    fn a_diagnostic_converts_to_a_finding_with_its_byte_span() {
        let src = "when HTTP_REQUEST {\n    pool my_pool\n}";
        let diags = get_xc_diagnostics(src);
        let pool = diags
            .iter()
            .find(|d| d.code == DiagCode::Xc100)
            .expect("`pool` is an origin-pool mapping");
        assert_eq!(pool.severity, XcSeverity::Hint);
        assert_eq!(
            &src[pool.span.start() as usize..pool.span.end() as usize],
            "pool my_pool"
        );
        let finding = Finding::from(pool.clone());
        assert_eq!(finding.code, DiagCode::Xc100);
        assert_eq!(finding.span, pool.span);
        assert_eq!(finding.severity, Severity::Hint);
        assert_eq!(finding.producer, Producer::Xc);
        assert!(finding.fixes.is_empty() && finding.data.is_none());
        // An untranslatable event is informational.
        let l4 = get_xc_diagnostics("when CLIENT_ACCEPTED {\n    TCP::collect\n}");
        let event = l4
            .iter()
            .find(|d| d.code == DiagCode::Xc201)
            .expect("an L4 event has no XC equivalent");
        assert_eq!(Finding::from(event.clone()).severity, Severity::Info);
    }
}
