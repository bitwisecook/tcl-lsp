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

  test("backslash line-continuation command is foldable", async () => {
    await activate(docUri);

    const ranges = (await vscode.commands.executeCommand(
      "vscode.executeFoldingRangeProvider",
      docUri,
    )) as vscode.FoldingRange[];

    assert.ok(ranges && ranges.length > 0, "Should have folding ranges");

    // The MyProcCall command is continued across the last three lines
    // (0-indexed 16..18); folding it collapses to the first of those lines.
    const contRange = ranges.find((r) => r.start === 16 && r.end === 18);
    assert.ok(
      contRange,
      `Should have a folding range covering the continued command, got ${JSON.stringify(
        ranges.map((r) => [r.start, r.end]),
      )}`,
    );
  });

  test("multi-line data literal is foldable", async () => {
    await activate(docUri);

    const ranges = (await vscode.commands.executeCommand(
      "vscode.executeFoldingRangeProvider",
      docUri,
    )) as vscode.FoldingRange[];

    // `set things { ... }` spans 0-indexed lines 20..23; the braced literal
    // folds with its closing `}` line left visible (ends at 22).
    const listRange = ranges.find((r) => r.start === 20 && r.end === 22);
    assert.ok(
      listRange,
      `Should fold the multi-line data literal, got ${JSON.stringify(
        ranges.map((r) => [r.start, r.end]),
      )}`,
    );
  });

  test("#region / #endregion markers fold with Region kind", async () => {
    await activate(docUri);

    const ranges = (await vscode.commands.executeCommand(
      "vscode.executeFoldingRangeProvider",
      docUri,
    )) as vscode.FoldingRange[];

    // The #region/#endregion pair spans 0-indexed lines 25..28.
    const regionRange = ranges.find((r) => r.start === 25 && r.end === 28);
    assert.ok(
      regionRange,
      `Should fold the #region block, got ${JSON.stringify(ranges.map((r) => [r.start, r.end]))}`,
    );
    assert.strictEqual(
      regionRange!.kind,
      vscode.FoldingRangeKind.Region,
      "region marker fold should carry Region kind",
    );
  });

  test("consecutive package require lines fold as Imports", async () => {
    await activate(docUri);

    const ranges = (await vscode.commands.executeCommand(
      "vscode.executeFoldingRangeProvider",
      docUri,
    )) as vscode.FoldingRange[];

    // The two trailing `package require` lines are 0-indexed 30..31.
    const importsRange = ranges.find((r) => r.start === 30 && r.end === 31);
    assert.ok(
      importsRange,
      `Should fold the import group, got ${JSON.stringify(ranges.map((r) => [r.start, r.end]))}`,
    );
    assert.strictEqual(
      importsRange!.kind,
      vscode.FoldingRangeKind.Imports,
      "import-group fold should carry Imports kind",
    );
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
