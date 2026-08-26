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
pub mod document_floor;
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
pub mod tk_preview;
pub mod type_definition;
pub mod type_hierarchy;
pub mod vfs;
pub mod workspace_index;
pub mod workspace_symbols;

/// Resolve a document's dialect *name* to the profile the providers thread.
///
/// This is the LSP layer's dialect ingress: a provider that needs a
/// [`tcl_dialect::DialectProfile`] resolves it here rather than calling
/// `by_name` on a dialect string itself (issue #1405).
///
/// Two kinds of name lookup deliberately remain, and neither is a provider
/// re-deriving this function's answer:
///
/// - [`registry_for_dialect`](tcl_registry::registry_for_dialect) is keyed by
///   dialect *name* by design — it is the registry cache's own key, not a
///   profile resolution. Providers holding a profile use
///   [`registry_for_dialect_profile`] instead, which routes back through that
///   name.
/// - Providers reached from an [`AnalysisResult`](tcl_compiler::analyser::AnalysisResult)
///   read `analysis.dialect`, the spelling the analyser recorded for the
///   document, and resolve it through this function.
///
/// [`resolve_known`](tcl_dialect::DialectProfile::resolve_known) first,
/// `by_name` only as the sink, because of the additive set-only ingress `tk`:
/// `tk` is not a catalogue profile (it is a command surface layered over a Tcl
/// base, not a runtime), so plain `by_name("tk")` lands on the permissive
/// plain-Tcl fallback and the name "tk" is lost. `resolve_known` returns the
/// real Tk profile, which keeps both the spelling and the `TK` availability
/// bit that [`registry_for_dialect_profile`] and the analyser depend on.
#[must_use]
pub fn profile_for_dialect(name: &str) -> &'static tcl_dialect::DialectProfile {
    tcl_dialect::DialectProfile::resolve_known(name)
        .unwrap_or_else(|| tcl_dialect::DialectProfile::by_name(name))
}

/// [`profile_for_dialect`] for the inputs where an *empty* name means "this
/// build named no dialect", which is not the same as naming plain Tcl.
///
/// The compiler's `UnitBuildOptions::dialect` and the lowering entry points
/// draw that distinction: an unstated dialect selects no semantic dialect bit
/// at all, while plain Tcl is a real profile. Salsa stores the document's
/// dialect as a `String` (it is an input, so it must own its value and hash),
/// so this is the conversion at every read of one.
#[must_use]
pub fn optional_profile_for_dialect(name: &str) -> Option<&'static tcl_dialect::DialectProfile> {
    (!name.is_empty()).then(|| profile_for_dialect(name))
}

/// The command registry a document of `dialect` is analysed against.
///
/// The one place the LSP layer still goes through a dialect *name* rather than
/// handing the resolved profile straight to
/// [`tcl_registry::cache::registry_for_profile`], and the hop is load-bearing
/// rather than leftover debt. `tk` resolves three ways that do not coincide:
///
/// * `by_name("tk")` sinks to plain Tcl — the correct **registry** and lexer
///   grammar for a `wish` document, but it drops the `TK` availability bit;
/// * [`profile_for_dialect`] keeps the bit and the name, but feeding that
///   profile to `registry_for_profile` would build a *different* registry than
///   a `tk` document has ever been analysed against;
/// * the analyser recovers the bit separately, via
///   `DialectProfile::availability_for_name`'s union of the profile mask with
///   the parsed set.
///
/// Routing the registry lookup back through the name reproduces the first
/// behaviour exactly while the threaded profile carries the other two. Giving
/// `tk` a catalogue entry would collapse the three and is tracked separately —
/// it changes which commands a `tk` document sees, so it is not a refactor.
#[must_use]
pub fn registry_for_dialect_profile(
    dialect: &tcl_dialect::DialectProfile,
) -> &'static tcl_registry::CommandRegistry {
    tcl_registry::cache::registry_for_dialect(dialect.name)
}

/// Crate version string.
///
/// ```
/// assert!(!tcl_lsp_core::VERSION.is_empty());
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod dialect_ingress_tests {
    /// Regression for the `tk` leg of issue #1405.
    ///
    /// A `wish` document typically carries no `package require Tk`, so the Tk
    /// checks are reachable only through the *dialect*. `tk` is not a
    /// catalogue profile, so resolving the ingress with plain `by_name` sinks
    /// it to the permissive plain-Tcl fallback, the name "tk" is lost, and
    /// every provider that re-derives an analysis from the threaded profile
    /// silently stops running the Tk checks.
    ///
    /// This pins all three legs of that triangle at once: the ingress keeps
    /// the spelling, an analysis driven from it still reaches the Tk checks,
    /// and the registry lookup still resolves to the plain-Tcl registry a
    /// `tk` document has always been analysed against.
    #[test]
    fn the_tk_ingress_keeps_its_dialect_through_a_provider_analysis() {
        let profile = super::profile_for_dialect("tk");
        assert_eq!(profile.name, "tk", "the ingress must keep the spelling");
        assert!(
            profile
                .availability_mask
                .contains(tcl_dialect::DialectSet::TK),
            "the ingress must keep the TK availability bit"
        );

        let src = "frame .top\npack .top.a\ngrid .top.b\n";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(src, profile.name);
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|d| d.code.to_string().starts_with("TK")),
            "a tk-dialect document must reach the Tk checks without \
             `package require Tk`; got {:?}",
            analysis
                .diagnostics
                .iter()
                .map(|d| d.code.to_string())
                .collect::<Vec<_>>()
        );

        // The third leg: the registry must stay the one `by_name` selects,
        // not the one the Tk profile would build.
        assert!(std::ptr::eq(
            super::registry_for_dialect_profile(profile),
            tcl_registry::cache::registry_for_dialect("tk"),
        ));
    }
}
