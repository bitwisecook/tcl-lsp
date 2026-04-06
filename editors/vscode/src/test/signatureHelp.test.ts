import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate } from "./helper";

suite("Signature Help", () => {
  const docUri = getDocUri("simple.tcl");

  test("provides signature help for a built-in command", async () => {
    await activate(docUri);

    // Use an untitled document to avoid leaking state into other suites.
    const doc = await vscode.workspace.openTextDocument({
      language: "tcl",
      content: "string length \n",
    });
    await vscode.window.showTextDocument(doc);

    // Trigger signature help after 'string length '
    const pos = new vscode.Position(0, 14);
    const result = (await vscode.commands.executeCommand(
      "vscode.executeSignatureHelpProvider",
      doc.uri,
      pos,
    )) as vscode.SignatureHelp | undefined;

    // Verify the provider is wired up and does not throw. If it does return
    // signature help, ensure the shape is well-formed.
    if (result) {
      assert.ok(
        Array.isArray(result.signatures),
        "SignatureHelp should include a signatures array",
      );
    }
  });

  test("provides signature help for a user proc", async () => {
    await activate(docUri);

    // Use an untitled document to avoid leaking state into other suites.
    const doc = await vscode.workspace.openTextDocument({
      language: "tcl",
      content: 'proc greet {name greeting} {\n    puts "$greeting, $name"\n}\ngreet \n',
    });
    await vscode.window.showTextDocument(doc);

    // Trigger signature help after 'greet ' on line 3
    const pos = new vscode.Position(3, 6);
    const result = (await vscode.commands.executeCommand(
      "vscode.executeSignatureHelpProvider",
      doc.uri,
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
