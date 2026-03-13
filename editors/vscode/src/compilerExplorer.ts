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
      message?: string;
      stack?: string;
      filename?: string;
      lineno?: number;
      colno?: number;
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
        highlightSourceRange(msg.start, msg.end);
      } else if (msg.type === "clearHighlight") {
        clearSourceHighlight();
      } else if (msg.type === "coreError") {
        console.error(`[compiler-explorer] core load error: ${msg.message ?? "unknown"}`);
        if (msg.stack) {
          console.error(msg.stack);
        }
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

function highlightSourceRange(startOffset: number, endOffset: number): void {
  const editor = explorerEditor;
  if (!editor) {
    return;
  }
  const doc = editor.document;
  const start = doc.positionAt(startOffset);
  const end = doc.positionAt(endOffset);
  const range = new vscode.Range(start, end);
  editor.setDecorations(highlightDecoration, [range]);
  editor.revealRange(range, vscode.TextEditorRevealType.InCenterIfOutsideViewport);
}

function clearSourceHighlight(): void {
  if (explorerEditor) {
    explorerEditor.setDecorations(highlightDecoration, []);
  }
}

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
