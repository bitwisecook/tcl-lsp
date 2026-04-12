import * as assert from "assert";
import * as vscode from "vscode";
import { activate, getDocUri, setTestContent } from "./helper";

suite("Selection Transforms", () => {
  const docUri = getDocUri("formatting.tcl");

  test("escapes and unescapes a selected Tcl fragment", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;

    const plain = 'Hello $name [clock seconds] "quoted" \\ tab\tend';
    await setTestContent(editor, plain);
    editor.selections = [
      new vscode.Selection(new vscode.Position(0, 0), new vscode.Position(0, plain.length)),
    ];

    await vscode.commands.executeCommand("tclLsp.escapeSelection");

    const escaped = 'Hello \\$name \\[clock seconds\\] \\"quoted\\" \\\\ tab\\tend';
    assert.strictEqual(editor.document.getText(), escaped);

    editor.selections = [
      new vscode.Selection(new vscode.Position(0, 0), new vscode.Position(0, escaped.length)),
    ];

    await vscode.commands.executeCommand("tclLsp.unescapeSelection");
    assert.strictEqual(editor.document.getText(), plain);
  });

  test("escapes all non-empty selections", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;

    const text = "set a $x\nset b $y\n";
    await setTestContent(editor, text);
    editor.selections = [
      new vscode.Selection(new vscode.Position(0, 6), new vscode.Position(0, 8)),
      new vscode.Selection(new vscode.Position(1, 6), new vscode.Position(1, 8)),
    ];

    await vscode.commands.executeCommand("tclLsp.escapeSelection");

    assert.strictEqual(editor.document.getText(), "set a \\$x\nset b \\$y\n");
  });

  test("base64 encodes and decodes a selected fragment", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;

    const plain = '[b64decode "SGVsbG8gV29ybGQ="]';
    await setTestContent(editor, plain);

    // Select just the base64 payload
    const b64 = "SGVsbG8gV29ybGQ=";
    const start = plain.indexOf(b64);
    editor.selections = [
      new vscode.Selection(
        new vscode.Position(0, start),
        new vscode.Position(0, start + b64.length),
      ),
    ];

    await vscode.commands.executeCommand("tclLsp.base64DecodeSelection");
    assert.strictEqual(editor.document.getText(), '[b64decode "Hello World"]');

    // Now select "Hello World" and encode it back
    const decoded = "Hello World";
    const decStart = editor.document.getText().indexOf(decoded);
    editor.selections = [
      new vscode.Selection(
        new vscode.Position(0, decStart),
        new vscode.Position(0, decStart + decoded.length),
      ),
    ];

    await vscode.commands.executeCommand("tclLsp.base64EncodeSelection");
    assert.strictEqual(editor.document.getText(), `[b64decode "${b64}"]`);
  });
});
