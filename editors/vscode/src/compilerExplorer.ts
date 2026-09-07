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

import * as vscode from "vscode";
import * as fs from "fs";
import * as path from "path";
import { LanguageClient } from "vscode-languageclient/node";
import { getWebviewHtml } from "./compilerExplorerHtml";
import { getActiveDialect, getClient, isTclLanguage } from "./extension";

let panel: vscode.WebviewPanel | undefined;
let debounceTimer: ReturnType<typeof setTimeout> | undefined;

/** Track which editor's document we last compiled so we can highlight in it. */
let explorerEditor: vscode.TextEditor | undefined;

/** Webview readiness handshake to avoid dropping sourceUpdate during startup. */
let webviewReady = false;
let pendingSourceUpdate:
  | {
      source: string;
      dialect: string;
    }
  | undefined;
let compileFallbackTimer: ReturnType<typeof setTimeout> | undefined;
let compileRequestSeq = 0;
let compileCompletedSeq = 0;

/** Deferred resolver for compile-complete signalling (screenshot mode). */
let compileResolver: ((compiled: boolean) => void) | undefined;

/**
 * Wait for the next compilation to complete (result sent to webview).
 * Resolves immediately if no panel is open.  Times out after `timeoutMs`.
 */
export function waitForCompileComplete(timeoutMs = 10_000): Promise<boolean> {
  if (!panel) {
    return Promise.resolve(true);
  }
  return new Promise<boolean>((resolve) => {
    compileResolver = resolve;
    setTimeout(() => {
      if (compileResolver === resolve) {
        compileResolver = undefined;
        resolve(false);
      }
    }, timeoutMs);
  });
}

/** URI scheme for the read-only projection panes the explorer opens. */
const PROJECTION_SCHEME = "tcl-explorer";

/**
 * A rendered projection, keyed by the URI its pane is showing.
 *
 * `lines[n]` is the source span the pane's line `n` points at, so moving the
 * caret in a projection navigates back into the user's file. The entry is
 * replaced whenever the view is re-opened, which is what makes a re-open pick
 * up an edited source.
 */
interface Projection {
  text: string;
  lines: (ExplorerRange | null)[];
  /** The file the projection was rendered from. */
  source: vscode.Uri | undefined;
}

/** A span in the two coordinate systems `range_dict` emits. */
interface ExplorerRange {
  startLine: number;
  startColUtf16: number;
  endLine: number;
  endColUtf16: number;
  startOffset: number;
  endOffset: number;
}

const projections = new Map<string, Projection>();

/**
 * Serves the projection panes.
 *
 * A virtual document rather than an untitled one — which is what the rest of
 * the extension uses for command output — because a projection is derived,
 * read-only, and re-rendered in place: an untitled document would prompt to
 * save and would accumulate a tab per compile.
 */
class ProjectionProvider implements vscode.TextDocumentContentProvider {
  private readonly changed = new vscode.EventEmitter<vscode.Uri>();
  readonly onDidChange = this.changed.event;

  provideTextDocumentContent(uri: vscode.Uri): string {
    return projections.get(uri.toString())?.text ?? "";
  }

  refresh(uri: vscode.Uri): void {
    this.changed.fire(uri);
  }
}

const projectionProvider = new ProjectionProvider();

/**
 * Register the projection machinery. Called once from `activate`.
 *
 * The caret listener is what makes a pane navigable: it maps the caret's line
 * through the projection's line map and reveals the matching span in the file
 * the projection came from.
 */
export function registerCompilerExplorerProjections(context: vscode.ExtensionContext): void {
  context.subscriptions.push(
    vscode.workspace.registerTextDocumentContentProvider(PROJECTION_SCHEME, projectionProvider),
    vscode.window.onDidChangeTextEditorSelection((event) => {
      if (event.textEditor.document.uri.scheme !== PROJECTION_SCHEME) {
        return;
      }
      void revealFromProjection(
        event.textEditor.document.uri,
        event.selections[0]?.active.line ?? 0,
      );
    }),
  );
}

