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

//! Standalone HTML BIG-IP estate report, built from the `f5-query` engine.
//!
//! This is the Rust port of the `f5report` generator (which remains as the
//! PyO3/Python demonstration of driving the query engine as a library). Given
//! one or more loaded `(uri, scf_text)` configs it produces the exact same
//! single-file, self-contained interactive HTML report — object tables, a
//! reference/orphan analysis, an SSL-certificate expiry inventory, an elkjs
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

mod apm;
mod certs;
mod crypt;
pub mod enrich;
mod forensics;
mod graph;
mod jutil;
mod lifecycle;
mod markdown;
mod model;
mod query;
mod render;
mod secrets;
mod security;
mod services;

pub use tcl_lexer::highlight_tcl;

pub use forensics::collect_forensics;
pub use lifecycle::{K5903_URL, bigip_release_lifecycle};
pub use markdown::render_markdown;
pub use model::{
    ENGINE_VERSION, GIT_DESCRIBE, GIT_HASH, collect_model, collect_model_full,
    collect_model_with_architecture, collect_model_with_certs,
};
pub use query::{ReportError, Source};
pub use render::{RenderOptions, build_report};
pub use secrets::{collect_secrets, count_encrypted_secrets, decrypt_secrets};
/// Architecture / topology detection, hoisted into the query engine so the
/// report, the Python binding, and the console wasm all share it. Re-exported
/// here for compatibility.
pub use tcl_bigip_query::build_architecture;
/// The full f5-query manual (grammar + builtins + cookbook), re-exported so the
/// wasm builder / report can embed a reference panel.
pub use tcl_bigip_query::manual::format_manual;
