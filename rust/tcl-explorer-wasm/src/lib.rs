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
    compile_with_optimisations(source, dialect, "")
}

/// [`compile`] with a comma-separated semantic/AOT optimisation selection —
/// the GUI's per-pass toggles.
///
/// `optimisations` takes the pass ids the `semanticOptimisations` view lists
/// (`native-lowering`, `representation-inference`, …) or one of its group
/// names (`native-tier`, `all`); empty means the generic lowering, which is
/// what [`compile`] asks for. An unknown name comes back as a JSON `error`
/// object rather than being dropped: a toggle the user set and did not get is
/// a wrong answer, and the GUI would otherwise show an unoptimised module as
/// though it were optimised.
#[wasm_bindgen]
#[must_use]
pub fn compile_with_optimisations(source: &str, dialect: &str, optimisations: &str) -> String {
    let config = match tcl_explorer::SemanticOptimisationConfig::from_names(optimisations) {
        Ok(config) => config,
        Err(message) => {
            return serde_json::json!({ "error": message }).to_string();
        }
    };
    let result = tcl_explorer::run_pipeline(source, dialect);
    serde_json::to_string(&tcl_explorer::serialise_result_with_optimisations(
        &result, config,
    ))
    .unwrap_or_else(|e| format!(r#"{{"error":"serialise failed: {e}"}}"#))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The GUI's toggle path: a pass selection reaches the emitter, and an
    /// unrecognised name comes back as an `error` object rather than being
    /// silently dropped — a toggle that does nothing would show an
    /// unoptimised module as though it were optimised.
    #[test]
    fn compile_with_optimisations_applies_the_selection_and_rejects_a_bad_name() {
        let source = "set a 1\nincr a\n";
        let off: serde_json::Value =
            serde_json::from_str(&compile(source, "tcl9.0")).expect("result JSON");
        assert_eq!(
            off["wasm"][0]["codegenPlan"]["nativeLowering"]["enabled"],
            false
        );

        let on: serde_json::Value =
            serde_json::from_str(&compile_with_optimisations(source, "tcl9.0", "native-tier"))
                .expect("result JSON");
        assert_eq!(
            on["wasm"][0]["codegenPlan"]["nativeLowering"]["enabled"],
            true
        );

        let bad: serde_json::Value =
            serde_json::from_str(&compile_with_optimisations(source, "tcl9.0", "nope"))
                .expect("error JSON");
        assert!(
            bad["error"].as_str().is_some_and(|e| e.contains("`nope`")),
            "{bad}"
        );
    }

    #[test]
    fn wasm_contract_exposes_the_descriptor_and_world_ssa_payload() {
        let meta: serde_json::Value = serde_json::from_str(&meta()).expect("meta JSON");
        assert!(
            meta["views"]
                .as_array()
                .unwrap()
                .iter()
                .any(|view| view["id"] == "worldSsa")
        );

        let compiled: serde_json::Value =
            serde_json::from_str(&compile("interp create child", "tcl9.0")).expect("compile JSON");
        assert!(compiled["worldSsa"].is_array());
        assert!(compiled["worldSsa"][0]["availability"]["kind"].is_string());
    }
}
