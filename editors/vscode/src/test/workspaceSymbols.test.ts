import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate } from "./helper";

suite("Workspace Symbols", () => {
  suiteSetup(async () => {
    // Ensure the server has indexed at least one file
    await activate(getDocUri("procs.tcl"));
  });

  test("returns workspace symbols matching a query", async () => {
    const symbols = (await vscode.commands.executeCommand(
      "vscode.executeWorkspaceSymbolProvider",
      "fib",
    )) as vscode.SymbolInformation[] | undefined;

    assert.ok(symbols, "Workspace symbol provider should return results");
    assert.ok(symbols.length > 0, "Should find at least one symbol matching 'fib'");

    const fib = symbols.find((s) => s.name.includes("fib"));
    assert.ok(fib, `Should find a symbol containing 'fib', got: ${symbols.map((s) => s.name)}`);
  });

  test("returns symbols for factorial query", async () => {
    const symbols = (await vscode.commands.executeCommand(
      "vscode.executeWorkspaceSymbolProvider",
      "factorial",
    )) as vscode.SymbolInformation[] | undefined;

    assert.ok(symbols, "Should return results for 'factorial' query");
    assert.ok(symbols.length > 0, "Should find 'factorial' proc");
  });

  test("empty query returns some symbols", async () => {
    const symbols = (await vscode.commands.executeCommand(
      "vscode.executeWorkspaceSymbolProvider",
      "",
    )) as vscode.SymbolInformation[] | undefined;

    // An empty query should return some or all symbols
    assert.ok(symbols !== undefined, "Empty query should not fail");
  });

  test("workspace symbols have valid locations", async () => {
    const symbols = (await vscode.commands.executeCommand(
      "vscode.executeWorkspaceSymbolProvider",
      "fib",
    )) as vscode.SymbolInformation[];

    if (symbols && symbols.length > 0) {
      for (const sym of symbols) {
        assert.ok(sym.location, `Symbol '${sym.name}' should have a location`);
        assert.ok(sym.location.uri, `Symbol '${sym.name}' should have a location URI`);
        assert.ok(
          sym.location.range.start.line >= 0,
          `Symbol '${sym.name}' should have a valid line number`,
        );
      }
    }
  });
});
