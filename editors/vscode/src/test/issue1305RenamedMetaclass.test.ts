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

// Issue #1305, editor-integration layer: a `rename`d metaclass command
// manufactures nothing — the class-factory record stayed keyed on the name
// the metaclass was *created* with, so a creation call through the renamed
// command was not recognised as a factory call.
//
// Fixture layout (0-based lines), `issue1305RenamedMetaclass.tcl`:
//   4  oo::class create ::R::M { superclass oo::class }
//   5  rename ::R::M ::R::Mk
//   6  ::R::Mk create ::R::W { method go {} { return went } }
//   8  puts [$w go]   <- go-to-definition on `go` must resolve

import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate } from "./helper";

async function definitionsAt(
  docUri: vscode.Uri,
  position: vscode.Position,
): Promise<vscode.Location[]> {
  const result = (await vscode.commands.executeCommand(
    "vscode.executeDefinitionProvider",
    docUri,
    position,
  )) as (vscode.Location | vscode.LocationLink)[] | undefined;
  return (result ?? []).map((item) =>
    "targetUri" in item
      ? new vscode.Location(item.targetUri, item.targetSelectionRange ?? item.targetRange)
      : item,
  );
}

suite("Issue #1305 renamed metaclass", () => {
  const docUri = getDocUri("issue1305RenamedMetaclass.tcl");

  test("a class made through a renamed metaclass command resolves", async () => {
    await activate(docUri);
    // `puts [$w go]` is line 8; `go` starts at character 10.
    const found = await definitionsAt(docUri, new vscode.Position(8, 10));
    assert.strictEqual(
      found.length,
      1,
      `renamed-metaclass creation must still manufacture a navigable class: ${JSON.stringify(found)}`,
    );
    assert.strictEqual(found[0].range.start.line, 6, "the method's own line");
  });
});