/** Reveal the source span the projection's line `line` points at. */
async function revealFromProjection(uri: vscode.Uri, line: number): Promise<void> {
  const projection = projections.get(uri.toString());
  const range = projection?.lines[line];
  if (!projection?.source || !range) {
    return;
  }
  const document = await vscode.workspace.openTextDocument(projection.source);
  const target = new vscode.Range(
    new vscode.Position(range.startLine, range.startColUtf16),
    new vscode.Position(range.endLine, range.endColUtf16),
  );
  // `preserveFocus` so the caret stays in the projection the user is reading:
  // arrowing down a projection should walk the source alongside it, not tear
  // focus away on every keypress.
  const editor = await vscode.window.showTextDocument(document, {
    viewColumn: vscode.ViewColumn.One,
    preserveFocus: true,
  });
  editor.setDecorations(highlightDecoration, [target]);
  editor.revealRange(target, vscode.TextEditorRevealType.InCenterIfOutsideViewport);
}

/**
 * Render `view` through the server and show it in a read-only editor pane.
 */
async function openProjection(
  view: string,
  label: string,
  source: string,
  dialect: string,
): Promise<void> {
  const client = getClient();
  if (!client) {
    void vscode.window.showWarningMessage(
      "Tcl LSP: the language server is not running, so the view cannot be rendered.",
    );
    return;
  }
  // Captured before the await: `explorerEditor` follows the active editor, so
  // switching files while the render is in flight would otherwise pair this
  // view's line map with a document it was not rendered from, and every caret
  // move in the projection would reveal an unrelated span there.
  const origin = explorerEditor?.document.uri;
  const result = (await client.sendRequest("workspace/executeCommand", {
    command: "tcl-lsp.compilerExplorerView",
    arguments: [source, dialect, view],
  })) as { text?: string; lines?: (ExplorerRange | null)[]; error?: string };

  if (!result || result.error || typeof result.text !== "string") {
    void vscode.window.showWarningMessage(
      `Tcl LSP: could not render the ${label} view${result?.error ? `: ${result.error}` : ""}.`,
    );
    return;
  }

  // One stable URI per view, so re-opening replaces the pane's contents
  // instead of stacking tabs. The label rides in the path because that is what
  // the tab shows.
  const uri = vscode.Uri.parse(
    `${PROJECTION_SCHEME}:${view}.tcl-explorer?${encodeURIComponent(label)}`,
  );
  projections.set(uri.toString(), {
    text: result.text,
    lines: result.lines ?? [],
    source: origin,
  });
  projectionProvider.refresh(uri);

  const document = await vscode.workspace.openTextDocument(uri);
  await vscode.window.showTextDocument(document, {
    viewColumn: vscode.ViewColumn.Beside,
    preview: false,
  });
}

const highlightDecoration = vscode.window.createTextEditorDecorationType({
  backgroundColor: new vscode.ThemeColor("editor.selectionBackground"),
  isWholeLine: false,
});

