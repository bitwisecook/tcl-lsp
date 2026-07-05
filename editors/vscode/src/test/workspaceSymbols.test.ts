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

suite("Workspace Symbols", () => {
  suiteSetup(async () => {
    // Ensure the server has indexed at least one file
    await activate(getDocUri("procs.tcl"));
  });

  test("returns workspace symbols matching a query", async () => {
    const symbols = (await vscode.commands.executeCommand(
      "vscode.executeWorkspaceSymbolProvider",
      "fib",
    )) as vscode.SymbolInformation[] | undefined;

    assert.ok(symbols, "Workspace symbol provider should return results");
    assert.ok(symbols.length > 0, "Should find at least one symbol matching 'fib'");

    const fib = symbols.find((s) => s.name.includes("fib"));
    assert.ok(fib, `Should find a symbol containing 'fib', got: ${symbols.map((s) => s.name)}`);
  });

  test("returns symbols for factorial query", async () => {
    const symbols = (await vscode.commands.executeCommand(
      "vscode.executeWorkspaceSymbolProvider",
      "factorial",
    )) as vscode.SymbolInformation[] | undefined;

    assert.ok(symbols, "Should return results for 'factorial' query");
    assert.ok(symbols.length > 0, "Should find 'factorial' proc");
  });

  test("empty query returns some symbols", async () => {
    const symbols = (await vscode.commands.executeCommand(
      "vscode.executeWorkspaceSymbolProvider",
      "",
    )) as vscode.SymbolInformation[] | undefined;

    // An empty query should return some or all symbols
    assert.ok(symbols !== undefined, "Empty query should not fail");
  });

  test("workspace symbols have valid locations", async () => {
    const symbols = (await vscode.commands.executeCommand(
      "vscode.executeWorkspaceSymbolProvider",
      "fib",
    )) as vscode.SymbolInformation[];

    if (symbols && symbols.length > 0) {
      for (const sym of symbols) {
        assert.ok(sym.location, `Symbol '${sym.name}' should have a location`);
        assert.ok(sym.location.uri, `Symbol '${sym.name}' should have a location URI`);
        assert.ok(
          sym.location.range.start.line >= 0,
          `Symbol '${sym.name}' should have a valid line number`,
        );
      }
    }
  });
});
