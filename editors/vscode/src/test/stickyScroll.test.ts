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
import { TCL_LANGUAGE_IDS } from "../languageIds";
import { getDocUri, activate, pollUntil } from "./helper";

// VS Code picks the sticky-scroll model per language with a fallback chain:
// outline model → folding-range provider → indentation heuristic.  A
// *non-empty* outline stops the chain, so registering a document-symbol
// provider silently replaces the indentation heuristic the user had before
// the extension was installed (issue #1122).  For Tcl code the outline is
// definitions only — a script whose top level is `if` / `for` / `foreach`
// contributes nothing sticky, so sticky scroll goes blank.  Our folding
// ranges cover exactly those blocks, so Tcl languages default to the
// folding-provider model.  BIG-IP config needs it for a different reason:
// its outline is a synthesised `module → kind → object` tree whose module
// and kind nodes select the line of their *first* child, so the outline
// model pins the first `ltm` stanza in the file no matter where you scroll.
//
// The version-pinned dialect languages (`tcl84`, `tcl90`, …) carry the same
// default as every other Tcl language.  They only can because their ids are
// *undotted*: a language id containing a `.` cannot be used as a configuration
// override identifier at all.  VS Code splits `[tcl8.4]` on the dot while
// building the default-configuration value tree, throws a TypeError, and drops
// every remaining override in the same `configurationDefaults` block — ours
// and, since the tree is shared, any other extension's (issue #1122).  That is
// why the four version-pinned ids were renamed `tcl8.4` → `tcl84` and why a
// dotted id must never be reintroduced.  The "no contributed language id
// contains a dot" test below is the guard.
const STICKY_FOLDING_LANGUAGES = [...TCL_LANGUAGE_IDS];

// VS Code exposes no API to read the sticky-scroll widget itself, so the
// tests below replicate its folding-provider candidate pipeline against
// *real* provider output and assert on the replicated result. This mirrors
// (not reimplements verbatim) two files under
// vscode/src/vs/editor/contrib/stickyScroll/browser/:
//   - stickyScrollModelProvider.ts (collectSyntaxRanges): converts
//     FoldingRangeProvider output (0-based, inclusive end) to 1-based Monaco
//     line numbers, drops empty/out-of-bounds ranges, and nests the survivors
//     by containment.
//   - stickyScrollProvider.ts (VS Code >=1.105's candidate filter): a range
//     is only sticky-eligible when it spans more than one line beyond its
//     own header (`endLineNumber > startLineNumber + 1`) and its end does
//     not point past the document (`endLineNumber <= lineCount`). A rejected
//     candidate prunes its *entire* subtree, not just itself.
interface StickyFoldNode {
  startLineNumber: number; // 1-based, mirrors Monaco's convention
  endLineNumber: number; // 1-based, inclusive
  children: StickyFoldNode[];
}

function buildStickyFoldingTree(
  ranges: readonly vscode.FoldingRange[],
  lineCount: number,
): StickyFoldNode[] {
  // collectSyntaxRanges' filter: 0-based -> 1-based, drop empty/out-of-bounds.
  const converted = ranges
    .map((r) => ({ startLineNumber: r.start + 1, endLineNumber: r.end + 1 }))
    .filter((r) => r.endLineNumber > r.startLineNumber && r.endLineNumber <= lineCount);

  const sorted = [...converted].sort((a, b) =>
    a.startLineNumber !== b.startLineNumber
      ? a.startLineNumber - b.startLineNumber
      : b.endLineNumber - a.endLineNumber,
  );

  // Nest by containment: pop ancestors off the stack once their end has
  // already passed the next range's start.
  const roots: StickyFoldNode[] = [];
  const stack: StickyFoldNode[] = [];
  for (const item of sorted) {
    const node: StickyFoldNode = { ...item, children: [] };
    while (stack.length > 0 && stack[stack.length - 1].endLineNumber < node.startLineNumber) {
      stack.pop();
    }
    (stack.length === 0 ? roots : stack[stack.length - 1].children).push(node);
    stack.push(node);
  }

  // The 1.105 candidate filter, applied post-nesting -- a rejected node
  // prunes its whole subtree rather than just being excluded itself.
  const prune = (nodes: StickyFoldNode[]): StickyFoldNode[] =>
    nodes
      .filter((n) => n.endLineNumber > n.startLineNumber + 1 && n.endLineNumber <= lineCount)
      .map((n) => ({ ...n, children: prune(n.children) }));

  return prune(roots);
}

