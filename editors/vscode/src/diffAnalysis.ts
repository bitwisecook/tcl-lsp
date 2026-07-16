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

import * as vscode from "vscode";

/**
 * Optional per-resource suppression of Tcl diagnostics for documents that are
 * only being viewed in a diff editor — a Git change (Source Control), "Compare
 * With…", "Compare with Saved", and similar.
 *
 * "Diff editor" is a purely client-side concept: the language server publishes
 * diagnostics per URI and has no idea an editor is rendering that URI as a diff.
 * The modified side of a Git diff is the real working-tree file (`file:` scheme)
 * and is analysed exactly like any open file, so its squiggles show up in the
 * diff. When the user just wants to read a change without the analyser's noise,
 * `tclLsp.diagnostics.suppressInDiffEditors` lets them hide it.
 *
 * The gating happens in the `handleDiagnostics` middleware: the real diagnostics
 * are remembered per URI, but an empty set is handed to the client whenever the
 * URI is shown *exclusively* in diff editors. A file that is also open in a
 * normal editor keeps its diagnostics, so active editing is never dimmed.
 */

const CONFIG_SECTION = "tclLsp.diagnostics";
const CONFIG_KEY = "suppressInDiffEditors";
const CONFIG_PATH = `${CONFIG_SECTION}.${CONFIG_KEY}`;

/** The client's diagnostic sink — the `next` handed to the middleware. */
type ApplyDiagnostics = (uri: vscode.Uri, diagnostics: vscode.Diagnostic[]) => void;

/** URIs shown in normal editors vs in diff editors, as `Uri.toString()` keys. */
export interface TabUriSets {
  normal: Set<string>;
  diff: Set<string>;
}

/**
 * Split the URIs across every open tab into those shown in a normal text editor
 * and those shown in a diff editor. Both sides of a diff (original and modified)
 * count as diff. Non-text tabs (webviews, notebooks, terminals) are ignored.
 */
export function collectTabUriSets(groups: readonly vscode.TabGroup[]): TabUriSets {
  const normal = new Set<string>();
  const diff = new Set<string>();
  for (const group of groups) {
    for (const tab of group.tabs) {
      const input = tab.input;
      if (input instanceof vscode.TabInputText) {
        normal.add(input.uri.toString());
      } else if (input instanceof vscode.TabInputTextDiff) {
        diff.add(input.original.toString());
        diff.add(input.modified.toString());
      }
    }
  }
  return { normal, diff };
}

/**
 * The URIs shown *only* in diff editors — present on a diff side and in no normal
 * editor tab. A file open both ways is deliberately excluded so its diagnostics
 * survive while it is being edited.
 */
export function diffOnlyUris(sets: TabUriSets): Set<string> {
  const out = new Set<string>();
  for (const uri of sets.diff) {
    if (!sets.normal.has(uri)) {
      out.add(uri);
    }
  }
  return out;
}

/** Keys present in exactly one of the two sets. */
export function symmetricDifference(a: Set<string>, b: Set<string>): Set<string> {
  const out = new Set<string>();
  for (const x of a) {
    if (!b.has(x)) out.add(x);
  }
  for (const x of b) {
    if (!a.has(x)) out.add(x);
  }
  return out;
}

/**
 * Decide what to hand the client for one URI: the real diagnostics, or an empty
 * set when suppression is enabled for that resource and the URI is diff-only.
 */
export function displayDiagnostics(
  uri: vscode.Uri,
  diagnostics: vscode.Diagnostic[],
  diffOnly: Set<string>,
  isEnabled: (uri: vscode.Uri) => boolean,
): vscode.Diagnostic[] {
  if (diagnostics.length === 0 || !diffOnly.has(uri.toString())) {
    return diagnostics;
  }
  return isEnabled(uri) ? [] : diagnostics;
}

/**
 * Tracks diff editors and gates diagnostics accordingly. Install its
 * [`handleDiagnostics`](#handleDiagnostics) as the language client's
 * `handleDiagnostics` middleware and add the instance to the extension's
 * subscriptions so its listeners are cleaned up on deactivate.
 */
export class DiffDiagnosticsSuppressor implements vscode.Disposable {
  private readonly disposables: vscode.Disposable[] = [];
  /** The last real diagnostics the server pushed, keyed by `Uri.toString()`. */
  private readonly latest = new Map<
    string,
    { uri: vscode.Uri; diagnostics: vscode.Diagnostic[] }
  >();
  /** The client's diagnostic sink, captured from the middleware's `next`. */
  private apply?: ApplyDiagnostics;
  /** URIs currently shown exclusively in diff editors. */
  private diffOnly: Set<string>;

  constructor() {
    this.diffOnly = diffOnlyUris(collectTabUriSets(vscode.window.tabGroups.all));
    this.disposables.push(
      vscode.window.tabGroups.onDidChangeTabs(() => this.onTabsChanged()),
      vscode.workspace.onDidChangeConfiguration((e) => {
        if (e.affectsConfiguration(CONFIG_PATH)) {
          this.reapplyAll();
        }
      }),
      vscode.workspace.onDidCloseTextDocument((doc) => {
        this.latest.delete(doc.uri.toString());
      }),
    );
  }

  /**
   * `Middleware.handleDiagnostics` entry point. Remembers the server's real
   * diagnostics for the URI, then forwards either those or an empty set.
   */
  handleDiagnostics(
    uri: vscode.Uri,
    diagnostics: vscode.Diagnostic[],
    next: ApplyDiagnostics,
  ): void {
    this.apply = next;
    this.latest.set(uri.toString(), { uri, diagnostics });
    next(uri, this.display(uri, diagnostics));
  }

  private display(uri: vscode.Uri, diagnostics: vscode.Diagnostic[]): vscode.Diagnostic[] {
    return displayDiagnostics(uri, diagnostics, this.diffOnly, isSuppressionEnabled);
  }

  private onTabsChanged(): void {
    const previous = this.diffOnly;
    this.diffOnly = diffOnlyUris(collectTabUriSets(vscode.window.tabGroups.all));
    // A URI whose diff-only status is unchanged already shows the right set;
    // only the ones that flipped need re-evaluating.
    for (const key of symmetricDifference(previous, this.diffOnly)) {
      this.reapply(key);
    }
  }

  private reapplyAll(): void {
    for (const key of this.latest.keys()) {
      this.reapply(key);
    }
  }

  private reapply(key: string): void {
    const entry = this.latest.get(key);
    if (!entry || !this.apply) {
      return;
    }
    this.apply(entry.uri, this.display(entry.uri, entry.diagnostics));
  }

  dispose(): void {
    for (const d of this.disposables) {
      d.dispose();
    }
    this.disposables.length = 0;
    this.latest.clear();
  }
}

/** Whether diff-editor suppression is enabled for the given resource. */
function isSuppressionEnabled(uri: vscode.Uri): boolean {
  return vscode.workspace.getConfiguration(CONFIG_SECTION, uri).get<boolean>(CONFIG_KEY, false);
}
