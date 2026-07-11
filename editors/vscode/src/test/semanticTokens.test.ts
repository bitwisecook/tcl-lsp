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
import { getDocUri, activate, setTestContent, pollUntil } from "./helper";

interface DecodedToken {
  line: number;
  char: number;
  length: number;
  type: string;
}

/**
 * Decode the LSP delta-encoded semantic-token stream into absolute tokens,
 * mapping the numeric type index back to its legend name.
 */
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

suite("Semantic Tokens", () => {
  const docUri = getDocUri("simple.tcl");

  test("provides semantic tokens for Tcl file", async () => {
    await activate(docUri);

    const result = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokens",
      docUri,
    )) as vscode.SemanticTokens;

    assert.ok(result, "Semantic tokens result should not be null");
    assert.ok(
      result.data.length > 0,
      `Expected non-empty semantic token data, got length ${result.data.length}`,
    );

    // Semantic token data is encoded as groups of 5 integers:
    // [deltaLine, deltaStart, length, tokenType, tokenModifiers]
    assert.strictEqual(
      result.data.length % 5,
      0,
      `Token data length should be a multiple of 5, got ${result.data.length}`,
    );
  });

  // PR #643 (issue #637): structural keywords (`else`/`elseif`, and `try`'s
  // `on`/`trap`/`finally`) sit at argument positions, not the command-name
  // slot, and used to render as strings.  They must now emit as keyword
  // semantic tokens, while a bareword built-in used as a plain argument
  // (`dict set frame proc "x"`) stays a string.
  test("structural keywords highlight as keywords, bareword builtin stays string", async () => {
    const kwUri = getDocUri("structuralKeywords.tcl");
    const doc = await activate(kwUri);

    const tokens = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokens",
      kwUri,
    )) as vscode.SemanticTokens;
    const legend = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokensLegend",
      kwUri,
    )) as vscode.SemanticTokensLegend;
    assert.ok(tokens && legend, "expected semantic tokens and a legend");

    const decoded = decodeTokens(tokens, legend);
    const textOf = (t: DecodedToken): string =>
      doc.lineAt(t.line).text.substring(t.char, t.char + t.length);

    const keywordWords = new Set(decoded.filter((t) => t.type === "keyword").map(textOf));
    for (const word of ["if", "elseif", "else", "try", "on", "finally"]) {
      assert.ok(
        keywordWords.has(word),
        `expected '${word}' as a keyword token, got ${JSON.stringify([...keywordWords])}`,
      );
    }

    // The `proc` on the `dict set frame proc "..."` line is a plain dict value.
    const procTok = decoded.find((t) => textOf(t) === "proc");
    assert.ok(procTok, "expected a token covering the bareword 'proc'");
    assert.strictEqual(
      procTok.type,
      "string",
      `bareword 'proc' argument must stay a string, got '${procTok.type}'`,
    );
  });

  // Regex-source tracking: a variable holding a regex literal that flows into a
  // `regexp` pattern makes the ORIGINATING `set` literal read as a regex — the
  // full SSA/SCCP-backed pipeline surfaced end-to-end through the extension.
  test("regex-source variable highlights the def-site literal as regex", async () => {
    const uri = getDocUri("regexSource.tcl");
    await activate(uri);

    const legend = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokensLegend",
      uri,
    )) as vscode.SemanticTokensLegend;
    assert.ok(legend, "expected a legend");

    // The retag comes from the enriched, `CompilationUnit`-backed tier, which
    // races a coarse fast path (issue #829) on this request's first arrival —
    // poll rather than asserting on the first synchronous response, matching
    // the "highlighting eventually converges" test below for the same shape.
    let decoded: DecodedToken[] = [];
    await pollUntil(
      async () =>
        (await vscode.commands.executeCommand(
          "vscode.provideDocumentSemanticTokens",
          uri,
        )) as vscode.SemanticTokens,
      (tokens) => {
        decoded = decodeTokens(tokens, legend);
        return decoded.some((t) => t.line === 0 && t.type === "regexpQuantifier");
      },
      { timeout: 20_000, interval: 250, label: "enriched regex-source retag" },
    );

    // The `*` in the def-site literal on line 0 is a regex quantifier.
    assert.ok(
      decoded.some((t) => t.line === 0 && t.type === "regexpQuantifier"),
      `expected a regexpQuantifier on the def-site literal, got ${JSON.stringify(
        decoded.filter((t) => t.line === 0),
      )}`,
    );
  });

  // Issue #760: tcllib commands that carry a script body (`control::do`,
  // `struct::list foreachperm`) or an expression argument (`control::do`'s
  // `while` test, `control::assert`) must recurse into that argument — the
  // inner commands/variables are highlighted rather than emitted as one
  // opaque string — and the `while` sense-word is a keyword.
  test("tcllib control-flow bodies recurse and 'while' is a keyword", async () => {
    const uri = getDocUri("tcllibControlBody.tcl");
    const doc = await activate(uri);

    const tokens = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokens",
      uri,
    )) as vscode.SemanticTokens;
    const legend = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokensLegend",
      uri,
    )) as vscode.SemanticTokensLegend;
    assert.ok(tokens && legend, "expected semantic tokens and a legend");

    const decoded = decodeTokens(tokens, legend);
    const textOf = (t: DecodedToken): string =>
      doc.lineAt(t.line).text.substring(t.char, t.char + t.length);

    // `set` / `puts` inside the recursed script bodies are function tokens.
    const functionWords = new Set(decoded.filter((t) => t.type === "function").map(textOf));
    for (const word of ["set", "puts", "expr"]) {
      assert.ok(
        functionWords.has(word),
        `expected '${word}' as a function token (recursed body), got ${JSON.stringify([
          ...functionWords,
        ])}`,
      );
    }

    // The `while` sense-word between body and test is a keyword.
    const keywordWords = new Set(decoded.filter((t) => t.type === "keyword").map(textOf));
    assert.ok(
      keywordWords.has("while"),
      `expected 'while' as a keyword token, got ${JSON.stringify([...keywordWords])}`,
    );

    // The recursed body/expr arguments surface variables ($total, $perm).
    assert.ok(
      decoded.some((t) => t.type === "variable"),
      "expected recursed bodies to surface variable tokens",
    );
  });

  // Issue #837: the braced body of `uplevel ?level? {…}` runs in another stack
  // frame but is still a Tcl script — it must recurse (its inner commands and
  // variables highlight) instead of being emitted as one opaque string.
  test("uplevel bodies recurse and highlight their inner commands (#837)", async () => {
    const uri = getDocUri("uplevelBody.tcl");
    const doc = await activate(uri);

    const tokens = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokens",
      uri,
    )) as vscode.SemanticTokens;
    const legend = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokensLegend",
      uri,
    )) as vscode.SemanticTokensLegend;
    assert.ok(tokens && legend, "expected semantic tokens and a legend");

    const decoded = decodeTokens(tokens, legend);
    const textOf = (t: DecodedToken): string =>
      doc.lineAt(t.line).text.substring(t.char, t.char + t.length);

    // `foreach` and `namespace` inside the `uplevel 1 {…}` body are keywords.
    const keywordWords = new Set(decoded.filter((t) => t.type === "keyword").map(textOf));
    for (const word of ["foreach", "namespace", "uplevel"]) {
      assert.ok(
        keywordWords.has(word),
        `expected '${word}' as a keyword token (recursed uplevel body), got ${JSON.stringify([
          ...keywordWords,
        ])}`,
      );
    }

    // `set`/`puts` inside the no-level and `#0` bodies are function tokens —
    // proof the bodies with and without a level word both recurse.
    const functionWords = new Set(decoded.filter((t) => t.type === "function").map(textOf));
    for (const word of ["set", "puts"]) {
      assert.ok(
        functionWords.has(word),
        `expected '${word}' as a function token (recursed uplevel body), got ${JSON.stringify([
          ...functionWords,
        ])}`,
      );
    }

    // The `${nameSpc}` reference deep inside the body surfaces a variable token.
    assert.ok(
      decoded.some((t) => t.type === "variable"),
      "expected the recursed uplevel body to surface variable tokens",
    );
  });

  // Issue #757: a braced string literal spanning multiple lines lost its
  // highlighting (the enclosing multi-line `string` token was dropped).  It
  // must now carry a `string` token on every covered line, just like the
  // quoted form.
  test("multi-line braced string literal is highlighted on every line", async () => {
    const uri = getDocUri("multilineString.tcl");
    await activate(uri);

    const tokens = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokens",
      uri,
    )) as vscode.SemanticTokens;
    const legend = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokensLegend",
      uri,
    )) as vscode.SemanticTokensLegend;
    assert.ok(tokens && legend, "expected semantic tokens and a legend");

    const decoded = decodeTokens(tokens, legend);
    const stringLines = new Set(decoded.filter((t) => t.type === "string").map((t) => t.line));
    // The braced literal spans lines 0..2; each must carry a string token.
    for (const line of [0, 1, 2]) {
      assert.ok(
        stringLines.has(line),
        `braced literal line ${line} must carry a string token, got string lines ${JSON.stringify([
          ...stringLines,
        ])}`,
      );
    }
    // The quoted literal spans lines 3..5 — highlighted the same way.
    for (const line of [3, 4, 5]) {
      assert.ok(
        stringLines.has(line),
        `quoted literal line ${line} must carry a string token, got string lines ${JSON.stringify([
          ...stringLines,
        ])}`,
      );
    }
  });

  // Issue #758: the braced case-list form of a plain (non-`-regexp`) `switch`
  // used to be walked as one opaque body, so the commands inside each case
  // body received no semantic tokens and appeared unhighlighted.  They must
  // now be recursed and highlighted like any other script body.
  test("switch case-list bodies are highlighted", async () => {
    const swUri = getDocUri("switchBodies.tcl");
    const doc = await activate(swUri);

    const tokens = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokens",
      swUri,
    )) as vscode.SemanticTokens;
    const legend = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokensLegend",
      swUri,
    )) as vscode.SemanticTokensLegend;
    assert.ok(tokens && legend, "expected semantic tokens and a legend");

    const decoded = decodeTokens(tokens, legend);
    const textOf = (t: DecodedToken): string =>
      doc.lineAt(t.line).text.substring(t.char, t.char + t.length);

    // Commands inside the case bodies are recursed and highlighted.
    const functionWords = new Set(decoded.filter((t) => t.type === "function").map(textOf));
    for (const word of ["set", "puts"]) {
      assert.ok(
        functionWords.has(word),
        `expected '${word}' inside a switch body as a function token, got ${JSON.stringify([...functionWords])}`,
      );
    }

    // `return` inside a body is a control-flow keyword.
    const keywordWords = new Set(decoded.filter((t) => t.type === "keyword").map(textOf));
    assert.ok(
      keywordWords.has("return"),
      `expected 'return' inside a switch body as a keyword token, got ${JSON.stringify([...keywordWords])}`,
    );
  });

  // Issue #774: `variable a b c` inside a TclOO class body declares every name
  // as an instance variable, not just the first.
  test("TclOO body 'variable' declares every name (issue #774)", async () => {
    const uri = getDocUri("tclooVariable.tcl");
    const doc = await activate(uri);

    const tokens = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokens",
      uri,
    )) as vscode.SemanticTokens;
    const legend = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokensLegend",
      uri,
    )) as vscode.SemanticTokensLegend;
    assert.ok(tokens && legend, "expected semantic tokens and a legend");

    const decoded = decodeTokens(tokens, legend);
    const textOf = (t: DecodedToken): string =>
      doc.lineAt(t.line).text.substring(t.char, t.char + t.length);
    const variableWords = new Set(decoded.filter((t) => t.type === "variable").map(textOf));
    for (const name of ["width", "height", "depth"]) {
      assert.ok(
        variableWords.has(name),
        `expected '${name}' as a variable token, got ${JSON.stringify([...variableWords])}`,
      );
    }
  });

  // Issue #776: after `namespace import tcltest::*`, a bare `test` resolves to
  // the tcltest spec — its `-body`/`-result` are options and the body recurses.
  test("imported tcltest 'test' structure is recognised (issue #776)", async () => {
    const uri = getDocUri("tcltestImport.tcl");
    const doc = await activate(uri);

    const tokens = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokens",
      uri,
    )) as vscode.SemanticTokens;
    const legend = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokensLegend",
      uri,
    )) as vscode.SemanticTokensLegend;
    assert.ok(tokens && legend, "expected semantic tokens and a legend");

    const decoded = decodeTokens(tokens, legend);
    const textOf = (t: DecodedToken): string =>
      doc.lineAt(t.line).text.substring(t.char, t.char + t.length);
    const decoratorWords = new Set(decoded.filter((t) => t.type === "decorator").map(textOf));
    for (const opt of ["-body", "-result"]) {
      assert.ok(
        decoratorWords.has(opt),
        `expected '${opt}' as an option (decorator) token, got ${JSON.stringify([...decoratorWords])}`,
      );
    }
    // The -body script is recursed: `set` is a command.
    const functionWords = new Set(decoded.filter((t) => t.type === "function").map(textOf));
    assert.ok(
      functionWords.has("set"),
      `expected the -body script recursed ('set' as function), got ${JSON.stringify([...functionWords])}`,
    );
  });

  // Issue #775: a command substitution in `source`'s argument is highlighted as
  // a command sequence (its head + variables), not one opaque string.
  test("source argument command substitution is tokenised (issue #775)", async () => {
    const uri = getDocUri("sourceArgument.tcl");
    const doc = await activate(uri);

    const tokens = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokens",
      uri,
    )) as vscode.SemanticTokens;
    const legend = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokensLegend",
      uri,
    )) as vscode.SemanticTokensLegend;
    assert.ok(tokens && legend, "expected semantic tokens and a legend");

    const decoded = decodeTokens(tokens, legend);
    const textOf = (t: DecodedToken): string =>
      doc.lineAt(t.line).text.substring(t.char, t.char + t.length);
    assert.ok(
      decoded.some((t) => textOf(t) === "file" && t.type === "function"),
      "expected the [file join …] substitution recursed ('file' as function)",
    );
    assert.ok(
      decoded.some((t) => textOf(t) === "$currentDir" && t.type === "variable"),
      "expected '$currentDir' inside source's argument as a variable",
    );
  });

  // Peer of issue #774: `global a b c` declares every name as a variable, not
  // just the first.
  test("'global' declares every name (peer of #774)", async () => {
    const uri = getDocUri("globalMultiName.tcl");
    const doc = await activate(uri);

    const tokens = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokens",
      uri,
    )) as vscode.SemanticTokens;
    const legend = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokensLegend",
      uri,
    )) as vscode.SemanticTokensLegend;
    assert.ok(tokens && legend, "expected semantic tokens and a legend");

    const decoded = decodeTokens(tokens, legend);
    const textOf = (t: DecodedToken): string =>
      doc.lineAt(t.line).text.substring(t.char, t.char + t.length);
    const variableWords = new Set(decoded.filter((t) => t.type === "variable").map(textOf));
    for (const name of ["alpha", "beta", "gamma"]) {
      assert.ok(
        variableWords.has(name),
        `expected '${name}' in 'global' as a variable token, got ${JSON.stringify([...variableWords])}`,
      );
    }
  });

  // `foreach` / `dict for` iteration variables — a single bareword and each
  // element of a braced list — read as variable declarations.
  test("loop variables highlight as variables", async () => {
    const uri = getDocUri("loopVariables.tcl");
    const doc = await activate(uri);

    const tokens = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokens",
      uri,
    )) as vscode.SemanticTokens;
    const legend = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokensLegend",
      uri,
    )) as vscode.SemanticTokensLegend;
    assert.ok(tokens && legend, "expected semantic tokens and a legend");

    const decoded = decodeTokens(tokens, legend);
    const textOf = (t: DecodedToken): string =>
      doc.lineAt(t.line).text.substring(t.char, t.char + t.length);
    const variableWords = new Set(decoded.filter((t) => t.type === "variable").map(textOf));
    for (const name of ["item", "key", "val", "dkey", "dval"]) {
      assert.ok(
        variableWords.has(name),
        `expected loop variable '${name}' as a variable token, got ${JSON.stringify([...variableWords])}`,
      );
    }
  });

  // Procedure / apply-lambda parameters and `dict map` loop variables read as
  // variable declarations.
  test("parameters and dict-map loop vars highlight as variables", async () => {
    const uri = getDocUri("paramLists.tcl");
    const doc = await activate(uri);

    const tokens = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokens",
      uri,
    )) as vscode.SemanticTokens;
    const legend = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokensLegend",
      uri,
    )) as vscode.SemanticTokensLegend;
    assert.ok(tokens && legend, "expected semantic tokens and a legend");

    const decoded = decodeTokens(tokens, legend);
    const textOf = (t: DecodedToken): string =>
      doc.lineAt(t.line).text.substring(t.char, t.char + t.length);
    const variableWords = new Set(decoded.filter((t) => t.type === "variable").map(textOf));
    for (const name of ["name", "age", "alpha", "beta", "mk", "mv"]) {
      assert.ok(
        variableWords.has(name),
        `expected '${name}' as a variable declaration, got ${JSON.stringify([...variableWords])}`,
      );
    }
  });

  // `upvar` locals and `dict update` var names read as variable declarations.
  test("upvar and dict-update locals highlight as variables", async () => {
    const uri = getDocUri("refVariables.tcl");
    const doc = await activate(uri);

    const tokens = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokens",
      uri,
    )) as vscode.SemanticTokens;
    const legend = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokensLegend",
      uri,
    )) as vscode.SemanticTokensLegend;
    assert.ok(tokens && legend, "expected semantic tokens and a legend");

    const decoded = decodeTokens(tokens, legend);
    const textOf = (t: DecodedToken): string =>
      doc.lineAt(t.line).text.substring(t.char, t.char + t.length);
    const variableWords = new Set(decoded.filter((t) => t.type === "variable").map(textOf));
    for (const name of ["localdata", "cnt", "sum"]) {
      assert.ok(
        variableWords.has(name),
        `expected '${name}' as a variable declaration, got ${JSON.stringify([...variableWords])}`,
      );
    }
  });

  // snit type bodies recurse + highlight via the registry definition-body
  // grammar, exactly like TclOO — no snit-specific token-walker code.
  test("snit type body members highlight", async () => {
    const uri = getDocUri("snitType.tcl");
    const doc = await activate(uri);

    const tokens = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokens",
      uri,
    )) as vscode.SemanticTokens;
    const legend = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokensLegend",
      uri,
    )) as vscode.SemanticTokensLegend;
    assert.ok(tokens && legend, "expected semantic tokens and a legend");

    const decoded = decodeTokens(tokens, legend);
    const textOf = (t: DecodedToken): string =>
      doc.lineAt(t.line).text.substring(t.char, t.char + t.length);
    // Declarations + a method parameter are variables.
    const variableWords = new Set(decoded.filter((t) => t.type === "variable").map(textOf));
    for (const name of ["barks", "count", "volume", "args"]) {
      assert.ok(
        variableWords.has(name),
        `expected snit '${name}' as a variable, got ${JSON.stringify([...variableWords])}`,
      );
    }
    // The snit-specific `typemethod` keyword and the recursed method body.
    const keywordWords = new Set(decoded.filter((t) => t.type === "keyword").map(textOf));
    assert.ok(keywordWords.has("typemethod"), "expected 'typemethod' keyword");
    const functionWords = new Set(decoded.filter((t) => t.type === "function").map(textOf));
    assert.ok(functionWords.has("set"), "expected the recursed constructor body ('set')");
  });

  // [incr Tcl] class bodies recurse + highlight via the same registry
  // definition-body grammar, including the public/protected/private access
  // modifiers (prefix wrappers) — no itcl-specific token-walker code.
  test("itcl class body members highlight", async () => {
    const uri = getDocUri("itclClass.tcl");
    const doc = await activate(uri);

    const tokens = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokens",
      uri,
    )) as vscode.SemanticTokens;
    const legend = (await vscode.commands.executeCommand(
      "vscode.provideDocumentSemanticTokensLegend",
      uri,
    )) as vscode.SemanticTokensLegend;
    assert.ok(tokens && legend, "expected semantic tokens and a legend");

    const decoded = decodeTokens(tokens, legend);
    const textOf = (t: DecodedToken): string =>
      doc.lineAt(t.line).text.substring(t.char, t.char + t.length);
    // Declarations (incl. a `private variable`) + a method parameter.
    const variableWords = new Set(decoded.filter((t) => t.type === "variable").map(textOf));
    for (const name of ["barks", "total", "volume", "secret", "args"]) {
      assert.ok(
        variableWords.has(name),
        `expected itcl '${name}' as a variable, got ${JSON.stringify([...variableWords])}`,
      );
    }
    // Member keywords, including the access modifiers and the inner wrapped
    // keyword.
    const keywordWords = new Set(decoded.filter((t) => t.type === "keyword").map(textOf));
    for (const kw of ["inherit", "method", "public", "private", "constructor"]) {
      assert.ok(keywordWords.has(kw), `expected itcl '${kw}' keyword`);
    }
    const functionWords = new Set(decoded.filter((t) => t.type === "function").map(textOf));
    assert.ok(functionWords.has("set"), "expected the recursed method body ('set')");
  });

  // -- issue #829: semantic tokens must not be starved behind whole-file
  // analysis on a large document ------------------------------------------

  // Mirrors `generate_big_tcl` in
  // rust/tcl-lsp-server/tests/e2e/semantic_tokens_reference_client.rs --
  // enough procs (~6000 lines) that the enriched, `CompilationUnit`-informed
  // `semanticTokens/full` computation reliably exceeds the server's
  // fast-path budget, so a request exercises the coarse-tier fallback the
  // fix added rather than coincidentally finishing fast.
  function generateBigTcl(count: number): string {
    const parts: string[] = ["namespace eval ::bench {\n    variable counter 0\n}\n\n"];
    for (let i = 0; i < count; i++) {
      parts.push(
        `# proc number ${i}\n` +
          `proc ::bench::step${i} {a b} {\n` +
          `    set v${i} [expr {$a + $b}]\n` +
          `    set msg "step ${i} = $v${i}"\n` +
          `    if {$v${i} > 10} {\n` +
          `        set v${i} [expr {$v${i} + 1}]\n` +
          `    }\n` +
          `    return $v${i}\n` +
          `}\n\n`,
      );
    }
    return parts.join("");
  }

  test("issue #829: semantic tokens arrive promptly on a large (600-proc) document", async function () {
    this.timeout(30_000);
    const uri = getDocUri("largeSemanticTokens.tcl");
    const doc = await activate(uri);
    const editor = await vscode.window.showTextDocument(doc);
    const originalContent = doc.getText();
    try {
      const big = generateBigTcl(600);
      await setTestContent(editor, big);

      const started = Date.now();
      const tokens = (await vscode.commands.executeCommand(
        "vscode.provideDocumentSemanticTokens",
        uri,
      )) as vscode.SemanticTokens;
      const elapsed = Date.now() - started;

      assert.ok(tokens, "expected a semantic tokens result for the large document");
      assert.ok(
        tokens.data.length > 0,
        "expected non-empty semantic token data for the large document -- this is " +
          "the 'sometimes never arrives at all' failure mode issue #829 reports",
      );
      // Generous, environment-tolerant bound.  The point of the server's
      // fast-path/coarse-fallback design (a 40ms race against the enriched
      // computation, issue #829) is that first-response latency stops scaling
      // with file size or system load, so this should hold under a debug
      // build / CI contention, not just a tuned release build.
      assert.ok(
        elapsed < 15_000,
        `first semanticTokens/full on a ${big.split("\n").length}-line cold document ` +
          `took ${elapsed}ms -- semantic tokens must never be starved behind the ` +
          "whole-file analysis (issue #829)",
      );
    } finally {
      // The 600 `::bench::step*` procs are visible workspace-wide (via the
      // server's open-document index) for as long as this buffer holds
      // them -- restore the placeholder so later tests' workspace-wide
      // completion / symbol results aren't polluted by this fixture.
      await setTestContent(editor, originalContent);
      await vscode.commands.executeCommand(
        "vscode.executeHoverProvider",
        uri,
        new vscode.Position(0, 0),
      );
    }
  });

  // Issue #829's second half: when the fast path serves the cheap coarse
  // tier for a cold/large document, the enriched (SSA/SCCP-informed)
  // computation keeps running in the background and the server asks the
  // client to re-request via `workspace/semanticTokens/refresh` once it
  // lands and differs from what was served.
  //
  // That JSON-RPC exchange itself is not asserted here.
  // `vscode-languageclient`'s `SemanticTokensFeature` claims the sole
  // `client.onRequest` slot for `workspace/semanticTokens/refresh` during
  // `LanguageClient.start()` (this repo's pinned `vscode-jsonrpc` stores
  // request handlers in a single-entry map keyed by method --
  // `node_modules/vscode-jsonrpc/lib/common/connection.js`'s `onRequest`
  // replaces rather than chains a second registration), so a test handler
  // installed on the same client would silently *replace* the library's own
  // handler instead of observing it -- permanently breaking live-highlighting
  // refresh for every test that runs afterwards in this shared extension
  // host. That risk isn't worth taking for an assertion the native Rust e2e
  // suite already covers directly and safely
  // (`large_file_semantic_tokens_refresh_delivers_enriched_result` in
  // rust/tcl-lsp-server/tests/e2e/semantic_tokens_reference_client.rs, which
  // observes the raw JSON-RPC request over a harness built for exactly
  // that).
  //
  // What this test observes instead is the user-facing consequence: without
  // ever editing the document again, a later
  // `vscode.provideDocumentSemanticTokens` request eventually returns the
  // fully enriched stream -- proving highlighting converges rather than
  // staying permanently stuck on the coarse tier.
  test("issue #829: highlighting eventually converges to the enriched result without an edit", async function () {
    this.timeout(30_000);
    const uri = getDocUri("largeSemanticTokens.tcl");
    const doc = await activate(uri);
    const editor = await vscode.window.showTextDocument(doc);
    const originalContent = doc.getText();
    try {
      // `generateBigTcl` alone exercises no construct the enriched tier
      // treats differently from the coarse one (no `regexp`, no object
      // dispatch), so append a provably-constant regex source -- the same
      // shape the "regex-source variable..." test above and the Rust db's
      // `semantic_tokens_retags_constant_regex_source_true_positive` use --
      // which only the enriched (`CompilationUnit`-informed) tier retags.
      const big =
        generateBigTcl(600) +
        '\nproc ::bench::regex_check {} {\n    set my_re ".*abc"\n    regexp $my_re $s\n}\n';
      await setTestContent(editor, big);
      const regexLine = big.split("\n").findIndex((l) => l.includes("set my_re"));
      assert.ok(regexLine >= 0, "fixture must contain the 'set my_re' line");

      const legend = (await vscode.commands.executeCommand(
        "vscode.provideDocumentSemanticTokensLegend",
        uri,
      )) as vscode.SemanticTokensLegend;

      await pollUntil(
        async () =>
          (await vscode.commands.executeCommand(
            "vscode.provideDocumentSemanticTokens",
            uri,
          )) as vscode.SemanticTokens,
        (tokens) => {
          const decoded = decodeTokens(tokens, legend);
          return decoded.some((t) => t.line === regexLine && t.type === "regexpQuantifier");
        },
        {
          timeout: 20_000,
          interval: 250,
          label: "enriched regex-source retag on the large document",
        },
      );
    } finally {
      // See the previous test's `finally` -- restore the placeholder so the
      // 600 `::bench::*` procs stop being visible workspace-wide.
      await setTestContent(editor, originalContent);
      await vscode.commands.executeCommand(
        "vscode.executeHoverProvider",
        uri,
        new vscode.Position(0, 0),
      );
    }
  });
});
