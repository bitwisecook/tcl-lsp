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

import { randomBytes } from "crypto";
import * as vscode from "vscode";
import { getClient, isTclLanguage } from "./extension";
import { getTkPreviewHtml } from "./tkPreviewPanelHtml";

let panel: vscode.WebviewPanel | undefined;
let debounceTimer: ReturnType<typeof setTimeout> | undefined;
let renderedDocument: { uri: string; version: number } | undefined;
let requestSerial = 0;

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

  const nonce = randomBytes(16).toString("hex");
  panel.webview.html = getTkPreviewHtml(panel.webview.cspSource, nonce);

  panel.webview.onDidReceiveMessage((msg: { type: string; start?: number; end?: number }) => {
    if (msg.type === "ready") {
      refreshPreview();
    } else if (msg.type === "revealSource") {
      void revealSource(msg.start, msg.end);
    }
  });

  panel.onDidDispose(() => {
    panel = undefined;
    renderedDocument = undefined;
    requestSerial += 1;
    if (debounceTimer) {
      clearTimeout(debounceTimer);
      debounceTimer = undefined;
    }
  });

  refreshPreview();
}

export function tkPreviewEditorChanged(): void {
  if (!panel) return;
  requestSerial += 1;
  renderedDocument = undefined;
  refreshPreview();
}

export function tkPreviewDocChanged(): void {
  if (!panel) return;
  requestSerial += 1;
  renderedDocument = undefined;
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    debounceTimer = undefined;
    refreshPreview();
  }, 600);
}

function refreshPreview(): void {
  const editor = vscode.window.activeTextEditor;
  if (!panel) return;
  if (!editor || !isTclLanguage(editor.document.languageId)) {
    requestSerial += 1;
    renderedDocument = undefined;
    void panel.webview.postMessage({
      type: "unavailable",
      message: "Tk preview is available for an active Tcl editor.",
    });
    return;
  }

  void runTkPreview(editor.document);
}

async function runTkPreview(document: vscode.TextDocument): Promise<void> {
  const client = getClient();
  if (!panel) return;
  if (!client) {
    requestSerial += 1;
    renderedDocument = undefined;
    void panel.webview.postMessage({
      type: "unavailable",
      message: "Tk preview is waiting for the Tcl language server.",
    });
    return;
  }
  const uri = document.uri.toString();
  const version = document.version;
  const request = ++requestSerial;

  try {
    void panel.webview.postMessage({
      type: "status",
      text: "Extracting layout...",
    });

    const result = await client.sendRequest("workspace/executeCommand", {
      command: "tcl-lsp.tkPreview",
      arguments: [{ uri, version }],
    });

    if (!panel || request !== requestSerial) return;

    if (result && typeof result === "object") {
      const model = result as {
        schema_version?: unknown;
        document_uri?: unknown;
        document_version?: unknown;
      };
      const active = vscode.window.activeTextEditor?.document;
      if (!active || active.uri.toString() !== uri || active.version !== version) {
        // A newer edit or editor selection won the race. Its own change event
        // has already queued a fresh request, so never paint this old model.
        return;
      }
      if (model.schema_version !== 1) {
        void panel.webview.postMessage({
          type: "error",
          message: `Unsupported Tk preview schema: ${String(model.schema_version)}.`,
        });
        return;
      }
      if (model.document_uri !== uri || model.document_version !== version) {
        void panel.webview.postMessage({
          type: "error",
          message: "The server returned a Tk preview for a different document snapshot.",
        });
        return;
      }
      renderedDocument = { uri, version };
      void panel.webview.postMessage({ type: "layout", data: result });
    } else {
      void panel.webview.postMessage({ type: "empty" });
    }
  } catch (err) {
    if (!panel || request !== requestSerial) return;
    const message = err instanceof Error ? err.message : String(err);
    void panel.webview.postMessage({ type: "error", message });
  }
}

async function revealSource(start: number | undefined, end: number | undefined): Promise<void> {
  const rendered = renderedDocument;
  if (
    !rendered ||
    typeof start !== "number" ||
    typeof end !== "number" ||
    start < 0 ||
    end < start ||
    !Number.isFinite(start) ||
    !Number.isFinite(end) ||
    !Number.isInteger(start) ||
    !Number.isInteger(end)
  ) {
    return;
  }
  const document = await vscode.workspace.openTextDocument(vscode.Uri.parse(rendered.uri));
  if (
    renderedDocument?.uri !== rendered.uri ||
    renderedDocument.version !== rendered.version ||
    document.uri.toString() !== rendered.uri ||
    document.version !== rendered.version
  ) {
    refreshPreview();
    return;
  }
  const bytes = Buffer.from(document.getText(), "utf8");
  if (end > bytes.length) return;
  const utf16Offset = (byteOffset: number): number =>
    bytes.subarray(0, Math.min(byteOffset, bytes.length)).toString("utf8").length;
  const range = new vscode.Range(
    document.positionAt(utf16Offset(start)),
    document.positionAt(utf16Offset(end)),
  );
  const editor = await vscode.window.showTextDocument(document, {
    viewColumn: vscode.ViewColumn.One,
    preserveFocus: false,
  });
  editor.selection = new vscode.Selection(range.start, range.end);
  editor.revealRange(range, vscode.TextEditorRevealType.InCenterIfOutsideViewport);
}
