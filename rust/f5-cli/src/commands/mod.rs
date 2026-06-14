//! Verb handlers for the `f5-query` CLI.
//!
//! Each handler mirrors a `_run_*` function in `tooling/f5/verbs/*`. Most f5
//! verbs await their BIG-IP engine ports (analysis/emit live in
//! `dialects/f5/bigip/*`); the ones here need only file I/O + the existing
//! `tcl-bigip` parser.

pub mod merge;
pub mod split;
