import * as assert from "assert";
import * as vscode from "vscode";
import {
  getDocUri,
  activate,
  setTestContent,
  nextDiagnosticsPublish,
  waitForDiagnostics,
} from "./helper";

/**
 * Error-recovery correctness contract — driven through the real VS Code
 * extension + packaged language server.
 *
 * Like the lsp_e2e suite, this is intentionally **implementation-agnostic**: it
 * asserts the observable recovery behaviour an editor user depends on, never a
 * server internal or a Python-specific diagnostic code.  It is the editor-side
 * half of the contract the recovery engine must satisfy in *any* implementation
 * — Python today, Rust after the port — so these exact tests re-run unchanged to
 * prove the Rust port behaves correctly end to end.
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

/** Replace the document content and resolve with the resulting diagnostics. */
async function setContentAndWait(
  editor: vscode.TextEditor,
  docUri: vscode.Uri,
  content: string,
): Promise<vscode.Diagnostic[]> {
  const fresh = nextDiagnosticsPublish(docUri, { timeout: 15_000 });
  await setTestContent(editor, content);
  await fresh;
  // A second short wait lets the deep/async pass settle before we read.
  return waitForDiagnostics(docUri, { timeout: 5_000, predicate: () => true });
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
    const diags = await setContentAndWait(editor, docUri, "set x [foo bar\nputs hi\n");
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
    // `if {$x > 5` is an unterminated braced *expression*; recovery must let the
    // following command be analysed — a bare `set` still raises its arity error.
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    const diags = await setContentAndWait(editor, docUri, "if {$x > 5\nset\n");
    assert.ok(
      diags.some((d) => codeOf(d) === "E002"),
      `tail \`set\` after \`if {\` should arity-error; got [${diags.map(codeOf)}]`,
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
    const diags = await setContentAndWait(editor, docUri, "set x [a [b [c [d [e\nputs tail\n");
    assert.ok(Array.isArray(diags), "server returned promptly with a diagnostics array");
  });

  // C4 ----------------------------------------------------------------------
  test("C4: no exact-duplicate diagnostics are published", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;
    const diags = await setContentAndWait(
      editor,
      docUri,
      'set x "\nif {1} {\n  puts [foo\n}\nset\n',
    );
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

    diags = await setContentAndWait(editor, docUri, "set x [foo bar\nputs hi\n");
    assert.ok(hasRecoveryError(diags), `break not flagged: [${diags.map(codeOf)}]`);

    diags = await setContentAndWait(editor, docUri, "set x [foo bar]\nputs hi\n");
    assert.ok(!hasRecoveryError(diags), `fix did not clear: [${diags.map(codeOf)}]`);
  });
});
