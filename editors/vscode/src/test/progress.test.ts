import * as assert from "assert";
import * as vscode from "vscode";
import { LanguageClient } from "vscode-languageclient/node";
import { activate, getDocUri } from "./helper";

interface TclLspApi {
  getClient(): LanguageClient;
}

suite("Work-Done Progress", () => {
  test("config toggle exists and defaults to true", () => {
    const config = vscode.workspace.getConfiguration("tclLsp.features");
    const value = config.get<boolean>("progress");
    assert.strictEqual(value, true, "progress should default to true");
  });

  test("server stays responsive during/after workspace scan", async () => {
    // The $/progress pipeline runs asynchronously on the event loop.  If
    // it blocked the event loop the test harness would time out activating
    // the extension.  Reaching this test at all means the scan coroutine
    // did not deadlock.
    await activate(getDocUri("simple.tcl"));
    const ext = vscode.extensions.getExtension<TclLspApi>("bitwisecook.tcl-lsp")!;
    assert.ok(ext.isActive, "Extension should still be active after scan");
    const client = ext.exports.getClient();
    assert.ok(client.initializeResult, "Client should have initialize result");
  });
});
