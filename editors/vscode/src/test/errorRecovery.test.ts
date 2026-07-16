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
import {
  getDocUri,
  activate,
  setTestContent,
  nextDiagnosticsPublish,
  waitForDiagnostics,
  getServerLogSize,
  waitForDeepDiagnostics,
} from "./helper";

/**
 * Error-recovery correctness contract — driven through the real VS Code
 * extension + packaged language server.
 *
 * Like the lsp_e2e suite, this is intentionally **implementation-agnostic**: it
 * asserts the observable recovery behaviour an editor user depends on, never a
 * server internal or a backend-specific diagnostic code.  It is the editor-side
 * half of the contract the recovery engine must satisfy in the native Rust
 * tcl-lsp-server, driven end to end through the real VS Code extension.
 *
 *   C1  an unterminated [ / " / { is flagged with an error diagnostic
 *   C2  recovery is non-fatal — a proc after the break is still a document symbol
 *   C3  the recovered semantic-token stream is well-formed and re-lexes the tail
 *   C4  the published diagnostic set has no exact duplicates
 *   C5  edits that introduce a break flag it and edits that fix it clear it
 *   C6  well-formed code produces no recovery error
 */

const RECOVERY_CODES = new Set(["E200", "E201", "E202", "E203", "E204", "E205", "E206"]);
const RECOVERY_WORDS = [
  "missing",
  "close",
  "unterminated",
  "unclosed",
  "unbalanced",
  "extra characters",
];

function codeOf(d: vscode.Diagnostic): string {
  const c = d.code;
  if (c && typeof c === "object" && "value" in c) return String((c as { value: unknown }).value);
  return String(c);
}

function isRecoveryError(d: vscode.Diagnostic): boolean {
  if (RECOVERY_CODES.has(codeOf(d))) return true;
  const msg = (d.message || "").toLowerCase();
  return (
    d.severity === vscode.DiagnosticSeverity.Error && RECOVERY_WORDS.some((w) => msg.includes(w))
  );
}

function hasRecoveryError(diags: vscode.Diagnostic[]): boolean {
  return diags.some(isRecoveryError);
}

function symbolNames(
  syms: (vscode.DocumentSymbol | vscode.SymbolInformation)[] | undefined,
): string[] {
  const out: string[] = [];
  const walk = (items: (vscode.DocumentSymbol | vscode.SymbolInformation)[] | undefined) => {
    for (const s of items || []) {
      out.push(s.name);
      walk((s as vscode.DocumentSymbol).children);
    }
  };
  walk(syms);
  return out;
}

/**
 * Replace the document content and resolve with the resulting diagnostics.
 *
 * `until` is what the caller is actually waiting for. The server publishes
 * diagnostics **progressively**: a first publish can carry the lint pass alone,
 * with the parse-recovery error arriving in a later one. Reading whatever
 * happens to be published first (`predicate: () => true`) therefore races the
 * analysis, and under CI load C1 saw `[W211]` and no recovery error and failed —
 * while passing every time on an idle laptop.
 *
 * A test asserting a diagnostic is *present* must say so, and wait for it.
 * A test asserting *absence* has nothing to wait for, so it keeps the settle
 * behaviour (the default `until`).
 */
async function setContentAndWait(
  editor: vscode.TextEditor,
  docUri: vscode.Uri,
  content: string,
  until: (diags: vscode.Diagnostic[]) => boolean = () => true,
): Promise<vscode.Diagnostic[]> {
  const fresh = nextDiagnosticsPublish(docUri, { timeout: 15_000 });
  await setTestContent(editor, content);
  await fresh;
  return waitForDiagnostics(docUri, { timeout: 15_000, predicate: until });
}

async function symbolsFor(docUri: vscode.Uri): Promise<string[]> {
  const syms = (await vscode.commands.executeCommand(
    "vscode.executeDocumentSymbolProvider",
    docUri,
  )) as (vscode.DocumentSymbol | vscode.SymbolInformation)[];
  return symbolNames(syms);
}

