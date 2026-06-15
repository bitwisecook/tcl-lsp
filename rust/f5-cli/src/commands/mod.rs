//! Verb handlers for the `f5-query` CLI.
//!
//! Each handler mirrors a `_run_*` function in `tooling/f5/verbs/*`. Most f5
//! verbs await their BIG-IP engine ports (analysis/emit live in
//! `dialects/f5/bigip/*`); the ones here need only file I/O + the existing
//! `tcl-bigip` parser.

pub mod cleanup;
pub mod diff;
pub mod difflib;
pub mod explain;
pub mod extract;
pub mod graph;
pub mod grep;
pub mod merge;
pub mod query;
pub mod rename;
pub mod scf;
pub mod split;
pub mod stats;
pub mod validate;
