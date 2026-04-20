import * as assert from "assert";
import * as vscode from "vscode";
import { LanguageClient } from "vscode-languageclient/node";
import { activate, getDocUri } from "./helper";

interface TclLspApi {
  getClient(): LanguageClient;
}

suite("Workspace File Operations", () => {
  test("server advertises willRename/didRename capabilities", async () => {
    await activate(getDocUri("simple.tcl"));
    const ext = vscode.extensions.getExtension<TclLspApi>("bitwisecook.tcl-lsp")!;
    assert.ok(ext.isActive, "Extension should be active");
    const client = ext.exports.getClient();
    const caps = client.initializeResult?.capabilities as Record<string, unknown> | undefined;
    assert.ok(caps, "Server should report capabilities");
    const workspace = caps.workspace as Record<string, unknown> | undefined;
    const fileOps = workspace?.fileOperations as Record<string, unknown> | undefined;
    assert.ok(fileOps, "Should have workspace.fileOperations");
    assert.ok(fileOps.willRename, "Should advertise willRename file-operations");
    assert.ok(fileOps.didRename, "Should advertise didRename file-operations");

    // Validate the glob filter covers Tcl file extensions.
    const willRename = fileOps.willRename as { filters?: Array<{ pattern?: { glob?: string } }> };
    assert.ok(
      willRename.filters && willRename.filters.length >= 1,
      "Should have at least one filter",
    );
    const globs = willRename.filters.map((f) => f.pattern?.glob ?? "").join("|");
    assert.ok(/tcl/.test(globs), `Expected filter glob to include 'tcl', got ${globs}`);
  });
});
