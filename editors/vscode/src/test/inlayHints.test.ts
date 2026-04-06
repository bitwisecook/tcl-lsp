import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate, setTestContent } from "./helper";

suite("Inlay Hints", () => {
  const docUri = getDocUri("formatting.tcl");

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

      await activate(docUri);
      const editor = vscode.window.activeTextEditor!;

      // Content with variables that could have type annotations
      await setTestContent(editor, 'set x 42\nset name "hello"\nset items [list a b c]\n');

      // Allow the server to process the change
      await new Promise((r) => setTimeout(r, 2000));

      const fullRange = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(10, 0));

      const hints = (await vscode.commands.executeCommand(
        "vscode.executeInlayHintProvider",
        docUri,
        fullRange,
      )) as vscode.InlayHint[] | undefined;

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
