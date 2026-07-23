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

  // Issue #1009: the const-`$cmd` dispatch settlement resolved through a
  // proc renamed/deleted away with no later re-establishment, the same
  // root cause #973/#1006/#1007 fixed for bareword calls. Deep TP/FP/TN
  // coverage lives in the analyser and native e2e suites (see
  // rust/tcl-lsp-server/tests/e2e/issue945.rs); this proves the fix
  // reaches a real VS Code session.
  test("const-dispatch draws no reference to a proc deleted with no re-establishment", async () => {
    const docUri = getDocUri("issue1009ConstDispatchDeleted.tcl");
    await activate(docUri);

    // `target` declaration name at line 0, col 5.
    const locations = (await vscode.commands.executeCommand(
      "vscode.executeReferenceProvider",
      docUri,
      new vscode.Position(0, 5),
    )) as vscode.Location[];

    // Same-document hits only: a sibling fixture left open by an earlier
    // test in this suite (`issue945ConstDispatch.tcl`) shares this file's
    // `target`/`$cmd` shape but without the deletion, and legitimately
    // surfaces its own live references under cross-document search — those
    // must not be mistaken for this fixture's dead dispatch.
    const lines = (locations ?? [])
      .filter((l) => l.uri.toString() === docUri.toString())
      .map((l) => l.range.start.line);
    assert.ok(
      !lines.includes(2) && !lines.includes(3),
      `a proc deleted with no re-establishment must draw no reference from ` +
        `the dead $cmd dispatch ("set cmd target" / "$cmd"): ${JSON.stringify(lines)}`,
    );
  });

  // Codex PR #1014 review follow-up — two confirmed false positives found
  // in review after the #1009/#1006/#973 fixes landed. Both confirmed
  // against tclsh 8.6.14; deep TP/FP/TN coverage lives in the analyser and
  // native e2e suites (see `fact_live_for_call` / `finalise_invocation_resolutions`
  // unit tests and `rust/tcl-lsp-server/tests/e2e/issue945.rs`); these
  // prove the fixes reach a real VS Code session.

  test("body call before a later deletion draws no W123 (Codex review)", async () => {
    const docUri = getDocUri("issue1009CodexBodyCallBeforeDeletion.tcl");
    await activate(docUri);

    // `caller`'s own top-level invocation (line 2) runs before `rename
    // helper {}` (line 3), so the `helper` call inside its body must not
    // draw W123 — confirmed against tclsh 8.6.14 (the script runs to
    // completion).
    const diags = vscode.languages.getDiagnostics(docUri);
    assert.ok(
      !diags.some((d) => String(d.code) === "W123"),
      `a body call before a later deletion must not draw W123: ${JSON.stringify(diags.map((d) => d.code))}`,
    );
  });

  test("namespaced local call before a later deletion is a reference to the local definition (Codex review)", async () => {
    const docUri = getDocUri("issue1009CodexLocalCallBeforeDeletion.tcl");
    await activate(docUri);

    // `foo::caller`'s own top-level invocation (line 5) runs before
    // `rename foo::bar {}` (line 6), so the `bar` call inside its body
    // must reference the local `::foo::bar` (line 2 — `bar` at col 9),
    // not the global `bar` (line 0 — `bar` at col 5).
    const localRefs = ((await vscode.commands.executeCommand(
      "vscode.executeReferenceProvider",
      docUri,
      new vscode.Position(2, 9),
    )) ?? []) as vscode.Location[];
    const localLines = localRefs
      .filter((l) => l.uri.toString() === docUri.toString())
      .map((l) => l.range.start.line);
    assert.ok(
      localLines.includes(3),
      `the call before the deletion must reference the local definition: ${JSON.stringify(localLines)}`,
    );

    const globalRefs = ((await vscode.commands.executeCommand(
      "vscode.executeReferenceProvider",
      docUri,
      new vscode.Position(0, 5),
    )) ?? []) as vscode.Location[];
    const globalLines = globalRefs
      .filter((l) => l.uri.toString() === docUri.toString())
      .map((l) => l.range.start.line);
    assert.ok(
      !globalLines.includes(3),
      `the call must not also reference the global definition: ${JSON.stringify(globalLines)}`,
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