suite("Error Recovery (contract)", () => {
  const docUri = getDocUri("errorRecovery.tcl");

  // C1 ----------------------------------------------------------------------
  test("C1: unterminated bracket is flagged", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    const diags = await setContentAndWait(
      editor,
      docUri,
      "set x [foo bar\nputs hi\n",
      hasRecoveryError,
    );
    assert.ok(hasRecoveryError(diags), `expected a recovery error, got [${diags.map(codeOf)}]`);
  });

  test("C6: well-formed code is not flagged", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    const diags = await setContentAndWait(editor, docUri, "set x [foo bar]\nputs hi\n");
    assert.ok(
      !hasRecoveryError(diags),
      `well-formed flagged a recovery error: [${diags.map(codeOf)}]`,
    );
  });

  // C2 ----------------------------------------------------------------------
  test("C2: proc after an unterminated bracket is still a symbol", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    await setContentAndWait(editor, docUri, "set x [foo\nproc recovered_after_bracket {} {}\n");
    const names = await symbolsFor(docUri);
    assert.ok(
      names.includes("recovered_after_bracket"),
      `tail proc not recovered; symbols=[${names}]`,
    );
  });

  test("C2: a command after an unterminated braced expression is analysed", async () => {
    // `if {$x > 5` is an unterminated braced *expression*; recovery must let
    // the swallowed tail still be analysed as code. The unclosed brace runs
    // to EOF, so `if`'s own single (swallowed) argument is itself an
    // arity/shape defect: E002 (a plain arity floor) or the more precise
    // E004 (`if`'s dedicated clause-shape check, which subsumes E002 for
    // `if`) — either is acceptable here, since this suite asserts
    // observable recovery behaviour, not a specific diagnostic code.
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    const hasArityOrShape = (ds: vscode.Diagnostic[]) =>
      ds.some((d) => codeOf(d) === "E002" || codeOf(d) === "E004");
    const diags = await setContentAndWait(editor, docUri, "if {$x > 5\nset\n", hasArityOrShape);
    assert.ok(
      hasArityOrShape(diags),
      `tail after \`if {\` should still be analysed and arity/shape-error; got [${diags.map(codeOf)}]`,
    );
  });

  test("C2: proc after an unterminated namespace body is still a symbol", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    await setContentAndWait(editor, docUri, "namespace eval n {\nproc recovered_in_ns {} {}\n");
    const names = await symbolsFor(docUri);
    assert.ok(names.includes("recovered_in_ns"), `tail proc not recovered; symbols=[${names}]`);
  });

  test("C2: proc after multiple independent breaks is still a symbol", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    await setContentAndWait(
      editor,
      docUri,
      "set a [foo\nset b 2\nset c [bar\nproc recovered_after_two {} {}\n",
    );
    const names = await symbolsFor(docUri);
    assert.ok(
      names.includes("recovered_after_two"),
      `tail proc after two breaks not recovered; symbols=[${names}]`,
    );
  });

  // C3 ----------------------------------------------------------------------
  test("C3: semantic tokens are well-formed for pathological broken input", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    const src = 'proc p {} {\n  set x [foo "bar\n  if {1} {\n    puts [baz\n}\nputs end\n';
    await setContentAndWait(editor, docUri, src);
    const result = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokens",
      docUri,
    )) as vscode.SemanticTokens;
    assert.ok(result, "expected a semantic-tokens result");
    assert.strictEqual(result.data.length % 5, 0, "token data must be 5-int groups");
    // Decode the delta-encoded stream and check every position is in-document.
    const nLines = src.split("\n").length;
    let line = 0;
    let char = 0;
    for (let i = 0; i < result.data.length; i += 5) {
      const dLine = result.data[i];
      const dStart = result.data[i + 1];
      const len = result.data[i + 2];
      line += dLine;
      char = dLine === 0 ? char + dStart : dStart;
      assert.ok(line >= 0 && line < nLines, `token line ${line} out of range`);
      assert.ok(len >= 0, "token length must be non-negative");
    }
  });

  test("C3: deeply nested unterminated input does not hang or crash", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    const diags = await setContentAndWait(
      editor,
      docUri,
      "set x [a [b [c [d [e\nputs tail\n",
      hasRecoveryError,
    );
    // A real hang publishes no recovery error inside the 15s budget, so the wait
    // times out with an empty/stale set and this assertion fails loudly — making
    // "does not hang" a genuine check rather than a vacuous Array.isArray pass.
    assert.ok(hasRecoveryError(diags), `expected a recovery error: [${diags.map(codeOf)}]`);
  });

  // C4 ----------------------------------------------------------------------
  test("C4: no exact-duplicate diagnostics are published", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    const since = getServerLogSize();
    const fresh = nextDiagnosticsPublish(docUri, { timeout: 15_000 });
    await setTestContent(editor, 'set x "\nif {1} {\n  puts [foo\n}\nset\n');
    await fresh;
    // Wait for the deep pass's marker (throws on timeout) so dedup runs over the
    // fully-settled basic+deep set. Reading the first publish alone would dedup a
    // partial set and pass "no duplicates" vacuously.
    await waitForDeepDiagnostics(docUri, { since });
    const diags = vscode.languages.getDiagnostics(docUri);
    const seen = new Set<string>();
    for (const d of diags) {
      const ident = JSON.stringify([
        codeOf(d),
        d.range.start.line,
        d.range.start.character,
        d.range.end.line,
        d.range.end.character,
        d.message,
      ]);
      assert.ok(!seen.has(ident), `duplicate diagnostic published: ${ident}`);
      seen.add(ident);
    }
  });

  // C5 ----------------------------------------------------------------------
  test("C5: editing a document to break it flags it, and fixing clears it", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;

    let diags = await setContentAndWait(editor, docUri, "set x [foo bar]\nputs hi\n");
    assert.ok(!hasRecoveryError(diags), `clean start flagged: [${diags.map(codeOf)}]`);

    diags = await setContentAndWait(editor, docUri, "set x [foo bar\nputs hi\n", hasRecoveryError);
    assert.ok(hasRecoveryError(diags), `break not flagged: [${diags.map(codeOf)}]`);

    diags = await setContentAndWait(editor, docUri, "set x [foo bar]\nputs hi\n");
    assert.ok(!hasRecoveryError(diags), `fix did not clear: [${diags.map(codeOf)}]`);
  });
});

