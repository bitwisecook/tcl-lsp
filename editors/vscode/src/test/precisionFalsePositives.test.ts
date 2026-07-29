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

// FP-STY-12 — braced indirect-array-element idiom ${var}(idx).
//
// The fixture's last code line (`puts ${arr}(x)`, line index 8) is a value-
// position broken read and MUST fire W216 — that doubles as the marker that
// analysis ran.  The varname-position uses on lines 4–6 must stay silent for
// both W216 and W212.
suite("Indirect-array idiom (FP-STY-12)", () => {
  const docUri = getDocUri("indirectArray.tcl");

  test("value-position ${arr}(x) fires W216 (true case)", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W216"),
    });
    const w216Lines = linesWithCode(diags, "W216");
    assert.ok(w216Lines.length >= 1, "expected at least one W216 (analysis ran)");
    // Every W216 must be the value-position one on line 8 — never the
    // indirect-array varname-position uses on lines 4–6.
    for (const ln of w216Lines) {
      assert.strictEqual(ln, 8, `W216 fired on unexpected line ${ln} (only line 8 is value pos)`);
    }
  });

  test("varname-position ${token}(idx) stays silent for W216/W212 (false case)", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W216"),
    });
    // No W216 on the indirect lines (set / info exists / unset).
    for (const ln of linesWithCode(diags, "W216")) {
      assert.ok(![4, 5, 6].includes(ln), `unexpected W216 on indirect-array line ${ln}`);
    }
    // No W212 anywhere — the idiom is not a name-vs-value foot-gun.
    assert.strictEqual(
      linesWithCode(diags, "W212").length,
      0,
      "indirect-array idiom must not fire W212",
    );
  });
});

// FP-STY-13 — redefining an overridable Tcl library procedure.
//
// Lines 7–8 (`proc set` / `proc clock`) redefine genuine built-ins and MUST
// fire W113 (the marker that analysis ran).  Lines 3–5 redefine overridable
// library procs (`unknown`, `history`, `auto_execok`) and must stay silent.
suite("Overridable library procs (FP-STY-13)", () => {
  const docUri = getDocUri("libraryProcs.tcl");

  test("redefining a C built-in fires W113 (true case)", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W113"),
    });
    const w113Lines = linesWithCode(diags, "W113");
    // proc set (line 7) and proc clock (line 8) must both fire.
    assert.ok(w113Lines.includes(7), `expected W113 on 'proc set' (line 7); got [${w113Lines}]`);
    assert.ok(w113Lines.includes(8), `expected W113 on 'proc clock' (line 8); got [${w113Lines}]`);
  });

  test("redefining a library proc stays silent for W113 (false case)", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W113"),
    });
    // unknown / history / auto_execok on lines 3–5 must NOT fire W113.
    for (const ln of linesWithCode(diags, "W113")) {
      assert.ok(![3, 4, 5].includes(ln), `unexpected W113 on library-proc line ${ln}`);
    }
  });
});

// FP-STY-14 — single bare-variable body is a script reference, not a block.
//
// Lines 8–9 (`eval "do $script"` / `eval $cmd$args`) weave substitutions into
// an inline script and MUST fire W105 (the marker that analysis ran).  The
// bare-variable bodies on lines 2–5 (`eval $cmd`, `$state(-command)`,
// `after 0 $coroName`, dynamic `proc`) must stay silent.
suite("Single bare-variable body (FP-STY-14)", () => {
  const docUri = getDocUri("singleVarBody.tcl");

  test("composite/quoted interpolated body fires W105 (true case)", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W105"),
    });
    const w105Lines = linesWithCode(diags, "W105");
    assert.ok(
      w105Lines.includes(8),
      `expected W105 on 'eval "do $script"' (line 8); got [${w105Lines}]`,
    );
    assert.ok(
      w105Lines.includes(9),
      `expected W105 on 'eval $cmd$args' (line 9); got [${w105Lines}]`,
    );
  });

  test("single bare-variable body stays silent for W105 (false case)", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W105"),
    });
    for (const ln of linesWithCode(diags, "W105")) {
      assert.ok(![2, 3, 4, 5].includes(ln), `unexpected W105 on bare-var-body line ${ln}`);
    }
  });
});

