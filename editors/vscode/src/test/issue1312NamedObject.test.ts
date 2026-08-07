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

// Issue #1312, editor-integration layer: `ClassName create objName` (as
// opposed to `[ClassName new]`) resolves no members — `obj method` gives no
// diagnostics or semantic-token classification where the handle form does.
//
// Fixture layout (0-based lines), `issue1312NamedObject.tcl`:
//   4  oo::class create C { … }
//   5      method mrun {} { … }
//   7  C create obj
//   8  obj mrun          <- the dispatch site: `mrun` must colour as `method`
//   9  obj nosuchmethod  <- W308 must fire here

import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate, waitForDiagnostics } from "./helper";

function codeOf(d: vscode.Diagnostic): string {
  return typeof d.code === "object" && d.code !== null ? String(d.code.value) : String(d.code);
}

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

suite("Issue #1312 named-object dispatch", () => {
  const docUri = getDocUri("issue1312NamedObject.tcl");

  test("`C create obj; obj nosuchmethod` draws W308 like the handle form", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W308"),
    });
    const w308 = diags.filter((d) => codeOf(d) === "W308");
    assert.strictEqual(w308.length, 1, `expected one W308: ${JSON.stringify(diags)}`);
    assert.strictEqual(w308[0].range.start.line, 9, "anchored on `obj nosuchmethod`");
  });

  test("the named-object dispatch site colours its method word as `method`", async () => {
    const doc = await activate(docUri);
    const legend = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokensLegend",
      docUri,
    )) as vscode.SemanticTokensLegend;
    const tokens = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokens",
      docUri,
    )) as vscode.SemanticTokens;
    const toks = decodeTokens(tokens, legend);
    const lines = doc.getText().split("\n");
    const covered = (t: DecodedToken): string =>
      lines[t.line]?.slice(t.char, t.char + t.length) ?? "";
    const dump = JSON.stringify(toks.map((t) => [t.line, t.char, covered(t), t.type]));
    // Line 8 is `obj mrun` — the dispatch site, not the declaration on
    // line 5. Only the dispatch classification was broken by issue #1312.
    const dispatched = toks.find((t) => t.line === 8 && covered(t) === "mrun");
    assert.ok(dispatched, `the dispatched \`mrun\` must colour as a method: ${dump}`);
    assert.strictEqual(dispatched?.type, "method");
  });
});
