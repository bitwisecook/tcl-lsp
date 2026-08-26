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
 * The Tcl language server, running in a Web Worker.
 *
 * The wire protocol is raw LSP JSON-RPC: one message per postMessage, as a
 * string, in both directions. That is what `monaco-languageclient`'s browser
 * worker transport speaks, so a page can point it straight at this worker —
 * `new Worker('worker.js')`, then `BrowserMessageReader/Writer`. No
 * Content-Length framing: postMessage already delimits messages.
 *
 * Three message shapes are NOT protocol traffic and are handled here instead,
 * because they are objects rather than strings:
 *
 *   { tclLsp: "upsert", uri, text }          register a closed file's contents
 *   { tclLsp: "delete", uri }                forget one
 *   { tclLsp: "upsertSpecPack", name, text } register a .tclspec pack
 *
 * They fill the server's in-memory file store, which is where every file the
 * editor has not opened comes from — there is no filesystem to read. That
 * store backs the whole-workspace paths too, so upserted siblings are indexed
 * by the workspace scan and upserted pkgIndex.tcl files build the package
 * database, not only the single file the editor has open.
 *
 * `upsertSpecPack` is separate from `upsert` because it does not key on a
 * `file:` URI: packs go under a virtual mount that deliberately cannot name a
 * real path (`LspWorker.spec_pack_mount()` reports it), which is where pack
 * discovery looks when there is no executable to sit beside. Its `name` is
 * relative to that mount and must stay inside it — a rooted name or one with a
 * `..` component is refused and logged, so a pack upsert cannot shadow an
 * unrelated store path.
 *
 * Send all three BEFORE `initialize`: `initialized` is what loads the pack set
 * and runs the workspace scan. A file that appears later needs no special
 * message — upsert it and post an ordinary `workspace/didChangeWatchedFiles`,
 * the same notification an editor sends for a file changed outside it.
 *
 * Built by `make lsp-server-wasm` (build-wasm.sh), which emits
 * `tcl_lsp_server_wasm.js` (defining the global `wasm_bindgen`) and
 * `tcl_lsp_server_wasm_bg.wasm` next to this file. The no-modules target keeps
 * this a classic worker, so `new Worker(url)` needs no `{ type: "module" }`.
 */

/* global importScripts, wasm_bindgen */

let server = null;
const backlog = [];

async function init() {
  // Everything is local — the worker makes no network request at runtime.
  const workerUrl = new URL(self.location.href);
  const assetVersion = workerUrl.searchParams.get("v");
  const sibling = (name) => {
    const url = new URL(name, workerUrl);
    if (assetVersion) url.searchParams.set("v", assetVersion);
    return url.href;
  };
  importScripts(sibling("tcl_lsp_server_wasm.js"));
  await wasm_bindgen(sibling("tcl_lsp_server_wasm_bg.wasm"));

  // Bind postMessage: the server calls it with `this` unbound.
  server = new wasm_bindgen.LspWorker((text) => self.postMessage(text));

  // A language client sends `initialize` the moment it is constructed, which
  // can beat the wasm instantiation above. Replaying the backlog in order is
  // what stops that race from losing the handshake.
  for (const message of backlog.splice(0)) {
    dispatch(message);
  }
}

function dispatch(data) {
  if (typeof data === "string") {
    server.send(data);
    return;
  }
  if (data && data.tclLsp === "upsert") {
    server.vfs_upsert(data.uri, data.text);
    return;
  }
  if (data && data.tclLsp === "delete") {
    server.vfs_delete(data.uri);
    return;
  }
  if (data && data.tclLsp === "upsertSpecPack") {
    server.vfs_upsert_spec_pack(data.name, data.text);
    return;
  }
  // A client that posts the message object rather than its JSON text.
  server.send(JSON.stringify(data));
}

self.onmessage = (event) => {
  if (server === null) {
    backlog.push(event.data);
    return;
  }
  dispatch(event.data);
};

init().catch((err) => {
  // Nothing can work after a failed init, and the language client is waiting
  // on an `initialize` reply that will never come — say so where a developer
  // will see it rather than hanging silently.
  console.error("tcl-lsp: the language server worker failed to start", err);
});
