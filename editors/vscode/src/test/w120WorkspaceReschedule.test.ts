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
import * as path from "path";
import { getDocUri, activate, waitForDiagnostics, scaledTimeout } from "./helper";

// Issue #829, third bug: `initialized()` kicks off `scan_workspace_folders()`
// (which populates `workspace_index` / `package_resolver`, the inputs the
// always-on W120/W123 cross-file diagnostic refinement reads) but used to
// never reschedule diagnostics for already-open documents once that scan
// completed. A document opened at the exact moment the server starts could
// get its first diagnostics run computed against the still-empty workspace
// index -- a false-positive W120 ("X requires `package require Y`") that
// never self-corrected until the user made an edit. The fix makes
// `initialized()`, `did_change_workspace_folders`, and
// `did_change_watched_files` all call `reschedule_all_open_documents()`
// (unconditionally -- not just for `xcDiagnostics`-enabled documents) after
// any operation that changes `workspace_index` / `package_resolver`.
//
// Reproducing the *exact* startup race needs a fresh VS Code process per
// test -- this mocha run shares one already-initialised extension host
// across every test file, so by the time this suite runs, `initialized()`'s
// one-time scan is long finished (see the analogous note in
// `multiFolder/multiFolderConfig.test.ts`'s race-detection harness for
// issue #407, which hits the same "mocha shares one process" wall and
// resolves it the same way: drive the same fix through a *different*,
// runtime-triggerable entry point into the identical code path). Here that
// entry point is `did_change_watched_files`: creating an on-disk ancestor
// file that the file-system watcher the *server* registers at
// `initialized` (a case-folded glob over its own Tcl extension set) turns
// into a `workspace/didChangeWatchedFiles` CREATE notification, which
// reaches the exact same `reschedule_all_open_documents()` call the
// startup-race fix added.
suite("W120 workspace-index reschedule (#829)", () => {
  const callerUri = getDocUri("w120Reschedule/caller.tcl");
  const entryUri = vscode.Uri.file(path.join(path.dirname(callerUri.fsPath), "entry.tcl"));

  function codeOf(d: vscode.Diagnostic): string | number | undefined {
    return typeof d.code === "object" ? d.code.value : d.code;
  }

  teardown(async () => {
    // Best-effort cleanup: the ancestor is created fresh by each test and
    // must not linger as an untracked file between runs.
    try {
      await vscode.workspace.fs.delete(entryUri, { useTrash: false });
    } catch {
      // Already absent -- nothing to clean up.
    }
  });

  test(
    "a false-positive W120 clears once an on-disk (unopened) ancestor sources " +
      "and requires it, without editing the caller",
    async function () {
      this.timeout(scaledTimeout(60_000));
      await activate(callerUri);

      // Precondition: with no ancestor on disk, the single-file W120 stands
      // -- proves the fixture genuinely exercises the diagnostic rather than
      // the assertion below trivially passing on an already-clean document.
      const before = await waitForDiagnostics(callerUri, {
        predicate: (diags) => diags.some((d) => codeOf(d) === "W120"),
        timeout: 20_000,
      });
      assert.ok(
        before.some((d) => codeOf(d) === "W120"),
        `precondition failed: W120 should fire with no ancestor on disk, got: ${JSON.stringify(
          before.map((d) => [codeOf(d), d.message]),
        )}`,
      );

      // Create the ancestor on disk -- deliberately never opened in the
      // editor -- that `source`s the caller and requires the package. The
      // watcher the server registered turns this real disk write into a
      // `workspace/didChangeWatchedFiles` CREATE notification.
      const entryContent = Buffer.from("package require report\nsource caller.tcl\n", "utf8");
      await vscode.workspace.fs.writeFile(entryUri, entryContent);

      const after = await waitForDiagnostics(callerUri, {
        predicate: (diags) => !diags.some((d) => codeOf(d) === "W120"),
        timeout: 30_000,
      });
      assert.ok(
        !after.some((d) => codeOf(d) === "W120"),
        "W120 must clear once the on-disk ancestor's `package require` + `source` " +
          `is indexed and the open caller document is rescheduled, got: ${JSON.stringify(
            after.map((d) => [codeOf(d), d.message]),
          )}`,
      );
    },
  );
});
