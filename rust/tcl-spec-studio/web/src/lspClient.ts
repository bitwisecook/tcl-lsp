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

// The language client: the studio's half of a conversation with the *real* Tcl
// language server, running as `rust/tcl-lsp-server-wasm` in a Web Worker.
//
// There is no protocol code here. The transport is `vscode-jsonrpc`'s browser
// reader/writer pair — the same one every browser-hosted language client uses,
// speaking exactly what the worker was built to speak: one JSON-RPC message per
// `postMessage`, no `Content-Length` framing. The request and notification
// *types* come from `vscode-languageserver-protocol`, so a parameter shape that
// drifts from the specification is a compile error rather than a silent no-op.
//
// What this file adds on top is only the session: the `initialize` handshake
// (with a deadline, because a client that hangs waiting for a server that never
// booted is indistinguishable from a broken page), open-document bookkeeping,
// and a push-diagnostics fan-out. Everything that turns a protocol reply into
// something on screen lives in `monacoHost.ts`.

import { BrowserMessageReader, BrowserMessageWriter } from "vscode-jsonrpc/browser";
import {
  createProtocolConnection,
  CompletionRequest,
  DidChangeTextDocumentNotification,
  DidCloseTextDocumentNotification,
  DidOpenTextDocumentNotification,
  DocumentFormattingRequest,
  ExitNotification,
  HoverRequest,
  InitializedNotification,
  InitializeRequest,
  PublishDiagnosticsNotification,
  SemanticTokensRequest,
  ShutdownRequest,
  SignatureHelpRequest,
  type CompletionItem,
  type CompletionList,
  type Diagnostic,
  type Hover,
  type InitializeParams,
  type InitializeResult,
  type ProtocolConnection,
  type SemanticTokensLegend,
  type SignatureHelp,
  type TextEdit,
} from "vscode-languageserver-protocol/browser";

/** How long `initialize` may take before the page gives up and degrades. */
const BOOT_TIMEOUT_MS = 30_000;

/** Makes a worker. Injectable so a test can hand over a stub. */
export type WorkerFactory = () => Worker;

/** The slice of `MessagePort` that `vscode-jsonrpc`'s browser pair uses. */
interface JsonRpcPort {
  postMessage(message: unknown): void;
  addEventListener(type: string, handler: (event: Event) => void): void;
  onmessage: ((event: MessageEvent) => void) | null;
}

/**
 * Bridge the two halves of "one JSON-RPC message per `postMessage`".
 *
 * The worker's wire format is a **string** in both directions — `worker.js`
 * posts `server.send`'s output verbatim and accepts a string on the fast path.
 * `vscode-jsonrpc`'s `BrowserMessageReader`, though, fires `event.data`
 * straight at the connection as if it were an already-parsed `Message`, and
 * `BrowserMessageWriter` posts the object. Left alone, the two agree on the
 * framing and disagree on the encoding: every reply from the server arrives as
 * a string the connection cannot recognise, so it is dropped, and `initialize`
 * hangs until the deadline with nothing in the console to show for it.
 *
 * Adapting here rather than in the worker keeps the worker's documented
 * contract — strings — intact for every other client of it.
 */
function jsonPort(worker: Worker): JsonRpcPort {
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
        // string, so this is a corrupt frame; dropping it is better than
        // handing the connection something it will reject noisily.
        return;
      }
    }
    // The reader reads `data` and nothing else off the event, so a plain
    // object carrying the parsed message is all it needs.
    port.onmessage?.({ data } as MessageEvent);
  });
  return port;
}

/** One open document's identity and version counter. */
interface OpenDocument {
  uri: string;
  languageId: string;
  version: number;
}

/**
 * A live session with the language server.
 *
 * Construct it with {@link startLspClient}, which resolves only once the server
 * has answered `initialize` — a resolved client is a working one, so no caller
 * has to handle "connected but not ready".
 */
export class LspClient {
  private readonly connection: ProtocolConnection;

  private readonly worker: Worker;

