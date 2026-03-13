import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate } from "./helper";

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
});
