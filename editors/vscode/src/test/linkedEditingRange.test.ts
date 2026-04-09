import * as assert from "assert";
import * as vscode from "vscode";
import { LanguageClient } from "vscode-languageclient/node";
import { activate, getDocUri } from "./helper";

interface TclLspApi {
  getClient(): LanguageClient;
}

suite("Linked Editing Range", () => {
  const docUri = getDocUri("procs.tcl");

  test("server advertises linkedEditingRangeProvider", async () => {
    await activate(docUri);
    const ext = vscode.extensions.getExtension<TclLspApi>("bitwisecook.tcl-lsp")!;
    const client = ext.exports.getClient();
    const caps = client.initializeResult?.capabilities as Record<string, unknown> | undefined;
    assert.ok(caps, "Server should report capabilities");
    assert.ok(caps.linkedEditingRangeProvider, "linkedEditingRangeProvider should be present");
  });

  test("links a recursive proc name with its self-calls", async () => {
    await activate(docUri);
    const ext = vscode.extensions.getExtension<TclLspApi>("bitwisecook.tcl-lsp")!;
    const client = ext.exports.getClient();

    // `fib` declaration at 0-indexed line 1 col 6 — fib recurses twice in
    // its own body so we expect at least 2 ranges back (decl + self-call).
    const result = (await client.sendRequest<{
      ranges?: Array<{ start: { line: number; character: number } }>;
    } | null>("textDocument/linkedEditingRange", {
      textDocument: { uri: docUri.toString() },
      position: { line: 1, character: 6 },
    })) as { ranges?: Array<{ start: { line: number; character: number } }> } | null;

    assert.ok(result, "Expected linked editing ranges for recursive fib");
    assert.ok(
      result.ranges && result.ranges.length >= 2,
      `Expected at least 2 linked ranges (decl + recursive call), got ${result.ranges?.length}`,
    );
  });
});
