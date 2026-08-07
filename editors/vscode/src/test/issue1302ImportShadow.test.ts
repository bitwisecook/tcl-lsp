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

// Issue #1302, through the editor: a `namespace import` cannot bind a name a
// fresh interpreter already holds, so a bare call to such a name is not a
// reference to the exported proc.
//
// The unit matrix lives in `tcl_lsp_core::workspace_index` and the protocol
// matrix in `tests/e2e/issue1302_import_builtin_shadow.rs`. This layer exists
// because the defect's user-visible face is find-references and the reference
// count code lens, and neither of the other two exercises the editor wiring —
// provider registration, the positions VS Code sends, the shapes it expects
// back.
//
// The fixture pair is pinned to real C Tcl. Sourcing the lib and running the
// import under tclsh 9.0.4 and 8.6.14 prints, byte-identically:
//
//   import: can't import command "set": already exists
//   origin ::set     = ::set
//   ::mything exists = 1
//
// Note the third line: the import *errors* on `set` and still binds
// `mything`. Both halves matter here — one call is not a reference, the other
// is, from the same import statement.
//
// issue1302ImportShadowLib.tcl:
//   13:     proc set {name value} {
//   16:     proc mything {} {
// issue1302ImportShadow.tcl:
//   11: namespace import ::Shadow::*
//   12: set counter 1
//   13: mything

const docUri = getDocUri("issue1302ImportShadow.tcl");
const libUri = getDocUri("issue1302ImportShadowLib.tcl");

/** Start lines of *locations* that land in `uri`, sorted and deduplicated. */
function linesIn(locations: vscode.Location[], uri: vscode.Uri): number[] {
  return [
    ...new Set(
      locations.filter((l) => l.uri.toString() === uri.toString()).map((l) => l.range.start.line),
    ),
  ].sort((a, b) => a - b);
}

async function referencesAt(uri: vscode.Uri, line: number, ch: number) {
  return (await vscode.commands.executeCommand(
    "vscode.executeReferenceProvider",
    uri,
    new vscode.Position(line, ch),
  )) as vscode.Location[];
}

suite("Issue #1302: a wildcard import cannot shadow a registry builtin", () => {
  test("the bare `set` call is not reported as a reference to ::Shadow::set", async () => {
    await activate(libUri);
    await activate(docUri);

    // Line 13, column 10 — the `set` of `proc set` in the library.
    const locations = await referencesAt(libUri, 13, 10);

    assert.deepStrictEqual(
      linesIn(locations, docUri),
      [],
      'real Tcl raises `can\'t import command "set": already exists` and installs ' +
        "nothing, so `set counter 1` reaches the builtin and must not be filed as a " +
        `reference: ${JSON.stringify(locations.map((l) => [l.uri.path, l.range.start.line]))}`,
    );
  });

  test("the bare `mything` call, bound by the same import, is a reference", async () => {
    await activate(libUri);
    await activate(docUri);

    // Line 16, column 10 — the `mything` of `proc mything` in the library.
    const locations = await referencesAt(libUri, 16, 10);

    assert.deepStrictEqual(
      linesIn(locations, docUri),
      [13],
      "`mything` is not a command any fresh interpreter holds, so the same import " +
        "really does bind it (`info commands ::mything` is non-empty in the oracle) " +
        `and the call is a genuine reference: ${JSON.stringify(
          locations.map((l) => [l.uri.path, l.range.start.line]),
        )}`,
    );
  });
});
