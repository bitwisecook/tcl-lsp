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

// Issue #1296, editor-integration layer: a class made by a cross-file
// metaclass that was itself made by *another* file's metaclass must resolve
// from a call site.
//
// The workspace class-factory index (issue #1276) is computed from a query
// that reads the index, so one publish only advances the metaclass chain by a
// single link. Four documents — `MetaA`, `MetaA create MetaB`,
// `MetaB create Widget`, and a call on `Widget`'s method — need three, and
// go-to-definition on the method came back empty. The server now drives the
// publish to a fixpoint.
//
// This tier exists because the fault is *when* the answer arrives, not only
// what it is: the index converges across several debounced publishes, and a
// real client asks its provider whenever it likes. The deep TP/FP/TN coverage
// lives in the salsa (`rust/tcl-lsp-db/tests/class_factory_fixpoint.rs`),
// analyser (`rust/tcl-compiler/tests/analyser.rs`, `mod class_factories`), and
// native e2e (`rust/tcl-lsp-server/tests/e2e/issue1296_metaclass_chain.rs`)
// suites.
//
// C-Tcl ground truth (tclsh 8.6.16 and 9.0.4, identical), sourcing the four
// fixtures in order: `info class methods ::MC::Widget` -> `go`, and
// `puts [$w go]` -> `went`. So the definition request has a real answer.

import * as assert from "assert";
import * as vscode from "vscode";
import { activate, getDocUri, waitForProviderResult } from "./helper";

/** The chain's documents, outermost metaclass first. */
const CHAIN = ["issue1296MetaA.tcl", "issue1296MetaB.tcl", "issue1296Widget.tcl"];

/** `go` in `puts [$w go]` — line 1, character 10 of the call fixture. */
const CALL_SITE = new vscode.Position(1, 10);

async function definitionsAt(
  docUri: vscode.Uri,
  position: vscode.Position,
): Promise<vscode.Location[]> {
  const result = (await vscode.commands.executeCommand(
    "vscode.executeDefinitionProvider",
    docUri,
    position,
  )) as (vscode.Location | vscode.LocationLink)[] | undefined;
  return (result ?? []).map((item) =>
    "targetUri" in item
      ? new vscode.Location(item.targetUri, item.targetSelectionRange ?? item.targetRange)
      : item,
  );
}

suite("Issue #1296 chained cross-file metaclass", () => {
  test("a method on a class made by a twice-removed metaclass resolves", async () => {
    // Open every link of the chain, then the call site, so the workspace the
    // server publishes its factory index over is the ticket's exact one.
    for (const name of CHAIN) {
      await activate(getDocUri(name));
    }
    const callUri = getDocUri("issue1296Call.tcl");
    await activate(callUri);

    // The index converges over a few debounced publishes, so wait on the
    // *result* rather than sampling once — `waitForProviderResult` re-probes on
    // this document's diagnostics events with a bounded backstop, and fails at
    // its bound if the answer genuinely never arrives.
    const found = await waitForProviderResult(
      callUri,
      () => definitionsAt(callUri, CALL_SITE),
      (locations) => locations.length > 0,
      {
        label: "`$w go` resolving through the chained metaclass",
        timeout: 30_000,
        describe: (locations) => `saw ${JSON.stringify((locations ?? []).map((l) => l.uri.path))}`,
      },
    );

    assert.strictEqual(found.length, 1, `exactly one target: ${JSON.stringify(found)}`);
    assert.ok(
      found[0].uri.toString().endsWith("issue1296Widget.tcl"),
      `the target is the document that writes the class: ${found[0].uri.toString()}`,
    );
    assert.strictEqual(
      found[0].range.start.line,
      0,
      `and the method's own line: ${JSON.stringify(found[0].range)}`,
    );
  });

  test("a metaclass nothing proves still resolves to nothing", async () => {
    // TN. Nothing in the workspace shows `::U::Meta` manufactures classes, so
    // `::U::Widget` is not a class and its supposed method has no declaration.
    // Guessing one would be worse than abstaining, and this is the assertion
    // that stops the fixpoint being "widened" into inventing entries.
    const docUri = getDocUri("issue1296Unproved.tcl");
    await activate(docUri);

    const found = await definitionsAt(docUri, new vscode.Position(3, 10));
    assert.strictEqual(
      found.length,
      0,
      `no file proves \`::U::Meta\` is a class factory: ${JSON.stringify(found)}`,
    );
  });
});
