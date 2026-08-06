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
import { getDocUri, activate, pollUntil, waitForDiagnostics } from "./helper";

// Caller-frame injection through `upvar` (issue #923 audit cluster C1 —
// idx 7, 22, 57, 58, 98; issue #1019), through the real editor surface.
//
// Every fact asserted here is pinned on tclsh 9.0.4 and 8.6.16, which agree;
// the transcripts are quoted in `testFixture/callerFrameCluster.tcl` and in
// the compiler-unit twins under `rust/tcl-compiler/tests/caller_frame_effects.rs`
// / `rust/tcl-lsp-core/src/caller_frame.rs`.
//
// The value of this tier is that it exercises the *client-visible* result:
// the extension's own hover / definition / reference plumbing on top of the
// server, which the Rust e2e suite does not run.

function hoverText(hovers: vscode.Hover[]): string {
  return hovers
    .flatMap((h) => h.contents)
    .map((c) => (typeof c === "string" ? c : (c as { value: string }).value))
    .join("\n");
}

async function hoverAt(uri: vscode.Uri, line: number, char: number, label: string) {
  const hovers = (await pollUntil(
    () =>
      vscode.commands.executeCommand(
        "vscode.executeHoverProvider",
        uri,
        new vscode.Position(line, char),
      ),
    (r) => Array.isArray(r) && r.length > 0,
    { timeout: 10_000, label },
  )) as vscode.Hover[];
  return hoverText(hovers);
}

async function definitionsAt(uri: vscode.Uri, line: number, char: number) {
  return (await vscode.commands.executeCommand(
    "vscode.executeDefinitionProvider",
    uri,
    new vscode.Position(line, char),
  )) as (vscode.Location | vscode.LocationLink)[];
}

function startLine(loc: vscode.Location | vscode.LocationLink): number {
  return "range" in loc ? loc.range.start.line : loc.targetRange.start.line;
}

suite("Caller-frame variables (issue #923 audit cluster C1)", () => {
  const docUri = getDocUri("callerFrameCluster.tcl");

  // idx 58 — the wrong-kind conflation. `$dataset` is created by the
  // callee's `upvar`; a sibling `method dataset {}` shares the name, and
  // Tcl keeps variables and commands in disjoint namespaces, so the read
  // can never denote the method.
  test("a caller-frame read never resolves to a same-named method", async () => {
    await activate(docUri);
    // Line 14: `        set _dataset $dataset` — inside `dataset`.
    const text = await hoverAt(docUri, 14, 24, "hover for the idx-58 caller-frame read");
    assert.ok(
      text.includes("Caller-frame variable") && text.includes("gridlayoutHasDataSetObj"),
      `expected the caller-frame card naming the creating callee, got: ${text}`,
    );
    assert.ok(
      !text.includes("method"),
      `a $-led read must never render the same-named method's card, got: ${text}`,
    );

    const defs = await definitionsAt(docUri, 14, 24);
    assert.strictEqual(defs.length, 1, `expected one definition, got ${defs.length}`);
    assert.strictEqual(
      startLine(defs[0]),
      13,
      "go-to-definition must reach the creating call site, not the method declaration",
    );
  });

  // idx 98 — `upvar ::tk::FocusGrab($index) data` names one fixed global
  // cell, so the `upvar` word and the sibling proc's own `$::tk::FocusGrab`
  // read are the same variable. The whole-file analysis answered this while
  // the live server's incremental analysis answered nothing for hover and
  // definition; that residual is what this pins from the client side.
  test("a fully-qualified upvar target hovers, defines and cross-references", async () => {
    await activate(docUri);
    // Line 24: `    upvar ::tk::FocusGrab($index) data` — inside FocusGrab.
    // Line 30: `        set data2 $::tk::FocusGrab($index)` — same.
    for (const [line, char] of [
      [24, 20],
      [30, 26],
    ] as const) {
      const text = await hoverAt(docUri, line, char, `hover for the idx-98 cell at ${line}`);
      assert.ok(
        text.includes("::tk::FocusGrab"),
        `hover at ${line}:${char} must name the cell, got: ${text}`,
      );
      const defs = await definitionsAt(docUri, line, char);
      assert.strictEqual(defs.length, 1, `expected one definition at ${line}:${char}`);
      assert.strictEqual(
        startLine(defs[0]),
        24,
        "the `upvar` otherVar word is where the cell comes to exist",
      );
    }

    const refs = (await vscode.commands.executeCommand(
      "vscode.executeReferenceProvider",
      docUri,
      new vscode.Position(30, 26),
    )) as vscode.Location[];
    const lines = refs.map((r) => r.range.start.line).sort((a, b) => a - b);
    assert.deepStrictEqual(
      lines,
      [24, 29, 30, 31],
      `otherVar word + info exists + read + unset, got: ${JSON.stringify(lines)}`,
    );
  });

  // idx 22 — the callee is a *mixin* method the call site never names
  // statically. The method-resolution-order walk (issues #1177 / #1164)
  // resolves `my NameProcess` through the mixin, so the literal
  // `upvar name name` target is now answerable.
  test("a mixin method reached by `my` dispatch creates this frame's variable", async () => {
    await activate(docUri);
    // Line 48: `        puts "name=$name params=$params"` — inside `name`.
    const text = await hoverAt(docUri, 48, 21, "hover for the idx-22 mixin-dispatch read");
    assert.ok(
      text.includes("Caller-frame variable") && text.includes("NameProcess"),
      `expected the caller-frame card naming the resolved method, got: ${text}`,
    );

    const defs = await definitionsAt(docUri, 48, 21);
    assert.strictEqual(defs.length, 1, `expected one definition, got ${defs.length}`);
    assert.strictEqual(
      startLine(defs[0]),
      46,
      "the `my NameProcess …` dispatch is where the variable comes to exist here",
    );
  });

  // The whole fixture is legitimate Tcl whose every read is really bound at
  // run time, so none of it may draw a read-before-set warning. This is the
  // false-positive half of the cluster (idx 7 / 38 / 57 / 59) seen from the
  // editor: a regression that resurrects any of those FPs shows up here.
  test("none of the cluster's caller-frame reads draws W210", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      timeout: 20_000,
      predicate: () => true,
    });
    const w210 = diags.filter((d) => String(d.code) === "W210");
    assert.strictEqual(
      w210.length,
      0,
      `no read here is read-before-set, got: ${JSON.stringify(w210.map((d) => d.message))}`,
    );
  });
});
