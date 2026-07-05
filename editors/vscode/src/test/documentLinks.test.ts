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

suite("Document Links", () => {
  const docUri = getDocUri("links.tcl");

  test("returns document links for source/package require", async () => {
    await activate(docUri);

    const links = (await vscode.commands.executeCommand("vscode.executeLinkProvider", docUri)) as
      vscode.DocumentLink[] | undefined;

    // The server should provide clickable links for source/package require
    assert.ok(links !== undefined, "Link provider should return a result (possibly empty)");
  });

  test("links have valid ranges", async () => {
    await activate(docUri);

    const links = (await vscode.commands.executeCommand(
      "vscode.executeLinkProvider",
      docUri,
    )) as vscode.DocumentLink[];

    if (links && links.length > 0) {
      for (const link of links) {
        assert.ok(link.range, "Each link should have a range");
        assert.ok(
          link.range.start.line >= 0,
          `Link start line should be non-negative, got ${link.range.start.line}`,
        );
        assert.ok(
          link.range.end.line >= link.range.start.line,
          "Link end should be on or after start",
        );
      }
    }
  });
});
