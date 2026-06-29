import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate, waitForDiagnostics } from "./helper";

// End-to-end regression coverage for the "preview version:" tickets that are
// analyser / delivery bugs surfaced in VS Code:
//
//   #720  `after 200 {...}` must not be flagged as an unknown subcommand (W001)
//   #721  a single diagnostic must not be displayed twice (E003 here)
//   #723  Tk commands behind an unknown `package require` must not draw W120
//   #725  `$::var` (a qualified global read) must not draw "read before set" (W210)
suite("Preview-version regression tickets", () => {
  const docUri = getDocUri("previewTickets.tcl");

  function codeOf(d: vscode.Diagnostic): string | number | undefined {
    return typeof d.code === "object" ? d.code.value : d.code;
  }

  test("the false-positive diagnostics never fire and E003 appears exactly once", async () => {
    await activate(docUri);
    // The fixture's `set var 10 10` guarantees at least one diagnostic (E003),
    // which also proves the analyser actually ran over the document.
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 1 });
    const codes = diagnostics.map(codeOf);

    // #720 — `after 200 {...}` is a delay, not an unknown subcommand.
    assert.ok(!codes.includes("W001"), `#720: unexpected W001 in [${codes}]`);
    // #725 — `$::myVar` is an explicit global read, never read-before-set.
    assert.ok(!codes.includes("W210"), `#725: unexpected W210 in [${codes}]`);
    // #723 — an unknown package may load Tk; the Tk commands must not draw W120.
    assert.ok(!codes.includes("W120"), `#723: unexpected W120 in [${codes}]`);

    // #721 — the genuine E003 must be present, and exactly once (no duplicate
    // from the server pushing *and* the client pulling the same diagnostic).
    const e003 = diagnostics.filter((d) => codeOf(d) === "E003");
    assert.strictEqual(
      e003.length,
      1,
      `#721: expected exactly one E003, got ${e003.length}: ${e003
        .map((d) => `${d.range.start.line}:${d.message}`)
        .join("; ")}`,
    );
  });
});
