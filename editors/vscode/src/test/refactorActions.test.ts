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
import { activate, getDocUri, pollUntil, setTestContent } from "./helper";

// Apply-the-edit refactoring tests.  Unlike codeActions.test.ts (which checks
// that an action exists), these load source into a scratch document, request
// the refactoring code action, APPLY its WorkspaceEdit, and assert the exact
// resulting document text — the only way to catch dropped characters, lost
// braces, or broken string interpolation end-to-end through VS Code.

suite("Refactor Actions (applied)", () => {
  const docUri = getDocUri("formatting.tcl");

  teardown(async () => {
    // Discard the scratch edits so the fixture file stays pristine.
    const editor = vscode.window.activeTextEditor;
    if (editor && editor.document.uri.toString() === docUri.toString()) {
      await vscode.commands.executeCommand("workbench.action.files.revert", docUri);
    }
  });

  async function loadScratch(source: string): Promise<vscode.TextEditor> {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    await setTestContent(editor, source);
    return editor;
  }

  async function findAction(range: vscode.Range, titleNeedle: string): Promise<vscode.CodeAction> {
    const found = await pollUntil(
      async () => {
        const actions = (await vscode.commands.executeCommand(
          "vscode.executeCodeActionProvider",
          docUri,
          range,
        )) as vscode.CodeAction[];
        return (actions || []).find((a) => a.title.includes(titleNeedle));
      },
      (a) => a !== undefined,
      { timeout: 10_000, label: titleNeedle },
    );
    assert.ok(found, `code action "${titleNeedle}" not found`);
    return found;
  }

  test("extract variable wraps a bare operator expression in [expr {...}]", async () => {
    const editor = await loadScratch("set total [expr {$a + $b}]\n");
    // Select the inner expression ``$a + $b``.
    const range = new vscode.Range(new vscode.Position(0, 17), new vscode.Position(0, 24));
    const action = await findAction(range, "Extract into variable");
    assert.ok(action.edit, "extract-variable action must carry an edit");
    await vscode.workspace.applyEdit(action.edit!);
    const text = editor.document.getText();
    assert.ok(
      text.includes("set result [expr {$a + $b}]"),
      `extracted assignment must wrap the expr; got:\n${text}`,
    );
    assert.ok(
      text.includes("set total [expr {$result}]"),
      `reference must be replaced with $result; got:\n${text}`,
    );
  });

  test("inline variable into a quoted string does not double the quotes", async () => {
    const editor = await loadScratch('set name "world"\nputs "hello $name"\n');
    const range = new vscode.Range(new vscode.Position(0, 4), new vscode.Position(0, 4));
    const action = await findAction(range, "Inline variable");
    assert.ok(action.edit, "inline-variable action must carry an edit");
    await vscode.workspace.applyEdit(action.edit!);
    assert.strictEqual(editor.document.getText(), 'puts "hello world"\n');
  });

  test("inline proc preserves the body braces", async () => {
    const editor = await loadScratch("proc double {x} {\n    expr {$x * 2}\n}\ndouble 5\n");
    const range = new vscode.Range(new vscode.Position(3, 0), new vscode.Position(3, 0));
    const action = await findAction(range, "Inline proc");
    assert.ok(action.edit, "inline-proc action must carry an edit");
    await vscode.workspace.applyEdit(action.edit!);
    const text = editor.document.getText();
    assert.ok(text.includes("expr {5 * 2}"), `braces must survive inlining; got:\n${text}`);
    assert.ok(!/expr 5 \* 2/.test(text), `unbraced expr must not appear; got:\n${text}`);
  });

  test("if/elseif chain converts to a switch", async () => {
    const source =
      "if {$x == 1} {\n" +
      "    puts one\n" +
      "} elseif {$x == 2} {\n" +
      "    puts two\n" +
      "} else {\n" +
      "    puts other\n" +
      "}\n";
    const editor = await loadScratch(source);
    const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 0));
    const action = await findAction(range, "switch");
    assert.ok(action.edit, "if-to-switch action must carry an edit");
    await vscode.workspace.applyEdit(action.edit!);
    const text = editor.document.getText();
    assert.ok(text.includes("switch -exact -- $x {"), `expected a switch; got:\n${text}`);
    assert.ok(text.includes("puts one"), "branch bodies must survive");
    assert.ok(text.includes("default {"), "else must become default");
    assert.ok(!text.includes("elseif"), "no elseif fragment may remain");
  });

  test("switch converts to a dict lookup", async () => {
    const source =
      "switch -exact -- $color {\n" +
      '    red { set hex "#ff0000" }\n' +
      '    green { set hex "#00ff00" }\n' +
      '    blue { set hex "#0000ff" }\n' +
      "}\n";
    const editor = await loadScratch(source);
    const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 0));
    const action = await findAction(range, "dict lookup");
    assert.ok(action.edit, "switch-to-dict action must carry an edit");
    await vscode.workspace.applyEdit(action.edit!);
    const text = editor.document.getText();
    assert.ok(text.includes("dict create"), `expected dict create; got:\n${text}`);
    assert.ok(text.includes("dict get"), `expected dict get; got:\n${text}`);
    assert.ok(text.includes('"#ff0000"'), "values must survive");
    assert.ok(!text.includes("switch"), "no switch fragment may remain");
  });

  test("brace expr braces an unbraced expr command", async () => {
    const editor = await loadScratch("expr $a + $b\n");
    const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 0));
    const action = await findAction(range, "Brace expr");
    assert.ok(action.edit, "brace-expr action must carry an edit");
    await vscode.workspace.applyEdit(action.edit!);
    assert.strictEqual(editor.document.getText(), "expr {$a + $b}\n");
  });

  // -- Extract into proc (issue #1201) --------------------------------------

  test("extract proc carries a caller-frame write through upvar", async () => {
    // Original output is `1` then `after=1`. Passing `x` as a value parameter
    // moved the write into a proc local and printed `after=0`; the extraction
    // now takes the variable by name and re-binds it with `upvar 1`.
    const editor = await loadScratch('set x 0\nset x 1\nputs $x\nputs "after=$x"\n');
    const range = new vscode.Range(new vscode.Position(1, 0), new vscode.Position(3, 0));
    const action = await findAction(range, "Extract selection into proc");
    assert.ok(action.edit, "extract-proc action must carry an edit");
    await vscode.workspace.applyEdit(action.edit!);
    const text = editor.document.getText();
    assert.ok(text.includes("upvar 1 $xName x"), `expected upvar plumbing; got:\n${text}`);
    assert.ok(text.includes("extracted_proc x\n"), `call passes the name; got:\n${text}`);
    assert.ok(text.includes('puts "after=$x"'), `tail must survive; got:\n${text}`);
  });

  test("extract proc leaves the file prologue above the new definition", async () => {
    const editor = await loadScratch("package require Tcl\nset x 5\nputs $x\n");
    const range = new vscode.Range(new vscode.Position(2, 0), new vscode.Position(3, 0));
    const action = await findAction(range, "Extract selection into proc");
    await vscode.workspace.applyEdit(action.edit!);
    const text = editor.document.getText();
    assert.ok(
      text.indexOf("package require Tcl") < text.indexOf("proc extracted_proc"),
      `the definition must not land above the prologue; got:\n${text}`,
    );
  });

  test("extract proc refuses a selection containing return", async () => {
    // A refused refactoring is offered *greyed out* with its reason, so the
    // user can tell "cannot be done here" from "is broken".
    await loadScratch("proc f {} {\n    set x 1\n    return $x\n}\n");
    const range = new vscode.Range(new vscode.Position(2, 0), new vscode.Position(3, 0));
    const action = await findAction(range, "Extract selection into proc");
    assert.ok(action.disabled, "a return in the selection must be refused");
    assert.ok(
      action.disabled!.reason.includes("call frame"),
      `reason should name the call frame; got: ${action.disabled!.reason}`,
    );
  });

  // -- Inline proc binding (issue #1199) ------------------------------------

  test("inline proc binds a declared default when the call omits the argument", async () => {
    const editor = await loadScratch("proc greet {{name world}} { puts $name }\ngreet\n");
    const range = new vscode.Range(new vscode.Position(1, 0), new vscode.Position(1, 0));
    const action = await findAction(range, "Inline proc");
    assert.ok(action.edit, "inline-proc action must carry an edit");
    await vscode.workspace.applyEdit(action.edit!);
    assert.strictEqual(
      editor.document.getText(),
      "proc greet {{name world}} { puts $name }\nputs world\n",
    );
  });

  test("inline proc refuses a braced argument whose value is not a plain word", async () => {
    // `greet {a b}` passes the value `a b`; splicing the written `{a b}` made
    // the body print the braces.
    await loadScratch('proc greet {name} { puts "hello $name" }\ngreet {a b}\n');
    const range = new vscode.Range(new vscode.Position(1, 0), new vscode.Position(1, 0));
    const action = await findAction(range, "Inline proc");
    assert.ok(action.disabled, "a non-plain-word value must be refused");
    assert.ok(
      action.disabled!.reason.includes("plain word"),
      `reason should explain the value; got: ${action.disabled!.reason}`,
    );
  });

  // -- W302 catch-result capture (issue #1190) ------------------------------
  //
  // The cursor sits on the `catch` keyword — where the diagnostic anchors,
  // and where VS Code's lightbulb is invoked. The inserted word must still
  // land after the *body*. `catch result {error oops}` is a catch of the
  // script `result`, which C Tcl 9 reports as `invalid command name
  // "result"`, so asserting the applied document (not the inserted string)
  // is what makes this test meaningful.

  test("catch result quick fix appends after a single-line body", async () => {
    const editor = await loadScratch("catch {error oops}\n");
    const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 0));
    const action = await findAction(range, "Add catch result variable");
    assert.ok(action.edit, "catch-result action must carry an edit");
    await vscode.workspace.applyEdit(action.edit!);
    assert.strictEqual(editor.document.getText(), "catch {error oops} result\n");
  });

  test("catch result quick fix appends after a multi-line body", async () => {
    const editor = await loadScratch("catch {\n    error oops\n}\n");
    const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 0));
    const action = await findAction(range, "Add catch result variable");
    assert.ok(action.edit, "catch-result action must carry an edit");
    await vscode.workspace.applyEdit(action.edit!);
    assert.strictEqual(editor.document.getText(), "catch {\n    error oops\n} result\n");
  });

  test("catch result + options quick fix appends both words after the body", async () => {
    const editor = await loadScratch("catch {error oops}\n");
    const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 0));
    const action = await findAction(range, "Add catch result + options");
    assert.ok(action.edit, "catch-result+options action must carry an edit");
    await vscode.workspace.applyEdit(action.edit!);
    assert.strictEqual(editor.document.getText(), "catch {error oops} result options\n");
  });

  test("catch result quick fix does not overshoot an empty body", async () => {
    // `catch {}`'s brace pair is a degenerate case whose token span already
    // covers its own closer; a naive `end + 1` anchor writes past it.
    const editor = await loadScratch("catch {}\n");
    const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 0));
    const action = await findAction(range, "Add catch result variable");
    assert.ok(action.edit, "catch-result action must carry an edit");
    await vscode.workspace.applyEdit(action.edit!);
    assert.strictEqual(editor.document.getText(), "catch {} result\n");
  });
});
