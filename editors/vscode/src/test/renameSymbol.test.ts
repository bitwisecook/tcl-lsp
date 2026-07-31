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
import { getDocUri, activate } from "./helper";

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
    await activate(uri);

    // No log-line wait needed here (issue #1003): the server's autoload
    // resolution (`ensure_autoload_indexed`) now blocks out any in-flight
    // `scan_workspace_folders` internally before consulting the package
    // database, so the rename request below is correct regardless of when
    // it lands relative to that scan — including immediately after
    // `activate`, before any workspace-wide scan has necessarily finished.
    // (An earlier version of this test waited for this document's own
    // `[timing] workspace_state.update` log line, which was both the wrong
    // signal for the actual dependency — that per-document commit is
    // unrelated to the workspace-wide package-database scan the autoload
    // tier needs — and unreliable in a full suite run, where an earlier
    // test can already have opened this same fixture, leaving no *new*
    // line for a fresh `since` cursor to ever match.)

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

  // Issue #923 finding idx 79 — the rename **safety gate**.
  //
  // `$other` in `Vector3d`'s copy constructor really is a `Vector3d` at run
  // time (tclsh 9.0.4 / 8.6.16 both print `7 9 7 9` for this fixture), but it
  // comes from `[lindex $args 0]` behind a runtime `info object isa` test, so
  // the analyser has no class binding for it.  Renaming `X` used to emit an
  // edit set touching only the declaration and the `export` list; applying it
  // and re-running gives, on both interpreters:
  //
  //   unknown method "X": must be Get, GetX, Y or destroy
  //       while executing
  //   "$other X"
  //
  // The server now refuses with an LSP error, so VS Code surfaces the reason
  // instead of quietly applying nothing.  This test is the editor-facing half:
  // the command must *reject*, not resolve to an empty edit.
  test("rename refuses a method dispatched on an untracked receiver", async () => {
    const uri = getDocUri("renameUntrackedReceiver.tcl");
    await activate(uri);

    // `X` in `method X {} { return $_x }` — line 14 (0-based), col 11.
    const pos = new vscode.Position(14, 11);

    let rejected: unknown;
    let resolved: vscode.WorkspaceEdit | undefined;
    try {
      resolved = (await vscode.commands.executeCommand(
        "vscode.executeDocumentRenameProvider",
        uri,
        pos,
        "GetX",
      )) as vscode.WorkspaceEdit | undefined;
    } catch (err) {
      rejected = err;
    }

    assert.ok(
      rejected !== undefined,
      `the rename must be refused, not answered with ${JSON.stringify(
        resolved?.entries().map(([u, edits]) => [u.path, edits.length]),
      )}`,
    );
    assert.match(
      String(rejected),
      /not tracked/,
      `the refusal must carry the gate's reason: ${String(rejected)}`,
    );
  });

  // FN guard for the gate above: a member the untracked receiver never names
  // still renames normally — over-refusal would make the feature unusable on
  // any class with an internal `$var method` helper.
  test("rename still applies for a member the untracked receiver never names", async () => {
    const uri = getDocUri("renameUntrackedReceiver.tcl");
    await activate(uri);

    // `Get` in `method Get {} { ... }` — line 16 (0-based), col 11.
    const pos = new vscode.Position(16, 11);

    const edit = (await vscode.commands.executeCommand(
      "vscode.executeDocumentRenameProvider",
      uri,
      pos,
      "Fetch",
    )) as vscode.WorkspaceEdit | undefined;

    assert.ok(edit, "an unaffected member must still rename");
    const docEntry = edit!.entries().find(([u]) => u.toString() === uri.toString());
    assert.ok(docEntry, "Should include edits for renameUntrackedReceiver.tcl");
    const lines = docEntry![1].map((te) => te.range.start.line);
    assert.ok(
      lines.includes(16) && lines.includes(17),
      `the declaration (16) and its \`export\` word (17) must both rename; got ${JSON.stringify(
        lines,
      )}`,
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
