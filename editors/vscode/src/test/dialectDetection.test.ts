import * as assert from "assert";
import * as vscode from "vscode";
import { activate, getDocUri, sleep } from "./helper";

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

/**
 * Poll completions until a predicate is satisfied or timeout expires.
 * Dialect switches propagate asynchronously from the extension to the
 * server, so we need to retry until the server has processed the
 * configuration change notification.
 */
async function waitForCompletions(
  uri: vscode.Uri,
  position: vscode.Position,
  predicate: (labels: string[]) => boolean,
  timeout = 15_000,
): Promise<string[]> {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    const labels = await completionLabels(uri, position);
    if (predicate(labels)) {
      return labels;
    }
    await sleep(250);
  }
  // Return last result so the assertion can produce a useful message.
  return completionLabels(uri, position);
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

    // Dialect detection sends a config notification asynchronously;
    // poll until the server reflects the tclsh8.5 dialect (no "try").
    const labels = await waitForCompletions(
      uri,
      new vscode.Position(1, 2),
      (l) => !l.includes("try"),
    );
    assert.ok(!labels.includes("try"), 'Did not expect "try" completion for shebang tclsh8.5');
  });

  test("maps .irul extension to f5-irules", async () => {
    const uri = getDocUri("dialect.irul");
    await activate(uri);

    const labels = await waitForCompletions(uri, new vscode.Position(2, 11), (l) =>
      l.includes("HTTP::header"),
    );
    assert.ok(
      labels.includes("HTTP::header"),
      'Expected "HTTP::header" completion for .irule file',
    );
  });

  test("maps .iapp extension to f5-iapps", async () => {
    const uri = getDocUri("dialect.iapp");
    await activate(uri);

    const labels = await waitForCompletions(uri, new vscode.Position(1, 6), (l) =>
      l.includes("iapp::template"),
    );
    assert.ok(
      labels.includes("iapp::template"),
      'Expected "iapp::template" completion for .iapp file',
    );
  });

  test("maps .exp extension to expect", async () => {
    const uri = getDocUri("dialect.exp");
    await activate(uri);

    const labels = await waitForCompletions(uri, new vscode.Position(2, 0), (l) =>
      l.includes("spawn"),
    );
    assert.ok(labels.includes("spawn"), 'Expected "spawn" completion for .exp file');
  });

  test("uses # tcl-dialect: comment directive for Tcl version", async () => {
    const uri = getDocUri("dialect-directive84.tcl");
    await activate(uri);

    // Dialect detection sends a config notification asynchronously;
    // poll until the server reflects the tcl8.4 dialect (no "try").
    const labels = await waitForCompletions(
      uri,
      new vscode.Position(1, 2),
      (l) => !l.includes("try"),
    );
    assert.ok(!labels.includes("try"), 'Did not expect "try" completion for tcl-dialect tcl8.4');
  });
});
