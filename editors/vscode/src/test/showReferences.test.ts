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
import { activate, getDocUri, scaledTimeout } from "./helper";
import { convertShowReferencesArgs } from "../showReferences";

suite("tcl-lsp.showReferences adapter", () => {
  suite("convertShowReferencesArgs", () => {
    test("parses the URI string into a vscode.Uri", () => {
      const { uri } = convertShowReferencesArgs(
        "file:///tmp/example.tcl",
        { line: 0, character: 0 },
        [],
      );
      assert.ok(uri instanceof vscode.Uri, "uri should be a vscode.Uri instance");
      assert.strictEqual(uri.scheme, "file");
      assert.strictEqual(uri.fsPath.replace(/\\/g, "/"), "/tmp/example.tcl");
    });

    test("rebuilds the position as a vscode.Position", () => {
      const { position } = convertShowReferencesArgs(
        "file:///tmp/example.tcl",
        { line: 12, character: 7 },
        [],
      );
      assert.ok(position instanceof vscode.Position, "position should be a vscode.Position");
      assert.strictEqual(position.line, 12);
      assert.strictEqual(position.character, 7);
    });

    test("maps each JSON location into a vscode.Location with a vscode.Range", () => {
      const { locations } = convertShowReferencesArgs(
        "file:///tmp/a.tcl",
        { line: 0, character: 0 },
        [
          {
            uri: "file:///tmp/a.tcl",
            range: {
              start: { line: 1, character: 2 },
              end: { line: 1, character: 10 },
            },
          },
          {
            uri: "file:///tmp/b.tcl",
            range: {
              start: { line: 5, character: 0 },
              end: { line: 5, character: 4 },
            },
          },
        ],
      );
      assert.strictEqual(locations.length, 2);
      for (const loc of locations) {
        assert.ok(loc instanceof vscode.Location, "each location should be a vscode.Location");
        assert.ok(loc.uri instanceof vscode.Uri, "location.uri should be a vscode.Uri");
        assert.ok(loc.range instanceof vscode.Range, "location.range should be a vscode.Range");
      }
      assert.strictEqual(locations[0].uri.fsPath.replace(/\\/g, "/"), "/tmp/a.tcl");
      assert.strictEqual(locations[0].range.start.line, 1);
      assert.strictEqual(locations[0].range.start.character, 2);
      assert.strictEqual(locations[0].range.end.line, 1);
      assert.strictEqual(locations[0].range.end.character, 10);
      assert.strictEqual(locations[1].uri.fsPath.replace(/\\/g, "/"), "/tmp/b.tcl");
    });

    test("returns an empty list when no locations are supplied", () => {
      const { locations } = convertShowReferencesArgs(
        "file:///tmp/example.tcl",
        { line: 0, character: 0 },
        [],
      );
      assert.deepStrictEqual(locations, []);
    });
  });

  suite("command registration", () => {
    suiteSetup(async function () {
      this.timeout(scaledTimeout(60_000));
      await activate(getDocUri("simple.tcl"));
    });

    test("tcl-lsp.showReferences is registered", async () => {
      const registered = await vscode.commands.getCommands(true);
      assert.ok(
        registered.includes("tcl-lsp.showReferences"),
        "tcl-lsp.showReferences should be registered by the extension",
      );
    });
  });
});
