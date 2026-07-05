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

suite("Go to Definition", () => {
  const docUri = getDocUri("procs.tcl");

  test("navigates from proc call to proc definition", async () => {
    await activate(docUri);

    // Line 16: puts "fib(10) = [fib 10]"
    // We need a position on a "fib" reference. Let's use the fib call
    // inside the proc body at line 5:
    //   return [expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}]
    // "fib" appears at roughly col 20
    // Actually, simpler: line 16 has "fib" starting around col 19
    // puts "fib(10) = [fib 10]"  -- "fib" after [ is at col 17
    const position = new vscode.Position(16, 17);

    const locations = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeDefinitionProvider", docUri, position),
      (r) => Array.isArray(r) && r.length > 0,
      { timeout: 10_000, label: "definition locations" },
    )) as vscode.Location[];

    assert.ok(locations, "Definition result should not be null");
    assert.ok(locations.length > 0, "Should find at least one definition");

    // The definition of "fib" is on line 1: proc fib {n} {
    const defLocation = locations[0];
    assert.strictEqual(
      defLocation.uri.fsPath,
      docUri.fsPath,
      "Definition should be in the same file",
    );
    assert.strictEqual(
      defLocation.range.start.line,
      1,
      `Definition should be on line 1 (proc fib), got line ${defLocation.range.start.line}`,
    );
  });
});
