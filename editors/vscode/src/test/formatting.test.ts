import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate, setTestContent } from "./helper";

suite("Formatting", () => {
  // Use a dedicated fixture to avoid mutating files used by other suites.
  const docUri = getDocUri("formatting.tcl");

  test("formats a poorly-indented document", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;

    // Set poorly formatted content
    const badContent = ["proc greet {name} {", 'set msg "Hello, $name"', "puts $msg", "}", ""].join(
      "\n",
    );

    await setTestContent(editor, badContent);

    const edits = (await vscode.commands.executeCommand(
      "vscode.executeFormatDocumentProvider",
      docUri,
      {
        tabSize: 4,
        insertSpaces: true,
      } as vscode.FormattingOptions,
    )) as vscode.TextEdit[];

    assert.ok(edits, "Format result should not be null");
    assert.ok(edits.length > 0, "Should have at least one edit");

    // Apply the edits
    const wsEdit = new vscode.WorkspaceEdit();
    wsEdit.set(docUri, edits);
    await vscode.workspace.applyEdit(wsEdit);

    const formatted = editor.document.getText();

    // The body of proc should be indented
    assert.ok(
      formatted.includes("    set msg"),
      `Body should be indented with 4 spaces, got:\n${formatted}`,
    );
    assert.ok(
      formatted.includes("    puts"),
      `Body should be indented with 4 spaces, got:\n${formatted}`,
    );
  });

  test("already-formatted code produces no edits", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;

    // Set already well-formatted content
    const goodContent = ["set x 10", "set y 20", "puts $x", ""].join("\n");

    await setTestContent(editor, goodContent);

    const edits = (await vscode.commands.executeCommand(
      "vscode.executeFormatDocumentProvider",
      docUri,
      {
        tabSize: 4,
        insertSpaces: true,
      } as vscode.FormattingOptions,
    )) as vscode.TextEdit[];

    // No edits needed for already-formatted code
    const editCount = edits ? edits.length : 0;
    assert.strictEqual(editCount, 0, `Expected no edits for well-formatted code, got ${editCount}`);
  });
});
