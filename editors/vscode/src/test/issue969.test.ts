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

// Issue #969: "Condition '$count & 1' is always false" — a false-positive
// I230 on a genuinely alternating parity check. The deep TP/FP/TN/FN
// coverage of the interprocedural call-site-literal seed lives in
// `compilation_unit.rs`'s unit tests and the native `e2e` suite
// (`diagnostics::namespaced_recursive_proc_parity_check_does_not_fire_i230`
// and friends); these prove the same behaviour arrives through a real
// VS Code session, diagnostics included.

import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate, waitForDiagnostics } from "./helper";

function codeOf(d: vscode.Diagnostic): string {
  return typeof d.code === "object" && d.code !== null ? String(d.code.value) : String(d.code);
}

suite("Issue #969 I230 false positive", () => {
  test("namespaced recursive proc's parity check draws no I230", async () => {
    const docUri = getDocUri("issue969RecursiveNamespace.tcl");
    await activate(docUri);

    const diags = vscode.languages.getDiagnostics(docUri);
    assert.ok(
      !diags.some((d) => codeOf(d) === "I230"),
      `recursive parity check on 'count' must not fold: ${JSON.stringify(diags.map((d) => codeOf(d)))}`,
    );
  });

  test("call site hidden inside a catch body draws no I230", async () => {
    const docUri = getDocUri("issue969CatchHiddenCall.tcl");
    await activate(docUri);

    const diags = vscode.languages.getDiagnostics(docUri);
    assert.ok(
      !diags.some((d) => codeOf(d) === "I230"),
      `is_even is called with both 3 and 4 (the latter inside catch): ${JSON.stringify(diags.map((d) => codeOf(d)))}`,
    );
  });

  test("TP control: two callers passing a uniform literal still fire I230", async () => {
    const docUri = getDocUri("issue969UniformLiteralControl.tcl");
    await activate(docUri);

    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "I230"),
    });
    assert.ok(
      diags.some((d) => codeOf(d) === "I230"),
      `two callers passing the identical literal should still fold: ${JSON.stringify(diags.map((d) => codeOf(d)))}`,
    );
  });
});
