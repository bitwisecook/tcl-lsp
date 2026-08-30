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
 * Find-references from an iRulesLX extension's JavaScript back to the iRules
 * that call it (issue #1707).
 *
 * The Tcl server already answers both ends of the relation, but the JavaScript
 * end was unreachable from the editor: `.js` is not a Tcl language id, so no
 * request ever left the client. Adding `javascript` to the language client's
 * document selector would have handed every JavaScript file in the project to
 * the Tcl server — the analyser, the workspace index and the diagnostics
 * pipeline all included — which is wrong for a file this extension understands
 * only two API calls of.
 *
 * So this is a *second* reference provider instead. VS Code merges the results
 * of every provider registered for a language, so the JavaScript language
 * service keeps answering for JavaScript and this one contributes the Tcl call
 * sites alongside it. The document never enters the Tcl document model at all.
 */

import * as vscode from "vscode";

import type { JsonLocation } from "./showReferences";

/** The `workspace/executeCommand` the server answers this with. */
const ILX_REFERENCES_COMMAND = "tcl-lsp.ilxReferences";

/** What the provider needs of the language client, and nothing more. */
export interface IlxReferenceClient {
  sendRequest(method: string, param: unknown): Promise<unknown>;
}

/**
 * Whether `path` could be an ILX extension source — a cheap pre-filter so an
 * ordinary JavaScript file never costs a round-trip.
 *
 * Deliberately looser than the server's own gate, which resolves the
 * extension's real entry point (`package.json`'s `main`, else `index.js`) and
 * requires this file to *be* it. This one only asks whether the path runs
 * through an `extensions/<name>/` directory, which every candidate does; the
 * server rejects the rest.
 */
export function looksLikeIlxExtensionSource(path: string): boolean {
  const parts = path.split(/[\\/]/);
  // `extensions` must be followed by the extension directory and then at least
  // one more segment (the file itself), so a match at the tail is not enough.
  const at = parts.indexOf("extensions");
  return at >= 0 && parts.length - at >= 3;
}

/**
 * The reference provider itself.
 *
 * Returns `undefined` (not an empty array) whenever this relation has nothing
 * to say — an ordinary `.js` file, a position that is not on an `addMethod`
 * name, a server that does not know the command — so VS Code falls back to
 * whatever the JavaScript language service found rather than showing an
 * emphatic "no references".
 */
export function createIlxReferenceProvider(
  getClient: () => IlxReferenceClient | undefined,
): vscode.ReferenceProvider {
  return {
    async provideReferences(document, position, context, token) {
      if (document.uri.scheme !== "file") {
        return undefined;
      }
      if (!looksLikeIlxExtensionSource(document.uri.fsPath)) {
        return undefined;
      }
      const client = getClient();
      if (!client || token.isCancellationRequested) {
        return undefined;
      }
      let result: unknown;
      try {
        result = await client.sendRequest("workspace/executeCommand", {
          command: ILX_REFERENCES_COMMAND,
          arguments: [
            {
              uri: document.uri.toString(),
              // The server holds no copy of a document it does not own, so the
              // buffer travels with the request — an unsaved `addMethod` is
              // then what navigation sees, as it is on the Tcl side.
              text: document.getText(),
              line: position.line,
              character: position.character,
              includeDeclaration: context.includeDeclaration,
            },
          ],
        });
      } catch {
        // A server that predates the command, or one that is restarting: this
        // provider simply has no answer, which is not an error to report.
        return undefined;
      }
      if (!Array.isArray(result) || result.length === 0) {
        return undefined;
      }
      return (result as JsonLocation[]).map(
        (loc) =>
          new vscode.Location(
            vscode.Uri.parse(loc.uri),
            new vscode.Range(
              loc.range.start.line,
              loc.range.start.character,
              loc.range.end.line,
              loc.range.end.character,
            ),
          ),
      );
    },
  };
}

/**
 * Register the provider for the JavaScript language ids VS Code uses for a
 * Node extension's sources.
 *
 * Registration is not itself an activation event: this extension activates on
 * a Tcl language id or on a workspace containing Tcl sources, which an ILX
 * workspace always does (its `rules/`). A JavaScript-only project therefore
 * never activates it, and never pays for this.
 */
export function registerIlxReferenceProvider(
  getClient: () => IlxReferenceClient | undefined,
): vscode.Disposable {
  return vscode.languages.registerReferenceProvider(
    [
      { scheme: "file", language: "javascript" },
      { scheme: "file", language: "javascriptreact" },
    ],
    createIlxReferenceProvider(getClient),
  );
}
