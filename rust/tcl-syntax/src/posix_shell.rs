// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Literal text rendering shared by jq-compatible output and shell source.

/// Force single quotes around text, closing and reopening around each quote.
/// This pure rendering recipe preserves NUL for descriptive formats such as
/// jq `@sh`. Executable source consumers must independently admit their target,
/// input representation and NUL-free word; rendering grants no execution.
#[must_use]
pub fn quote_literal(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}
