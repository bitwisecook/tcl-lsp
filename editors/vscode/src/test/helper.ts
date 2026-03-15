import * as vscode from "vscode";
import * as path from "path";

/**
 * Resolve a fixture file name to a URI.
 * e.g. getDocUri("simple.tcl") → file:///…/testFixture/simple.tcl
 */
export function getDocUri(fileName: string): vscode.Uri {
  return vscode.Uri.file(path.resolve(__dirname, "../../testFixture", fileName));
}

/** Promisified setTimeout. */
export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Open a document and wait for the language server to finish its initial
 * analysis. Returns the opened TextDocument.
 */
export async function activate(docUri: vscode.Uri): Promise<vscode.TextDocument> {
  // Ensure the extension is activated first.
  // ext.activate() awaits client.start() which resolves after the LSP
  // initialise/initialised handshake -- the server is ready at that point.
  const ext = vscode.extensions.getExtension("bitwisecook.tcl-lsp");
  if (ext && !ext.isActive) {
    await ext.activate();
  }

  const doc = await vscode.workspace.openTextDocument(docUri);
  await vscode.window.showTextDocument(doc);

  // Send a lightweight LSP request that will be serialized behind the
  // server's processing of the didOpen notification.  When this resolves
  // we know the server has finished analysing the document.
  await vscode.commands.executeCommand(
    "vscode.executeHoverProvider",
    docUri,
    new vscode.Position(0, 0),
  );

  return doc;
}

/**
 * Poll for diagnostics on the given URI until `minCount` are available
 * or the timeout expires.  Combines event listening with periodic polling
 * for robustness.
 */
export async function waitForDiagnostics(
  uri: vscode.Uri,
  opts?: { timeout?: number; minCount?: number },
): Promise<vscode.Diagnostic[]> {
  const timeout = opts?.timeout ?? 20_000;
  const minCount = opts?.minCount ?? 1;

  // Check immediately
  const immediate = vscode.languages.getDiagnostics(uri);
  if (immediate.length >= minCount) {
    return immediate;
  }

  return new Promise<vscode.Diagnostic[]>((resolve) => {
    let resolved = false;

    const finish = (diags: vscode.Diagnostic[]) => {
      if (resolved) return;
      resolved = true;
      clearTimeout(timer);
      clearInterval(poller);
      disposable.dispose();
      resolve(diags);
    };

    // Timeout -- return whatever we have
    const timer = setTimeout(() => {
      finish(vscode.languages.getDiagnostics(uri));
    }, timeout);

    // Event-driven: listen for diagnostic changes
    const disposable = vscode.languages.onDidChangeDiagnostics((e) => {
      const changed = e.uris.some((u) => u.toString() === uri.toString());
      if (changed) {
        const diags = vscode.languages.getDiagnostics(uri);
        if (diags.length >= minCount) {
          finish(diags);
        }
      }
    });

    // Polling fallback: check every 500ms in case we missed an event
    const poller = setInterval(() => {
      const diags = vscode.languages.getDiagnostics(uri);
      if (diags.length >= minCount) {
        finish(diags);
      }
    }, 500);
  });
}

/**
 * Replace the entire document content in the given editor.
 */
export async function setTestContent(editor: vscode.TextEditor, content: string): Promise<boolean> {
  const doc = editor.document;
  const fullRange = new vscode.Range(doc.positionAt(0), doc.positionAt(doc.getText().length));
  return editor.edit((editBuilder) => {
    editBuilder.replace(fullRange, content);
  });
}
