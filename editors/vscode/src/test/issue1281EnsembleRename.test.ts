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

// Issue #1281, editor-integration layer: an ensemble `-map` key must survive
// a rename of the command it dispatches to, while a `-subcommands` entry —
// which IS that command's tail — must follow it.
//
// This tier exists for the same reason issue #945 fault 1's does: the defect
// is an edit that must *not* be in the set. A rename the user triggers from
// the editor applies whatever comes back, unreviewed and in one keystroke, so
// the assertion worth making through a real VS Code session is the negative
// one — the dispatch line is absent from the entries the client would apply.
// The deep TP/FP/TN coverage lives in the analyser, `tcl-lsp-core` and native
// e2e suites (`rust/tcl-lsp-server/tests/e2e/issue1281_ensemble_rename.rs`),
// including applying the emitted edits and running the result under tclsh
// 8.6.14 / 9.0.4.

import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate } from "./helper";

async function renameEditLines(
  docUri: vscode.Uri,
  position: vscode.Position,
  newName: string,
): Promise<{ lines: number[]; texts: string[] }> {
  const edit = (await vscode.commands.executeCommand(
    "vscode.executeDocumentRenameProvider",
    docUri,
    position,
    newName,
  )) as vscode.WorkspaceEdit | undefined;
  assert.ok(edit, "rename should return a workspace edit");
  const entry = edit.entries().find(([uri]) => uri.toString() === docUri.toString());
  assert.ok(entry, "should include edits for the fixture");
  const [, textEdits] = entry;
  return {
    lines: textEdits.map((e) => e.range.start.line).sort((a, b) => a - b),
    texts: textEdits.map((e) => e.newText),
  };
}

suite("Issue #1281 ensemble rename provenance", () => {
  test("renaming a -map target leaves the dispatch subcommand word alone", async () => {
    const docUri = getDocUri("issue1281EnsembleMapRename.tcl");
    await activate(docUri);

    // `Show` in the `proc ::app::widget::Show` declaration (line 8, col 20).
    const { lines, texts } = await renameEditLines(docUri, new vscode.Position(8, 20), "Display");

    assert.ok(lines.includes(8), `the declaration follows the rename: ${JSON.stringify(lines)}`);
    assert.ok(
      lines.includes(9),
      `the -map value names the target and must follow it: ${JSON.stringify(lines)}`,
    );
    assert.ok(
      !lines.includes(10),
      `the -map key is arbitrary — the dispatch word must not be rewritten: ${JSON.stringify(lines)}`,
    );
    assert.ok(
      texts.every((t) => t.includes("Display")),
      `every edit splices the new name: ${JSON.stringify(texts)}`,
    );
  });

  test("renaming a -subcommands target still rewrites the dispatch word", async () => {
    // The true negative for the gate above: the fix must not over-correct
    // into never touching ensemble dispatch.
    const docUri = getDocUri("issue1281EnsembleSubcommandsRename.tcl");
    await activate(docUri);

    // `alpha` in the `proc alpha` declaration (line 7, col 9).
    const { lines } = await renameEditLines(docUri, new vscode.Position(7, 9), "beta");

    assert.ok(lines.includes(7), `the declaration follows the rename: ${JSON.stringify(lines)}`);
    assert.ok(
      lines.includes(8),
      `the -subcommands entry is the target's tail: ${JSON.stringify(lines)}`,
    );
    assert.ok(
      lines.includes(10),
      `the dispatch word must follow the target under -subcommands: ${JSON.stringify(lines)}`,
    );
  });

  test("find-references still reports the -map dispatch site", async () => {
    // The gate is a rename gate, not a reference rule — the dispatch site is
    // a genuine reference under either provenance, and the editor surfaces it.
    const docUri = getDocUri("issue1281EnsembleMapRename.tcl");
    await activate(docUri);

    const locations = (await vscode.commands.executeCommand(
      "vscode.executeReferenceProvider",
      docUri,
      new vscode.Position(8, 20),
    )) as vscode.Location[];
    const lines = (locations ?? [])
      .filter((l) => l.uri.toString() === docUri.toString())
      .map((l) => l.range.start.line);
    assert.ok(
      lines.includes(10),
      `the dispatch site stays a reference even though rename leaves it alone: ${JSON.stringify(lines)}`,
    );
  });
});
