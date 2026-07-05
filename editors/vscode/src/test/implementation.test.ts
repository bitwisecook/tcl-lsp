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

suite("Go to Implementation", () => {
  const docUri = getDocUri("oo-shapes.tcl");

  test("returns subclass overrides for a base-class method", async () => {
    await activate(docUri);

    // `speak` in `method speak {}` inside Animal (line 2 col 11).
    const position = new vscode.Position(2, 11);
    const locations = (await vscode.commands.executeCommand(
      "vscode.executeImplementationProvider",
      docUri,
      position,
    )) as vscode.Location[] | undefined;

    assert.ok(locations, "implementation result should not be null");
    assert.ok(
      locations.length >= 2,
      `Expected at least 2 implementations (Dog + Cat), got ${locations.length}`,
    );
    const lines = locations.map((l) => l.range.start.line);
    // Dog.speak at line 8, Cat.speak at line 13.
    assert.ok(lines.includes(8) || lines.includes(13));
  });

  test("returns subclass declarations for a class name", async () => {
    await activate(docUri);
    // `Animal` in its declaration at line 1 col 18.
    const position = new vscode.Position(1, 18);
    const locations = (await vscode.commands.executeCommand(
      "vscode.executeImplementationProvider",
      docUri,
      position,
    )) as vscode.Location[] | undefined;
    assert.ok(locations);
    assert.ok(locations.length >= 1, "Expected at least one subclass declaration");
  });
});
