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
import { getDocUri, activate, setTestContent } from "./helper";

// Issue #1138: a script argument built with `list` rather than written as a
// literal `{...}` block was never walked as the command it provably is. Tk's
// own `library/tk.tcl:289` is
// `uplevel #0 [list upvar #0 ::tk::Priv.$disp ::tk::Priv]`, and the very next
// line declares the same cell with a plain `variable ::tk::Priv`; only the
// built spelling was painted as a namespace word. tclsh 9.0.4 and 8.6.16
// agree the two spellings do the same thing.

interface DecodedToken {
  line: number;
  char: number;
  length: number;
  type: string;
  modifiers: number;
}

function decodeTokens(
  tokens: vscode.SemanticTokens,
  legend: vscode.SemanticTokensLegend,
): DecodedToken[] {
  const out: DecodedToken[] = [];
  let line = 0;
  let char = 0;
  for (let i = 0; i < tokens.data.length; i += 5) {
    const [dLine, dChar, length, typeIdx, mods] = tokens.data.slice(i, i + 5);
    if (dLine) {
      line += dLine;
      char = dChar;
    } else {
      char += dChar;
    }
    out.push({ line, char, length, type: legend.tokenTypes[typeIdx], modifiers: mods });
  }
  return out;
}

suite("Issue #1138: a script argument built with [list ...]", () => {
  const docUri = getDocUri("issue1138ListBuiltScript.tcl");

  async function tokensFor(
    editor: vscode.TextEditor,
    content: string,
  ): Promise<{
    toks: DecodedToken[];
    lines: string[];
    legend: vscode.SemanticTokensLegend;
  }> {
    await setTestContent(editor, content);
    const legend = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokensLegend",
      docUri,
    )) as vscode.SemanticTokensLegend;
    const tokens = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokens",
      docUri,
    )) as vscode.SemanticTokens;
    return { toks: decodeTokens(tokens, legend), lines: content.split("\n"), legend };
  }

  function covered(lines: string[], t: DecodedToken): string {
    return lines[t.line]?.slice(t.char, t.char + t.length) ?? "";
  }

  function declarationBit(legend: vscode.SemanticTokensLegend): number {
    const idx = legend.tokenModifiers.indexOf("declaration");
    assert.ok(idx >= 0, "the legend must declare a `declaration` modifier");
    return 1 << idx;
  }

  test("Tk's `uplevel #0 [list upvar ...]` declares its local like the literal spelling", async () => {
    const doc = await activate(docUri);
    const editor = await vscode.window.showTextDocument(doc);
    const original = doc.getText();
    try {
      const src =
        "proc f {disp} {\n" +
        "    uplevel #0 [list upvar #0 ::tk::Priv.$disp ::tk::Priv]\n" +
        "    variable ::tk::Priv\n" +
        "}\n";
      const { toks, lines, legend } = await tokensFor(editor, src);
      const declBit = declarationBit(legend);
      const dump = JSON.stringify(toks.map((t) => [t.line, t.char, covered(lines, t), t.type]));
      const at = (line: number): DecodedToken => {
        const found = toks.find((t) => t.line === line && covered(lines, t) === "::tk::Priv");
        if (!found) {
          throw new Error(`no \`::tk::Priv\` token on line ${line}; got ${dump}`);
        }
        return found;
      };
      const built = at(1);
      const literal = at(2);
      assert.strictEqual(
        built.type,
        "variable",
        `the built \`upvar\`'s local must be a variable, not a namespace word; got ${dump}`,
      );
      assert.notStrictEqual(
        built.modifiers & declBit,
        0,
        `the built \`upvar\`'s local must carry the declaration modifier; got ${dump}`,
      );
      assert.strictEqual(
        `${built.type}/${built.modifiers}`,
        `${literal.type}/${literal.modifiers}`,
        `both spellings of the same declaration must agree; got ${dump}`,
      );
    } finally {
      await setTestContent(editor, original);
    }
  });

  test("FP guard: the same words in a value slot declare nothing", async () => {
    const doc = await activate(docUri);
    const editor = await vscode.window.showTextDocument(doc);
    const original = doc.getText();
    try {
      // `list` invokes nothing — tclsh: `set x [list upvar 1 a b]; info exists b` -> 0.
      const src = "proc g {} {\n    set x [list upvar 1 a b]\n}\n";
      const { toks, lines, legend } = await tokensFor(editor, src);
      const declBit = declarationBit(legend);
      assert.ok(
        !toks.some(
          (t) =>
            covered(lines, t) === "b" && t.type === "variable" && (t.modifiers & declBit) !== 0,
        ),
        `a value slot must declare nothing; got ${JSON.stringify(
          toks.map((t) => [t.line, t.char, covered(lines, t), t.type]),
        )}`,
      );
    } finally {
      await setTestContent(editor, original);
    }
  });

  test("a dynamic list head keeps today's abstain-on-doubt behaviour", async () => {
    const doc = await activate(docUri);
    const editor = await vscode.window.showTextDocument(doc);
    const original = doc.getText();
    try {
      const src = "proc f {cb} {\n    uplevel #0 [list $cb #0 a b]\n}\n";
      const { toks, lines, legend } = await tokensFor(editor, src);
      const declBit = declarationBit(legend);
      assert.ok(
        !toks.some(
          (t) =>
            covered(lines, t) === "b" && t.type === "variable" && (t.modifiers & declBit) !== 0,
        ),
        "an unresolvable head must not produce a declaration",
      );
    } finally {
      await setTestContent(editor, original);
    }
  });
});
