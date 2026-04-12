import * as assert from "assert";
import * as vscode from "vscode";
import { activate, getDocUri } from "./helper";

suite("Document Highlight", () => {
  const docUri = getDocUri("procs.tcl");

  test("highlights a proc name at its definition", async () => {
    await activate(docUri);

    // `fib` declaration at line 1 col 6.
    const position = new vscode.Position(1, 6);
    const highlights = (await vscode.commands.executeCommand(
      "vscode.executeDocumentHighlights",
      docUri,
      position,
    )) as vscode.DocumentHighlight[] | undefined;

    assert.ok(highlights, "documentHighlight result should not be null");
    assert.ok(
      highlights.length >= 2,
      `Expected at least 2 highlights (decl + recursive calls), got ${highlights.length}`,
    );
  });

  test("highlight ranges include the definition", async () => {
    await activate(docUri);
    const position = new vscode.Position(1, 6);
    const highlights = (await vscode.commands.executeCommand(
      "vscode.executeDocumentHighlights",
      docUri,
      position,
    )) as vscode.DocumentHighlight[] | undefined;

    assert.ok(highlights);
    const declLine = highlights.some((h) => h.range.start.line === 1);
    assert.ok(declLine, "Expected at least one highlight on the declaration line");
  });
});
