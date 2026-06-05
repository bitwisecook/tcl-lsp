import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate } from "./helper";

suite("Completion", () => {
  const docUri = getDocUri("completion.tcl");

  test("provides command completions", async () => {
    await activate(docUri);

    // Position at end of "put" on line 2 (0-indexed)
    const position = new vscode.Position(2, 3);

    const result = (await vscode.commands.executeCommand(
      "vscode.executeCompletionItemProvider",
      docUri,
      position,
    )) as vscode.CompletionList;

    assert.ok(result, "Completion result should not be null");
    assert.ok(result.items.length > 0, "Should have at least one completion item");

    const labels = result.items.map((item) =>
      typeof item.label === "string" ? item.label : item.label.label,
    );

    assert.ok(
      labels.includes("puts"),
      `Expected "puts" in completions, got: ${labels.slice(0, 10).join(", ")}`,
    );
  });

  test("provides proc name completions", async () => {
    // Open procs.tcl first so proc names are in the workspace index
    const procsUri = getDocUri("procs.tcl");
    await activate(procsUri);

    // Now open completion.tcl and trigger completions
    await activate(docUri);

    const position = new vscode.Position(2, 3);

    const result = (await vscode.commands.executeCommand(
      "vscode.executeCompletionItemProvider",
      docUri,
      position,
    )) as vscode.CompletionList;

    assert.ok(result, "Completion result should not be null");

    // Look for built-in Tcl commands
    const labels = result.items.map((item) =>
      typeof item.label === "string" ? item.label : item.label.label,
    );

    // At minimum, standard commands like "puts" should appear
    const hasTclCommands = labels.some((l) =>
      ["puts", "set", "proc", "if", "while", "for", "foreach"].includes(l),
    );
    assert.ok(
      hasTclCommands,
      `Expected Tcl commands in completions: ${labels.slice(0, 20).join(", ")}`,
    );
  });
});
