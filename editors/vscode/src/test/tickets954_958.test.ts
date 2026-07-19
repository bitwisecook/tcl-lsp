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

// TP/FP/TN/FN matrix for tickets #954–#958, driven through the real VS Code
// extension against the packaged language server.  These assert the behaviour
// the reporter sees in the editor: TclOO method reference counts ($obj / my),
// expr math-function reference counts, and apply-body highlighting.

import * as assert from "assert";
import * as vscode from "vscode";
import { activate, getDocUri, pollUntil } from "./helper";

interface DecodedToken {
  line: number;
  char: number;
  length: number;
  type: string;
}

function decodeTokens(
  tokens: vscode.SemanticTokens,
  legend: vscode.SemanticTokensLegend,
): DecodedToken[] {
  const out: DecodedToken[] = [];
  let line = 0;
  let char = 0;
  for (let i = 0; i < tokens.data.length; i += 5) {
    const [dLine, dChar, length, typeIdx] = tokens.data.slice(i, i + 5);
    if (dLine) {
      line += dLine;
      char = dChar;
    } else {
      char += dChar;
    }
    out.push({ line, char, length, type: legend.tokenTypes[typeIdx] });
  }
  return out;
}

// Poll until the reference-count lenses on every requested line have resolved,
// then return a map of line → title.
async function resolvedLensTitles(uri: vscode.Uri, lines: number[]): Promise<Map<number, string>> {
  const lenses = (await pollUntil(
    () => vscode.commands.executeCommand("vscode.executeCodeLensProvider", uri, 100),
    (r) => {
      const ls = r as vscode.CodeLens[] | undefined;
      if (!ls) return false;
      const resolved = new Set(
        ls
          .filter((l) => l.command && typeof l.command.title === "string")
          .map((l) => l.range.start.line),
      );
      return lines.every((line) => resolved.has(line));
    },
    { timeout: 10_000, label: "resolved reference-count lenses" },
  )) as vscode.CodeLens[] | undefined;
  assert.ok(lenses, "codeLens result should not be null");
  const byLine = new Map<number, string>();
  for (const lens of lenses) {
    if (lens.command && typeof lens.command.title === "string") {
      byLine.set(lens.range.start.line, lens.command.title);
    }
  }
  return byLine;
}

suite("Tickets #954–#958", () => {
  // #956 / #957 — TclOO method references.
  suite("TclOO method references", () => {
    const uri = getDocUri("tickets954_958_oo.tcl");

    test("reference-count lenses: obj-dispatch TP, my-dispatch TP, uncalled TN, channel FP", async () => {
      await activate(uri);
      // Method defs sit on lines 1 (fetch), 2 (get), 3 (unused), 4 (close).
      const titles = await resolvedLensTitles(uri, [1, 2, 3, 4]);
      // TP: `method get` is called via `$obj get` twice (lines 7, 8).
      assert.strictEqual(titles.get(2), "2 references", `get: ${titles.get(2)}`);
      // TP: `method fetch` is called via `my fetch` once (line 2).
      assert.strictEqual(titles.get(1), "1 reference", `fetch: ${titles.get(1)}`);
      // TN: `method unused` is never called.
      assert.strictEqual(titles.get(3), "0 references", `unused: ${titles.get(3)}`);
      // FP: `$ch close` is a channel op — `$ch` is never a Ticket956 object, so
      // `method close` must not be credited that call.
      assert.strictEqual(titles.get(4), "0 references", `close(FP): ${titles.get(4)}`);
    });

    test("find references on a method includes its $obj call sites", async () => {
      await activate(uri);
      // Cursor on `method get` (line 2, the `get` word at column 11).
      const locations = (await vscode.commands.executeCommand(
        "vscode.executeReferenceProvider",
        uri,
        new vscode.Position(2, 11),
      )) as vscode.Location[];
      assert.ok(locations, "references result should not be null");
      const lines = new Set(locations.map((l) => l.range.start.line));
      assert.ok(lines.has(7), "expected the `$obj get foo` call on line 7");
      assert.ok(lines.has(8), "expected the `$obj get bar` call on line 8");
    });
  });

  // #958 — expr math-function references.
  suite("expr math-function references", () => {
    const uri = getDocUri("tickets954_958_mathfunc.tcl");

    test("reference-count lenses credit ::tcl::mathfunc procs called in expr", async () => {
      await activate(uri);
      // proc defs on lines 1 (Pi) and 2 (deg2rad).
      const titles = await resolvedLensTitles(uri, [1, 2]);
      // Pi() is called in deg2rad's body (line 2) and at top level (line 4).
      assert.strictEqual(titles.get(1), "2 references", `Pi: ${titles.get(1)}`);
      // deg2rad(45) is called once (line 5).
      assert.strictEqual(titles.get(2), "1 reference", `deg2rad: ${titles.get(2)}`);
    });
  });

  // #954 — apply lambda body highlighting.
  suite("apply body highlighting", () => {
    const uri = getDocUri("tickets954_958_apply.tcl");

    test("body commands and variables are highlighted, param is a parameter", async () => {
      const doc = await activate(uri);
      const tokens = (await pollUntil(
        () => vscode.commands.executeCommand("vscode.provideDocumentSemanticTokens", uri),
        (r) => !!(r as vscode.SemanticTokens | undefined)?.data.length,
        { timeout: 10_000, label: "apply semantic tokens" },
      )) as vscode.SemanticTokens;
      const legend = (await vscode.commands.executeCommand(
        "vscode.provideDocumentSemanticTokensLegend",
        uri,
      )) as vscode.SemanticTokensLegend;
      assert.ok(tokens && legend, "expected semantic tokens and a legend");

      const decoded = decodeTokens(tokens, legend);
      const textOf = (t: DecodedToken): string =>
        doc.lineAt(t.line).text.substring(t.char, t.char + t.length);

      // Line 1 is the lambda body: `set` is a keyword and `$dir` a variable —
      // before the fix the whole body was one opaque braced string.
      const line1 = decoded.filter((t) => t.line === 1);
      assert.ok(
        line1.some((t) => t.type === "keyword" && textOf(t) === "set"),
        "expected `set` highlighted as a keyword inside the apply body",
      );
      assert.ok(
        line1.some((t) => t.type === "variable"),
        "expected a variable token inside the apply body",
      );
      // The lambda parameter `dir` (line 0) is a parameter declaration.
      assert.ok(
        decoded.some((t) => t.line === 0 && t.type === "parameter"),
        "expected the lambda parameter `dir` highlighted as a parameter",
      );
    });
  });
});
