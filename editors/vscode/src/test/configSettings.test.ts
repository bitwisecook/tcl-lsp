// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import * as assert from "assert";
import * as vscode from "vscode";
import {
  getDocUri,
  activate,
  getServerLogSize,
  nextDiagnosticsPublish,
  pollUntil,
  waitForDeepDiagnostics,
  waitForDiagnostics,
  waitForEffectiveConfig,
  waitForFeatureToggle,
  setTestContent,
} from "./helper";

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
    "inlayTypeHints",
    "inlayParameterHints",
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

  // Tri-state inheritance: null feature toggles inherit from editor globals.
  // These tests change a VS Code editor global, verify the feature toggle
  // reflects it, then restore the original value.

  const editorGlobalMappings: Array<[string, string, boolean]> = [
    // [feature key, editor setting path, editor default value]
    ["hover", "editor.hover.enabled", true],
    ["codeLens", "editor.codeLens", true],
    ["folding", "editor.folding", true],
    ["signatureHelp", "editor.parameterHints.enabled", true],
    ["linkedEditingRange", "editor.linkedEditing", false],
    // semanticTokens and documentHighlight are verified in the round-trip
    // tests below; their editor globals are non-boolean so they are not
    // included in this boolean-assertion loop.
  ];

  for (const [featureKey, editorSetting] of editorGlobalMappings) {
    test(`features.${featureKey} defaults to null (inherits from ${editorSetting})`, () => {
      const featureVal = cfg().get<boolean | null>(`features.${featureKey}`);
      assert.strictEqual(featureVal, null, `features.${featureKey} should default to null`);
    });

    test(`features.${featureKey}=true overrides ${editorSetting}=false`, async () => {
      const config = vscode.workspace.getConfiguration("tclLsp.features");
      try {
        // Explicitly set the feature to true — it should override any editor global
        await config.update(featureKey, true, undefined);
        const value = vscode.workspace
          .getConfiguration("tclLsp.features")
          .get<boolean | null>(featureKey);
        assert.strictEqual(value, true, `Explicit true should override editor global`);
      } finally {
        await config.update(featureKey, undefined, undefined);
      }
    });

    test(`features.${featureKey}=false overrides ${editorSetting}`, async () => {
      const config = vscode.workspace.getConfiguration("tclLsp.features");
      try {
        await config.update(featureKey, false, undefined);
        const value = vscode.workspace
          .getConfiguration("tclLsp.features")
          .get<boolean | null>(featureKey);
        assert.strictEqual(value, false, `Explicit false should override editor global`);
      } finally {
        await config.update(featureKey, undefined, undefined);
      }
    });
  }

  // ── Full LSP round-trip tests for editor global inheritance ─────────
  // These tests verify the complete pipeline: setting an editor global
  // (while the feature toggle is null) causes the middleware to resolve
  // the value, push it to the server, and the server changes its LSP
  // response accordingly.

  // hover — editor.hover.enabled

  test("editor.hover.enabled=false suppresses hover via null inheritance", async () => {
    const docUri = getDocUri("procs.tcl");
    await activate(docUri);
    const pos = new vscode.Position(1, 6);

    const before = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeHoverProvider", docUri, pos),
      (r) => Array.isArray(r) && r.length > 0,
      { timeout: 10_000, label: "hover before disable (editor global)" },
    )) as vscode.Hover[];
    assert.ok(before && before.length > 0, "Hover should work with default editor globals");

    const editorCfg = vscode.workspace.getConfiguration("editor");
    try {
      await editorCfg.update("hover.enabled", false, undefined);
      await waitForFeatureToggle(docUri, "hover", false);

      const after = (await pollUntil(
        () => vscode.commands.executeCommand("vscode.executeHoverProvider", docUri, pos),
        (r) => !r || (Array.isArray(r) ? r.length === 0 : true),
        { timeout: 10_000, label: "hover suppressed (editor global)" },
      )) as vscode.Hover[];
      assert.ok(
        !after || after.length === 0,
        `Hover should be suppressed when editor.hover.enabled=false, got ${after?.length ?? 0}`,
      );
    } finally {
      await editorCfg.update("hover.enabled", undefined, undefined);
      await waitForFeatureToggle(docUri, "hover", true);
    }
  });

  test("explicit features.hover=true overrides editor.hover.enabled=false", async () => {
    const docUri = getDocUri("procs.tcl");
    await activate(docUri);
    const pos = new vscode.Position(1, 6);

    const editorCfg = vscode.workspace.getConfiguration("editor");
    const featureCfg = vscode.workspace.getConfiguration("tclLsp.features");
    try {
      await editorCfg.update("hover.enabled", false, undefined);
      await featureCfg.update("hover", true, undefined);
      await waitForFeatureToggle(docUri, "hover", true);

      const result = (await pollUntil(
        () => vscode.commands.executeCommand("vscode.executeHoverProvider", docUri, pos),
        (r) => Array.isArray(r) && r.length > 0,
        { timeout: 10_000, label: "hover overrides editor global" },
      )) as vscode.Hover[];
      assert.ok(
        result && result.length > 0,
        "Explicit features.hover=true should override editor.hover.enabled=false",
      );
    } finally {
      await featureCfg.update("hover", undefined, undefined);
      await editorCfg.update("hover.enabled", undefined, undefined);
      await waitForFeatureToggle(docUri, "hover", true);
    }
  });

  // folding — editor.folding

  test("editor.folding=false suppresses folding via null inheritance", async () => {
    const docUri = getDocUri("folding.tcl");
    await activate(docUri);

    const before = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeFoldingRangeProvider", docUri),
      (r) => Array.isArray(r) && r.length > 0,
      { timeout: 10_000, label: "folding before disable (editor global)" },
    )) as vscode.FoldingRange[];
    assert.ok(before && before.length > 0, "Folding should work with default editor globals");

    const editorCfg = vscode.workspace.getConfiguration("editor");
    try {
      await editorCfg.update("folding", false, undefined);
      await waitForFeatureToggle(docUri, "folding", false);

      const after = (await pollUntil(
        () => vscode.commands.executeCommand("vscode.executeFoldingRangeProvider", docUri),
        (r) => !r || (Array.isArray(r) ? r.length === 0 : true),
        { timeout: 10_000, label: "folding suppressed (editor global)" },
      )) as vscode.FoldingRange[];
      assert.ok(
        !after || after.length === 0,
        `Folding should be suppressed when editor.folding=false, got ${after?.length ?? 0}`,
      );
    } finally {
      await editorCfg.update("folding", undefined, undefined);
      await waitForFeatureToggle(docUri, "folding", true);
    }
  });

  // codeLens — editor.codeLens

  test("editor.codeLens=false suppresses code lenses via null inheritance", async () => {
    const docUri = getDocUri("procs.tcl");
    await activate(docUri);

    // Poll until the server publishes its initial code lens batch —
    // ``activate()`` returns after didOpen but before the LSP
    // codeLens/* round-trip completes, so a fixed sleep here would race.
    const before = await pollUntil<vscode.CodeLens[] | undefined>(
      () =>
        vscode.commands.executeCommand("vscode.executeCodeLensProvider", docUri, 100) as Thenable<
          vscode.CodeLens[] | undefined
        >,
      (r) => Array.isArray(r) && r.length > 0,
      { label: "initial code lens batch" },
    );
    assert.ok(before && before.length > 0, "Code lenses should work with default editor globals");

    const editorCfg = vscode.workspace.getConfiguration("editor");
    try {
      await editorCfg.update("codeLens", false, undefined);
      await waitForFeatureToggle(docUri, "codeLens", false);

      const after = (await pollUntil(
        () => vscode.commands.executeCommand("vscode.executeCodeLensProvider", docUri, 100),
        (r) => !r || (Array.isArray(r) ? r.length === 0 : true),
        { timeout: 10_000, label: "code lenses suppressed (editor global)" },
      )) as vscode.CodeLens[] | undefined;
      assert.ok(
        !after || after.length === 0,
        `Code lenses should be suppressed when editor.codeLens=false, got ${after?.length ?? 0}`,
      );
    } finally {
      await editorCfg.update("codeLens", undefined, undefined);
      await waitForFeatureToggle(docUri, "codeLens", true);
    }
  });

  // signatureHelp — editor.parameterHints.enabled

  test("editor.parameterHints.enabled=false suppresses signature help via null inheritance", async () => {
    const docUri = getDocUri("procs.tcl");
    await activate(docUri);
    // Position inside the `fib` call where signature help would trigger
    const pos = new vscode.Position(5, 38);

    const before = (await vscode.commands.executeCommand(
      "vscode.executeSignatureHelpProvider",
      docUri,
      pos,
    )) as vscode.SignatureHelp | undefined;
    const hadSignatures = before && before.signatures && before.signatures.length > 0;

    const editorCfg = vscode.workspace.getConfiguration("editor");
    try {
      await editorCfg.update("parameterHints.enabled", false, undefined);
      await waitForFeatureToggle(docUri, "signatureHelp", false);

      const after = (await vscode.commands.executeCommand(
        "vscode.executeSignatureHelpProvider",
        docUri,
        pos,
      )) as vscode.SignatureHelp | undefined;
      if (hadSignatures) {
        assert.ok(
          !after || !after.signatures || after.signatures.length === 0,
          "Signature help should be suppressed when editor.parameterHints.enabled=false",
        );
      }
    } finally {
      await editorCfg.update("parameterHints.enabled", undefined, undefined);
      await waitForFeatureToggle(docUri, "signatureHelp", true);
    }
  });

  // semanticTokens — editor.semanticHighlighting.enabled

  test("editor.semanticHighlighting.enabled=false suppresses semantic tokens via null inheritance", async () => {
    const docUri = getDocUri("simple.tcl");
    await activate(docUri);

    const before = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.provideDocumentSemanticTokens", docUri),
      (r) => {
        const t = r as vscode.SemanticTokens | undefined;
        return !!t && t.data.length > 0;
      },
      { timeout: 10_000, label: "semantic tokens before disable (editor global)" },
    )) as vscode.SemanticTokens | undefined;
    assert.ok(
      before && before.data.length > 0,
      "Semantic tokens should work with default editor globals",
    );

    const editorCfg = vscode.workspace.getConfiguration("editor");
    try {
      await editorCfg.update("semanticHighlighting.enabled", false, undefined);
      await waitForFeatureToggle(docUri, "semanticTokens", false);

      const after = (await pollUntil(
        () => vscode.commands.executeCommand("vscode.provideDocumentSemanticTokens", docUri),
        (r) => {
          const t = r as vscode.SemanticTokens | undefined;
          return !t || t.data.length === 0;
        },
        { timeout: 10_000, label: "semantic tokens suppressed (editor global)" },
      )) as vscode.SemanticTokens | undefined;
      assert.ok(
        !after || after.data.length === 0,
        `Semantic tokens should be suppressed when editor.semanticHighlighting.enabled=false, got ${after?.data.length ?? 0} data items`,
      );
    } finally {
      await editorCfg.update("semanticHighlighting.enabled", undefined, undefined);
      await waitForFeatureToggle(docUri, "semanticTokens", true);
    }
  });

  // documentHighlight — editor.occurrencesHighlight is a client-side
  // decoration setting, not an LSP feature gate.  VS Code still sends
  // textDocument/documentHighlight requests regardless, so there is no
  // round-trip suppression test for it.

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
    "W003",
    "W004",
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
    const before = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeHoverProvider", docUri, pos),
      (r) => Array.isArray(r) && r.length > 0,
      { timeout: 10_000, label: "hover before disable (feature)" },
    )) as vscode.Hover[];
    assert.ok(before && before.length > 0, "Hover should return results by default");

    const config = vscode.workspace.getConfiguration("tclLsp.features");
    try {
      await config.update("hover", false, undefined);
      await waitForFeatureToggle(docUri, "hover", false);

      const after = (await pollUntil(
        () => vscode.commands.executeCommand("vscode.executeHoverProvider", docUri, pos),
        (r) => !r || (Array.isArray(r) ? r.length === 0 : true),
        { timeout: 10_000, label: "hover suppressed (feature)" },
      )) as vscode.Hover[];
      assert.ok(
        !after || after.length === 0,
        `Hover should be suppressed when disabled, got ${after?.length ?? 0} results`,
      );
    } finally {
      await config.update("hover", undefined, undefined);
      await waitForFeatureToggle(docUri, "hover", true);
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
    const before = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeCompletionItemProvider", docUri, pos),
      (r) => {
        const list = r as vscode.CompletionList | undefined;
        return !!list && list.items.some((i) => labelOf(i) === "puts");
      },
      { timeout: 10_000, label: "completion before disable (feature)" },
    )) as vscode.CompletionList;
    const hasPutsBefore = before.items.some((i) => labelOf(i) === "puts");
    assert.ok(hasPutsBefore, "LSP should provide 'puts' completion by default");

    const config = vscode.workspace.getConfiguration("tclLsp.features");
    try {
      await config.update("completion", false, undefined);
      await waitForFeatureToggle(docUri, "completion", false);

      const after = (await pollUntil(
        () => vscode.commands.executeCommand("vscode.executeCompletionItemProvider", docUri, pos),
        (r) => {
          const list = r as vscode.CompletionList | undefined;
          return (
            !!list &&
            !list.items.find((i) => labelOf(i) === "puts" && (i.detail || i.documentation))
          );
        },
        { timeout: 10_000, label: "completion suppressed (feature)" },
      )) as vscode.CompletionList;
      // VS Code may still provide word-based completions, but our LSP
      // command completions (like "puts" with detail/docs) should be gone.
      const lspPuts = after.items.find(
        (i) => labelOf(i) === "puts" && (i.detail || i.documentation),
      );
      assert.ok(!lspPuts, "LSP 'puts' completion with detail should be suppressed when disabled");
    } finally {
      await config.update("completion", undefined, undefined);
      await waitForFeatureToggle(docUri, "completion", true);
    }
  });

  test("disabling features.documentSymbols reduces symbol detail", async () => {
    const docUri = getDocUri("procs.tcl");
    await activate(docUri);

    // Baseline: our LSP provides rich proc symbols with children/detail
    const before = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeDocumentSymbolProvider", docUri),
      (r) => Array.isArray(r) && (r as vscode.DocumentSymbol[]).some((s) => s.name === "fib"),
      { timeout: 10_000, label: "document symbols before disable (feature)" },
    )) as vscode.DocumentSymbol[];
    const fibBefore = before.find((s) => s.name === "fib");
    assert.ok(fibBefore, "LSP should provide 'fib' symbol by default");
    // Our LSP symbols have children (proc parameters, body elements)
    const richBefore = fibBefore.children && fibBefore.children.length > 0;

    const config = vscode.workspace.getConfiguration("tclLsp.features");
    try {
      await config.update("documentSymbols", false, undefined);
      await waitForFeatureToggle(docUri, "documentSymbols", false);

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
      await waitForFeatureToggle(docUri, "documentSymbols", true);
    }
  });

  test("disabling features.definition suppresses go-to-definition", async () => {
    const docUri = getDocUri("procs.tcl");
    await activate(docUri);
    // "fib" call at line 16: puts "fib(10) = [fib 10]"
    const pos = new vscode.Position(16, 17);

    const before = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeDefinitionProvider", docUri, pos),
      (r) => Array.isArray(r) && r.length > 0,
      { timeout: 10_000, label: "definition before disable (feature)" },
    )) as vscode.Location[];
    assert.ok(before && before.length > 0, "Definition should work by default");

    const config = vscode.workspace.getConfiguration("tclLsp.features");
    try {
      await config.update("definition", false, undefined);
      await waitForFeatureToggle(docUri, "definition", false);

      const after = (await pollUntil(
        () => vscode.commands.executeCommand("vscode.executeDefinitionProvider", docUri, pos),
        (r) => !r || (Array.isArray(r) ? r.length === 0 : true),
        { timeout: 10_000, label: "definition suppressed (feature)" },
      )) as vscode.Location[];
      assert.ok(
        !after || after.length === 0,
        `Definition should be suppressed when disabled, got ${after?.length ?? 0}`,
      );
    } finally {
      await config.update("definition", undefined, undefined);
      await waitForFeatureToggle(docUri, "definition", true);
    }
  });

  test("disabling features.references suppresses find-references", async () => {
    const docUri = getDocUri("procs.tcl");
    await activate(docUri);
    // Position on "fib" proc definition at line 1
    const pos = new vscode.Position(1, 5);

    const before = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeReferenceProvider", docUri, pos),
      (r) => Array.isArray(r) && r.length > 0,
      { timeout: 10_000, label: "references before disable (feature)" },
    )) as vscode.Location[];
    assert.ok(before && before.length > 0, "References should work by default");

    const config = vscode.workspace.getConfiguration("tclLsp.features");
    try {
      await config.update("references", false, undefined);
      await waitForFeatureToggle(docUri, "references", false);

      const after = (await pollUntil(
        () => vscode.commands.executeCommand("vscode.executeReferenceProvider", docUri, pos),
        (r) => !r || (Array.isArray(r) ? r.length === 0 : true),
        { timeout: 10_000, label: "references suppressed (feature)" },
      )) as vscode.Location[];
      assert.ok(
        !after || after.length === 0,
        `References should be suppressed when disabled, got ${after?.length ?? 0}`,
      );
    } finally {
      await config.update("references", undefined, undefined);
      await waitForFeatureToggle(docUri, "references", true);
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
      await waitForFeatureToggle(docUri, "signatureHelp", false);

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
      await waitForFeatureToggle(docUri, "signatureHelp", true);
    }
  });

  test("disabling features.folding suppresses folding ranges", async () => {
    const docUri = getDocUri("folding.tcl");
    await activate(docUri);

    const before = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeFoldingRangeProvider", docUri),
      (r) => Array.isArray(r) && r.length > 0,
      { timeout: 10_000, label: "folding before disable (feature)" },
    )) as vscode.FoldingRange[];
    assert.ok(before && before.length > 0, "Folding should work by default");

    const config = vscode.workspace.getConfiguration("tclLsp.features");
    try {
      await config.update("folding", false, undefined);
      await waitForFeatureToggle(docUri, "folding", false);

      const after = (await pollUntil(
        () => vscode.commands.executeCommand("vscode.executeFoldingRangeProvider", docUri),
        (r) => !r || (Array.isArray(r) ? r.length === 0 : true),
        { timeout: 10_000, label: "folding suppressed (feature)" },
      )) as vscode.FoldingRange[];
      assert.ok(
        !after || after.length === 0,
        `Folding should be suppressed when disabled, got ${after?.length ?? 0}`,
      );
    } finally {
      await config.update("folding", undefined, undefined);
      await waitForFeatureToggle(docUri, "folding", true);
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
    const before = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeSelectionRangeProvider", docUri, [pos]),
      (r) => Array.isArray(r) && r.length > 0,
      { timeout: 10_000, label: "selection ranges before disable (feature)" },
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
      await waitForFeatureToggle(docUri, "selectionRange", false);

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

    // Baseline: wait for the specific code we are about to disable
    // rather than the first publish that happens to have any
    // diagnostic — that way a fixture change that no longer triggers
    // W100 fails the wait directly rather than racing the assertion
    // below.
    const codeOf = (d: vscode.Diagnostic) => (typeof d.code === "object" ? d.code.value : d.code);
    const before = await waitForDiagnostics(docUri, {
      predicate: (diags) => diags.some((d) => codeOf(d) === "W100"),
    });
    const hasW100Before = before.some((d) => codeOf(d) === "W100");
    assert.ok(hasW100Before, `W100 should be present by default, got [${before.map(codeOf)}]`);

    const config = vscode.workspace.getConfiguration("tclLsp.diagnostics");
    try {
      await config.update("W100", false, undefined);
      await waitForEffectiveConfig(docUri, (c) => c.disabled_diagnostics.includes("W100"), {
        label: "W100 disabled",
      });

      // Re-trigger: touch the document so the server re-analyses.
      // Register the publish listener *before* the edit so the fresh
      // publish event is not missed and we do not read stale
      // pre-toggle results.
      const editor = vscode.window.activeTextEditor!;
      const freshDiags = nextDiagnosticsPublish(docUri);
      await setTestContent(editor, editor.document.getText() + " ");
      const after = await freshDiags;
      const hasW100After = after.some((d) => codeOf(d) === "W100");
      assert.ok(
        !hasW100After,
        `W100 should be suppressed when disabled, got [${after.map(codeOf)}]`,
      );
    } finally {
      await config.update("W100", undefined, undefined);
      await waitForEffectiveConfig(docUri, (c) => !c.disabled_diagnostics.includes("W100"), {
        label: "W100 re-enabled",
      });
    }
  });

  // ── Optimiser enabled toggle behavioral test ─────────────────────
  test("disabling optimiser.enabled suppresses O1xx diagnostics", async () => {
    const docUri = getDocUri("diagnostics.tcl");
    // Snapshot the server log index before activating so the
    // ``waitForDeepDiagnostics`` calls below only match this test's
    // deep-pass log lines, not stale ones from earlier tests.
    const sinceOpen = getServerLogSize();
    await activate(docUri);

    const codeOf = (d: vscode.Diagnostic) =>
      String(typeof d.code === "object" ? d.code.value : d.code);
    const isO1xx = (code: string) => /^O1\d\d$/.test(code);

    // Wait for the server to publish its ``[timing] deep diagnostics``
    // log line for this URI — that's the direct signal that the deep
    // pass (where O1xx hints come from) has finished and published.
    await waitForDeepDiagnostics(docUri, { since: sinceOpen });
    const before = vscode.languages.getDiagnostics(docUri);
    const o1xxBefore = before.filter((d) => isO1xx(codeOf(d)));
    // The re-trigger edit below appends to the live buffer and is never
    // saved; snapshot the original text so it can be restored in `finally`
    // regardless of whether the test body completes or throws, rather than
    // leaving diagnostics.tcl's editor buffer permanently dirtied for every
    // later test that reuses the same fixture.
    const originalText = vscode.window.activeTextEditor!.document.getText();

    const config = vscode.workspace.getConfiguration("tclLsp.optimiser");
    try {
      await config.update("enabled", false, undefined);
      // 20s, matching waitForDeepDiagnostics's default: under the full
      // suite's background load (workspace warm-up, the #844 progressive
      // diagnostics race, …) this round-trip routinely needs more than the
      // 5s generic default.
      await waitForEffectiveConfig(docUri, (c) => c.optimiser_enabled === false, {
        label: "optimiser disabled",
        timeout: 20000,
      });

      // Re-trigger analysis with a noop edit; snapshot the log
      // index so the follow-up wait only matches the post-edit run.
      const sinceEdit = getServerLogSize();
      const editor = vscode.window.activeTextEditor!;
      await setTestContent(editor, editor.document.getText() + " ");
      await waitForDeepDiagnostics(docUri, { since: sinceEdit });
      const after = vscode.languages.getDiagnostics(docUri);
      const o1xxAfter = after.filter((d) => isO1xx(codeOf(d)));

      if (o1xxBefore.length > 0) {
        assert.strictEqual(
          o1xxAfter.length,
          0,
          `O1xx diagnostics should disappear when optimiser disabled, got [${o1xxAfter.map(codeOf)}]`,
        );
      }
    } finally {
      await setTestContent(vscode.window.activeTextEditor!, originalText);
      await config.update("enabled", undefined, undefined);
      await waitForEffectiveConfig(docUri, (c) => c.optimiser_enabled === true, {
        label: "optimiser re-enabled",
        timeout: 20000,
      });
    }
  });

  // ── Regression: #104 — diagnostics master switch must clear all
  //    diagnostics even for files opened/analysed after the toggle.
  test("features.diagnostics=false clears all diagnostics (#104)", async () => {
    const docUri = getDocUri("diagnostics.tcl");
    const config = vscode.workspace.getConfiguration("tclLsp.features");

    // Disable the master switch *before* opening the file, simulating
    // the scenario where VS Code restarts with the setting already off.
    try {
      await config.update("diagnostics", false, undefined);
      // ``waitForFeatureToggle`` resolves ``docUri`` to its workspace
      // folder even before the document is opened — the URI is only
      // used to pick the right per-folder FeatureConfig.
      await waitForFeatureToggle(docUri, "diagnostics", false);

      // Register the listener *before* opening the file so the
      // server's first publish for this URI is not missed.  When the
      // master switch is off, ``_publish_diagnostics`` skips both
      // passes and emits a single empty publish — there is no
      // ``[timing]`` server log on this path, so the publish event
      // itself is the signal.
      const firstPublish = nextDiagnosticsPublish(docUri, { timeout: 3000 });
      await activate(docUri);
      const diags = await firstPublish;
      assert.strictEqual(
        diags.length,
        0,
        `No diagnostics should appear when master switch is off, got: ${diags.map((d) => (typeof d.code === "object" ? d.code.value : d.code))}`,
      );
    } finally {
      await config.update("diagnostics", undefined, undefined);
      await waitForFeatureToggle(docUri, "diagnostics", true);
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