/**
 * The chain of surviving sticky candidates (outermost first) containing
 * 1-based `lineNumber` -- what the sticky widget would stack for a viewport
 * whose top line is `lineNumber`. Empty when nothing survives to pin.
 */
function stickyChainForLine(
  forest: readonly StickyFoldNode[],
  lineNumber: number,
): StickyFoldNode[] {
  for (const node of forest) {
    if (lineNumber >= node.startLineNumber && lineNumber <= node.endLineNumber) {
      return [node, ...stickyChainForLine(node.children, lineNumber)];
    }
  }
  return [];
}

function flattenDocumentSymbols(
  symbols: readonly vscode.DocumentSymbol[],
): vscode.DocumentSymbol[] {
  return symbols.flatMap((s) => [s, ...flattenDocumentSymbols(s.children ?? [])]);
}

function findDocumentSymbol(
  symbols: readonly vscode.DocumentSymbol[],
  name: string,
): vscode.DocumentSymbol | undefined {
  return flattenDocumentSymbols(symbols).find((s) => s.name === name);
}

// TclOO class fixture whose body closes at end-of-file with no trailing
// newline -- the boundary shape VS Code's isValidRange rejects when a
// provider's range points one line past the document. Same class shape as
// the on-disk `meter-2.0.tm` fixture (which mirrors the file shape from
// issue #1122 with invented identifiers -- the reporter's file and names
// are not public), minus the code that normally trails the class.
const METER_EOF_VARIANT = [
  "oo::class create ::example::meter::Meter {",
  "    superclass ::oo::object",
  "",
  "    variable Handle Config",
  "",
  "    constructor {handle config} {",
  "        set Handle $handle",
  "        set Config $config",
  "    }",
  "",
  "    method open {} {",
  "        set Handle [dict get $Config device]",
  "        return $Handle",
  "    }",
  "",
  "    method close {} {",
  "        set Handle {}",
  "    }",
  "}",
].join("\n");

