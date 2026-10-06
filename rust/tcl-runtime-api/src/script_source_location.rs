// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Runtime script-object locations captured at genuine source entry.

/// The location retained by a script value, independent of the variable frame
/// in which the value is later evaluated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptSourceLocation<T = String> {
    /// Actual source filename. Empty for an evaluation without a filename.
    pub file: T,
    /// One-based source line at which the script value begins.
    pub line: u32,
}
