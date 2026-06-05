import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate } from "./helper";

suite("Document Symbols", () => {
  const docUri = getDocUri("procs.tcl");

  test("returns document symbols for proc definitions", async () => {
    await activate(docUri);

    const symbols = (await vscode.commands.executeCommand(
      "vscode.executeDocumentSymbolProvider",
      docUri,
    )) as vscode.DocumentSymbol[] | vscode.SymbolInformation[];

    assert.ok(symbols, "Should return symbols");
    assert.ok(symbols.length > 0, "Should find at least one symbol");

    const names = symbols.map((s) => s.name);
    assert.ok(
      names.some((n) => n.includes("fib")),
      `Should find 'fib' proc, got: ${names}`,
    );
    assert.ok(
      names.some((n) => n.includes("factorial")),
      `Should find 'factorial' proc, got: ${names}`,
    );
  });

  test("symbols have proper kinds", async () => {
    await activate(docUri);

    const symbols = (await vscode.commands.executeCommand(
      "vscode.executeDocumentSymbolProvider",
      docUri,
    )) as vscode.DocumentSymbol[] | vscode.SymbolInformation[];

    assert.ok(symbols && symbols.length > 0, "Should return symbols");

    // Proc definitions should be functions
    for (const sym of symbols) {
      if (sym.name.includes("fib") || sym.name.includes("factorial")) {
        assert.strictEqual(
          sym.kind,
          vscode.SymbolKind.Function,
          `Proc '${sym.name}' should have Function kind`,
        );
      }
    }
  });

  test("symbols have valid ranges", async () => {
    await activate(docUri);

    const symbols = (await vscode.commands.executeCommand(
      "vscode.executeDocumentSymbolProvider",
      docUri,
    )) as vscode.DocumentSymbol[];

    assert.ok(symbols && symbols.length > 0, "Should return symbols");

    for (const sym of symbols) {
      if ("range" in sym) {
        const range = sym.range;
        assert.ok(
          range.start.line >= 0,
          `Symbol '${sym.name}' should have a non-negative start line`,
        );
        assert.ok(
          range.end.line >= range.start.line,
          `Symbol '${sym.name}' end line should be >= start line`,
        );
      }
    }
  });
});