  private readonly open = new Map<string, OpenDocument>();

  private readonly diagnosticHandlers = new Set<(uri: string, items: Diagnostic[]) => void>();

  /** The server's semantic-token legend, straight from `initialize`. */
  readonly legend: SemanticTokensLegend;

  private constructor(
    connection: ProtocolConnection,
    worker: Worker,
    legend: SemanticTokensLegend,
  ) {
    this.connection = connection;
    this.worker = worker;
    this.legend = legend;
    connection.onNotification(PublishDiagnosticsNotification.type, (params) => {
      for (const handler of this.diagnosticHandlers) {
        handler(params.uri, params.diagnostics);
      }
    });
  }

  /** Boot a worker, shake hands, and return the ready client. */
  static async start(makeWorker: WorkerFactory): Promise<LspClient> {
    const worker = makeWorker();
    // `as unknown as Worker`: the reader/writer are typed against `Worker` but
    // only ever touch the three members `JsonRpcPort` declares.
    const port = jsonPort(worker) as unknown as Worker;
    const connection = createProtocolConnection(
      new BrowserMessageReader(port),
      new BrowserMessageWriter(port),
    );
    connection.listen();

    // A worker that fails to fetch its wasm never answers, and a worker that
    // throws during `importScripts` fires `error` — race both against the
    // handshake so a dead server surfaces as a rejection rather than a hang.
    let onWorkerError: ((event: Event) => void) | null = null;
    const failed = new Promise<never>((_resolve, reject) => {
      onWorkerError = (event: Event): void => {
        const message = event instanceof ErrorEvent ? event.message : "worker error";
        reject(new Error(`the language server worker failed: ${message}`));
      };
      worker.addEventListener("error", onWorkerError);
    });
    const timedOut = new Promise<never>((_resolve, reject) => {
      window.setTimeout(
        () => reject(new Error(`the language server did not start within ${BOOT_TIMEOUT_MS}ms`)),
        BOOT_TIMEOUT_MS,
      );
    });

    try {
      const result = (await Promise.race([
        connection.sendRequest(InitializeRequest.type, initializeParams()),
        failed,
        timedOut,
      ])) as InitializeResult;
      connection.sendNotification(InitializedNotification.type, {});
      const legend = result.capabilities.semanticTokensProvider?.legend ?? {
        tokenTypes: [],
        tokenModifiers: [],
      };
      return new LspClient(connection, worker, legend);
    } catch (e) {
      if (onWorkerError) worker.removeEventListener("error", onWorkerError);
      worker.terminate();
      throw e;
    }
  }

  /** Subscribe to push diagnostics. Returns an unsubscribe. */
  onDiagnostics(handler: (uri: string, items: Diagnostic[]) => void): () => void {
    this.diagnosticHandlers.add(handler);
    return () => this.diagnosticHandlers.delete(handler);
  }

  /** Open `uri` (or re-open it under a different language id). */
  openDocument(uri: string, languageId: string, text: string): void {
    const existing = this.open.get(uri);
    if (existing) {
      if (existing.languageId === languageId) {
        this.changeDocument(uri, text);
        return;
      }
      // The dialect *is* the language id, so a dialect change is a close and
      // a re-open — not a configuration round-trip. That keeps one rule for
      // how a document's dialect is decided instead of two that can disagree.
      this.closeDocument(uri);
    }
    this.open.set(uri, { uri, languageId, version: 1 });
    this.connection.sendNotification(DidOpenTextDocumentNotification.type, {
      textDocument: { uri, languageId, version: 1, text },
    });
  }

  /** Push the whole document. Full replacement is legal under either sync kind. */
  changeDocument(uri: string, text: string): void {
    const doc = this.open.get(uri);
    if (!doc) return;
    doc.version += 1;
    this.connection.sendNotification(DidChangeTextDocumentNotification.type, {
      textDocument: { uri, version: doc.version },
      contentChanges: [{ text }],
    });
  }

