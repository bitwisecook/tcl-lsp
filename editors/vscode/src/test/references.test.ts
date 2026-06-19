import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate, pollUntil } from "./helper";

suite("Find References", () => {
  const docUri = getDocUri("procs.tcl");

  test("finds references for a proc", async () => {
    await activate(docUri);

    // Position on "fib" at its definition, line 1 col 5:
    // proc fib {n} {
    const position = new vscode.Position(1, 6);

    const locations = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeReferenceProvider", docUri, position),
      (r) => Array.isArray(r) && r.length >= 1,
      { timeout: 10_000, label: "references for proc" },
    )) as vscode.Location[];

    assert.ok(locations, "References result should not be null");

    // At minimum, the definition itself should be found
    assert.ok(
      locations.length >= 1,
      `Expected at least 1 reference to "fib", got ${locations.length}`,
    );

    // All locations should be in the same file
    for (const loc of locations) {
      assert.strictEqual(
        loc.uri.fsPath,
        docUri.fsPath,
        "All references should be in the same file",
      );
    }
  });
});
