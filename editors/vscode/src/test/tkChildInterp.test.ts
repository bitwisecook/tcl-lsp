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

// Tk widget checks are keyed per interpreter (issue #923 audit idx 91), end
// to end in the editor.
//
// `interp create child` gives the child its own command table and its own
// widget hierarchy — verified on tclsh 9.0.4 and 8.6.16: a command created
// inside `child eval { … }` is absent from the parent's `info commands`, and
// vice versa.  So a `.top` in the child and a `.top` in the parent are two
// unrelated widgets.
//
// tkChildInterp.tcl (cross-interpreter):
//   19: child eval { frame .top; pack .top.a }
//   20: frame .top
//   21: grid .top.b            → NOT a geometry conflict, no TK1001
//   22: frame .childOnly.deep  → parent never created, TK1002 fires
//
// tkChildInterpControl.tcl (single interpreter — the checks are narrowed,
// not disabled):
//   12: frame .top
//   13: pack .top.a
//   14: grid .top.b            → TK1001 fires
//   15: frame .missing.child   → TK1002 fires

/** The `code` string of a diagnostic, whatever shape VS Code hands us. */
function codeOf(d: vscode.Diagnostic): string | undefined {
  const code = typeof d.code === "object" && d.code !== null ? d.code.value : d.code;
  return code === undefined ? undefined : String(code);
}

/** Start lines of the diagnostics in *diags* carrying *code*. */
function linesWithCode(diags: vscode.Diagnostic[], code: string): number[] {
  return diags.filter((d) => codeOf(d) === code).map((d) => d.range.start.line);
}

suite("Tk checks across child interpreters (TK1001 / TK1002)", () => {
  const docUri = getDocUri("tkChildInterp.tcl");
  const controlUri = getDocUri("tkChildInterpControl.tcl");

  test("no TK1001 when the two geometry managers are in different interpreters", async () => {
    await activate(docUri);
    // The TK1002 on line 22 is the marker that the Tk checks ran at all.
    const diagnostics = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "TK1002"),
    });
    assert.deepStrictEqual(
      linesWithCode(diagnostics, "TK1001"),
      [],
      `The child's pack and the parent's grid manage two different '.top' ` +
        `widgets: ${diagnostics.map((d) => `${codeOf(d)}@${d.range.start.line}`).join(", ")}`,
    );
  });

  test("TK1002 fires for a parent-interpreter widget whose parent was never created", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "TK1002"),
    });
    assert.ok(
      linesWithCode(diagnostics, "TK1002").includes(22),
      `'.childOnly.deep' has no parent: ${linesWithCode(diagnostics, "TK1002")}`,
    );
    assert.ok(
      diagnostics.some((d) => codeOf(d) === "TK1002" && d.message.includes(".childOnly")),
      `The message must name the missing parent: ${diagnostics
        .map((d) => d.message)
        .join("; ")}`,
    );
  });

  test("both checks still fire inside a single interpreter", async () => {
    await activate(controlUri);
    const diagnostics = await waitForDiagnostics(controlUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "TK1001"),
    });
    assert.ok(
      linesWithCode(diagnostics, "TK1001").length > 0,
      `Mixing pack and grid in one interpreter is still a conflict: ${diagnostics
        .map((d) => `${codeOf(d)}@${d.range.start.line}`)
        .join(", ")}`,
    );
    assert.ok(
      linesWithCode(diagnostics, "TK1002").includes(15),
      `'.missing.child' still has no parent: ${linesWithCode(diagnostics, "TK1002")}`,
    );
  });
});
