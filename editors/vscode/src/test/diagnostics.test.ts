// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate, waitForDiagnostics, waitForEffectiveConfig } from "./helper";

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

  test("W128 fires for a call to a command renamed away earlier in the file", async () => {
    const renameUri = getDocUri("diagnostics-rename.tcl");
    await activate(renameUri);
    const diagnostics = await waitForDiagnostics(renameUri, { minCount: 1 });

    const w128 = diagnostics.filter((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "W128";
    });

    assert.ok(w128.length >= 1, `Expected at least one W128 diagnostic, got ${w128.length}`);
    assert.ok(
      w128.every((d) => d.severity === vscode.DiagnosticSeverity.Warning),
      "W128 should be a warning",
    );
    assert.ok(
      w128.some((d) => d.message.includes("renamed or deleted")),
      `Expected W128 message to mention rename/delete, got: ${w128.map((d) => d.message).join("; ")}`,
    );
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

    // Wait on the server's resolved config (message passing) rather than a
    // fixed sleep, so the optimiser.enabled=false round-trip is observed to
    // have applied before analysing.
    await waitForEffectiveConfig(cleanUri, (cfg) => cfg.optimiser_enabled === false, {
      label: "optimiser.enabled = false",
    });

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

  test("no false dead-store / unused diagnostics where variables are read", async () => {
    const uri = getDocUri("precision-lifecycle.tcl");
    await activate(uri);
    // The fixture's last line (unbraced expr) yields W100, proving analysis ran.
    const diagnostics = await waitForDiagnostics(uri, { minCount: 1 });
    const codes = diagnostics.map((d) => (typeof d.code === "object" ? d.code.value : d.code));

    assert.ok(codes.includes("W100"), `expected analysis to run (W100) in [${codes}]`);
    for (const lifecycle of ["W210", "W211", "W214", "W220"]) {
      assert.ok(
        !codes.includes(lifecycle),
        `unexpected ${lifecycle} (variable is read) in [${codes}]`,
      );
    }
  });

  test("W100 fires inside a catch body (analyser recurses into catch)", async () => {
    const uri = getDocUri("catchBody.tcl");
    await activate(uri);
    const diagnostics = await waitForDiagnostics(uri, { minCount: 1 });
    const codes = diagnostics.map((d) => (typeof d.code === "object" ? d.code.value : d.code));

    // The unbraced `expr` lives inside `catch { ... }`; the analyser must walk
    // the catch body and report W100 there (catch-body-walk parity fix).
    assert.ok(codes.includes("W100"), `expected W100 inside the catch body, got [${codes}]`);
    const w100 = diagnostics.find((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "W100";
    });
    assert.ok(
      w100 && w100.range.start.line === 3,
      `W100 should anchor to the catch body line, got ${w100?.range.start.line}`,
    );
  });

  test("S110 fires for byte-array corruption (string op on a byte array)", async () => {
    // Plain-Tcl Case A: `binary format` -> `string toupper` mangles high bytes.
    // A `.tcl` fixture keeps the shared server in the default tcl8.6 dialect —
    // a `.irul` fixture here would switch it to f5-irules and leak that state
    // into the next suite (dialectDetection).  The iRules `*::payload`
    // round-trip is covered by the Python e2e suite.
    const uri = getDocUri("byteArrayCorruption.tcl");
    await activate(uri);
    // S110 is a *deep*-tier diagnostic (a second publish after the basic
    // tier), so wait for it specifically rather than for any first diagnostic.
    const codeOf = (d: vscode.Diagnostic) => (typeof d.code === "object" ? d.code.value : d.code);
    const diagnostics = await waitForDiagnostics(uri, {
      predicate: (diags) => diags.some((d) => codeOf(d) === "S110"),
    });

    const s110 = diagnostics.find((d) => codeOf(d) === "S110");

    assert.ok(s110, "expected S110 (byte-array corruption) for `string toupper` on a byte array");
    assert.strictEqual(
      s110.severity,
      vscode.DiagnosticSeverity.Warning,
      "S110 should be a warning",
    );
    assert.ok(
      /[Bb]yte-array corruption/.test(s110.message),
      `S110 message should describe the corruption, got: ${s110.message}`,
    );
  });

  // Issue #777: object commands bound by `CLASS create NAME` and iterated via
  // `foreach elem [list c1 l1 …]` are known commands, so dispatching `$elem`
  // must not fire W307. Analysis has settled once the unknown-class commands
  // (`C`/`L`) surface their own W123.
  test("W307 silent for create-named objects iterated via [list] (issue #777)", async () => {
    const uri = getDocUri("createNamedObjects.tcl");
    await activate(uri);
    const codeOf = (d: vscode.Diagnostic) => (typeof d.code === "object" ? d.code.value : d.code);
    const diagnostics = await waitForDiagnostics(uri, {
      predicate: (diags) => diags.some((d) => codeOf(d) === "W123"),
    });
    const codes = diagnostics.map(codeOf);
    assert.ok(
      !codes.includes("W307"),
      `dispatch over created object names must not fire W307, got [${codes}]`,
    );
  });
});
