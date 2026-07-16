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
import { getDocUri, activate, pollUntil } from "./helper";

/**
 * Open the fixture, apply an in-memory edit that inserts a probe (``$`` or
 * array suffix) at a known offset, trigger a completion request at the end
 * of the inserted text, and return the items.  After the test, revert the
 * file so the next test sees the canonical on-disk content.
 *
 * The completion request is **polled** until the server has re-indexed the
 * post-edit content and offers at least one variable (``$``-prefixed) item —
 * every caller probes a ``puts $`` site and asserts on ``$``-prefixed labels.
 * Requesting once immediately after ``applyEdit`` raced the analysis-snapshot
 * rebuild and was flaky under load (the request could land before the new
 * version was analysed, returning only keyword/command items).  This mirrors
 * the robust ``pollUntil`` pattern in ``variableContexts.test.ts``.
 *
 * The **revert is awaited to quiescence**, not merely issued.
 * ``workbench.action.files.revert`` resolves once the editor buffer is
 * restored, but the server has not necessarily re-analysed the restored text
 * and re-published diagnostics by then.  A later test that reads diagnostics
 * could therefore observe ranges computed against the *edited* document —
 * shifted by the length of ``insertion`` — and go on to query code actions at
 * the wrong line.  That is what made ``variableContexts``'s W216 quick-fix
 * test fail in a full run while passing in isolation: it found the
 * ``${arr}(name)`` diagnostic but at a stale range, so the provider returned
 * the *other* W216's fix.  Waiting for the buffer to be back to baseline here
 * keeps the staleness from leaking out of this helper at all.
 */
async function completionItems(
  uri: vscode.Uri,
  insertion: string,
  position: vscode.Position,
): Promise<vscode.CompletionItem[]> {
  const doc = await activate(uri);
  const baseline = doc.getText();
  const startOffset = doc.offsetAt(position);
  const edit = new vscode.WorkspaceEdit();
  edit.insert(uri, position, insertion);
  await vscode.workspace.applyEdit(edit);
  // Recompute the completion position from the post-insertion offset so
  // newlines in ``insertion`` don't throw off the line/character math.
  const completionPos = doc.positionAt(startOffset + insertion.length);
  try {
    return await pollUntil(
      async () => {
        const result = (await vscode.commands.executeCommand(
          "vscode.executeCompletionItemProvider",
          uri,
          completionPos,
        )) as vscode.CompletionList;
        return result.items;
      },
      (items) => items.some((i) => labelOf(i).startsWith("$")),
      {
        timeout: 10_000,
        label: `variable completions at ${completionPos.line}:${completionPos.character}`,
      },
    );
  } finally {
    await revertToBaseline(doc, baseline);
  }
}

/**
 * Revert `doc` and wait until its buffer is actually back to `baseline`.
 *
 * The revert command resolves before the restored text is necessarily visible
 * to everything downstream; returning early leaves the *next* test reading a
 * document — and diagnostics — that still reflect the edit.
 */
async function revertToBaseline(doc: vscode.TextDocument, baseline: string): Promise<void> {
  await vscode.commands.executeCommand("workbench.action.files.revert", doc);
  await pollUntil(
    () => Promise.resolve(doc.getText()),
    (text) => text === baseline,
    { timeout: 10_000, label: "document reverted to its on-disk content" },
  );
}

function labelOf(item: vscode.CompletionItem): string {
  return typeof item.label === "string" ? item.label : item.label.label;
}

function insertedText(item: vscode.CompletionItem): string | undefined {
  if (typeof item.insertText === "string") return item.insertText;
  if (item.insertText) return item.insertText.value;
  return undefined;
}

// Fixture content (mirrors testFixture/variableCompletion.tcl) -- offsets
// in the tests below are relative to the unedited fixture.
//
// Line index 17 = ``        set local_var 10`` (24 chars; pos.character 24
// is end of line).  This is the canonical "scratch" point inside the
// helper proc body where probe edits are inserted.
const HELPER_BODY_INSERT = new vscode.Position(17, 24);

