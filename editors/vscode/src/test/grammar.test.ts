import * as assert from "assert";
import * as fs from "fs";
import * as path from "path";
import * as oniguruma from "vscode-oniguruma";
import * as vsctm from "vscode-textmate";

// TextMate grammar tokenisation tests.  These exercise the *static* syntax
// grammar (editors/vscode/syntaxes/tcl.tmLanguage.json) directly — the layer
// that colours code before/without the LSP's semantic tokens, and the only
// layer GitHub and other TextMate consumers ever see.  Regression cover for
// issue #759: a comment whose line ends in an unescaped backslash continues
// onto the next physical line, and that continuation must stay coloured as a
// comment.

const GRAMMAR_PATH = path.resolve(__dirname, "../../syntaxes/tcl.tmLanguage.json");

function makeRegistry(): vsctm.Registry {
  const wasmPath = path.join(path.dirname(require.resolve("vscode-oniguruma")), "onig.wasm");
  const wasmBin = fs.readFileSync(wasmPath).buffer;
  const onigLib = oniguruma.loadWASM(wasmBin).then(() => ({
    createOnigScanner: (patterns: string[]) => new oniguruma.OnigScanner(patterns),
    createOnigString: (s: string) => new oniguruma.OnigString(s),
  }));
  return new vsctm.Registry({
    onigLib,
    loadGrammar: (scopeName: string) => {
      if (scopeName === "source.tcl") {
        const raw = fs.readFileSync(GRAMMAR_PATH, "utf8");
        return Promise.resolve(vsctm.parseRawGrammar(raw, GRAMMAR_PATH));
      }
      return Promise.resolve(null);
    },
  });
}

/**
 * Tokenise a multi-line source and return, for each line, whether every
 * non-whitespace character carries a comment scope.  A line with no
 * non-whitespace content is reported as `null`.
 */
async function commentMask(grammar: vsctm.IGrammar, source: string): Promise<(boolean | null)[]> {
  const lines = source.split("\n");
  const mask: (boolean | null)[] = [];
  let ruleStack: vsctm.StateStack = vsctm.INITIAL;
  for (const line of lines) {
    const result = grammar.tokenizeLine(line, ruleStack);
    let sawContent = false;
    let allComment = true;
    for (const tok of result.tokens) {
      const text = line.slice(tok.startIndex, tok.endIndex);
      if (text.trim() === "") continue;
      sawContent = true;
      if (!tok.scopes.some((s) => s.startsWith("comment.line.number-sign"))) {
        allComment = false;
      }
    }
    mask.push(sawContent ? allComment : null);
    ruleStack = result.ruleStack;
  }
  return mask;
}

suite("TextMate grammar: comment continuation (#759)", () => {
  let grammar: vsctm.IGrammar;

  suiteSetup(async () => {
    const registry = makeRegistry();
    const loaded = await registry.loadGrammar("source.tcl");
    assert.ok(loaded, "source.tcl grammar should load");
    grammar = loaded;
  });

  test("a trailing backslash continues the comment onto the next line", async () => {
    const mask = await commentMask(grammar, "# this is a comment \\\nstill a comment\nset x 1\n");
    assert.strictEqual(mask[0], true, "line 0 is a comment");
    assert.strictEqual(mask[1], true, "line 1 is the continued comment");
    assert.strictEqual(mask[2], false, "line 2 is code, not a comment");
  });

  test("multiple continuation lines all stay comments", async () => {
    const mask = await commentMask(grammar, "# level one \\\nlevel two \\\nlevel three\nset y 2\n");
    assert.deepStrictEqual(mask.slice(0, 4), [true, true, true, false]);
  });

  test("an escaped backslash pair does not continue the comment", async () => {
    // ``# ... \\`` ends in an *even* run of backslashes: the last backslash is
    // escaped, so the line does NOT continue (matching the lexer).
    const mask = await commentMask(grammar, "# escaped end \\\\\nset z 3\n");
    assert.strictEqual(mask[0], true, "line 0 is a comment");
    assert.strictEqual(mask[1], false, "line 1 is code — no continuation past \\\\");
  });

  test("an odd run of trailing backslashes continues the comment", async () => {
    // Three backslashes = one escaped pair + one continuation backslash.
    const mask = await commentMask(grammar, "# three \\\\\\\nyes continued\nset a 4\n");
    assert.strictEqual(mask[0], true, "line 0 is a comment");
    assert.strictEqual(mask[1], true, "line 1 is the continued comment");
    assert.strictEqual(mask[2], false, "line 2 is code");
  });

  test("a plain comment does not bleed onto the following line", async () => {
    const mask = await commentMask(grammar, "# plain comment\nset x 1\n");
    assert.strictEqual(mask[0], true);
    assert.strictEqual(mask[1], false);
  });

  test("a plain comment does not bleed to EOF across many code lines", async () => {
    // A non-continuation comment must pop its context at the line end.  The
    // `commentMask` helper threads the rule stack across `tokenizeLine` calls
    // exactly as an editor does; vscode-textmate appends a `\n` to every line's
    // buffer (including the last), so the `(?=\n)` end pattern always matches at
    // the line end and the following code is never scoped as a comment.
    const mask = await commentMask(
      grammar,
      "# just a comment\nset a 1\nset b 2\nputs done\nreturn\n",
    );
    assert.strictEqual(mask[0], true, "line 0 is the comment");
    assert.deepStrictEqual(
      mask.slice(1, 5),
      [false, false, false, false],
      "no following line is scoped as a comment",
    );
  });

  test("a trailing ';#' comment continues onto the next line", async () => {
    const mask = await commentMask(grammar, "set x 1 ;# trailing \\\nstill comment\nset y 2\n");
    assert.strictEqual(mask[0], false, "line 0 mixes code and a comment tail");
    assert.strictEqual(mask[1], true, "line 1 is the continued comment");
    assert.strictEqual(mask[2], false, "line 2 is code");
    // The comment tail on line 0 must itself be scoped as a comment.
    const first = grammar.tokenizeLine("set x 1 ;# trailing \\", vsctm.INITIAL);
    const hashTok = first.tokens.find((t) =>
      "set x 1 ;# trailing \\".slice(t.startIndex).startsWith("#"),
    );
    assert.ok(hashTok, "found the '#' token");
    assert.ok(
      hashTok!.scopes.some((s) => s.startsWith("comment.line.number-sign")),
      "the '#...' tail is a comment",
    );
  });

  test("'#' not at command position is not a comment", async () => {
    const result = grammar.tokenizeLine("set d $x#y", vsctm.INITIAL);
    const hashTok = result.tokens.find((t) => "set d $x#y".slice(t.startIndex).startsWith("#"));
    if (hashTok) {
      assert.ok(
        !hashTok.scopes.some((s) => s.startsWith("comment.line.number-sign")),
        "a mid-word '#' must not start a comment",
      );
    }
  });
});
