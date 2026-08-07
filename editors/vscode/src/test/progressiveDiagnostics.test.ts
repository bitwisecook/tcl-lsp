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
import * as fs from "fs";
import { getDocUri, activate, waitForDiagnostics, scaledTimeout } from "./helper";
import { MAX_TEST_TIMEOUT_BASE_MS } from "./runnerWatchdog";

// Issue #844: diagnostics are progressive. On a large document the server
// publishes a workspace-independent *fast tier* (the analyser's syntax /
// structural / style diagnostics) as soon as the per-file walk lands — before
// the compiler/optimiser + cross-file + refinement passes — then replaces it
// with the complete *deep tier* for the same version. This drives that pipeline
// through the real extension end to end and asserts (a) the deep tier surfaces
// both the fast-tier analyser code and a deep-only optimiser code, and (b) when
// the split is observable, the fast tier is published first.
//
// The fast→deep ordering / subset property is asserted more strictly — when the
// race is observable — by the Rust lsp_e2e suite
// (`large_file_publishes_fast_tier_before_deep_tier`); like this test, that one
// falls back to a completeness-only check on a host fast enough to coalesce the
// two waves into a single publish. This test is the editor-integration companion.
suite("Progressive diagnostics (#844)", () => {
  const fixtureName = "progressiveDiagnostics.tcl";
  const docUri = getDocUri(fixtureName);

  function codeOf(d: vscode.Diagnostic): string {
    return String(typeof d.code === "object" ? d.code?.value : d.code);
  }

  suiteSetup(() => {
    // A document comfortably above the server's fast-tier line-count floor so
    // its deep pass overruns the budget and publishes progressively. The procs
    // are clean; the distinguishing pair is one unbraced `expr`, which yields a
    // fast-tier W100 (analyser) and its deep-tier-only paired optimiser hint
    // O111.
    let body = "namespace eval ::bench {\n    variable counter 0\n}\n\n";
    for (let i = 0; i < 600; i++) {
      body +=
        `proc ::bench::step${i} {a b} {\n` +
        `    set v${i} [expr {$a + $b}]\n` +
        `    if {$v${i} > 10} {\n` +
        `        set v${i} [expr {$v${i} + 1}]\n` +
        `    }\n` +
        `    return $v${i}\n` +
        `}\n\n`;
    }
    body += "proc ::bench::w100_probe {x} {\n    return [expr $x + 1]\n}\n";
    fs.writeFileSync(docUri.fsPath, body);
  });

  suiteTeardown(() => {
    try {
      fs.unlinkSync(docUri.fsPath);
    } catch {
      // Already gone — nothing to clean up.
    }
  });

  test("a large file surfaces the complete deep tier, fast tier first", async function () {
    // A per-test backstop must outlast the sum of the bounded waits inside the
    // test, or it fires first and reports "Timeout of Nms exceeded" — naming
    // neither the wait that was outstanding nor why, which is the failure shape
    // issue #1274 set out to remove. This test's waits are `activate` (up to
    // 60s for a cold LSP handshake, plus its two 30s didOpen drains) followed
    // by the 50s deep-tier wait below, so 60s could not cover them even on an
    // idle machine. Load-scaled for the same reason every other bound here is:
    // it used to be a raw `60_000`, and a loaded container turned a correct run
    // into an unattributable timeout.
    this.timeout(scaledTimeout(MAX_TEST_TIMEOUT_BASE_MS));

    // Capture the sequence of diagnostic publishes for this URI. Registered
    // before `activate` (which sends `didOpen`) so no publish is missed — a
    // watched-file CREATE for an unopened file publishes nothing, so the run
    // starts at `didOpen`.
    const publishes: string[][] = [];
    const disposable = vscode.languages.onDidChangeDiagnostics((e) => {
      if (e.uris.some((u) => u.toString() === docUri.toString())) {
        publishes.push(vscode.languages.getDiagnostics(docUri).map(codeOf));
      }
    });

    try {
      await activate(docUri);
      // Wait for the deep tier to land. The optimiser O111 (paired with the
      // analyser W100) is produced only by the deep pass, so its presence marks
      // the deep tier — and the final set must carry both, proving nothing is
      // lost by going progressive. Keyed on the diagnostics themselves (not the
      // server-log ring buffer, which a late-running suite can have already
      // filled past its cap).
      const finalDiags = await waitForDiagnostics(docUri, {
        predicate: (diags) => {
          const codes = diags.map(codeOf);
          return codes.includes("W100") && codes.includes("O111");
        },
        timeout: 50_000,
      });
      const finalCodes = finalDiags.map(codeOf);
      assert.ok(
        finalCodes.includes("W100"),
        `deep tier must surface the analyser W100 through the extension: [${finalCodes}]`,
      );
      assert.ok(
        finalCodes.includes("O111"),
        `deep tier must surface the optimiser O111 through the extension: [${finalCodes}]`,
      );

      // Progressive delivery: a publish carrying W100 but NOT the deep-only O111
      // (the fast tier) should precede one carrying both (the deep tier).
      const fastIdx = publishes.findIndex((c) => c.includes("W100") && !c.includes("O111"));
      const deepIdx = publishes.findIndex((c) => c.includes("W100") && c.includes("O111"));
      if (fastIdx >= 0 && deepIdx >= 0) {
        assert.ok(
          fastIdx < deepIdx,
          `fast tier (W100, no O111) must be published before the deep tier ` +
            `(W100 + O111): fast@${fastIdx} deep@${deepIdx}`,
        );
      } else {
        // On an unexpectedly fast host the whole pipeline can land inside the
        // budget and coalesce to one publish. The split itself is asserted (when
        // observable) by the Rust lsp_e2e suite, which carries the same coalesce
        // fallback; completeness above is still asserted here.
        console.log(
          "progressive diagnostics: fast/deep split coalesced into one publish " +
            "this run — completeness still asserted",
        );
      }
    } finally {
      disposable.dispose();
    }
  });
});
