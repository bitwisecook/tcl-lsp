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
