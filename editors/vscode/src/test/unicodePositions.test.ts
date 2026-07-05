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

// UTF-16 position correctness through the real VS Code client. VS Code positions
// are UTF-16 code-unit offsets; a byte-vs-UTF-16 confusion on the server would
// mis-resolve a request whose column sits past multibyte characters. The token
// of interest (`$myvar`) sits after `café résumé` (two multibyte chars) on its
// line. `document.positionAt(offset)` maps a JS-string (UTF-16) offset to the
// correct Position, so these tests do not hand-count columns.
suite("Unicode position correctness (UTF-16)", () => {
  const docUri = getDocUri("unicodePositions.tcl");

  function useSitePosition(doc: vscode.TextDocument): vscode.Position {
    const text = doc.getText();
    const lineStart = text.indexOf("\n") + 1; // skip the `set myvar 1` line
    const useOffset = text.indexOf("$myvar", lineStart) + 1; // land on the `m`
    return doc.positionAt(useOffset);
  }

  test("go-to-definition resolves through a multibyte prefix", async () => {
    const doc = await activate(docUri);
    const pos = useSitePosition(doc);
    const defs = (await vscode.commands.executeCommand(
      "vscode.executeDefinitionProvider",
      docUri,
      pos,
    )) as (vscode.Location | vscode.LocationLink)[] | undefined;
    assert.ok(defs && defs.length > 0, "definition through a multibyte prefix should resolve");
    const targetLine = (l: vscode.Location | vscode.LocationLink) =>
      "range" in l ? l.range.start.line : l.targetRange.start.line;
    assert.ok(
      defs.some((l) => targetLine(l) === 0),
      "definition should resolve to `set myvar` on line 0",
    );
  });

  test("find-references resolves through a multibyte prefix", async () => {
    const doc = await activate(docUri);
    const pos = useSitePosition(doc);
    const refs = (await vscode.commands.executeCommand(
      "vscode.executeReferenceProvider",
      docUri,
      pos,
    )) as vscode.Location[] | undefined;
    assert.ok(refs, "references result should not be null");
    const lines = new Set(refs.map((r) => r.range.start.line));
    assert.ok(
      lines.has(1) && lines.has(2),
      `expected references on both use lines (1 and 2), got ${[...lines].join(", ")}`,
    );
  });
});
