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

  test("provides history subcommand completions for a bare call", async () => {
    // `history` is a `WithSubcommands` registry command like `string`, but a
    // bare call is itself valid Tcl (defaults to `history info`, so it does
    // not fire E001 — see diagnostics.test.ts); completion at the bare
    // position must still offer its subcommand list.
    const historyUri = getDocUri("completion-history.tcl");
    await activate(historyUri);

    const position = new vscode.Position(0, 8);

    const result = (await pollUntil(
      () =>
        vscode.commands.executeCommand(
          "vscode.executeCompletionItemProvider",
          historyUri,
          position,
        ),
      (r) => {
        const list = r as vscode.CompletionList | undefined;
        if (!list) return false;
        const labels = list.items.map((item) =>
          typeof item.label === "string" ? item.label : item.label.label,
        );
        return labels.includes("info");
      },
      { timeout: 10_000, label: "history subcommand completions" },
    )) as vscode.CompletionList;

    assert.ok(result, "Completion result should not be null");
    const labels = result.items.map((item) =>
      typeof item.label === "string" ? item.label : item.label.label,
    );
    for (const expected of ["add", "clear", "info", "keep"]) {
      assert.ok(labels.includes(expected), `missing ${expected} in [${labels.join(", ")}]`);
    }
  });

  test("offers `link` only inside a TclOO method body", async () => {
    // `link` is `oo::Helpers::link`: real Tcl 9.0 resolves it only from
    // inside a method body, and `link foo` at the top level raises
    // `invalid command name "link"` (`info commands ::link` is empty).
    // Completion follows the same scoping, so the same partial word offers
    // it in one place and not the other (issue #1026).
    const ooUri = getDocUri("ooMethodContext.tcl");
    await activate(ooUri);

    const labelsAt = async (position: vscode.Position, label: string) => {
      const result = (await pollUntil(
        () =>
          vscode.commands.executeCommand("vscode.executeCompletionItemProvider", ooUri, position),
        (r) => {
          const list = r as vscode.CompletionList | undefined;
          return !!list && list.items.length > 0;
        },
        { timeout: 10_000, label },
      )) as vscode.CompletionList;
      return result.items.map((item) =>
        typeof item.label === "string" ? item.label : item.label.label,
      );
    };

    // Line 1 is the top-level `lin`; line 5 is the `lin` inside `method bar`.
    const topLevel = await labelsAt(new vscode.Position(1, 3), "top-level completions");
    assert.ok(
      !topLevel.includes("link"),
      `link must not be offered at the top level: ${topLevel.slice(0, 20).join(", ")}`,
    );

    const inMethod = await labelsAt(new vscode.Position(5, 11), "method-body completions");
    assert.ok(
      inMethod.includes("link"),
      `link must be offered inside a method body: ${inMethod.slice(0, 20).join(", ")}`,
    );
  });
});
