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
import { getDocUri, activate, waitForDiagnostics } from "./helper";

// Issue #1333 — the *user-visible* half.
//
// The server-side wire format is asserted in `issue1333_diagnostic_tags.rs`.
// What this suite establishes is that the tag survives the round trip into
// VS Code's own `Diagnostic` model, because that model is what drives the
// rendering: `DiagnosticTag.Unnecessary` is the only thing that makes VS Code
// fade an identifier, and `DiagnosticTag.Deprecated` the only thing that
// strikes one through. A `hint` on a one-character span renders as three
// near-invisible dots, which is precisely the report this issue came from —
// so "the diagnostic fires" is not the property under test here.

function codeOf(d: vscode.Diagnostic): string {
  return typeof d.code === "object" && d.code !== null ? String(d.code.value) : String(d.code);
}

function withCode(diags: vscode.Diagnostic[], code: string): vscode.Diagnostic[] {
  return diags.filter((d) => codeOf(d) === code);
}

function tagsOf(d: vscode.Diagnostic): vscode.DiagnosticTag[] {
  return d.tags ?? [];
}

suite("Diagnostic tags render unused code faded and deprecated code struck through", () => {
  const tclUri = getDocUri("diagnosticTags.tcl");
  const iruleUri = getDocUri("deprecatedTag.irule");

  async function tclDiagnostics(): Promise<vscode.Diagnostic[]> {
    await activate(tclUri);
    return waitForDiagnostics(tclUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W214"),
    });
  }

  test("an unused proc parameter is marked Unnecessary, so VS Code fades it", async () => {
    const diags = await tclDiagnostics();
    const w214 = withCode(diags, "W214");
    assert.strictEqual(w214.length, 1, `expected one W214; got ${JSON.stringify(w214)}`);
    assert.deepStrictEqual(
      tagsOf(w214[0]),
      [vscode.DiagnosticTag.Unnecessary],
      "W214 must carry DiagnosticTag.Unnecessary — without it the hint is invisible",
    );
    // ...and the span is still just the identifier, which is what gets faded.
    assert.strictEqual(w214[0].range.start.line, 4);
    assert.strictEqual(w214[0].range.start.character, 16);
    assert.strictEqual(w214[0].range.end.character, 17);
  });

  test("the severity stays a Hint — the tag is the fix, not a louder squiggle", async () => {
    const diags = await tclDiagnostics();
    const w214 = withCode(diags, "W214");
    assert.strictEqual(
      w214[0].severity,
      vscode.DiagnosticSeverity.Hint,
      "an unused parameter is often deliberate (interface conformance, callback " +
        "signatures); promoting it to a warning would misreport it as a defect",
    );
  });

  test("an unused local variable is marked Unnecessary too", async () => {
    const diags = await tclDiagnostics();
    const w211 = withCode(diags, "W211");
    assert.ok(w211.length >= 1, `expected a W211; got ${JSON.stringify(diags.map(codeOf))}`);
    assert.deepStrictEqual(tagsOf(w211[0]), [vscode.DiagnosticTag.Unnecessary]);
  });

  test("a read-before-set defect is never faded", async () => {
    // The one way "tag the unused-variable family" could go wrong: fading a
    // real bug hides it.
    const diags = await tclDiagnostics();
    const w210 = withCode(diags, "W210");
    assert.ok(w210.length >= 1, `expected a W210; got ${JSON.stringify(diags.map(codeOf))}`);
    assert.deepStrictEqual(tagsOf(w210[0]), [], "W210 describes a defect and must stay solid");
  });

  test("ordinary diagnostics carry no tags at all", async () => {
    const diags = await tclDiagnostics();
    const untagged = diags.filter(
      (d) => !["W211", "W214", "W220", "O126"].includes(codeOf(d)) && tagsOf(d).length > 0,
    );
    assert.deepStrictEqual(
      untagged.map(codeOf),
      [],
      "only the declared codes may be tagged — see DiagCode::lsp_tag",
    );
  });

  test("a deprecated command is marked Deprecated, so VS Code strikes it through", async () => {
    await activate(iruleUri);
    const diags = await waitForDiagnostics(iruleUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "IRULE2002"),
    });
    const dep = withCode(diags, "IRULE2002");
    assert.strictEqual(dep.length, 1, `expected one IRULE2002; got ${JSON.stringify(dep)}`);
    assert.deepStrictEqual(
      tagsOf(dep[0]),
      [vscode.DiagnosticTag.Deprecated],
      "IRULE2002 must carry DiagnosticTag.Deprecated",
    );
  });
});
