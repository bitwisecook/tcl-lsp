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

  postMessage({ type: "status", message: "Fetching compiler wheel..." });

  // Discover the wheel filename from build_info.json (populated by the
  // Makefile from pyproject.toml's version) so the worker doesn't need to
  // be rewritten every time the package version bumps.
  const buildInfoResponse = await fetch(baseUrl + "build_info.json");
  if (!buildInfoResponse.ok) {
    throw new Error(
      `Failed to fetch build_info.json: ${buildInfoResponse.status} ${buildInfoResponse.statusText}`,
    );
  }
  const buildInfo = await buildInfoResponse.json();
  if (!buildInfo.wheel_filename) {
    throw new Error("build_info.json is missing wheel_filename");
  }

  // Fetch the wheel and extract it directly into site-packages.
  //
  // We deliberately avoid micropip: the version shipped with Pyodide 0.27.3
  // (micropip 0.8.0) has a race in its `deps=False` code path — it creates a
  // download task with `asyncio.create_task` but never awaits it, so
  // `install()` runs before the wheel bytes are loaded and fails with
  // "Micropip internal error: attempted to install wheel before downloading
  // it?". Fixed upstream in micropip 0.9.0 (pyodide/micropip#180).
  //
  // Our wheel is pure Python and the worker only imports modules under
  // `core` and `explorer`, none of which need pygls/lsprotocol/jinja2 at
  // import time, so we don't need a dependency resolver here.
  const wheelUrl = baseUrl + buildInfo.wheel_filename;
  const wheelResponse = await fetch(wheelUrl);
  if (!wheelResponse.ok) {
    throw new Error(
      `Failed to fetch ${wheelUrl}: ${wheelResponse.status} ${wheelResponse.statusText}`,
    );
  }
  const wheelBytes = new Uint8Array(await wheelResponse.arrayBuffer());

  postMessage({ type: "status", message: "Installing compiler package..." });

  const sitePackages = pyodide.runPython(
    "import site; site.getsitepackages()[0]",
  );
  pyodide.unpackArchive(wheelBytes, "zip", { extractDir: sitePackages });
  pyodide.runPython("import importlib; importlib.invalidate_caches()");

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
