// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Diagnostics derived from the Unicode source text itself.
//!
//! These checks belong below the LSP/CLI/MCP adapters: every consumer of the
//! analyser must see the same security verdict, and non-Tcl adapters can reuse
//! the pure producer without copying the Unicode table or the message.

use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;

use tcl_core_types::{DiagCode, Severity};
use tcl_lexer::{LineIndex, Span};

use super::confusables_table::bidi_control_name;
use super::types::Diagnostic;

/// W305 — bidirectional formatting controls anywhere in `source`.
///
/// This is a whole-source scan because a Trojan Source control changes how the
/// surrounding text renders even in a comment or between tokens. Ordinary
/// right-to-left content and directional marks are not in the canonical table
/// and therefore remain silent.
#[must_use]
pub fn bidi_control_diagnostics(source: &str) -> Vec<Diagnostic> {
    source
        .char_indices()
        .filter_map(|(offset, ch)| {
            let name = bidi_control_name(ch)?;
            let start = u32::try_from(offset).unwrap_or(u32::MAX);
            let len = u32::try_from(ch.len_utf8()).unwrap_or(0);
            Some(Diagnostic {
                code: DiagCode::W305,
                span: Span::new(start, start.saturating_add(len)),
                message: format!(
                    "Bidirectional formatting control U+{:04X} {name} — this makes the surrounding \
                     source render in a different order from the one it is parsed and executed in, \
                     so a reviewer can approve code that does something other than what their editor \
                     showed them (Trojan Source, CVE-2021-42574). Remove it, or write the text with \
                     an escape (`\\u{:04X}`) if the character is genuinely part of the data.",
                    ch as u32, ch as u32,
                ),
                severity: Severity::Error,
                fixes: Vec::new(),
            })
        })
        .collect()
}

/// Apply established code and per-line suppression data to W305 findings.
///
/// `suppressed_lines` already maps a preceding `# noqa` comment onto the lines
/// occupied by its following command. This function only consumes that map; it
/// does not add a directive shape or turn a line-local directive into a
/// file-wide one.
pub(super) fn bidi_control_diagnostics_with_suppressions(
    source: &str,
    disabled: &HashSet<String>,
    suppressed_lines: &HashMap<i32, HashSet<String>>,
) -> Vec<Diagnostic> {
    let line_index = LineIndex::new(source);
    bidi_control_diagnostics(source)
        .into_iter()
        .filter(|diagnostic| {
            let code = diagnostic.code.as_str();
            if disabled.contains("*") || disabled.contains(code) {
                return false;
            }
            let line = i32::try_from(line_index.position_at(diagnostic.span.start()).line)
                .unwrap_or(i32::MAX);
            !suppressed_lines
                .get(&line)
                .is_some_and(|codes| codes.contains("*") || codes.contains(code))
        })
        .collect()
}

/// W305 diagnostics filtered through editor and source suppression settings.
///
/// Non-Tcl document adapters (BIG-IP configuration and iApp APL) do not run
/// the Tcl analyser, but they still use the same `# tcl-lsp: disable` and
/// preceding `# noqa` conventions. Keeping that filtering beside the producer
/// prevents each adapter from inventing a subtly different rule.
#[must_use]
pub fn filtered_bidi_control_diagnostics<S: BuildHasher>(
    source: &str,
    user_disabled: &HashSet<String, S>,
) -> Vec<Diagnostic> {
    let mut disabled = super::utils::parse_file_suppression(source);
    disabled.extend(user_disabled.iter().cloned());
    let suppressed_lines = super::utils::parse_noqa_line_suppressions(source);
    bidi_control_diagnostics_with_suppressions(source, &disabled, &suppressed_lines)
}
