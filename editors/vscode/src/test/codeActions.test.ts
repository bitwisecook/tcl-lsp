import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate, waitForDiagnostics } from "./helper";

suite("Code Actions", () => {
  const docUri = getDocUri("diagnostics.tcl");
  const irulesDocUri = getDocUri("diagnostics-irules.irul");

  test("provides quick fix for W100 (unbraced expr)", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 1 });

    // Find the W100 diagnostic
    const w100 = diagnostics.find((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "W100";
    });
    assert.ok(w100, "W100 diagnostic should be present");

    // Request code actions at the W100 range
    const actions = (await vscode.commands.executeCommand(
      "vscode.executeCodeActionProvider",
      docUri,
      w100.range,
    )) as vscode.CodeAction[];

    assert.ok(actions, "Code actions should not be null");
    assert.ok(actions.length > 0, "Should have at least one code action");

    // Find the quick fix action
    const quickFix = actions.find(
      (a) => a.kind && a.kind.value === vscode.CodeActionKind.QuickFix.value,
    );
    assert.ok(quickFix, "Should have a QuickFix code action");
    assert.ok(quickFix.title.length > 0, "Quick fix should have a title");
  });

  test("provides quick fix for W304 (missing option terminator)", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 1 });

    const w304 = diagnostics.find((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "W304";
    });
    assert.ok(w304, "W304 diagnostic should be present");

    const actions = (await vscode.commands.executeCommand(
      "vscode.executeCodeActionProvider",
      docUri,
      w304.range,
    )) as vscode.CodeAction[];

    const quickFix = actions.find(
      (a) => typeof a.title === "string" && a.title.toLowerCase().includes("option terminator"),
    );
    assert.ok(quickFix, "Should provide an option terminator quick fix");
  });

  test("provides quick fix for W302 (catch result capture)", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 1 });

    const w302 = diagnostics.find((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "W302";
    });
    assert.ok(w302, "W302 diagnostic should be present");

    const actions = (await vscode.commands.executeCommand(
      "vscode.executeCodeActionProvider",
      docUri,
      w302.range,
    )) as vscode.CodeAction[];

    const resultFix = actions.find(
      (a) => typeof a.title === "string" && a.title.includes("catch result variable"),
    );
    const resultOptsFix = actions.find(
      (a) => typeof a.title === "string" && a.title.includes("result + options"),
    );
    assert.ok(resultFix, "Should provide a result capture quick fix");
    assert.ok(resultOptsFix, "Should provide a result+options capture quick fix");
  });

  test("provides guided collect bootstrap fix for IRULE1005", async () => {
    await activate(irulesDocUri);
    const diagnostics = await waitForDiagnostics(irulesDocUri, { minCount: 1 });

    const irule1005 = diagnostics.find((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "IRULE1005";
    });
    assert.ok(irule1005, "IRULE1005 diagnostic should be present");

    const actions = (await vscode.commands.executeCommand(
      "vscode.executeCodeActionProvider",
      irulesDocUri,
      irule1005.range,
    )) as vscode.CodeAction[];

    const collectFix = actions.find(
      (a) =>
        typeof a.title === "string" &&
        a.title.includes("collect") &&
        a.title.includes("CLIENT_ACCEPTED"),
    );
    assert.ok(collectFix, "Should provide a collect bootstrap quick fix");
  });
});
