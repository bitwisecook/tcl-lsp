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
import { activate, getDocUri, getServerLog, sleep } from "./helper";

async function completionLabels(uri: vscode.Uri, position: vscode.Position): Promise<string[]> {
  const result = (await vscode.commands.executeCommand(
    "vscode.executeCompletionItemProvider",
    uri,
    position,
  )) as vscode.CompletionList;

  return result.items.map((item) =>
    typeof item.label === "string" ? item.label : item.label.label,
  );
}

/**
 * Poll completions until a predicate is satisfied or timeout expires.
 * Dialect switches propagate asynchronously from the extension to the
 * server, so we need to retry until the server has processed the
 * configuration change notification.
 */
async function waitForCompletions(
  uri: vscode.Uri,
  position: vscode.Position,
  predicate: (labels: string[]) => boolean,
  timeout = 30_000,
): Promise<string[]> {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    const labels = await completionLabels(uri, position);
    if (predicate(labels)) {
      return labels;
    }
    await sleep(250);
  }
  // Return last result so the assertion can produce a useful message.
  return completionLabels(uri, position);
}

suite("Dialect Detection", () => {
  test("defaults .tcl files to Tcl 8.6", async () => {
    const uri = getDocUri("dialect-default.tcl");
    await activate(uri);

    // Use waitForCompletions because the server may still be processing
    // a dialect change from a previous test suite.
    const labels = await waitForCompletions(uri, new vscode.Position(1, 2), (l) =>
      l.includes("try"),
    );
    assert.ok(labels.includes("try"), 'Expected "try" completion for default tcl8.6 dialect');
  });

  test("uses shebang tclshX.X hint for Tcl version", async () => {
    const uri = getDocUri("dialect-shebang85.tcl");
    await activate(uri);

    // Dialect detection sends a config notification asynchronously; poll until
    // the server reflects the tclsh8.5 dialect.  Gate on a POSITIVE settle
    // signal (`trace`, a real 8.0+ command present in 8.5 and matching the `tr`
    // prefix) as well as the negative, so an empty/transient completion set —
    // which trivially satisfies a bare `!includes("try")` — cannot let this
    // pass vacuously while proving nothing about the 8.5 catalog.
    const labels = await waitForCompletions(
      uri,
      new vscode.Position(1, 2),
      (l) => l.includes("trace") && !l.includes("try"),
    );
    assert.ok(
      labels.includes("trace"),
      'Expected "trace" — the tclsh8.5 command catalog must have loaded',
    );
    assert.ok(!labels.includes("try"), 'Did not expect "try" completion for shebang tclsh8.5');
  });

  test("maps .irul extension to f5-irules", async () => {
    const uri = getDocUri("dialect.irul");
    const doc = await activate(uri);

    const labels = await waitForCompletions(uri, new vscode.Position(2, 11), (l) =>
      l.includes("HTTP::header"),
    );
    const httpLabels = labels.filter((l) => l.startsWith("HTTP::"));
    const dialectLines = getServerLog()
      .filter((m) => /Auto-switch|Switched|Dialect|dialect/.test(m))
      .slice(-8);
    assert.ok(
      labels.includes("HTTP::header"),
      `Expected "HTTP::header" completion for .irule file ` +
        `(languageId=${doc.languageId}, totalLabels=${labels.length}, ` +
        `httpLabels=${JSON.stringify(httpLabels.slice(0, 5))}, ` +
        `recentServerDialectLog=${JSON.stringify(dialectLines)})`,
    );
  });

  test("maps .iapp extension to f5-iapps", async () => {
    const uri = getDocUri("dialect.iapp");
    const doc = await activate(uri);

    const labels = await waitForCompletions(uri, new vscode.Position(1, 6), (l) =>
      l.includes("iapp::template"),
    );
    const iappLabels = labels.filter((l) => l.startsWith("iapp::"));
    const dialectLines = getServerLog()
      .filter((m) => /Auto-switch|Switched|Dialect|dialect/.test(m))
      .slice(-8);
    assert.ok(
      labels.includes("iapp::template"),
      `Expected "iapp::template" completion for .iapp file ` +
        `(languageId=${doc.languageId}, totalLabels=${labels.length}, ` +
        `iappLabels=${JSON.stringify(iappLabels.slice(0, 5))}, ` +
        `recentServerDialectLog=${JSON.stringify(dialectLines)})`,
    );
  });

  test("maps .exp extension to expect", async () => {
    const uri = getDocUri("dialect.exp");
    await activate(uri);

    const labels = await waitForCompletions(uri, new vscode.Position(2, 0), (l) =>
      l.includes("spawn"),
    );
    assert.ok(labels.includes("spawn"), 'Expected "spawn" completion for .exp file');
  });

  test("uses # tcl-dialect: comment directive for Tcl version", async () => {
    const uri = getDocUri("dialect-directive84.tcl");
    await activate(uri);

    // Dialect detection sends a config notification asynchronously;
    // poll until the server reflects the tcl8.4 dialect (no "try").
    const labels = await waitForCompletions(
      uri,
      new vscode.Position(1, 2),
      (l) => !l.includes("try"),
    );
    assert.ok(!labels.includes("try"), 'Did not expect "try" completion for tcl-dialect tcl8.4');
  });
});
