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
    assert.ok(codes.includes("E100"), `Expected E100 (stray close bracket) in [${codes}]`);
    assert.ok(codes.includes("E102"), `Expected E102 (stray close brace) in [${codes}]`);
  });

  test("E100 range covers the stray ']' character itself", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 1 });

    const e100 = diagnostics.find((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "E100";
    });
    assert.ok(e100, "E100 diagnostic should be present");

    // `set y string]` — the range must end one column past the `]`
    // (LSP ranges are end-exclusive), not stop short of it.
    const line = vscode.window.activeTextEditor!.document.lineAt(e100!.range.end.line).text;
    assert.strictEqual(line[e100!.range.end.character - 1], "]");
  });

  test("E102 range covers the stray '}' character itself", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 1 });

    const e102 = diagnostics.find((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "E102";
    });
    assert.ok(e102, "E102 diagnostic should be present");

    const line = vscode.window.activeTextEditor!.document.lineAt(e102!.range.end.line).text;
    assert.strictEqual(line[e102!.range.end.character - 1], "}");
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

  test("T100 fires for a tainted if-condition operand but not for a pure string compare", async () => {
    const taintUri = getDocUri("diagnostics-taint-t100.tcl");
    await activate(taintUri);
    const diagnostics = await waitForDiagnostics(taintUri, { minCount: 2 });

    const t100 = diagnostics.filter((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "T100";
    });

    // Two T100 hits: the direct `eval $cmd` sink, and `$n` as a numeric
    // operand of `+` inside the `if` condition — conditions aren't a bare
    // `expr` statement, but are evaluated exactly like one.
    assert.ok(t100.length >= 2, `Expected at least 2 T100 diagnostics, got ${t100.length}`);
    assert.ok(
      t100.every((d) => d.severity === vscode.DiagnosticSeverity.Warning),
      "T100 should be a warning",
    );
    assert.ok(
      t100.some((d) => d.message.includes("eval")),
      `Expected a T100 for the eval sink, got: ${t100.map((d) => d.message).join("; ")}`,
    );
    assert.ok(
      t100.some((d) => d.message.includes("numeric coercion")),
      `Expected a T100 for the if-condition numeric operand, got: ${t100.map((d) => d.message).join("; ")}`,
    );

    // The `if {$who eq "admin"}` branch (pure string compare) and the
    // `subst -nocommands $template` call (command substitution disabled)
    // must not raise T100 at all.
    assert.ok(
      t100.every((d) => !d.message.includes("who") && !d.message.includes("template")),
      `Neither $who (eq compare) nor $template (subst -nocommands) should raise T100, got: ${t100
        .map((d) => d.message)
        .join("; ")}`,
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

  test("E004 fires for each malformed `if` clause shape, precisely and not for well-formed ones", async () => {
    const e004Uri = getDocUri("diagnostics-e004.tcl");
    await activate(e004Uri);
    const diagnostics = await waitForDiagnostics(e004Uri, { minCount: 3 });

    const e004 = diagnostics.filter((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "E004";
    });

    // Exactly the three genuinely malformed `if`s in the fixture — the
    // leading-`else`-as-condition and the well-formed elseif/else chain
    // must not add a fourth or fifth.
    assert.strictEqual(
      e004.length,
      3,
      `Expected exactly 3 E004 diagnostics, got ${e004.length}: ${e004.map((d) => `${d.range.start.line}:${d.message}`).join("; ")}`,
    );

    const messages = e004.map((d) => d.message);
    assert.ok(
      messages.some((m) => m === 'No script following "1" argument'),
      `Expected the "if {1}" case's message, got: ${messages.join("; ")}`,
    );
    assert.ok(
      messages.some((m) => m === 'No script following "2" argument'),
      `Expected the dangling "elseif {2}" case's message, got: ${messages.join("; ")}`,
    );
    assert.ok(
      messages.some((m) => m === 'Extra words after "else" clause in "if" command'),
      `Expected the extra-words case's message, got: ${messages.join("; ")}`,
    );

    // Every E004 is an error. The fixture's `if` heads sit at lines 3, 6,
    // and 11 — a diagnostic landing on line 6 or 11 would mean the span
    // regressed to whole-statement.
    for (const d of e004) {
      assert.strictEqual(d.severity, vscode.DiagnosticSeverity.Error, "E004 should be an error");
    }
    const byMessage = new Map(e004.map((d) => [d.message, d.range]));

    // `if {1}` is a single-line statement, so its own tight anchor and a
    // (hypothetical) whole-statement span would coincide on the line —
    // the single-line range itself is the tight-anchoring proof here.
    const conditionRange = byMessage.get('No script following "1" argument');
    assert.strictEqual(conditionRange?.start.line, 3, `"if {1}" should anchor on line 3`);
    assert.strictEqual(
      conditionRange?.start.line,
      conditionRange?.end.line,
      `"if {1}" should anchor on a single line, got ${JSON.stringify(conditionRange)}`,
    );

    // The dangling `elseif {2}` sits on line 8; anchoring on line 6 (the
    // `if` head) would mean the span regressed to whole-statement.
    const elseifRange = byMessage.get('No script following "2" argument');
    assert.strictEqual(elseifRange?.start.line, 8, `dangling "elseif {2}" should anchor on line 8`);
    assert.strictEqual(
      elseifRange?.start.line,
      elseifRange?.end.line,
      `dangling "elseif {2}" should anchor on a single line, got ${JSON.stringify(elseifRange)}`,
    );

    // The recognised final (multi-line) body legitimately spans several
    // lines — the tightness property here is that it starts *after* the
    // `if` head (line 11), not that it is single-line.
    const extraWordsRange = byMessage.get('Extra words after "else" clause in "if" command');
    assert.ok(
      extraWordsRange !== undefined && extraWordsRange.start.line > 11,
      `extra-words should anchor past line 11 (the \`if\` head), got ${JSON.stringify(extraWordsRange)}`,
    );
  });

  test("E004 does not fire for a leading `else` bareword condition or a well-formed elseif/else chain", async () => {
    const e004Uri = getDocUri("diagnostics-e004.tcl");
    await activate(e004Uri);
    const diagnostics = await waitForDiagnostics(e004Uri, { minCount: 3 });

    const e004 = diagnostics.filter((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "E004";
    });
    // Neither the `if else { ... }` fixture block (lines 20-23) nor the
    // well-formed elseif/else chain (lines 25-32) contributes an E004.
    assert.ok(
      e004.every((d) => d.range.start.line < 19),
      `Expected no E004 at/after line 19 (the leading-else and well-formed blocks), got: ${e004.map((d) => `${d.range.start.line}:${d.message}`).join("; ")}`,
    );
  });

  test("clean file produces no diagnostics", async () => {
    const cleanUri = getDocUri("simple.tcl");

    // Disable optimiser so info-level suggestions (O1xx) don't count.
    const config = vscode.workspace.getConfiguration("tclLsp.optimiser");
    await config.update("enabled", false, vscode.ConfigurationTarget.Global);

    try {
      // Wait on the server's resolved config (message passing) rather than a
      // fixed sleep, so the optimiser.enabled=false round-trip is observed to
      // have applied before analysing.  Kept inside the `try` so a wait
      // timeout still restores the global setting in `finally`.  20s,
      // matching waitForDeepDiagnostics's default: under the full suite's
      // background load (workspace warm-up, the #844 progressive
      // diagnostics race, …) this round-trip routinely needs more than the
      // 5s generic default.
      await waitForEffectiveConfig(cleanUri, (cfg) => cfg.optimiser_enabled === false, {
        label: "optimiser.enabled = false",
        timeout: 20000,
      });

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

  test("dict-for body nesting keeps reads alive but still catches real dead stores (#833)", async () => {
    const uri = getDocUri("dict-for-nesting.tcl");
    await activate(uri);
    // The body-internal dead store yields exactly one W220, proving analysis ran
    // AND that the analyser walked into the (now-lowered) dict-for body.
    const diagnostics = await waitForDiagnostics(uri, { minCount: 1 });
    const codeOf = (d: vscode.Diagnostic) => (typeof d.code === "object" ? d.code.value : d.code);
    const codes = diagnostics.map(codeOf);

    const w220 = diagnostics.filter((d) => codeOf(d) === "W220");
    // The ONLY dead store is `set tmp 1` inside the second proc's body (line 18,
    // 0-indexed). The `set x set` read via `$x a $key` nested in `if`/`dict for`
    // must NOT be flagged.
    assert.strictEqual(w220.length, 1, `expected one W220, got [${codes}]`);
    assert.strictEqual(
      w220[0].range.start.line,
      18,
      `W220 should anchor to the body-internal dead store, got line ${w220[0].range.start.line}`,
    );
    // No unused/dead-store hint on `x` (the command-name read keeps it live).
    for (const d of diagnostics) {
      const line = d.range.start.line;
      assert.ok(
        !((codeOf(d) === "W220" || codeOf(d) === "W211") && line === 6),
        `unexpected ${codeOf(d)} on 'set x set' (it is read via $x) at line ${line}`,
      );
    }
  });

  test("uplevel body is analysed: braced body clean, unbraced body flags W105 (#837)", async () => {
    const uri = getDocUri("uplevel-frame.tcl");
    await activate(uri);
    // The unbraced substituted body (`uplevel 1 "puts $x"`) yields a W105,
    // proving the analyser now walks the uplevel body arg.
    const diagnostics = await waitForDiagnostics(uri, { minCount: 1 });
    const codeOf = (d: vscode.Diagnostic) => (typeof d.code === "object" ? d.code.value : d.code);
    const codes = diagnostics.map(codeOf);

    const w105 = diagnostics.filter((d) => codeOf(d) === "W105");
    assert.strictEqual(w105.length, 1, `expected one W105 (unbraced body), got [${codes}]`);
    // W105 anchors to the unbraced `uplevel 1 "puts $x"` body (line 8, 0-indexed).
    assert.strictEqual(
      w105[0].range.start.line,
      8,
      `W105 should anchor to the unbraced uplevel body, got line ${w105[0].range.start.line}`,
    );
    // The clean braced body (forgetXyce, lines 0-4) must not carry any
    // dead-store / read-before-set hint — it is correct caller-frame code.
    for (const d of diagnostics) {
      const line = d.range.start.line;
      const code = codeOf(d);
      assert.ok(
        !(["W210", "W211", "W220"].includes(String(code)) && line <= 4),
        `unexpected ${code} on the clean braced uplevel body at line ${line}`,
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

  test("S102 fires for loop-carried oscillation, anchored inside the loop, and is silent under a write trace", async () => {
    // Both scenarios live in one fixture (like optimisation-o101.tcl): the
    // top `accumulate` proc genuinely oscillates x between int and string
    // every pass (S102's own KCS canonical example); the bottom `traced`
    // proc has the identical shape but under a `trace add variable … write`
    // — a write trace can rewrite the value's type on every access, so the
    // literal-only view must not drive S102 (deep-review FP guard).
    const uri = getDocUri("shimmerOscillation.tcl");
    await activate(uri);
    const codeOf = (d: vscode.Diagnostic) => (typeof d.code === "object" ? d.code.value : d.code);
    const diagnostics = await waitForDiagnostics(uri, {
      predicate: (diags) => diags.some((d) => codeOf(d) === "S102"),
    });

    const s102s = diagnostics.filter((d) => codeOf(d) === "S102");
    assert.ok(s102s.length > 0, "expected at least one S102 diagnostic");
    assert.strictEqual(s102s[0].severity, vscode.DiagnosticSeverity.Warning, "S102 is a warning");

    // Precision: anchored inside `accumulate`'s loop body (line >= 3), not
    // the pre-loop initialiser `set x 0` (line 1).
    assert.ok(
      s102s.some((d) => d.range.start.line >= 3 && d.range.start.line <= 4),
      `expected S102 anchored inside the loop body (lines 3-4), got lines [${s102s.map((d) => d.range.start.line)}]`,
    );

    // FP guard: no S102 anywhere inside `traced` (lines 8-15).
    assert.ok(
      !s102s.some((d) => d.range.start.line >= 8),
      `traced variable must not fire S102, got lines [${s102s.map((d) => d.range.start.line)}]`,
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

  // Same-file proc-call arity: calling a same-file proc with the wrong
  // number of arguments previously produced no diagnostic at all — the
  // E002/E003 arity check was wired only to the builtin command registry.
  // The fixture also covers `forward NAME my TARGET ?ARG…?`, the TclOO
  // idiom for forwarding to a sibling method (a bare method name is never
  // a valid forward target — confirmed against tclsh 9.0.4).
  test("E003 fires for a same-file proc call with too many arguments", async () => {
    const uri = getDocUri("diagnostics-arity.tcl");
    await activate(uri);
    const codeOf = (d: vscode.Diagnostic) => (typeof d.code === "object" ? d.code.value : d.code);
    const diagnostics = await waitForDiagnostics(uri, {
      predicate: (diags) => diags.some((d) => codeOf(d) === "E003"),
    });
    const e003 = diagnostics.filter((d) => codeOf(d) === "E003");
    assert.strictEqual(
      e003.length,
      2,
      `expected exactly two E003s, got [${diagnostics.map(codeOf)}]`,
    );
    assert.ok(
      e003.some((d) => d.message.includes("demonstrate")),
      `an E003 message should name the proc, got: ${e003.map((d) => d.message)}`,
    );
    assert.ok(
      e003.some((d) => d.message.includes("fwd")),
      `an E003 message should name the forward, got: ${e003.map((d) => d.message)}`,
    );
    for (const d of e003) {
      assert.strictEqual(d.severity, vscode.DiagnosticSeverity.Error, "E003 should be an error");
    }
    // The correctly-arg-counted `need3 1 2 3` call must not also fire.
    assert.ok(
      !diagnostics.some((d) => codeOf(d) === "E002"),
      `unexpected E002 in [${diagnostics.map(codeOf)}]`,
    );
  });

  // TclOO constructor calls (`ClassName new` / `ClassName create`) and
  // direct `apply {{params} body}` lambda calls previously produced no
  // arity diagnostic at all, however wrong the argument count.
  test("E002/E003 fire for TclOO constructor calls and apply lambdas", async () => {
    const uri = getDocUri("diagnostics-arity-tcloo-ctor.tcl");
    await activate(uri);
    const codeOf = (d: vscode.Diagnostic) => (typeof d.code === "object" ? d.code.value : d.code);
    const diagnostics = await waitForDiagnostics(uri, {
      predicate: (diags) => diags.filter((d) => codeOf(d) === "E002").length >= 3,
    });
    const codes = diagnostics.map(codeOf).sort();
    assert.deepStrictEqual(
      codes,
      ["E002", "E002", "E002", "E003"],
      `expected exactly 3×E002 + 1×E003, got [${diagnostics.map(codeOf)}]`,
    );
    const e002 = diagnostics.filter((d) => codeOf(d) === "E002");
    assert.ok(
      e002.some((d) => d.message.includes("Widget new")),
      `an E002 message should name the constructor call, got: ${e002.map((d) => d.message)}`,
    );
    assert.ok(
      e002.some((d) => d.message.includes("Sub new")),
      `an E002 message should cover an inherited constructor, got: ${e002.map((d) => d.message)}`,
    );
    assert.ok(
      e002.some((d) => d.message.includes("apply")),
      `an E002 message should name apply, got: ${e002.map((d) => d.message)}`,
    );
    const e003 = diagnostics.find((d) => codeOf(d) === "E003");
    assert.ok(
      e003?.message.includes("Widget create"),
      `the E003 message should name the create call, got: ${e003?.message}`,
    );
    for (const d of diagnostics) {
      assert.strictEqual(
        d.severity,
        vscode.DiagnosticSeverity.Error,
        `${codeOf(d)} should be an error`,
      );
    }
  });

  // W004 (option not available in the active dialect): an abbreviated
  // ensemble subcommand (`chan conf` ⇒ `configure`) must still be resolved
  // against its own option table, and a same-file proc that redefines a
  // builtin must suppress the diagnostic for calls that resolve to it.
  test("W004 fires for a dialect-gated option, including on an abbreviated subcommand", async () => {
    const uri = getDocUri("diagnostics-w004.tcl");
    await activate(uri);
    const codeOf = (d: vscode.Diagnostic) => (typeof d.code === "object" ? d.code.value : d.code);
    const diagnostics = await waitForDiagnostics(uri, {
      predicate: (diags) => diags.filter((d) => codeOf(d) === "W004").length >= 2,
    });
    const w004 = diagnostics.filter((d) => codeOf(d) === "W004");
    assert.strictEqual(
      w004.length,
      2,
      `expected exactly two W004s (the proc-shadowed call must be suppressed), got [${w004.map((d) => d.message)}]`,
    );
    assert.ok(
      w004.some((d) => d.message.includes("-stride") && d.message.includes("lsearch")),
      `a W004 message should name lsearch's -stride, got: ${w004.map((d) => d.message)}`,
    );
    assert.ok(
      w004.some((d) => d.message.includes("-inputmode") && d.message.includes("'chan' configure")),
      `a W004 message should resolve the abbreviated 'chan conf' to 'chan' configure, got: ${w004.map((d) => d.message)}`,
    );
    for (const d of w004) {
      assert.strictEqual(d.severity, vscode.DiagnosticSeverity.Warning, "W004 should be a warning");
    }
  });

  // E001 ("missing dispatch word"): a subcommand-dispatch registry command
  // (`string`) or a TclOO object (`$o`) invoked with no dispatch word at all
  // is a genuine arity error, tightly highlighted at just the command head.
  // `history` (bare call defaults to `history info` per history(n)) and
  // snit (unmodelled generated dispatcher) are the false-positive carve-outs
  // a correct implementation must not flag.
  test("E001 fires for bare `string` and bare TclOO object dispatch only", async () => {
    const uri = getDocUri("diagnostics-e001.tcl");
    await activate(uri);
    const codeOf = (d: vscode.Diagnostic) => (typeof d.code === "object" ? d.code.value : d.code);
    const diagnostics = await waitForDiagnostics(uri, {
      predicate: (diags) => diags.filter((d) => codeOf(d) === "E001").length >= 2,
    });
    const e001 = diagnostics.filter((d) => codeOf(d) === "E001");
    assert.strictEqual(
      e001.length,
      2,
      `expected exactly two E001s (bare 'string' + bare '$o'), got [${diagnostics.map(codeOf)}]` +
        ` — history/snit false positives would inflate this count`,
    );
    for (const d of e001) {
      assert.strictEqual(d.severity, vscode.DiagnosticSeverity.Error, "E001 should be an error");
    }

    const bareString = e001.find((d) => d.message.includes("string"));
    assert.ok(
      bareString,
      `an E001 message should name 'string', got: ${e001.map((d) => d.message)}`,
    );
    // Tight highlighting: the span must cover only the command word, line 6
    // (0-indexed) columns 0-6 — no subcommand exists to extend it over.
    assert.strictEqual(bareString.range.start.line, 6);
    assert.strictEqual(bareString.range.start.character, 0);
    assert.strictEqual(bareString.range.end.line, 6);
    assert.strictEqual(bareString.range.end.character, 6);

    const bareObject = e001.find((d) => d.message.includes("requires a method"));
    assert.ok(
      bareObject,
      `an E001 message should report the missing TclOO method, got: ${e001.map((d) => d.message)}`,
    );
    assert.strictEqual(bareObject.range.start.line, 22);
  });

  // Issue #832: `autoloadLibrary.tcl` calls two commands the workspace's
  // `rbclib/tclIndex` auto-loads (`Rbc_ActiveLegend` / `Rbc_ZoomStack`, the
  // BLT/Rbc idiom) with no `package require`, plus one genuinely-unknown
  // command. The package database resolves the library commands exactly as
  // go-to-definition does, so they must NOT be flagged "Unknown command"
  // W002 (disabled-in-dialect command): a genuinely disabled command with no
  // shadowing definition must fire, while a same-file proc / interp alias /
  // forward-declared proc-body shadow — resolved with Tcl's real
  // namespace-then-global, load-order-aware rules — must not.
  test("W002 fires only for the unshadowed disabled call, not the shadowed ones", async () => {
    const uri = getDocUri("w002.tcl");
    const doc = await activate(uri);
    const codeOf = (d: vscode.Diagnostic) => (typeof d.code === "object" ? d.code.value : d.code);

    const text = doc.getText();
    const lineOf = (needle: string): number => {
      const idx = text.indexOf(needle);
      assert.ok(idx >= 0, `fixture must contain ${JSON.stringify(needle)}`);
      return doc.positionAt(idx).line;
    };
    const unshadowedLine = lineOf("dict create a 1");
    const namespaceShadowedLine = lineOf("dict foo bar");
    const aliasShadowedLine = lineOf("aliasedDisabled foo bar");
    const forwardDeclaredLine = lineOf("lmap x {1 2 3}");

    const w002On = (diags: vscode.Diagnostic[], line: number) =>
      diags.some((d) => codeOf(d) === "W002" && d.range.start.line === line);

    const diagnostics = await waitForDiagnostics(uri, {
      predicate: (diags) => w002On(diags, unshadowedLine),
    });

    assert.ok(
      w002On(diagnostics, unshadowedLine),
      `expected W002 on the unshadowed 'dict create' call (line ${unshadowedLine})`,
    );
    assert.ok(
      diagnostics.some((d) => codeOf(d) === "W002" && /available in: tcl8\.5/.test(d.message)),
      `expected the W002 message to name the dialects dict is available in: ` +
        `${diagnostics.map((d) => d.message).join("; ")}`,
    );
    assert.ok(
      !w002On(diagnostics, namespaceShadowedLine),
      `a namespace-scoped shadowing proc must suppress W002 (line ${namespaceShadowedLine})`,
    );
    assert.ok(
      !w002On(diagnostics, aliasShadowedLine),
      `an interp alias establishing the name must suppress W002 (line ${aliasShadowedLine})`,
    );
    assert.ok(
      !w002On(diagnostics, forwardDeclaredLine),
      `a forward-declared proc-body shadow must suppress W002 (line ${forwardDeclaredLine})`,
    );
  });

  // (W123) — while the genuine unknown still is. `xcDiagnostics` stays off.
  test("auto_path library commands are not unknown (issue #832)", async () => {
    const uri = getDocUri("autoloadLibrary.tcl");
    // W123 defaults to false (opt-in, see configSettings.test.ts's
    // "diagnostics.W123 defaults to false" test) -- without this the whole
    // assertion is vacuous: nothing is ever flagged, on lines 0/1 or 2.
    const w123config = vscode.workspace.getConfiguration("tclLsp.diagnostics");
    await w123config.update("W123", true, vscode.ConfigurationTarget.Global);
    try {
      await activate(uri);
      const codeOf = (d: vscode.Diagnostic) => (typeof d.code === "object" ? d.code.value : d.code);
      const w123On = (diags: vscode.Diagnostic[], line: number) =>
        diags.some((d) => codeOf(d) === "W123" && d.range.start.line === line);
      // Settle on the loaded-database state: the genuine unknown (line 2) is
      // flagged AND the library calls (lines 0-1) are not — the exact fixed shape.
      const diagnostics = await waitForDiagnostics(uri, {
        predicate: (diags) => w123On(diags, 2) && !w123On(diags, 0) && !w123On(diags, 1),
      });
      assert.ok(!w123On(diagnostics, 0), "Rbc_ActiveLegend (auto-loaded) must not be W123");
      assert.ok(!w123On(diagnostics, 1), "Rbc_ZoomStack (auto-loaded) must not be W123");
      assert.ok(
        w123On(diagnostics, 2),
        "the genuinely-unknown command must still be W123 (check stays live)",
      );
    } finally {
      await w123config.update("W123", undefined, vscode.ConfigurationTarget.Global);
    }
  });

  // Regression: the missing-open-brace recovery only excluded registry
  // builtins from looking like an orphaned switch case, so a genuine call to
  // an already-declared user proc with a single braced argument right after
  // the case list — `renderReport { prose text }` — was swallowed as an
  // extra case, corrupting the switch's argv and running the braced prose
  // through command analysis as if it were Tcl (a phantom "Unknown command").
  test("E101 recovery does not swallow a call to a known proc", async () => {
    const uri = getDocUri("diagnostics-e101-known-proc.tcl");
    await activate(uri);
    const codeOf = (d: vscode.Diagnostic) => (typeof d.code === "object" ? d.code.value : d.code);
    const diagnostics = await waitForDiagnostics(uri, {
      predicate: (diags) => diags.some((d) => codeOf(d) === "E101"),
    });
    const codes = diagnostics.map(codeOf);
    assert.ok(codes.includes("E101"), `expected E101, got [${codes}]`);
    assert.ok(
      !codes.includes("W123"),
      `the renderReport call must not be parsed as switch-case body text: [${codes}]`,
    );
  });

  // Regression: the "stolen close brace" heuristic used to fire on whichever
  // `}` was LAST in the swallowed text, even when that text spanned more
  // than one top-level statement (here a sibling `proc` swallowed along with
  // the `if` that actually stole the brace). Applying that fix parsed clean
  // but silently nested the sibling proc inside the unclosed one instead of
  // closing it where the missing brace belongs. Pure brace-counting can't
  // safely pick a location once more than one statement is swallowed, so
  // this must fall back to the generic (fix-less) E200 instead of guessing.
  test("E103 abstains when the missing brace swallows more than one statement", async () => {
    const uri = getDocUri("diagnostics-e103-multi-statement.tcl");
    await activate(uri);
    const codeOf = (d: vscode.Diagnostic) => (typeof d.code === "object" ? d.code.value : d.code);
    const diagnostics = await waitForDiagnostics(uri, {
      predicate: (diags) => diags.some((d) => codeOf(d) === "E200" || codeOf(d) === "E103"),
    });
    const codes = diagnostics.map(codeOf);
    assert.ok(!codes.includes("E103"), `expected no E103, got [${codes}]`);
    assert.ok(codes.includes("E200"), `expected the generic E200, got [${codes}]`);
  });
});
