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

// Issue #977: the interprocedural call-site seed was unsound for a plain
// library file — no `package provide`, so PR #970's guard never fired —
// `source`d by another file that calls its procs with a different literal.
// The deep TP/FP/TN/FN coverage lives in `tcl-compiler`'s `unit_scope` /
// `call_site_param_constants` unit tests, `tcl-lsp-db`'s
// `cross_file_call_sites` module, and the native `e2e` suite; these prove the
// same behaviour arrives through a real VS Code session, where the workspace
// (not a single buffer) is the compilation scope.

import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate, waitForDiagnostics } from "./helper";

function codeOf(d: vscode.Diagnostic): string {
  return typeof d.code === "object" && d.code !== null ? String(d.code.value) : String(d.code);
}

suite("Issue #977 cross-file interprocedural seeding", () => {
  test("a caller in a sourcing file with a differing literal clears I230", async () => {
    // Open the caller first so it is already in the project when the library
    // is analysed — the ordering a workspace scan produces.
    await activate(getDocUri("issue977Main.tcl"));
    const libUri = getDocUri("issue977Lib.tcl");
    await activate(libUri);

    const diags = vscode.languages.getDiagnostics(libUri);
    assert.ok(
      !diags.some((d) => codeOf(d) === "I230"),
      `issue977Main.tcl calls helper with "dev"; the library must not fold on "prod": ${JSON.stringify(
        diags.map((d) => codeOf(d)),
      )}`,
    );
  });

  test("TP control: every project caller agreeing on the literal still fires I230", async () => {
    await activate(getDocUri("issue977AgreeingMain.tcl"));
    const libUri = getDocUri("issue977AgreeingLib.tcl");
    await activate(libUri);

    const diags = await waitForDiagnostics(libUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "I230"),
    });
    assert.ok(
      diags.some((d) => codeOf(d) === "I230"),
      `every caller in the project passes "prod", so the seed is sound: ${JSON.stringify(
        diags.map((d) => codeOf(d)),
      )}`,
    );
  });
});
