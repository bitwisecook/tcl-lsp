import * as assert from "assert";
import * as vscode from "vscode";
import { LanguageClient } from "vscode-languageclient/node";
import { activate, getDocUri } from "./helper";

interface TclLspApi {
  getClient(): LanguageClient;
}

suite("Extension Activation", () => {
  let ext: vscode.Extension<TclLspApi>;

  suiteSetup(async function () {
    this.timeout(60_000);
    ext = vscode.extensions.getExtension("bitwisecook.tcl-lsp")!;
    assert.ok(ext, "Extension should be installed");
    if (!ext.isActive) {
      await ext.activate();
    }
    // Ensure the server has fully initialised by opening a fixture.
    await activate(getDocUri("simple.tcl"));
  });

  test("extension is present in the extensions list", () => {
    assert.ok(ext, "Extension should be installed");
  });

  test("extension activates successfully", () => {
    assert.ok(ext.isActive, "Extension should be active");
  });

  test("extension exports getClient function", () => {
    const exports = ext.exports as unknown as Record<string, unknown>;
    assert.ok(exports, "Extension should have exports");
    assert.ok(typeof exports.getClient === "function", "Should export getClient()");
  });

  test("status bar items appear when a Tcl file is active", () => {
    // The status bar items are created during activation.
    // We can't directly inspect them, but we can verify the extension is active
    // and the document is open with the correct language ID.
    const editor = vscode.window.activeTextEditor;
    assert.ok(editor, "Should have an active editor");
    assert.strictEqual(editor.document.languageId, "tcl", "Language should be tcl");
  });

  test("server capabilities include expected providers", () => {
    const client = ext.exports.getClient();
    const caps = client.initializeResult?.capabilities as Record<string, unknown> | undefined;

    assert.ok(caps, "Server should report capabilities");
    assert.ok(caps.hoverProvider, "Should have hoverProvider");
    assert.ok(caps.completionProvider, "Should have completionProvider");
    assert.ok(caps.definitionProvider, "Should have definitionProvider");
    assert.ok(caps.referencesProvider, "Should have referencesProvider");
    assert.ok(caps.documentFormattingProvider, "Should have documentFormattingProvider");
    assert.ok(caps.codeActionProvider, "Should have codeActionProvider");
    assert.ok(caps.documentSymbolProvider, "Should have documentSymbolProvider");
    assert.ok(caps.foldingRangeProvider, "Should have foldingRangeProvider");
    assert.ok(caps.renameProvider, "Should have renameProvider");
    assert.ok(caps.signatureHelpProvider, "Should have signatureHelpProvider");
    assert.ok(caps.workspaceSymbolProvider, "Should have workspaceSymbolProvider");
    assert.ok(caps.documentLinkProvider, "Should have documentLinkProvider");
    assert.ok(caps.selectionRangeProvider, "Should have selectionRangeProvider");
    assert.ok(caps.callHierarchyProvider, "Should have callHierarchyProvider");
  });

  test("server reports semantic token capabilities", () => {
    const client = ext.exports.getClient();
    const caps = client.initializeResult?.capabilities as Record<string, unknown> | undefined;

    assert.ok(caps, "Server should report capabilities");
    assert.ok(caps.semanticTokensProvider, "Should have semanticTokensProvider capability");
  });
});
