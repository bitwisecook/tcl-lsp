// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

import * as fs from "fs";
import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";

type Surface = "dsl" | "sample" | "rust" | "stub";

interface StudioMessage {
  type: string;
  surface?: Surface;
  text?: string;
  dialect?: string;
}

const filenames: Record<Surface, string> = {
  dsl: "active.tclspec",
  sample: "test.tcl",
  rust: "generated.rs",
  stub: "generated-stub.tcl",
};

let current: SpecStudioSession | undefined;

const studioIgnore = "*\n";

/**
 * Remove sessions which could only have survived an editor crash, and make
 * every subsequently materialised projection invisible to Git. This runs
 * before the language client starts, so an abandoned highest-precedence pack
 * can never enter the server's initial discovery snapshot.
 */
export async function prepareSpecStudioStorage(): Promise<void> {
  for (const folder of vscode.workspace.workspaceFolders ?? []) {
    const root = vscode.Uri.joinPath(folder.uri, ".tcl-lsp", ".spec-studio");
    const abandoned = vscode.Uri.joinPath(root, "vscode");
    try {
      await vscode.workspace.fs.delete(abandoned, { recursive: true, useTrash: false });
    } catch (error) {
      if (!(error instanceof vscode.FileSystemError && error.code === "FileNotFound")) throw error;
    }
    await vscode.workspace.fs.createDirectory(root);
    await vscode.workspace.fs.writeFile(
      vscode.Uri.joinPath(root, ".gitignore"),
      Buffer.from(studioIgnore, "utf8"),
    );
  }
}

function injectNativeBridge(html: string, nativeModule: string): string {
  const source = JSON.stringify(nativeModule).replace(/</g, "\\u003c");
  const bridge = `<script>const vscode=acquireVsCodeApi();window.__tclSpecStudioHost={postMessage:(message)=>vscode.postMessage(message)};window.__tclSpecStudioNativeModuleUrl=URL.createObjectURL(new Blob([${source}],{type:'text/javascript'}));</script>`;
  return html
    .replace("script-src 'self'", "script-src 'self' blob:")
    .replace("<head>", `<head>${bridge}`);
}

class SpecStudioSession implements vscode.Disposable {
  private readonly panel: vscode.WebviewPanel;
  private readonly root: vscode.Uri;
  private readonly sessionDir: vscode.Uri;
  private readonly documents = new Map<Surface, vscode.Uri>();
  private readonly subscriptions: vscode.Disposable[] = [];
  private readonly saveTimers = new Map<string, ReturnType<typeof setTimeout>>();
  private disposed = false;

