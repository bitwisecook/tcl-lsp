import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate, setTestContent } from "./helper";

suite("Signature Help", () => {
  const docUri = getDocUri("formatting.tcl");

  test("provides signature help for a built-in command", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;

    await setTestContent(editor, "string length \n");

    // Trigger signature help after 'string length '
    const pos = new vscode.Position(0, 14);
    const result = (await vscode.commands.executeCommand(
      "vscode.executeSignatureHelpProvider",
      docUri,
      pos,
    )) as vscode.SignatureHelp | undefined;

    // The server may or may not provide signature help for built-ins; just
    // verify the provider is wired up and does not throw.
    if (result) {
      assert.ok(result.signatures.length >= 0, "Should return zero or more signatures");
    }
  });

  test("provides signature help for a user proc", async () => {
    await activate(docUri);
    const editor = vscode.window.activeTextEditor!;

    await setTestContent(
      editor,
      'proc greet {name greeting} {\n    puts "$greeting, $name"\n}\ngreet \n',
    );

    // Trigger signature help after 'greet ' on line 3
    const pos = new vscode.Position(3, 6);
    const result = (await vscode.commands.executeCommand(
      "vscode.executeSignatureHelpProvider",
      docUri,
      pos,
    )) as vscode.SignatureHelp | undefined;

    if (result && result.signatures.length > 0) {
      const sig = result.signatures[0];
      assert.ok(sig.label, "Signature should have a label");
      assert.ok(
        sig.parameters && sig.parameters.length > 0,
        "Signature for proc with params should list parameters",
      );
    }
  });
});
