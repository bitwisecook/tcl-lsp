//! Shared Tcl parse-tree + byte-exact-semantics layer.
//!
//! This is the convergence crate (see
//! `docs/design/runtime/rust-runtime-port.md`): the single home for the pure
//! Tcl parsing primitives that both the LSP/compiler suite and the WASM runtime
//! port need, so neither side re-derives (and drifts) its own copy.
//!
//! It depends only on [`tcl_lexer`] (the canonical scanner +
//! [`tcl_lexer::backslash_subst`]) and so stays `wasm32`-clean. Everything is
//! `&str`-based — Tcl strings are UTF-8 internally, and byte consumers convert
//! at the call (the UTF-8-internal-rep invariant).
//!
//! Modules land per the phased extraction plan:
//! - [`backslash`] — the canonical `TclParseBackslash` decoder (done).
//! - [`list`] — `Tcl_SplitList` / `Tcl_Merge` (done).
//! - [`naming`] — variable/command name normalisation (done).
//! - [`expr`] — the `expr` AST + Pratt parser (done).
//! - [`format`] — the `format` conversion-specifier grammar (done).
//! - [`number`] — the `TclParseNumber` numeric-literal grammar (done).
//! - [`glob`] — `Tcl_StringCaseMatch` (`string match`) (done).
//! - `subst` — to follow.

pub mod backslash;
pub mod expr;
pub mod format;
pub mod glob;
pub mod list;
pub mod naming;
pub mod number;
