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

// End-to-end coverage of `interp alias` COMMAND aliasing over the wire
// (issue #923 audit idx 21 and idx 89).
//
// This is the command-table half of aliasing.  `aliasTracking.test.ts` covers
// the unrelated `upvar` VARIABLE aliasing and provides no coverage here.
//
// The fixture commandAliasTracking.tcl is pinned to real C Tcl — running it
// under tclsh 9.0.4 and 8.6.16 prints, byte-identically:
//
//   hi
//   hi
//   classic tk::spinbox: .sb
//
// so (a) both `[sayHi]` calls really execute `greet`'s body, and (b) the
// `::ttk::spinbox` proc on line 17 never runs — the alias on line 23 took its
// name over for good.
//
//    8: proc greet {} {
//   11: interp alias {} sayHi {} greet
//   12: puts [sayHi]
//   13: puts [sayHi]
//   17: proc ::ttk::spinbox {w args} {
//   20: proc ::tk::spinbox {w args} {
//   23: interp alias {} ::ttk::spinbox {} ::tk::spinbox
//   24: ::ttk::spinbox .sb

const docUri = getDocUri("commandAliasTracking.tcl");

/** Start lines of *locations*, sorted and deduplicated. */
function startLines(locations: vscode.Location[]): number[] {
  return [...new Set(locations.map((l) => l.range.start.line))].sort((a, b) => a - b);
}

/** Start lines of the edits *edit* carries for the fixture document. */
function editLines(edit: vscode.WorkspaceEdit | undefined): number[] {
  assert.ok(edit, "Expected a workspace edit");
  const entry = edit.entries().find(([uri]) => uri.toString() === docUri.toString());
  assert.ok(entry, "Expected edits for the fixture document");
  return [...new Set(entry[1].map((e) => e.range.start.line))].sort((a, b) => a - b);
}

suite("Command Alias Tracking (interp alias)", () => {
  test("references from an alias target include the call sites spelled through it", async () => {
    await activate(docUri);

    // Line 8, column 6 — the `greet` declaration.
    const locations = (await vscode.commands.executeCommand(
      "vscode.executeReferenceProvider",
      docUri,
      new vscode.Position(8, 6),
    )) as vscode.Location[];

    const lines = startLines(locations);
    assert.ok(lines.includes(8), `The declaration must be reported: ${lines}`);
    assert.ok(
      lines.includes(12) && lines.includes(13),
      `Both [sayHi] calls really run greet's body and must be reported: ${lines}`,
    );
  });

  test("references from an alias call site resolve through to the target", async () => {
    await activate(docUri);

    // Line 12, column 6 — inside the `sayHi` call.
    const locations = (await vscode.commands.executeCommand(
      "vscode.executeReferenceProvider",
      docUri,
      new vscode.Position(12, 6),
    )) as vscode.Location[];

    const lines = startLines(locations);
    assert.ok(lines.includes(8), `The target's declaration must be reported: ${lines}`);
    assert.ok(
      lines.includes(12) && lines.includes(13),
      `Both alias call sites must be reported: ${lines}`,
    );
  });

  test("rename of an alias target leaves the alias's own call sites alone", async () => {
    await activate(docUri);

    const edit = (await vscode.commands.executeCommand(
      "vscode.executeDocumentRenameProvider",
      docUri,
      new vscode.Position(8, 6),
      "salute",
    )) as vscode.WorkspaceEdit | undefined;

    const lines = editLines(edit);
    assert.ok(lines.includes(8), `The declaration must be rewritten: ${lines}`);
    assert.ok(
      lines.includes(11),
      `The alias's own TARGET word names greet and must be rewritten: ${lines}`,
    );
    assert.ok(
      !lines.includes(12) && !lines.includes(13),
      `A [sayHi] call names the ALIAS, which keeps its spelling: ${lines}`,
    );
  });

  test("go-to-definition on a shadowed call reaches the alias target, not the displaced proc", async () => {
    await activate(docUri);

    // Line 24, column 2 — the `::ttk::spinbox .sb` call, which really runs
    // `::tk::spinbox`'s body.
    const locations = (await vscode.commands.executeCommand(
      "vscode.executeDefinitionProvider",
      docUri,
      new vscode.Position(24, 2),
    )) as (vscode.Location | vscode.LocationLink)[];

    const lines = [
      ...new Set(
        locations.map((l) =>
          "range" in l ? l.range.start.line : (l as vscode.LocationLink).targetRange.start.line,
        ),
      ),
    ];
    assert.deepStrictEqual(
      lines,
      [20],
      `The call reaches ::tk::spinbox's declaration on line 20, got ${lines}`,
    );
  });

  test("references from a proc an alias shadowed exclude the redirected call", async () => {
    await activate(docUri);

    // Line 17, column 12 — the displaced `::ttk::spinbox` declaration.
    const locations = (await vscode.commands.executeCommand(
      "vscode.executeReferenceProvider",
      docUri,
      new vscode.Position(17, 12),
    )) as vscode.Location[];

    const lines = startLines(locations);
    assert.ok(
      !lines.includes(24),
      `Line 24 runs ::tk::spinbox's body, so it is not a reference to the ` +
        `displaced proc: ${lines}`,
    );
  });

  test("rename of a proc an alias shadowed does not rewrite the redirected call", async () => {
    await activate(docUri);

    const edit = (await vscode.commands.executeCommand(
      "vscode.executeDocumentRenameProvider",
      docUri,
      new vscode.Position(17, 12),
      "newname",
    )) as vscode.WorkspaceEdit | undefined;

    const lines = editLines(edit);
    assert.deepStrictEqual(
      lines,
      [17],
      `Rewriting line 24 too would change "classic tk::spinbox: .sb" into ` +
        `"themed ttk::spinbox: .sb" — a rename that silently changes which ` +
        `body runs. Got: ${lines}`,
    );
  });

  test("references from the alias target still reach the redirected call", async () => {
    await activate(docUri);

    // Line 20, column 12 — the `::tk::spinbox` declaration, which the alias
    // on line 23 redirects the line-24 call onto.
    const locations = (await vscode.commands.executeCommand(
      "vscode.executeReferenceProvider",
      docUri,
      new vscode.Position(20, 12),
    )) as vscode.Location[];

    const lines = startLines(locations);
    assert.ok(lines.includes(20), `The declaration must be reported: ${lines}`);
    assert.ok(lines.includes(23), `The alias's target word must be reported: ${lines}`);
    assert.ok(lines.includes(24), `The redirected call site must be reported: ${lines}`);
  });
});
