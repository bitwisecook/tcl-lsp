import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate } from "./helper";

suite("Go to Definition", () => {
  const docUri = getDocUri("procs.tcl");

  test("navigates from proc call to proc definition", async () => {
    await activate(docUri);

    // Line 16: puts "fib(10) = [fib 10]"
    // We need a position on a "fib" reference. Let's use the fib call
    // inside the proc body at line 5:
    //   return [expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}]
    // "fib" appears at roughly col 20
    // Actually, simpler: line 16 has "fib" starting around col 19
    // puts "fib(10) = [fib 10]"  -- "fib" after [ is at col 17
    const position = new vscode.Position(16, 17);

    const locations = (await vscode.commands.executeCommand(
      "vscode.executeDefinitionProvider",
      docUri,
      position,
    )) as vscode.Location[];

    assert.ok(locations, "Definition result should not be null");
    assert.ok(locations.length > 0, "Should find at least one definition");

    // The definition of "fib" is on line 1: proc fib {n} {
    const defLocation = locations[0];
    assert.strictEqual(
      defLocation.uri.fsPath,
      docUri.fsPath,
      "Definition should be in the same file",
    );
    assert.strictEqual(
      defLocation.range.start.line,
      1,
      `Definition should be on line 1 (proc fib), got line ${defLocation.range.start.line}`,
    );
  });
});
