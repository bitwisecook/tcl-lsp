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
import { LanguageClient, State } from "vscode-languageclient/node";
import { activate, bounded, getDocUri, scaledTimeout } from "./helper";

interface TclLspApi {
  getClient(): LanguageClient;
}

function getApi(): TclLspApi {
  const ext = vscode.extensions.getExtension("bitwisecook.tcl-lsp")!;
  return ext.exports as TclLspApi;
}

// Root-level hooks bracket the entire test run.

// Runs before ALL test suites.  If the native tcl-lsp-server binary is
// missing or crashes on startup then ext.activate() rejects because
// client.start() fails, and the whole test run aborts with a clear message.
suiteSetup(async function () {
  this.timeout(scaledTimeout(60_000));
  const ext = vscode.extensions.getExtension("bitwisecook.tcl-lsp")!;
  await ext.activate();
  assert.ok(ext.isActive, "Extension failed to activate – server may have crashed on startup");
});

// Runs after ALL test suites.  Catches server crashes that happen mid-run.
suiteTeardown(async function () {
  this.timeout(scaledTimeout(30_000));
  const client = getApi().getClient();
  assert.strictEqual(
    client.state,
    State.Running,
    `Server should still be Running at end of tests, got state ${client.state}`,
  );
});

// Explicit health-check suite with named tests.

suite("Server Health", () => {
  test("language client is in Running state", () => {
    const client = getApi().getClient();
    assert.strictEqual(
      client.state,
      State.Running,
      `Expected Running (${State.Running}), got ${client.state}`,
    );
  });

  test("server returned capabilities", () => {
    const client = getApi().getClient();
    const result = client.initializeResult as { capabilities: Record<string, unknown> } | undefined;
    assert.ok(result, "Server did not return an InitializeResult");
    assert.ok(result.capabilities, "InitializeResult has no capabilities");
    assert.ok(result.capabilities.hoverProvider, "Server should advertise hoverProvider");
    assert.ok(result.capabilities.completionProvider, "Server should advertise completionProvider");
  });

  test("server responds to hover request on a fixture file", async () => {
    const docUri = getDocUri("simple.tcl");
    await activate(docUri);
    // activate() sends a hover request serialised behind didOpen.
    // Reaching here means the server processed both successfully.
  });

  test("server remains responsive past the transport queue size", async function () {
    this.timeout(scaledTimeout(30_000));
    const docUri = getDocUri("simple.tcl");
    await activate(docUri);
    const client = getApi().getClient();
    // Negative control for the new admission wrapper: ordinary provider work
    // still drains when the burst is larger than the transport's old queue.
    // The Rust transport/E2E fixtures inject the delayed client reply which
    // VS Code's built-in configuration handler does not expose here.
    const requests = Array.from({ length: 240 }, () =>
      client.sendRequest("textDocument/hover", {
        textDocument: { uri: docUri.toString() },
        position: { line: 0, character: 0 },
      }),
    );

    const results = await bounded(Promise.all(requests), "240 concurrent hover responses", {
      timeout: 10_000,
    });
    assert.strictEqual(results.length, 240);
    assert.strictEqual(client.state, State.Running);
  });
});
