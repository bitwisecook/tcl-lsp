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

// Issue #968: every built-in `expr` math function (`sin(...)`, `max(...)`,
// ...) drew a spurious W123 "unknown command" hint — the analyser only ever
// recognised a same-named user `proc ::tcl::mathfunc::<name>` override, never
// the built-in function table itself. The deep TP/FP/TN/FN coverage lives in
// the analyser and native lsp_e2e suites; this proves the same fix arrives
// through a real VS Code session.

import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate, waitForDiagnostics } from "./helper";

function codeOf(d: vscode.Diagnostic): string | undefined {
  return typeof d.code === "object" ? String(d.code.value) : (d.code as string | undefined);
}

suite("Issue #968: expr math functions", () => {
  test("built-in expr math functions draw no W123, unknown function still does", async () => {
    const docUri = getDocUri("issue968.tcl");
    await activate(docUri);

    // Wait for the deep pass to settle: the unknown `frobnicate` call on
    // line 12 must eventually surface its W123 — once it has, the earlier
    // (fixed) lines are guaranteed to have been analysed too.
    const diagnostics = await waitForDiagnostics(docUri, {
      predicate: (diags) => diags.some((d) => codeOf(d) === "W123" && d.range.start.line === 12),
      timeout: 15_000,
    });

    const w123Lines = diagnostics
      .filter((d) => codeOf(d) === "W123")
      .map((d) => d.range.start.line);

    // Line 5: `set builtin [expr {sin(1.0) + max(1, 2, 3)}]`.
    assert.ok(
      !w123Lines.includes(5),
      `built-in sin()/max() must not draw W123, got W123 on lines [${w123Lines}]`,
    );
    // Line 8: `set nested [expr {sqrt(abs($builtin))}]` — both the outer and
    // inner function calls must resolve.
    assert.ok(
      !w123Lines.includes(8),
      `nested built-in sqrt(abs(...)) must not draw W123, got W123 on lines [${w123Lines}]`,
    );
    // Line 12 (control): a genuinely unknown function name is still flagged.
    assert.ok(
      w123Lines.includes(12),
      `an unknown expr function must still draw W123, got W123 on lines [${w123Lines}]`,
    );
    const frobnicate = diagnostics.find((d) => codeOf(d) === "W123" && d.range.start.line === 12);
    assert.ok(
      frobnicate?.message.includes("frobnicate"),
      `W123 message should name the unknown function: ${frobnicate?.message}`,
    );
  });
});
