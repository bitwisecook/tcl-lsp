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
 * The JavaScript end of the iRulesLX method relation (issue #1707).
 *
 * The provider itself is exercised against a stub client rather than a live
 * server: what has to hold here is the client-side contract — which documents
 * are even asked about, what the request carries, and that "no answer" stays
 * `undefined` so the JavaScript language service's own references survive.
 * That the server answers correctly is pinned by the Rust e2e suite, over a
 * real on-disk ILX workspace.
 */

import * as assert from "assert";
import * as vscode from "vscode";

import {
  createIlxReferenceProvider,
  looksLikeIlxExtensionSource,
  type IlxReferenceClient,
} from "../ilxReferences";

/** A `TextDocument` with only the fields the provider reads. */
function jsDocument(fsPath: string, text: string): vscode.TextDocument {
  return {
    uri: vscode.Uri.file(fsPath),
    getText: () => text,
  } as unknown as vscode.TextDocument;
}

const NEVER_CANCELLED = new vscode.CancellationTokenSource().token;
const WITH_DECLARATION: vscode.ReferenceContext = { includeDeclaration: true };

suite("iRulesLX JavaScript references", () => {
  test("the pre-filter accepts an extension source and rejects everything else", () => {
    assert.ok(looksLikeIlxExtensionSource("/w/ws/extensions/my_extension/index.js"));
    assert.ok(
      looksLikeIlxExtensionSource("/w/ws/extensions/my_extension/lib/server.js"),
      "a package.json `main` may sit deeper",
    );
    assert.ok(!looksLikeIlxExtensionSource("/w/ws/tool.js"));
    assert.ok(
      !looksLikeIlxExtensionSource("/w/ws/extensions/index.js"),
      "a file directly in `extensions/` is not inside an extension",
    );
    // Windows separators reach the same answer.
    assert.ok(looksLikeIlxExtensionSource("C:\\w\\ws\\extensions\\my_extension\\index.js"));
  });

  test("an ordinary JavaScript file never reaches the server", async () => {
    let calls = 0;
    const client: IlxReferenceClient = {
      sendRequest: async () => {
        calls += 1;
        return [];
      },
    };
    const provider = createIlxReferenceProvider(() => client);
    const found = await provider.provideReferences(
      jsDocument("/w/ws/tool.js", "ilx.addMethod('m', cb);"),
      new vscode.Position(0, 16),
      WITH_DECLARATION,
      NEVER_CANCELLED,
    );
    assert.strictEqual(found, undefined);
    assert.strictEqual(calls, 0, "the pre-filter must spend no round-trip");
  });

  test("the request carries the buffer, and the reply becomes Locations", async () => {
    let seen: Record<string, unknown> | undefined;
    const client: IlxReferenceClient = {
      sendRequest: async (_method, param) => {
        seen = (param as { arguments: Record<string, unknown>[] }).arguments[0];
        return [
          {
            uri: "file:///w/ws/rules/rule1.tcl",
            range: { start: { line: 2, character: 34 }, end: { line: 2, character: 48 } },
          },
        ];
      },
    };
    const provider = createIlxReferenceProvider(() => client);
    const found = await provider.provideReferences(
      jsDocument("/w/ws/extensions/my_extension/index.js", "ilx.addMethod('m', cb);"),
      new vscode.Position(0, 16),
      { includeDeclaration: false },
      NEVER_CANCELLED,
    );
    // The server holds no copy of a document it does not own, so the unsaved
    // buffer travels with the request.
    assert.strictEqual(seen?.text, "ilx.addMethod('m', cb);");
    assert.strictEqual(seen?.line, 0);
    assert.strictEqual(seen?.character, 16);
    assert.strictEqual(seen?.includeDeclaration, false);

    const locations = found as vscode.Location[];
    assert.strictEqual(locations.length, 1);
    assert.strictEqual(locations[0].uri.fsPath, vscode.Uri.file("/w/ws/rules/rule1.tcl").fsPath);
    assert.strictEqual(locations[0].range.start.line, 2);
    assert.strictEqual(locations[0].range.start.character, 34);
  });

  test("no answer stays undefined so the JavaScript provider's own results survive", async () => {
    const doc = jsDocument("/w/ws/extensions/my_extension/index.js", "ilx.listen();");
    const empty = createIlxReferenceProvider(() => ({ sendRequest: async () => [] }));
    assert.strictEqual(
      await empty.provideReferences(
        doc,
        new vscode.Position(0, 4),
        WITH_DECLARATION,
        NEVER_CANCELLED,
      ),
      undefined,
      "an empty list is not an emphatic `no references`",
    );
    const nullish = createIlxReferenceProvider(() => ({ sendRequest: async () => null }));
    assert.strictEqual(
      await nullish.provideReferences(
        doc,
        new vscode.Position(0, 4),
        WITH_DECLARATION,
        NEVER_CANCELLED,
      ),
      undefined,
    );
    // A server that predates the command, or one that is restarting.
    const failing = createIlxReferenceProvider(() => ({
      sendRequest: async () => {
        throw new Error("Unhandled method");
      },
    }));
    assert.strictEqual(
      await failing.provideReferences(
        doc,
        new vscode.Position(0, 4),
        WITH_DECLARATION,
        NEVER_CANCELLED,
      ),
      undefined,
    );
    // No client at all (activation raced the server start).
    const noClient = createIlxReferenceProvider(() => undefined);
    assert.strictEqual(
      await noClient.provideReferences(
        doc,
        new vscode.Position(0, 4),
        WITH_DECLARATION,
        NEVER_CANCELLED,
      ),
      undefined,
    );
  });
});
