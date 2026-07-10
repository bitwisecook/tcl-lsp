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
import { getDocUri, activate, waitForDiagnostics } from "./helper";

// End-to-end regression coverage for the "preview version:" tickets that are
// analyser / delivery bugs surfaced in VS Code:
//
//   #720  `after 200 {...}` must not be flagged as an unknown subcommand (W001)
//   #721  a single diagnostic must not be displayed twice (E003 here)
//   #723  Tk commands behind an unknown `package require` must not draw W120
//   #725  `$::var` (a qualified global read) must not draw "read before set" (W210)
//   #726  a nested `[expr]` that is a command argument must not draw W114
//   #727  go-to-definition of a method parameter resolves to the parameter name
//   #867  `lassign` targets are list elements, not the List it returns -> no S100
suite("Preview-version regression tickets", () => {
  const docUri = getDocUri("previewTickets.tcl");

  function codeOf(d: vscode.Diagnostic): string | number | undefined {
    return typeof d.code === "object" ? d.code.value : d.code;
  }

  test("the false-positive diagnostics never fire and E003 appears exactly once", async () => {
    await activate(docUri);
    // The fixture's `set var 10 10` guarantees at least one diagnostic (E003),
    // which also proves the analyser actually ran over the document.
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 1 });
    const codes = diagnostics.map(codeOf);

    // #720 — `after 200 {...}` is a delay, not an unknown subcommand.
    assert.ok(!codes.includes("W001"), `#720: unexpected W001 in [${codes}]`);
    // #725 — `$::myVar` is an explicit global read, never read-before-set.
    assert.ok(!codes.includes("W210"), `#725: unexpected W210 in [${codes}]`);
    // #723 — an unknown package may load Tk; the Tk commands must not draw W120.
    assert.ok(!codes.includes("W120"), `#723: unexpected W120 in [${codes}]`);
    // #726 — the nested `[expr]` is a command argument, not an expression
    // context, so it must not draw W114.
    assert.ok(!codes.includes("W114"), `#726: unexpected W114 in [${codes}]`);
    // #867 — `lassign $point px py pz` writes list elements, not the List the
    // command returns, so the arithmetic on them must not draw an S100 shimmer.
    assert.ok(!codes.includes("S100"), `#867: unexpected S100 in [${codes}]`);

    // #721 — the genuine E003 must be present, and exactly once (no duplicate
    // from the server pushing *and* the client pulling the same diagnostic).
    const e003 = diagnostics.filter((d) => codeOf(d) === "E003");
    assert.strictEqual(
      e003.length,
      1,
      `#721: expected exactly one E003, got ${e003.length}: ${e003
        .map((d) => `${d.range.start.line}:${d.message}`)
        .join("; ")}`,
    );
  });

  test("#727: go-to-definition of a method parameter resolves to its name", async () => {
    const methodUri = getDocUri("methodParam.tcl");
    await activate(methodUri);
    // `return "$greeting, $name"` is on line 4 (0-based); `$name` ends the
    // string. Put the cursor inside the `$name` usage.
    const doc = await vscode.workspace.openTextDocument(methodUri);
    const line = doc.lineAt(4).text;
    const nameIdx = line.lastIndexOf("name");
    const pos = new vscode.Position(4, nameIdx + 1);
    const locations = (await vscode.commands.executeCommand(
      "vscode.executeDefinitionProvider",
      methodUri,
      pos,
    )) as vscode.Location[];
    assert.ok(locations && locations.length >= 1, "expected a definition location");
    const target = locations[0].range;
    // The declaration is the parameter `name` on line 2 (`method greet {name
    // greeting} {`), a single-line, name-sized range — NOT the multi-line body.
    assert.strictEqual(
      target.start.line,
      2,
      `#727: param def should be on the declaration line 2, got ${target.start.line}`,
    );
    assert.strictEqual(
      target.start.line,
      target.end.line,
      "#727: param def must be a single-line name span, not the body",
    );
    assert.ok(
      target.end.character - target.start.character <= "greeting".length,
      "#727: param def must be a name-sized span",
    );
  });
});
