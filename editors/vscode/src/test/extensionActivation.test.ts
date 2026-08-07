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
import { activate, getDocUri, scaledTimeout } from "./helper";

interface TclLspApi {
  getClient(): LanguageClient;
}

suite("Extension Activation", () => {
  let ext: vscode.Extension<TclLspApi>;

  suiteSetup(async function () {
    this.timeout(scaledTimeout(60_000));
    ext = vscode.extensions.getExtension("bitwisecook.tcl-lsp")!;
    assert.ok(ext, "Extension should be installed");
    if (!ext.isActive) {
      await ext.activate();
    }
    // Ensure the server has fully initialised by opening a fixture.
    await activate(getDocUri("simple.tcl"));
  });

  test("extension is present in the extensions list", () => {
    assert.ok(ext, "Extension should be installed");
  });

  test("extension activates successfully", () => {
    assert.ok(ext.isActive, "Extension should be active");
  });

  test("extension exports getClient function", () => {
    const exports = ext.exports as unknown as Record<string, unknown>;
    assert.ok(exports, "Extension should have exports");
    assert.ok(typeof exports.getClient === "function", "Should export getClient()");
  });

  test("status bar items appear when a Tcl file is active", () => {
    // The status bar items are created during activation.
    // We can't directly inspect them, but we can verify the extension is active
    // and the document is open with the correct language ID.
    const editor = vscode.window.activeTextEditor;
    assert.ok(editor, "Should have an active editor");
    assert.strictEqual(editor.document.languageId, "tcl", "Language should be tcl");
  });

  test("server capabilities include expected providers", () => {
    const client = ext.exports.getClient();
    const caps = client.initializeResult?.capabilities as Record<string, unknown> | undefined;

    assert.ok(caps, "Server should report capabilities");
    assert.ok(caps.hoverProvider, "Should have hoverProvider");
    assert.ok(caps.completionProvider, "Should have completionProvider");
    assert.ok(caps.definitionProvider, "Should have definitionProvider");
    assert.ok(caps.referencesProvider, "Should have referencesProvider");
    assert.ok(caps.documentFormattingProvider, "Should have documentFormattingProvider");
    assert.ok(caps.codeActionProvider, "Should have codeActionProvider");
    assert.ok(caps.documentSymbolProvider, "Should have documentSymbolProvider");
    assert.ok(caps.foldingRangeProvider, "Should have foldingRangeProvider");
    assert.ok(caps.renameProvider, "Should have renameProvider");
    assert.ok(caps.signatureHelpProvider, "Should have signatureHelpProvider");
    assert.ok(caps.workspaceSymbolProvider, "Should have workspaceSymbolProvider");
    assert.ok(caps.documentLinkProvider, "Should have documentLinkProvider");
    assert.ok(caps.selectionRangeProvider, "Should have selectionRangeProvider");
    assert.ok(caps.callHierarchyProvider, "Should have callHierarchyProvider");
    // New LSP features.
    assert.ok(caps.documentHighlightProvider, "Should have documentHighlightProvider");
    assert.ok(caps.implementationProvider, "Should have implementationProvider");
    assert.ok(caps.typeDefinitionProvider, "Should have typeDefinitionProvider");
    assert.ok(caps.declarationProvider, "Should have declarationProvider");
    assert.ok(caps.linkedEditingRangeProvider, "Should have linkedEditingRangeProvider");
    assert.ok(caps.codeLensProvider, "Should have codeLensProvider");
    assert.ok(caps.typeHierarchyProvider, "Should have typeHierarchyProvider");
    // workspace/willRenameFiles + didRenameFiles are surfaced under
    // workspace.fileOperations.
    const workspace = caps.workspace as Record<string, unknown> | undefined;
    const fileOps = workspace?.fileOperations as Record<string, unknown> | undefined;
    assert.ok(fileOps, "Should have workspace.fileOperations");
    assert.ok(fileOps.willRename, "Should have willRename file-operation filter");
    assert.ok(fileOps.didRename, "Should have didRename file-operation filter");
  });

  test("server reports semantic token capabilities", () => {
    const client = ext.exports.getClient();
    const caps = client.initializeResult?.capabilities as Record<string, unknown> | undefined;

    assert.ok(caps, "Server should report capabilities");
    assert.ok(caps.semanticTokensProvider, "Should have semanticTokensProvider capability");
  });
});
