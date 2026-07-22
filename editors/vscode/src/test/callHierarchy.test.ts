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

suite("Call Hierarchy", () => {
  const docUri = getDocUri("procs.tcl");

  test("prepareCallHierarchy returns items for a proc definition", async () => {
    await activate(docUri);

    // 'fib' at line 1, col 5
    const pos = new vscode.Position(1, 5);

    const items = (await vscode.commands.executeCommand(
      "vscode.prepareCallHierarchy",
      docUri,
      pos,
    )) as vscode.CallHierarchyItem[] | undefined;

    assert.ok(items, "prepareCallHierarchy should return items for a proc");
    assert.ok(items.length > 0, "Should return at least one call hierarchy item");
    assert.ok(items[0].name.includes("fib"), `First item should be 'fib', got '${items[0].name}'`);
  });

  test("incoming calls includes recursive self-call", async () => {
    await activate(docUri);

    const pos = new vscode.Position(1, 5);

    const items = (await vscode.commands.executeCommand(
      "vscode.prepareCallHierarchy",
      docUri,
      pos,
    )) as vscode.CallHierarchyItem[];

    assert.ok(items && items.length > 0, "prepareCallHierarchy must return items");

    const incoming = (await vscode.commands.executeCommand(
      "vscode.provideIncomingCalls",
      items[0],
    )) as vscode.CallHierarchyIncomingCall[] | undefined;

    assert.ok(incoming, "Should return incoming calls");

    // fib calls itself recursively, so it should appear in its own incoming calls
    if (incoming.length > 0) {
      const selfCall = incoming.find((call) => call.from.name.includes("fib"));
      assert.ok(selfCall, "fib should have a recursive incoming call from itself");
    }
  });

  test("outgoing calls for fib includes self-reference", async () => {
    await activate(docUri);

    const pos = new vscode.Position(1, 5);

    const items = (await vscode.commands.executeCommand(
      "vscode.prepareCallHierarchy",
      docUri,
      pos,
    )) as vscode.CallHierarchyItem[];

    assert.ok(items && items.length > 0, "prepareCallHierarchy must return items");

    const outgoing = (await vscode.commands.executeCommand(
      "vscode.provideOutgoingCalls",
      items[0],
    )) as vscode.CallHierarchyOutgoingCall[] | undefined;

    assert.ok(outgoing, "Should return outgoing calls");

    if (outgoing.length > 0) {
      const selfCall = outgoing.find((call) => call.to.name.includes("fib"));
      assert.ok(selfCall, "fib should have a recursive outgoing call to itself");
    }
  });

  // Regression for issue #957's general form: TclOO method call-hierarchy
  // edges must match `my <method>` dispatch (including nested in `if`
  // control flow), never a bare method-name call — a method is not a
  // bare-callable command in Tcl, so a bare `greet` head never dispatches.
  suite("TclOO method dispatch (`my <method>`)", () => {
    const methodDocUri = getDocUri("methodCallHierarchy.tcl");

    test("incoming calls for a method match `my <method>` dispatch, including control-flow-nested sites", async () => {
      await activate(methodDocUri);

      // `greet` declaration — line 1, col 11.
      const pos = new vscode.Position(1, 11);
      const items = (await vscode.commands.executeCommand(
        "vscode.prepareCallHierarchy",
        methodDocUri,
        pos,
      )) as vscode.CallHierarchyItem[] | undefined;

      assert.ok(items && items.length > 0, "prepareCallHierarchy must return items for `greet`");

      const incoming = (await vscode.commands.executeCommand(
        "vscode.provideIncomingCalls",
        items[0],
      )) as vscode.CallHierarchyIncomingCall[] | undefined;

      assert.ok(incoming, "Should return incoming calls");
      const twiceCall = incoming?.find((call) => call.from.name.includes("twice"));
      assert.ok(
        twiceCall,
        `twice should dispatch to greet via 'my greet', got ${JSON.stringify(incoming)}`,
      );
      // Two `my greet` sites — one nested inside `if`, one top-level.
      assert.strictEqual(
        twiceCall?.fromRanges.length,
        2,
        `expected both my-dispatch sites, got ${JSON.stringify(twiceCall?.fromRanges)}`,
      );
    });

    test("outgoing calls from a method include a `my <method>` site nested inside `if`", async () => {
      await activate(methodDocUri);

      // `twice` declaration — line 2, col 11.
      const pos = new vscode.Position(2, 11);
      const items = (await vscode.commands.executeCommand(
        "vscode.prepareCallHierarchy",
        methodDocUri,
        pos,
      )) as vscode.CallHierarchyItem[] | undefined;

      assert.ok(items && items.length > 0, "prepareCallHierarchy must return items for `twice`");

      const outgoing = (await vscode.commands.executeCommand(
        "vscode.provideOutgoingCalls",
        items[0],
      )) as vscode.CallHierarchyOutgoingCall[] | undefined;

      assert.ok(outgoing, "Should return outgoing calls");
      const greetCall = outgoing?.find((call) => call.to.name.includes("greet"));
      assert.ok(greetCall, `twice should call greet, got ${JSON.stringify(outgoing)}`);
    });
  });
});
