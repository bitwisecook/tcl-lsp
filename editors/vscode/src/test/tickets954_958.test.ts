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
import { activate, getDocUri, pollUntil, waitForDiagnostics } from "./helper";

// TP/FP/TN/FN matrix for the "preview version:" tickets #954-#958, exercised
// through the real VS Code client → language-server round trip:
//
//   #954  commands inside an `apply` lambda body highlight as a script and a
//         bare arg-list name is a `parameter` (semantic tokens)
//   #955  a `$dir` read in `pkgIndex.tcl` is not read-before-set (W210), but
//         the suppression is filename-scoped
//   #956  a `$obj method` dispatch is a reference and is counted by the lens
//   #957  a `my method` dispatch (nested in `[ … ]`) is a reference / counted
//   #958  a `::tcl::mathfunc::<fn>` expr-function application is a reference

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

/** The semantic-token type covering (line, char), or undefined. */
function typeAt(decoded: DecodedToken[], line: number, char: number): string | undefined {
  return decoded.find((t) => t.line === line && t.char <= char && char < t.char + t.length)?.type;
}

function codeOf(d: vscode.Diagnostic): string | number | undefined {
  return typeof d.code === "object" ? d.code.value : d.code;
}

/** The reference-count lens title anchored on `line`, once resolved. */
async function lensTitleOnLine(uri: vscode.Uri, line: number): Promise<string | undefined> {
  const lenses = (await pollUntil(
    () => vscode.commands.executeCommand("vscode.executeCodeLensProvider", uri, 100),
    (r) => {
      const ls = r as vscode.CodeLens[] | undefined;
      return (
        !!ls &&
        ls.some(
          (l) => l.range.start.line === line && l.command && typeof l.command.title === "string",
        )
      );
    },
    { timeout: 10_000, label: `resolved lens on line ${line}` },
  )) as vscode.CodeLens[];
  return lenses.find((l) => l.range.start.line === line)?.command?.title;
}

suite("Preview tickets #954-#958 matrix", () => {
  // ── #954 — apply lambda semantic tokens ────────────────────────────────
  test("#954 TP: apply body highlights and the bare arg-list name is a parameter", async () => {
    const uri = getDocUri("apply954.tcl");
    await activate(uri);
    const tokens = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokens",
      uri,
    )) as vscode.SemanticTokens;
    const legend = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokensLegend",
      uri,
    )) as vscode.SemanticTokensLegend;
    const decoded = decodeTokens(tokens, legend);
    // `apply {dir {` — `dir` starts at col 7 on line 0.
    assert.strictEqual(
      typeAt(decoded, 0, 7),
      "parameter",
      "#954: bare arg-list `dir` is a parameter",
    );
    // `    puts $dir` — `puts` starts at col 4 on line 1.
    assert.strictEqual(typeAt(decoded, 1, 4), "function", "#954: apply body command highlights");
  });

  // ── #955 — pkgIndex `$dir` W210 ────────────────────────────────────────
  test("#955 TP: `$dir` in pkgIndex.tcl draws no W210", async () => {
    const uri = getDocUri("pkgIndex.tcl");
    await activate(uri);
    // `set guard955 10 10` guarantees a diagnostic (arity), proving analysis
    // ran; the `$dir` reads must not add a W210.
    const diags = await waitForDiagnostics(uri, { minCount: 1 });
    assert.ok(
      !diags.map(codeOf).includes("W210"),
      `#955: unexpected W210 on pkgIndex.tcl in [${diags.map(codeOf)}]`,
    );
  });

  test("#955 FP: the same `$dir` read in an ordinary file still draws W210", async () => {
    const uri = getDocUri("pkgIndexControl.tcl");
    await activate(uri);
    const diags = await waitForDiagnostics(uri, {
      predicate: (ds) => ds.some((d) => codeOf(d) === "W210"),
    });
    assert.ok(
      diags.map(codeOf).includes("W210"),
      `#955: expected W210 outside pkgIndex.tcl, got [${diags.map(codeOf)}]`,
    );
  });

  // ── #956 — `$obj method` references + lens ─────────────────────────────
  test("#956 TP: `$obj method` dispatch is a reference and the lens counts it", async () => {
    const uri = getDocUri("objMethod956.tcl");
    await activate(uri);
    // `method get` — cursor inside `get` on line 1 (col 11).
    const position = new vscode.Position(1, 11);
    const locations = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeReferenceProvider", uri, position),
      (r) => Array.isArray(r) && (r as vscode.Location[]).length >= 2,
      { timeout: 10_000, label: "references for $obj method" },
    )) as vscode.Location[];
    const lines = new Set(locations.map((l) => l.range.start.line));
    assert.ok(lines.has(1) && lines.has(4), `#956: decl + dispatch, got ${[...lines]}`);
    assert.strictEqual(
      await lensTitleOnLine(uri, 1),
      "1 reference",
      "#956: lens counts the dispatch",
    );
  });

  // ── #957 — `my method` references + lens ───────────────────────────────
  test("#957 TP: `my method` nested in `[ … ]` is a reference and the lens counts it", async () => {
    const uri = getDocUri("myMethod957.tcl");
    await activate(uri);
    // `method getOptions` — cursor inside `getOptions` on line 1 (col 11).
    const position = new vscode.Position(1, 11);
    const locations = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeReferenceProvider", uri, position),
      (r) => Array.isArray(r) && (r as vscode.Location[]).length >= 2,
      { timeout: 10_000, label: "references for my method" },
    )) as vscode.Location[];
    const lines = new Set(locations.map((l) => l.range.start.line));
    assert.ok(lines.has(1) && lines.has(2), `#957: decl + my-dispatch, got ${[...lines]}`);
    assert.strictEqual(
      await lensTitleOnLine(uri, 1),
      "1 reference",
      "#957: lens counts the my-dispatch",
    );
  });

  // ── #958 — `::tcl::mathfunc` expr functions ────────────────────────────
  test("#958 TP: an expr math-function application is a reference to its proc", async () => {
    const uri = getDocUri("mathfunc958.tcl");
    await activate(uri);
    // `proc Pi958` — cursor inside `Pi958` on line 1 (col 9).
    const position = new vscode.Position(1, 9);
    const locations = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeReferenceProvider", uri, position),
      (r) => Array.isArray(r) && (r as vscode.Location[]).length >= 2,
      { timeout: 10_000, label: "references for mathfunc proc" },
    )) as vscode.Location[];
    const lines = new Set(locations.map((l) => l.range.start.line));
    assert.ok(lines.has(1) && lines.has(3), `#958: decl + expr-fn call, got ${[...lines]}`);
  });
});
