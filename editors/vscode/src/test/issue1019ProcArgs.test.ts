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

// Issue #1019 differential audit, `proc_args` group — the editor tier.
//
// The findings below were verified against tclsh 9.0.4 and 8.6.16 (which
// agree byte-for-byte) and are pinned in the Rust unit and end-to-end tiers.
// What these tests add is the property only the editor can show: that the
// behaviour survives the real VS Code request path — including the one that
// matters most, a *rename the editor must refuse* rather than silently apply.

import * as assert from "assert";
import * as vscode from "vscode";
import {
  getDocUri,
  activate,
  getServerLogSize,
  waitForDeepDiagnostics,
  waitForDiagnostics,
} from "./helper";

function codeOf(d: vscode.Diagnostic): string {
  return typeof d.code === "object" && d.code !== null ? String(d.code.value) : String(d.code);
}

async function renameAt(
  uri: vscode.Uri,
  pos: vscode.Position,
  newName: string,
): Promise<vscode.WorkspaceEdit | undefined> {
  return (await vscode.commands.executeCommand(
    "vscode.executeDocumentRenameProvider",
    uri,
    pos,
    newName,
  )) as vscode.WorkspaceEdit | undefined;
}

// -- idx 79 ------------------------------------------------------------
//
// `$other` really holds an `Idx79Vector` at run time, but nothing in the
// source *assigns* a constructor result to it, so no analysis can prove the
// dispatch reaches `X` — nor prove it does not.  Renaming `X` and leaving
// `[$other X]` behind makes both interpreters throw
// `unknown method "X": must be Get, GetX or destroy`.
//
// The gate used to be reachable only from the method's own declaration.  VS
// Code's rename gesture works from any occurrence of the symbol, so the two
// positions below are the *common* ones — and they must refuse too.
suite("Issue #1019 idx 79 — rename refuses an untracked dispatch", () => {
  const docUri = getDocUri("issue1019Idx79Hazard.tcl");

  // Cursor on the `X` of `[$other X]` (line 5) — the hazardous dispatch.
  test("refuses when triggered from the untracked call site", async () => {
    await activate(docUri);
    await assert.rejects(
      () => renameAt(docUri, new vscode.Position(5, 27), "GetX"),
      "renaming from the untracked call site must be refused, not applied",
    );
  });

  // Cursor on the `X` of `export X Get` (line 12).
  test("refuses when triggered from the export bareword", async () => {
    await activate(docUri);
    await assert.rejects(
      () => renameAt(docUri, new vscode.Position(12, 11), "GetX"),
      "renaming from the export list must be refused, not applied",
    );
  });

  // Cursor on the `X` of `method X` (line 10) — the direction that already
  // worked, kept so a regression that loosened the gate shows up here too.
  test("refuses when triggered from the declaration", async () => {
    await activate(docUri);
    await assert.rejects(
      () => renameAt(docUri, new vscode.Position(10, 11), "GetX"),
      "renaming from the declaration must be refused, not applied",
    );
  });

  // TN control: a member the untracked receiver never names still renames.
  // Over-refusal would make rename unusable on any class with an internal
  // `$var method` helper.
  test("still renames a member the untracked receiver never names", async () => {
    await activate(docUri);
    const edit = await renameAt(docUri, new vscode.Position(11, 11), "Fetch");
    assert.ok(edit, "renaming `Get` must succeed");
    const entries = edit.entries();
    const found = entries.find(([uri]) => uri.toString() === docUri.toString());
    assert.ok(found, "the document must be edited");
    assert.ok(
      found[1].length >= 2,
      `declaration + export must both rename, got ${found[1].length} edits`,
    );
  });
});

// -- idx 28 ------------------------------------------------------------
//
// SpiceGenTcl's shape: the mixin and the class that mixes it in live in one
// file, the device class calling `my ArgsPreprocess` in another.  Oracle
// (9.0.4 / 8.6.16, sourcing both): `ArgsPreprocess dispatched via class:
// ::Idx28::Utility` — the call genuinely reaches the mixin's method.
//
// Go-to-definition crossed the file boundary; hover answered nothing.
suite("Issue #1019 idx 28 — cross-file mixin hover", () => {
  const deviceUri = getDocUri("issue1019Idx28Device.tcl");
  const libUri = getDocUri("issue1019Idx28Lib.tcl");

  test("hover on `my <method>` names the provider in the sibling file", async () => {
    await activate(libUri);
    await activate(deviceUri);
    // Cursor on `ArgsPreprocess` in the `my` dispatch (line 4).
    const hovers = (await vscode.commands.executeCommand(
      "vscode.executeHoverProvider",
      deviceUri,
      new vscode.Position(4, 20),
    )) as vscode.Hover[];
    const text = (hovers ?? [])
      .flatMap((h) => h.contents)
      .map((c) => (typeof c === "string" ? c : (c as vscode.MarkdownString).value))
      .join("\n");
    assert.ok(text.includes("ArgsPreprocess"), `hover must name the method, got: ${text}`);
    assert.ok(
      text.includes("::Idx28::Utility"),
      `hover must name the providing class from the sibling file, got: ${text}`,
    );
  });

  test("hover and go-to-definition agree about the provider", async () => {
    await activate(libUri);
    await activate(deviceUri);
    const locations = (await vscode.commands.executeCommand(
      "vscode.executeDefinitionProvider",
      deviceUri,
      new vscode.Position(4, 20),
    )) as vscode.Location[];
    assert.ok(locations && locations.length > 0, "definition must resolve cross-file");
    assert.strictEqual(
      locations[0].uri.toString(),
      libUri.toString(),
      "the declaration lives in the library file",
    );
  });
});

