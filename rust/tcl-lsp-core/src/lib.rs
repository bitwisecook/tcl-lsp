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

//! Pure Rust LSP feature providers for tcl-lsp.
//!
//! This crate owns the algorithmic LSP feature surface — folding,
//! document symbols, hover, diagnostic projection, completion,
//! references, rename, and semantic tokens. It
//! contains no `pyo3` dependency and no binding-compat shims; the
//! binding crate wraps these providers for its callers, and
//! the `tcl-lsp-server` binary links against this crate
//! directly over the LSP protocol.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

/// Cap on [`tcl_compiler::analyser::Scope`]-tree walking depth shared by
/// every LSP feature provider that recurses over `scope.children`
/// (document symbols, go-to-definition, the symbol graph, inlay hints,
/// the inline-variable refactor). Mirrors the compiler analyser's own
/// `MAX_BODY_DEPTH`: since a `Scope` node is created only for a
/// `namespace`/`proc`/`method` body, and the analyser already caps body
/// nesting at that same value, the tree these walkers see can never
/// exceed it in practice — this cap is defence-in-depth against a scope
/// tree built or received some other way, and keeps every consumer from
/// needing its own copy of the same constant. See
/// `docs/design/compiler/recursive-descent-depth-limits.md`.
pub(crate) const MAX_SCOPE_WALK_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

pub mod bigip;
pub mod call_hierarchy;
mod caller_frame;
pub mod code_actions;
pub mod code_lens;
pub mod completion;
pub mod declaration;
pub mod definition;
pub mod document_links;
pub mod document_symbols;
mod executable_regions;
pub mod expr_context;
pub mod file_ops;
pub mod folding;
pub mod formatting;
pub mod graphs;
pub mod hover;
pub mod implementation;
pub mod inert_text;
pub mod inlay_hints;
pub mod irules_context;
pub mod irules_object_refs;
pub mod linked_editing_range;
pub mod minify;
pub mod namespace_import;
pub mod namespace_rename;
pub mod namespace_symbol;
pub mod oo_body;
mod oo_dispatch;
pub mod package_resolver;
pub mod refactor;
pub mod references;
pub mod rename;
pub mod rename_safety;
pub mod selection_range;
pub mod semantic_tokens;
pub mod signature_help;
pub mod snippets;
pub mod source_decode;
pub mod source_graph;
pub mod source_style;
pub mod tcl_install;
pub mod type_definition;
pub mod type_hierarchy;
pub mod workspace_index;
pub mod workspace_symbols;

/// Crate version string.
///
/// ```
/// assert!(!tcl_lsp_core::VERSION.is_empty());
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
