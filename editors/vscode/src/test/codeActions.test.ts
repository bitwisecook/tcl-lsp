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
import {
  getDocUri,
  activate,
  waitForDiagnostics,
  waitForCodeActions,
  waitForProviderResult,
} from "./helper";

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
    const actions = await waitForCodeActions(
      docUri,
      w100.range,
      (actions) =>
        actions.some((a) => a.kind && a.kind.value === vscode.CodeActionKind.QuickFix.value),
      { timeout: 10_000, label: "W100 quick fix" },
    );

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

    const actions = await waitForCodeActions(
      docUri,
      w304.range,
      (actions) =>
        actions.some(
          (a) => typeof a.title === "string" && a.title.toLowerCase().includes("option terminator"),
        ),
      { timeout: 10_000, label: "W304 option terminator quick fix" },
    );

    const quickFix = actions.find(
      (a) => typeof a.title === "string" && a.title.toLowerCase().includes("option terminator"),
    );
    assert.ok(quickFix, "Should provide an option terminator quick fix");
  });

  test("provides quick fix for T102 (tainted data in option position) in a plain .tcl file", async () => {
    // T102 is dialect-general (not iRules-specific), so its `--`-insertion
    // fix must reach the code-action response for a plain .tcl document
    // too — not only under the f5-irules dialect.
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 1 });

    const t102 = diagnostics.find((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "T102";
    });
    assert.ok(t102, "T102 diagnostic should be present");

    const actions = await waitForCodeActions(
      docUri,
      t102.range,
      (actions) => actions.some((a) => typeof a.title === "string" && a.title.includes("--")),
      { timeout: 10_000, label: "T102 insert '--' quick fix" },
    );

    const quickFix = actions.find((a) => typeof a.title === "string" && a.title.includes("--"));
    assert.ok(quickFix, "Should provide an insert '--' quick fix for T102");
  });

  test("provides quick fix for W302 (catch result capture)", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 1 });

    const w302 = diagnostics.find((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "W302";
    });
    assert.ok(w302, "W302 diagnostic should be present");

    const actions = await waitForCodeActions(
      docUri,
      w302.range,
      (actions) =>
        actions.some(
          (a) => typeof a.title === "string" && a.title.includes("catch result variable"),
        ) &&
        actions.some((a) => typeof a.title === "string" && a.title.includes("result + options")),
      { timeout: 10_000, label: "W302 catch result quick fixes" },
    );

    const resultFix = actions.find(
      (a) => typeof a.title === "string" && a.title.includes("catch result variable"),
    );
    const resultOptsFix = actions.find(
      (a) => typeof a.title === "string" && a.title.includes("result + options"),
    );
    assert.ok(resultFix, "Should provide a result capture quick fix");
    assert.ok(resultOptsFix, "Should provide a result+options capture quick fix");
  });

  test("provides quick fix for E100 (stray close bracket)", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 1 });

    const e100 = diagnostics.find((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "E100";
    });
    assert.ok(e100, "E100 diagnostic should be present");

    const actions = await waitForCodeActions(
      docUri,
      e100.range,
      (actions) =>
        actions.some(
          (a) =>
            typeof a.title === "string" && a.title.toLowerCase().includes("insert missing '['"),
        ),
      { timeout: 10_000, label: "E100 insert bracket quick fix" },
    );

    const quickFix = actions.find(
      (a) => typeof a.title === "string" && a.title.toLowerCase().includes("insert missing '['"),
    );
    assert.ok(quickFix, "Should provide an insert-missing-bracket quick fix");

    // The fixture's `set y string]` recognises `string` as a known
    // command — the fix must insert `[` right before it, not just
    // anywhere in the command.
    const edit = quickFix!.edit;
    assert.ok(edit, "quick fix should carry a workspace edit");
    const changes = edit!.get(docUri);
    assert.strictEqual(changes.length, 1);
    assert.strictEqual(changes[0].newText, "[");
  });

  test("provides quick fix for E102 (stray close brace)", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 1 });

    const e102 = diagnostics.find((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "E102";
    });
    assert.ok(e102, "E102 diagnostic should be present");

    const actions = await waitForCodeActions(
      docUri,
      e102.range,
      (actions) =>
        actions.some(
          (a) => typeof a.title === "string" && a.title.toLowerCase().includes("remove extra '}'"),
        ),
      { timeout: 10_000, label: "E102 remove brace quick fix" },
    );

    const quickFix = actions.find(
      (a) => typeof a.title === "string" && a.title.toLowerCase().includes("remove extra '}'"),
    );
    assert.ok(quickFix, "Should provide a remove-extra-brace quick fix");
  });

  test("provides merge-into-body quick fix for E004 extra words", async () => {
    const e004Uri = getDocUri("diagnostics-e004.tcl");
    await activate(e004Uri);
    const diagnostics = await waitForDiagnostics(e004Uri, { minCount: 3 });

    const extraWords = diagnostics.find(
      (d) => d.message === 'Extra words after "else" clause in "if" command',
    );
    assert.ok(extraWords, "extra-words E004 diagnostic should be present");

    const actions = await waitForCodeActions(
      e004Uri,
      extraWords.range,
      (actions) =>
        actions.some((a) => typeof a.title === "string" && a.title.toLowerCase().includes("merge")),
      { timeout: 10_000, label: "E004 extra-words merge quick fix" },
    );

    const mergeFix = actions.find(
      (a) => typeof a.title === "string" && a.title.toLowerCase().includes("merge"),
    );
    assert.ok(mergeFix, "Should provide a merge-into-body quick fix");
  });

  test("provides remove-clause quick fix for a dangling E004 elseif", async () => {
    const e004Uri = getDocUri("diagnostics-e004.tcl");
    await activate(e004Uri);
    const diagnostics = await waitForDiagnostics(e004Uri, { minCount: 3 });

    const danglingElseif = diagnostics.find(
      (d) => d.message === 'No script following "2" argument',
    );
    assert.ok(danglingElseif, "dangling-elseif E004 diagnostic should be present");

    const actions = await waitForCodeActions(
      e004Uri,
      danglingElseif.range,
      (actions) =>
        actions.some(
          (a) => typeof a.title === "string" && a.title.toLowerCase().includes("remove"),
        ),
      { timeout: 10_000, label: "E004 dangling-clause remove quick fix" },
    );

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

  test("provides quick fix for W004 (dialect-invalid option)", async () => {
    const w004Uri = getDocUri("diagnostics-w004.tcl");
    await activate(w004Uri);
    const diagnostics = await waitForDiagnostics(w004Uri, {
      predicate: (diags) =>
        diags.some((d) => {
          const code = typeof d.code === "object" ? d.code.value : d.code;
          return code === "W004" && d.message.includes("-stride");
        }),
    });

    const w004 = diagnostics.find((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "W004" && d.message.includes("-stride");
    });
    assert.ok(w004, "W004 diagnostic for -stride should be present");

    const actions = await waitForCodeActions(
      w004Uri,
      w004.range,
      (actions) => actions.some((a) => typeof a.title === "string" && a.title.includes("-stride")),
      { timeout: 10_000, label: "W004 remove-option quick fix" },
    );

    const removeFix = actions.find(
      (a) => typeof a.title === "string" && a.title.includes("-stride"),
    );
    assert.ok(removeFix, "Should provide a 'Remove -stride' quick fix");
    assert.ok(removeFix.edit, "the quick fix should carry a workspace edit");
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

    const actions = await waitForCodeActions(
      irulesDocUri,
      irule1005.range,
      (actions) =>
        actions.some(
          (a) =>
            typeof a.title === "string" &&
            a.title.includes("HTTP::collect") &&
            a.title.includes("HTTP_REQUEST"),
        ),
      { timeout: 10_000, label: "IRULE1005 collect bootstrap quick fix" },
    );

    const collectFix = actions.find(
      (a) =>
        typeof a.title === "string" &&
        a.title.includes("HTTP::collect") &&
        a.title.includes("HTTP_REQUEST"),
    );
    assert.ok(collectFix, "Should provide a collect bootstrap quick fix");
  });

  test("provides quick fix for T101 (tainted data into puts)", async () => {
    const t101Uri = getDocUri("taint-t101.tcl");
    await activate(t101Uri);
    const diagnostics = await waitForDiagnostics(t101Uri, {
      predicate: (diags) =>
        diags.some((d) => {
          const code = typeof d.code === "object" ? d.code.value : d.code;
          return code === "T101";
        }),
    });

    const t101 = diagnostics.find((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "T101";
    });
    assert.ok(t101, "T101 diagnostic should be present");
    // The diagnostic must highlight only the tainted `$x` argument, not the
    // whole `puts $x` statement.
    assert.strictEqual(t101.range.start.character, t101.range.end.character - 2);

    const actions = await waitForCodeActions(
      t101Uri,
      t101.range,
      (actions) =>
        actions.some((a) => typeof a.title === "string" && a.title.includes("strip CR/LF")),
      { timeout: 10_000, label: "T101 sanitise quick fix" },
    );

    const sanitiseFix = actions.find(
      (a) => typeof a.title === "string" && a.title.includes("strip CR/LF"),
    );
    assert.ok(sanitiseFix, "Should provide a strip-CR/LF sanitise quick fix");
  });

  test("provides a noqa suppress quick fix for S100 (shimmer)", async () => {
    const shimmerUri = getDocUri("shimmerPrecision.tcl");
    await activate(shimmerUri);
    const diagnostics = await waitForDiagnostics(shimmerUri, {
      predicate: (diags) =>
        diags.some((d) => {
          const code = typeof d.code === "object" ? d.code.value : d.code;
          return code === "S100";
        }),
    });

    // Line 7 (0-indexed): `    lindex $x 0` inside `proc shimmer_true_case`.
    const s100 = diagnostics.find((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "S100" && d.range.start.line === 7;
    });
    assert.ok(s100, "S100 diagnostic on line 7 should be present");

    const actions = await waitForCodeActions(
      shimmerUri,
      s100.range,
      (actions) =>
        actions.some(
          (a) => typeof a.title === "string" && a.title === "Suppress S100 with a noqa comment",
        ),
      { timeout: 10_000, label: "S100 noqa suppress quick fix" },
    );

    const suppressFix = actions.find(
      (a) => typeof a.title === "string" && a.title === "Suppress S100 with a noqa comment",
    );
    assert.ok(suppressFix, "Should provide an S100 noqa suppress quick fix");

    const edit = suppressFix.edit;
    assert.ok(edit, "Suppress fix should carry a workspace edit");
    const changes = edit.get(shimmerUri);
    assert.strictEqual(changes.length, 1, "expected exactly one text edit");
    // The inserted comment must match the call's 4-space indentation.
    assert.strictEqual(changes[0].newText, "    # noqa: S100\n");
  });

  // -- `package require` suggestions are evidence-backed (issue #1191) ------
  //
  // Applying one of these mutates package loading and runs the package's
  // initialisation code, so the lightbulb must offer it only over a command
  // head resolution could not satisfy — never over a comment, a string, or
  // an argument word that merely names the same identifier.

  const packageContextUri = getDocUri("packageSuggestionContext.tcl");

  /** Titles of the quick fixes offered at `line`:`character`. */
  async function quickFixTitlesAt(
    uri: vscode.Uri,
    line: number,
    character: number,
  ): Promise<string[]> {
    const position = new vscode.Position(line, character);
    const actions = (await vscode.commands.executeCommand(
      "vscode.executeCodeActionProvider",
      uri,
      new vscode.Range(position, position),
    )) as vscode.CodeAction[] | undefined;
    return (actions ?? []).map((a) => a.title);
  }

  test("offers 'package require' on an unresolved command head", async () => {
    await activate(packageContextUri);
    await waitForDiagnostics(packageContextUri, { minCount: 0 });
    const titles = await waitForProviderResult(
      packageContextUri,
      () => quickFixTitlesAt(packageContextUri, 11, 3),
      (t) => t.some((title) => title.startsWith("Add 'package require")),
      { timeout: 10_000, label: "the 'package require' suggestion on the call at 11:3" },
    );
    assert.ok(
      titles.includes("Add 'package require http'"),
      `expected the http suggestion on the call, got: ${JSON.stringify(titles)}`,
    );
  });

  test("does not offer 'package require' on comments, strings, or argument words", async () => {
    await activate(packageContextUri);
    await waitForDiagnostics(packageContextUri, { minCount: 0 });
    // The comment mention, the quoted datum, and the `dict set` value word.
    const dataPositions: Array<[number, number]> = [
      [8, 10],
      [9, 16],
      [10, 25],
    ];
    for (const [line, character] of dataPositions) {
      const titles = await quickFixTitlesAt(packageContextUri, line, character);
      assert.ok(
        !titles.some((title) => title.startsWith("Add 'package require")),
        `line ${line} is data, not a call site; got: ${JSON.stringify(titles)}`,
      );
    }
  });
});