// FP-RBS control-flow family (PR #634) — imprecise control-flow modelling.
//
// PR #634 fixed a family of false W210 (read-before-set) rooted in control-flow
// modelling.  The fixture's silent cases are a `tailcall`-terminated branch
// (line 3), a non-empty-literal `foreach` (line 7), and a `while 1` whose only
// exit is a `break` (line 11) — none may fire W210.  The empty-literal
// `foreach` (line 15) never runs its body, so `$y` is genuinely unset and MUST
// fire W210 — that doubles as the marker that analysis ran.
suite("Control-flow read-before-set (PR #634)", () => {
  const docUri = getDocUri("controlFlowRbs.tcl");

  test("empty-literal foreach read fires W210 (true case)", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W210"),
    });
    const w210Lines = linesWithCode(diags, "W210");
    assert.ok(
      w210Lines.includes(15),
      `expected W210 on the empty foreach read (line 15); got [${w210Lines}]`,
    );
  });

  test("tailcall / non-empty foreach / while-1-break stay silent (false case)", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W210"),
    });
    // None of the provably-defined reads on lines 3, 7, 11 may fire W210.
    for (const ln of linesWithCode(diags, "W210")) {
      assert.ok(
        ![3, 7, 11].includes(ln),
        `unexpected W210 on a provably-defined read (line ${ln})`,
      );
    }
  });

  // FP-RBS-19 (#756): a may-run loop whose body defines the variable is assumed
  // to run, so the after-loop reads (`return $acc` on line 21, `puts $y` on
  // line 25) must stay silent — while the provably-empty foreach read (line 15)
  // still fires.
  test("dynamic-loop after-loop reads stay silent (false case, #756)", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W210"),
    });
    for (const ln of linesWithCode(diags, "W210")) {
      assert.ok(
        ![21, 25].includes(ln),
        `unexpected W210 on a may-run-loop after-loop read (line ${ln})`,
      );
    }
  });
});

// FP-STY-15 — `$` before a closing `"` is a literal regex end-anchor.
//
// Line 7 (`regexp -- "^foo$bar" $text`) has a genuine live `$bar` substitution
// and MUST fire W306 (the marker that analysis ran).  The end-anchor cases on
// lines 3–5 (`"\n$"`, `"abc$"`, `"^foo$"`) must stay silent for W306 — and the
// lexer must not merge the quoted word with the next (no E002/E205).
suite("Dollar-before-close-quote (FP-STY-15)", () => {
  const docUri = getDocUri("dollarCloseQuote.tcl");

  test("live $bar in quoted pattern fires W306 (true case)", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W306"),
    });
    const w306Lines = linesWithCode(diags, "W306");
    assert.ok(w306Lines.includes(7), `expected W306 on live $bar (line 7); got [${w306Lines}]`);
  });

  test("regex end-anchor stays silent, no E002/E205/W306 (false case)", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W306"),
    });
    // No W306 on the end-anchor lines 3–5.
    for (const ln of linesWithCode(diags, "W306")) {
      assert.ok(![3, 4, 5].includes(ln), `unexpected W306 on end-anchor line ${ln}`);
    }
    // The lexer-merge symptoms (arity / extra-chars) must be absent entirely.
    assert.strictEqual(linesWithCode(diags, "E002").length, 0, "no spurious E002 arity errors");
    assert.strictEqual(
      linesWithCode(diags, "E205").length,
      0,
      "no spurious E205 close-quote errors",
    );
  });
});

// FP-STY-17 — same-file shadow suppression for W001 (unknown subcommand).
//
// Line 6 (`string reverse hello`, proc shadow) and line 9 (`info bogus`,
// alias shadow) must stay silent for W001 — the call resolves to the
// same-file proc/alias, not the registry ensemble. Line 12 (`dict bogus a
// b`, untouched by either shadow) MUST fire W001 — the marker that
// analysis ran.
suite("Ensemble-command shadow suppression (FP-STY-17)", () => {
  const docUri = getDocUri("ensembleShadowing.tcl");

  test("unshadowed ensemble fires W001 (true case)", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W001"),
    });
    const w001Lines = linesWithCode(diags, "W001");
    assert.ok(
      w001Lines.includes(12),
      `expected W001 on unshadowed 'dict bogus' (line 12); got [${w001Lines}]`,
    );
  });

  test("proc/alias shadowed ensemble calls stay silent (false case)", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W001"),
    });
    for (const ln of linesWithCode(diags, "W001")) {
      assert.ok([6, 9].includes(ln) === false, `unexpected W001 on shadowed-call line ${ln}`);
    }
  });
});

// FP-STY-18 — `{*}`-expanded subcommand position for W001.
//
// Line 4 (`dict {*}{create a b}`) splices list elements into the argument
// list — a genuine valid call — and must stay silent for W001. Line 5
// (`dict bogus a b`, unexpanded) MUST fire W001 (the marker that analysis
// ran).
suite("Expanded subcommand position (FP-STY-18)", () => {
  const docUri = getDocUri("expandedSubcommand.tcl");

  test("genuine unknown subcommand without expansion fires W001 (true case)", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W001"),
    });
    const w001Lines = linesWithCode(diags, "W001");
    assert.ok(
      w001Lines.includes(5),
      `expected W001 on unexpanded 'dict bogus' (line 5); got [${w001Lines}]`,
    );
  });

  test("{*}-expanded literal subcommand stays silent (false case)", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W001"),
    });
    for (const ln of linesWithCode(diags, "W001")) {
      assert.notStrictEqual(ln, 4, `unexpected W001 on {*}-expanded line ${ln}`);
    }
  });
});

