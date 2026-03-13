/*
 * Pyodide Web Worker — loads the Tcl compiler pipeline and runs it
 * in response to {source, dialect} messages from the main thread.
 *
 * Protocol:
 *   main → worker:  { type: "compile", source: string, dialect: string }
 *   worker → main:  { type: "result", data: <serialised JSON> }
 *   worker → main:  { type: "error", error: string, traceback?: string }
 *   worker → main:  { type: "ready" }
 *   worker → main:  { type: "status", message: string }
 */

/* global importScripts, loadPyodide */

let pyodide = null;
let ready = false;

async function init() {
  postMessage({ type: "status", message: "Loading Pyodide runtime..." });

  // All assets are local — no external network requests at runtime.
  const baseUrl = new URL(".", self.location.href).href;
  const pyodideUrl = baseUrl + "pyodide/";

  importScripts(pyodideUrl + "pyodide.js");

  pyodide = await loadPyodide({
    indexURL: pyodideUrl,
  });

  postMessage({ type: "status", message: "Installing compiler package..." });

  await pyodide.loadPackage("micropip");
  const micropip = pyodide.pyimport("micropip");

  // Install our wheel without pulling pygls/lsprotocol (not needed in worker).
  const wheelUrl = baseUrl + "tcl_lsp-0.2.0-py3-none-any.whl";
  await micropip.install(wheelUrl, { deps: false });

  postMessage({ type: "status", message: "Initialising compiler..." });

  // Pre-import the pipeline so the first compile is fast
  await pyodide.runPythonAsync(`
from explorer.pipeline import run_pipeline, AVAILABLE_DIALECTS
from explorer.serialise import serialise_result
import json
`);

  ready = true;
  postMessage({ type: "ready" });
}

async function compile(source, dialect) {
  if (!ready) {
    postMessage({ type: "error", error: "Compiler not ready yet" });
    return;
  }

  try {
    const resultJson = await pyodide.runPythonAsync(`
_source = ${JSON.stringify(source)}
_dialect = ${JSON.stringify(dialect)}
_result = run_pipeline(_source, dialect=_dialect)
json.dumps(serialise_result(_result))
`);
    postMessage({ type: "result", data: JSON.parse(resultJson) });
  } catch (err) {
    postMessage({
      type: "error",
      error: err.message || String(err),
      traceback: err.stack || "",
    });
  }
}

onmessage = async function (e) {
  const msg = e.data;
  if (msg.type === "compile") {
    await compile(msg.source, msg.dialect);
  }
};

init().catch((err) => {
  postMessage({
    type: "error",
    error: "Failed to initialise Pyodide: " + (err.message || String(err)),
  });
});
