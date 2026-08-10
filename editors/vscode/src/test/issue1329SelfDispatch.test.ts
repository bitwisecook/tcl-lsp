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

// Issue #1329, editor-integration layer: bareword `my <method>` was the only
// TclOO same-object dispatch spelling that navigated and highlighted
// correctly but was never diagnosed — `my nosuchmethod` drew nothing.
//
// Issue #1330, same layer: the range a dispatch diagnostic reports. That one
// already fired before the fix and only its range was wrong, so every
// assertion below compares the reported range against the *actual document
// text* it should cover, not merely against the presence of a code.

import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate, waitForDiagnostics } from "./helper";

function codeOf(d: vscode.Diagnostic): string {
  return typeof d.code === "object" && d.code !== null ? String(d.code.value) : String(d.code);
}

/** The range of `word`'s single occurrence in `doc`. */
function rangeOfWord(doc: vscode.TextDocument, word: string): vscode.Range {
  const text = doc.getText();
  const first = text.indexOf(word);
  assert.notStrictEqual(first, -1, `fixture must contain ${word}`);
  const last = text.lastIndexOf(word);
  assert.strictEqual(first, last, `${word} must occur exactly once in the fixture`);
  return new vscode.Range(doc.positionAt(first), doc.positionAt(first + word.length));
}

function describe(doc: vscode.TextDocument, diags: vscode.Diagnostic[]): string {
  return JSON.stringify(
    diags.map((d) => [
      codeOf(d),
      doc.getText(d.range),
      d.range.start.line,
      d.range.start.character,
    ]),
  );
}

suite("Issue #1329 / #1330 self-dispatch W308 and dispatch-site ranges", () => {
  const docUri = getDocUri("issue1329SelfDispatch.tcl");

  test("`my nosuchmethod` draws W308 underlining the method word alone", async () => {
    const doc = await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.filter((x) => codeOf(x) === "W308").length >= 2,
    });
    const w308 = diags.filter((d) => codeOf(d) === "W308");
    const expected = rangeOfWord(doc, "nosuchissue1329method");
    const hit = w308.find((d) => d.range.isEqual(expected));
    assert.ok(
      hit,
      `W308 must underline \`nosuchissue1329method\` exactly at ` +
        `${expected.start.line}:${expected.start.character}-` +
        `${expected.end.line}:${expected.end.character}; got ${describe(doc, w308)}`,
    );
    assert.ok(
      hit!.message.includes("nosuchissue1329method"),
      `message must name the method: ${hit!.message}`,
    );
  });

  test("`[Class new] badmethod` W308 underlines the method word, not across lines", async () => {
    const doc = await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.filter((x) => codeOf(x) === "W308").length >= 2,
    });
    const w308 = diags.filter((d) => codeOf(d) === "W308");
    const expected = rangeOfWord(doc, "nosuchissue1329bark");
    const hit = w308.find((d) => d.range.isEqual(expected));
    assert.ok(hit, `W308 must underline \`nosuchissue1329bark\` alone; got ${describe(doc, w308)}`);
    assert.strictEqual(
      hit!.range.start.line,
      hit!.range.end.line,
      "the range must not cross a line boundary (issue #1330)",
    );
  });

  test("a real method and `my variable` draw nothing", async () => {
    const doc = await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.filter((x) => codeOf(x) === "W308").length >= 2,
    });
    const w308 = diags.filter((d) => codeOf(d) === "W308");
    assert.strictEqual(
      w308.length,
      2,
      `only the two unknown methods may draw W308 — \`my issue1329Tick\` and ` +
        `\`my variable\` must stay silent; got ${describe(doc, w308)}`,
    );
    for (const word of ["issue1329Tick", "variable"]) {
      assert.ok(
        !w308.some((d) => doc.getText(d.range) === word),
        `\`my ${word}\` must not draw W308; got ${describe(doc, w308)}`,
      );
    }
  });

  test("the `my` head itself never draws W307", async () => {
    const doc = await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.filter((x) => codeOf(x) === "W308").length >= 2,
    });
    const w307 = diags.filter((d) => codeOf(d) === "W307");
    assert.strictEqual(
      w307.length,
      0,
      `recording \`my\` as a dispatch site must not leak into W307; ` +
        `got ${describe(doc, w307)}`,
    );
  });
});
