//! Verb handlers for the `tcl` CLI.
//!
//! Each handler mirrors a `_run_*` function in `tooling/tcl/verbs/*`: it
//! resolves inputs via `tcl-cli-support`, drives the relevant Rust engine
//! crate, and writes output. Handlers return the intended process exit code.

pub mod diag;
pub mod diagram;
pub mod diff;
pub mod explore;
pub mod graphs;
pub mod highlight;
pub mod lookup;
pub mod registry;
pub mod transform;
