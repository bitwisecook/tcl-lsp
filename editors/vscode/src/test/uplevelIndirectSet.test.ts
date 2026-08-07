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

// Caller-frame injection via `uplevel`, end to end in the editor
// (issue #923 audit idx 24 and idx 38).
//
// uplevelIndirectSet.tcl prints `99`, `99`, `1 1 1` under tclsh 9.0.4 and
// 8.6.16, so every variable it reads really is assigned first, and every
// `$varName` in a `[list set …]` is the idiom rather than a name/value
// confusion.  It carries one deliberate W214 marker so an empty
// W210/W212 set cannot be mistaken for "analysis has not run yet".
//
// uplevelIndirectSetControl.tcl strips the frame-crossing part out, at which
// point the same read genuinely fails (`can't read "answer": no such
// variable`) and the `set $var` really is written by hand — so a suppression
// that had gone too far shows up there as a *missing* diagnostic.

/** The `code` string of a diagnostic, whatever shape VS Code hands us. */
function codeOf(d: vscode.Diagnostic): string | undefined {
  const code = typeof d.code === "object" && d.code !== null ? d.code.value : d.code;
  return code === undefined ? undefined : String(code);
}

suite("Uplevel Indirect Assignment (W210 / W212)", () => {
  const docUri = getDocUri("uplevelIndirectSet.tcl");
  const controlUri = getDocUri("uplevelIndirectSetControl.tcl");

  test("no W210 or W212 on any uplevel-injected assignment", async () => {
    await activate(docUri);
    // W214 on the marker proc is the signal that a full analysis ran.
    const diagnostics = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W214"),
    });
    const offenders = diagnostics
      .filter((d) => codeOf(d) === "W210" || codeOf(d) === "W212")
      .map((d) => `${codeOf(d)}@${d.range.start.line}: ${d.message}`);
    assert.deepStrictEqual(
      offenders,
      [],
      "uplevel-injected assignments are real assignments and their list-built " +
        "name words are the idiom, so neither code may fire",
    );
  });

  test("W210 and W212 still fire when nothing reaches an outer frame", async () => {
    await activate(controlUri);
    const diagnostics = await waitForDiagnostics(controlUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W210"),
    });

    const w210Lines = diagnostics
      .filter((d) => codeOf(d) === "W210")
      .map((d) => d.range.start.line);
    assert.ok(
      w210Lines.includes(11),
      `W210 belongs on the 'return $answer' read (line 11), got ${w210Lines.join(", ")}`,
    );

    const w212Lines = diagnostics
      .filter((d) => codeOf(d) === "W212")
      .map((d) => d.range.start.line);
    assert.ok(
      w212Lines.includes(7),
      `W212 belongs on the written 'set $var' (line 7), got ${w212Lines.join(", ")}`,
    );
  });
});
