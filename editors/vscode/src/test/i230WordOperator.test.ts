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

// Fixture (`i230WordOperator.irul`):
//   line 2: set x "abcdef"
//   line 3: if {$x contains "cd"} { HTTP::respond 200 }
//
// Issue #1048: the document's dialect never reached the compiler's expression
// parser, so an iRules word operator lowered to an opaque expression the
// constant folder could not evaluate — the always-true condition drew no
// I230 in the editor at all.

function codeOf(d: vscode.Diagnostic): string | number | undefined {
  return typeof d.code === "object" ? d.code?.value : d.code;
}

suite("I230 on an iRules word-operator condition", () => {
  const docUri = getDocUri("i230WordOperator.irul");

  test("reports the always-true `contains` condition", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, {
      predicate: (diags) => diags.some((d) => codeOf(d) === "I230"),
    });
    const i230 = diagnostics.filter((d) => codeOf(d) === "I230");
    assert.ok(
      i230.length >= 1,
      `expected an I230, got ${JSON.stringify(diagnostics.map((d) => [codeOf(d), d.message]))}`,
    );
    const onCondition = i230.find((d) => d.range.start.line === 3);
    assert.ok(
      onCondition,
      `expected the I230 on the condition line, got ${JSON.stringify(i230.map((d) => d.range))}`,
    );
    assert.ok(
      onCondition!.message.includes("contains"),
      `message must quote the folded condition: ${onCondition!.message}`,
    );
  });
});
