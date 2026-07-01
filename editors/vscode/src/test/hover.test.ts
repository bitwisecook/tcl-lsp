import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate, pollUntil } from "./helper";

suite("Hover", () => {
  const docUri = getDocUri("procs.tcl");

  test("shows hover for proc call", async () => {
    await activate(docUri);

    // "fib" call on line 16: puts "fib(10) = [fib 10]"
    // The inner "fib" is inside [fib 10], somewhere around column 22
    // Let's use a position on the proc name "fib" at line 0, col 5
    // proc fib {n} {  -- "fib" starts at col 5
    const position = new vscode.Position(1, 6);

    const hovers = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeHoverProvider", docUri, position),
      (r) => Array.isArray(r) && r.length > 0,
      { timeout: 10_000, label: "hover for proc call" },
    )) as vscode.Hover[];

    assert.ok(hovers, "Hover result should not be null");
    assert.ok(hovers.length > 0, "Should have at least one hover");

    const hoverText = hovers
      .flatMap((h) => h.contents)
      .map((c) => {
        if (typeof c === "string") return c;
        if (c instanceof vscode.MarkdownString) return c.value;
        // MarkdownString with language
        return (c as { value: string }).value;
      })
      .join("\n");

    assert.ok(hoverText.length > 0, "Hover content should not be empty");
    // The hover should mention "fib" or show the signature
    assert.ok(hoverText.includes("fib"), `Hover should mention "fib", got: ${hoverText}`);
  });
});
