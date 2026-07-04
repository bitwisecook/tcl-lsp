//! Standalone HTML BIG-IP estate report, built from the `f5-query` engine.
//!
//! This is the Rust port of the `f5report` generator (which remains as the
//! PyO3/Python demonstration of driving the query engine as a library). Given
//! one or more loaded `(uri, scf_text)` configs it produces the exact same
//! single-file, self-contained interactive HTML report — object tables, a
//! reference/orphan analysis, an SSL-certificate expiry inventory, a Mermaid
//! topology explorer, a listener/flow simulator and an in-browser `f5-query`
//! console — with no server and no external assets.
//!
//! The heavy lifting (config parsing, object projection, the `referenced_by`
//! reference-graph walk) is done by [`tcl_bigip_query`]; this crate only shapes
//! that output into a model ([`collect_model`]) and renders it ([`build_report`]).

// This crate is a faithful, line-for-line port of the Python `f5report`
// generator; its shaping functions mirror the original's structure rather than
// being re-decomposed, and it maps the DSL's unbounded integers to `usize` for
// lengths / counts. These pedantic lints fight that fidelity for no real gain.
#![allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::manual_let_else
)]

mod certs;
mod graph;
mod jutil;
mod model;
mod query;
mod render;
mod secrets;
mod services;

pub use tcl_lexer::highlight_tcl;

pub use model::{ENGINE_VERSION, collect_model, collect_model_with_certs};
pub use query::{ReportError, Source};
pub use render::{RenderOptions, build_report};
pub use secrets::{collect_secrets, count_encrypted_secrets, decrypt_secrets};
