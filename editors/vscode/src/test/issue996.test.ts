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

// Issue #996: the native server crashed (uncatchable SIGABRT) analysing Tcl
// source nested 100-150 levels deep. The deep TP/FP/TN coverage — the exact
// crash mechanism, the depth-cap calibration, every fixed subsystem — lives
// in the Rust unit and native e2e suites (`docs/design/compiler/
// recursive-descent-depth-limits.md`); this proves the same fix (survival,
// not a hang or a dead extension) arrives through a real VS Code session,
// exercising diagnostics, formatting, and the minify command — all three
// independently-fixed recursive walkers reachable from the editor.
//
// `issue996DeepNesting.tcl` (300 levels) is well past every relevant cap
// (the analyser's 256, the formatter's/minifier's 128) without being so
// deep the extension host test times out.

import * as assert from "assert";
import * as vscode from "vscode";
import { LanguageClient } from "vscode-languageclient/node";
import { getDocUri, activate, waitForDiagnostics, scaledTimeout } from "./helper";

interface TclLspApi {
  getClient(): LanguageClient;
}

function getClient(): LanguageClient {
  const ext = vscode.extensions.getExtension("bitwisecook.tcl-lsp")!;
  return (ext.exports as TclLspApi).getClient();
}

async function execLspCommand(command: string, ...args: unknown[]): Promise<unknown> {
  return getClient().sendRequest("workspace/executeCommand", {
    command,
    arguments: args,
  });
}

suite("Issue #996 deep nesting survival", () => {
  const docUri = getDocUri("issue996DeepNesting.tcl");

  test("the server answers diagnostics instead of crashing", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 1, timeout: 30_000 });
    assert.ok(diagnostics.length > 0, "expected at least one diagnostic (E207), got none");
    const codes = diagnostics.map((d) => (typeof d.code === "object" ? d.code.value : d.code));
    assert.ok(
      codes.includes("E207"),
      `expected E207 (nesting depth exceeds analysis limit) in [${codes}]`,
    );

    // The extension host / language client is still alive and responsive to
    // unrelated work afterwards — a crashed server leaves the client hung,
    // which would time out every request from here on, not just this one.
    const simpleUri = getDocUri("simple.tcl");
    await activate(simpleUri);
    await waitForDiagnostics(simpleUri, { minCount: 0, timeout: 10_000 });
  });

  test("formatting the deeply nested document returns edits, not a hang", async function () {
    this.timeout(scaledTimeout(30_000));
    await activate(docUri);
    const edits = (await vscode.commands.executeCommand(
      "vscode.executeFormatDocumentProvider",
      docUri,
      { tabSize: 4, insertSpaces: true } as vscode.FormattingOptions,
    )) as vscode.TextEdit[] | undefined;
    assert.ok(edits, "format-document should return edits, not null/undefined");
  });

  test("tcl-lsp.minifyDocument on the deeply nested document returns a result", async function () {
    this.timeout(scaledTimeout(30_000));
    await activate(docUri);
    const result = (await execLspCommand(
      "tcl-lsp.minifyDocument",
      docUri.toString(),
      false,
      false,
      false,
    )) as { source: string } | null;
    assert.ok(result, "minifyDocument should return a result, not null");
    assert.ok(result.source.length > 0, "expected non-empty minified source");
  });
});
