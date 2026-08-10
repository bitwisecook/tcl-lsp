// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

// Editor-level coverage for registry-driven TclOO factory inference:
// Tk-style unknown dispatch (#1303), unopened factory files (#1304), and a
// metaclass name resolved from a literal proc call (#1306).

import * as assert from "assert";
import * as vscode from "vscode";
import { activate, getDocUri, waitForProviderResult } from "./helper";

const METHOD = new vscode.Position(1, 10);

async function definitionsAt(
  uri: vscode.Uri,
  position: vscode.Position,
): Promise<vscode.Location[]> {
  const result = (await vscode.commands.executeCommand(
    "vscode.executeDefinitionProvider",
    uri,
    position,
  )) as (vscode.Location | vscode.LocationLink)[] | undefined;
  return (result ?? []).map((item) =>
    "targetUri" in item
      ? new vscode.Location(item.targetUri, item.targetSelectionRange ?? item.targetRange)
      : item,
  );
}

async function settledDefinitions(uri: vscode.Uri): Promise<vscode.Location[]> {
  return waitForProviderResult(
    uri,
    () => definitionsAt(uri, METHOD),
    (locations) => locations.length > 0,
    { label: `method definition at ${uri.path}`, timeout: 30_000 },
  );
}

suite("Issues #1303, #1304, and #1306 TclOO factories", () => {
  test("a proved unknown-dispatch constructor types its result", async () => {
    await activate(getDocUri("issue1303Meta.tcl"));
    await activate(getDocUri("issue1303Widget.tcl"));
    const call = getDocUri("issue1303Call.tcl");
    await activate(call);
    const found = await settledDefinitions(call);
    assert.strictEqual(found.length, 1, JSON.stringify(found));
    assert.ok(found[0].uri.path.endsWith("issue1303Widget.tcl"));
  });

  test("an echoing unknown does not invent an object handle", async () => {
    const uri = getDocUri("issue1303Echo.tcl");
    await activate(uri);
    const found = await definitionsAt(uri, new vscode.Position(9, 10));
    assert.strictEqual(found.length, 0, JSON.stringify(found));
  });

  test("unopened metaclass and class files feed an open consumer", async () => {
    const call = getDocUri("issue1304Call.tcl");
    await activate(call);
    const found = await settledDefinitions(call);
    assert.strictEqual(found.length, 1, JSON.stringify(found));
    assert.ok(found[0].uri.path.endsWith("issue1304Widget.tcl"));
  });

  test("a literal call resolves a computed metaclass name", async () => {
    await activate(getDocUri("issue1306Dialect.tcl"));
    await activate(getDocUri("issue1306Widget.tcl"));
    const call = getDocUri("issue1306Call.tcl");
    await activate(call);
    const found = await settledDefinitions(call);
    assert.strictEqual(found.length, 1, JSON.stringify(found));
    assert.ok(found[0].uri.path.endsWith("issue1306Widget.tcl"));
  });
});
