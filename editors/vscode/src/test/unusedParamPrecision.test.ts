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

// W214 (unused parameter) range precision: each unused parameter must be
// squiggled on its own *name*, aligned with go-to-definition/rename, instead
// of stacking N squiggles on the whole proc definition.
//
// Fixture line map (0-based):
//   3  proc f {unused x} { … }   → W214 on `unused` (cols 8..14); `x` is used
//   6  proc g {aa bb} { … }       → W214 on `aa` (8..10) and `bb` (11..13)
suite("Unused-parameter range precision (W214)", () => {
  const docUri = getDocUri("unusedParamPrecision.tcl");

  async function w214Diagnostics(): Promise<vscode.Diagnostic[]> {
    await activate(docUri);
    return waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W214"),
    });
  }

  test("a single unused param is squiggled on its name only", async () => {
    const diags = await w214Diagnostics();
    const onF = withCode(diags, "W214").filter((d) => d.range.start.line === 3);
    assert.strictEqual(onF.length, 1, `expected one W214 on line 3; got ${JSON.stringify(onF)}`);
    assert.strictEqual(onF[0].range.start.character, 8, "must start at `unused`");
    assert.strictEqual(onF[0].range.end.character, 14, "must end after `unused`");
  });

  test("two unused params get two separate tight ranges", async () => {
    const diags = await w214Diagnostics();
    const onG = withCode(diags, "W214").filter((d) => d.range.start.line === 6);
    const ranges = onG.map((d) => [d.range.start.character, d.range.end.character]);
    assert.ok(
      ranges.some(([s, e]) => s === 8 && e === 10),
      `expected an \`aa\` range 8..10; got ${JSON.stringify(ranges)}`,
    );
    assert.ok(
      ranges.some(([s, e]) => s === 11 && e === 13),
      `expected a \`bb\` range 11..13; got ${JSON.stringify(ranges)}`,
    );
  });
});
