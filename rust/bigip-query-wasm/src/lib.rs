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

//! WebAssembly binding for the F5 BIG-IP query engine.
//!
//! Exposes a single entry point, [`run_query`], that the report's in-page
//! console calls: given a query expression, the loaded configs (as an ordered
//! `[[uri, text], …]` JSON array) and a render mode, it evaluates the query
//! entirely in the browser and returns the rendered output.
//!
//! The network-probe builtins are compiled out (`tcl-bigip-query` with
//! `default-features = false`), so this binary has no socket / TLS / x509
//! dependency — only the pure parse / project / evaluate / render pipeline.

use wasm_bindgen::prelude::*;

use tcl_bigip_query::output;
use tcl_bigip_query::value::Value;
use tcl_bigip_query::{QueryOptions, run_query as engine_run_query};

/// Run a `f5-query` expression against the embedded configs.
///
/// * `expr` — the query expression.
/// * `sources_json` — an ordered JSON array of `[uri, text]` pairs.
/// * `mode` — an output mode (`json`, `auto`, `raw`, `paths`, `table`,
///   `table-lineart`, `scf`); empty defaults to `json`.
/// * `merge` — treat every config as one namespace (`--merge`).
///
/// Returns the rendered output string, or a `JsError` carrying the engine's
/// error message (parse error, evaluation error, unknown builtin — including
/// the probe builtins, which are absent from this build).
#[wasm_bindgen]
pub fn run_query(
    expr: &str,
    sources_json: &str,
    mode: &str,
    merge: bool,
) -> Result<String, JsError> {
    let sources: Vec<(String, String)> = serde_json::from_str(sources_json)
        .map_err(|e| JsError::new(&format!("invalid sources JSON: {e}")))?;

    let opts = QueryOptions { merge, ..QueryOptions::default() };
    let result = engine_run_query(expr, &sources, &opts)
        .map_err(|e| JsError::new(&e.to_string()))?;

    if result.has_mutation {
        return Err(JsError::new(
            "mutating queries are not supported in the browser console (read-only)",
        ));
    }

    // Flatten every file's values into one stream, matching `f5 query`'s
    // multi-file output, then render in the requested mode.
    let flat: Vec<Value> = result
        .values_per_file
        .into_iter()
        .flat_map(|(_uri, vals)| vals)
        .collect();
    let mode = if mode.is_empty() { "json" } else { mode };
    output::render(&flat, mode).map_err(|e| JsError::new(&e.to_string()))
}

/// The engine version string (for the console's status line).
#[wasm_bindgen]
pub fn engine_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
