//! Verb handlers for the `tcl` CLI.
//!
//! Each handler mirrors a `_run_*` function in `tooling/tcl/verbs/*`: it
//! resolves inputs via `tcl-cli-support`, drives the relevant Rust engine
//! crate, and writes output. Handlers return the intended process exit code.

pub mod compile;
pub mod diag;
pub mod diagram;
pub mod diff;
pub mod docker;
pub mod explore;
pub mod graphs;
pub mod help;
pub mod highlight;
pub mod lookup;
pub mod minimize;
pub mod misc;
pub mod pkg;
pub mod registry;
pub mod transform;
pub mod venv;
