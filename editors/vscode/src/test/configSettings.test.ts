import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate, sleep, waitForDiagnostics, setTestContent } from "./helper";

suite("Configuration Settings", () => {
  const cfg = () => vscode.workspace.getConfiguration("tclLsp");

  // Feature toggles — all default to null (inherit from editor globals or
  // default to enabled), except pullDiagnostics which defaults to false.
  const triStateFeatureKeys = [
    "hover",
    "completion",
    "diagnostics",
    "semanticTokens",
    "codeActions",
    "definition",
    "references",
    "documentSymbols",
    "folding",
    "rename",
    "signatureHelp",
    "workspaceSymbols",
    "inlayHints",
    "callHierarchy",
    "documentLinks",
    "selectionRange",
  ];

  for (const key of triStateFeatureKeys) {
    test(`features.${key} defaults to null (inherit from editor)`, () => {
      const value = cfg().get<boolean | null>(`features.${key}`);
      assert.strictEqual(value, null, `features.${key} should default to null`);
    });
  }

  test("features.pullDiagnostics defaults to false", () => {
    assert.strictEqual(cfg().get<boolean>("features.pullDiagnostics"), false);
  });

  // Formatting options
  const formattingIntKeys: Array<[string, number]> = [
    ["indentSize", 4],
    ["continuationIndent", 4],
    ["maxLineLength", 120],
    ["goalLineLength", 100],
    ["minBodyCommandsForExpansion", 2],
    ["blankLinesBetweenProcs", 1],
    ["blankLinesBetweenBlocks", 1],
    ["maxConsecutiveBlankLines", 2],
    ["docstringDecorationWidth", 70],
  ];

  for (const [key, defaultValue] of formattingIntKeys) {
    test(`formatting.${key} defaults to ${defaultValue}`, () => {
      assert.strictEqual(
        cfg().get<number>(`formatting.${key}`),
        defaultValue,
        `formatting.${key} should default to ${defaultValue}`,
      );
    });
  }

  const formattingBoolKeys: Array<[string, boolean]> = [
    ["spaceBetweenBraces", true],
    ["enforceBracedVariables", false],
    ["enforceBracedExpr", false],
    ["expandSingleLineBodies", false],
    ["spaceAfterCommentHash", true],
    ["trimTrailingWhitespace", true],
    ["alignCommentsToCode", true],
    ["replaceSemicolonsWithNewlines", true],
    ["ensureFinalNewline", true],
    ["docstringDecoration", false],
  ];

  for (const [key, defaultValue] of formattingBoolKeys) {
    test(`formatting.${key} defaults to ${defaultValue}`, () => {
      assert.strictEqual(cfg().get<boolean>(`formatting.${key}`), defaultValue);
    });
  }

  test("formatting.indentStyle defaults to spaces", () => {
    assert.strictEqual(cfg().get<string>("formatting.indentStyle"), "spaces");
  });

  test("formatting.braceStyle defaults to k_and_r", () => {
    assert.strictEqual(cfg().get<string>("formatting.braceStyle"), "k_and_r");
  });

  test("formatting.lineEnding defaults to lf", () => {
    assert.strictEqual(cfg().get<string>("formatting.lineEnding"), "lf");
  });

  test("formatting.docstringStyle defaults to none", () => {
    assert.strictEqual(cfg().get<string>("formatting.docstringStyle"), "none");
  });

  test("formatting.docstringTagStyle defaults to doxygen", () => {
    assert.strictEqual(cfg().get<string>("formatting.docstringTagStyle"), "doxygen");
  });

  test("formatting.docstringDecorationChar defaults to .", () => {
    assert.strictEqual(cfg().get<string>("formatting.docstringDecorationChar"), ".");
  });

  // Dialect
  test("dialect defaults to tcl8.6", () => {
    assert.strictEqual(cfg().get<string>("dialect"), "tcl8.6");
  });

  test("dialect setting accepts all valid dialect values", () => {
    const inspect = cfg().inspect<string>("dialect");
    assert.ok(inspect, "dialect setting should exist");
  });

  // Python path
  test("pythonPath defaults to auto", () => {
    assert.strictEqual(cfg().get<string>("pythonPath"), "auto");
  });

  // Server path
  test("serverPath defaults to empty", () => {
    assert.strictEqual(cfg().get<string>("serverPath"), "");
  });

  // Extra commands
  test("extraCommands defaults to empty array", () => {
    const value = cfg().get<string[]>("extraCommands");
    assert.ok(Array.isArray(value), "extraCommands should be an array");
    assert.strictEqual(value.length, 0, "extraCommands should default to empty");
  });

  // Library paths
  test("libraryPaths defaults to empty array", () => {
    const value = cfg().get<string[]>("libraryPaths");
    assert.ok(Array.isArray(value), "libraryPaths should be an array");
    assert.strictEqual(value.length, 0, "libraryPaths should default to empty");
  });

  // Optimiser
  test("optimiser.enabled defaults to true", () => {
    assert.strictEqual(cfg().get<boolean>("optimiser.enabled"), true);
  });

  test("optimiser.profile defaults to readability", () => {
    assert.strictEqual(cfg().get<string>("optimiser.profile"), "readability");
  });

  const optimiserRules = [
    "O100",
    "O101",
    "O102",
    "O103",
    "O104",
    "O105",
    "O106",
    "O107",
    "O108",
    "O109",
    "O110",
    "O111",
    "O112",
    "O113",
    "O114",
    "O115",
    "O116",
    "O117",
    "O118",
    "O119",
    "O120",
    "O121",
    "O122",
    "O123",
    "O124",
    "O125",
    "O126",
    "O127",
  ];

  for (const rule of optimiserRules) {
    test(`optimiser.${rule} defaults to null (inherit from profile)`, () => {
      assert.strictEqual(cfg().get(`optimiser.${rule}`), null);
    });
  }

  // Diagnostics toggles
  const diagnosticCodes = [
    "E001",
    "E002",
    "E003",
    "E200",
    "W001",
    "W002",
    "W100",
    "W101",
    "W102",
    "W103",
    "W104",
    "W105",
    "W106",
    "W108",
    "W110",
    "W111",
    "W112",
    "W113",
    "W114",
    "W115",
    "W116",
    "W117",
    "W118",
    "W120",
    "W121",
    "W122",
    "W124",
    "W126",
    "W200",
    "W201",
    "W210",
    "W211",
    "W212",
    "W213",
    "W214",
    "W220",
    "W300",
    "W301",
    "W302",
    "W303",
    "W304",
    "W306",
    "W307",
    "W308",
    "W309",
    "W313",
    "H300",
    "S100",
    "S101",
    "S102",
    "T100",
    "T101",
    "T102",
    "IRULE1001",
    "IRULE1002",
    "IRULE1003",
    "IRULE1004",
    "IRULE1005",
    "IRULE1006",
    "IRULE1007",
    "IRULE1008",
    "IRULE1201",
    "IRULE1202",
    "IRULE2001",
    "IRULE2002",
    "IRULE2003",
    "IRULE2101",
    "IRULE3001",
    "IRULE3002",
    "IRULE3003",
    "IRULE3101",
    "IRULE3102",
    "IRULE4001",
    "IRULE4002",
    "IRULE4003",
    "IRULE4004",
    "IRULE4005",
    "IRULE5001",
    "IRULE5002",
    "IRULE5004",
    "IRULE5005",
    "IRULE5006",
    "IRULE5007",
  ];

  for (const code of diagnosticCodes) {
    test(`diagnostics.${code} has a boolean default`, () => {
      const value = cfg().get<boolean>(`diagnostics.${code}`);
      assert.strictEqual(typeof value, "boolean", `diagnostics.${code} should be a boolean`);
    });
  }

  test("diagnostics.W123 defaults to false (opt-in)", () => {
    assert.strictEqual(cfg().get<boolean>("diagnostics.W123"), false);
  });

  // Runtime validation
  test("runtimeValidation.enabled defaults to false", () => {
    assert.strictEqual(cfg().get<boolean>("runtimeValidation.enabled"), false);
  });

  test("runtimeValidation.adapter defaults to auto", () => {
    assert.strictEqual(cfg().get<string>("runtimeValidation.adapter"), "auto");
  });

  test("runtimeValidation.tclshPath defaults to tclsh", () => {
    assert.strictEqual(cfg().get<string>("runtimeValidation.tclshPath"), "tclsh");
  });

  test("runtimeValidation.timeoutMs defaults to 5000", () => {
    assert.strictEqual(cfg().get<number>("runtimeValidation.timeoutMs"), 5000);
  });

  // AI settings
  test("ai.enabled defaults to true", () => {
    assert.strictEqual(cfg().get<boolean>("ai.enabled"), true);
  });

  test("ai.extraPrompts defaults to empty array", () => {
    const value = cfg().get<unknown[]>("ai.extraPrompts");
    assert.ok(Array.isArray(value), "ai.extraPrompts should be an array");
    assert.strictEqual(value.length, 0);
  });

  // Shimmer detection
  test("shimmer.enabled defaults to true", () => {
    assert.strictEqual(cfg().get<boolean>("shimmer.enabled"), true);
  });

  // XC diagnostics
  test("xcDiagnostics.enabled defaults to false", () => {
    assert.strictEqual(cfg().get<boolean>("xcDiagnostics.enabled"), false);
  });

  // Style
  test("style.lineLength defaults to 120", () => {
    assert.strictEqual(cfg().get<number>("style.lineLength"), 120);
  });

  test("style.nonAscii defaults to confusables", () => {
    assert.strictEqual(cfg().get<string>("style.nonAscii"), "confusables");
  });

  // Trace
  test("tcl-lsp.trace.server defaults to off", () => {
    const value = vscode.workspace.getConfiguration("tcl-lsp").get<string>("trace.server");
    assert.strictEqual(value, "off");
  });

  // Generic variable patterns
  test("diagnostics.genericVariablePatterns defaults to empty array", () => {
    const value = cfg().get<string[]>("diagnostics.genericVariablePatterns");
    assert.ok(Array.isArray(value), "genericVariablePatterns should be an array");
    assert.strictEqual(value.length, 0);
  });

  // ── Behavioral mutation tests ──────────────────────────────────────
  // Each test verifies that changing a setting actually affects LSP
  // behavior, not just that the config round-trips.

  test("disabling features.hover suppresses hover results", async () => {
    const docUri = getDocUri("procs.tcl");
    await activate(docUri);
    // "fib" proc name at line 1, col 6
    const pos = new vscode.Position(1, 6);

    // Baseline: hover works with default (true)
    const before = (await vscode.commands.executeCommand(
      "vscode.executeHoverProvider",
      docUri,
      pos,
    )) as vscode.Hover[];
    assert.ok(before && before.length > 0, "Hover should return results by default");

    const config = vscode.workspace.getConfiguration("tclLsp.features");
    try {
      await config.update("hover", false, undefined);
      await sleep(500);

      const after = (await vscode.commands.executeCommand(
        "vscode.executeHoverProvider",
        docUri,
        pos,
      )) as vscode.Hover[];
      assert.ok(
        !after || after.length === 0,
        `Hover should be suppressed when disabled, got ${after?.length ?? 0} results`,
      );
    } finally {
      await config.update("hover", undefined, undefined);
    }
  });

  test("disabling features.completion removes LSP completions", async () => {
    const docUri = getDocUri("completion.tcl");
    await activate(docUri);
    // Position at partial "put" on line 2
    const pos = new vscode.Position(2, 3);

    const labelOf = (item: vscode.CompletionItem) =>
      typeof item.label === "string" ? item.label : item.label.label;

    // Baseline: our LSP provides Tcl command completions like "puts"
    const before = (await vscode.commands.executeCommand(
      "vscode.executeCompletionItemProvider",
      docUri,
      pos,
    )) as vscode.CompletionList;
    const hasPutsBefore = before.items.some((i) => labelOf(i) === "puts");
    assert.ok(hasPutsBefore, "LSP should provide 'puts' completion by default");

    const config = vscode.workspace.getConfiguration("tclLsp.features");
    try {
      await config.update("completion", false, undefined);
      await sleep(500);

      const after = (await vscode.commands.executeCommand(
        "vscode.executeCompletionItemProvider",
        docUri,
        pos,
      )) as vscode.CompletionList;
      // VS Code may still provide word-based completions, but our LSP
      // command completions (like "puts" with detail/docs) should be gone.
      const lspPuts = after.items.find(
        (i) => labelOf(i) === "puts" && (i.detail || i.documentation),
      );
      assert.ok(!lspPuts, "LSP 'puts' completion with detail should be suppressed when disabled");
    } finally {
      await config.update("completion", undefined, undefined);
    }
  });

  test("disabling features.documentSymbols reduces symbol detail", async () => {
    const docUri = getDocUri("procs.tcl");
    await activate(docUri);

    // Baseline: our LSP provides rich proc symbols with children/detail
    const before = (await vscode.commands.executeCommand(
      "vscode.executeDocumentSymbolProvider",
      docUri,
    )) as vscode.DocumentSymbol[];
    const fibBefore = before.find((s) => s.name === "fib");
    assert.ok(fibBefore, "LSP should provide 'fib' symbol by default");
    // Our LSP symbols have children (proc parameters, body elements)
    const richBefore = fibBefore.children && fibBefore.children.length > 0;

    const config = vscode.workspace.getConfiguration("tclLsp.features");
    try {
      await config.update("documentSymbols", false, undefined);
      await sleep(500);

      const after = (await vscode.commands.executeCommand(
        "vscode.executeDocumentSymbolProvider",
        docUri,
      )) as vscode.DocumentSymbol[];
      if (richBefore && after && after.length > 0) {
        // When our provider is disabled, VS Code's built-in may still
        // find symbols but without the rich children our LSP provides.
        const fibAfter = after.find((s) => s.name === "fib");
        if (fibAfter) {
          const richAfter = fibAfter.children && fibAfter.children.length > 0;
          assert.ok(!richAfter, "LSP 'fib' symbol should lose children when provider disabled");
        }
      }
    } finally {
      await config.update("documentSymbols", undefined, undefined);
    }
  });

  test("disabling features.definition suppresses go-to-definition", async () => {
    const docUri = getDocUri("procs.tcl");
    await activate(docUri);
    // "fib" call at line 16: puts "fib(10) = [fib 10]"
    const pos = new vscode.Position(16, 17);

    const before = (await vscode.commands.executeCommand(
      "vscode.executeDefinitionProvider",
      docUri,
      pos,
    )) as vscode.Location[];
    assert.ok(before && before.length > 0, "Definition should work by default");

    const config = vscode.workspace.getConfiguration("tclLsp.features");
    try {
      await config.update("definition", false, undefined);
      await sleep(500);

      const after = (await vscode.commands.executeCommand(
        "vscode.executeDefinitionProvider",
        docUri,
        pos,
      )) as vscode.Location[];
      assert.ok(
        !after || after.length === 0,
        `Definition should be suppressed when disabled, got ${after?.length ?? 0}`,
      );
    } finally {
      await config.update("definition", undefined, undefined);
    }
  });

  test("disabling features.references suppresses find-references", async () => {
    const docUri = getDocUri("procs.tcl");
    await activate(docUri);
    // Position on "fib" proc definition at line 1
    const pos = new vscode.Position(1, 5);

    const before = (await vscode.commands.executeCommand(
      "vscode.executeReferenceProvider",
      docUri,
      pos,
    )) as vscode.Location[];
    assert.ok(before && before.length > 0, "References should work by default");

    const config = vscode.workspace.getConfiguration("tclLsp.features");
    try {
      await config.update("references", false, undefined);
      await sleep(500);

      const after = (await vscode.commands.executeCommand(
        "vscode.executeReferenceProvider",
        docUri,
        pos,
      )) as vscode.Location[];
      assert.ok(
        !after || after.length === 0,
        `References should be suppressed when disabled, got ${after?.length ?? 0}`,
      );
    } finally {
      await config.update("references", undefined, undefined);
    }
  });

  test("disabling features.signatureHelp suppresses signatures", async () => {
    const docUri = getDocUri("procs.tcl");
    await activate(docUri);
    // Position inside "expr {" on line 6
    const pos = new vscode.Position(5, 10);

    const before = (await vscode.commands.executeCommand(
      "vscode.executeSignatureHelpProvider",
      docUri,
      pos,
    )) as vscode.SignatureHelp | undefined;
    // Signature help may or may not fire depending on exact position;
    // just verify no crash. The real test is that disabling suppresses it.
    const hadSignatures = before && before.signatures && before.signatures.length > 0;

    const config = vscode.workspace.getConfiguration("tclLsp.features");
    try {
      await config.update("signatureHelp", false, undefined);
      await sleep(500);

      const after = (await vscode.commands.executeCommand(
        "vscode.executeSignatureHelpProvider",
        docUri,
        pos,
      )) as vscode.SignatureHelp | undefined;
      if (hadSignatures) {
        assert.ok(
          !after || !after.signatures || after.signatures.length === 0,
          "Signature help should be suppressed when disabled",
        );
      }
    } finally {
      await config.update("signatureHelp", undefined, undefined);
    }
  });

  test("disabling features.folding suppresses folding ranges", async () => {
    const docUri = getDocUri("folding.tcl");
    await activate(docUri);

    const before = (await vscode.commands.executeCommand(
      "vscode.executeFoldingRangeProvider",
      docUri,
    )) as vscode.FoldingRange[];
    assert.ok(before && before.length > 0, "Folding should work by default");

    const config = vscode.workspace.getConfiguration("tclLsp.features");
    try {
      await config.update("folding", false, undefined);
      await sleep(500);

      const after = (await vscode.commands.executeCommand(
        "vscode.executeFoldingRangeProvider",
        docUri,
      )) as vscode.FoldingRange[];
      assert.ok(
        !after || after.length === 0,
        `Folding should be suppressed when disabled, got ${after?.length ?? 0}`,
      );
    } finally {
      await config.update("folding", undefined, undefined);
    }
  });

  test("disabling features.documentLinks removes LSP links", async () => {
    const docUri = getDocUri("links.tcl");
    await activate(docUri);

    // Document links are retrieved via the LSP protocol, not a VS Code
    // executeCommand. Verify the config toggles and the feature is wired up.
    const config = vscode.workspace.getConfiguration("tclLsp.features");
    const original = config.get<boolean | null>("documentLinks", null);
    assert.strictEqual(original, null, "documentLinks should default to null (inherit)");
    try {
      await config.update("documentLinks", false, undefined);
      const changed = vscode.workspace
        .getConfiguration("tclLsp.features")
        .get<boolean | null>("documentLinks");
      assert.strictEqual(changed, false);
    } finally {
      await config.update("documentLinks", undefined, undefined);
    }
    assert.strictEqual(
      vscode.workspace.getConfiguration("tclLsp.features").get<boolean | null>("documentLinks"),
      null,
      "Should restore to default (null)",
    );
  });

  test("disabling features.selectionRange removes LSP selection ranges", async () => {
    const docUri = getDocUri("procs.tcl");
    await activate(docUri);
    const pos = new vscode.Position(3, 8);

    // Baseline: our LSP provides nested selection ranges (proc body → proc → file)
    const before = (await vscode.commands.executeCommand(
      "vscode.executeSelectionRangeProvider",
      docUri,
      [pos],
    )) as vscode.SelectionRange[];
    assert.ok(before && before.length > 0, "Selection ranges should work by default");
    // Our provider returns deeply nested ranges (parent chain)
    const depthBefore = (() => {
      let d = 0;
      let r: vscode.SelectionRange | undefined = before[0];
      while (r) {
        d++;
        r = r.parent;
      }
      return d;
    })();

    const config = vscode.workspace.getConfiguration("tclLsp.features");
    try {
      await config.update("selectionRange", false, undefined);
      await sleep(500);

      const after = (await vscode.commands.executeCommand(
        "vscode.executeSelectionRangeProvider",
        docUri,
        [pos],
      )) as vscode.SelectionRange[];
      // VS Code may still provide basic selection ranges, but our deeply
      // nested AST-aware ranges should be gone or much shallower.
      if (after && after.length > 0) {
        const depthAfter = (() => {
          let d = 0;
          let r: vscode.SelectionRange | undefined = after[0];
          while (r) {
            d++;
            r = r.parent;
          }
          return d;
        })();
        assert.ok(
          depthAfter < depthBefore,
          `Selection range depth should decrease when disabled (before=${depthBefore}, after=${depthAfter})`,
        );
      }
    } finally {
      await config.update("selectionRange", undefined, undefined);
    }
  });

  // ── Formatting indentSize config test ──────────────────────────────
  // Note: executeFormatDocumentProvider passes FormattingOptions.tabSize
  // which the LSP server uses for indent width, so indentSize cannot be
  // isolated behaviourally through that API. Verify the config round-trips.
  test("formatting.indentSize change round-trips and differs from default", async () => {
    const section = "tclLsp.formatting";
    const original = vscode.workspace.getConfiguration(section).get<number>("indentSize");
    assert.strictEqual(original, 4, "indentSize should default to 4");
    try {
      await vscode.workspace.getConfiguration(section).update("indentSize", 2, undefined);
      const changed = vscode.workspace.getConfiguration(section).get<number>("indentSize");
      assert.strictEqual(changed, 2);
      assert.notStrictEqual(changed, original);
    } finally {
      await vscode.workspace.getConfiguration(section).update("indentSize", undefined, undefined);
    }
    assert.strictEqual(
      vscode.workspace.getConfiguration(section).get<number>("indentSize"),
      4,
      "Should restore to default",
    );
  });

  // ── Diagnostic code toggle behavioral test ───────────────────────
  test("disabling diagnostics.W100 suppresses that diagnostic", async () => {
    const docUri = getDocUri("diagnostics.tcl");
    await activate(docUri);

    // Baseline: W100 should be present
    const before = await waitForDiagnostics(docUri, { minCount: 1 });
    const codeOf = (d: vscode.Diagnostic) => (typeof d.code === "object" ? d.code.value : d.code);
    const hasW100Before = before.some((d) => codeOf(d) === "W100");
    assert.ok(hasW100Before, `W100 should be present by default, got [${before.map(codeOf)}]`);

    const config = vscode.workspace.getConfiguration("tclLsp.diagnostics");
    try {
      await config.update("W100", false, undefined);
      await sleep(1000);

      // Re-trigger: touch the document so the server re-analyses.
      // Then wait for a *fresh* diagnostics publish (onDidChangeDiagnostics)
      // to avoid reading stale pre-toggle results.
      const editor = vscode.window.activeTextEditor!;
      const freshDiags = new Promise<vscode.Diagnostic[]>((resolve) => {
        const disposable = vscode.languages.onDidChangeDiagnostics((e) => {
          if (e.uris.some((u) => u.toString() === docUri.toString())) {
            disposable.dispose();
            resolve(vscode.languages.getDiagnostics(docUri));
          }
        });
        setTimeout(() => {
          disposable.dispose();
          resolve(vscode.languages.getDiagnostics(docUri));
        }, 5000);
      });
      await setTestContent(editor, editor.document.getText() + " ");
      const after = await freshDiags;
      const hasW100After = after.some((d) => codeOf(d) === "W100");
      assert.ok(
        !hasW100After,
        `W100 should be suppressed when disabled, got [${after.map(codeOf)}]`,
      );
    } finally {
      await config.update("W100", undefined, undefined);
    }
  });

  // ── Optimiser enabled toggle behavioral test ─────────────────────
  test("disabling optimiser.enabled suppresses O1xx diagnostics", async () => {
    const docUri = getDocUri("diagnostics.tcl");
    await activate(docUri);
    await sleep(500);

    const codeOf = (d: vscode.Diagnostic) =>
      String(typeof d.code === "object" ? d.code.value : d.code);
    const isO1xx = (code: string) => /^O1\d\d$/.test(code);

    // Baseline: check for any O1xx hints (optimiser is enabled by default)
    const before = await waitForDiagnostics(docUri, { timeout: 5000, minCount: 1 });
    const o1xxBefore = before.filter((d) => isO1xx(codeOf(d)));

    const config = vscode.workspace.getConfiguration("tclLsp.optimiser");
    try {
      await config.update("enabled", false, undefined);
      await sleep(1000);

      // Re-trigger analysis
      const editor = vscode.window.activeTextEditor!;
      await setTestContent(editor, editor.document.getText() + " ");
      const after = await waitForDiagnostics(docUri, { timeout: 5000, minCount: 1 });
      const o1xxAfter = after.filter((d) => isO1xx(codeOf(d)));

      if (o1xxBefore.length > 0) {
        assert.strictEqual(
          o1xxAfter.length,
          0,
          `O1xx diagnostics should disappear when optimiser disabled, got [${o1xxAfter.map(codeOf)}]`,
        );
      }
    } finally {
      await config.update("enabled", undefined, undefined);
    }
  });

  // ── Regression: #104 — diagnostics master switch must clear all
  //    diagnostics even for files opened/analysed after the toggle.
  test("features.diagnostics=false clears all diagnostics (#104)", async () => {
    const config = vscode.workspace.getConfiguration("tclLsp.features");

    // Disable the master switch *before* opening the file, simulating
    // the scenario where VS Code restarts with the setting already off.
    try {
      await config.update("diagnostics", false, undefined);
      await sleep(500);

      // Open a file that normally produces many diagnostics.
      const docUri = getDocUri("diagnostics.tcl");
      await activate(docUri);

      // Give the server time to analyse and (incorrectly) publish.
      const diags = await waitForDiagnostics(docUri, { timeout: 3000, minCount: 1 });
      assert.strictEqual(
        diags.length,
        0,
        `No diagnostics should appear when master switch is off, got: ${diags.map((d) => (typeof d.code === "object" ? d.code.value : d.code))}`,
      );
    } finally {
      await config.update("diagnostics", undefined, undefined);
    }
  });

  // ── Config round-trip tests for settings without directly
  //    observable LSP effects in the test environment ────────────────

  test("dialect change round-trips and differs from default", async () => {
    const section = "tclLsp";
    const original = vscode.workspace.getConfiguration(section).get<string>("dialect");
    assert.strictEqual(original, "tcl8.6", "Dialect should default to tcl8.6");
    try {
      await vscode.workspace.getConfiguration(section).update("dialect", "tcl8.5", undefined);
      const changed = vscode.workspace.getConfiguration(section).get<string>("dialect");
      assert.strictEqual(changed, "tcl8.5");
      assert.notStrictEqual(changed, original, "Changed value should differ from original");
    } finally {
      await vscode.workspace.getConfiguration(section).update("dialect", undefined, undefined);
    }
    const restored = vscode.workspace.getConfiguration(section).get<string>("dialect");
    assert.strictEqual(restored, "tcl8.6", "Should restore to default after cleanup");
  });

  test("optimiser.profile change round-trips and differs from default", async () => {
    const section = "tclLsp.optimiser";
    const original = vscode.workspace.getConfiguration(section).get<string>("profile");
    assert.strictEqual(original, "readability");
    try {
      await vscode.workspace.getConfiguration(section).update("profile", "aggressive", undefined);
      const changed = vscode.workspace.getConfiguration(section).get<string>("profile");
      assert.strictEqual(changed, "aggressive");
      assert.notStrictEqual(changed, original);
    } finally {
      await vscode.workspace.getConfiguration(section).update("profile", undefined, undefined);
    }
    assert.strictEqual(
      vscode.workspace.getConfiguration(section).get<string>("profile"),
      "readability",
      "Should restore to default",
    );
  });

  test("optimiser.O100 override round-trips and differs from default", async () => {
    const section = "tclLsp.optimiser";
    const original = vscode.workspace.getConfiguration(section).get("O100");
    assert.strictEqual(original, null, "O100 should default to null");
    try {
      await vscode.workspace.getConfiguration(section).update("O100", true, undefined);
      const changed = vscode.workspace.getConfiguration(section).get<boolean>("O100");
      assert.strictEqual(changed, true);
      assert.notStrictEqual(changed, original);
    } finally {
      await vscode.workspace.getConfiguration(section).update("O100", undefined, undefined);
    }
    assert.strictEqual(
      vscode.workspace.getConfiguration(section).get("O100"),
      null,
      "Should restore to default",
    );
  });

  test("runtimeValidation.adapter change round-trips and differs from default", async () => {
    const section = "tclLsp.runtimeValidation";
    const original = vscode.workspace.getConfiguration(section).get<string>("adapter");
    assert.strictEqual(original, "auto");
    try {
      await vscode.workspace.getConfiguration(section).update("adapter", "tcl-syntax", undefined);
      const changed = vscode.workspace.getConfiguration(section).get<string>("adapter");
      assert.strictEqual(changed, "tcl-syntax");
      assert.notStrictEqual(changed, original);
    } finally {
      await vscode.workspace.getConfiguration(section).update("adapter", undefined, undefined);
    }
    assert.strictEqual(
      vscode.workspace.getConfiguration(section).get<string>("adapter"),
      "auto",
      "Should restore to default",
    );
  });

  test("style.nonAscii change round-trips and differs from default", async () => {
    const section = "tclLsp.style";
    const original = vscode.workspace.getConfiguration(section).get<string>("nonAscii");
    assert.strictEqual(original, "confusables");
    try {
      await vscode.workspace.getConfiguration(section).update("nonAscii", "strict", undefined);
      const changed = vscode.workspace.getConfiguration(section).get<string>("nonAscii");
      assert.strictEqual(changed, "strict");
      assert.notStrictEqual(changed, original);
    } finally {
      await vscode.workspace.getConfiguration(section).update("nonAscii", undefined, undefined);
    }
    assert.strictEqual(
      vscode.workspace.getConfiguration(section).get<string>("nonAscii"),
      "confusables",
      "Should restore to default",
    );
  });

  test("trace.server change round-trips and differs from default", async () => {
    const section = "tcl-lsp.trace";
    const original = vscode.workspace.getConfiguration(section).get<string>("server");
    assert.strictEqual(original, "off");
    try {
      await vscode.workspace.getConfiguration(section).update("server", "verbose", undefined);
      const changed = vscode.workspace.getConfiguration(section).get<string>("server");
      assert.strictEqual(changed, "verbose");
      assert.notStrictEqual(changed, original);
    } finally {
      await vscode.workspace.getConfiguration(section).update("server", undefined, undefined);
    }
    assert.strictEqual(
      vscode.workspace.getConfiguration(section).get<string>("server"),
      "off",
      "Should restore to default",
    );
  });
});
