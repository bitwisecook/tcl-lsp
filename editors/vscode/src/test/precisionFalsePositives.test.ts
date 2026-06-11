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
    assert.ok(w105Lines.includes(8), `expected W105 on 'eval "do $script"' (line 8); got [${w105Lines}]`);
    assert.ok(w105Lines.includes(9), `expected W105 on 'eval $cmd$args' (line 9); got [${w105Lines}]`);
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
    assert.strictEqual(linesWithCode(diags, "E205").length, 0, "no spurious E205 close-quote errors");
  });
});