  closeDocument(uri: string): void {
    if (!this.open.delete(uri)) return;
    this.connection.sendNotification(DidCloseTextDocumentNotification.type, {
      textDocument: { uri },
    });
  }

  async hover(uri: string, line: number, character: number): Promise<Hover | null> {
    return this.connection.sendRequest(HoverRequest.type, {
      textDocument: { uri },
      position: { line, character },
    });
  }

  async completion(
    uri: string,
    line: number,
    character: number,
  ): Promise<CompletionItem[] | CompletionList | null> {
    return this.connection.sendRequest(CompletionRequest.type, {
      textDocument: { uri },
      position: { line, character },
    });
  }

  async signatureHelp(uri: string, line: number, character: number): Promise<SignatureHelp | null> {
    return this.connection.sendRequest(SignatureHelpRequest.type, {
      textDocument: { uri },
      position: { line, character },
    });
  }

  async formatting(
    uri: string,
    tabSize: number,
    insertSpaces: boolean,
  ): Promise<TextEdit[] | null> {
    return this.connection.sendRequest(DocumentFormattingRequest.type, {
      textDocument: { uri },
      options: { tabSize, insertSpaces },
    });
  }

  async semanticTokens(uri: string): Promise<{ data: number[]; resultId?: string } | null> {
    return this.connection.sendRequest(SemanticTokensRequest.type, { textDocument: { uri } });
  }

  /** Stop the server and the worker holding it. */
  async stop(): Promise<void> {
    try {
      await this.connection.sendRequest(ShutdownRequest.type, undefined);
      this.connection.sendNotification(ExitNotification.type);
    } catch {
      // A server that cannot be asked to stop is stopped the blunt way below.
    }
    this.connection.dispose();
    this.worker.terminate();
  }
}

/**
 * Boot the language server worker and return a ready client.
 *
 * `workerDir` is the directory holding the worker's three build outputs. It is
 * a parameter rather than a constant because the studio's dist lays them out
 * under `lsp/` while a test serves a stub from wherever it likes.
 */
export async function startLspClient(workerDir: string): Promise<LspClient> {
  const url = new URL(`${workerDir.replace(/\/$/, "")}/worker.js`, document.baseURI).href;
  // A classic worker, deliberately: the server's glue is built with
  // wasm-bindgen's `--target no-modules`, so `new Worker(url)` needs no
  // `{ type: "module" }` and works in every browser the studio supports.
  return LspClient.start(() => new Worker(url));
}

/**
 * The `initialize` parameters.
 *
 * Only the capabilities the studio actually consumes are declared. Claiming
 * more would be a lie the server acts on — it decides what to compute from
 * what the client says it can render.
 */
function initializeParams(): InitializeParams {
  return {
    processId: null,
    // No workspace: every document the studio has is one it holds in memory.
    // The server's in-memory file store (`vfs_upsert`) is the only "disk".
    rootUri: null,
    workspaceFolders: null,
    clientInfo: { name: "tcl-lsp spec studio", version: "1" },
    capabilities: {
      textDocument: {
        synchronization: { dynamicRegistration: false, didSave: false },
        publishDiagnostics: { relatedInformation: true, versionSupport: false },
        hover: { contentFormat: ["markdown", "plaintext"] },
        completion: {
          completionItem: { documentationFormat: ["markdown", "plaintext"], snippetSupport: true },
        },
        signatureHelp: { signatureInformation: { documentationFormat: ["markdown", "plaintext"] } },
        formatting: { dynamicRegistration: false },
        semanticTokens: {
          dynamicRegistration: false,
          requests: { full: true, range: false },
          // Left empty on purpose: the server answers with its own legend in
          // `initialize`, and the client maps whatever it is offered. Naming a
          // fixed list here would silently drop any token type the server
          // gains later.
          tokenTypes: [],
          tokenModifiers: [],
          formats: ["relative"],
          overlappingTokenSupport: false,
          multilineTokenSupport: true,
        },
      },
      workspace: { configuration: false, workspaceFolders: false },
    },
  };
}
