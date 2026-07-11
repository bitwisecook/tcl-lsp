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
import { getDocUri, activate, waitForDiagnostics, pollUntil } from "./helper";

// Issue #806 — report::defstyle style scripts expose the report configuration
// methods (top/data/columns/…) as scoped commands.  They must not be flagged as
// unknown commands, while a genuine typo still is, and they hover with their
// scoped-environment documentation.
suite("Report style scoped commands (#806)", () => {
  const docUri = getDocUri("reportStyle.tcl");

  const codeOf = (d: vscode.Diagnostic): string =>
    typeof d.code === "object" ? String(d.code.value) : String(d.code);

  test("scoped report commands do not draw W123, but a typo does", async () => {
    // W123 defaults to false (opt-in, see configSettings.test.ts's
    // "diagnostics.W123 defaults to false" test) -- without this the wait
    // below never sees the typo flagged and the assertion is vacuous.
    const w123config = vscode.workspace.getConfiguration("tclLsp.diagnostics");
    await w123config.update("W123", true, vscode.ConfigurationTarget.Global);
    try {
      await activate(docUri);
      // Wait until the `toop` typo (line 7) has been flagged.
      const diagnostics = await waitForDiagnostics(docUri, {
        predicate: (diags) => diags.some((d) => codeOf(d) === "W123" && d.range.start.line === 7),
      });

      const w123 = diagnostics.filter((d) => codeOf(d) === "W123");
      // The typo on line 7 is flagged.
      assert.ok(
        w123.some((d) => d.range.start.line === 7),
        `Expected W123 on the \`toop\` typo (line 7), got: ${w123.map((d) => d.range.start.line)}`,
      );
      // The valid scoped commands on lines 2–6 are NOT flagged.
      for (const line of [2, 3, 4, 5, 6]) {
        assert.ok(
          !w123.some((d) => d.range.start.line === line),
          `Line ${line} carries a valid scoped command and must not draw W123`,
        );
      }
    } finally {
      await w123config.update("W123", undefined, vscode.ConfigurationTarget.Global);
    }
  });

  test("hover on a scoped report command shows its documentation", async () => {
    await activate(docUri);
    // `top` on line 2 (col 5).
    const position = new vscode.Position(2, 5);
    const hovers = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeHoverProvider", docUri, position),
      (r) => Array.isArray(r) && r.length > 0,
      { timeout: 10_000, label: "hover for scoped report command" },
    )) as vscode.Hover[];

    const hoverText = hovers
      .flatMap((h) => h.contents)
      .map((c) => (typeof c === "string" ? c : (c as { value: string }).value))
      .join("\n");

    assert.ok(
      hoverText.includes("report style"),
      `Hover should describe the scoped environment, got: ${hoverText}`,
    );
  });
});
