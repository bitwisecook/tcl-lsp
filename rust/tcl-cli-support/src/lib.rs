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
pub mod spec_import;

pub use highlight::{highlight_ansi, highlight_html};
pub use input::{
    CliError, InputDocument, combine_sources, combined_effective_dialect, read_input_documents,
};
pub use output::{
    OutputTarget, ensure_ascii, expand_tabs, resolve_use_colour, write_binary_output,
    write_highlighted_output, write_text_output,
};
/// The command registry for `dialect`, with project, user, and shipped
/// `.tclspec` loadables installed.
///
/// A CLI invocation treats its current directory as its workspace root. That
/// gives a project the same automatic `.tcl-lsp/` and `tclpkg.tcl`-adjacent
/// discovery the language server performs, while retaining the user and
/// bundled tiers. The set is loaded once because a CLI process has one stable
/// working directory and exits after its verb completes.
///
/// An owning handle, because the pack-carrying entry it names is refcounted
/// and retirable. A CLI verb resolves its packs once and runs to exit, so in
/// practice the handle simply lives as long as the process — but it is the
/// binding that keeps it alive, not the type.
#[must_use]
pub fn registry_for_dialect(
    dialect: &str,
) -> std::sync::Arc<tcl_registry::registry::CommandRegistry> {
    tcl_spectcl::install::registry_for_dialect_with_packs(dialect, cli_packs())
}

/// The discovered loadables' content identity, for
/// [`Analyser::with_pack_overlay`](tcl_compiler::analyser::Analyser::with_pack_overlay).
///
/// The analyser resolves its own registry from its `DialectProfile`, so it
/// needs this number to reach the same pack-carrying entry
/// [`registry_for_dialect`] returns. Without it an EDA document's every
/// command reads as unknown, because the vendor libraries are `.tclspec` packs
/// now and nothing compiled-in answers for them.
///
/// This builds the dialect overlay before returning its key. The analyser's
/// overlay lookup is intentionally read-only, so returning a key without
/// installing the corresponding `(profile, key)` entry would silently fall
/// back to the plain profile.
#[must_use]
pub fn spec_pack_key(dialect: &str) -> u64 {
    let packs = cli_packs();
    let _registry = tcl_spectcl::install::registry_for_dialect_with_packs(dialect, packs);
    packs.key
}

/// Discover the CLI process's workspace once, rooted at its current directory.
fn cli_packs() -> &'static tcl_spectcl::PackSet {
    static PACKS: std::sync::OnceLock<tcl_spectcl::PackSet> = std::sync::OnceLock::new();
    PACKS.get_or_init(|| {
        let workspace_roots = std::env::current_dir().into_iter().collect();
        let files = tcl_spectcl::discover(&tcl_spectcl::DiscoveryOptions {
            workspace_roots,
            ..tcl_spectcl::DiscoveryOptions::default()
        });
        let packs = tcl_spectcl::bundled::load_discovered(&files);
        // Pack commands and their typed behaviour are one publication. Hook
        // dispatch consults the process-wide plan, while explorer consumers
        // consult the active pack set; publish both before exposing `packs`.
        tcl_spectcl::hooks::publish(&packs);
        tcl_spectcl::hooks::ensure_thread_host();
        tcl_spectcl::bundled::set_active(Some(std::sync::Arc::new(packs.clone())));
        packs
    })
}

/// Result alias for fallible CLI-support operations.
pub type Result<T> = std::result::Result<T, CliError>;
