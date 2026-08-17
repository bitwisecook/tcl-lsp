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

//! Miscellaneous verbs: `find-legacy`.
//!
//! Runs the analyser over the combined input source, keeps the diagnostics
//! whose codes have a
//! known mechanical modernisation (`_CONVERTIBLE_CODES`), and reports them with
//! the matching conversion hint (`_CONVERSION_MAP`).
//!
//! This verb only *reports*; it never rewrites source. The diagnostic firing
//! conditions, spans, and messages are asserted against the captured golden output —
//! only the emission *order* may differ (Rust sorts diagnostics by source
//! position; emitted in pass order).
//!
//! [`CONVERTIBLE_CODES`]/[`conversion_for`] are `pub` (re-exported at the
//! crate root) so `tcl-mcp`'s `find-legacy` MCP tool can share this table
//! instead of hand-duplicating the 6 codes and their hint strings in
//! `tcl-mcp/diagnostics.json`.

use serde::Serialize;
use tcl_cli_support::{
    OutputTarget, combine_sources, combined_effective_dialect, ensure_ascii, read_input_documents,
};
use tcl_compiler::analyser::Analyser;
use tcl_lexer::LineIndex;

use crate::cli::InputArgs;

/// Diagnostic codes with a known mechanical modernisation.
pub const CONVERTIBLE_CODES: [&str; 6] = ["W100", "W104", "W110", "W304", "IRULE2001", "IRULE5001"];

/// The modernisation hint shown per code. Codes not
/// in this table fall back to `"modernise"` (the default;
/// every convertible code is present, so the fallback is unreachable in practice).
#[must_use]
pub fn conversion_for(code: &str) -> &'static str {
    match code {
        "W100" => "Unbraced expr -> braced expr",
        "W104" => "String concat for lists -> lappend",
        "W110" => "== / != for strings -> eq / ne",
        "W304" => "Missing -- option terminator -> add --",
        "IRULE2001" => "Deprecated matchclass -> class match",
        "IRULE5001" => "Ungated log in hot event -> add debug gating",
        _ => "modernise",
    }
}

/// One reported legacy pattern. Fields are emitted in declaration order
/// (`code, line, column, message, conversion`); `serde_json::to_string_pretty`
/// preserves that order, so the JSON layout is stable.
#[derive(Serialize)]
struct LegacyIssue {
    code: String,
    line: u32,
    column: u32,
    message: String,
    conversion: String,
}

/// The `find-legacy --json` payload (`count, dialect, issues`).
#[derive(Serialize)]
struct LegacyPayload {
    count: usize,
    dialect: String,
    issues: Vec<LegacyIssue>,
}

/// `tcl find-legacy` — report legacy patterns eligible for modernisation.
pub fn run_find_legacy(input: &InputArgs, json: bool) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let dialect = combined_effective_dialect(&documents, input.dialect_profile()?);
    let source = combine_sources(&documents);

    let result = Analyser::new()
        .with_pack_overlay(tcl_cli_support::spec_pack_key(dialect.name))
        .analyse(&source, dialect.name);
    let line_index = LineIndex::new(&source);

    let issues: Vec<LegacyIssue> = result
        .diagnostics
        .iter()
        .filter(|d| CONVERTIBLE_CODES.contains(&d.code.as_str()))
        .map(|d| {
            let pos = line_index.position_at_utf16(d.span.start(), &source);
            LegacyIssue {
                code: d.code.to_string(),
                line: pos.line + 1,
                column: pos.character.get() + 1,
                message: d.message.clone(),
                conversion: conversion_for(d.code.as_str()).to_owned(),
            }
        })
        .collect();

    let target = OutputTarget::from_arg(input.output.as_deref());

    if json {
        let payload = LegacyPayload {
            count: issues.len(),
            dialect: dialect.name.to_owned(),
            issues,
        };
        let rendered = ensure_ascii(&serde_json::to_string_pretty(&payload)?);
        tcl_cli_support::write_text_output(&target, &rendered)?;
        return Ok(0);
    }

    if issues.is_empty() {
        tcl_cli_support::write_text_output(&target, "no legacy patterns detected")?;
        return Ok(0);
    }

    let mut lines = vec![format!("legacy patterns: {}", issues.len())];
    for issue in &issues {
        lines.push(format!(
            "  {} line {}:{} {}",
            issue.code, issue.line, issue.column, issue.message
        ));
        lines.push(format!("    conversion: {}", issue.conversion));
    }
    tcl_cli_support::write_text_output(&target, &lines.join("\n"))?;
    Ok(0)
}
