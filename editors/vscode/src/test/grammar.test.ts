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

function makeOnigLib(): Promise<vsctm.IOnigLib> {
  const wasmPath = path.join(path.dirname(require.resolve("vscode-oniguruma")), "onig.wasm");
  const wasmBin = fs.readFileSync(wasmPath).buffer;
  return oniguruma.loadWASM(wasmBin).then(() => ({
    createOnigScanner: (patterns: string[]) => new oniguruma.OnigScanner(patterns),
    createOnigString: (s: string) => new oniguruma.OnigString(s),
  }));
}

function makeRegistry(): vsctm.Registry {
  const onigLib = makeOnigLib();
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

// Regression cover for issue #749: the grammar recurses `source.tcl` into every
// brace group, so a bare control word (`for`, `else`, `in`, …) inside an unknown
// command's braced *data* argument — e.g. `argparse -help {...}` — is coloured as
// a keyword.  A context-free TextMate grammar cannot tell a script body from
// data, so this is a known, documented limitation: the fix is the LSP semantic
// token overlay (which classifies the whole brace group as a string), and the
// `highlightingHealth` checks surface the cases where that overlay isn't active.
// This test pins the grammar behaviour so a future context-aware change is a
// conscious, reviewed update rather than a silent regression.
suite("TextMate grammar: keywords inside unknown-command braces (#749)", () => {
  let grammar: vsctm.IGrammar;

  suiteSetup(async () => {
    const registry = makeRegistry();
    const loaded = await registry.loadGrammar("source.tcl");
    assert.ok(loaded, "source.tcl grammar should load");
    grammar = loaded;
  });

  /** Scopes applied to the first standalone occurrence of `word` in `line`. */
  function scopesForWord(line: string, word: string): string[] | undefined {
    const result = grammar.tokenizeLine(line, vsctm.INITIAL);
    const tok = result.tokens.find((t) => line.slice(t.startIndex, t.endIndex).trim() === word);
    return tok?.scopes;
  }

  test("a control word inside a braced -help argument is scoped as a keyword", () => {
    const scopes = scopesForWord("argparse -help {termination occurs for the loop}", "for");
    assert.ok(scopes, "found the 'for' token");
    assert.ok(
      scopes.includes("keyword.control.tcl"),
      `grammar colours 'for' inside braces as a keyword (got ${scopes.join(", ")})`,
    );
  });

  test("'else' inside any braced data argument is likewise scoped as a keyword", () => {
    const scopes = scopesForWord("set opts {run else stop}", "else");
    assert.ok(scopes, "found the 'else' token");
    assert.ok(scopes.includes("keyword.control.tcl"), scopes.join(", "));
  });

  test("the same word at real command position is also a keyword (grammar is context-free)", () => {
    const scopes = scopesForWord("for {set i 0} {$i < 3} {incr i} {}", "for");
    assert.ok(scopes, "found the 'for' token");
    assert.ok(scopes.includes("keyword.control.tcl"), scopes.join(", "));
  });
});

// Regression cover for issue #862: `lmap` carries the registry's
// LANGUAGE_KEYWORD trait (it binds loop variables, like `foreach`), but the
// grammar used to list it among the plain "common built-in commands"
// (support.function.tcl) — so it visibly flipped colour between the
// TextMate-only fallback and the LSP's semantic-token overlay. It now lives
// in the same keyword.control.tcl alternation as foreach/for/while.
suite("TextMate grammar: lmap is a control keyword, not a plain builtin (#862)", () => {
  let grammar: vsctm.IGrammar;

  suiteSetup(async () => {
    const registry = makeRegistry();
    const loaded = await registry.loadGrammar("source.tcl");
    assert.ok(loaded, "source.tcl grammar should load");
    grammar = loaded;
  });

  function scopesForWord(line: string, word: string): string[] | undefined {
    const result = grammar.tokenizeLine(line, vsctm.INITIAL);
    const tok = result.tokens.find((t) => line.slice(t.startIndex, t.endIndex).trim() === word);
    return tok?.scopes;
  }

  test("lmap scopes the same as foreach", () => {
    const lmapScopes = scopesForWord("lmap x {1 2 3} {expr {$x * 2}}", "lmap");
    assert.ok(lmapScopes, "found the 'lmap' token");
    assert.ok(lmapScopes.includes("keyword.control.tcl"), lmapScopes.join(", "));
    assert.ok(!lmapScopes.includes("support.function.tcl"), lmapScopes.join(", "));
  });

  test("plain builtins next to lmap stay support.function.tcl", () => {
    const scopes = scopesForWord("lappend result 1", "lappend");
    assert.ok(scopes, "found the 'lappend' token");
    assert.ok(scopes.includes("support.function.tcl"), scopes.join(", "));
  });
});

// Issue #903: the grammar is the paint the user sees before the server answers,
// and the only paint anywhere the server never runs (GitHub/Linguist, a file too
// large for semantic tokens, an editor with no extension). Where it disagrees
// with the semantic-token layer, the colour visibly flips once the LSP replies.
//
// The semantic layer types proc/method parameters as `parameter`, TclOO class
// names as `class` and method names as `method`. The grammar previously typed
// none of them: it scoped a proc's *name* and nothing else. Sublime's bundled
// Tcl syntax has scoped proc parameters for years; the TextMate bundle scopes
// neither, and no surveyed Tcl grammar scopes TclOO at all.
suite("TextMate grammar: definitions agree with the semantic-token types (#903)", () => {
  let grammar: vsctm.IGrammar;

  suiteSetup(async () => {
    const registry = makeRegistry();
    const loaded = await registry.loadGrammar("source.tcl");
    assert.ok(loaded, "source.tcl grammar should load");
    grammar = loaded;
  });

  /** Scopes applied to the first standalone occurrence of `word` in `line`. */
  function scopesForWord(line: string, word: string): string[] | undefined {
    const result = grammar.tokenizeLine(line, vsctm.INITIAL);
    const tok = result.tokens.find((t) => line.slice(t.startIndex, t.endIndex).trim() === word);
    return tok?.scopes;
  }

  test("proc parameters are variable.parameter, matching the semantic `parameter` type", () => {
    const scopes = scopesForWord("proc greet {name} { puts $name }", "name");
    assert.ok(scopes, "found the 'name' parameter token");
    assert.ok(scopes.includes("variable.parameter.tcl"), scopes.join(", "));
  });

  // The closing brace of a defaulted parameter must not end the parameter list:
  // a naive begin/end rule stops at the `}` of `{greeting hello}` and leaves
  // every later parameter unscoped.
  test("a parameter with a default value does not terminate the parameter list", () => {
    const line = "proc greet {greeting {name world} {punct !}} { }";
    for (const word of ["greeting", "name", "punct"]) {
      const scopes = scopesForWord(line, word);
      assert.ok(scopes, `found the '${word}' token`);
      assert.ok(
        scopes.includes("variable.parameter.tcl"),
        `'${word}' should be a parameter (got ${scopes.join(", ")})`,
      );
    }
  });

  test("a proc name is still entity.name.function", () => {
    const scopes = scopesForWord("proc greet {name} { }", "greet");
    assert.ok(scopes, "found the 'greet' token");
    assert.ok(scopes.includes("entity.name.function.tcl"), scopes.join(", "));
  });

  test("a TclOO class name is entity.name.class, matching the semantic `class` type", () => {
    const scopes = scopesForWord("oo::class create Account {", "Account");
    assert.ok(scopes, "found the 'Account' token");
    assert.ok(scopes.includes("entity.name.class.tcl"), scopes.join(", "));
  });

  test("a superclass is scoped as an inherited class", () => {
    const scopes = scopesForWord("    superclass Base", "Base");
    assert.ok(scopes, "found the 'Base' token");
    assert.ok(scopes.includes("entity.other.inherited-class.tcl"), scopes.join(", "));
  });

  test("a method name and its parameters are scoped", () => {
    const line = "    method deposit {amount} { incr balance $amount }";
    const name = scopesForWord(line, "deposit");
    assert.ok(name, "found the 'deposit' token");
    assert.ok(name.includes("entity.name.function.tcl"), name.join(", "));
    const param = scopesForWord(line, "amount");
    assert.ok(param, "found the 'amount' token");
    assert.ok(param.includes("variable.parameter.tcl"), param.join(", "));
  });

  test("a constructor's parameters are scoped", () => {
    const scopes = scopesForWord("    constructor {balance} { }", "balance");
    assert.ok(scopes, "found the 'balance' token");
    assert.ok(scopes.includes("variable.parameter.tcl"), scopes.join(", "));
  });

  // The guard from issue #637: a bareword `proc` used as a *value* must not
  // start a definition and swallow the following quote.
  test("a bareword 'proc' used as an argument does not start a definition", () => {
    const scopes = scopesForWord('dict set frame proc "x"', "proc");
    assert.ok(scopes, "found the 'proc' token");
    assert.ok(
      !scopes.includes("storage.type.function.tcl"),
      `'proc' as data must not be scoped as a definition (got ${scopes.join(", ")})`,
    );
  });

  // Issue #904. `throw` was a control keyword and `error` an ordinary library
  // call, so the two halves of one construct came out in different colours.
  // Every Tcl grammar that *has* a function category agrees `error` is not one.
  test("catch, error and throw are all control keywords", () => {
    for (const [line, word] of [
      ["catch { error boom } msg", "catch"],
      ["catch { error boom } msg", "error"],
      ["throw {MY ERR} oops", "throw"],
    ]) {
      const scopes = scopesForWord(line, word);
      assert.ok(scopes, `found the '${word}' token`);
      assert.ok(
        scopes.includes("keyword.control.tcl"),
        `'${word}' should be a control keyword (got ${scopes.join(", ")})`,
      );
    }
  });

  test("a computing builtin is still a function, not a keyword", () => {
    const scopes = scopesForWord("puts $msg", "puts");
    assert.ok(scopes, "found the 'puts' token");
    assert.ok(scopes.includes("support.function.tcl"), scopes.join(", "));
  });

  test("proc, method and constructor are declaration keywords", () => {
    for (const [line, word] of [
      ["proc greet {name} { }", "proc"],
      ["    method get {} { }", "method"],
      ["    constructor {bal} { }", "constructor"],
    ]) {
      const scopes = scopesForWord(line, word);
      assert.ok(scopes, `found the '${word}' token`);
      assert.ok(scopes.includes("storage.type.function.tcl"), scopes.join(", "));
    }
  });
});

// Issue #903: `tcl-apl` and `tcl-bigip` were both contributed with
// `tcl.tmLanguage.json` — the *Tcl* grammar, which contains no APL rule and no
// BIG-IP rule. A `bigip.conf` was therefore painted with Tcl rules and then
// replaced wholesale by a 34-type BIG-IP token stream once the server answered.
// They now have their own grammars.
//
// The design rule these tests hold: the grammar may be *less specific* than the
// semantic layer (it emits `entity.name.type.bigip` where the server can tell a
// pool from a monitor and emits `entity.name.type.pool.bigip` — same scope
// family, same colour), but it must never paint a span a *different* colour than
// the server is about to.
suite("TextMate grammars: BIG-IP config and APL (#903)", () => {
  let bigip: vsctm.IGrammar;
  let apl: vsctm.IGrammar;

  suiteSetup(async () => {
    const registry = new vsctm.Registry({
      onigLib: makeOnigLib(),
      loadGrammar: (scopeName: string) => {
        const file = {
          "source.tcl": "tcl.tmLanguage.json",
          "source.tcl-bigip": "bigip.tmLanguage.json",
          "source.tcl-apl": "apl.tmLanguage.json",
        }[scopeName];
        if (!file) {
          return Promise.resolve(null);
        }
        const p = path.resolve(__dirname, "../../syntaxes", file);
        return Promise.resolve(vsctm.parseRawGrammar(fs.readFileSync(p, "utf8"), p));
      },
    });
    const b = await registry.loadGrammar("source.tcl-bigip");
    const a = await registry.loadGrammar("source.tcl-apl");
    assert.ok(b && a, "both grammars should load");
    bigip = b;
    apl = a;
  });

  /** Last scope applied to each distinct trimmed token across `lines`. */
  function scopeMap(grammar: vsctm.IGrammar, lines: string[]): Map<string, string> {
    const out = new Map<string, string>();
    let stack = vsctm.INITIAL;
    for (const line of lines) {
      const r = grammar.tokenizeLine(line, stack);
      stack = r.ruleStack;
      for (const t of r.tokens) {
        const text = line.slice(t.startIndex, t.endIndex).trim();
        if (text) {
          out.set(text, t.scopes[t.scopes.length - 1]);
        }
      }
    }
    return out;
  }

  test("a BIG-IP config is not painted as Tcl", () => {
    const scopes = scopeMap(bigip, [
      "ltm pool /Common/api_pool {",
      "    monitor /Common/http",
      "}",
    ]);
    assert.strictEqual(scopes.get("ltm"), "keyword.control.bigip");
    assert.strictEqual(scopes.get("Common"), "entity.name.namespace.partition.bigip");
    assert.strictEqual(scopes.get("api_pool"), "entity.name.type.bigip");
    assert.strictEqual(scopes.get("/Common/http"), "entity.name.type.monitor.bigip");
  });

  // A pool member's name *is* an address. If it fell through to the generic
  // object-path rule it would be painted as an object name — a different colour
  // than the server's `ipAddress`, which is the one place these two layers could
  // have contradicted each other outright.
  test("an address in a member path is an address, not an object name", () => {
    const scopes = scopeMap(bigip, [
      "ltm virtual /Common/vs1 {",
      "    destination /Common/192.0.2.10:443",
      "}",
    ]);
    assert.strictEqual(scopes.get("192.0.2.10"), "constant.numeric.ip-address.bigip");
    assert.strictEqual(scopes.get("443"), "constant.numeric.port.bigip");
    assert.strictEqual(scopes.get("vs1"), "entity.name.type.bigip");
  });

  test("an `ltm rule` body is highlighted as Tcl, as the server walks it", () => {
    const scopes = scopeMap(bigip, [
      "ltm rule /Common/my_rule {",
      "when HTTP_REQUEST {",
      '    if { [HTTP::uri] starts_with "/api" } { }',
      "}",
      "}",
    ]);
    assert.strictEqual(scopes.get("when"), "keyword.control.tcl");
    assert.strictEqual(scopes.get("if"), "keyword.control.tcl");
  });

  test("APL sections, fields and attributes are scoped", () => {
    const scopes = scopeMap(apl, [
      "section pool_config {",
      '    string pool_name required display "large"',
      "}",
    ]);
    assert.strictEqual(scopes.get("section"), "keyword.control.apl");
    assert.strictEqual(scopes.get("pool_config"), "entity.name.section.apl");
    assert.strictEqual(scopes.get("string"), "keyword.other.apl");
    assert.strictEqual(scopes.get("pool_name"), "variable.other.field.apl");
    assert.strictEqual(scopes.get("required"), "entity.other.attribute-name.apl");
  });

  // The validator name sits *inside* the quotes. TextMate resolves competing
  // patterns by which starts earliest, so the opening quote would otherwise win
  // and the name would be painted as ordinary string content.
  test("a validator name inside its quotes is a validator, but only in that position", () => {
    const scopes = scopeMap(apl, [
      '    string addr validator "IpAddress"',
      '    string x display "Number"',
    ]);
    assert.strictEqual(scopes.get("IpAddress"), "support.constant.validator.apl");
    assert.strictEqual(scopes.get("Number"), "string.quoted.double.apl");
  });

  test("an APL bracket expression is highlighted as Tcl", () => {
    const scopes = scopeMap(apl, ["    string x default [ expr { 1 + 1 } ]"]);
    assert.strictEqual(scopes.get("expr"), "support.function.tcl");
    assert.strictEqual(scopes.get("+"), "keyword.operator.arithmetic.tcl");
  });

  // `optional` guards a block on APL's own tiny expression language, in
  // PARENTHESES — not on a Tcl bracket expression. The first draft of this
  // grammar assumed brackets, which no real APL file uses.
  test("optional takes a parenthesised APL condition, not Tcl", () => {
    const scopes = scopeMap(apl, ['    optional ( basic.protocol == "tcp" ) {']);
    assert.strictEqual(scopes.get("optional"), "keyword.control.optional.apl");
    assert.strictEqual(scopes.get("basic.protocol"), "variable.other.field.apl");
    assert.strictEqual(scopes.get("=="), "keyword.operator.apl");
  });

  // `define <base-type> <new-name>` — the SECOND word is the type being reused.
  // Scoping `choice` as the definition's name is a bug both this grammar's first
  // draft and bitwisecook/vscode-iApp shipped.
  test("define names the new type, not the base type", () => {
    const scopes = scopeMap(apl, ["define choice lb_method {"]);
    assert.strictEqual(scopes.get("define"), "keyword.other.define.apl");
    assert.strictEqual(scopes.get("choice"), "keyword.other.apl");
    assert.strictEqual(scopes.get("lb_method"), "entity.name.function.apl");
  });

  // Without a trailing word boundary on the field-type alternation, `yesno`
  // matches inside the user-defined type name `yesno_choice`.
  test("a field-type keyword does not match inside a longer identifier", () => {
    const scopes = scopeMap(apl, ["define choice yesno_choice {"]);
    assert.strictEqual(scopes.get("yesno_choice"), "entity.name.function.apl");
    assert.strictEqual(scopes.get("yesno"), undefined);
  });
});

// Marked-up grammar fixtures (#903/#904).
//
// `src/test/grammarFixtures/*` are ordinary, valid source files in which each
//
//     #   ^^^^ some.scope.name
//
// line asserts that the caret span — at those columns of the code line above —
// carries that TextMate scope. This is the `vscode-tmgrammar-test` convention:
// the assertions are comments, so the fixture still parses as the real thing and
// the grammar sees exactly what an editor would.
//
// The point is coverage that is cheap to *extend*: adding a case is one line of
// code plus one line of carets, so there is no excuse for a rule shipping
// unasserted. Hand-written `scopesForWord` tests can only ask "does this word
// have this scope somewhere"; these pin an exact span at an exact column, which
// is what actually catches a rule that ends early or swallows a delimiter.
suite("TextMate grammars: marked-up fixtures (#903)", () => {
  const FIXTURES = path.resolve(__dirname, "../../src/test/grammarFixtures");
  const SCOPE_FOR_EXT: Record<string, string> = {
    ".tcl": "source.tcl",
    ".conf": "source.tcl-bigip",
    ".apl": "source.tcl-apl",
  };
  const GRAMMAR_FOR_SCOPE: Record<string, string> = {
    "source.tcl": "tcl.tmLanguage.json",
    "source.tcl-bigip": "bigip.tmLanguage.json",
    "source.tcl-apl": "apl.tmLanguage.json",
  };

  let registry: vsctm.Registry;

  suiteSetup(() => {
    registry = new vsctm.Registry({
      onigLib: makeOnigLib(),
      loadGrammar: (scopeName: string) => {
        const file = GRAMMAR_FOR_SCOPE[scopeName];
        if (!file) {
          return Promise.resolve(null);
        }
        const p = path.resolve(__dirname, "../../syntaxes", file);
        return Promise.resolve(vsctm.parseRawGrammar(fs.readFileSync(p, "utf8"), p));
      },
    });
  });

  /** `#   ^^^^ scope.name` — carets mark a span of the preceding code line. */
  const ASSERTION = /^\s*#\s*(\^+)\s+(\S+)\s*$/;

  for (const file of fs.readdirSync(FIXTURES)) {
    const scopeName = SCOPE_FOR_EXT[path.extname(file)];
    if (!scopeName) {
      continue;
    }

    test(`${file} matches its marked-up scopes`, async () => {
      const grammar = await registry.loadGrammar(scopeName);
      assert.ok(grammar, `${scopeName} should load`);

      const lines = fs.readFileSync(path.join(FIXTURES, file), "utf8").split("\n");
      const failures: string[] = [];
      let checked = 0;

      // Tokenise every line in order — including the assertion comments — so the
      // rule stack is exactly what an editor would have at each point.
      let stack: vsctm.StateStack = vsctm.INITIAL;
      let codeLine = "";
      let codeTokens: vsctm.IToken[] = [];

      lines.forEach((line, i) => {
        const m = ASSERTION.exec(line);
        const result = grammar.tokenizeLine(line, stack);

        if (m) {
          const start = line.indexOf(m[1]);
          const end = start + m[1].length;
          const expected = m[2];
          // Every column under the carets must carry the expected scope.
          for (let col = start; col < end; col++) {
            const tok = codeTokens.find((t) => col >= t.startIndex && col < t.endIndex);
            const scopes = tok?.scopes ?? [];
            if (!scopes.includes(expected)) {
              const ch = codeLine[col] ?? "<eol>";
              failures.push(
                `${file}:${i + 1} col ${col} (${JSON.stringify(ch)}) — expected ${expected}, got [${scopes.join(", ")}]`,
              );
              break; // one failure per assertion is enough
            }
          }
          checked++;
        } else {
          codeLine = line;
          codeTokens = result.tokens;
        }
        stack = result.ruleStack;
      });

      assert.ok(checked > 0, `${file} contains no scope assertions`);
      assert.deepStrictEqual(failures, [], `\n${failures.join("\n")}\n`);
    });
  }
});
