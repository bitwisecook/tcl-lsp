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
import { getDocUri, activate, getServerLogSize, waitForServerLog } from "./helper";

suite("Rename Symbol", () => {
  const docUri = getDocUri("procs.tcl");

  test("prepare rename succeeds on a proc name", async () => {
    await activate(docUri);

    // 'fib' starts at line 1, col 5
    const pos = new vscode.Position(1, 5);

    const result = (await vscode.commands.executeCommand("vscode.prepareRename", docUri, pos)) as
      vscode.Range | { range: vscode.Range; placeholder: string } | undefined;

    assert.ok(result, "prepareRename should return a result for a proc name");
  });

  test("rename returns workspace edit for proc name", async () => {
    await activate(docUri);

    // 'fib' at line 1, col 5 (inside 'proc fib {n} {')
    const pos = new vscode.Position(1, 5);

    const edit = (await vscode.commands.executeCommand(
      "vscode.executeDocumentRenameProvider",
      docUri,
      pos,
      "fibonacci",
    )) as vscode.WorkspaceEdit | undefined;

    assert.ok(edit, "Rename should return a workspace edit");
    const entries = edit.entries();
    assert.ok(entries.length > 0, "WorkspaceEdit should have at least one entry");

    // Check that the edit includes changes to our document
    const docEdits = entries.find(([uri]) => uri.toString() === docUri.toString());
    assert.ok(docEdits, "Should include edits for the target document");

    // Should rename both the definition and the call site
    const [, textEdits] = docEdits;
    assert.ok(
      textEdits.length >= 2,
      `Should rename at least definition and call site, got ${textEdits.length} edits`,
    );
  });

  test("rename on a variable returns edits", async () => {
    await activate(docUri);

    // 'result' variable in factorial proc, line 9 col 8
    const pos = new vscode.Position(9, 8);

    const edit = (await vscode.commands.executeCommand(
      "vscode.executeDocumentRenameProvider",
      docUri,
      pos,
      "total",
    )) as vscode.WorkspaceEdit | undefined;

    assert.ok(edit, "Rename should return a workspace edit for a variable");
    const entries = edit.entries();
    assert.ok(entries.length > 0, "WorkspaceEdit should have entries for variable rename");
  });

  // Renaming a TclOO instance variable must edit its
  // `variable` declaration and `$var` uses, and NEVER rewrite the method body.
  test("rename of a TclOO instance variable does not rewrite the method body", async () => {
    const ooUri = getDocUri("tclooVariableRename.tcl");
    await activate(ooUri);

    // `$n` in `method get {} { return $n }` — line 2 (0-based); `$` is at
    // column 27 and `n` at 28 (the `}` is column 30), so put the cursor on `n`.
    const pos = new vscode.Position(2, 28);

    const edit = (await vscode.commands.executeCommand(
      "vscode.executeDocumentRenameProvider",
      ooUri,
      pos,
      "count",
    )) as vscode.WorkspaceEdit | undefined;

    assert.ok(edit, "Rename should return a workspace edit for an instance variable");
    const docEdits = edit.entries().find(([uri]) => uri.toString() === ooUri.toString());
    assert.ok(docEdits, "Should include edits for the target document");
    const [, textEdits] = docEdits!;
    assert.ok(textEdits.length > 0, "expected at least one edit");

    for (const te of textEdits) {
      // FP guard: no edit may span multiple lines (the body-destroying symptom),
      // and none may cover the whole `{ return $n }` body on line 2.
      assert.strictEqual(
        te.range.start.line,
        te.range.end.line,
        `no rename edit may span lines (body-destroying): ${JSON.stringify(te.range)}`,
      );
      const coversBody =
        te.range.start.line === 2 && te.range.start.character <= 22 && te.range.end.character >= 33;
      assert.ok(!coversBody, `edit must not cover the method body: ${JSON.stringify(te.range)}`);
    }

    // The `variable n` declaration (line 1) is renamed.
    const declEdit = textEdits.find((te) => te.range.start.line === 1);
    assert.ok(declEdit, "expected the `variable n` declaration (line 1) to be renamed");
    assert.strictEqual(declEdit!.newText, "count");
  });

  // Renaming from a bareword call site must target the caller's namespace,
  // never a same-named proc in an unrelated namespace.
  test("rename from a call site does not touch a same-named proc in another namespace", async () => {
    const uri = getDocUri("renameNamespaceCollision.tcl");
    await activate(uri);

    // `helper` call inside `::a::run` — line 2 (0-based).
    const pos = new vscode.Position(2, 18);

    const edit = (await vscode.commands.executeCommand(
      "vscode.executeDocumentRenameProvider",
      uri,
      pos,
      "assist",
    )) as vscode.WorkspaceEdit | undefined;

    assert.ok(edit, "Rename should return a workspace edit");
    const docEdits = edit.entries().find(([u]) => u.toString() === uri.toString());
    assert.ok(docEdits, "Should include edits for the target document");
    const [, textEdits] = docEdits!;
    const lines = textEdits.map((te) => te.range.start.line);
    // ::a::helper decl (line 1) + call (line 2) rename; ::b::helper (line 5) must not.
    assert.ok(lines.includes(1), `::a::helper decl should rename: ${JSON.stringify(lines)}`);
    assert.ok(
      !lines.includes(5),
      `::b::helper (line 5) must NOT be renamed: ${JSON.stringify(lines)}`,
    );
  });

  // M8: a rename triggered from a consumer-only document — the command is
  // defined in the auto-loaded library file, not locally — resolves through
  // the workspace oracle and rewrites the library declaration alongside the
  // consumer's call site.  (Previously the empty in-document rename aborted
  // the whole request.)
  test("rename from a consumer document rewrites the auto-loaded library definition (M8)", async () => {
    const uri = getDocUri("autoloadLibrary.tcl");
    // Snapshot the log cursor BEFORE opening so the wait below only matches
    // this document's settle, not a stale line from an earlier test.
    const since = getServerLogSize();
    await activate(uri);

    // Deterministic readiness via message passing — no polling.  The consumer's
    // own call site (`Rbc_ActiveLegend .g`) only becomes a cross-document
    // reference once its *focused* analysis is committed to the workspace index
    // (that walk is what gives the invocation its resolution candidates, which
    // the cross-document rename matches against).  The server logs
    // `[timing] workspace_state.update (uri=…autoloadLibrary.tcl …)` at the end
    // of that commit, so wait for exactly that message.  The timeout is only a
    // backstop against a total hang, never the synchronisation mechanism.
    const settled = await waitForServerLog(
      (line) => line.includes("workspace_state.update") && line.includes("autoloadLibrary.tcl"),
      { since, timeout: 20_000 },
    );
    assert.ok(
      settled,
      "server must report the consumer document's workspace_state.update before the deadline",
    );

    // `Rbc_ActiveLegend .g` — line 0; the definition lives in
    // rbclib/graph.tcl (line 2, after two comment lines).  A single rename now
    // resolves the whole edit set (the library file is merged synchronously by
    // the autoload tier within this request).
    const pos = new vscode.Position(0, 3);
    const edit = (await vscode.commands.executeCommand(
      "vscode.executeDocumentRenameProvider",
      uri,
      pos,
      "Rbc_ShinyLegend",
    )) as vscode.WorkspaceEdit | undefined;

    assert.ok(edit, "Rename should return a workspace edit");
    const libEntry = edit!.entries().find(([u]) => u.path.endsWith("rbclib/graph.tcl"));
    assert.ok(libEntry, "the library declaration in rbclib/graph.tcl must be rewritten");
    const [, libEdits] = libEntry!;
    assert.ok(
      libEdits.some((te) => te.range.start.line === 2 && te.newText === "Rbc_ShinyLegend"),
      `graph.tcl's declaration (line 2) must be renamed: ${JSON.stringify(
        libEdits.map((te) => te.range.start.line),
      )}`,
    );
    const docEntry = edit!.entries().find(([u]) => u.toString() === uri.toString());
    assert.ok(docEntry, "the consumer's own call site must be rewritten too");
    assert.ok(
      docEntry![1].some((te) => te.range.start.line === 0),
      "the line-0 call site is part of the edit",
    );
  });

  // Issue #923: renaming a class must rewrite every `superclass` site that
  // names it, or the inheritance graph is silently broken.  In `oo-shapes.tcl`
  // both `Dog` and `Cat` declare `superclass Animal`.
  test("rename of a class rewrites its superclass sites", async () => {
    const uri = getDocUri("oo-shapes.tcl");
    await activate(uri);

    // "Animal" in its declaration `oo::class create Animal` (line 1, col 18).
    const pos = new vscode.Position(1, 18);

    const edit = (await vscode.commands.executeCommand(
      "vscode.executeDocumentRenameProvider",
      uri,
      pos,
      "Creature",
    )) as vscode.WorkspaceEdit | undefined;

    assert.ok(edit, "Rename should return a workspace edit for a class name");
    const docEntry = edit.entries().find(([u]) => u.toString() === uri.toString());
    assert.ok(docEntry, "Should include edits for oo-shapes.tcl");
    const lines = docEntry![1].map((te) => te.range.start.line);
    assert.ok(
      lines.includes(7) && lines.includes(12),
      `Both superclass sites (lines 7 and 12) must be rewritten; got ${JSON.stringify(lines)}`,
    );
  });
});
