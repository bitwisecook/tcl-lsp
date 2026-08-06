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

// Issue #923 differential audit, findings idx 92 and idx 48, through a real
// VS Code session.  The analyser and native lsp_e2e suites carry the deep
// TP/FP/TN matrices; these prove the same answers arrive over the client's own
// providers, which is the tier both findings were missing.
//
// idx 92 — `trace add variable S(size) write [namespace code [list Tracer]]`,
// the shape Tk's own library/fontchooser.tcl writes.  Oracle, tclsh 8.6.16 and
// 9.0.4:
//
//     TRACED: ::demo::S size write
//     info: {write {::namespace inscope ::demo Tracer}}
//     final S(size)=99
//
// The handler really is dispatched through the wrapped `[list …]` prefix, and
// `trace remove` with the identical wrapper really does detach it — so both
// sites are genuine references to the proc.
//
// idx 48 — a query anchored on a bare declaring token.  Oracle, tclsh 8.6.16
// and 9.0.4: `set y 10; puts $y; puts $y` prints `10` twice, and
// `foreach cmd {alpha beta}` prints `cmd is alpha` / `cmd is beta`.

import * as assert from "assert";
import * as vscode from "vscode";
import { activate, getDocUri, pollUntil } from "./helper";

/** Reference start lines for the symbol at (line, character). */
async function referenceLines(uri: vscode.Uri, line: number, character: number): Promise<number[]> {
  const locations = (await vscode.commands.executeCommand(
    "vscode.executeReferenceProvider",
    uri,
    new vscode.Position(line, character),
  )) as vscode.Location[] | undefined;
  return (locations ?? [])
    .filter((l) => l.uri.fsPath === uri.fsPath)
    .map((l) => l.range.start.line)
    .sort((a, b) => a - b);
}

/** The reference-count lens title anchored on `line`, once resolved. */
async function lensTitleOnLine(uri: vscode.Uri, line: number): Promise<string | undefined> {
  const lenses = (await pollUntil(
    () => vscode.commands.executeCommand("vscode.executeCodeLensProvider", uri, 100),
    (r) => {
      const ls = r as vscode.CodeLens[] | undefined;
      return (
        !!ls &&
        ls.some(
          (l) => l.range.start.line === line && l.command && typeof l.command.title === "string",
        )
      );
    },
    { timeout: 15_000, label: `resolved lens on line ${line}` },
  )) as vscode.CodeLens[];
  return lenses.find((l) => l.range.start.line === line)?.command?.title;
}

suite("Issue #923 idx 92: namespace-code-wrapped trace callbacks", () => {
  const docUri = getDocUri("issue923NamespaceCodeCallback.tcl");

  test("both trace sites are references to the wrapped handler", async () => {
    await activate(docUri);
    // `proc Idx92Tracer` is declared on line 5; the `trace add` wrapper sits on
    // line 11 and the `trace remove` wrapper on line 15.
    const fromDecl = await referenceLines(docUri, 5, 12);
    assert.ok(
      fromDecl.includes(11) && fromDecl.includes(15),
      `idx 92: both [namespace code [list …]] sites must be references, got [${fromDecl}]`,
    );

    // Symmetry: the same query from one of the call sites answers identically.
    const fromCallSite = await referenceLines(docUri, 11, 65);
    assert.deepStrictEqual(
      fromCallSite,
      fromDecl,
      `idx 92: a call-site-anchored query must match the declaration-anchored one`,
    );
  });

  test("the reference lens counts each wrapped callback exactly once", async () => {
    await activate(docUri);
    const title = await lensTitleOnLine(docUri, 5);
    assert.strictEqual(
      title,
      "2 references",
      `idx 92: the trace add + trace remove sites are two references, not more \
(the braced-callback double-count guard) — got "${title}"`,
    );
  });

  test("the wrapped handler is an incoming call from both enclosing procs", async () => {
    await activate(docUri);
    const items = (await vscode.commands.executeCommand(
      "vscode.prepareCallHierarchy",
      docUri,
      new vscode.Position(5, 12),
    )) as vscode.CallHierarchyItem[] | undefined;
    assert.ok(items && items.length > 0, "idx 92: prepareCallHierarchy returned nothing");
    const incoming = (await vscode.commands.executeCommand(
      "vscode.provideIncomingCalls",
      items[0],
    )) as vscode.CallHierarchyIncomingCall[] | undefined;
    const callers = (incoming ?? []).map((c) => c.from.name).sort();
    assert.ok(
      callers.some((n) => n.includes("Idx92Setup")) && callers.some((n) => n.includes("Idx92Done")),
      `idx 92: both enclosing procs must show as callers, got [${callers}]`,
    );
  });
});

suite("Issue #923 idx 48: bare declaring tokens as query anchors", () => {
  const docUri = getDocUri("issue923BareDeclaration.tcl");

  test("a `set` left-hand side answers exactly as a `$name` read does", async () => {
    await activate(docUri);
    // `set idx48Total 10` on line 3; `$idx48Total` reads on lines 4 and 5.
    const fromDecl = await referenceLines(docUri, 3, 4);
    const fromRead = await referenceLines(docUri, 4, 6);
    assert.deepStrictEqual(
      fromDecl,
      fromRead,
      `idx 48: the declaring token must not answer less than a read does`,
    );
    assert.ok(
      fromDecl.includes(3) && fromDecl.includes(4) && fromDecl.includes(5),
      `idx 48: all three occurrences expected, got [${fromDecl}]`,
    );
  });

  test("a `foreach` loop variable answers exactly as its body read does", async () => {
    await activate(docUri);
    // `foreach idx48Cmd {alpha beta} {` on line 6; `$idx48Cmd` on line 7.
    const fromDecl = await referenceLines(docUri, 6, 8);
    const fromRead = await referenceLines(docUri, 7, 20);
    assert.deepStrictEqual(
      fromDecl,
      fromRead,
      `idx 48: the loop variable's declaring token must not answer less than \
its body read does`,
    );
    assert.ok(
      fromDecl.includes(6) && fromDecl.includes(7),
      `idx 48: declaration and body read expected, got [${fromDecl}]`,
    );
  });
});
