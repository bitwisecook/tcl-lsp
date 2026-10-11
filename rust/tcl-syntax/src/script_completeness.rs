// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Readonly reexports; the Lexer retains grammar and original source ownership.

pub use tcl_lexer::{
    LexerConfig, ScriptCompletenessUnavailable, SourceImage, script_image_is_complete,
};
