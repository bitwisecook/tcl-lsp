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

suite("Work-Done Progress", () => {
  // Note: there is no `tclLsp.features.progress` setting — the server emits no
  // `$/progress` and nothing consumed the toggle, so the phantom override was
  // removed (issue 201).
  test("server stays responsive during/after workspace scan", async () => {
    // The $/progress pipeline runs asynchronously on the event loop.  If
    // it blocked the event loop the test harness would time out activating
    // the extension.  Reaching this test at all means the scan coroutine
    // did not deadlock.
    await activate(getDocUri("simple.tcl"));
    const ext = vscode.extensions.getExtension<TclLspApi>("bitwisecook.tcl-lsp")!;
    assert.ok(ext.isActive, "Extension should still be active after scan");
    const client = ext.exports.getClient();
    assert.ok(client.initializeResult, "Client should have initialize result");
  });
});
