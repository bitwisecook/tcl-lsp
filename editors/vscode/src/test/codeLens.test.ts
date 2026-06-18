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

  // Regression for issue #637 / PR #644: the reference-count title must match
  // the actual references, including a call written before its definition
  // (which resolves to null at analysis time), and a bare call must be
  // attributed only to the same-named proc in its own namespace.
  test("reference count matches resolution for forward and namespaced calls", async () => {
    const refsUri = getDocUri("codeLensRefs.tcl");
    await activate(refsUri);
    await sleep(500);
    const lenses = (await vscode.commands.executeCommand(
      "vscode.executeCodeLensProvider",
      refsUri,
      100,
    )) as vscode.CodeLens[] | undefined;
    assert.ok(lenses, "codeLens result should not be null");

    // Map each resolved lens to the line its proc name sits on.
    const titleByLine = new Map<number, string>();
    for (const lens of lenses) {
      if (lens.command && typeof lens.command.title === "string") {
        titleByLine.set(lens.range.start.line, lens.command.title);
      }
    }

    // Line 1: `proc greet637` — called once before its definition (forward
    // reference). The old count path reported "0 references" here.
    assert.strictEqual(
      titleByLine.get(1),
      "1 reference",
      `forward-referenced proc: got "${titleByLine.get(1)}"`,
    );
    // Line 5: `proc dup644` in nsa644 — the bare `dup644` call inside nsa644
    // resolves here.
    assert.strictEqual(
      titleByLine.get(5),
      "1 reference",
      `::nsa644::dup644: got "${titleByLine.get(5)}"`,
    );
    // Line 8: `proc dup644` in nsb644 — must NOT be credited the nsa644 call.
    assert.strictEqual(
      titleByLine.get(8),
      "0 references",
      `::nsb644::dup644 should have no phantom reference: got "${titleByLine.get(8)}"`,
    );
  });
});