suite("Sticky Scroll", () => {
  const stickyModelFor = (languageId: string): string | undefined =>
    vscode.workspace
      .getConfiguration("editor", { languageId })
      .get<string>("stickyScroll.defaultModel");

  test("no contributed language id contains a dot", () => {
    // A dotted language id is unusable as a `configurationDefaults` override
    // key: VS Code splits `"[tcl8.4]"` on the `.` while building the
    // default-configuration value tree, throws a TypeError, and abandons every
    // remaining override in that block.  The damage is not even limited to us —
    // the defaults tree is shared, so one dotted id can silently strip other
    // extensions' defaults too (issue #1122).  This is why the version-pinned
    // dialect ids are `tcl84` / `tcl85` / `tcl90` / `tcl91` rather than
    // `tcl8.4` / … .  Do not reintroduce a dotted id; the *dialect* strings
    // (`tcl8.4`) are a separate namespace and keep their dots.
    const dotted = [...TCL_LANGUAGE_IDS].filter((id) => id.includes("."));
    assert.deepStrictEqual(
      dotted,
      [],
      `language ids must not contain a '.' (would break configurationDefaults): ${dotted.join(", ")}`,
    );
  });

  test("every contributed Tcl language id is covered by the sticky-scroll default", () => {
    // Guards the flip side of the rename: the point of undotted ids is that
    // the four version-pinned languages can now carry the default, so none of
    // them may be missing from `configurationDefaults`.
    assert.ok(
      STICKY_FOLDING_LANGUAGES.length === TCL_LANGUAGE_IDS.size,
      "every contributed language id should be checked for the sticky-scroll default",
    );
    for (const languageId of ["tcl84", "tcl85", "tcl90", "tcl91"]) {
      assert.ok(
        TCL_LANGUAGE_IDS.has(languageId),
        `'${languageId}' should be a contributed language id`,
      );
    }
  });

  for (const languageId of STICKY_FOLDING_LANGUAGES) {
    test(`'${languageId}' defaults sticky scroll to the folding-provider model`, () => {
      assert.strictEqual(
        stickyModelFor(languageId),
        "foldingProviderModel",
        `'${languageId}' should default editor.stickyScroll.defaultModel to foldingProviderModel`,
      );
    });
  }

  test("BIG-IP config stanzas fold, so sticky scroll has real lines to pin", async () => {
    // A `.conf` is not Tcl, so it used to produce only comment folds and the
    // folding model would have had nothing to stick.  Folding now runs off
    // the stanza tree, at every nesting depth.
    await activate(getDocUri("folding.tcl"));

    const doc = await vscode.workspace.openTextDocument({
      language: "tcl-bigip",
      content: [
        "ltm virtual /Common/www_vs {",
        "    destination /Common/1.2.3.4:80",
        "    profiles {",
        "        /Common/http {",
        "        }",
        "    }",
        "}",
        "",
      ].join("\n"),
    });
    await vscode.window.showTextDocument(doc);

    const ranges = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeFoldingRangeProvider", doc.uri),
      (r) => Array.isArray(r) && r.length > 0,
      { timeout: 10_000, label: "BIG-IP folding ranges present" },
    )) as vscode.FoldingRange[];
    const spans = ranges.map((r) => [r.start, r.end]).sort((a, b) => a[0] - b[0]);
    assert.deepStrictEqual(
      spans,
      [
        [0, 5],
        [2, 4],
      ],
      `stanza and nested profiles block should fold, got ${JSON.stringify(ranges)}`,
    );
  });

  test("control-flow-only Tcl has no sticky outline but does have folds", async () => {
    // The regression the default guards against: every symbol in this
    // document is single-line, so VS Code's outline sticky model yields
    // nothing while the folding model pins `for` / `if` / `else`.
    await activate(getDocUri("folding.tcl"));

    const doc = await vscode.workspace.openTextDocument({
      language: "tcl",
      content: [
        "set total 0",
        "",
        "for {set i 0} {$i < 5} {incr i} {",
        "    set total [expr {$total + $i}]",
        "}",
        "",
        "if {$total > 5} {",
        '    puts "total is $total"',
        "} else {",
        '    puts "small total"',
        "}",
        "",
      ].join("\n"),
    });
    await vscode.window.showTextDocument(doc);

    const symbols =
      ((await vscode.commands.executeCommand(
        "vscode.executeDocumentSymbolProvider",
        doc.uri,
      )) as vscode.DocumentSymbol[]) ?? [];
    const multiLine = symbols.filter((s) => s.selectionRange.start.line !== s.range.end.line);
    assert.deepStrictEqual(
      multiLine.map((s) => s.name),
      [],
      "outline model would have nothing to stick for a control-flow-only script",
    );

    const ranges = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeFoldingRangeProvider", doc.uri),
      (r) => Array.isArray(r) && r.length > 0,
      { timeout: 10_000, label: "folding ranges present" },
    )) as vscode.FoldingRange[];
    const starts = ranges.map((r) => r.start).sort((a, b) => a - b);
    assert.deepStrictEqual(
      starts,
      [2, 6, 8],
      `folding should pin the for/if/else headers, got ${JSON.stringify(ranges)}`,
    );
  });

  // Issue #1122's reporter file is a TclOO Tcl module (`meter-2.0.tm`, language
  // id `tcl`). These tests replicate VS Code's actual sticky candidate
  // pipeline (see the `buildStickyFoldingTree` block comment above) against
  // real folding-provider output for that shape, rather than only checking
  // the folding-range command in isolation.
  test("TclOO module (meter-2.0.tm) folding survives VS Code's sticky candidate filter (issue #1122)", async () => {
    const docUri = getDocUri("meter-2.0.tm");
    const doc = await activate(docUri);

    const ranges = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeFoldingRangeProvider", docUri),
      (r) => Array.isArray(r) && r.length > 0,
      { timeout: 10_000, label: "meter-2.0.tm folding ranges present" },
    )) as vscode.FoldingRange[];
    assert.ok(ranges.length > 0, "TclOO module should produce folding ranges");

    const forest = buildStickyFoldingTree(ranges, doc.lineCount);
    assert.ok(
      forest.length > 0,
      `at least one folding range should survive the sticky candidate filter, got ranges ${JSON.stringify(
        ranges.map((r) => [r.start, r.end]),
      )}`,
    );

    const lines = doc.getText().split("\n");
    const classHeaderLine = lines.findIndex((l) => l.includes("oo::class create"));
    assert.ok(classHeaderLine >= 0, "fixture should declare the Meter class");

    // Viewport top-lines inside the class body: the class body itself, the
    // constructor, an ordinary (3-line-body) method, and a one-line-body
    // method whose own fold is too short to survive the 1.105 filter alone.
    // In every case the class header must still be the outermost surviving
    // candidate -- sticky scroll always has the class name to pin.
    const markers: Array<[string, number]> = [
      [
        "class body (before any method)",
        lines.findIndex((l) => l.trim() === "variable Handle Config"),
      ],
      ["constructor body", lines.findIndex((l) => l.includes("set Handle $handle"))],
      ["method open body", lines.findIndex((l) => l.includes("dict get $Config"))],
      [
        "method close body (2-line fold, pruned on its own)",
        lines.findIndex((l) => l.trim() === "set Handle {}"),
      ],
    ];
    for (const [label, zeroBasedLine] of markers) {
      assert.ok(zeroBasedLine >= 0, `fixture should contain the '${label}' marker line`);
      const chain = stickyChainForLine(forest, zeroBasedLine + 1);
      assert.ok(
        chain.length > 0,
        `expected a surviving sticky candidate for ${label} (line ${zeroBasedLine})`,
      );
      assert.strictEqual(
        chain[0].startLineNumber - 1,
        classHeaderLine,
        `outermost surviving sticky candidate for ${label} should be the class header ` +
          `(line ${classHeaderLine}), got chain ${JSON.stringify(chain)}`,
      );
    }

    // `method close`'s own fold is exactly the shape 1.105 rejects
    // (endLineNumber > startLineNumber + 1 fails for a 2-line-tall fold):
    // confirm it was pruned rather than surviving, so its chain is only the
    // class header deep, not class-then-method.
    const closeLine = lines.findIndex((l) => l.trim() === "set Handle {}");
    const closeChain = stickyChainForLine(forest, closeLine + 1);
    assert.strictEqual(
      closeChain.length,
      1,
      "method close's own 2-line fold should have been pruned by the 1.105 filter, " +
        `leaving only the class header; got ${JSON.stringify(closeChain)}`,
    );
  });

  test("TclOO module (meter-2.0.tm) outline symbols never point past EOF (issue #1122)", async () => {
    const docUri = getDocUri("meter-2.0.tm");
    const doc = await activate(docUri);

    const symbols =
      ((await pollUntil(
        () => vscode.commands.executeCommand("vscode.executeDocumentSymbolProvider", docUri),
        (r) => Array.isArray(r) && r.length > 0,
        { timeout: 10_000, label: "meter-2.0.tm document symbols present" },
      )) as vscode.DocumentSymbol[]) ?? [];

    const classSymbol = findDocumentSymbol(symbols, "Meter");
    assert.ok(
      classSymbol,
      `expected a 'Meter' class symbol, got ${JSON.stringify(symbols.map((s) => s.name))}`,
    );
    assert.strictEqual(
      classSymbol?.kind,
      vscode.SymbolKind.Class,
      "'Meter' should be a Class symbol",
    );
    assert.ok(
      classSymbol!.range.end.line - classSymbol!.range.start.line >= 2,
      `class symbol should span at least 3 lines, got ${classSymbol!.range.start.line}..${classSymbol!.range.end.line}`,
    );

    // VS Code's isValidRange guard for outline sticky candidates: end.line
    // (0-based) must be strictly less than the document's line count -- a
    // symbol range pointing at (or past) a phantom line past EOF is silently
    // dropped from the sticky model. Breadcrumbs mask the same violation
    // (they tolerate an out-of-bounds range), which is why this needs its
    // own assertion here rather than relying on the outline UI looking fine.
    for (const sym of flattenDocumentSymbols(symbols)) {
      assert.ok(
        sym.range.end.line < doc.lineCount,
        `symbol '${sym.name}' range.end.line (${sym.range.end.line}) must be < doc.lineCount (${doc.lineCount})`,
      );
    }
  });

  test("TclOO class closing at end-of-file with no trailing newline keeps outline symbols in bounds (issue #1122)", async () => {
    await activate(getDocUri("folding.tcl"));

    const doc = await vscode.workspace.openTextDocument({
      language: "tcl",
      content: METER_EOF_VARIANT,
    });
    await vscode.window.showTextDocument(doc);
    assert.ok(!doc.getText().endsWith("\n"), "fixture must have no trailing newline");

    const symbols =
      ((await pollUntil(
        () => vscode.commands.executeCommand("vscode.executeDocumentSymbolProvider", doc.uri),
        (r) => Array.isArray(r) && r.length > 0,
        { timeout: 10_000, label: "EOF-closing class document symbols present" },
      )) as vscode.DocumentSymbol[]) ?? [];

    const flat = flattenDocumentSymbols(symbols);
    assert.ok(flat.length > 0, "expected at least the class symbol");
    for (const sym of flat) {
      assert.ok(
        sym.range.end.line < doc.lineCount,
        `symbol '${sym.name}' range.end.line (${sym.range.end.line}) must be < doc.lineCount (${doc.lineCount})`,
      );
    }

    const classSymbol = findDocumentSymbol(symbols, "Meter");
    assert.ok(classSymbol, "expected the class symbol to survive a class closing at EOF");
  });
});
