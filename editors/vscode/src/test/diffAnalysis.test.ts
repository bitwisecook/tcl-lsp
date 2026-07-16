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
  collectTabUriSets,
  diffOnlyUris,
  displayDiagnostics,
  symmetricDifference,
} from "../diffAnalysis";
import { activate, getDocUri, pollUntil, waitForDiagnostics } from "./helper";

function fakeTab(input: unknown): vscode.Tab {
  return { input } as unknown as vscode.Tab;
}

function fakeGroup(...tabs: vscode.Tab[]): vscode.TabGroup {
  return { tabs } as unknown as vscode.TabGroup;
}

function diag(message: string): vscode.Diagnostic {
  return new vscode.Diagnostic(
    new vscode.Range(0, 0, 0, 1),
    message,
    vscode.DiagnosticSeverity.Warning,
  );
}

suite("Diff Editor Diagnostics Suppression", () => {
  const a = vscode.Uri.file("/w/a.tcl").toString();
  const b = vscode.Uri.file("/w/b.tcl").toString();
  const c = vscode.Uri.file("/w/c.tcl").toString();

  suite("collectTabUriSets", () => {
    test("classifies normal and diff tabs, ignoring other tab kinds", () => {
      const uriA = vscode.Uri.file("/w/a.tcl");
      const uriB = vscode.Uri.file("/w/b.tcl");
      const groups = [
        fakeGroup(
          fakeTab(new vscode.TabInputText(uriA)),
          fakeTab(new vscode.TabInputTextDiff(uriB, uriA)),
          fakeTab(new vscode.TabInputTerminal()),
        ),
      ];

      const sets = collectTabUriSets(groups);

      assert.deepStrictEqual([...sets.normal].sort(), [a].sort());
      assert.deepStrictEqual([...sets.diff].sort(), [a, b].sort());
    });

    test("returns empty sets when no text tabs are open", () => {
      const sets = collectTabUriSets([fakeGroup(fakeTab(new vscode.TabInputTerminal()))]);
      assert.strictEqual(sets.normal.size, 0);
      assert.strictEqual(sets.diff.size, 0);
    });
  });

  suite("diffOnlyUris", () => {
    test("includes a URI shown only in a diff editor", () => {
      const result = diffOnlyUris({ normal: new Set(), diff: new Set([a]) });
      assert.deepStrictEqual([...result], [a]);
    });

    test("excludes a URI also open in a normal editor", () => {
      const result = diffOnlyUris({ normal: new Set([a]), diff: new Set([a, b]) });
      assert.deepStrictEqual([...result], [b]);
    });

    test("excludes a URI shown only in a normal editor", () => {
      const result = diffOnlyUris({ normal: new Set([a]), diff: new Set() });
      assert.strictEqual(result.size, 0);
    });
  });

  suite("symmetricDifference", () => {
    test("returns keys present in exactly one set", () => {
      const result = symmetricDifference(new Set([a, b]), new Set([b, c]));
      assert.deepStrictEqual([...result].sort(), [a, c].sort());
    });

    test("is empty for equal sets", () => {
      assert.strictEqual(symmetricDifference(new Set([a, b]), new Set([a, b])).size, 0);
    });
  });

  suite("displayDiagnostics", () => {
    const uri = vscode.Uri.file("/w/a.tcl");
    const diags = [diag("W100")];
    const enabled = () => true;
    const disabled = () => false;

    test("suppresses to empty when enabled and diff-only", () => {
      const out = displayDiagnostics(uri, diags, new Set([a]), enabled);
      assert.strictEqual(out.length, 0);
    });

    test("passes diagnostics through when the setting is disabled", () => {
      const out = displayDiagnostics(uri, diags, new Set([a]), disabled);
      assert.strictEqual(out, diags);
    });

    test("passes diagnostics through when the URI is not diff-only", () => {
      const out = displayDiagnostics(uri, diags, new Set(), enabled);
      assert.strictEqual(out, diags);
    });

    test("leaves an already-empty set untouched", () => {
      const empty: vscode.Diagnostic[] = [];
      const out = displayDiagnostics(uri, empty, new Set([a]), enabled);
      assert.strictEqual(out, empty);
    });
  });

  // End-to-end: drive a real diff editor and the language server, and confirm
  // the setting hides — and later restores — the file's diagnostics.
  suite("end-to-end", () => {
    const docUri = getDocUri("diagnostics.tcl");
    const originalUri = getDocUri("formatting.tcl");
    const diagCfg = () => vscode.workspace.getConfiguration("tclLsp.diagnostics");

    teardown(async () => {
      await diagCfg().update("suppressInDiffEditors", undefined, undefined);
      await vscode.commands.executeCommand("workbench.action.closeAllEditors");
    });

    test("hides diagnostics for a file shown only in a diff editor, then restores them", async () => {
      await diagCfg().update("suppressInDiffEditors", true, undefined);

      // Baseline: the file is open in a normal editor, so — even with the
      // setting on — its diagnostics are visible (it is not diff-only).
      await activate(docUri);
      const baseline = await waitForDiagnostics(docUri, { minCount: 1 });
      assert.ok(baseline.length > 0, "expected the fixture to produce diagnostics");

      // Open the same file as the modified side of a diff editor…
      await vscode.commands.executeCommand(
        "vscode.diff",
        originalUri,
        docUri,
        "diagnostics.tcl (diff-suppression test)",
      );
      // …then close its normal tab, leaving it shown only in the diff.
      await closeNormalTabsFor(docUri);

      await pollUntil(
        () => vscode.languages.getDiagnostics(docUri).length,
        (n) => n === 0,
        { label: "diagnostics suppressed while diff-only", timeout: 15_000 },
      );

      // Turning the setting off brings them straight back — no edit needed.
      await diagCfg().update("suppressInDiffEditors", false, undefined);
      await pollUntil(
        () => vscode.languages.getDiagnostics(docUri).length,
        (n) => n > 0,
        { label: "diagnostics restored after disabling the setting", timeout: 15_000 },
      );
    });
  });
});

/** Close every *normal* editor tab showing `uri` (leaving diff tabs open). */
async function closeNormalTabsFor(uri: vscode.Uri): Promise<void> {
  const targets: vscode.Tab[] = [];
  for (const group of vscode.window.tabGroups.all) {
    for (const tab of group.tabs) {
      const input = tab.input;
      if (input instanceof vscode.TabInputText && input.uri.toString() === uri.toString()) {
        targets.push(tab);
      }
    }
  }
  if (targets.length > 0) {
    await vscode.window.tabGroups.close(targets);
  }
}
