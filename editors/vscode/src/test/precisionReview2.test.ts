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

function codeOf(d: vscode.Diagnostic): string {
  return typeof d.code === "object" && d.code !== null ? String(d.code.value) : String(d.code);
}

function withCode(diags: vscode.Diagnostic[], code: string): vscode.Diagnostic[] {
  return diags.filter((d) => codeOf(d) === code);
}

// Diagnostics deep review 2 — tight ranges for the expression-shimmer
// operand (S100), the destructive-file path argument (W313), the
// string-compare operator (W110), and I230 message fidelity, asserted
// through the real editor client.
//
// Fixture layout (0-based lines):
//   4  set s "hello"
//   5  set y [expr {$s + 1}]     ← S100 tight on $s (cols 13..15)
//   7  set p [gets stdin]
//   8  file delete -force $p     ← W313 tight on $p (cols 19..21)
//  10  if $n { … } else { … }    ← I230 quotes `$n`, not `${n}`
//  12  if {$c == "x"} { … }      ← W110 tight on `==` (cols 7..9)
suite("Diagnostics deep review 2 precision", () => {
  const docUri = getDocUri("precisionReview2.tcl");

  test("S100 range is tight on the expression operand", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "S100"),
    });
    const s100 = withCode(diags, "S100");
    assert.ok(s100.length >= 1, "the String operand in arithmetic shimmers");
    const r = s100[0].range;
    assert.strictEqual(r.start.line, 5);
    assert.strictEqual(r.start.character, 13, "range starts at $s");
    assert.strictEqual(r.end.character, 15, "range covers exactly $s");
  });

  test("W313 range is tight on the dynamic path argument", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W313"),
    });
    const w313 = withCode(diags, "W313");
    assert.strictEqual(w313.length, 1, "one destructive-file path in the fixture");
    const r = w313[0].range;
    assert.strictEqual(r.start.line, 8);
    assert.strictEqual(r.start.character, 19, "range starts at $p (past -force)");
    assert.strictEqual(r.end.character, 21, "range covers exactly $p");
  });

  test("I230 message quotes the bare $n as the source spells it", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "I230"),
    });
    const i230 = withCode(diags, "I230");
    assert.ok(i230.length >= 1, "the constant `if $n` condition draws I230");
    assert.ok(
      i230[0].message.includes("'$n'") && !i230[0].message.includes("${n}"),
      `message must quote \`$n\` verbatim: ${i230[0].message}`,
    );
  });

  test("W110 range is tight on the == operator", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W110"),
    });
    const w110 = withCode(diags, "W110");
    assert.strictEqual(w110.length, 1, "one string-compare `==` in the fixture");
    const r = w110[0].range;
    assert.strictEqual(r.start.line, 12);
    assert.strictEqual(r.start.character, 7, "range starts at ==");
    assert.strictEqual(r.end.character, 9, "range covers exactly ==");
  });
});