// FP-SH-13 / FP-SH-15 / FP-SH-18 — command/variable indirection and the
// array-element / numeric-shimmer precision fixes, end-to-end through the
// real server (fixture `shimmerIndirection.tcl`).
//
//   • `aliased` (lines 4–10): an `interp alias {} myset {} set` store still
//     resolves to the real `set`, so its int/string oscillation fires S102.
//   • `arrayelems` (lines 14–18): `arr(n)` (int) and `arr(label)` (string)
//     collapse onto one symbol but are independent slots — no S100/S101.
//   • `numeric` (lines 23–34): a Numeric/String oscillation seeded by an Int
//     entry, previously masked to OVERDEFINED by `type_join` — now fires S102.
suite("Command/variable indirection + numeric shimmer (FP-SH-13/15/18)", () => {
  const docUri = getDocUri("shimmerIndirection.tcl");

  test("aliased set and int-seeded numeric loops fire S102 (true case)", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "S102"),
    });
    const s102Lines = linesWithCode(diags, "S102");
    assert.ok(s102Lines.length >= 1, "expected at least one S102 (analysis ran)");
    // The aliased-store oscillation fires inside `aliased`'s loop body (7–8).
    assert.ok(
      s102Lines.some((ln) => ln >= 5 && ln <= 9),
      `expected S102 in the aliased loop (lines 5-9); got [${s102Lines}]`,
    );
    // The numeric/string oscillation fires inside `numeric`'s loop (>= 24).
    assert.ok(
      s102Lines.some((ln) => ln >= 24),
      `expected S102 in the numeric loop (line >= 24); got [${s102Lines}]`,
    );
  });

  test("array-element conflation stays silent for S100/S101 (false case)", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "S102"),
    });
    // No S100/S101 on the array-element proc lines (15–18).
    for (const code of ["S100", "S101"]) {
      for (const ln of linesWithCode(diags, code)) {
        assert.ok(
          !(ln >= 14 && ln <= 18),
          `unexpected ${code} on array-element line ${ln} (independent slots)`,
        );
      }
    }
  });
});

// W212/W216 registry-driven name positions.
//
// `upvar 1 remote ${arr}(x)` (line 3) is the legitimate indirect-array idiom
// in a name position — no W216, no W212. `catch {…} $res` (line 4) is a
// genuine name/value confusion the old hardcoded list missed — W212, and the
// marker that analysis ran.
suite("Variable-name positions (W212/W216)", () => {
  const docUri = getDocUri("nameVsValuePositions.tcl");

  test("catch result-var substitution fires W212 (registry FN fix)", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W212"),
    });
    assert.ok(linesWithCode(diags, "W212").includes(4), "catch {…} $res must fire W212 on line 4");
  });

  test("upvar local ${arr}(x) is the indirect-array idiom — no W216/W212", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W212"),
    });
    // No W216 anywhere (the upvar local-name slot is the legitimate idiom).
    assert.strictEqual(
      linesWithCode(diags, "W216").length,
      0,
      "upvar local idiom must not fire W216",
    );
    // No W212 on the upvar line (line 3) — only the catch line (4).
    assert.ok(!linesWithCode(diags, "W212").includes(3), "upvar line must not fire W212");
  });
});

// The `Tcl_ConcatObj` eval family (issue #1051).
//
// `eval`, `uplevel`, `namespace eval`, and `interp eval` evaluate the
// *concatenation* of every trailing script word, so analysing only the first
// one invents an E002 and loses the writes the joined script performs. The
// fixture's last line (`eval set`, line index 9) is a genuinely short joined
// script and MUST fire E002 — that doubles as the marker that analysis ran.
// `namespace inscope`'s tail is list-appended rather than joined, so its line
// must stay silent too.
suite("Multi-word eval concatenation (#1051)", () => {
  const docUri = getDocUri("multiWordEval.tcl");

  test("a genuinely short joined script still fires E002 (true case)", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "E002"),
    });
    assert.deepStrictEqual(
      linesWithCode(diags, "E002"),
      [9],
      "only the `eval set` line is a short joined script",
    );
  });

  test("well-formed multi-word eval draws no E002 and no W210 (false case)", async () => {
    await activate(docUri);
    const diags = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "E002"),
    });
    for (const ln of linesWithCode(diags, "E002")) {
      assert.strictEqual(ln, 9, `E002 fired on well-formed eval line ${ln}`);
    }
    // The joined scripts really do set `total` and `label`, so the reads on
    // lines 3 and 5 are not read-before-set.
    assert.strictEqual(
      linesWithCode(diags, "W210").length,
      0,
      "a variable a multi-word eval sets must not read as read-before-set",
    );
  });
});
