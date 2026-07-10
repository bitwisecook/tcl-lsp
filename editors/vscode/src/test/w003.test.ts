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

// Fixture (`# tcl-dialect: tcl8.4`):
//   line 1: if {$a lt $b} {        -- 'lt' at columns 7..9
//   line 4: if {$c in $d && $e in $f} {  -- 'in' at columns 7..9 and 19..21

function codeOf(d: vscode.Diagnostic): string | number | undefined {
  return typeof d.code === "object" ? d.code?.value : d.code;
}

suite("W003 (dialect-gated expr operators)", () => {
  const docUri = getDocUri("w003.tcl");

  test("fires with a tight range covering only the operator", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 1 });
    const w003 = diagnostics.filter((d) => codeOf(d) === "W003");
    assert.ok(
      w003.length >= 1,
      `expected at least one W003, got ${JSON.stringify(diagnostics.map((d) => [codeOf(d), d.range]))}`,
    );

    const lt = w003.find((d) => d.range.start.line === 1);
    assert.ok(
      lt,
      `expected a W003 on line 1 ('lt'), got ${JSON.stringify(w003.map((d) => d.range))}`,
    );
    assert.strictEqual(lt!.range.start.character, 7, "range must start at 'lt'");
    assert.strictEqual(lt!.range.end.character, 9, "range must cover only 'lt'");
    assert.ok(lt!.message.includes("TIP 461"), `message should cite TIP 461: ${lt!.message}`);
  });

  test("reports one diagnostic per occurrence, each at its own range", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 3 });
    const w003 = diagnostics.filter((d) => codeOf(d) === "W003");
    // line 1: 'lt'; line 4: 'in' twice.
    assert.ok(
      w003.length >= 3,
      `expected 3 W003 diagnostics, got ${w003.length}: ${JSON.stringify(w003.map((d) => d.range))}`,
    );
    const onLine4 = w003.filter((d) => d.range.start.line === 4);
    assert.strictEqual(
      onLine4.length,
      2,
      `expected 2 W003 on line 4, got ${JSON.stringify(onLine4.map((d) => d.range))}`,
    );
    assert.notStrictEqual(
      onLine4[0].range.start.character,
      onLine4[1].range.start.character,
      "the two 'in' occurrences must not collapse onto the same range",
    );
  });

  test("offers a quick fix that rewrites 'lt' to a portable form", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 1 });
    const lt = diagnostics.find((d) => codeOf(d) === "W003" && d.range.start.line === 1);
    assert.ok(lt, "expected a W003 on line 1");

    const actions = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeCodeActionProvider", docUri, lt!.range),
      (r) =>
        Array.isArray(r) &&
        (r as vscode.CodeAction[]).some(
          (a) => a.kind && a.kind.value === vscode.CodeActionKind.QuickFix.value && a.edit,
        ),
      { timeout: 10_000, label: "W003 quick fix" },
    )) as vscode.CodeAction[];

    const fix = actions.find(
      (a) => a.edit && a.kind && a.kind.value === vscode.CodeActionKind.QuickFix.value,
    );
    assert.ok(
      fix,
      `expected a quick fix action, got titles: ${JSON.stringify(actions.map((a) => a.title))}`,
    );

    const edits = fix!.edit!.get(docUri);
    assert.ok(
      edits.some((e) => e.newText === "([string compare $a $b] < 0)"),
      `unexpected edit text: ${JSON.stringify(edits.map((e) => e.newText))}`,
    );
  });

  test("no quick fix is offered for the nested 'in' occurrences", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 3 });
    const inHit = diagnostics.find((d) => codeOf(d) === "W003" && d.range.start.line === 4);
    assert.ok(inHit, "expected a W003 on line 4");

    const actions = ((await vscode.commands.executeCommand(
      "vscode.executeCodeActionProvider",
      docUri,
      inHit!.range,
    )) ?? []) as vscode.CodeAction[];
    const lsearchFix = actions.find(
      (a) => a.edit && a.edit.get(docUri).some((e) => e.newText.includes("lsearch")),
    );
    assert.strictEqual(
      lsearchFix,
      undefined,
      "a gated operator nested in a larger expression must not offer the lsearch rewrite",
    );
  });
});
