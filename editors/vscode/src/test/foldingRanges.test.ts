import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate } from "./helper";

suite("Folding Ranges", () => {
  const docUri = getDocUri("folding.tcl");

  test("returns folding ranges for procs and namespaces", async () => {
    await activate(docUri);

    const ranges = (await vscode.commands.executeCommand(
      "vscode.executeFoldingRangeProvider",
      docUri,
    )) as vscode.FoldingRange[];

    assert.ok(ranges, "Should return folding ranges");
    assert.ok(ranges.length > 0, `Should have at least one folding range, got ${ranges.length}`);
  });

  test("proc body is foldable", async () => {
    await activate(docUri);

    const ranges = (await vscode.commands.executeCommand(
      "vscode.executeFoldingRangeProvider",
      docUri,
    )) as vscode.FoldingRange[];

    assert.ok(ranges && ranges.length > 0, "Should have folding ranges");

    // The 'greet' proc starts at line 1 (0-indexed)
    const procRange = ranges.find((r) => r.start <= 1 && r.end >= 7);
    assert.ok(procRange, `Should have a folding range covering the greet proc body`);
  });

  test("namespace body is foldable", async () => {
    await activate(docUri);

    const ranges = (await vscode.commands.executeCommand(
      "vscode.executeFoldingRangeProvider",
      docUri,
    )) as vscode.FoldingRange[];

    assert.ok(ranges && ranges.length > 0, "Should have folding ranges");

    // The namespace eval block starts around line 10
    const nsRange = ranges.find((r) => r.start >= 10 && r.end >= 13);
    assert.ok(nsRange, "Should have a folding range covering the namespace block");
  });

  test("all folding ranges have valid line numbers", async () => {
    await activate(docUri);

    const ranges = (await vscode.commands.executeCommand(
      "vscode.executeFoldingRangeProvider",
      docUri,
    )) as vscode.FoldingRange[];

    for (const range of ranges || []) {
      assert.ok(range.start >= 0, `Folding start should be >= 0, got ${range.start}`);
      assert.ok(
        range.end >= range.start,
        `Folding end (${range.end}) should be >= start (${range.start})`,
      );
    }
  });
});
