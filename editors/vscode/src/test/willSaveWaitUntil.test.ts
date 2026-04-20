import * as assert from "assert";
import * as vscode from "vscode";
import { LanguageClient } from "vscode-languageclient/node";
import { activate, getDocUri } from "./helper";

interface TclLspApi {
  getClient(): LanguageClient;
}

suite("Will Save Wait Until", () => {
  test("server advertises textDocumentSync.willSaveWaitUntil capability", async () => {
    await activate(getDocUri("simple.tcl"));
    const ext = vscode.extensions.getExtension<TclLspApi>("bitwisecook.tcl-lsp")!;
    assert.ok(ext.isActive, "Extension should be active");
    const client = ext.exports.getClient();
    const caps = client.initializeResult?.capabilities as Record<string, unknown> | undefined;
    assert.ok(caps, "Server should report capabilities");
    const sync = caps.textDocumentSync as Record<string, unknown> | number | undefined;
    assert.ok(
      typeof sync === "object" && sync !== null && sync.willSaveWaitUntil === true,
      "Should advertise textDocumentSync.willSaveWaitUntil",
    );
  });
});
