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

// Normalise the `code` field (string | { value } ) to a plain string.
function codeOf(d: vscode.Diagnostic): string {
  return typeof d.code === "object" && d.code !== null ? String(d.code.value) : String(d.code);
}

function linesWithCode(diags: vscode.Diagnostic[], code: string): number[] {
  return diags.filter((d) => codeOf(d) === code).map((d) => d.range.start.line);
}

// S100 shimmer precision — coverage for the deep shimmer review: interp-alias
// resolution, TclOO/namespace-eval body coverage, tight per-argument spans,
// write-trace awareness, and `# noqa` suppression directives.
//
// `shimmerPrecision.tcl` (0-indexed diagnostic lines):
//   7  lindex $x 0             TRUE  — committed dict shimmered to a list intrep
//   13 lindex $y 0             FALSE — value already carries a list intrep
//   20 myindex $z 0            TRUE  — shimmer detected through an interp alias
//   27 incr a (TclOO method)   TRUE  — method bodies now get shimmer coverage
//   34 incr b (namespace eval) TRUE  — namespace eval bodies now get coverage
//   42 incr c (traced)         FALSE — a write-traced var is never confidently typed
//   49 lindex $d 0 (noqa S100) FALSE — suppressed by a matching preceding directive
//   56 lindex $e 0 (noqa W100) TRUE  — a mismatched directive must not suppress
//
// Every test below waits for *some* S100 to appear as the "analysis ran"
// marker (the file is a single deep-analysis unit, so once one S100 has
// landed the absence of another on the same publish is meaningful), then
// asserts the specific line in question.
suite("Shimmer precision (S100 deep review)", () => {
  const docUri = getDocUri("shimmerPrecision.tcl");

  async function s100Diagnostics(): Promise<vscode.Diagnostic[]> {
    await activate(docUri);
    // The settle signal for this file: once *some* S100 has landed, the deep
    // shimmer pass has run, so the absence of another S100 on the same publish
    // is meaningful rather than "not analysed yet".  That matters most for the
    // false-case tests below (lines 13/42/49), whose assertions are negative —
    // an empty or S100-free set satisfies every `!linesWithCode(...).includes(N)`
    // trivially.
    //
    // `waitForDiagnostics` rejects on timeout, so an unsettled analysis fails
    // here with the helper's diagnostic (what was awaited, how long, what was
    // last published) rather than returning a partial set for those negative
    // assertions to pass vacuously against.  This function used to re-assert
    // the predicate afterwards because the helper resolved leniently instead;
    // that check is now unreachable by construction (issue #1274).
    return waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "S100"),
    });
  }

  test("committed dict shimmered by lindex fires S100 with a tight span (true case)", async () => {
    const diags = await s100Diagnostics();
    const x = diags.find((d) => codeOf(d) === "S100" && d.range.start.line === 7);
    assert.ok(x, `expected S100 on line 7 (lindex $x 0); got [${linesWithCode(diags, "S100")}]`);
    assert.strictEqual(
      x.severity,
      vscode.DiagnosticSeverity.Information,
      "S100 should be informational",
    );
    // The span must cover only `$x` — not the whole `lindex $x 0` call.
    assert.strictEqual(x.range.start.character, 11, "must start at '$x'");
    assert.strictEqual(x.range.end.character, 13, "must end after '$x'");
  });

  test("a value already carrying a list intrep stays silent (false case)", async () => {
    const diags = await s100Diagnostics();
    assert.ok(
      !linesWithCode(diags, "S100").includes(13),
      "a clean list passed to lindex must not fire S100",
    );
  });

  test("shimmer is detected through an interp alias to a shimmering builtin (true case)", async () => {
    const diags = await s100Diagnostics();
    assert.ok(
      linesWithCode(diags, "S100").includes(20),
      `expected S100 through 'interp alias {} myindex {} ::lindex'; got [${linesWithCode(diags, "S100")}]`,
    );
  });

  test("shimmer fires inside a TclOO method body (true case)", async () => {
    const diags = await s100Diagnostics();
    assert.ok(
      linesWithCode(diags, "S100").includes(27),
      `expected S100 inside an oo::class method body; got [${linesWithCode(diags, "S100")}]`,
    );
  });

  test("shimmer fires inside a namespace eval body (true case)", async () => {
    const diags = await s100Diagnostics();
    assert.ok(
      linesWithCode(diags, "S100").includes(34),
      `expected S100 inside a namespace eval body; got [${linesWithCode(diags, "S100")}]`,
    );
  });

  test("a write-traced variable never fires shimmer (false-negative guard)", async () => {
    const diags = await s100Diagnostics();
    assert.ok(
      !linesWithCode(diags, "S100").includes(42),
      "a write-traced variable must widen to overdefined and stay silent",
    );
  });

  test("a matching noqa directive suppresses S100 (true suppression)", async () => {
    const diags = await s100Diagnostics();
    assert.ok(
      !linesWithCode(diags, "S100").includes(49),
      "'# noqa: S100' on the preceding line must suppress the diagnostic",
    );
  });

  test("a mismatched noqa directive does not suppress S100 (false-suppression guard)", async () => {
    const diags = await s100Diagnostics();
    assert.ok(
      linesWithCode(diags, "S100").includes(56),
      "'# noqa: W100' must not suppress an unrelated S100",
    );
  });
});

// Issue #940 — a pure list literal used as a list is a free first conversion,
// not a shimmer. `issue940Shimmer.tcl` (0-indexed lines):
//   4  foreach $fontSizes  FALSE — braced list literal (reporter's case)
//   6  foreach $empty      FALSE — the `{}` empty list in the issue title
//   8  foreach $a          FALSE — a number-shaped literal is still a valid list
//   10 dict for $d         FALSE — even-length list literal is a valid dict
//   12 foreach $committed  TRUE  — committed dict shimmers to a list
suite("Shimmer pure-literal suppression (issue #940)", () => {
  const docUri = getDocUri("issue940Shimmer.tcl");

  async function shimmerDiagnostics(): Promise<vscode.Diagnostic[]> {
    await activate(docUri);
    // The committed-dict case on line 12 is the "analysis ran" settle signal:
    // once its S100 lands, the absence of S100/S101 on the pure-literal lines
    // is meaningful rather than a not-yet-analysed empty set.  The wait rejects
    // if it never lands (issue #1274), so the follow-up assertion this function
    // used to carry — guarding against the helper's old lenient timeout — is
    // now unreachable.
    return waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "S100" && x.range.start.line === 12),
    });
  }

  test("braced list, empty {}, scalar, and dict literals do not shimmer", async () => {
    const diags = await shimmerDiagnostics();
    for (const line of [4, 6, 8, 10]) {
      assert.ok(
        !linesWithCode(diags, "S100").includes(line) &&
          !linesWithCode(diags, "S101").includes(line),
        `a pure list literal on line ${line} must not shimmer; got S100 [${linesWithCode(
          diags,
          "S100",
        )}] S101 [${linesWithCode(diags, "S101")}]`,
      );
    }
  });

  test("a committed container still shimmers (true-positive control)", async () => {
    const diags = await shimmerDiagnostics();
    assert.ok(
      linesWithCode(diags, "S100").includes(12),
      "a committed dict read as a list must still fire S100",
    );
  });
});
