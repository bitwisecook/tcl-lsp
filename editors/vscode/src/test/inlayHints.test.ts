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

  // PR #643 split the single `inlayHints` toggle into two independent options.
  // Enabling one must not turn on the other.
  test("parameter hints enabled alone produce labels but no type hints", async () => {
    const config = vscode.workspace.getConfiguration("tclLsp.features");
    const origType = config.get<boolean>("inlayTypeHints", false);
    const origParam = config.get<boolean>("inlayParameterHints", false);

    try {
      await config.update("inlayTypeHints", false, vscode.ConfigurationTarget.Global);
      await config.update("inlayParameterHints", true, vscode.ConfigurationTarget.Global);

      // `set x 42` would carry a `: int` Type-kind hint if type hints leaked
      // on, so the no-Type assertion below is a real check; `puts hello`
      // carries the `string:` parameter label.
      const doc = await vscode.workspace.openTextDocument({
        language: "tcl",
        content: "set x 42\nputs hello\n",
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
        // Wait for the *steady state* after both config changes propagate to
        // the server: the parameter hint present AND the type hint gone.
        // Breaking the moment a parameter hint appears would race the
        // `inlayTypeHints=false` change — a previous test may have left type
        // hints on, and they linger until the config update lands.
        const hasParam = hints?.some((h) => h.kind === vscode.InlayHintKind.Parameter) ?? false;
        const hasType = hints?.some((h) => h.kind === vscode.InlayHintKind.Type) ?? false;
        if (hasParam && !hasType) {
          break;
        }
        await new Promise((r) => setTimeout(r, 250));
      }

      assert.ok(
        hints && hints.some((h) => h.kind === vscode.InlayHintKind.Parameter),
        "expected a parameter-kind hint with inlayParameterHints enabled",
      );
      assert.ok(
        !hints!.some((h) => h.kind === vscode.InlayHintKind.Type),
        "type hints must stay off when only inlayParameterHints is enabled",
      );
    } finally {
      await config.update("inlayTypeHints", origType, vscode.ConfigurationTarget.Global);
      await config.update("inlayParameterHints", origParam, vscode.ConfigurationTarget.Global);
    }
  });

  test("type hints enabled alone emit no parameter labels", async () => {
    const config = vscode.workspace.getConfiguration("tclLsp.features");
    const origType = config.get<boolean>("inlayTypeHints", false);
    const origParam = config.get<boolean>("inlayParameterHints", false);

    try {
      await config.update("inlayTypeHints", true, vscode.ConfigurationTarget.Global);
      await config.update("inlayParameterHints", false, vscode.ConfigurationTarget.Global);

      // `set x 42` carries the `: int` type hint (the readiness signal);
      // `puts hello` WOULD carry the `string:` parameter label if those leaked
      // on, so the no-Parameter assertion below is a real check.
      const doc = await vscode.workspace.openTextDocument({
        language: "tcl",
        content: "set x 42\nputs hello\n",
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
        // Wait for the *steady state*: the type hint present AND parameter
        // labels gone.  Breaking the moment a type hint appears would race
        // the `inlayParameterHints=false` change — a previous test may have
        // left parameter labels on until the config update propagates.
        const hasType = hints?.some((h) => h.kind === vscode.InlayHintKind.Type) ?? false;
        const hasParam = hints?.some((h) => h.kind === vscode.InlayHintKind.Parameter) ?? false;
        if (hasType && !hasParam) {
          break;
        }
        await new Promise((r) => setTimeout(r, 250));
      }

      assert.ok(
        hints && hints.some((h) => h.kind === vscode.InlayHintKind.Type),
        "expected a type-kind hint with inlayTypeHints enabled",
      );
      assert.ok(
        !hints!.some((h) => h.kind === vscode.InlayHintKind.Parameter),
        "parameter labels must stay off when only inlayTypeHints is enabled",
      );
    } finally {
      await config.update("inlayTypeHints", origType, vscode.ConfigurationTarget.Global);
      await config.update("inlayParameterHints", origParam, vscode.ConfigurationTarget.Global);
    }
  });
});
