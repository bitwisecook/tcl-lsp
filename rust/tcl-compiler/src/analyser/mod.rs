//! Single-pass Tcl analyser — Rust port of `core/analysis/_analyser/`.
//!
//! Mirrors the Python ``Analyser`` mixin class in
//! ``core/analysis/_analyser/__init__.py``: walks segmented Tcl
//! commands, populates an [`AnalysisResult`] (procs, classes,
//! variables, diagnostics, package requires, …), and emits
//! W-coded warnings via the diagnostic emitters.
//!
//! Module shape (mirrors the Python file layout 1:1 for handler
//! files; mixin-trait pattern is *not* ported — the Rust analyser
//! is a single struct with methods grouped by concern):
//!
//! - [`state`] — the [`Analyser`] struct + per-walk state fields.
//! - [`types`] — [`AnalysisResult`] + the per-record types
//!   ([`ProcDef`], [`ClassDef`], [`VarDef`], [`Diagnostic`], …).
//! - [`snapshot`] — [`AnalyserSnapshot`] for chunked re-analysis
//!   (filled in by **C41a3**).
//! - [`utils`] — pure helpers ported from
//!   ``core/analysis/_analyser/_utils.py`` (filled in by **C41a2**).
//!
//! Subsequent strips add per-concern modules — `commands.rs`
//! (**C41b**), `proc.rs` (**C41c**), `diagnostics/` (**C41d**),
//! `oo.rs` + `recovery.rs` (**C41e**), and the public entry +
//! `PyO3` binding (**C41f**).
//!
//! The `PyO3` binding for ``analyser_analyse(source, dialect)``
//! lives in the ``tcl-lsp-rust`` crate; the dispatch shim in
//! ``core.analysis._analyser.__init__`` will route to the Rust
//! binding via ``TCL_LSP_RUST_ANALYSER=1`` (default-off at first;
//! flip to default-on once the differential corpus has baked, same
//! as ``C40-default-on``).

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
