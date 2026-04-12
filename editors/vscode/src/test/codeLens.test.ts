import * as assert from "assert";
import * as vscode from "vscode";
import { activate, getDocUri, sleep } from "./helper";

suite("Code Lens", () => {
  const docUri = getDocUri("procs.tcl");

  test("returns resolved lenses for each proc", async () => {
    await activate(docUri);
    // Code lenses are populated asynchronously — give the provider a beat.
    await sleep(500);
    // executeCodeLensProvider's second argument is the number of lenses
    // to resolve; without it VS Code returns unresolved lenses (command=null).
    const lenses = (await vscode.commands.executeCommand(
      "vscode.executeCodeLensProvider",
      docUri,
      100,
    )) as vscode.CodeLens[] | undefined;

    assert.ok(lenses, "codeLens result should not be null");
    assert.ok(
      lenses.length >= 2,
      `Expected at least 2 code lenses (fib + factorial), got ${lenses.length}`,
    );
    const resolved = lenses.filter((l) => l.command !== undefined);
    assert.ok(resolved.length >= 2, `Expected at least 2 resolved lenses, got ${resolved.length}`);
    for (const lens of resolved) {
      assert.ok(
        lens.command &&
          typeof lens.command.title === "string" &&
          /\d+\s+reference/i.test(lens.command.title),
        `Expected reference-count title, got "${lens.command?.title}"`,
      );
    }
  });
});
