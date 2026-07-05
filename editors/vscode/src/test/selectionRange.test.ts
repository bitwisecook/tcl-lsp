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
