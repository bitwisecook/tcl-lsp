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
 * **string**, in both directions. No Content-Length framing: postMessage
 * already delimits messages. (Inbound, an object that is not one of the three
 * host messages below is accepted too and serialised here, for a client that
 * posts the message rather than its text.)
 *
 * `vscode-jsonrpc`'s `BrowserMessageReader`/`Writer` — which is what both
 * `monaco-languageclient` and `vscode-languageclient/browser` use — read and
 * write the *parsed object*, not the string, so a client built on them needs a
 * one-function port adapter: `JSON.parse` inbound, `JSON.stringify` outbound.
 * Both in-tree clients carry it (`rust/tcl-spec-studio/web/src/lspClient.ts`,
 * `editors/vscode/src/webLspTransport.ts`). Without it the server answers
 * `initialize` in full and the client discards the reply as "neither a response
 * nor a notification", so the handshake hangs with nothing in the console.
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
 * this a classic worker, so `new Worker(url)` needs no `{ type: "module" }` —
 * and must not be given one: a cross-origin host loads this through a classic
 * `importScripts` shim (see `assetBaseUrl` below), which a module worker breaks.
 */

/* global importScripts, wasm_bindgen */

let server = null;
const backlog = [];

/*
 * Where the three files sit, as a URL relative names can resolve against.
 *
 * `self.location.href` is the natural answer, and is what a page that loads
 * this worker directly gets. A host that loads it CROSS-ORIGIN cannot use it:
 * browsers refuse a cross-origin worker script outright, so such a host wraps
 * it in a same-origin blob that `importScripts()`es the real URL — and
 * `self.location` is then the opaque `blob:` URL, which no relative name can
 * resolve against at all (`new URL(name, blobUrl)` throws). VS Code's web
 * extension host does exactly this for every nested worker an extension
 * creates, so this is the normal case there, not an edge one.
 *
 * Such a host passes the directory holding these files as the worker's name —
 * `new Worker(url, { name: "…/dist/web/" })` — which reaches the real `Worker`,
 * so `self.name` survives where `self.location` does not. The name may arrive
 * *decorated*: VS Code prefixes it, so the worker sees
 * `"ExtensionHostWorker -> http://…/dist/web/"`. The last whitespace-separated
 * field is therefore tried as well as the whole string.
 */
function assetBaseUrl() {
  const name = typeof self.name === "string" ? self.name : "";
  const decorated = name.split(/\s+/).pop();
  for (const candidate of [self.location.href, name, decorated]) {
    if (!candidate) continue;
    try {
      // Throws for an opaque base (`blob:`, `data:`) — the whole point of the
      // probe, since that is exactly what cannot resolve a sibling.
      new URL("./", candidate);
      return new URL(candidate);
    } catch {
      // Try the next candidate.
    }
  }
  throw new Error(
    "tcl-lsp: the worker cannot locate its wasm assets. A host that loads " +
      "this worker cross-origin must pass the directory holding " +
      "tcl_lsp_server_wasm.js as the worker's name: new Worker(url, { name }).",
  );
}

async function init() {
  // Everything is local — the worker makes no network request at runtime.
  const workerUrl = assetBaseUrl();
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
