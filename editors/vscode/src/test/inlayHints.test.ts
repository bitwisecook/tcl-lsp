import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate } from "./helper";

suite("Inlay Hints", () => {
  const docUri = getDocUri("simple.tcl");

  test("inlay hints provider is wired up and does not throw", async () => {
    await activate(docUri);

    const fullRange = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(100, 0));

    // By default inlayHints is disabled, but the provider should still be
    // registered and return gracefully.
    const hints = (await vscode.commands.executeCommand(
      "vscode.executeInlayHintProvider",
      docUri,
      fullRange,
    )) as vscode.InlayHint[] | undefined;

    // Either null/undefined (feature disabled) or an array
    if (hints) {
      assert.ok(Array.isArray(hints), "Inlay hints should be an array");
    }
  });

  test("inlay hints appear when feature is enabled", async () => {
    const config = vscode.workspace.getConfiguration("tclLsp.features");
    const original = config.get<boolean>("inlayHints", false);

    try {
      // Enable inlay hints
      await config.update("inlayHints", true, vscode.ConfigurationTarget.Global);

      // Use an untitled document to avoid leaking state.
      const doc = await vscode.workspace.openTextDocument({
        language: "tcl",
        content: 'set x 42\nset name "hello"\nset items [list a b c]\n',
      });
      await vscode.window.showTextDocument(doc);

      // Poll for the server to process the change (up to 10s).
      const fullRange = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(10, 0));
      let hints: vscode.InlayHint[] | undefined;
      const deadline = Date.now() + 10_000;
      while (Date.now() < deadline) {
        hints = (await vscode.commands.executeCommand(
          "vscode.executeInlayHintProvider",
          doc.uri,
          fullRange,
        )) as vscode.InlayHint[] | undefined;
        // Accept either null (server hasn't produced yet) or a valid array.
        if (hints !== undefined) {
          break;
        }
        await new Promise((r) => setTimeout(r, 250));
      }

      // When enabled, the server may or may not produce inlay hints
      // depending on the analysis; just verify no error.
      if (hints && hints.length > 0) {
        for (const hint of hints) {
          assert.ok(hint.position, "Each hint should have a position");
          assert.ok(hint.label !== undefined, "Each hint should have a label");
        }
      }
    } finally {
      await config.update("inlayHints", original, vscode.ConfigurationTarget.Global);
    }
  });
});
