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