export function openCompilerExplorer(): void {
  if (panel) {
    panel.reveal(vscode.ViewColumn.Beside, true);
    pushSourceFromActiveEditor();
    return;
  }

  panel = vscode.window.createWebviewPanel(
    "tclCompilerExplorer",
    "Tcl Compiler Explorer",
    { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
    { enableScripts: true, retainContextWhenHidden: true },
  );

  webviewReady = false;
  pendingSourceUpdate = undefined;

  panel.webview.onDidReceiveMessage(
    async (msg: {
      type: string;
      source?: string;
      dialect?: string;
      start?: number;
      end?: number;
      startLine?: number;
      startCol?: number;
      endLine?: number;
      endCol?: number;
      view?: string;
      label?: string;
      message?: string;
      stack?: string;
      filename?: string;
      lineno?: number;
      colno?: number;
      panes?: string[];
    }) => {
      console.log(`[compiler-explorer] webview message: ${msg.type}`);
      if (msg.type === "ready") {
        // Webview JS has loaded and is ready to receive messages.
        webviewReady = true;
        if (pendingSourceUpdate) {
          postSourceUpdate(pendingSourceUpdate);
        } else {
          pushSourceFromActiveEditor();
        }
      } else if (msg.type === "compile" && msg.source) {
        if (compileFallbackTimer) {
          clearTimeout(compileFallbackTimer);
          compileFallbackTimer = undefined;
        }
        const requestSeq = ++compileRequestSeq;
        await runCompile(msg.source, msg.dialect ?? getActiveDialect(), requestSeq);
      } else if (
        msg.type === "highlightSource" &&
        msg.start !== undefined &&
        msg.end !== undefined
      ) {
        highlightSourceRange(msg);
      } else if (
        msg.type === "openProjection" &&
        typeof msg.view === "string" &&
        typeof msg.source === "string"
      ) {
        await openProjection(
          msg.view,
          typeof msg.label === "string" ? msg.label : msg.view,
          msg.source,
          typeof msg.dialect === "string" ? msg.dialect : getActiveDialect(),
        );
      } else if (msg.type === "clearHighlight") {
        clearSourceHighlight();
      } else if (msg.type === "coreError") {
        console.error(`[compiler-explorer] core load error: ${msg.message ?? "unknown"}`);
        if (msg.stack) {
          console.error(msg.stack);
        }
      } else if (msg.type === "renderError") {
        // One or more panes threw while rendering a result. The webview
        // still shows the other tabs and clears its spinner; surface the
        // detail here so the failure is diagnosable rather than silent.
        console.error(`[compiler-explorer] pane render failure: ${msg.message ?? "unknown"}`);
      } else if (msg.type === "scriptError") {
        console.error(
          `[compiler-explorer] webview script error: ${msg.message ?? "unknown"} (${msg.filename ?? ""}:${msg.lineno ?? 0}:${msg.colno ?? 0})`,
        );
        if (msg.stack) {
          console.error(msg.stack);
        }
      } else if (msg.type === "scriptRejection") {
        console.error(
          `[compiler-explorer] webview unhandled rejection: ${msg.message ?? "unknown"}`,
        );
        if (msg.stack) {
          console.error(msg.stack);
        }
      }
    },
  );

  panel.onDidDispose(() => {
    panel = undefined;
    explorerEditor = undefined;
    if (debounceTimer) {
      clearTimeout(debounceTimer);
      debounceTimer = undefined;
    }
    if (compileFallbackTimer) {
      clearTimeout(compileFallbackTimer);
      compileFallbackTimer = undefined;
    }
    webviewReady = false;
    pendingSourceUpdate = undefined;
    compileRequestSeq = 0;
    compileCompletedSeq = 0;
  });

  // Set HTML after message handlers are wired so the initial "ready" post
  // cannot race and get dropped.
  const html = getWebviewHtml();
  const screenshotOutputDir = process.env.SCREENSHOT_OUTPUT_DIR;
  if (screenshotOutputDir) {
    try {
      fs.writeFileSync(
        path.join(screenshotOutputDir, "compiler-explorer-debug.html"),
        html,
        "utf8",
      );
    } catch (err) {
      console.error("[compiler-explorer] Failed to write debug HTML:", err);
    }
  }
  panel.webview.html = html;
}

/** Switch the compiler explorer webview to a specific tab. */
export function switchCompilerExplorerTab(tabId: string): void {
  if (panel) {
    void panel.webview.postMessage({ type: "switchTab", tabId });
  }
}

/** Close the compiler explorer panel if open. */
export function closeCompilerExplorer(): void {
  if (panel) {
    panel.dispose();
  }
}

/**
 * Force a compile from the currently active Tcl editor.
 *
 * Used by screenshot automation when the webview startup handshake misses
 * compile triggers and the panel stays on "Waiting for source...".
 */
export async function forceCompileFromActiveEditor(): Promise<boolean> {
  if (!panel) {
    return false;
  }
  let editor = vscode.window.activeTextEditor;
  if (!editor || !isTclLanguage(editor.document.languageId)) {
    editor = vscode.window.visibleTextEditors.find((e) => isTclLanguage(e.document.languageId));
  }
  if (!editor) {
    return false;
  }

  const update = {
    source: editor.document.getText(),
    dialect: getActiveDialect(),
  };
  explorerEditor = editor;
  pendingSourceUpdate = update;

  const delivered = await panel.webview.postMessage({
    type: "sourceUpdate",
    source: update.source,
    dialect: update.dialect,
  });
  const requestSeq = ++compileRequestSeq;
  await runCompile(update.source, update.dialect, requestSeq);
  pendingSourceUpdate = undefined;
  return delivered;
}

export function explorerEditorChanged(): void {
  if (!panel) {
    return;
  }
  pushSourceFromActiveEditor();
}

export function explorerDocChanged(): void {
  if (!panel) {
    return;
  }
  if (debounceTimer) {
    clearTimeout(debounceTimer);
  }
  debounceTimer = setTimeout(() => {
    debounceTimer = undefined;
    pushSourceFromActiveEditor();
  }, 600);
}

function pushSourceFromActiveEditor(): void {
  let editor = vscode.window.activeTextEditor;
  if (!editor || !isTclLanguage(editor.document.languageId)) {
    // When a webview panel has focus, activeTextEditor can be undefined.
    // Fall back to the most recently active visible Tcl editor.
    editor = vscode.window.visibleTextEditors.find((e) => isTclLanguage(e.document.languageId));
  }
  if (!editor || !panel) {
    return;
  }
  explorerEditor = editor;
  postSourceUpdate({
    source: editor.document.getText(),
    dialect: getActiveDialect(),
  });
}

function postSourceUpdate(update: { source: string; dialect: string }): void {
  if (!panel) {
    return;
  }
  pendingSourceUpdate = update;
  if (!webviewReady) {
    // Optimistically send once before ready — some webview loads accept this
    // early. Keep `pendingSourceUpdate` intact so ready-time replay still
    // happens if this message is dropped.
    const requestSeq = ++compileRequestSeq;
    if (compileFallbackTimer) {
      clearTimeout(compileFallbackTimer);
    }
    compileFallbackTimer = setTimeout(() => {
      if (!panel || compileCompletedSeq >= requestSeq) {
        return;
      }
      console.log("[compiler-explorer] Forcing compile fallback before webview ready");
      void runCompile(update.source, update.dialect, requestSeq);
    }, 1_200);
    void panel.webview.postMessage({
      type: "sourceUpdate",
      source: update.source,
      dialect: update.dialect,
    });
    return;
  }
  if (compileFallbackTimer) {
    clearTimeout(compileFallbackTimer);
  }
  const requestSeq = ++compileRequestSeq;
  compileFallbackTimer = setTimeout(() => {
    // Some webview boots never emit compile after sourceUpdate; force one so
    // the explorer cannot stay stuck on "Waiting for source from editor...".
    if (!panel || compileCompletedSeq >= requestSeq) {
      return;
    }
    console.log("[compiler-explorer] Forcing compile fallback after sourceUpdate");
    void runCompile(update.source, update.dialect, requestSeq);
  }, 900);
  void panel.webview
    .postMessage({ type: "sourceUpdate", source: update.source, dialect: update.dialect })
    .then((delivered) => {
      if (delivered && pendingSourceUpdate === update) {
        pendingSourceUpdate = undefined;
      }
    });
}

/**
 * Highlight the source span the webview is hovering.
 *
 * The explorer's `startOffset`/`endOffset` count **bytes** (the payload's
 * columns come from `LineIndex::position_at`, which returns a byte column),
 * while `Position` and `positionAt` are UTF-16. Feeding one to the other is
 * correct only for ASCII and drifts by one position per non-ASCII byte
 * thereafter, so prefer the UTF-16 line/column pair the payload now carries
 * and keep the offsets as the fallback for an older payload.
 */
function highlightSourceRange(msg: {
  start?: number;
  end?: number;
  startLine?: number;
  startCol?: number;
  endLine?: number;
  endCol?: number;
}): void {
  const editor = explorerEditor;
  if (!editor) {
    return;
  }
  const doc = editor.document;
  const range =
    msg.startLine !== undefined &&
    msg.startCol !== undefined &&
    msg.endLine !== undefined &&
    msg.endCol !== undefined
      ? new vscode.Range(
          new vscode.Position(msg.startLine, msg.startCol),
          new vscode.Position(msg.endLine, msg.endCol),
        )
      : new vscode.Range(doc.positionAt(msg.start ?? 0), doc.positionAt(msg.end ?? 0));
  editor.setDecorations(highlightDecoration, [range]);
  editor.revealRange(range, vscode.TextEditorRevealType.InCenterIfOutsideViewport);
}

function clearSourceHighlight(): void {
  if (explorerEditor) {
    explorerEditor.setDecorations(highlightDecoration, []);
  }
}

// Host-brokered compile via the LSP `tcl-lsp.compilerExplorer` command. This is
// now the *fallback* path: when the webview bundles the Rust → WASM module it
// compiles in-process and never posts `{type:"compile"}`, so this only runs in
// dev builds without `make explorer-wasm` (and requires the Python LSP server,
// which the native Rust server does not yet implement).
async function runCompile(source: string, dialect: string, requestSeq: number): Promise<void> {
  const client: LanguageClient = getClient();
  if (!client || !panel) {
    compileCompletedSeq = Math.max(compileCompletedSeq, requestSeq);
    if (compileResolver) {
      const resolve = compileResolver;
      compileResolver = undefined;
      resolve(false);
    }
    return;
  }

  try {
    void panel.webview.postMessage({ type: "status", text: "Compiling..." });

    const result = await client.sendRequest("workspace/executeCommand", {
      command: "tcl-lsp.compilerExplorer",
      arguments: [source, dialect],
    });

    if (!panel) {
      return;
    }

    if (!result || typeof result !== "object") {
      void panel.webview.postMessage({
        type: "error",
        data: {
          error:
            "Compiler explorer did not receive a structured result. " +
            "Check for source script issues in the active editor.",
        },
      });
      return;
    }

    const resultRecord = result as Record<string, unknown>;
    if ("error" in resultRecord) {
      const baseError =
        typeof resultRecord.error === "string" ? resultRecord.error : "Compiler explorer failed.";
      const details = typeof resultRecord.details === "string" ? resultRecord.details : "";
      const traceback = typeof resultRecord.traceback === "string" ? resultRecord.traceback : "";
      const diagnosticsText = Array.isArray(resultRecord.diagnostics)
        ? resultRecord.diagnostics
            .map((entry) => {
              if (!entry || typeof entry !== "object") {
                return "";
              }
              const diag = entry as Record<string, unknown>;
              const code = typeof diag.code === "string" ? diag.code : "E000";
              const line = typeof diag.line === "number" ? diag.line : 0;
              const column = typeof diag.column === "number" ? diag.column : 0;
              const message = typeof diag.message === "string" ? diag.message : "Unknown issue";
              return `${code} (${line}:${column}) ${message}`;
            })
            .filter((line) => line.length > 0)
            .join("\n")
        : "";
      const errorText = [baseError, details, diagnosticsText].filter((part) => part).join("\n\n");
      void panel.webview.postMessage({
        type: "error",
        data: {
          error: errorText,
          traceback,
        },
      });
      return;
    }

    void panel.webview.postMessage({ type: "result", data: result });
  } catch (err) {
    if (!panel) {
      return;
    }
    const message = err instanceof Error ? err.message : String(err);
    void panel.webview.postMessage({ type: "error", data: { error: message } });
  } finally {
    compileCompletedSeq = Math.max(compileCompletedSeq, requestSeq);
    // Signal any waiters that compilation is done (screenshot mode).
    if (compileResolver) {
      const resolve = compileResolver;
      compileResolver = undefined;
      resolve(true);
    }
  }
}
