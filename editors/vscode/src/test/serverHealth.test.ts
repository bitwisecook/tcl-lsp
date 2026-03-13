import * as assert from "assert";
import * as vscode from "vscode";
import { LanguageClient, State } from "vscode-languageclient/node";
import { activate, getDocUri } from "./helper";

interface TclLspApi {
  getClient(): LanguageClient;
}

function getApi(): TclLspApi {
  const ext = vscode.extensions.getExtension("tcl-lsp.tcl-lsp")!;
  return ext.exports as TclLspApi;
}

// Root-level hooks bracket the entire test run.

// Runs before ALL test suites.  If the server crashed on startup (e.g. a
// missing module in the .pyz bundle) then ext.activate() rejects because
// client.start() fails, and the whole test run aborts with a clear message.
suiteSetup(async function () {
  this.timeout(60_000);
  const ext = vscode.extensions.getExtension("tcl-lsp.tcl-lsp")!;
  await ext.activate();
  assert.ok(ext.isActive, "Extension failed to activate – server may have crashed on startup");
});

// Runs after ALL test suites.  Catches server crashes that happen mid-run.
suiteTeardown(async function () {
  this.timeout(30_000);
  const client = getApi().getClient();
  assert.strictEqual(
    client.state,
    State.Running,
    `Server should still be Running at end of tests, got state ${client.state}`,
  );
});

// Explicit health-check suite with named tests.

suite("Server Health", () => {
  test("language client is in Running state", () => {
    const client = getApi().getClient();
    assert.strictEqual(
      client.state,
      State.Running,
      `Expected Running (${State.Running}), got ${client.state}`,
    );
  });

  test("server returned capabilities", () => {
    const client = getApi().getClient();
    const result = client.initializeResult as { capabilities: Record<string, unknown> } | undefined;
    assert.ok(result, "Server did not return an InitializeResult");
    assert.ok(result.capabilities, "InitializeResult has no capabilities");
    assert.ok(result.capabilities.hoverProvider, "Server should advertise hoverProvider");
    assert.ok(result.capabilities.completionProvider, "Server should advertise completionProvider");
  });

  test("server responds to hover request on a fixture file", async () => {
    const docUri = getDocUri("simple.tcl");
    await activate(docUri);
    // activate() sends a hover request serialised behind didOpen.
    // Reaching here means the server processed both successfully.
  });
});
