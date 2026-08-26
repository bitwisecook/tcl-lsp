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

import {
  BrowserMessageReader,
  BrowserMessageWriter,
  MessageTransports,
} from "vscode-languageclient/browser";

/** The slice of `MessagePort` that `vscode-jsonrpc`'s browser pair uses. */
interface JsonRpcPort {
  postMessage(message: unknown): void;
  addEventListener(type: string, handler: (event: Event) => void): void;
  onmessage: ((event: MessageEvent) => void) | null;
}

/**
 * Bridge the two halves of "one JSON-RPC message per `postMessage`".
 *
 * `rust/tcl-lsp-server-wasm/worker.js`'s wire format is a **string** in both
 * directions: it posts `server.send`'s output verbatim and takes a string on
 * its fast path. `vscode-jsonrpc`'s `BrowserMessageReader`, though, fires
 * `event.data` straight at the connection as if it were an already-parsed
 * `Message`, and `BrowserMessageWriter` posts the object. Left alone the two
 * agree on the framing and disagree on the encoding, and the symptom is
 * peculiarly quiet: the server answers `initialize` in full, the client logs
 * "Received message which is neither a response nor a notification message",
 * drops it, and `client.start()` never resolves — which, because activation
 * awaits it, hangs the whole extension host rather than failing.
 *
 * Adapting here rather than in the worker keeps the worker's documented
 * contract intact for every other client of it. The spec studio's web client
 * (`rust/tcl-spec-studio/web/src/lspClient.ts`) carries the same adapter for
 * the same reason.
 *
 * Only protocol traffic goes through this. The three store messages
 * (`{ tclLsp: "upsert" | "delete" | "upsertSpecPack", … }`) are objects by
 * design and are posted to the worker directly.
 */
export function workerLspTransports(worker: Worker): MessageTransports {
  const port: JsonRpcPort = {
    onmessage: null,
    postMessage: (message: unknown) => worker.postMessage(JSON.stringify(message)),
    addEventListener: (type, handler) => worker.addEventListener(type, handler),
  };
  worker.addEventListener("message", (event: MessageEvent) => {
    let data: unknown = event.data;
    if (typeof data === "string") {
      try {
        data = JSON.parse(data);
      } catch {
        // Not a protocol message. The worker has no other reason to post a
        // string, so this is a corrupt frame; dropping it beats handing the
        // connection something it will reject noisily.
        return;
      }
    }
    // The reader reads `data` and nothing else off the event, so a plain
    // object carrying the parsed message is all it needs.
    port.onmessage?.({ data } as MessageEvent);
  });
  // The reader/writer only ever use the three members `JsonRpcPort` declares;
  // their parameter type names the concrete DOM classes that carry them.
  const asPort = port as unknown as Worker;
  return { reader: new BrowserMessageReader(asPort), writer: new BrowserMessageWriter(asPort) };
}
