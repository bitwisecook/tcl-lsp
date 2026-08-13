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
/// The command registry for `dialect`, **with the shipped `.tclspec`
/// loadables installed**.
///
/// The per-dialect registry cache lives in `tcl-registry` so every downstream
/// tool shares one cache; this is the same cache, entered through the one door
/// that also carries the bundled packs. It has to be: the EDA vendor libraries
/// are loadables now (`docs/design/spec-packs.md`), so a `tcl lookup --dialect
/// xilinx-eda-tcl synth_design` reaches its spec only if the CLI parses
/// `specs/synth_design`'s pack the way the language server does. With no
/// bundled directory present this is byte-for-byte
/// [`tcl_registry::registry_for_dialect`], down to the same cached instance.
///
/// An owning handle, because the pack-carrying entry it names is refcounted
/// and retirable. A CLI verb resolves its packs once and runs to exit, so in
/// practice the handle simply lives as long as the process — but it is the
/// binding that keeps it alive, not the type.
#[must_use]
pub fn registry_for_dialect(
    dialect: &str,
) -> std::sync::Arc<tcl_registry::registry::CommandRegistry> {
    tcl_spectcl::bundled::registry_for_dialect(dialect)
}

/// The shipped loadables' content identity, for
/// [`Analyser::with_pack_overlay`](tcl_compiler::analyser::Analyser::with_pack_overlay).
///
/// The analyser resolves its own registry from its `DialectProfile`, so it
/// needs this number to reach the same pack-carrying entry
/// [`registry_for_dialect`] returns. Without it an EDA document's every
/// command reads as unknown, because the vendor libraries are `.tclspec` packs
/// now and nothing compiled-in answers for them.
///
/// Call [`registry_for_dialect`] for the same dialect first (every CLI verb
/// already does): that is what *builds* the entry this key names, and the
/// analyser can only look it up.
#[must_use]
pub fn spec_pack_key() -> u64 {
    tcl_spectcl::bundled::packs().key
}

/// Result alias for fallible CLI-support operations.
pub type Result<T> = std::result::Result<T, CliError>;