suite("Variable Completion", () => {
  const docUri = getDocUri("variableCompletion.tcl");

  test("bare $ inside helper proc offers params, locals, upvar, and global aliases", async () => {
    const items = await completionItems(docUri, "\n        puts $", HELPER_BODY_INSERT);
    const labels = items.map(labelOf);
    assert.ok(
      labels.includes("$local_var"),
      `Expected $local_var in completions: ${labels.slice(0, 30).join(", ")}`,
    );
    assert.ok(
      labels.includes("$arg"),
      `Expected $arg (proc param) in completions: ${labels.slice(0, 30).join(", ")}`,
    );
    assert.ok(
      labels.includes("$local_alias"),
      `Expected $local_alias (upvar alias) in completions: ${labels.slice(0, 30).join(", ")}`,
    );
    assert.ok(
      labels.includes("$myglobal"),
      `Expected $myglobal (global decl alias) in completions: ${labels.slice(0, 30).join(", ")}`,
    );
  });

  test("brace-required name auto-promotes to ${name}", async () => {
    // ``foo-bar`` is set at top level (see fixture).  From inside the
    // ``::myns::helper`` proc, Tcl's runtime requires the qualified
    // form ``$::foo-bar`` to reach a global var that has no local
    // alias, so completion offers ``$::foo-bar`` (with the brace-form
    // promotion for the hyphen).
    const items = await completionItems(docUri, "\n        puts $", HELPER_BODY_INSERT);
    const item = items.find((i) => labelOf(i) === "$::foo-bar");
    assert.ok(
      item,
      `Expected $::foo-bar in completions; labels: ${items
        .map(labelOf)
        .filter((s) => s.startsWith("$"))
        .join(", ")}`,
    );
    assert.strictEqual(
      insertedText(item),
      "${::foo-bar}",
      `Expected '${"${::foo-bar}"}' for hyphenated qualified name, got '${insertedText(item)}'`,
    );
  });

  test("cross-namespace $::other::baz and $::globalvar are offered from ::myns::helper", async () => {
    const items = await completionItems(docUri, "\n        puts $", HELPER_BODY_INSERT);
    const labels = items.map(labelOf);
    assert.ok(
      labels.includes("$::other::baz"),
      `Expected $::other::baz: ${labels.slice(0, 30).join(", ")}`,
    );
    assert.ok(
      labels.includes("$::globalvar"),
      `Expected $::globalvar: ${labels.slice(0, 30).join(", ")}`,
    );
  });

  test("foreach loop var $item is in scope inside the loop body", async () => {
    // ``        foreach item {a b c} {`` is line 20; body line 21 ``            puts $item`` (22 chars).
    const items = await completionItems(
      docUri,
      "\n            puts $",
      new vscode.Position(21, 22),
    );
    const labels = items.map(labelOf);
    assert.ok(
      labels.includes("$item"),
      `Expected $item in foreach body: ${labels.slice(0, 30).join(", ")}`,
    );
  });

  test("dict for loop vars $k and $v are in scope inside body", async () => {
    // ``        dict for {k v} {x 1 y 2} {`` is line 23; body line 24
    // ``            puts $k`` (19 chars).
    const items = await completionItems(
      docUri,
      "\n            puts $",
      new vscode.Position(24, 19),
    );
    const labels = items.map(labelOf);
    assert.ok(
      labels.includes("$k"),
      `Expected $k in dict for body: ${labels.slice(0, 30).join(", ")}`,
    );
    assert.ok(
      labels.includes("$v"),
      `Expected $v in dict for body: ${labels.slice(0, 30).join(", ")}`,
    );
  });

  test("dict with body sees keys from a const-literal dict", async () => {
    // ``        dict with d {`` is line 27; body line 28 ``            puts $name`` (22 chars).
    const items = await completionItems(
      docUri,
      "\n            puts $",
      new vscode.Position(28, 22),
    );
    const labels = items.map(labelOf);
    assert.ok(
      labels.includes("$name"),
      `Expected $name from dict literal: ${labels.slice(0, 30).join(", ")}`,
    );
    assert.ok(
      labels.includes("$age"),
      `Expected $age from dict literal: ${labels.slice(0, 30).join(", ")}`,
    );
  });

  test("uplevel #0 body sees globals + body locals but NOT proc locals", async () => {
    // ``        uplevel #0 {`` is line 30; body line 31
    // ``            set uplevel_var 99`` (30 chars).
    const items = await completionItems(
      docUri,
      "\n            puts $",
      new vscode.Position(31, 30),
    );
    const labels = items.map(labelOf);
    assert.ok(
      labels.includes("$::globalvar"),
      `Expected $::globalvar in uplevel #0 body: ${labels.slice(0, 30).join(", ")}`,
    );
    assert.ok(
      labels.includes("$uplevel_var"),
      `Expected $uplevel_var in uplevel #0 body: ${labels.slice(0, 30).join(", ")}`,
    );
    assert.ok(
      !labels.includes("$local_var"),
      `Did NOT expect $local_var (proc local) in uplevel #0 body: ${labels
        .slice(0, 30)
        .join(", ")}`,
    );
  });

  test("array element completion offers known indices", async () => {
    // Use a balanced ``$arr()`` probe and place the cursor between the
    // parens.  An unbalanced ``$arr(`` would let the lexer's array-index
    // scanner eat the rest of the file looking for ``)``, which breaks
    // the surrounding proc analysis and starves the LSP of array data.
    const doc = await activate(docUri);
    const baseline = doc.getText();
    const insertOffset = doc.offsetAt(HELPER_BODY_INSERT);
    const insertion = "\n        puts $arr()";
    const edit = new vscode.WorkspaceEdit();
    edit.insert(docUri, HELPER_BODY_INSERT, insertion);
    await vscode.workspace.applyEdit(edit);
    // Cursor goes between ``(`` and ``)`` -- one char before the end.
    const completionPos = doc.positionAt(insertOffset + insertion.length - 1);
    try {
      // Poll the completion request until the re-indexed array data lands,
      // mirroring the file's `completionItems` helper: firing once right after
      // applyEdit races the analysis-snapshot rebuild and returned only
      // keyword/command items under load.  pollUntil throws on timeout, so a
      // genuine "array indices never offered" regression still fails loudly.
      const items = await pollUntil(
        async () => {
          const result = (await vscode.commands.executeCommand(
            "vscode.executeCompletionItemProvider",
            docUri,
            completionPos,
          )) as vscode.CompletionList;
          return result.items;
        },
        (candidates) => {
          const labels = candidates.map(labelOf);
          return labels.includes("$arr(name)") && labels.includes("$arr(age)");
        },
        {
          timeout: 10_000,
          label: `array element completions at ${completionPos.line}:${completionPos.character}`,
        },
      );
      const labels = items.map(labelOf);
      assert.ok(
        labels.includes("$arr(name)"),
        `Expected $arr(name): ${labels.slice(0, 30).join(", ")}`,
      );
      assert.ok(
        labels.includes("$arr(age)"),
        `Expected $arr(age): ${labels.slice(0, 30).join(", ")}`,
      );
    } finally {
      await revertToBaseline(doc, baseline);
    }
  });
});
