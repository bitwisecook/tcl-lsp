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

// Issue #945 resolution-model follow-up, editor-integration layer:
// constant-`$cmd` rename provenance (fault 1), TclOO export visibility +
// dispatch entry + per-object binding identity (faults 4-6), and probe
// references (fault 9).  The deep TP/FP/TN/FN coverage lives in the
// analyser and native e2e suites; these prove the same behaviour arrives
// through a real VS Code session.

import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate } from "./helper";

suite("Issue #945 resolution model", () => {
  test("const-dispatch rename rewrites the defining literal, never $cmd", async () => {
    const docUri = getDocUri("issue945ConstDispatch.tcl");
    await activate(docUri);

    // `target` declaration name at line 0, col 5.
    const edit = (await vscode.commands.executeCommand(
      "vscode.executeDocumentRenameProvider",
      docUri,
      new vscode.Position(0, 5),
      "renamed",
    )) as vscode.WorkspaceEdit | undefined;

    assert.ok(edit, "rename should return a workspace edit");
    const docEdits = edit.entries().find(([uri]) => uri.toString() === docUri.toString());
    assert.ok(docEdits, "should include edits for the fixture");
    const [, textEdits] = docEdits;

    // The defining literal on line 1 (`set cmd target`) must be rewritten
    // — leaving it stale produced Tcl that dies with `invalid command
    // name "target"` (tclsh 9.0.4) — and the `$cmd` head on line 2 must
    // never be touched.
    const lines = textEdits.map((e) => e.range.start.line).sort();
    assert.ok(
      lines.includes(1),
      `the defining literal follows the rename: ${JSON.stringify(lines)}`,
    );
    assert.ok(!lines.includes(2), `the $cmd head is never rewritten: ${JSON.stringify(lines)}`);
    assert.ok(
      textEdits.every((e) => e.newText === "renamed"),
      "every edit splices the new name",
    );
  });

  test("externally unexported TclOO method does not resolve; dispatch entry does", async () => {
    const docUri = getDocUri("issue945Tcloo.tcl");
    await activate(docUri);

    // `$v _secret` (line 4, col 4): tclsh 9.0.4 raises `unknown method
    // "_secret"` — external navigation must resolve nothing.
    const hidden = (await vscode.commands.executeCommand(
      "vscode.executeDefinitionProvider",
      docUri,
      new vscode.Position(4, 4),
    )) as vscode.Location[];
    assert.strictEqual(
      (hidden ?? []).length,
      0,
      `an externally unexported method is not callable: ${JSON.stringify(hidden)}`,
    );

    // `$d speak` (line 13, col 4): the dispatch enters Dog::speak
    // (line 10), never the whole Animal/Dog override family.
    const speak = (await vscode.commands.executeCommand(
      "vscode.executeDefinitionProvider",
      docUri,
      new vscode.Position(13, 4),
    )) as vscode.Location[];
    assert.strictEqual(speak.length, 1, `one dispatch entry: ${JSON.stringify(speak)}`);
    assert.strictEqual(
      speak[0].range.start.line,
      10,
      `Dog::speak is the runtime entry: ${JSON.stringify(speak)}`,
    );

    // Per-object methods key by binding identity: b's `$o m` (line 23,
    // col 7) resolves b's own objdefine override (line 22), never a's
    // (line 17).
    const perObject = (await vscode.commands.executeCommand(
      "vscode.executeDefinitionProvider",
      docUri,
      new vscode.Position(23, 7),
    )) as vscode.Location[];
    assert.strictEqual(perObject.length, 1, JSON.stringify(perObject));
    assert.strictEqual(
      perObject[0].range.start.line,
      22,
      `b's dispatch resolves b's own override: ${JSON.stringify(perObject)}`,
    );
  });

  test("command probes navigate without asserting existence", async () => {
    const docUri = getDocUri("issue945Probe.tcl");
    await activate(docUri);

    // The probe of the existing proc navigates to its declaration.
    const defs = (await vscode.commands.executeCommand(
      "vscode.executeDefinitionProvider",
      docUri,
      new vscode.Position(1, 27),
    )) as vscode.Location[];
    assert.strictEqual(defs.length, 1, JSON.stringify(defs));
    assert.strictEqual(
      defs[0].range.start.line,
      0,
      `the probe site navigates to the declaration: ${JSON.stringify(defs)}`,
    );

    // The missing-target probe draws no W123 — a probe asserts nothing.
    const diags = vscode.languages.getDiagnostics(docUri);
    assert.ok(
      !diags.some((d) => String(d.code) === "W123"),
      `a probe of an absent command draws no W123: ${JSON.stringify(diags.map((d) => d.code))}`,
    );
  });
});
