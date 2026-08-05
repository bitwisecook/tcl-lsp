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

//! Rust → WASM facade for the Tcl compiler-explorer GUI.
//!
//! Exposes a single `compile(source, dialect) -> String` that returns the
//! explorer contract JSON (`docs/design/contracts/wasm-explorer-view.md`
//! and the de-facto `explorer-core.js` shape). The standalone web worker
//! and the editor webviews call this directly — a self-contained WASM module
//! with no runtime interpreter and no `executeCommand` server round-trip.
//!
//! The compile path is pure compute over a string (no WASI I/O), so this
//! builds for `wasm32-unknown-unknown` with `wasm-bindgen`.

use wasm_bindgen::prelude::*;

/// Install a panic hook that routes Rust panics to `console.error` (panics
/// abort under wasm, so this is the only way to see them).
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// The explorer's `meta` block (dialect list, view-tab table, severity
/// vocabulary) as JSON, without compiling anything.
///
/// The GUI needs the dialect list to populate its dropdown *before* the
/// first compile — otherwise the dropdown shows only the hard-coded
/// `tcl8.6` fallback until a compile happens to land (issue #1183).
#[wasm_bindgen]
#[must_use]
pub fn meta() -> String {
    serde_json::to_string(&tcl_explorer::serialise_meta())
        .unwrap_or_else(|e| format!(r#"{{"error":"serialise failed: {e}"}}"#))
}

/// Compile `source` for `dialect` and return the explorer contract JSON.
///
/// On a serialisation failure it returns a JSON `{"error": ...}` object
/// rather than panicking, so the worker's `{type:"result"}` path always
/// receives parseable JSON.
#[wasm_bindgen]
#[must_use]
pub fn compile(source: &str, dialect: &str) -> String {
    let result = tcl_explorer::run_pipeline(source, dialect);
    serde_json::to_string(&tcl_explorer::serialise_result(&result))
        .unwrap_or_else(|e| format!(r#"{{"error":"serialise failed: {e}"}}"#))
}
