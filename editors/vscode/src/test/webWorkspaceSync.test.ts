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

/**
 * The browser host's workspace sweep derives its globs from the extension's own
 * manifest rather than restating the server's extension set. These tests run on
 * the desktop — the derivation is pure, and the web smoke test cannot tell a
 * subtly-narrow glob from a correct one, because both produce *some* files.
 */

import * as assert from "assert";
import * as vscode from "vscode";
import { deriveSyncGlobs, DEFAULT_BUDGET, SyncManifest } from "../webWorkspaceSync";

suite("Web workspace sync", () => {
  test("derives the source glob from the generated workspaceContains activation event", () => {
    const globs = deriveSyncGlobs({
      activationEvents: ["onLanguage:tcl", "workspaceContains:**/*.{[tT][cC][lL]}"],
    });
    assert.ok(
      globs.includes("**/*.{[tT][cC][lL]}"),
      `expected the activation glob verbatim, got ${globs.join(" ")}`,
    );
  });

  test("adds the whole-basename language registrations", () => {
    const globs = deriveSyncGlobs({
      activationEvents: ["workspaceContains:**/*.{tcl}"],
      contributes: { languages: [{ filenames: ["bigip.conf", "presentation"] }] },
    });
    assert.ok(globs.includes("**/bigip.conf"));
    assert.ok(globs.includes("**/presentation"));
  });

  test("always covers the project config files and sidecar stubs", () => {
    const globs = deriveSyncGlobs({});
    assert.ok(globs.includes("**/.tcl-lsp.ini"));
    assert.ok(globs.includes("**/tcl-lsp/config.ini"));
    assert.ok(globs.includes("**/*.tcl.stubs"));
  });

  test("falls back to the registered extensions when no activation glob exists", () => {
    const globs = deriveSyncGlobs({
      contributes: { languages: [{ extensions: [".tcl", ".irule"] }] },
    });
    assert.ok(
      globs.some((glob) => glob.includes("irule") && glob.includes("tcl")),
      `expected an extension glob, got ${globs.join(" ")}`,
    );
  });

  test("the real manifest yields the server's own case-folded source glob", () => {
    const extension = vscode.extensions.getExtension("bitwisecook.tcl-lsp");
    assert.ok(extension, "the extension under test is not installed");
    const globs = deriveSyncGlobs(extension.packageJSON as SyncManifest);
    // Case-folded per character (issue #1215), which is what makes the sweep
    // pick up `UPPER.TCL` on Linux — the same reason the server's watcher
    // registration folds it.
    const source = globs.find((glob) => glob.startsWith("**/*.{"));
    assert.ok(source, `no source glob derived: ${globs.join(" ")}`);
    assert.ok(source.includes("[tT][cC][lL]"), `the source glob is not case-folded: ${source}`);
    assert.ok(
      source.includes("[tT][cC][lL][sS][pP][eE][cC]"),
      `the source glob omits .tclspec packs: ${source}`,
    );
    assert.ok(globs.includes("**/bigip.conf"), "the BIG-IP filenames are missing");
  });

  test("the shipped budget is finite and per-file bounded", () => {
    assert.ok(DEFAULT_BUDGET.maxFiles > 0 && Number.isFinite(DEFAULT_BUDGET.maxFiles));
    assert.ok(DEFAULT_BUDGET.maxFileBytes < DEFAULT_BUDGET.maxTotalBytes);
  });
});
