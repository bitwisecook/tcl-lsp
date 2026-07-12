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
import {
  getDocUri,
  activateViaWorkbench,
  documentClosed,
  waitForDiagnostics,
  waitForDeepDiagnostics,
  getServerLogSize,
} from "./helper";

// #865 — "File Explorer doesn't seem to retain files with issues": a workspace
// file that was opened and showed problems must keep its Problems / File-Explorer
// badge after its editor tab is closed. Before the fix the server cleared a
// closed file's diagnostics (an empty publishDiagnostics), so badges vanished as
// VS Code cycled background tabs closed. The fix republishes the on-disk file's
// diagnostics on close, so they survive.
suite("Closed-file diagnostics retention (#865)", () => {
  const docUri = getDocUri("closedFileRetention.tcl");

  function codeOf(d: vscode.Diagnostic): string | number | undefined {
    return typeof d.code === "object" ? d.code.value : d.code;
  }

  test("problems survive closing the editor tab", async () => {
    // Must be opened through the workbench: an extension-initiated
    // `openTextDocument` pins the text model for up to three minutes, and a
    // pinned document never closes — so the close this test is about would
    // never reach the server. See `activateViaWorkbench`.
    await activateViaWorkbench(docUri);
    // The fixture's `set y 1` (assigned, never read) guarantees a W211, proving
    // the analyser ran while the file was open.
    const opened = await waitForDiagnostics(docUri, {
      predicate: (ds) => ds.some((d) => codeOf(d) === "W211"),
    });
    assert.ok(
      opened.some((d) => codeOf(d) === "W211"),
      `open file must surface W211: [${opened.map(codeOf)}]`,
    );

    // Close every editor. Subscribe to the document-close event *first*: closing
    // the tab is what VS Code does synchronously, but disposing the document —
    // the event vscode-languageclient turns into `textDocument/didClose` — is a
    // separate, later step, and it is that step the server reacts to. Capture
    // the log offset too, so the wait below matches the *close's* republish
    // rather than the open's.
    const closed = documentClosed(docUri);
    const sinceClose = getServerLogSize();
    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
    await closed;

    // The server re-analyses the on-disk file and republishes its diagnostics;
    // `did_close` emits an unconditional deep-diagnostics marker once that has
    // been delivered, so waiting on it is exact rather than best-effort. Then
    // assert the W211 is still present rather than cleared.
    await waitForDeepDiagnostics(docUri, { since: sinceClose });
    const retained = vscode.languages.getDiagnostics(docUri);
    assert.ok(
      retained.some((d) => codeOf(d) === "W211"),
      `#865: W211 must be retained after the tab closes, not cleared: [${retained.map(codeOf)}]`,
    );
  });
});
