import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate } from "./helper";

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
});
