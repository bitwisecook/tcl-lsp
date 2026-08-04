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

suite("Hover", () => {
  const docUri = getDocUri("procs.tcl");

  test("shows hover for proc call", async () => {
    await activate(docUri);

    // "fib" call on line 16: puts "fib(10) = [fib 10]"
    // The inner "fib" is inside [fib 10], somewhere around column 22
    // Let's use a position on the proc name "fib" at line 0, col 5
    // proc fib {n} {  -- "fib" starts at col 5
    const position = new vscode.Position(1, 6);

    const hovers = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeHoverProvider", docUri, position),
      (r) => Array.isArray(r) && r.length > 0,
      { timeout: 10_000, label: "hover for proc call" },
    )) as vscode.Hover[];

    assert.ok(hovers, "Hover result should not be null");
    assert.ok(hovers.length > 0, "Should have at least one hover");

    const hoverText = hovers
      .flatMap((h) => h.contents)
      .map((c) => {
        if (typeof c === "string") return c;
        if (c instanceof vscode.MarkdownString) return c.value;
        // MarkdownString with language
        return (c as { value: string }).value;
      })
      .join("\n");

    assert.ok(hoverText.length > 0, "Hover content should not be empty");
    // The hover should mention "fib" or show the signature
    assert.ok(hoverText.includes("fib"), `Hover should mention "fib", got: ${hoverText}`);
  });

  // Peer of issue #776: a bare command imported into the global scope hovers as
  // its qualified spec — `test` after `namespace import ::tcltest::*`.
  test("resolves hover for an imported command", async () => {
    const uri = getDocUri("tcltestImport.tcl");
    await activate(uri);
    // `test` is the command head on line 1.
    const position = new vscode.Position(1, 2);

    const hovers = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeHoverProvider", uri, position),
      (r) => Array.isArray(r) && r.length > 0,
      { timeout: 10_000, label: "hover for imported command" },
    )) as vscode.Hover[];

    const hoverText = hovers
      .flatMap((h) => h.contents)
      .map((c) => (typeof c === "string" ? c : (c as { value: string }).value))
      .join("\n");

    assert.ok(
      hoverText.includes("tcltest::test"),
      `imported 'test' should hover as tcltest::test, got: ${hoverText}`,
    );
  });

  // Issue #1139 (issue #923 audit idx 22): a callee that binds a literal
  // caller-frame name in its own body (`upvar name name`) creates the
  // variable in the calling frame with no call-site word to point at.
  // Hovering the `$name` read must render the caller-frame card naming the
  // creating callee instead of answering nothing.
  test("hovers a literal upvar caller-frame read with the caller-frame card", async () => {
    const uri = getDocUri("callerFrame.tcl");
    await activate(uri);
    // Line 9: `    set out $name` — position inside `name` (col 14).
    const position = new vscode.Position(9, 14);

    const hovers = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeHoverProvider", uri, position),
      (r) => Array.isArray(r) && r.length > 0,
      { timeout: 10_000, label: "hover for literal caller-frame read" },
    )) as vscode.Hover[];

    const hoverText = hovers
      .flatMap((h) => h.contents)
      .map((c) => (typeof c === "string" ? c : (c as { value: string }).value))
      .join("\n");

    assert.ok(
      hoverText.includes("Caller-frame variable") && hoverText.includes("NameProcess"),
      `expected the caller-frame card naming the callee, got: ${hoverText}`,
    );
  });
});
