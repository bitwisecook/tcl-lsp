import * as vscode from "vscode";
import { getClient, isTclLanguage } from "./extension";
import { getTkPreviewHtml } from "./tkPreviewPanelHtml";

let panel: vscode.WebviewPanel | undefined;
let debounceTimer: ReturnType<typeof setTimeout> | undefined;

export function openTkPreview(): void {
  if (panel) {
    panel.reveal(vscode.ViewColumn.Beside);
    refreshPreview();
    return;
  }

  panel = vscode.window.createWebviewPanel("tclTkPreview", "Tk Preview", vscode.ViewColumn.Beside, {
    enableScripts: true,
    retainContextWhenHidden: true,
  });

  panel.webview.html = getTkPreviewHtml();

  panel.webview.onDidReceiveMessage((msg: { type: string }) => {
    if (msg.type === "ready") {
      refreshPreview();
    }
  });

  panel.onDidDispose(() => {
    panel = undefined;
    if (debounceTimer) {
      clearTimeout(debounceTimer);
      debounceTimer = undefined;
    }
  });

  refreshPreview();
}

export function tkPreviewEditorChanged(): void {
  if (!panel) return;
  refreshPreview();
}

export function tkPreviewDocChanged(): void {
  if (!panel) return;
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    debounceTimer = undefined;
    refreshPreview();
  }, 600);
}

function refreshPreview(): void {
  const editor = vscode.window.activeTextEditor;
  if (!editor || !isTclLanguage(editor.document.languageId) || !panel) return;

  const source = editor.document.getText();
  if (!source.includes("package require Tk") && !source.includes("package require tk")) return;

  void runTkPreview(source);
}

async function runTkPreview(source: string): Promise<void> {
  const client = getClient();
  if (!client || !panel) return;

  try {
    void panel.webview.postMessage({
      type: "status",
      text: "Extracting layout...",
    });

    const result = await client.sendRequest("workspace/executeCommand", {
      command: "tcl-lsp.tkPreview",
      arguments: [source],
    });

    if (!panel) return;

    if (result && typeof result === "object") {
      void panel.webview.postMessage({ type: "layout", data: result });
    } else {
      void panel.webview.postMessage({ type: "empty" });
    }
  } catch (err) {
    if (!panel) return;
    const message = err instanceof Error ? err.message : String(err);
    void panel.webview.postMessage({ type: "error", message });
  }
}
