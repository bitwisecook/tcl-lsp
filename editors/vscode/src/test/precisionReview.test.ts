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

// Diagnostics precision review — tight ranges, rename resolution, and
// did-you-mean suggestions, asserted through the real editor client so a
// projection bug (byte→UTF-16, span rebasing) cannot hide behind the
// server-side e2e suite.
//
// Fixture layout (0-based lines):
//   3  proc user_args {args} { … }
//   4  rename user_args ua2
//   5  ua2 1 2                  ← known (rename target), no W123
//   6  user_args 9              ← W128 (vacated name)
//   7  set x [lindex $missing 0] ← W210 tight on $missing (cols 14..22)
//   8  puts $x
//   9  proc greet {name} { … }
//  10  greet a b                ← E003 tight on surplus `b` (cols 8..9)
//  11..13  oo::class Animal { method speak }
//  14  set pet [Animal new]
//  15  $pet spek                ← W308 tight on `spek` (cols 5..9)
suite("Diagnostics precision review", () => {
  const docUri = getDocUri("precisionReview.tcl");

  test("rename target is known; vacated name draws W128", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W128"),
    });
    assert.strictEqual(
      withCode(diags, "W123").length,
      0,
      "the rename target `ua2` must not be an unknown command",
    );
    const w128 = withCode(diags, "W128");
    assert.strictEqual(w128.length, 1, "the vacated `user_args` draws exactly one W128");
    assert.strictEqual(w128[0].range.start.line, 6);
  });

  test("W210 range is tight on the nested $missing read", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W210"),
    });
    const w210 = withCode(diags, "W210");
    assert.strictEqual(w210.length, 1, "one read-before-set in the fixture");
    const r = w210[0].range;
    assert.strictEqual(r.start.line, 7);
    assert.strictEqual(r.start.character, 14, "range starts at $missing");
    assert.strictEqual(r.end.character, 22, "range ends after $missing");
  });

  test("E003 range covers only the surplus argument", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "E003"),
    });
    const e003 = withCode(diags, "E003");
    assert.strictEqual(e003.length, 1, "one surplus-argument call in the fixture");
    const r = e003[0].range;
    assert.strictEqual(r.start.line, 10);
    assert.strictEqual(r.start.character, 8, "range starts at the surplus `b`");
    assert.strictEqual(r.end.character, 9, "range covers exactly `b`");
  });

  test("W308 anchors the method word and suggests the MRO method", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W308"),
    });
    const w308 = withCode(diags, "W308");
    assert.strictEqual(w308.length, 1, "one unknown-method dispatch in the fixture");
    const r = w308[0].range;
    assert.strictEqual(r.start.line, 15);
    assert.strictEqual(r.start.character, 5, "range starts at `spek`");
    assert.strictEqual(r.end.character, 9, "range covers exactly `spek`");
    assert.ok(
      w308[0].message.includes("did you mean 'speak'"),
      `suggestion drawn from the class's methods: ${w308[0].message}`,
    );
  });

  test("quick fix replaces exactly the typo'd method word", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W308"),
    });
    const w308 = withCode(diags, "W308")[0];
    const actions = await vscode.commands.executeCommand<vscode.CodeAction[]>(
      "vscode.executeCodeActionProvider",
      docUri,
      w308.range,
    );
    const fix = (actions ?? []).find((a) => a.title.includes("Replace with 'speak'"));
    assert.ok(
      fix,
      `expected a Replace with 'speak' quick fix, got: ${(actions ?? []).map((a) => a.title).join(", ")}`,
    );
  });
});
