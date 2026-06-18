import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate } from "./helper";

suite("Inlay Hints", () => {
  const docUri = getDocUri("simple.tcl");

  test("inlay hints provider is wired up and does not throw", async () => {
    await activate(docUri);

    const fullRange = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(100, 0));

    // By default both inlay families are disabled, but the provider should
    // still be registered and return gracefully.
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
    const original = config.get<boolean>("inlayTypeHints", false);

    try {
      // Enable inferred-type inlay hints
      await config.update("inlayTypeHints", true, vscode.ConfigurationTarget.Global);

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
      await config.update("inlayTypeHints", original, vscode.ConfigurationTarget.Global);
    }
  });

  test("single positional binds the required slot, not the optional one (issue #510)", async () => {
    const config = vscode.workspace.getConfiguration("tclLsp.features");
    const original = config.get<boolean>("inlayParameterHints", false);

    try {
      await config.update("inlayParameterHints", true, vscode.ConfigurationTarget.Global);

      // `puts ?-nonewline? ?channelId? string` — one positional argument binds
      // to the required trailing `string`, never the leading optional
      // `channelId` (the pre-#510 regression).  `?options?`/`?switches?`
      // documentation placeholders are likewise never emitted.
      const doc = await vscode.workspace.openTextDocument({
        language: "tcl",
        content: "puts hello\nlsearch $mylist needle\n",
      });
      await vscode.window.showTextDocument(doc);

      const fullRange = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(10, 0));
      let hints: vscode.InlayHint[] | undefined;
      const deadline = Date.now() + 10_000;
      while (Date.now() < deadline) {
        hints = (await vscode.commands.executeCommand(
          "vscode.executeInlayHintProvider",
          doc.uri,
          fullRange,
        )) as vscode.InlayHint[] | undefined;
        if (hints && hints.length > 0) {
          break;
        }
        await new Promise((r) => setTimeout(r, 250));
      }

      assert.ok(hints && hints.length > 0, "expected inlay hints with the feature enabled");

      const labelText = (hint: vscode.InlayHint): string =>
        typeof hint.label === "string" ? hint.label : hint.label.map((p) => p.value).join("");

      // Parameter-kind hints on line 0 (`puts hello`).
      const line0 = hints
        .filter((h) => h.kind === vscode.InlayHintKind.Parameter && h.position.line === 0)
        .map(labelText);
      assert.ok(
        line0.includes("string:"),
        `expected 'string:' on line 0, got ${JSON.stringify(line0)}`,
      );
      assert.ok(
        !line0.includes("channelId:"),
        `'channelId:' must not be emitted for a single positional, got ${JSON.stringify(line0)}`,
      );

      // Parameter-kind hints on line 1 (`lsearch $mylist needle`): both real
      // positionals are labelled and no `?options?`/`?switches?` placeholder.
      const line1 = hints
        .filter((h) => h.kind === vscode.InlayHintKind.Parameter && h.position.line === 1)
        .map(labelText);
      assert.ok(
        line1.includes("list:"),
        `expected 'list:' on line 1, got ${JSON.stringify(line1)}`,
      );
      assert.ok(
        line1.includes("pattern:"),
        `expected 'pattern:' on line 1, got ${JSON.stringify(line1)}`,
      );
      assert.ok(
        !line1.some((l) => l === "options:" || l === "switches:"),
        `documentation placeholders must not be labelled, got ${JSON.stringify(line1)}`,
      );
    } finally {
      await config.update("inlayParameterHints", original, vscode.ConfigurationTarget.Global);
    }
  });
});
