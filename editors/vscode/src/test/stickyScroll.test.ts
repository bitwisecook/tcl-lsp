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
import { TCL_LANGUAGE_IDS } from "../languageIds";
import { getDocUri, activate, pollUntil } from "./helper";

// VS Code picks the sticky-scroll model per language with a fallback chain:
// outline model → folding-range provider → indentation heuristic.  A
// *non-empty* outline stops the chain, so registering a document-symbol
// provider silently replaces the indentation heuristic the user had before
// the extension was installed (issue #1122).  For Tcl code the outline is
// definitions only — a script whose top level is `if` / `for` / `foreach`
// contributes nothing sticky, so sticky scroll goes blank.  Our folding
// ranges cover exactly those blocks, so Tcl code languages default to the
// folding-provider model.  BIG-IP config is the opposite shape (a rich
// stanza outline, almost no folds) and stays on the outline model.
//
// The version-pinned dialects (`tcl8.4`, `tcl9.0`, …) are excluded because a
// language id containing a `.` cannot be used as a configuration override
// identifier: VS Code splits `[tcl8.4]` on the dot while building the
// default-configuration value tree, throws, and drops every remaining
// override in the same `configurationDefaults` block.  See the
// "dotted language ids" test below.
const DOTTED_LANGUAGE_IDS = [...TCL_LANGUAGE_IDS].filter((id) => id.includes("."));
const STICKY_FOLDING_LANGUAGES = [...TCL_LANGUAGE_IDS].filter(
  (id) => id !== "tcl-bigip" && !id.includes("."),
);

suite("Sticky Scroll", () => {
  const stickyModelFor = (languageId: string): string | undefined =>
    vscode.workspace
      .getConfiguration("editor", { languageId })
      .get<string>("stickyScroll.defaultModel");

  for (const languageId of STICKY_FOLDING_LANGUAGES) {
    test(`'${languageId}' defaults sticky scroll to the folding-provider model`, () => {
      assert.strictEqual(
        stickyModelFor(languageId),
        "foldingProviderModel",
        `'${languageId}' should default editor.stickyScroll.defaultModel to foldingProviderModel`,
      );
    });
  }

  test("dotted language ids cannot carry a sticky-scroll default", () => {
    // Known VS Code limitation, asserted so we notice if it is ever fixed:
    // `"[tcl8.4]"` in `configurationDefaults` makes VS Code throw while
    // updating the default configuration model, so the override never lands
    // (and takes the rest of the block with it).  If this starts returning
    // `foldingProviderModel`, add the dotted ids back to the manifest.
    assert.ok(DOTTED_LANGUAGE_IDS.length > 0, "expected version-pinned dialects like tcl9.0");
    for (const languageId of DOTTED_LANGUAGE_IDS) {
      assert.strictEqual(stickyModelFor(languageId), "outlineModel", languageId);
    }
  });

  test("'tcl-bigip' keeps the outline model", () => {
    // The BIG-IP outline is the stanza tree (module → kind → object) and is
    // far richer than the handful of folds a `.conf` produces, so the
    // default chain is already the right one there.
    assert.strictEqual(stickyModelFor("tcl-bigip"), "outlineModel");
  });

  test("control-flow-only Tcl has no sticky outline but does have folds", async () => {
    // The regression the default guards against: every symbol in this
    // document is single-line, so VS Code's outline sticky model yields
    // nothing while the folding model pins `for` / `if` / `else`.
    await activate(getDocUri("folding.tcl"));

    const doc = await vscode.workspace.openTextDocument({
      language: "tcl",
      content: [
        "set total 0",
        "",
        "for {set i 0} {$i < 5} {incr i} {",
        "    set total [expr {$total + $i}]",
        "}",
        "",
        "if {$total > 5} {",
        '    puts "total is $total"',
        "} else {",
        '    puts "small total"',
        "}",
        "",
      ].join("\n"),
    });
    await vscode.window.showTextDocument(doc);

    const symbols =
      ((await vscode.commands.executeCommand(
        "vscode.executeDocumentSymbolProvider",
        doc.uri,
      )) as vscode.DocumentSymbol[]) ?? [];
    const multiLine = symbols.filter((s) => s.selectionRange.start.line !== s.range.end.line);
    assert.deepStrictEqual(
      multiLine.map((s) => s.name),
      [],
      "outline model would have nothing to stick for a control-flow-only script",
    );

    const ranges = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeFoldingRangeProvider", doc.uri),
      (r) => Array.isArray(r) && r.length > 0,
      { timeout: 10_000, label: "folding ranges present" },
    )) as vscode.FoldingRange[];
    const starts = ranges.map((r) => r.start).sort((a, b) => a - b);
    assert.deepStrictEqual(
      starts,
      [2, 6, 8],
      `folding should pin the for/if/else headers, got ${JSON.stringify(ranges)}`,
    );
  });
});
