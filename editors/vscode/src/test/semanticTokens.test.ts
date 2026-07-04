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

  // Regex-source tracking: a variable holding a regex literal that flows into a
  // `regexp` pattern makes the ORIGINATING `set` literal read as a regex — the
  // full SSA/SCCP-backed pipeline surfaced end-to-end through the extension.
  test("regex-source variable highlights the def-site literal as regex", async () => {
    const uri = getDocUri("regexSource.tcl");
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
    // The `*` in the def-site literal on line 0 is a regex quantifier.
    assert.ok(
      decoded.some((t) => t.line === 0 && t.type === "regexpQuantifier"),
      `expected a regexpQuantifier on the def-site literal, got ${JSON.stringify(
        decoded.filter((t) => t.line === 0),
      )}`,
    );
  });
});
