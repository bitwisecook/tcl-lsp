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

// End-to-end coverage that an ``upvar`` alias is tracked as its own local
// symbol over the wire: references and rename stay scoped to the alias name
// (``alias_x``) and never bleed into the differently-named caller variable
// (``caller_x``).
//
// This file is about ``upvar`` VARIABLE aliasing ONLY.  The unrelated
// ``interp alias`` COMMAND aliasing (issue #923 audit idx 21 / idx 89) lives
// in ``commandAliasTracking.test.ts``; nothing here covers it.
//
// The fixture aliasTracking.tcl is:
//
//   0: proc fixed {} {
//   1:     upvar 1 caller_x alias_x
//   2:     set alias_x 5
//   3:     incr alias_x
//   4:     return $alias_x
//   5: }
//   6: set caller_x 0
//   7: fixed
suite("Alias Tracking (upvar variable aliases)", () => {
  const docUri = getDocUri("aliasTracking.tcl");

  test("references for an upvar alias stay within the proc", async () => {
    await activate(docUri);

    // Inside ``$alias_x`` on the return line (line 4); the alias name begins at
    // column 12, so column 13 is one char in.
    const position = new vscode.Position(4, 13);

    const locations = (await vscode.commands.executeCommand(
      "vscode.executeReferenceProvider",
      docUri,
      position,
    )) as vscode.Location[];

    assert.ok(locations, "References result should not be null");
    assert.ok(
      locations.length >= 3,
      `Expected the upvar declaration plus its uses, got ${locations.length}`,
    );

    // The caller variable ``caller_x`` is declared on line 6 and has a
    // different name — it must never appear among the alias's references.
    for (const loc of locations) {
      assert.strictEqual(loc.uri.fsPath, docUri.fsPath, "References should be in the same file");
      assert.notStrictEqual(
        loc.range.start.line,
        6,
        "Alias references must not include the caller variable on line 6",
      );
    }
  });

  test("prepareRename on an upvar alias offers the local name", async () => {
    await activate(docUri);

    const position = new vscode.Position(4, 13);

    const result = (await vscode.commands.executeCommand(
      "vscode.prepareRename",
      docUri,
      position,
    )) as vscode.Range | { range: vscode.Range; placeholder: string } | undefined;

    assert.ok(result, "prepareRename should succeed on an upvar alias");
    const placeholder = (result as { placeholder?: string }).placeholder;
    if (placeholder !== undefined) {
      assert.strictEqual(placeholder, "alias_x", "Placeholder should be the local alias name");
    }
  });

  test("rename of an upvar alias does not touch the caller variable", async () => {
    await activate(docUri);

    const position = new vscode.Position(4, 13);

    const edit = (await vscode.commands.executeCommand(
      "vscode.executeDocumentRenameProvider",
      docUri,
      position,
      "renamed",
    )) as vscode.WorkspaceEdit | undefined;

    assert.ok(edit, "Rename should return a workspace edit for an upvar alias");
    const docEdits = edit.entries().find(([uri]) => uri.toString() === docUri.toString());
    assert.ok(docEdits, "Should include edits for the fixture document");

    const [, textEdits] = docEdits;
    assert.ok(textEdits.length >= 3, `Expected the declaration plus uses, got ${textEdits.length}`);
    for (const e of textEdits) {
      assert.notStrictEqual(
        e.range.start.line,
        6,
        "Rename must not edit the caller variable on line 6",
      );
    }
  });
});
