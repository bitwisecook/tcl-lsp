import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate } from "./helper";

suite("Selection Range", () => {
  const docUri = getDocUri("procs.tcl");

  test("returns nested selection ranges for a position inside a proc body", async () => {
    await activate(docUri);

    // Position inside the if body of fib proc (line 3, col 8)
    const pos = new vscode.Position(3, 8);

    const ranges = (await vscode.commands.executeCommand(
      "vscode.executeSelectionRangeProvider",
      docUri,
      [pos],
    )) as vscode.SelectionRange[] | undefined;

    assert.ok(ranges, "Should return selection ranges");
    assert.ok(ranges.length > 0, "Should return at least one selection range");

    // Walk the chain - each parent should be strictly wider than its child
    let current = ranges[0];
    let depth = 0;
    while (current.parent) {
      const child = current.range;
      const parent = current.parent.range;
      assert.ok(
        parent.contains(child),
        `Parent range at depth ${depth} should contain child range`,
      );
      current = current.parent;
      depth++;
    }

    assert.ok(depth >= 1, `Selection range chain should have at least 2 levels, got ${depth + 1}`);
  });

  test("top-level position returns selection ranges", async () => {
    await activate(docUri);

    // Position at the top level 'puts' on line 16
    const pos = new vscode.Position(16, 0);

    const ranges = (await vscode.commands.executeCommand(
      "vscode.executeSelectionRangeProvider",
      docUri,
      [pos],
    )) as vscode.SelectionRange[] | undefined;

    assert.ok(ranges, "Should return selection ranges for top-level code");
    assert.ok(ranges.length > 0, "Should return at least one selection range");
  });

  test("multiple positions return corresponding ranges", async () => {
    await activate(docUri);

    const positions = [new vscode.Position(1, 5), new vscode.Position(9, 5)];

    const ranges = (await vscode.commands.executeCommand(
      "vscode.executeSelectionRangeProvider",
      docUri,
      positions,
    )) as vscode.SelectionRange[] | undefined;

    assert.ok(ranges, "Should return selection ranges for multiple positions");
    assert.strictEqual(
      ranges.length,
      positions.length,
      "Should return one selection range per position",
    );
  });
});