suite("Error Recovery (known-command generality)", () => {
  // A break just before a call to a command the document itself defines (a
  // proc, a TclOO class, an `interp alias`) must recover exactly as well as
  // a break before a call to a builtin — previously the "does the next line
  // start with a known command?" recovery signal only ever consulted the
  // static registry, so real-world files (almost all of which call their
  // own procs) silently lost the rest of the document to analysis whenever
  // no *builtin* call happened to follow the break.
  const docUri = getDocUri("errorRecovery.tcl");

  test("a call to a user-defined proc recovers the tail like a builtin", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    const diags = await setContentAndWait(
      editor,
      docUri,
      "proc my_helper {x} {puts $x}\n\nset q {\n  aaa\nmy_helper\n",
      (ds) => hasRecoveryError(ds) && ds.some((d) => codeOf(d) === "E002"),
    );
    assert.ok(hasRecoveryError(diags), `expected a recovery error: [${diags.map(codeOf)}]`);
    assert.ok(
      diags.some((d) => codeOf(d) === "E002"),
      `swallowed \`my_helper\` call should still arity-error, proving the tail \
       is analysed as code rather than dropped: [${diags.map(codeOf)}]`,
    );
  });

  test("a call to a user-defined TclOO class recovers the tail", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    await setContentAndWait(
      editor,
      docUri,
      "oo::class create Widget {\n  method draw {} {}\n}\n\nset q {\n    aaa\nproc recovered_after_class {} {}\n",
    );
    const names = await symbolsFor(docUri);
    assert.ok(
      names.includes("recovered_after_class"),
      `tail proc not recovered past a user-class recovery signal; symbols=[${names}]`,
    );
  });

  test("a namespace-qualified proc call recovers the tail", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    await setContentAndWait(
      editor,
      docUri,
      "namespace eval myns {\n  proc helper {x} {return $x}\n}\n\nset q {\n  aaa\nmyns::helper 1\nproc recovered_after_ns {} {}\n",
    );
    const names = await symbolsFor(docUri);
    assert.ok(
      names.includes("recovered_after_ns"),
      `tail proc not recovered past a namespace-qualified call; symbols=[${names}]`,
    );
  });

  // The known-command signal extends past this file's own procs/classes/
  // aliases to two more layers: a `rename OLD NEW` target (this file), a
  // proc defined in a *different* workspace file the on-disk scan indexes
  // without ever being opened (`workspaceSiblingHelper.tcl`), and a command
  // a `package require`d library provides (`mypkglib/pkgIndex.tcl`, scanned
  // the same way the pre-existing `rbclib` auto-load fixture is). Each is
  // proven live the same way as the `my_helper` case above: a bare `string`
  // chained onto the same line has no subcommand, which always draws its
  // own E001 ("requires a subcommand") once analysed — independent of
  // whatever recognised the target head.

  test("a renamed command recovers the tail", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    const diags = await setContentAndWait(
      editor,
      docUri,
      "rename puts my_puts\n\nset q {\n  aaa\nmy_puts hi; string\n",
      (ds) => hasRecoveryError(ds) && ds.some((d) => codeOf(d) === "E001"),
    );
    assert.ok(hasRecoveryError(diags), `expected a recovery error: [${diags.map(codeOf)}]`);
    assert.ok(
      diags.some((d) => codeOf(d) === "E001"),
      `renamed-command tail call should be live code (chained bare \`string\` \
       has no subcommand): [${diags.map(codeOf)}]`,
    );
  });

  test("a proc defined in a different workspace file recovers the tail", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    // Route through setContentAndWait so nextDiagnosticsPublish is registered
    // BEFORE the edit and acts as a fresh-publish barrier: a raw
    // setTestContent + waitForDiagnostics reads the current set first and can
    // latch onto the *previous* test's stale E001 (the renamed-command test
    // just above also publishes E001 on this shared fixture), passing without
    // ever analysing this content.
    const diags = await setContentAndWait(
      editor,
      docUri,
      "set q {\n  aaa\nworkspace_helper 1; string\n",
      (ds) => hasRecoveryError(ds) && ds.some((d) => codeOf(d) === "E001"),
    );
    assert.ok(hasRecoveryError(diags), `expected a recovery error: [${diags.map(codeOf)}]`);
    assert.ok(
      diags.some((d) => codeOf(d) === "E001"),
      `a call to a sibling-file proc should be live code (chained bare \`string\` \
       has no subcommand): [${diags.map(codeOf)}]`,
    );
  });

  test("a command from a package-required library recovers the tail", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    // Same fresh-publish barrier as the sibling-file test above: avoid latching
    // onto the previous test's stale E001 on this shared fixture.
    const diags = await setContentAndWait(
      editor,
      docUri,
      "package require mypkg\nset q {\n  aaa\nmypkg_helper 1; string\n",
      (ds) => hasRecoveryError(ds) && ds.some((d) => codeOf(d) === "E001"),
    );
    assert.ok(hasRecoveryError(diags), `expected a recovery error: [${diags.map(codeOf)}]`);
    assert.ok(
      diags.some((d) => codeOf(d) === "E001"),
      `a call to a package-provided command should be live code (chained bare \
       \`string\` has no subcommand): [${diags.map(codeOf)}]`,
    );
  });
});

