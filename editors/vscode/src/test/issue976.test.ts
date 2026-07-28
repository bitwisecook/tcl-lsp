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

// Issue #976: interprocedural call-site literal seeding was unsound across
// dynamic dispatch — `set cmd helper; $cmd dev` reached a proc the scan had
// already seeded from its literal call sites. The deep TP/FP/TN/FN coverage
// lives in `call_site_scan.rs` / `compilation_unit.rs` unit tests and the
// native `e2e` suite; these prove the same behaviour arrives through a real
// VS Code session.

import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate, waitForDiagnostics } from "./helper";

function codeOf(d: vscode.Diagnostic): string {
  return typeof d.code === "object" && d.code !== null ? String(d.code.value) : String(d.code);
}

suite("Issue #976 dynamic-dispatch call-site seeding", () => {
  test("dispatch passing a differing literal draws no I230", async () => {
    const docUri = getDocUri("issue976DynamicDispatch.tcl");
    await activate(docUri);

    const diags = vscode.languages.getDiagnostics(docUri);
    assert.ok(
      !diags.some((d) => codeOf(d) === "I230"),
      `'$cmd dev' also reaches helper, with a differing literal: ${JSON.stringify(diags.map((d) => codeOf(d)))}`,
    );
  });

  test("dispatch whose target cannot be enumerated draws no I230", async () => {
    const docUri = getDocUri("issue976UnenumerableDispatch.tcl");
    await activate(docUri);

    const diags = vscode.languages.getDiagnostics(docUri);
    assert.ok(
      !diags.some((d) => codeOf(d) === "I230"),
      `an unenumerable dispatch may reach helper: ${JSON.stringify(diags.map((d) => codeOf(d)))}`,
    );
  });

  test("TP control: dispatch agreeing on the literal still fires I230", async () => {
    const docUri = getDocUri("issue976DynamicDispatchAgrees.tcl");
    await activate(docUri);

    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "I230"),
    });
    assert.ok(
      diags.some((d) => codeOf(d) === "I230"),
      `every caller — dispatched or not — passes "prod": ${JSON.stringify(diags.map((d) => codeOf(d)))}`,
    );
  });
});
