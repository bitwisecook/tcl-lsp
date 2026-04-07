import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate, sleep, waitForDiagnostics } from "./helper";

suite("Diagnostics", () => {
  const docUri = getDocUri("diagnostics.tcl");

  test("produces expected diagnostic codes", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 3 });

    assert.ok(
      diagnostics.length >= 3,
      `Expected at least 3 diagnostics, got ${diagnostics.length}`,
    );

    const codes = diagnostics.map((d) => (typeof d.code === "object" ? d.code.value : d.code));

    assert.ok(codes.includes("W100"), `Expected W100 (unbraced expr) in [${codes}]`);
    assert.ok(codes.includes("W101"), `Expected W101 (eval injection) in [${codes}]`);
    assert.ok(codes.includes("W302"), `Expected W302 (catch without result) in [${codes}]`);
  });

  test("W100 diagnostic has error severity when expr contains substitutions", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 1 });

    const w100 = diagnostics.find((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "W100";
    });

    assert.ok(w100, "W100 diagnostic not found");
    assert.strictEqual(
      w100.severity,
      vscode.DiagnosticSeverity.Error,
      "W100 with substitutions should be an error",
    );
  });

  test("W302 diagnostic has hint severity", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 3 });

    const w302 = diagnostics.find((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "W302";
    });

    assert.ok(w302, "W302 diagnostic not found");
    assert.strictEqual(w302.severity, vscode.DiagnosticSeverity.Hint, "W302 should be a hint");
  });

  test("W125 fires for orphaned else/elseif on separate line", async () => {
    const orphanedUri = getDocUri("diagnostics-orphaned.tcl");
    await activate(orphanedUri);
    const diagnostics = await waitForDiagnostics(orphanedUri, { minCount: 2 });

    const w125 = diagnostics.filter((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "W125";
    });

    assert.ok(w125.length >= 2, `Expected at least 2 W125 diagnostics, got ${w125.length}`);

    // Verify the messages reference the right keywords
    const messages = w125.map((d) => d.message);
    assert.ok(
      messages.some((m) => m.includes('"else"')),
      `Expected a W125 for "else", got: ${messages.join("; ")}`,
    );
    assert.ok(
      messages.some((m) => m.includes('"elseif"')),
      `Expected a W125 for "elseif", got: ${messages.join("; ")}`,
    );

    // All W125 should be warnings
    for (const d of w125) {
      assert.strictEqual(d.severity, vscode.DiagnosticSeverity.Warning, "W125 should be a warning");
    }
  });

  test("W125 does not fire for correctly placed else", async () => {
    const orphanedUri = getDocUri("diagnostics-orphaned.tcl");
    await activate(orphanedUri);
    const diagnostics = await waitForDiagnostics(orphanedUri, { minCount: 2 });

    const w125 = diagnostics.filter((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "W125";
    });

    // The fixture has exactly 2 orphaned keywords (else + elseif),
    // the correct } else { should not trigger W125
    assert.strictEqual(
      w125.length,
      2,
      `Expected exactly 2 W125 (orphaned else + elseif), got ${w125.length}: ${w125.map((d) => d.message).join("; ")}`,
    );
  });

  test("clean file produces no diagnostics", async () => {
    const cleanUri = getDocUri("simple.tcl");

    // Disable optimiser so info-level suggestions (O1xx) don't count.
    const config = vscode.workspace.getConfiguration("tclLsp.optimiser");
    await config.update("enabled", false, vscode.ConfigurationTarget.Global);

    // Allow the pull-model config round-trip to complete so the
    // server applies optimiser.enabled=false before analysing.
    await sleep(500);

    try {
      await activate(cleanUri);

      // Wait briefly for any diagnostics to appear (proving none arrive)
      const diagnostics = await waitForDiagnostics(cleanUri, {
        timeout: 2000,
        minCount: 1,
      });

      assert.strictEqual(
        diagnostics.length,
        0,
        `Expected no diagnostics for simple.tcl, got ${diagnostics.length}: ${diagnostics.map((d) => d.code).join(", ")}`,
      );
    } finally {
      await config.update("enabled", undefined, vscode.ConfigurationTarget.Global);
    }
  });
});
