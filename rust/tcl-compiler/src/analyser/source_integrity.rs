// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Diagnostics derived from the Unicode source text itself.
//!
//! These checks belong below the LSP/CLI/MCP adapters: every consumer of the
//! analyser must see the same security verdict, and non-Tcl adapters can reuse
//! the pure producer without copying the Unicode table or the message. The
//! producer filters nothing: directives and disabled codes are the policy
//! step's.

use tcl_core_types::{DiagCode, Severity};
use tcl_lexer::Span;

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
            Some(crate::analyser::types::Diagnostic::new(
    DiagCode::W305,
    Span::new(start, start.saturating_add(len)),
    format!(
                    "Bidirectional formatting control U+{:04X} {name} — this makes the surrounding \
                     source render in a different order from the one it is parsed and executed in, \
                     so a reviewer can approve code that does something other than what their editor \
                     showed them (Trojan Source, CVE-2021-42574). Remove it, or write the text with \
                     an escape (`\\u{:04X}`) if the character is genuinely part of the data.",
                    ch as u32, ch as u32,
                ),
    Severity::Error,
))
        })
        .collect()
}
