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

// Issue #974 — the `expr` math functions, as VS Code actually surfaces them.
// Both defects were VS Code-visible: hovering a bare `sin(1.0)` inside `expr`
// popped up nothing, and completing inside an unfinished `expr {si` offered
// only unrelated same-prefixed procs.

import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate, pollUntil } from "./helper";

/** Flatten a hover result's contents to one string. */
function hoverText(hovers: vscode.Hover[]): string {
  return hovers
    .flatMap((h) => h.contents)
    .map((c) => {
      if (typeof c === "string") return c;
      if (c instanceof vscode.MarkdownString) return c.value;
      return (c as { value: string }).value;
    })
    .join("\n");
}

/** Completion labels as plain strings. */
function labelsOf(list: vscode.CompletionList): string[] {
  return list.items.map((item) => (typeof item.label === "string" ? item.label : item.label.label));
}

suite("Expr math functions (#974)", () => {
  const docUri = getDocUri("exprMathFunc.tcl");

  test("hovers a bare math-function call inside expr", async () => {
    await activate(docUri);
    // Line 5: `set finished [expr {sin(1.0)}]` — `sin` starts at column 20.
    const position = new vscode.Position(5, 21);

    const hovers = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeHoverProvider", docUri, position),
      (r) => Array.isArray(r) && r.length > 0,
      { timeout: 10_000, label: "hover for a bare expr math function" },
    )) as vscode.Hover[];

    const text = hoverText(hovers);
    assert.ok(text.includes("sin"), `hover should mention "sin", got: ${text}`);
    assert.ok(
      text.includes("math function"),
      `hover should identify it as an expr math function, got: ${text}`,
    );
    assert.ok(
      text.includes("::tcl::mathfunc::sin"),
      `hover should name the dispatch target, got: ${text}`,
    );
  });

  test("completes math functions inside an unfinished expr", async () => {
    await activate(docUri);
    // Line 6: `set unfinished [expr {si` — the cursor sits after `si`.
    const position = new vscode.Position(6, 24);

    const result = (await pollUntil(
      () =>
        vscode.commands.executeCommand("vscode.executeCompletionItemProvider", docUri, position),
      (r) => {
        const list = r as vscode.CompletionList | undefined;
        return !!list && labelsOf(list).includes("sin");
      },
      { timeout: 10_000, label: "expr math-function completions" },
    )) as vscode.CompletionList;

    const labels = labelsOf(result);
    assert.ok(labels.includes("sin"), `expected "sin", got: ${labels.slice(0, 15).join(", ")}`);
    assert.ok(labels.includes("sinh"), `expected "sinh", got: ${labels.slice(0, 15).join(", ")}`);
  });
});
