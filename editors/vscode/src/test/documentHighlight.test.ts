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

suite("Document Highlight", () => {
  const docUri = getDocUri("procs.tcl");

  test("highlights a proc name at its definition", async () => {
    await activate(docUri);

    // `fib` declaration at line 1 col 6.
    const position = new vscode.Position(1, 6);
    const highlights = (await vscode.commands.executeCommand(
      "vscode.executeDocumentHighlights",
      docUri,
      position,
    )) as vscode.DocumentHighlight[] | undefined;

    assert.ok(highlights, "documentHighlight result should not be null");
    assert.ok(
      highlights.length >= 2,
      `Expected at least 2 highlights (decl + recursive calls), got ${highlights.length}`,
    );
  });

  test("highlight ranges include the definition", async () => {
    await activate(docUri);
    const position = new vscode.Position(1, 6);
    const highlights = (await vscode.commands.executeCommand(
      "vscode.executeDocumentHighlights",
      docUri,
      position,
    )) as vscode.DocumentHighlight[] | undefined;

    assert.ok(highlights);
    const declLine = highlights.some((h) => h.range.start.line === 1);
    assert.ok(declLine, "Expected at least one highlight on the declaration line");
  });
});
