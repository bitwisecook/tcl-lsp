import * as assert from "assert";
import * as vscode from "vscode";
import { LanguageClient } from "vscode-languageclient/node";
import { activate, getDocUri } from "./helper";

interface TclLspApi {
  getClient(): LanguageClient;
}

suite("Pull Diagnostics (opt-in)", () => {
  test("pull diagnostic provider is NOT advertised by default", async () => {
    await activate(getDocUri("simple.tcl"));
    const ext = vscode.extensions.getExtension<TclLspApi>("bitwisecook.tcl-lsp")!;
    assert.ok(ext.isActive, "Extension should be active");
    const client = ext.exports.getClient();
    const caps = client.initializeResult?.capabilities as Record<string, unknown> | undefined;
    assert.ok(caps, "Server should report capabilities");

    // Pull diagnostics are opt-in: vscode-languageclient switches to pull mode
    // whenever diagnosticProvider is present, which disables the push pipeline.
    // Default behaviour therefore MUST keep diagnosticProvider absent.
    assert.strictEqual(
      caps.diagnosticProvider,
      undefined,
      "diagnosticProvider should be absent by default (pull diagnostics opt-in)",
    );
  });

  test("config toggle exists in package.json metadata", () => {
    const config = vscode.workspace.getConfiguration("tclLsp.features");
    const value = config.get<boolean>("pullDiagnostics");
    assert.strictEqual(value, false, "pullDiagnostics should default to false");
  });
});
