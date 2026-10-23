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
import {
  snapshotDocumentSource,
  isDocumentSourceSnapshotCurrent,
  type DocumentSourceSnapshot,
} from "./chat/codeUtils";
import { getActiveDialect, getClient, isTclLanguage } from "./extension";

let panel: vscode.WebviewPanel | undefined;
let panelGeneration = 0;
let nextPanelGeneration = 0;
let nextSourceBinding = 0;
let nextCompileToken = 0;
let currentSourceBinding: ExplorerSourceBinding | undefined;
let latestCompileToken: string | undefined;
let activeCompile: CompileLease | undefined;
let debounceTimer: ReturnType<typeof setTimeout> | undefined;

/** Webview readiness handshake to avoid dropping sourceUpdate during startup. */
let webviewReady = false;
let pendingSourceUpdate: SourceUpdate | undefined;
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
interface ExplorerSourceBinding {
  readonly id: string;
  readonly panelGeneration: number;
  readonly snapshot: DocumentSourceSnapshot;
}

interface SourceUpdate {
  readonly source: string;
  readonly dialect: string;
  readonly binding: ExplorerSourceBinding;
  readonly compile: boolean;
  readonly compileToken?: string;
}

interface CompileLease {
  readonly token: string;
  readonly panelGeneration: number;
  readonly requestSeq: number;
  readonly source: string;
  readonly dialect: string;
  readonly binding?: ExplorerSourceBinding;
}

interface Projection {
  text: string;
  lines: (ExplorerRange | null)[];
  /** Present only when exact current editor bytes own the mapped ranges. */
  binding?: ExplorerSourceBinding;
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

function createSourceUpdate(
  editor: vscode.TextEditor,
  compile = true,
  compileToken?: string,
): SourceUpdate {
  const snapshot = snapshotDocumentSource(editor.document);
  const binding: ExplorerSourceBinding = {
    id: `${panelGeneration}:${++nextSourceBinding}`,
    panelGeneration,
    snapshot,
  };
  currentSourceBinding = binding;
  latestCompileToken = compileToken;
  activeCompile = undefined;
  ++compileRequestSeq;
  return {
    source: snapshot.fullSource,
    dialect: getActiveDialect(),
    binding,
    compile,
    compileToken,
  };
}

function isSourceBindingCurrent(binding: ExplorerSourceBinding): boolean {
  const snapshot = binding.snapshot;
  return (
    panel !== undefined &&
    binding.panelGeneration === panelGeneration &&
    currentSourceBinding === binding &&
    isDocumentSourceSnapshotCurrent(snapshot)
  );
}

function bindingForMessage(
  sourceToken: unknown,
  source: string,
): ExplorerSourceBinding | undefined {
  const binding = currentSourceBinding;
  return binding &&
    typeof sourceToken === "string" &&
    sourceToken === binding.id &&
    source === binding.snapshot.fullSource
    ? binding
    : undefined;
}

function sameUri(left: vscode.Uri, right: vscode.Uri): boolean {
  return left.toString() === right.toString();
}

function isExplorerRange(value: unknown): value is ExplorerRange {
  if (!value || typeof value !== "object") {
    return false;
  }
  const range = value as Record<string, unknown>;
  return ["startLine", "startColUtf16", "endLine", "endColUtf16", "startOffset", "endOffset"].every(
    (key) => typeof range[key] === "number" && Number.isSafeInteger(range[key]),
  );
}

function beginCompile(
  token: string,
  source: string,
  dialect: string,
  binding?: ExplorerSourceBinding,
): CompileLease {
  const lease: CompileLease = {
    token,
    panelGeneration,
    requestSeq: ++compileRequestSeq,
    source,
    dialect,
    binding,
  };
  latestCompileToken = token;
  activeCompile = lease;
  return lease;
}

function isCompileLeaseCurrent(lease: CompileLease): boolean {
  return (
    panel !== undefined &&
    lease.panelGeneration === panelGeneration &&
    activeCompile === lease &&
    lease.requestSeq === compileRequestSeq &&
    latestCompileToken === lease.token &&
    (!lease.binding || isSourceBindingCurrent(lease.binding))
  );
}

function isCurrentProjectionBinding(binding: ExplorerSourceBinding | undefined): boolean {
  return binding !== undefined && isSourceBindingCurrent(binding);
}

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
  const binding = projection?.binding;
  if (!binding || !range || !isCurrentProjectionBinding(binding)) {
    return;
  }
  const snapshot = binding.snapshot;
  const sourceDocument = snapshot.document;
  const target = explorerRangeToRange(range, sourceDocument);
  if (!target) {
    return;
  }
  const document = await vscode.workspace.openTextDocument(snapshot.uri);
  if (
    !isCurrentProjectionBinding(binding) ||
    !sameUri(document.uri, snapshot.uri) ||
    document.version !== snapshot.version ||
    document.getText() !== snapshot.fullSource
  ) {
    return;
  }
  // `preserveFocus` so the caret stays in the projection the user is reading:
  // arrowing down a projection should walk the source alongside it, not tear
  // focus away on every keypress.
  const editor = await vscode.window.showTextDocument(document, {
    viewColumn: vscode.ViewColumn.One,
    preserveFocus: true,
  });
  if (!isCurrentProjectionBinding(binding) || editor.document !== document) {
    return;
  }
  editor.setDecorations(highlightDecoration, [target]);
  editor.revealRange(target, vscode.TextEditorRevealType.InCenterIfOutsideViewport);
}

