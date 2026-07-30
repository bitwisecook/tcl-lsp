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

//! Shared CLI plumbing for the native `tcl` / `f5` Rust CLIs.
//!
//! Provides input-document resolution (files / directories / inline
//! `--source` / stdin), the
//! source-combining rule the verbs share, output writers (with faithful
//! tab expansion), and the per-dialect [`CommandRegistry`] cache.
//!
//! Behaviour here is asserted against the captured golden output, so the
//! discovery order, supported-extension set, skip-directory set, and the
//! `"\n\n".join(rstrip)` combine rule all match the captured behaviour exactly.

#![forbid(unsafe_code)]

pub mod chrome;
pub mod difflib;
mod highlight;
mod input;
mod output;
pub mod prompt;
pub mod secret_input;

pub use highlight::{highlight_ansi, highlight_html};
pub use input::{
    CliError, InputDocument, combine_sources, combined_effective_dialect, read_input_documents,
};
pub use output::{
    OutputTarget, ensure_ascii, expand_tabs, resolve_use_colour, write_binary_output,
    write_highlighted_output, write_text_output,
};
// The per-dialect registry cache now lives in `tcl-registry` so every
// downstream tool (CLI, compiler explorer, …) shares one cache. Re-exported
// here to preserve the existing `tcl_cli_support::registry_for_dialect` path.
pub use tcl_registry::registry_for_dialect;

/// Result alias for fallible CLI-support operations.
pub type Result<T> = std::result::Result<T, CliError>;
