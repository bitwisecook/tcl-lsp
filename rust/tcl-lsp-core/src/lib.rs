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
pub mod ilx_navigation;
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
pub mod sslictcl_diagnostics;
pub mod tcl_install;
pub mod tk_preview;
pub mod type_definition;
pub mod type_hierarchy;
pub mod vfs;
pub mod workspace_index;
pub mod workspace_symbols;

/// Resolve a document's dialect *name* to its environment — **the** LSP
/// dialect ingress (centralisation ledger rows F2/F3, P1-F wave 2).
///
/// Every dialect string this crate accepts — `analysis.dialect`, a Salsa
/// `dialect` input, a settings value, an editor language id already mapped
/// to a canonical name — resolves here, once, through
/// [`tcl_registry::model::resolve_environment`]: the one
/// `EnvironmentRegistry::resolve` seam the compiler's ingress also
/// delegates to. Providers derive the registry generation
/// ([`context_for_dialect`]), the availability view
/// (`generation.context()`), and the interop [`DialectProfile`] they still
/// thread ([`profile_for_dialect`]) from the resolved environment, never
/// from a second parse of the string.
///
/// [`DialectProfile`]: tcl_dialect::DialectProfile
#[must_use]
pub fn environment_for_dialect(name: &str) -> tcl_registry::model::DocumentEnvironment {
    tcl_registry::model::resolve_environment(name)
}

/// The interned profile a document of `name` threads.
///
/// The environment-model face of the old ingress
/// (`resolve_known(name).unwrap_or_else(|| by_name(name))`): the resolved
/// environment's [`unit_profile`], which is its same-named catalogue
/// profile, the typed additive `tk` profile for the `tk` environment, and
/// the permissive fallback for the model-only and unknown names. Pinned to
/// the old answer for every catalogue name, alias, `tk`, and the
/// unknown-name sink by `tcl_registry::model::ingress`'s
/// `unit_profile_reproduces_the_old_lsp_ingress`.
///
/// Post-P1-G (which deleted the name validators): the profile itself
/// retires with ledger C1's re-type; consumers then move to the
/// environment and its
/// [`ResolvedContext`](tcl_registry::model::ResolvedContext) queries.
///
/// [`unit_profile`]: tcl_registry::model::DocumentEnvironment::unit_profile
#[must_use]
pub fn profile_for_dialect(name: &str) -> &'static tcl_dialect::DialectProfile {
    environment_for_dialect(name).unit_profile()
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

/// The profile a **stated** dialect names — `None` when `name` states no
/// dialect at all (empty, unknown, or the lenient `tcl` spelling).
///
/// The environment-derived replacement for `DialectProfile::resolve_known`
/// (ledger row C2): the consumers passing an `Option<&DialectProfile>` that
/// means "the dialect this build selected, if any" — the compiler's
/// interprocedural, compiler-checks and optimiser entry points — read it
/// here. Distinct from [`optional_profile_for_dialect`], which answers
/// `Some(fallback)` for an unknown name because *its* `None` means only
/// "the caller stated no name at all".
#[must_use]
pub fn stated_profile_for_dialect(name: &str) -> Option<&'static tcl_dialect::DialectProfile> {
    environment_for_dialect(name).stated_profile()
}

/// The **registry generation** a document of `dialect` is analysed
/// against: the per-environment [`ContextRegistry`] assembled for the
/// resolved environment, carrying both the spec store
/// ([`ContextRegistry::commands`]) and the availability view
/// ([`ContextRegistry::context`]).
///
/// This replaces the old name-keyed `registry_for_dialect` hop *and* the
/// `tk` triangle it existed to reproduce: `tk` is now an environment, its
/// registry is the generation assembled for it (whose store is the same
/// plain-Tcl `Arc` a `wish` document has always been analysed against —
/// `store_profile("tk")` is the fallback profile), and the `TK` fact is the
/// environment's own **ambient `Tk` placement** (P3) — read as
/// [`ambient_package`](tcl_registry::model::ResolvedContext::ambient_package)
/// on the generation's context — rather than a re-parsed bit or an
/// environment name.
///
/// The generation is promoted to `&'static` exactly as the old per-profile
/// registry was: the un-overlaid axis is a closed set and is retained
/// unconditionally by the generation cache, so the promotion leaks a clone
/// of one `Arc`, never a second assembly.
///
/// [`ContextRegistry`]: tcl_registry::model::ContextRegistry
/// [`ContextRegistry::commands`]: tcl_registry::model::ContextRegistry::commands
/// [`ContextRegistry::context`]: tcl_registry::model::ContextRegistry::context
#[must_use]
pub fn context_for_dialect(dialect: &str) -> &'static tcl_registry::model::ContextRegistry {
    tcl_registry::model::static_context_for(dialect)
}