function explorerRangeToRange(
  range: ExplorerRange,
  document: vscode.TextDocument,
): vscode.Range | undefined {
  const coordinates = [range.startLine, range.startColUtf16, range.endLine, range.endColUtf16];
  if (
    !coordinates.every(Number.isSafeInteger) ||
    range.startOffset < 0 ||
    range.endOffset < range.startOffset ||
    range.endOffset > Buffer.byteLength(document.getText(), "utf8")
  ) {
    return undefined;
  }
  if (
    range.startLine < 0 ||
    range.endLine < range.startLine ||
    range.endLine >= document.lineCount
  ) {
    return undefined;
  }
  const start = new vscode.Position(range.startLine, range.startColUtf16);
  const end = new vscode.Position(range.endLine, range.endColUtf16);
  const startLine = document.lineAt(start.line);
  const endLine = document.lineAt(end.line);
  if (
    start.character < 0 ||
    end.character < 0 ||
    start.character > startLine.text.length ||
    end.character > endLine.text.length ||
    document.offsetAt(end) < document.offsetAt(start)
  ) {
    return undefined;
  }
  return new vscode.Range(start, end);
}

/**
 * Render `view` through the server and show it in a read-only editor pane.
 */
async function openProjection(
  view: string,
  label: string,
  source: string,
  dialect: string,
  sourceToken: unknown,
  compileToken: unknown,
): Promise<void> {
  const client = getClient();
  const lease = activeCompile;
  if (
    !client ||
    !lease ||
    typeof compileToken !== "string" ||
    lease.token !== compileToken ||
    !isCompileLeaseCurrent(lease) ||
    source !== lease.source ||
    dialect !== lease.dialect
  ) {
    return;
  }
  const binding = lease.binding;
  const exactBinding =
    binding &&
    sourceToken === binding.id &&
    source === binding.snapshot.fullSource &&
    isCurrentProjectionBinding(binding)
      ? binding
      : undefined;
  const result = (await client.sendRequest("workspace/executeCommand", {
    command: "tcl-lsp.compilerExplorerView",
    arguments: exactBinding
      ? [
          source,
          dialect,
          view,
          { uri: exactBinding.snapshot.uri.toString(), version: exactBinding.snapshot.version },
        ]
      : [source, dialect, view],
  })) as { text?: string; lines?: (ExplorerRange | null)[]; error?: string };

  if (!isCompileLeaseCurrent(lease)) {
    return;
  }
  if (!result || result.error || typeof result.text !== "string") {
    void vscode.window.showWarningMessage(
      `Tcl LSP: could not render the ${label} view${result?.error ? `: ${result.error}` : ""}.`,
    );
    return;
  }

  const uri = vscode.Uri.parse(
    `${PROJECTION_SCHEME}:${view}.tcl-explorer?${encodeURIComponent(label)}`,
  );
  const lines = Array.isArray(result.lines) ? result.lines : [];
  const safeLines =
    exactBinding && lines.length === result.text.split("\n").length
      ? lines.map((range) =>
          range === null
            ? null
            : isExplorerRange(range) && explorerRangeToRange(range, exactBinding.snapshot.document)
              ? range
              : null,
        )
      : [];
  projections.set(uri.toString(), {
    text: result.text,
    lines: safeLines,
    binding: exactBinding,
  });
  projectionProvider.refresh(uri);

  const document = await vscode.workspace.openTextDocument(uri);
  if (!isCompileLeaseCurrent(lease)) {
    return;
  }
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

  const registeredPanel = vscode.window.createWebviewPanel(
    "tclCompilerExplorer",
    "Tcl Compiler Explorer",
    { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
    { enableScripts: true, retainContextWhenHidden: true },
  );
  panel = registeredPanel;
  const registeredGeneration = ++nextPanelGeneration;
  panelGeneration = registeredGeneration;
  currentSourceBinding = undefined;
  latestCompileToken = undefined;
  activeCompile = undefined;

  webviewReady = false;
  pendingSourceUpdate = undefined;

  registeredPanel.webview.onDidReceiveMessage(
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
      sourceToken?: string;
      compileToken?: string;
      compile?: boolean;
      hostRequestSeq?: number;
    }) => {
      if (panel !== registeredPanel || panelGeneration !== registeredGeneration) {
        return;
      }
      console.log(`[compiler-explorer] webview message: ${msg.type}`);
      if (msg.type === "ready") {
        // Webview JS has loaded and is ready to receive messages.
        webviewReady = true;
        if (pendingSourceUpdate) {
          postSourceUpdate(pendingSourceUpdate);
        } else {
          pushSourceFromActiveEditor();
        }
      } else if (msg.type === "compileCurrentnessCheck" && typeof msg.compileToken === "string") {
        const lease = activeCompile;
        const sourceTokenMatches = lease?.binding
          ? msg.sourceToken === lease.binding.id
          : msg.sourceToken === undefined;
        const accepted =
          lease !== undefined &&
          lease.token === msg.compileToken &&
          sourceTokenMatches &&
          isCompileLeaseCurrent(lease);
        void registeredPanel.webview.postMessage({
          type: accepted ? "compileCurrentnessAccepted" : "compileCurrentnessRejected",
          compileToken: msg.compileToken,
          hostRequestSeq: lease?.requestSeq,
        });
      } else if (msg.type === "compileStarted" && typeof msg.source === "string") {
        const sourceBinding = currentSourceBinding;
        if (
          (sourceBinding &&
            (msg.sourceToken !== sourceBinding.id ||
              msg.source !== sourceBinding.snapshot.fullSource)) ||
          (!sourceBinding && msg.sourceToken !== undefined)
        ) {
          return;
        }
        if (typeof msg.compileToken !== "string") {
          return;
        }
        if (compileFallbackTimer) {
          clearTimeout(compileFallbackTimer);
          compileFallbackTimer = undefined;
        }
        beginCompile(
          msg.compileToken,
          msg.source,
          msg.dialect ?? getActiveDialect(),
          bindingForMessage(msg.sourceToken, msg.source),
        );
      } else if (msg.type === "compile" && typeof msg.source === "string") {
        const lease = activeCompile;
        if (
          !lease ||
          lease.token !== msg.compileToken ||
          lease.source !== msg.source ||
          lease.dialect !== (msg.dialect ?? getActiveDialect()) ||
          !isCompileLeaseCurrent(lease)
        ) {
          return;
        }
        await runCompile(lease);
      } else if (msg.type === "compileComplete" && typeof msg.compileToken === "string") {
        const lease = activeCompile;
        if (lease?.token === msg.compileToken && isCompileLeaseCurrent(lease)) {
          finishCompile(lease, true);
        }
      } else if (msg.type === "highlightSource") {
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
          msg.sourceToken,
          msg.compileToken,
        );
      } else if (msg.type === "clearHighlight") {
        clearSourceHighlight(msg.sourceToken, msg.compileToken);
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

  registeredPanel.onDidDispose(() => {
    if (panel !== registeredPanel || panelGeneration !== registeredGeneration) {
      return;
    }
    panel = undefined;
    currentSourceBinding = undefined;
    latestCompileToken = undefined;
    activeCompile = undefined;
    projections.clear();
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
  registeredPanel.webview.html = html;
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

  const compilePanel = panel;
  const compileGeneration = panelGeneration;
  const compileToken = `force:${panelGeneration}:${++nextCompileToken}`;
  const update = createSourceUpdate(editor, false, compileToken);
  pendingSourceUpdate = update;
  const delivered = await compilePanel.webview.postMessage({
    type: "sourceUpdate",
    source: update.source,
    dialect: update.dialect,
    sourceToken: update.binding.id,
    compile: false,
    compileToken,
  });
  if (
    panel !== compilePanel ||
    panelGeneration !== compileGeneration ||
    currentSourceBinding !== update.binding
  ) {
    return delivered;
  }
  const lease = beginCompile(compileToken, update.source, update.dialect, update.binding);
  await runCompile(lease);
  if (pendingSourceUpdate === update) {
    pendingSourceUpdate = undefined;
  }
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
  postSourceUpdate(createSourceUpdate(editor));
}

function postSourceUpdate(update: SourceUpdate): void {
  if (!panel) {
    return;
  }
  pendingSourceUpdate = update;
  const send = (keepPending: boolean) => {
    if (!panel || currentSourceBinding !== update.binding) {
      return;
    }
    void panel.webview
      .postMessage({
        type: "sourceUpdate",
        source: update.source,
        dialect: update.dialect,
        sourceToken: update.binding.id,
        compile: update.compile,
        compileToken: update.compileToken,
      })
      .then((delivered) => {
        if (delivered && !keepPending && pendingSourceUpdate === update) {
          pendingSourceUpdate = undefined;
        }
      });
  };
  const scheduleFallback = (delay: number) => {
    if (compileFallbackTimer) {
      clearTimeout(compileFallbackTimer);
    }
    compileFallbackTimer = setTimeout(() => {
      compileFallbackTimer = undefined;
      if (!panel || currentSourceBinding !== update.binding || latestCompileToken !== undefined) {
        return;
      }
      const token = `fallback:${update.binding.id}:${++nextCompileToken}`;
      const lease = beginCompile(token, update.source, update.dialect, update.binding);
      const targetPanel = panel;
      if (!targetPanel) return;
      void targetPanel.webview
        .postMessage({
          type: "compileFallbackStarted",
          source: update.source,
          dialect: update.dialect,
          sourceToken: update.binding.id,
          compileToken: token,
        })
        .then(() => {
          if (isCompileLeaseCurrent(lease)) void runCompile(lease);
        });
    }, delay);
  };

  if (!webviewReady) {
    // Keep the captured source and binding for the ready-time replay if the
    // optimistic message is dropped during startup.
    send(true);
    if (update.compile) scheduleFallback(1_200);
    return;
  }

  if (update.compile) scheduleFallback(900);
  send(false);
}

/**
 * Accept only the renderer's paired UTF-16 coordinates. Byte offsets are
 * bounded against the exact source snapshot separately and are never lifted
 * into VS Code positions locally.
 */
function utf16Range(
  document: vscode.TextDocument,
  startLine: unknown,
  startCol: unknown,
  endLine: unknown,
  endCol: unknown,
): vscode.Range | undefined {
  const coordinates = [startLine, startCol, endLine, endCol];
  if (!coordinates.every((value) => typeof value === "number" && Number.isSafeInteger(value))) {
    return undefined;
  }
  const start = new vscode.Position(startLine as number, startCol as number);
  const end = new vscode.Position(endLine as number, endCol as number);
  if (
    start.line < 0 ||
    end.line < start.line ||
    end.line >= document.lineCount ||
    start.character < 0 ||
    end.character < 0 ||
    start.character > document.lineAt(start.line).text.length ||
    end.character > document.lineAt(end.line).text.length ||
    document.offsetAt(end) < document.offsetAt(start)
  ) {
    return undefined;
  }
  return new vscode.Range(start, end);
}

function highlightSourceRange(msg: {
  sourceToken?: string;
  compileToken?: string;
  start?: number;
  end?: number;
  startLine?: number;
  startCol?: number;
  endLine?: number;
  endCol?: number;
}): void {
  const binding = currentSourceBinding;
  const lease = activeCompile;
  if (
    !binding ||
    !lease ||
    lease.binding !== binding ||
    msg.sourceToken !== binding.id ||
    msg.compileToken !== latestCompileToken ||
    !isCompileLeaseCurrent(lease) ||
    !isSourceBindingCurrent(binding)
  ) {
    return;
  }
  const document = binding.snapshot.document;
  if (
    !Number.isSafeInteger(msg.start) ||
    !Number.isSafeInteger(msg.end) ||
    (msg.start as number) < 0 ||
    (msg.end as number) < (msg.start as number) ||
    (msg.end as number) > Buffer.byteLength(binding.snapshot.fullSource, "utf8")
  ) {
    return;
  }
  const editor = vscode.window.visibleTextEditors.find(
    (candidate) => candidate.document === document,
  );
  if (!editor) {
    return;
  }
  const range = utf16Range(document, msg.startLine, msg.startCol, msg.endLine, msg.endCol);
  if (!range) {
    return;
  }
  editor.setDecorations(highlightDecoration, [range]);
  editor.revealRange(range, vscode.TextEditorRevealType.InCenterIfOutsideViewport);
}

function clearSourceHighlight(sourceToken: unknown, compileToken: unknown): void {
  const binding = currentSourceBinding;
  const lease = activeCompile;
  if (
    !binding ||
    !lease ||
    lease.binding !== binding ||
    sourceToken !== binding.id ||
    compileToken !== latestCompileToken ||
    !isCompileLeaseCurrent(lease)
  ) {
    return;
  }
  const editor = vscode.window.visibleTextEditors.find(
    (candidate) => candidate.document === binding.snapshot.document,
  );
  editor?.setDecorations(highlightDecoration, []);
}

function finishCompile(lease: CompileLease, compiled: boolean): void {
  if (!isCompileLeaseCurrent(lease)) {
    return;
  }
  compileCompletedSeq = Math.max(compileCompletedSeq, lease.requestSeq);
  if (compileResolver) {
    const resolve = compileResolver;
    compileResolver = undefined;
    resolve(compiled);
  }
}

async function runCompile(lease: CompileLease): Promise<void> {
  const client: LanguageClient | undefined = getClient();
  const currentPanel = panel;
  if (!client || !currentPanel || !isCompileLeaseCurrent(lease)) {
    finishCompile(lease, false);
    return;
  }

  const messageContext = {
    compileToken: lease.token,
    sourceToken: lease.binding?.id,
    hostRequestSeq: lease.requestSeq,
  };
  try {
    void currentPanel.webview.postMessage({
      type: "status",
      text: "Compiling...",
      ...messageContext,
    });

    const result = await client.sendRequest("workspace/executeCommand", {
      command: "tcl-lsp.compilerExplorer",
      arguments: lease.binding
        ? [
            lease.source,
            lease.dialect,
            {
              uri: lease.binding.snapshot.uri.toString(),
              version: lease.binding.snapshot.version,
            },
          ]
        : [lease.source, lease.dialect],
    });

    if (!isCompileLeaseCurrent(lease)) {
      return;
    }

    if (!result || typeof result !== "object") {
      void currentPanel.webview.postMessage({
        type: "error",
        data: {
          error:
            "Compiler explorer did not receive a structured result. " +
            "Check for source script issues in the active editor.",
        },
        ...messageContext,
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
      void currentPanel.webview.postMessage({
        type: "error",
        data: { error: errorText, traceback },
        ...messageContext,
      });
      return;
    }

    void currentPanel.webview.postMessage({ type: "result", data: result, ...messageContext });
  } catch (err) {
    if (!isCompileLeaseCurrent(lease)) {
      return;
    }
    const message = err instanceof Error ? err.message : String(err);
    void currentPanel.webview.postMessage({
      type: "error",
      data: { error: message },
      ...messageContext,
    });
  } finally {
    finishCompile(lease, true);
  }
}
