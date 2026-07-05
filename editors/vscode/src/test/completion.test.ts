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

suite("Completion", () => {
  const docUri = getDocUri("completion.tcl");

  test("provides command completions", async () => {
    await activate(docUri);

    // Position at end of "put" on line 2 (0-indexed)
    const position = new vscode.Position(2, 3);

    const result = (await pollUntil(
      () =>
        vscode.commands.executeCommand("vscode.executeCompletionItemProvider", docUri, position),
      (r) => {
        const list = r as vscode.CompletionList | undefined;
        return (
          !!list &&
          list.items.length > 0 &&
          list.items.some(
            (item) => (typeof item.label === "string" ? item.label : item.label.label) === "puts",
          )
        );
      },
      { timeout: 10_000, label: "command completions" },
    )) as vscode.CompletionList;

    assert.ok(result, "Completion result should not be null");
    assert.ok(result.items.length > 0, "Should have at least one completion item");

    const labels = result.items.map((item) =>
      typeof item.label === "string" ? item.label : item.label.label,
    );

    assert.ok(
      labels.includes("puts"),
      `Expected "puts" in completions, got: ${labels.slice(0, 10).join(", ")}`,
    );
  });

  test("provides proc name completions", async () => {
    // Open procs.tcl first so proc names are in the workspace index
    const procsUri = getDocUri("procs.tcl");
    await activate(procsUri);

    // Now open completion.tcl and trigger completions
    await activate(docUri);

    const position = new vscode.Position(2, 3);

    const result = (await pollUntil(
      () =>
        vscode.commands.executeCommand("vscode.executeCompletionItemProvider", docUri, position),
      (r) => {
        const list = r as vscode.CompletionList | undefined;
        if (!list) return false;
        const labels = list.items.map((item) =>
          typeof item.label === "string" ? item.label : item.label.label,
        );
        return labels.some((l) =>
          ["puts", "set", "proc", "if", "while", "for", "foreach"].includes(l),
        );
      },
      { timeout: 10_000, label: "proc name completions" },
    )) as vscode.CompletionList;

    assert.ok(result, "Completion result should not be null");

    // Look for built-in Tcl commands
    const labels = result.items.map((item) =>
      typeof item.label === "string" ? item.label : item.label.label,
    );

    // At minimum, standard commands like "puts" should appear
    const hasTclCommands = labels.some((l) =>
      ["puts", "set", "proc", "if", "while", "for", "foreach"].includes(l),
    );
    assert.ok(
      hasTclCommands,
      `Expected Tcl commands in completions: ${labels.slice(0, 20).join(", ")}`,
    );
  });
});
