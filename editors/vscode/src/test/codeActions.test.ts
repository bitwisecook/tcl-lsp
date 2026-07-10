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
import { getDocUri, activate, waitForDiagnostics, pollUntil } from "./helper";

suite("Code Actions", () => {
  const docUri = getDocUri("diagnostics.tcl");
  const irulesDocUri = getDocUri("diagnostics-irules.irul");

  test("provides quick fix for W100 (unbraced expr)", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 1 });

    // Find the W100 diagnostic
    const w100 = diagnostics.find((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "W100";
    });
    assert.ok(w100, "W100 diagnostic should be present");

    // Request code actions at the W100 range
    const actions = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeCodeActionProvider", docUri, w100.range),
      (r) =>
        Array.isArray(r) &&
        (r as vscode.CodeAction[]).some(
          (a) => a.kind && a.kind.value === vscode.CodeActionKind.QuickFix.value,
        ),
      { timeout: 10_000, label: "W100 quick fix" },
    )) as vscode.CodeAction[];

    assert.ok(actions, "Code actions should not be null");
    assert.ok(actions.length > 0, "Should have at least one code action");

    // Find the quick fix action
    const quickFix = actions.find(
      (a) => a.kind && a.kind.value === vscode.CodeActionKind.QuickFix.value,
    );
    assert.ok(quickFix, "Should have a QuickFix code action");
    assert.ok(quickFix.title.length > 0, "Quick fix should have a title");
  });

  test("provides quick fix for W304 (missing option terminator)", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 1 });

    const w304 = diagnostics.find((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "W304";
    });
    assert.ok(w304, "W304 diagnostic should be present");

    const actions = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeCodeActionProvider", docUri, w304.range),
      (r) =>
        Array.isArray(r) &&
        (r as vscode.CodeAction[]).some(
          (a) => typeof a.title === "string" && a.title.toLowerCase().includes("option terminator"),
        ),
      { timeout: 10_000, label: "W304 option terminator quick fix" },
    )) as vscode.CodeAction[];

    const quickFix = actions.find(
      (a) => typeof a.title === "string" && a.title.toLowerCase().includes("option terminator"),
    );
    assert.ok(quickFix, "Should provide an option terminator quick fix");
  });

  test("provides quick fix for W302 (catch result capture)", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 1 });

    const w302 = diagnostics.find((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "W302";
    });
    assert.ok(w302, "W302 diagnostic should be present");

    const actions = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeCodeActionProvider", docUri, w302.range),
      (r) =>
        Array.isArray(r) &&
        (r as vscode.CodeAction[]).some(
          (a) => typeof a.title === "string" && a.title.includes("catch result variable"),
        ) &&
        (r as vscode.CodeAction[]).some(
          (a) => typeof a.title === "string" && a.title.includes("result + options"),
        ),
      { timeout: 10_000, label: "W302 catch result quick fixes" },
    )) as vscode.CodeAction[];

    const resultFix = actions.find(
      (a) => typeof a.title === "string" && a.title.includes("catch result variable"),
    );
    const resultOptsFix = actions.find(
      (a) => typeof a.title === "string" && a.title.includes("result + options"),
    );
    assert.ok(resultFix, "Should provide a result capture quick fix");
    assert.ok(resultOptsFix, "Should provide a result+options capture quick fix");
  });

  test("provides merge-into-body quick fix for E004 extra words", async () => {
    const e004Uri = getDocUri("diagnostics-e004.tcl");
    await activate(e004Uri);
    const diagnostics = await waitForDiagnostics(e004Uri, { minCount: 3 });

    const extraWords = diagnostics.find(
      (d) => d.message === 'Extra words after "else" clause in "if" command',
    );
    assert.ok(extraWords, "extra-words E004 diagnostic should be present");

    const actions = (await pollUntil(
      () =>
        vscode.commands.executeCommand(
          "vscode.executeCodeActionProvider",
          e004Uri,
          extraWords.range,
        ),
      (r) =>
        Array.isArray(r) &&
        (r as vscode.CodeAction[]).some(
          (a) => typeof a.title === "string" && a.title.toLowerCase().includes("merge"),
        ),
      { timeout: 10_000, label: "E004 extra-words merge quick fix" },
    )) as vscode.CodeAction[];

    const mergeFix = actions.find(
      (a) => typeof a.title === "string" && a.title.toLowerCase().includes("merge"),
    );
    assert.ok(mergeFix, "Should provide a merge-into-body quick fix");
  });

  test("provides remove-clause quick fix for a dangling E004 elseif", async () => {
    const e004Uri = getDocUri("diagnostics-e004.tcl");
    await activate(e004Uri);
    const diagnostics = await waitForDiagnostics(e004Uri, { minCount: 3 });

    const danglingElseif = diagnostics.find((d) => d.message === 'No script following "2" argument');
    assert.ok(danglingElseif, "dangling-elseif E004 diagnostic should be present");

    const actions = (await pollUntil(
      () =>
        vscode.commands.executeCommand(
          "vscode.executeCodeActionProvider",
          e004Uri,
          danglingElseif.range,
        ),
      (r) =>
        Array.isArray(r) &&
        (r as vscode.CodeAction[]).some(
          (a) => typeof a.title === "string" && a.title.toLowerCase().includes("remove"),
        ),
      { timeout: 10_000, label: "E004 dangling-clause remove quick fix" },
    )) as vscode.CodeAction[];

    const removeFix = actions.find(
      (a) => typeof a.title === "string" && a.title.toLowerCase().includes("remove"),
    );
    assert.ok(removeFix, "Should provide a remove-incomplete-clause quick fix");
  });

  test("offers no quick fix for an E004 whose first clause never completed", async () => {
    const e004Uri = getDocUri("diagnostics-e004.tcl");
    await activate(e004Uri);
    const diagnostics = await waitForDiagnostics(e004Uri, { minCount: 3 });

    const bareCondition = diagnostics.find((d) => d.message === 'No script following "1" argument');
    assert.ok(bareCondition, "bare-condition E004 diagnostic should be present");

    // No well-formed prefix exists to fall back to, so no quick fix should
    // ever be offered here — a single read is enough (there is nothing to
    // wait for; the assertion is that it never appears).
    const actions = (await vscode.commands.executeCommand(
      "vscode.executeCodeActionProvider",
      e004Uri,
      bareCondition.range,
    )) as vscode.CodeAction[] | undefined;
    const quickFixes = (actions ?? []).filter(
      (a) => a.kind && a.kind.value === vscode.CodeActionKind.QuickFix.value,
    );
    assert.strictEqual(
      quickFixes.length,
      0,
      `expected no quick fix, got: ${quickFixes.map((a) => a.title).join("; ")}`,
    );
  });

  test("provides guided collect bootstrap fix for IRULE1005", async () => {
    await activate(irulesDocUri);
    // IRULE1005 is a deep diagnostic — it fires after the initial basic
    // pass has produced several IRULE1001/IRULE1004/W211 diagnostics.
    // Waiting on ``minCount`` alone returns too early, so poll until
    // IRULE1005 specifically appears.
    const diagnostics = await waitForDiagnostics(irulesDocUri, {
      predicate: (diags) =>
        diags.some((d) => {
          const code = typeof d.code === "object" ? d.code.value : d.code;
          return code === "IRULE1005";
        }),
    });

    const irule1005 = diagnostics.find((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "IRULE1005";
    });
    assert.ok(irule1005, "IRULE1005 diagnostic should be present");

    const actions = (await pollUntil(
      () =>
        vscode.commands.executeCommand(
          "vscode.executeCodeActionProvider",
          irulesDocUri,
          irule1005.range,
        ),
      (r) =>
        Array.isArray(r) &&
        (r as vscode.CodeAction[]).some(
          (a) =>
            typeof a.title === "string" &&
            a.title.includes("collect") &&
            a.title.includes("CLIENT_ACCEPTED"),
        ),
      { timeout: 10_000, label: "IRULE1005 collect bootstrap quick fix" },
    )) as vscode.CodeAction[];

    const collectFix = actions.find(
      (a) =>
        typeof a.title === "string" &&
        a.title.includes("collect") &&
        a.title.includes("CLIENT_ACCEPTED"),
    );
    assert.ok(collectFix, "Should provide a collect bootstrap quick fix");
  });
});
