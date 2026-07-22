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

// Issue #954 follow-up: the original fix (a bare-unbraced-parameter-list
// highlighting bug in `apply {dir {...}}`) shipped in v2.1.11, but the
// reporter's actual screenshot -- a pkgIndex.tcl-style
// `package ifneeded name ver [list apply {dir {...}} $dir]` entry -- reaches
// `apply` *indirectly* through the `[list ...]` command-quoting idiom, a
// shape the first fix never touched. These tests drive the real repro (and
// the closely related shapes the general, registry-driven fix now also
// covers) through VS Code's semantic-tokens provider.

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

suite("Issue #954 follow-up: apply reached through [list ...]", () => {
  const docUri = getDocUri("issue954Followup.tcl");

  async function tokensFor(
    editor: vscode.TextEditor,
    content: string,
  ): Promise<{ toks: DecodedToken[]; lines: string[] }> {
    await setTestContent(editor, content);
    const legend = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokensLegend",
      docUri,
    )) as vscode.SemanticTokensLegend;
    const tokens = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokens",
      docUri,
    )) as vscode.SemanticTokens;
    return { toks: decodeTokens(tokens, legend), lines: content.split("\n") };
  }

  function covered(lines: string[], t: DecodedToken): string {
    return lines[t.line]?.slice(t.char, t.char + t.length) ?? "";
  }

  function hasTyped(toks: DecodedToken[], lines: string[], text: string, type: string): boolean {
    return toks.some((t) => t.type === type && covered(lines, t) === text);
  }

  test("the reported pkgIndex.tcl repro: [list apply {dir {...}} $dir] body highlights", async () => {
    const doc = await activate(docUri);
    const editor = await vscode.window.showTextDocument(doc);
    const original = doc.getText();
    try {
      const src =
        "package ifneeded myPackage 1.0.0 [list apply {dir {\n" +
        "\n" +
        "    source [file join $dir src font.tcl]\n" +
        "    source [file join $dir src utils.tcl]\n" +
        "\n" +
        "}} $dir]\n";
      const { toks, lines } = await tokensFor(editor, src);
      assert.ok(
        hasTyped(toks, lines, "source", "keyword"),
        `expected \`source\` to tokenise inside the list-quoted apply body; got ${JSON.stringify(
          toks.map((t) => [t.line, t.char, covered(lines, t), t.type]),
        )}`,
      );
      assert.ok(hasTyped(toks, lines, "file", "function"), "expected `file` to tokenise");
      assert.ok(
        hasTyped(toks, lines, "dir", "parameter"),
        "expected the bare `dir` param to be a Parameter declaration",
      );
    } finally {
      await setTestContent(editor, original);
    }
  });

  test("a qualified ::apply inside [list ...] resolves the same way", async () => {
    const doc = await activate(docUri);
    const editor = await vscode.window.showTextDocument(doc);
    const original = doc.getText();
    try {
      const src = "package ifneeded p 1.0 [list ::apply {dir {puts $dir}} $dir]\n";
      const { toks, lines } = await tokensFor(editor, src);
      assert.ok(
        hasTyped(toks, lines, "puts", "function"),
        `expected \`puts\` to tokenise inside the qualified ::apply body; got ${JSON.stringify(
          toks.map((t) => [t.line, t.char, covered(lines, t), t.type]),
        )}`,
      );
    } finally {
      await setTestContent(editor, original);
    }
  });

  test("the same idiom under a different Body-role command (after idle) also highlights", async () => {
    const doc = await activate(docUri);
    const editor = await vscode.window.showTextDocument(doc);
    const original = doc.getText();
    try {
      const src = "after idle [list apply {{x} {puts $x}} 5]\n";
      const { toks, lines } = await tokensFor(editor, src);
      assert.ok(hasTyped(toks, lines, "puts", "function"));
      assert.ok(hasTyped(toks, lines, "x", "parameter"));
    } finally {
      await setTestContent(editor, original);
    }
  });

  test("package ifneeded's literal (non-list-quoted) script also highlights", async () => {
    const doc = await activate(docUri);
    const editor = await vscode.window.showTextDocument(doc);
    const original = doc.getText();
    try {
      const src = "package ifneeded myPackage 1.0.0 {\n    source [file join $dir x.tcl]\n}\n";
      const { toks, lines } = await tokensFor(editor, src);
      assert.ok(
        hasTyped(toks, lines, "source", "keyword"),
        "expected `source` to tokenise inside package ifneeded's literal script body",
      );
    } finally {
      await setTestContent(editor, original);
    }
  });

  test("FP guard: a plain data list is never split as a lambda literal", async () => {
    const doc = await activate(docUri);
    const editor = await vscode.window.showTextDocument(doc);
    const original = doc.getText();
    try {
      const src = "set data [list puts hello world]\n";
      const { toks, lines } = await tokensFor(editor, src);
      assert.ok(
        !hasTyped(toks, lines, "hello", "parameter"),
        `a plain data list must never be split as a lambda literal; got ${JSON.stringify(
          toks.map((t) => [t.line, t.char, covered(lines, t), t.type]),
        )}`,
      );
    } finally {
      await setTestContent(editor, original);
    }
  });

  test("regression: direct apply with a bare unbraced param list still highlights", async () => {
    const doc = await activate(docUri);
    const editor = await vscode.window.showTextDocument(doc);
    const original = doc.getText();
    try {
      const src = "apply {dir {\n    puts $dir\n}} /tmp\n";
      const { toks, lines } = await tokensFor(editor, src);
      assert.ok(hasTyped(toks, lines, "dir", "parameter"));
      assert.ok(hasTyped(toks, lines, "puts", "function"));
    } finally {
      await setTestContent(editor, original);
    }
  });
});
