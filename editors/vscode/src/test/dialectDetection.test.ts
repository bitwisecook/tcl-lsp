import * as assert from "assert";
import * as vscode from "vscode";
import { activate, getDocUri } from "./helper";

async function completionLabels(uri: vscode.Uri, position: vscode.Position): Promise<string[]> {
  const result = (await vscode.commands.executeCommand(
    "vscode.executeCompletionItemProvider",
    uri,
    position,
  )) as vscode.CompletionList;

  return result.items.map((item) =>
    typeof item.label === "string" ? item.label : item.label.label,
  );
}

suite("Dialect Detection", () => {
  test("defaults .tcl files to Tcl 8.6", async () => {
    const uri = getDocUri("dialect-default.tcl");
    await activate(uri);

    const labels = await completionLabels(uri, new vscode.Position(1, 2));
    assert.ok(labels.includes("try"), 'Expected "try" completion for default tcl8.6 dialect');
  });

  test("uses shebang tclshX.X hint for Tcl version", async () => {
    const uri = getDocUri("dialect-shebang85.tcl");
    await activate(uri);

    const labels = await completionLabels(uri, new vscode.Position(1, 2));
    assert.ok(!labels.includes("try"), 'Did not expect "try" completion for shebang tclsh8.5');
  });

  test("maps .irul extension to f5-irules", async () => {
    const uri = getDocUri("dialect.irul");
    await activate(uri);

    // Query completion within "HTTP::header" on line 2.
    const labels = await completionLabels(uri, new vscode.Position(2, 11));
    assert.ok(
      labels.includes("HTTP::header"),
      'Expected "HTTP::header" completion for .irule file',
    );
  });

  test("maps .iapp extension to f5-iapps", async () => {
    const uri = getDocUri("dialect.iapp");
    await activate(uri);

    // Request completion at the command-start position inside "[...]"
    // so results are not filtered by an in-progress partial token.
    const labels = await completionLabels(uri, new vscode.Position(1, 6));
    assert.ok(
      labels.includes("iapp::template"),
      'Expected "iapp::template" completion for .iapp file',
    );
  });
});
