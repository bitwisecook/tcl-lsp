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
import { LanguageClient } from "vscode-languageclient/node";
import { activate, getDocUri, waitForFeatureToggle } from "./helper";

interface TclLspApi {
  getClient(): LanguageClient;
}

suite("Linked Editing Range", () => {
  const docUri = getDocUri("procs.tcl");

  // editor.linkedEditing defaults to false in VS Code, so the tri-state
  // null default inherits "off".  Explicitly enable for these tests.
  suiteSetup(async () => {
    // Activate first so the extension's `tcl-lsp.getEffectiveConfig` command is
    // registered for the toggle wait below.
    await activate(docUri);
    const cfg = vscode.workspace.getConfiguration("tclLsp.features");
    await cfg.update("linkedEditingRange", true, undefined);
    // Message-passing wait on the server's resolved config (throws on timeout as
    // a backstop) instead of racing a fixed sleep against the async
    // didChangeConfiguration round-trip — the enable is load-bearing (the
    // provider is off until the server sees it), so a slow apply under parallel
    // load could otherwise make the recursive-fib test flake.
    await waitForFeatureToggle(docUri, "linkedEditingRange", true, { timeout: 20_000 });
  });

  suiteTeardown(async () => {
    const cfg = vscode.workspace.getConfiguration("tclLsp.features");
    await cfg.update("linkedEditingRange", undefined, undefined);
  });

  test("server advertises linkedEditingRangeProvider", async () => {
    await activate(docUri);
    const ext = vscode.extensions.getExtension<TclLspApi>("bitwisecook.tcl-lsp")!;
    const client = ext.exports.getClient();
    const caps = client.initializeResult?.capabilities as Record<string, unknown> | undefined;
    assert.ok(caps, "Server should report capabilities");
    assert.ok(caps.linkedEditingRangeProvider, "linkedEditingRangeProvider should be present");
  });

  test("links a recursive proc name with its self-calls", async () => {
    await activate(docUri);
    const ext = vscode.extensions.getExtension<TclLspApi>("bitwisecook.tcl-lsp")!;
    const client = ext.exports.getClient();

    // `fib` declaration at 0-indexed line 1 col 6 — fib recurses twice in
    // its own body so we expect at least 2 ranges back (decl + self-call).
    const result = (await client.sendRequest<{
      ranges?: Array<{ start: { line: number; character: number } }>;
    } | null>("textDocument/linkedEditingRange", {
      textDocument: { uri: docUri.toString() },
      position: { line: 1, character: 6 },
    })) as { ranges?: Array<{ start: { line: number; character: number } }> } | null;

    assert.ok(result, "Expected linked editing ranges for recursive fib");
    assert.ok(
      result.ranges && result.ranges.length >= 2,
      `Expected at least 2 linked ranges (decl + recursive call), got ${result.ranges?.length}`,
    );
  });
});
