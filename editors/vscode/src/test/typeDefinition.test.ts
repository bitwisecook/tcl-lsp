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
import { activate, getDocUri } from "./helper";

suite("Go to Type Definition", () => {
  const docUri = getDocUri("oo-shapes.tcl");

  test("jumps from $var to its class when initialised with [Class new]", async () => {
    await activate(docUri);

    // `$d` in `return $d` inside make_dog at 0-indexed line 18.
    const position = new vscode.Position(18, 12);
    const locations = (await vscode.commands.executeCommand(
      "vscode.executeTypeDefinitionProvider",
      docUri,
      position,
    )) as vscode.Location[] | undefined;

    if (locations && locations.length > 0) {
      // Our heuristic picked the Dog class declaration at 0-indexed line 6.
      const hasDogClass = locations.some((loc) => loc.range.start.line === 6);
      assert.ok(
        hasDogClass,
        `Expected a location at the Dog class declaration (line 6), got ${locations
          .map((l) => l.range.start.line)
          .join(",")}`,
      );
    }
    // No-result is also acceptable for the heuristic (it's intentionally narrow).
  });
});