// -- idx 11 ------------------------------------------------------------
//
// A `proc` named after a *package-gated* registry command is the package's
// own definition, not a redefinition of a built-in — and needs no `package
// require` to make its name exist.  Oracle (9.0.4 / 8.6.16): `argparse` is
// not a built-in (`invalid command name "argparse"` in a bare interpreter),
// and the file prints `case0`.
suite("Issue #1019 idx 11 — a proc named after a package command", () => {
  const docUri = getDocUri("issue1019Idx11Argparse.tcl");

  test("fires neither W113 nor W120 nor an arity error (false case)", async () => {
    // The document is clean, so there is no diagnostic to wait *for*.  Waiting
    // out a deadline with a predicate that can never hold is the anti-pattern
    // issue #1274 is about: it costs the full deadline on every run even when
    // the server settled in milliseconds, and it proves nothing — an empty set
    // satisfies all three negative assertions below whether analysis ran or
    // not.  The server's deep-diagnostics marker is the actual signal that the
    // pass completed for this URI, and it fires on a clean file too
    // (`diags=0`), so this returns as soon as the work is done and throws if it
    // never happens.
    const since = getServerLogSize();
    await activate(docUri);
    await waitForDeepDiagnostics(docUri, { since });
    const diags = vscode.languages.getDiagnostics(docUri);
    const seen = diags.map(codeOf);
    assert.ok(!seen.includes("W113"), `the proc shadows no built-in: ${seen.join(", ")}`);
    assert.ok(!seen.includes("W120"), `the file defines the command itself: ${seen.join(", ")}`);
    assert.ok(!seen.includes("E002"), `the 0-argument call runs (\`case0\`): ${seen.join(", ")}`);
  });

  // TP control: redefining a real core built-in still warns — the exclusion
  // is for package-gated names only, not a blanket one.
  test("redefining a core built-in still fires W113 (true case)", async () => {
    const builtinUri = getDocUri("procs.tcl");
    await activate(builtinUri);
    const doc = await vscode.workspace.openTextDocument(builtinUri);
    const edit = new vscode.WorkspaceEdit();
    edit.insert(builtinUri, new vscode.Position(0, 0), "proc lindex {a b} { return $a }\n");
    await vscode.workspace.applyEdit(edit);
    try {
      const diags = await waitForDiagnostics(builtinUri, {
        predicate: (d) => d.some((x) => codeOf(x) === "W113"),
      });
      assert.ok(
        diags.some((d) => codeOf(d) === "W113"),
        "`lindex` is a core built-in and must still be flagged",
      );
    } finally {
      const undo = new vscode.WorkspaceEdit();
      undo.delete(builtinUri, doc.lineAt(0).rangeIncludingLineBreak);
      await vscode.workspace.applyEdit(undo);
      await vscode.commands.executeCommand("workbench.action.closeActiveEditor");
    }
  });
});

// -- idx 104 -----------------------------------------------------------
//
// Tk's `library/tk.tcl` shape: a parameter named `destroy`, defaulting to the
// literal `destroy`, beside a `destroy` command.  Oracle (9.0.4 / 8.6.16):
// `$destroy` always reads the parameter, bare `destroy $grab` always calls
// the command.
suite("Issue #1019 idx 104 — a parameter shadowing a command", () => {
  const docUri = getDocUri("issue1019Idx104Param.tcl");

  test("the parameter's own name resolves to the parameter, not the proc", async () => {
    await activate(docUri);
    const locations = (await vscode.commands.executeCommand(
      "vscode.executeDefinitionProvider",
      docUri,
      new vscode.Position(1, 38),
    )) as vscode.Location[];
    assert.ok(locations && locations.length > 0, "the parameter must resolve to something");
    assert.strictEqual(
      locations[0].range.start.line,
      1,
      "it must be the parameter's own binding on line 1, not `proc destroy` on line 0",
    );
  });

  test("the real call in the body still reaches the proc (false-positive guard)", async () => {
    await activate(docUri);
    const locations = (await vscode.commands.executeCommand(
      "vscode.executeDefinitionProvider",
      docUri,
      new vscode.Position(2, 44),
    )) as vscode.Location[];
    assert.ok(locations && locations.length > 0, "the bare call must resolve");
    assert.strictEqual(locations[0].range.start.line, 0, "it must reach `proc destroy`");
  });
});
