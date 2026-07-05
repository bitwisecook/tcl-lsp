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
import { activate, getDocUri } from "./helper";

interface TclLspApi {
  getClient(): LanguageClient;
}

suite("Workspace File Operations", () => {
  test("server advertises willRename/didRename capabilities", async () => {
    await activate(getDocUri("simple.tcl"));
    const ext = vscode.extensions.getExtension<TclLspApi>("bitwisecook.tcl-lsp")!;
    assert.ok(ext.isActive, "Extension should be active");
    const client = ext.exports.getClient();
    const caps = client.initializeResult?.capabilities as Record<string, unknown> | undefined;
    assert.ok(caps, "Server should report capabilities");
    const workspace = caps.workspace as Record<string, unknown> | undefined;
    const fileOps = workspace?.fileOperations as Record<string, unknown> | undefined;
    assert.ok(fileOps, "Should have workspace.fileOperations");
    assert.ok(fileOps.willRename, "Should advertise willRename file-operations");
    assert.ok(fileOps.didRename, "Should advertise didRename file-operations");

    // Validate the glob filter covers Tcl file extensions.
    const willRename = fileOps.willRename as { filters?: Array<{ pattern?: { glob?: string } }> };
    assert.ok(
      willRename.filters && willRename.filters.length >= 1,
      "Should have at least one filter",
    );
    const globs = willRename.filters.map((f) => f.pattern?.glob ?? "").join("|");
    assert.ok(/tcl/.test(globs), `Expected filter glob to include 'tcl', got ${globs}`);
  });
});