  constructor(
    private readonly context: vscode.ExtensionContext,
    private readonly client: LanguageClient,
    workspaceRoot: vscode.Uri,
  ) {
    this.root = vscode.Uri.joinPath(context.extensionUri, "spec-studio");
    this.sessionDir = vscode.Uri.joinPath(workspaceRoot, ".tcl-lsp", ".spec-studio", "vscode");
    this.panel = vscode.window.createWebviewPanel(
      "tclSpecStudio",
      "Tcl Spec Studio",
      { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
      {
        enableScripts: true,
        retainContextWhenHidden: true,
        localResourceRoots: [this.root],
      },
    );

    for (const surface of Object.keys(filenames) as Surface[]) {
      this.documents.set(surface, vscode.Uri.joinPath(this.sessionDir, filenames[surface]));
    }
    this.subscriptions.push(
      this.panel.webview.onDidReceiveMessage((message: StudioMessage) => this.receive(message)),
      vscode.workspace.onDidChangeTextDocument((event) => this.documentChanged(event.document)),
      this.panel.onDidDispose(() => this.dispose()),
    );
  }

  async start(): Promise<void> {
    const index = vscode.Uri.joinPath(this.root, "index.html");
    let html: string;
    try {
      html = (await vscode.workspace.fs.readFile(index)).toString();
      const nativeModule = (
        await vscode.workspace.fs.readFile(
          vscode.Uri.joinPath(this.root, "assets", "native-editor-host.js"),
        )
      ).toString();
      html = injectNativeBridge(html, nativeModule);
    } catch {
      void vscode.window.showErrorMessage(
        "Tcl Spec Studio assets are missing from this extension build. Rebuild with `make spec-studio-wasm`.",
      );
      this.dispose();
      return;
    }
    await vscode.workspace.fs.createDirectory(this.sessionDir);
    this.panel.webview.html = html;
  }

  reveal(): void {
    this.panel.reveal(vscode.ViewColumn.Beside, true);
  }

  private async receive(message: StudioMessage): Promise<void> {
    if (message.type === "openSurface" && message.surface) {
      await this.openSurface(message.surface);
      return;
    }
    if (message.type === "surfaceUpdate" && message.surface && typeof message.text === "string") {
      await this.writeSurface(message.surface, message.text);
    }
  }

  private async writeSurface(surface: Surface, text: string): Promise<void> {
    const uri = this.documents.get(surface)!;
    const existed = fs.existsSync(uri.fsPath);
    await vscode.workspace.fs.createDirectory(this.sessionDir);
    const open = vscode.workspace.textDocuments.find(
      (document) => document.uri.toString() === uri.toString(),
    );
    if (open && open.getText() !== text) {
      const edit = new vscode.WorkspaceEdit();
      edit.replace(
        uri,
        new vscode.Range(open.positionAt(0), open.positionAt(open.getText().length)),
        text,
      );
      await vscode.workspace.applyEdit(edit);
      await open.save();
    } else if (!open) {
      await vscode.workspace.fs.writeFile(uri, Buffer.from(text, "utf8"));
    }
    if (surface === "dsl") await this.notifyPack(uri, existed ? 2 : 1);
  }

  private async openSurface(surface: Surface): Promise<void> {
    const uri = this.documents.get(surface)!;
    if (!fs.existsSync(uri.fsPath)) await this.writeSurface(surface, "");
    const document = await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(document, {
      viewColumn: vscode.ViewColumn.One,
      preserveFocus: false,
    });
  }

  private documentChanged(document: vscode.TextDocument): void {
    const entry = [...this.documents.entries()].find(
      ([, uri]) => uri.toString() === document.uri.toString(),
    );
    if (!entry || this.disposed) return;
    const [surface] = entry;
    if (surface === "rust" || surface === "stub") return;
    void this.panel.webview.postMessage({
      type: "tclSpecStudioHostUpdate",
      surface,
      text: document.getText(),
    });
    const key = document.uri.toString();
    const previous = this.saveTimers.get(key);
    if (previous) clearTimeout(previous);
    this.saveTimers.set(
      key,
      setTimeout(() => {
        this.saveTimers.delete(key);
        void document.save().then(async () => {
          if (surface === "dsl") await this.notifyPack(document.uri, 2);
        });
      }, 120),
    );
  }

  private async notifyPack(uri: vscode.Uri, type: 1 | 2 | 3): Promise<void> {
    await this.client.sendNotification("workspace/didChangeWatchedFiles", {
      changes: [{ uri: uri.toString(), type }],
    });
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    for (const timer of this.saveTimers.values()) clearTimeout(timer);
    for (const subscription of this.subscriptions.splice(0)) subscription.dispose();
    const pack = this.documents.get("dsl")!;
    const packExisted = fs.existsSync(pack.fsPath);
    void vscode.workspace.fs.delete(this.sessionDir, { recursive: true, useTrash: false }).then(
      async () => {
        if (packExisted) await this.notifyPack(pack, 3);
      },
      (error) => console.error("[spec-studio] could not remove temporary override", error),
    );
    if (current === this) current = undefined;
  }
}

export async function openSpecStudio(
  context: vscode.ExtensionContext,
  client: LanguageClient,
): Promise<void> {
  if (current) {
    current.reveal();
    return;
  }
  const folder = vscode.workspace.workspaceFolders?.[0];
  if (!folder) {
    void vscode.window.showErrorMessage(
      "Tcl Spec Studio needs an open workspace for its temporary pack override.",
    );
    return;
  }
  current = new SpecStudioSession(context, client, folder.uri);
  context.subscriptions.push(current);
  await current.start();
}

export const specStudioTest = { injectNativeBridge };