/// [`context_for_dialect`] for a caller that already holds the resolved
/// profile — transitional plumbing for the providers whose signatures still
/// take a `&DialectProfile` (retired with the profile itself under
/// ledger C1). The
/// profile's canonical name **is** a canonical environment id, so this is an
/// id-keyed lookup, not a re-parse of a user string.
#[must_use]
pub fn context_for_dialect_profile(
    dialect: &tcl_dialect::DialectProfile,
) -> &'static tcl_registry::model::ContextRegistry {
    tcl_registry::model::static_context_for_profile(dialect)
}

/// The **document context** a document of `dialect` is assisted under:
/// the resolved environment's [`ResolvedContext`] carrying its document
/// authoring mask.
///
/// Every availability, option, floor, and subcommand question a provider
/// asks about a document goes here — the assistance view of centralisation
/// R-c/R-d, and the replacement for the whole `ProfileQueries` surface
/// (ledger row F1's assistance half). Deliberately *not*
/// [`context_for_dialect`]'s own `context()`: the two differ for the `tk`
/// environment by exactly the additive `TK` bit a `tk` document has always
/// been answered under (see
/// [`document_authoring_scope`](tcl_registry::model::DocumentEnvironment::document_authoring_scope)).
///
/// [`ResolvedContext`]: tcl_registry::model::ResolvedContext
#[must_use]
pub fn document_context_for_dialect(
    dialect: &str,
) -> &'static tcl_registry::model::ResolvedContext {
    tcl_registry::model::static_document_context_for(dialect)
}

/// [`document_context_for_dialect`] for a caller that already holds the
/// resolved profile — transitional plumbing for the providers whose
/// signatures still take a `&DialectProfile` (retired with the profile
/// itself under ledger C1).
#[must_use]
pub fn document_context_for_profile(
    dialect: &tcl_dialect::DialectProfile,
) -> &'static tcl_registry::model::ResolvedContext {
    tcl_registry::model::static_document_context_for_profile(dialect)
}

/// The command **store** a document of `dialect` is analysed against —
/// [`context_for_dialect`]'s spec content, for the consumers that read raw
/// spec data (name sets, ensemble tables, hover text) rather than asking
/// availability questions.
#[must_use]
pub fn registry_for_dialect(dialect: &str) -> &'static tcl_registry::CommandRegistry {
    context_for_dialect(dialect).commands()
}

/// The command **store** a document of `dialect`'s profile is analysed
/// against — [`context_for_dialect_profile`]'s spec content, for the
/// consumers that read raw spec data (name sets, ensemble tables, hover
/// text) rather than asking availability questions.
#[must_use]
pub fn registry_for_dialect_profile(
    dialect: &tcl_dialect::DialectProfile,
) -> &'static tcl_registry::CommandRegistry {
    context_for_dialect_profile(dialect).commands()
}

/// Crate version string.
///
/// ```
/// assert!(!tcl_lsp_core::VERSION.is_empty());
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The shared Tcl qualified-name canonicaliser, re-exported for LSP hosts that
/// must normalise command names before passing provider options into this
/// crate. The semantic owner remains `tcl-syntax::naming`.
pub use tcl_syntax::naming::normalise_qualified_name as normalise_qualified_command_name;

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
            profile.surface_query().packages.contains(&"Tk"),
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

        // The third leg: the registry must stay the plain-store one the
        // lenient ingress selects, not the one the Tk profile would build.
        assert!(std::ptr::eq(
            super::registry_for_dialect_profile(profile),
            tcl_registry::model::ingress::static_context_for("tk")
                .commands()
                .as_ref(),
        ));
    }
}
