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

// Normalise the `code` field (string | { value }) to a plain string.
function codeOf(d: vscode.Diagnostic): string {
  return typeof d.code === "object" && d.code !== null ? String(d.code.value) : String(d.code);
}

function withCode(diags: vscode.Diagnostic[], code: string): vscode.Diagnostic[] {
  return diags.filter((d) => codeOf(d) === code);
}

function linesWithCode(diags: vscode.Diagnostic[], code: string): number[] {
  return withCode(diags, code).map((d) => d.range.start.line);
}

// Arity-data corrections (verified against tclsh 9.0.4) plus the tight E003
// range and the W241 throw/tailcall loop-exit fix, delivered end-to-end.
//
// Fixture line map (0-based):
//   2  lreverse {a b c} extra1 extra2   → E003, tight on the surplus words
//   3  lmap a b c d                     → E005 (even count is wrong # args)
//   4  global                           → no E002 (valid no-op)
//   5  auto_load foo ::ns               → no E003 (arity is 1..2)
//   6  while 1 {throw MYERR boom}        → no W241 (throw leaves the loop)
//   7  while 1 {incr counter}            → W241 (control: no exit → infinite)
suite("Arity + loop-termination precision", () => {
  const docUri = getDocUri("arityLoopPrecision.tcl");

  test("E003 highlights only the surplus arguments", async () => {
    await activate(docUri);
    // W241 on line 7 is the marker that a full analysis ran.
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W241"),
    });
    const e003 = withCode(diags, "E003");
    assert.strictEqual(e003.length, 1, `expected one E003; got ${JSON.stringify(e003)}`);
    const r = e003[0].range;
    assert.strictEqual(r.start.line, 2, "E003 must be on the lreverse line");
    // Tight: the squiggle starts at the first surplus word (`extra1`, col 17),
    // not at the command name (col 0).
    assert.strictEqual(r.start.character, 17, "E003 must start at the first surplus word");
    assert.strictEqual(r.end.character, 30, "E003 must end at the last surplus word");
  });

  test("lmap even arg count fires E005", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W241"),
    });
    assert.ok(linesWithCode(diags, "E005").includes(3), "lmap a b c d must fire E005 on line 3");
  });

  test("global and auto_load two-arg forms have no arity error", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W241"),
    });
    assert.ok(!linesWithCode(diags, "E002").includes(4), "bare `global` must not fire E002");
    assert.ok(
      !linesWithCode(diags, "E003").includes(5),
      "`auto_load foo ::ns` must not fire E003 (arity 1..2)",
    );
  });

  test("while-true with throw is not flagged infinite (W241)", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W241"),
    });
    const w241Lines = linesWithCode(diags, "W241");
    assert.ok(!w241Lines.includes(6), "throw in the body must suppress W241");
    assert.ok(w241Lines.includes(7), "a body with no exit must still fire W241");
  });
});
