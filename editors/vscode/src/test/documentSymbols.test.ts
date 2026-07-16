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

suite("Document Symbols", () => {
  const docUri = getDocUri("procs.tcl");

  test("returns document symbols for proc definitions", async () => {
    await activate(docUri);

    const symbols = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeDocumentSymbolProvider", docUri),
      (r) => {
        const syms = r as (vscode.DocumentSymbol[] | vscode.SymbolInformation[]) | undefined;
        if (!syms || syms.length === 0) return false;
        const names = syms.map((s) => s.name);
        return names.some((n) => n.includes("fib")) && names.some((n) => n.includes("factorial"));
      },
      { timeout: 10_000, label: "document symbols for proc definitions" },
    )) as vscode.DocumentSymbol[] | vscode.SymbolInformation[];

    assert.ok(symbols, "Should return symbols");
    assert.ok(symbols.length > 0, "Should find at least one symbol");

    const names = symbols.map((s) => s.name);
    assert.ok(
      names.some((n) => n.includes("fib")),
      `Should find 'fib' proc, got: ${names}`,
    );
    assert.ok(
      names.some((n) => n.includes("factorial")),
      `Should find 'factorial' proc, got: ${names}`,
    );
  });

  test("symbols have proper kinds", async () => {
    await activate(docUri);

    const symbols = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeDocumentSymbolProvider", docUri),
      (r) => {
        const syms = r as (vscode.DocumentSymbol[] | vscode.SymbolInformation[]) | undefined;
        return !!syms && syms.length > 0;
      },
      { timeout: 10_000, label: "document symbols (kinds)" },
    )) as vscode.DocumentSymbol[] | vscode.SymbolInformation[];

    assert.ok(symbols && symbols.length > 0, "Should return symbols");

    // Proc definitions should be functions
    for (const sym of symbols) {
      if (sym.name.includes("fib") || sym.name.includes("factorial")) {
        assert.strictEqual(
          sym.kind,
          vscode.SymbolKind.Function,
          `Proc '${sym.name}' should have Function kind`,
        );
      }
    }
  });

  test("symbols have valid ranges", async () => {
    await activate(docUri);

    const symbols = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeDocumentSymbolProvider", docUri),
      (r) => {
        const syms = r as vscode.DocumentSymbol[] | undefined;
        return !!syms && syms.length > 0;
      },
      { timeout: 10_000, label: "document symbols (ranges)" },
    )) as vscode.DocumentSymbol[];

    assert.ok(symbols && symbols.length > 0, "Should return symbols");

    for (const sym of symbols) {
      if ("range" in sym) {
        const range = sym.range;
        assert.ok(
          range.start.line >= 0,
          `Symbol '${sym.name}' should have a non-negative start line`,
        );
        assert.ok(
          range.end.line >= range.start.line,
          `Symbol '${sym.name}' end line should be >= start line`,
        );
      }
    }
  });

  // Issue #790: tcltest `test` cases appear in the outline for navigation.
  test("lists tcltest test cases as symbols", async () => {
    const tcltestUri = getDocUri("tcltestImport.tcl");
    await activate(tcltestUri);

    const symbols = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeDocumentSymbolProvider", tcltestUri),
      (r) => {
        const syms = r as (vscode.DocumentSymbol[] | vscode.SymbolInformation[]) | undefined;
        return !!syms && syms.some((s) => s.name === "t-1");
      },
      { timeout: 10_000, label: "tcltest test-case symbol" },
    )) as vscode.DocumentSymbol[] | vscode.SymbolInformation[];

    const testSym = symbols.find((s) => s.name === "t-1");
    assert.ok(testSym, `Should list the 't-1' test case, got: ${symbols.map((s) => s.name)}`);
    // No dedicated editor kind for a test case — it surfaces as a function.
    assert.strictEqual(
      testSym.kind,
      vscode.SymbolKind.Function,
      "test case should have Function kind",
    );
  });

  // Issue #790: every tcltest command that binds a name gets its own kind —
  // test cases (Function), constraints (Constant), custom match modes (Operator).
  test("lists tcltest constraints and match modes with their own kinds", async () => {
    const defsUri = getDocUri("tcltestDefinitions.tcl");
    await activate(defsUri);

    const symbols = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeDocumentSymbolProvider", defsUri),
      (r) => {
        const syms = r as (vscode.DocumentSymbol[] | vscode.SymbolInformation[]) | undefined;
        const names = (syms ?? []).map((s) => s.name);
        return names.includes("needsNet") && names.includes("approx") && names.includes("case-1");
      },
      { timeout: 10_000, label: "tcltest definition symbols" },
    )) as vscode.DocumentSymbol[] | vscode.SymbolInformation[];

    const byName = new Map(symbols.map((s) => [s.name, s]));
    assert.strictEqual(
      byName.get("needsNet")?.kind,
      vscode.SymbolKind.Constant,
      "constraint should have Constant kind",
    );
    assert.strictEqual(
      byName.get("approx")?.kind,
      vscode.SymbolKind.Operator,
      "custom match mode should have Operator kind",
    );
    assert.strictEqual(
      byName.get("case-1")?.kind,
      vscode.SymbolKind.Function,
      "test case should have Function kind",
    );
  });

  // Issue #934: a proc named `:` (and a namespace named `:`) must produce
  // symbols with real, non-empty names.  The 2.1.9 regression returned an
  // empty name, which VS Code's DocumentSymbol validation rejects with
  // "name must not be falsy" — killing the entire outline.
  test("colon-named definitions keep non-empty symbol names (#934)", async () => {
    const uri = getDocUri("colonNames.tcl");
    await activate(uri);

    const symbols = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeDocumentSymbolProvider", uri),
      (r) => {
        const syms = r as (vscode.DocumentSymbol[] | vscode.SymbolInformation[]) | undefined;
        return !!syms && syms.length > 0;
      },
      { timeout: 10_000, label: "document symbols for colon-named defs" },
    )) as vscode.DocumentSymbol[] | vscode.SymbolInformation[];

    assert.ok(symbols && symbols.length > 0, "outline must not be rejected");
    const names = symbols.map((s) => s.name);
    assert.ok(
      names.every((n) => n.length > 0),
      `no falsy symbol names: ${JSON.stringify(names)}`,
    );
    assert.ok(names.includes(":"), `the ':' proc appears in the outline: ${names}`);
    assert.ok(names.includes("a:b"), `the 'a:b' proc appears in the outline: ${names}`);
  });
});
