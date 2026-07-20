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

  // Regression for issue #923: a proc nested two `namespace eval` levels
  // deep, called from a Tk `bind` callback script by its fully-qualified
  // name, must be found — the reported symptom was that this exact call
  // shape showed "0 references".
  test("finds a fully-qualified proc call embedded in a bind callback inside a two-level nested namespace", async () => {
    const nsUri = getDocUri("issue923NestedNamespace.tcl");
    await activate(nsUri);

    // Position on "specAddButtonPopUp923" at its definition, line 2:
    // "        proc specAddButtonPopUp923 {x y} {"
    const position = new vscode.Position(2, 20);

    const locations = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeReferenceProvider", nsUri, position),
      (r) => Array.isArray(r) && r.length >= 2,
      { timeout: 10_000, label: "references for nested-namespace proc" },
    )) as vscode.Location[];

    assert.ok(locations, "References result should not be null");
    assert.ok(
      locations.length >= 2,
      `Expected declaration + the qualified bind call site, got ${locations.length}`,
    );
    // The bind callback with the fully-qualified call is line 8.
    const lines = locations.map((l) => l.range.start.line);
    assert.ok(
      lines.includes(8),
      `Expected the fully-qualified bind callback call site (line 8) among ${JSON.stringify(lines)}`,
    );
  });

  test("finds a bare proc call embedded in a bind callback inside the same nested namespace", async () => {
    const nsUri = getDocUri("issue923NestedNamespace.tcl");
    await activate(nsUri);

    // Position on "testAddButtonPopUp923" at its definition, line 5:
    // "        proc testAddButtonPopUp923 {x y} {"
    const position = new vscode.Position(5, 20);

    const locations = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeReferenceProvider", nsUri, position),
      (r) => Array.isArray(r) && r.length >= 2,
      { timeout: 10_000, label: "references for bare nested-namespace call" },
    )) as vscode.Location[];

    assert.ok(locations, "References result should not be null");
    assert.ok(
      locations.length >= 2,
      `Expected declaration + the bare bind call site, got ${locations.length}`,
    );
    const lines = locations.map((l) => l.range.start.line);
    assert.ok(
      lines.includes(9),
      `Expected the bare bind callback call site (line 9) among ${JSON.stringify(lines)}`,
    );
  });

  // Issue #923: a class named as a `superclass` is a reference to that class.
  // In `oo-shapes.tcl`, `Animal` is subclassed by both `Dog` and `Cat`, so its
  // reference set must include both `superclass Animal` sites.
  test("finds superclass usages as references to the base class", async () => {
    const uri = getDocUri("oo-shapes.tcl");
    await activate(uri);

    // Position on "Animal" in its declaration `oo::class create Animal` (line 1).
    const position = new vscode.Position(1, 18);

    const locations = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeReferenceProvider", uri, position),
      (r) => Array.isArray(r) && r.length >= 3,
      { timeout: 10_000, label: "references for a subclassed class" },
    )) as vscode.Location[];

    assert.ok(locations, "References result should not be null");
    const lines = locations.map((l) => l.range.start.line);
    // Dog's `superclass Animal` (line 7) and Cat's `superclass Animal` (line 12).
    assert.ok(
      lines.includes(7) && lines.includes(12),
      `Expected both superclass sites (lines 7 and 12) among ${JSON.stringify(lines)}`,
    );
  });
});