suite("Error Recovery (short-form unterminated quote / brace)", () => {
  // A delimiter left open with content on the *same* line as the opener
  // (the overwhelmingly common real-world typo) must be flagged exactly
  // like a long multi-line run. Previously both detectors required the run
  // to already span multiple lines, so `set x "hello` / `set x {hello`
  // (content then EOF, or content then a single line break) went
  // completely unflagged — not even the generic fallback fired.
  const docUri = getDocUri("errorRecovery.tcl");

  test("a same-line unterminated quote with no newline is flagged", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    const diags = await setContentAndWait(editor, docUri, 'set x "hello', hasRecoveryError);
    assert.ok(hasRecoveryError(diags), `expected a recovery error: [${diags.map(codeOf)}]`);
  });

  test("a same-line unterminated brace with no newline is flagged", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    const diags = await setContentAndWait(editor, docUri, "set x {hello", hasRecoveryError);
    assert.ok(hasRecoveryError(diags), `expected a recovery error: [${diags.map(codeOf)}]`);
  });

  test("content before a single line break still flags an unterminated quote", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    const diags = await setContentAndWait(
      editor,
      docUri,
      'set x "hello\nworld\n',
      hasRecoveryError,
    );
    assert.ok(hasRecoveryError(diags), `expected a recovery error: [${diags.map(codeOf)}]`);
  });

  test("content before a single line break still flags an unterminated brace", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    const diags = await setContentAndWait(
      editor,
      docUri,
      "set x {hello\nworld\n",
      hasRecoveryError,
    );
    assert.ok(hasRecoveryError(diags), `expected a recovery error: [${diags.map(codeOf)}]`);
  });

  test("well-formed empty and short delimiters stay silent", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    for (const src of ['set x ""\n', "set x {}\n", 'set x "hello"\n', "set x {hello}\n"]) {
      const diags = await setContentAndWait(editor, docUri, src);
      assert.ok(
        !hasRecoveryError(diags),
        `well-formed ${JSON.stringify(src)} flagged a recovery error: [${diags.map(codeOf)}]`,
      );
    }
  });
});

suite("Error Recovery (E200 tight highlighting)", () => {
  const docUri = getDocUri("errorRecovery.tcl");

  test("the generic fallback does not span the whole document", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    // The outer class body's `{` never closes; its content contains a
    // stray `}` from the balanced `method bar {}` parameter list, which
    // routes detection to the generic fallback rather than the precise
    // E203 path.
    const src = "oo::class create Foo {\n  method bar {} {\n    puts hi\n";
    const diags = await setContentAndWait(editor, docUri, src, hasRecoveryError);
    assert.ok(hasRecoveryError(diags), `expected a recovery error: [${diags.map(codeOf)}]`);
    const nLines = src.split("\n").length;
    for (const d of diags.filter(isRecoveryError)) {
      assert.ok(
        d.range.end.line < nLines - 1,
        `recovery diagnostic spans to (or past) the document's last line — \
         expected a tight anchor near the unclosed delimiter, not a \
         whole-document underline: ${JSON.stringify(d.range)}`,
      );
    }
  });
});
