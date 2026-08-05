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

/*
 * Rust → WASM Web Worker — loads the `tcl-explorer-wasm` module and runs
 * the compiler pipeline in response to {source, dialect} messages. No
 * Pyodide, no Python at runtime.
 *
 * The module is built by `make explorer-wasm` (wasm-pack --target
 * no-modules), which emits `tcl_explorer_wasm.js` (defining the global
 * `wasm_bindgen`) + `tcl_explorer_wasm_bg.wasm` next to this file. Using
 * the no-modules target keeps this a classic worker (importScripts), so
 * `index.html`'s `new Worker('worker.js')` and `explorer-core.js` are
 * unchanged.
 *
 * Protocol (unchanged from the Pyodide worker, plus `ready.meta`):
 *   main → worker:  { type: "compile", source: string, dialect: string }
 *   worker → main:  { type: "result", data: <serialised JSON> }
 *   worker → main:  { type: "error", error: string, traceback?: string }
 *   worker → main:  { type: "ready", meta?: <serialised meta JSON> }
 *   worker → main:  { type: "status", message: string }
 *
 * `ready.meta` carries the explorer `meta` block (dialect list, view table,
 * severities). It needs no compile, so the GUI can fill its dialect dropdown
 * as soon as the module loads instead of waiting for a first result.
 */

/* global importScripts, wasm_bindgen */

let ready = false;

async function init() {
  postMessage({ type: "status", message: "Loading WASM compiler..." });

  // All assets are local — no external network requests at runtime.
  const baseUrl = new URL(".", self.location.href).href;

  // `--target no-modules` defines a global `wasm_bindgen` init function
  // with `compile` attached to it.
  importScripts(baseUrl + "tcl_explorer_wasm.js");
  await wasm_bindgen(baseUrl + "tcl_explorer_wasm_bg.wasm");

  ready = true;
  postMessage({ type: "ready", meta: readMeta() });
}

// The `meta` block, or null when the module predates the `meta` export (the
// GUI then keeps its hard-coded dialect fallback until the first result).
function readMeta() {
  if (typeof wasm_bindgen.meta !== "function") return null;
  try {
    return JSON.parse(wasm_bindgen.meta());
  } catch {
    return null;
  }
}

function compile(source, dialect) {
  if (!ready) {
    postMessage({ type: "error", error: "Compiler not ready yet" });
    return;
  }

  try {
    const resultJson = wasm_bindgen.compile(source, dialect);
    const data = JSON.parse(resultJson);
    // `compile` returns a JSON {error: ...} object on a pipeline failure
    // rather than throwing.
    if (data && typeof data === "object" && data.error) {
      postMessage({ type: "error", error: data.error });
      return;
    }
    postMessage({ type: "result", data });
  } catch (err) {
    postMessage({
      type: "error",
      error: err.message || String(err),
      traceback: err.stack || "",
    });
  }
}

onmessage = function (e) {
  const msg = e.data;
  if (msg.type === "compile") {
    compile(msg.source, msg.dialect);
  }
};

init().catch((err) => {
  postMessage({
    type: "error",
    error: "Failed to initialise WASM compiler: " + (err.message || String(err)),
  });
});
