import * as assert from "assert";
import * as vscode from "vscode";

suite("Configuration Settings", () => {
  const cfg = () => vscode.workspace.getConfiguration("tclLsp");

  // Feature toggles
  const featureKeys = [
    "hover",
    "completion",
    "diagnostics",
    "formatting",
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

  for (const key of featureKeys) {
    test(`features.${key} has a boolean default`, () => {
      const value = cfg().get<boolean>(`features.${key}`);
      assert.strictEqual(typeof value, "boolean", `features.${key} should be a boolean`);
    });
  }

  test("features.inlayHints defaults to false", () => {
    assert.strictEqual(cfg().get<boolean>("features.inlayHints"), false);
  });

  test("features.hover defaults to true", () => {
    assert.strictEqual(cfg().get<boolean>("features.hover"), true);
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

  // Feature toggle mutation
  // Note: after config.update(), the WorkspaceConfiguration snapshot is stale;
  // always re-fetch via workspace.getConfiguration() to read the new value.
  // Use undefined target (not Global) so writes go to the workspace scope,
  // matching the scope used by the extension's toggle commands.

  test("can programmatically toggle a feature setting", async () => {
    const section = "tclLsp.features";
    const original = vscode.workspace.getConfiguration(section).get<boolean>("hover", true);
    try {
      await vscode.workspace.getConfiguration(section).update("hover", !original, undefined);
      const toggled = vscode.workspace.getConfiguration(section).get<boolean>("hover");
      assert.strictEqual(toggled, !original, "Setting should be toggled");
    } finally {
      await vscode.workspace.getConfiguration(section).update("hover", undefined, undefined);
    }
  });

  test("can programmatically change dialect", async () => {
    const section = "tclLsp";
    const original = vscode.workspace.getConfiguration(section).get<string>("dialect", "tcl8.6");
    try {
      await vscode.workspace.getConfiguration(section).update("dialect", "tcl8.5", undefined);
      assert.strictEqual(
        vscode.workspace.getConfiguration(section).get<string>("dialect"),
        "tcl8.5",
      );
    } finally {
      await vscode.workspace.getConfiguration(section).update("dialect", undefined, undefined);
    }
  });

  test("can programmatically change formatting indent size", async () => {
    const section = "tclLsp.formatting";
    try {
      await vscode.workspace.getConfiguration(section).update("indentSize", 2, undefined);
      assert.strictEqual(vscode.workspace.getConfiguration(section).get<number>("indentSize"), 2);
    } finally {
      await vscode.workspace.getConfiguration(section).update("indentSize", undefined, undefined);
    }
  });

  test("can programmatically change optimiser profile", async () => {
    const section = "tclLsp.optimiser";
    try {
      await vscode.workspace.getConfiguration(section).update("profile", "aggressive", undefined);
      assert.strictEqual(
        vscode.workspace.getConfiguration(section).get<string>("profile"),
        "aggressive",
      );
    } finally {
      await vscode.workspace.getConfiguration(section).update("profile", undefined, undefined);
    }
  });

  test("can toggle individual diagnostic codes", async () => {
    const section = "tclLsp.diagnostics";
    const original = vscode.workspace.getConfiguration(section).get<boolean>("W100", true);
    try {
      await vscode.workspace.getConfiguration(section).update("W100", !original, undefined);
      assert.strictEqual(
        vscode.workspace.getConfiguration(section).get<boolean>("W100"),
        !original,
      );
    } finally {
      await vscode.workspace.getConfiguration(section).update("W100", undefined, undefined);
    }
  });

  test("can toggle individual optimiser rules", async () => {
    const section = "tclLsp.optimiser";
    try {
      await vscode.workspace.getConfiguration(section).update("O100", true, undefined);
      assert.strictEqual(vscode.workspace.getConfiguration(section).get<boolean>("O100"), true);
    } finally {
      await vscode.workspace.getConfiguration(section).update("O100", undefined, undefined);
    }
  });

  test("can change runtime validation adapter", async () => {
    const section = "tclLsp.runtimeValidation";
    try {
      await vscode.workspace.getConfiguration(section).update("adapter", "tcl-syntax", undefined);
      assert.strictEqual(
        vscode.workspace.getConfiguration(section).get<string>("adapter"),
        "tcl-syntax",
      );
    } finally {
      await vscode.workspace.getConfiguration(section).update("adapter", undefined, undefined);
    }
  });

  test("can change style.nonAscii setting", async () => {
    const section = "tclLsp.style";
    try {
      await vscode.workspace.getConfiguration(section).update("nonAscii", "strict", undefined);
      assert.strictEqual(
        vscode.workspace.getConfiguration(section).get<string>("nonAscii"),
        "strict",
      );
    } finally {
      await vscode.workspace.getConfiguration(section).update("nonAscii", undefined, undefined);
    }
  });

  test("can change trace server level", async () => {
    const section = "tcl-lsp.trace";
    try {
      await vscode.workspace.getConfiguration(section).update("server", "verbose", undefined);
      assert.strictEqual(
        vscode.workspace.getConfiguration(section).get<string>("server"),
        "verbose",
      );
    } finally {
      await vscode.workspace.getConfiguration(section).update("server", undefined, undefined);
    }
  });
});
