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
pub mod commands;
pub mod confusables_table;
pub mod diagnostics;
pub mod dispatch;
pub mod handlers;
pub mod irules_event_checks;
pub mod item_tree;
pub mod mro;
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
pub use item_tree::{FileDecls, Item, ItemId, ItemKind, ItemSig, ItemTree};
pub use mro::{MroError, build_mro_map, tcloo_linearise};
pub use snapshot::AnalyserSnapshot;
pub use state::{Analyser, NonAsciiMode};
pub use types::{
    AnalysisResult, ClassDef, CodeFix, Diagnostic, MethodDef, ProcArgTrait, ProcDef, PropertyDef,
    Scope, ScopeKind, Severity, StubFlags, UnknownProcInfo, VarDef,
};
