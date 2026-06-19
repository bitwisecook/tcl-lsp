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
 * Protocol (unchanged from the Pyodide worker):
 *   main → worker:  { type: "compile", source: string, dialect: string }
 *   worker → main:  { type: "result", data: <serialised JSON> }
 *   worker → main:  { type: "error", error: string, traceback?: string }
 *   worker → main:  { type: "ready" }
 *   worker → main:  { type: "status", message: string }
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
  postMessage({ type: "ready" });
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
