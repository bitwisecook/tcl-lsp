import * as assert from "assert";
import * as vscode from "vscode";
import { activate, getDocUri } from "./helper";

suite("Command Registration", () => {
  const allCommands = [
    "tclLsp.restartServer",
    "tclLsp.selectDialect",
    "tclLsp.exportConfig",
    "tclLsp.optimiseDocument",
    "tclLsp.showOptimisations",
    "tclLsp.fixAllSafeIssues",
    "tclLsp.toggleDiagnostics",
    "tclLsp.toggleOptimiser",
    "tclLsp.toggleAi",
    "tclLsp.formatDocument",
    "tclLsp.minifyDocument",
    "tclLsp.unminifyError",
    "tclLsp.escapeSelection",
    "tclLsp.unescapeSelection",
    "tclLsp.base64EncodeSelection",
    "tclLsp.base64DecodeSelection",
    "tclLsp.copyFileAsBase64",
    "tclLsp.copyFileAsGzipBase64",
    "tclLsp.insertPackageRequire",
    "tclLsp.scaffoldPackageStarter",
    "tclLsp.insertIruleEventSkeleton",
    "tclLsp.insertTemplateSnippet",
    "tclLsp.runRuntimeValidation",
    "tclLsp.openCompilerExplorer",
    "tclLsp.openTkPreview",
    "tclLsp.insertIrule",
    "tclLsp.applyFix",
    "tclLsp.translateXc",
    "tclLsp.extractRule",
    "tclLsp.extractRulePick",
    "tclLsp.extractAllRules",
    "tclLsp.extractLinkedObjects",
    "tclLsp.renameSymbolAtPosition",
    "tclLsp.generateDocstring",
  ];

  let registeredCommands: string[];

  suiteSetup(async function () {
    this.timeout(60_000);
    // Ensure the extension is activated before querying commands.
    await activate(getDocUri("simple.tcl"));
    registeredCommands = await vscode.commands.getCommands(true);
  });

  for (const cmd of allCommands) {
    test(`${cmd} is registered`, () => {
      assert.ok(registeredCommands.includes(cmd), `Command '${cmd}' should be registered`);
    });
  }

  test("toggleDiagnostics toggles the feature setting", async () => {
    const config = vscode.workspace.getConfiguration("tclLsp.features");
    const before = config.get<boolean>("diagnostics", true);
    try {
      await vscode.commands.executeCommand("tclLsp.toggleDiagnostics");
      const after = config.get<boolean>("diagnostics");
      assert.strictEqual(after, !before, "diagnostics should be toggled");
    } finally {
      await config.update("diagnostics", before, vscode.ConfigurationTarget.Global);
    }
  });

  test("toggleOptimiser toggles the optimiser.enabled setting", async () => {
    const config = vscode.workspace.getConfiguration("tclLsp.optimiser");
    const before = config.get<boolean>("enabled", true);
    try {
      await vscode.commands.executeCommand("tclLsp.toggleOptimiser");
      const after = config.get<boolean>("enabled");
      assert.strictEqual(after, !before, "optimiser should be toggled");
    } finally {
      await config.update("enabled", before, vscode.ConfigurationTarget.Global);
    }
  });

  test("toggleAi toggles the ai.enabled setting", async () => {
    const config = vscode.workspace.getConfiguration("tclLsp.ai");
    const before = config.get<boolean>("enabled", true);
    try {
      await vscode.commands.executeCommand("tclLsp.toggleAi");
      const after = config.get<boolean>("enabled");
      assert.strictEqual(after, !before, "AI features should be toggled");
    } finally {
      await config.update("enabled", before, vscode.ConfigurationTarget.Global);
    }
  });

  test("formatDocument executes without error on a Tcl file", async () => {
    // Use an untitled document to avoid leaking state into other suites.
    const doc = await vscode.workspace.openTextDocument({
      language: "tcl",
      content: "set x 10\n",
    });
    await vscode.window.showTextDocument(doc);
    // Should not throw
    await vscode.commands.executeCommand("tclLsp.formatDocument");
  });
});
