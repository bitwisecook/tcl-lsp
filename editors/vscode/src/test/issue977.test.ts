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

// A condition over a procedure parameter is never folded from the file's
// calls, whether or not a `source`ing file passes a different literal.
// The deep TP/FP/TN/FN coverage lives in `tcl-compiler`'s `unit_scope` /
// `call_site_param_constants` unit tests, `tcl-lsp-db`'s
// `cross_file_call_sites` module, and the native `e2e` suite; these prove the
// same behaviour arrives through a real VS Code session, where the workspace
// (not a single buffer) is the compilation scope.

import * as assert from "assert";
import * as vscode from "vscode";
import {
  getDocUri,
  activate,
  waitForCurrentDiagnostics,
  waitForCurrentDiagnosticsSnapshot,
} from "./helper";

function codeOf(d: vscode.Diagnostic): string {
  return typeof d.code === "object" && d.code !== null ? String(d.code.value) : String(d.code);
}

suite("Issue #977 cross-file interprocedural seeding", () => {
  test("a caller in a sourcing file with a differing literal clears I230", async () => {
    // Open the caller first so it is already in the project when the library
    // is analysed — the ordering a workspace scan produces.
    const callerUri = getDocUri("issue977Main.tcl");
    const caller = await activate(callerUri);
    await waitForCurrentDiagnostics(callerUri, {
      expectedSource: caller.getText(),
    });
    const libUri = getDocUri("issue977Lib.tcl");
    const document = await activate(libUri);

    const currentSnapshotFor_diags = await waitForCurrentDiagnosticsSnapshot(libUri, {
      expectedSource: document.getText(),
    });
    if (!currentSnapshotFor_diags.isCurrent()) {
      throw new Error("The retained diagnostics report became stale before its assertions");
    }
    const diags = [...currentSnapshotFor_diags.report.diagnostics];
    assert.ok(
      !diags.some((d) => codeOf(d) === "I230"),
      `issue977Main.tcl calls helper with "dev"; the library must not fold on "prod": ${JSON.stringify(
        diags.map((d) => codeOf(d)),
      )}`,
    );
  });

  test("every project caller agreeing on the literal still leaves the condition open", async () => {
    // A body is analysed from a generic entry: the procedure stays callable
    // from a later `source`, a callback or an interactive session. tclsh 8.4 to
    // 9.1: after sourcing, `list [helper prod] [helper dev]` is `1 2`.
    await activate(getDocUri("issue977AgreeingMain.tcl"));
    const libUri = getDocUri("issue977AgreeingLib.tcl");
    const document = await activate(libUri);

    const currentSnapshotFor_diags = await waitForCurrentDiagnosticsSnapshot(libUri, {
      expectedSource: document.getText(),
    });
    if (!currentSnapshotFor_diags.isCurrent()) {
      throw new Error("The retained diagnostics report became stale before its assertions");
    }
    const diags = [...currentSnapshotFor_diags.report.diagnostics];
    assert.ok(
      !diags.some((d) => codeOf(d) === "I230"),
      `a condition over a parameter is never constant: ${JSON.stringify(
        diags.map((d) => codeOf(d)),
      )}`,
    );
  });
});
