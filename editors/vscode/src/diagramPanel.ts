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
import { getDiagramPanelHtml } from "./diagramPanelHtml";

let panel: vscode.WebviewPanel | undefined;
let pendingSource: string | undefined;

/**
 * Open (or reveal) a webview tab showing a rendered Mermaid diagram.
 *
 * If the panel already exists it is revealed and updated with the new source.
 */
export function openDiagramPanel(mermaidSource: string): void {
  if (panel) {
    panel.reveal(vscode.ViewColumn.Beside);
    void panel.webview.postMessage({ type: "setDiagram", source: mermaidSource });
    return;
  }

  pendingSource = mermaidSource;

  panel = vscode.window.createWebviewPanel(
    "tclIruleDiagram",
    "iRule Diagram",
    vscode.ViewColumn.Beside,
    { enableScripts: true, retainContextWhenHidden: true },
  );

  panel.webview.html = getDiagramPanelHtml();

  panel.webview.onDidReceiveMessage((msg: { type: string }) => {
    if (msg.type === "ready" && pendingSource) {
      void panel!.webview.postMessage({ type: "setDiagram", source: pendingSource });
      pendingSource = undefined;
    }
  });

  panel.onDidDispose(() => {
    panel = undefined;
    pendingSource = undefined;
  });
}
