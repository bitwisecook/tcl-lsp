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

//! Single-pass Tcl analyser.
//!
//! Walks segmented Tcl commands, populates an [`AnalysisResult`]
//! (procs, classes, variables, diagnostics, package requires, …),
//! and emits W-coded warnings via the diagnostic emitters.
//!
//! The analyser is a single struct with methods grouped by concern:
//!
//! - [`state`] — the [`Analyser`] struct + per-walk state fields.
//! - [`types`] — [`AnalysisResult`] + the per-record types
//!   ([`ProcDef`], [`ClassDef`], [`VarDef`], [`Diagnostic`], …).
//! - [`snapshot`] — [`AnalyserSnapshot`] for chunked re-analysis.
//! - [`utils`] — pure helpers.
//!
//! Per-concern modules cover commands (`commands.rs`), procs
//! (`proc.rs`), diagnostics (`diagnostics/`), `TclOO` and recovery
//! (`oo.rs` + `recovery.rs`), and the public entry plus the `PyO3`
//! binding.
//!
//! This pure analyser is consumed directly by the native
//! `tcl-lsp-server` (it is the default and only path) and exposed to
//! Python wheel consumers through the `tcl-lsp-py` `PyO3` surface
//! (`tcl-lsp-rust` is a re-export alias).

pub mod bounds_checks;
pub mod class_hierarchy;
/// EXPERIMENT — object→class binding lattice + dispatch resolver.
/// Not wired into shipping diagnostics; measured by the `mro_eval`
/// harness.  See `docs/design/tcloo-mro-lattice.md`.
pub mod class_lattice;
pub mod commands;
pub mod confusables_table;
pub mod diagnostics;
pub mod dispatch;
pub mod handlers;
pub mod indirection;
pub mod irules_event_checks;
pub mod item_tree;
pub mod oo;
pub mod param_traits;
pub mod per_item;
pub mod recovery;
pub mod scope;
pub mod snapshot;
pub mod state;
pub mod syntax_checks;
pub mod tk_checks;
pub mod types;
pub mod utils;

pub use class_hierarchy::{ClassHierarchy, build_class_hierarchy};
pub use class_lattice::{
    AblationConfig, ClassValue, DispatchStats, DispatchVerdict, NsContext, SiteReport, TopReason,
    analyse_dispatch, class_values, next_provider,
};
pub use item_tree::{FileDecls, Item, ItemId, ItemKind, ItemSig, ItemTree};
// The MRO linearisation lives in `tcl-syntax` so the bytecode VM (which must
// not depend on the compiler — the `CompileService` injection keeps the wasm
// core light) can share it. Re-exported here so `tcl_compiler::analyser::…`
// callers are unchanged.
pub use scope::{
    VariableAliasLink, command_resolution_namespace_at, implicit_command_namespace_path_at,
    innermost_scope_is_oo_method_frame, innermost_scope_reaches_oo_helpers,
    lookup_var_by_qualified_name, lookup_var_in_namespace, namespace_variables,
    qualified_name_for_var_decl, variable_alias_links,
};
pub use snapshot::AnalyserSnapshot;
pub use state::{Analyser, NonAsciiMode};
pub use tcl_syntax::mro::{self, MroError, build_mro_map, tcloo_linearise};
pub use types::{
    AnalysisResult, ClassDef, CodeFix, DefinedSymbol, DefinitionAbortKind, Diagnostic, FixSafety,
    MemberSide, MethodDef, ObjectMethodDef, ProcArgTrait, ProcDef, PropertyDef, QualifiedVarRef,
    RenamedMember, Scope, ScopeKind, Severity, StubFlags, UnknownProcInfo, VarDef,
    class_constructor_key, class_destructor_key, class_member_key, class_property_key,
};
