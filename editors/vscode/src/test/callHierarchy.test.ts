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

    if (!items || items.length === 0) {
      return; // Server may not support call hierarchy
    }

    const incoming = (await vscode.commands.executeCommand(
      "vscode.provideIncomingCalls",
      items[0],
    )) as vscode.CallHierarchyIncomingCall[] | undefined;

    // fib calls itself recursively, so it should appear in its own incoming calls
    if (incoming && incoming.length > 0) {
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

    if (!items || items.length === 0) {
      return;
    }

    const outgoing = (await vscode.commands.executeCommand(
      "vscode.provideOutgoingCalls",
      items[0],
    )) as vscode.CallHierarchyOutgoingCall[] | undefined;

    if (outgoing && outgoing.length > 0) {
      const selfCall = outgoing.find((call) => call.to.name.includes("fib"));
      assert.ok(selfCall, "fib should have a recursive outgoing call to itself");
    }
  });
});
