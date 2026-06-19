//! Rust → WASM facade for the Tcl compiler-explorer GUI.
//!
//! Exposes a single `compile(source, dialect) -> String` that returns the
//! explorer contract JSON (`docs/design/contracts/wasm-explorer-view.md`
//! and the de-facto `explorer-core.js` shape). The standalone web worker
//! and the editor webviews call this directly — no Pyodide, no Python at
//! runtime, no `executeCommand` server round-trip.
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
