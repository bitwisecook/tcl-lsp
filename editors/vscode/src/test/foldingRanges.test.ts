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
import { getDocUri, activate, pollUntil } from "./helper";

suite("Folding Ranges", () => {
  const docUri = getDocUri("folding.tcl");

  test("returns folding ranges for procs and namespaces", async () => {
    await activate(docUri);

    const ranges = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeFoldingRangeProvider", docUri),
      (r) => Array.isArray(r) && r.length > 0,
      { timeout: 10_000, label: "folding ranges present" },
    )) as vscode.FoldingRange[];

    assert.ok(ranges, "Should return folding ranges");
    assert.ok(ranges.length > 0, `Should have at least one folding range, got ${ranges.length}`);
  });

  test("proc body is foldable", async () => {
    await activate(docUri);

    const ranges = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeFoldingRangeProvider", docUri),
      (r) =>
        Array.isArray(r) && (r as vscode.FoldingRange[]).some((fr) => fr.start <= 1 && fr.end >= 7),
      { timeout: 10_000, label: "proc body foldable" },
    )) as vscode.FoldingRange[];

    assert.ok(ranges && ranges.length > 0, "Should have folding ranges");

    // The 'greet' proc starts at line 1 (0-indexed)
    const procRange = ranges.find((r) => r.start <= 1 && r.end >= 7);
    assert.ok(procRange, `Should have a folding range covering the greet proc body`);
  });

  test("namespace body is foldable", async () => {
    await activate(docUri);

    const ranges = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeFoldingRangeProvider", docUri),
      (r) =>
        Array.isArray(r) &&
        (r as vscode.FoldingRange[]).some((fr) => fr.start >= 10 && fr.end >= 13),
      { timeout: 10_000, label: "namespace body foldable" },
    )) as vscode.FoldingRange[];

    assert.ok(ranges && ranges.length > 0, "Should have folding ranges");

    // The namespace eval block starts around line 10
    const nsRange = ranges.find((r) => r.start >= 10 && r.end >= 13);
    assert.ok(nsRange, "Should have a folding range covering the namespace block");
  });

  test("backslash-continued command folds as one logical line (issue #541)", async () => {
    await activate(docUri);

    const ranges = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeFoldingRangeProvider", docUri),
      (r) =>
        Array.isArray(r) &&
        (r as vscode.FoldingRange[]).some((fr) => fr.start === 17 && fr.end >= 19),
      { timeout: 10_000, label: "backslash-continued command folds" },
    )) as vscode.FoldingRange[];

    assert.ok(ranges && ranges.length > 0, "Should have folding ranges");

    // The continuation run `MyLongCommand $argument1 \ ... $argument3` spans the
    // last three lines of the fixture (0-indexed 17..19).  Line-continuation
    // folding was dropped by the folding rewrite and restored in #541, so a
    // fold must cover that run.
    const contRange = ranges.find((r) => r.start === 17 && r.end >= 19);
    assert.ok(
      contRange,
      `Should fold the backslash-continued command (lines 17..19); got ${JSON.stringify(
        ranges.map((r) => [r.start, r.end]),
      )}`,
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
