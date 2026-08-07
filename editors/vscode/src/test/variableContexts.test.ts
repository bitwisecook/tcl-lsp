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
  waitForDeepDiagnostics,
  getServerLogSize,
  pollUntil,
  scaledTimeout,
} from "./helper";

function labelOf(item: vscode.CompletionItem): string {
  return typeof item.label === "string" ? item.label : item.label.label;
}

/**
 * Request completion at *position* and poll until every label in *expect*
 * is present, or a deadline passes.
 *
 * ``applyEdit`` returns as soon as VS Code has buffered the change for the
 * language server; it does **not** wait for the server to re-analyse the
 * mutated document.  A completion request fired immediately afterwards can
 * therefore race the re-index and return stale results that omit a
 * newly-in-scope variable — the source of the intermittent failures these
 * probes used to exhibit under parallel ``make test-slow`` load.
 *
 * Rather than sleep for a fixed interval, we re-request completion on a
 * bounded poll (via the shared ``pollUntil`` helper) until the server has
 * caught up and surfaces the expected items.  ``pollUntil`` throws on
 * timeout with the last-seen item list, so a genuine regression still
 * fails loudly — the wait only absorbs the re-index latency, it does not
 * mask a missing variable.
 */
async function completionItemsAt(
  uri: vscode.Uri,
  position: vscode.Position,
  expect: string[],
): Promise<vscode.CompletionItem[]> {
  return pollUntil(
    async () => {
      const result = (await vscode.commands.executeCommand(
        "vscode.executeCompletionItemProvider",
        uri,
        position,
      )) as vscode.CompletionList;
      return result.items;
    },
    (items) => {
      const labels = new Set(items.map(labelOf));
      return expect.every((label) => labels.has(label));
    },
    {
      timeout: 10_000,
      label: `completion items [${expect.join(", ")}] at ${position.line}:${position.character}`,
    },
  );
}

/**
 * Insert *insertion* at the end of the line containing *marker* (so the
 * probe sits on the line after the marker, matching the fixture's
 * indentation), and return the position right after the inserted text —
 * the spot completion should be requested at. Shared by ``probeAt`` (one
 * probe, revert immediately) and the "command contexts" suite below
 * (all ten probes inserted up front, reverted once at the end).
 */
async function insertProbe(
  uri: vscode.Uri,
  doc: vscode.TextDocument,
  marker: string,
  insertion: string,
): Promise<vscode.Position> {
  const text = doc.getText();
  const idx = text.indexOf(marker);
  assert.ok(idx >= 0, `Marker '${marker}' not found in fixture`);
  const markerLine = doc.positionAt(idx).line;
  const eolPos = doc.lineAt(markerLine).range.end;
  const eolOffset = doc.offsetAt(eolPos);
  const edit = new vscode.WorkspaceEdit();
  edit.insert(uri, eolPos, insertion);
  await vscode.workspace.applyEdit(edit);
  return doc.positionAt(eolOffset + insertion.length);
}

/**
 * Marker-based completion probing: find ``# PROBE_X`` in the fixture,
 * insert the supplied probe text on the line after the marker, position
 * the cursor at the end of the inserted text, request completion (waiting
 * for the server to re-index — see ``completionItemsAt``), then revert the
 * in-memory edit so the next test sees the canonical fixture.
 *
 * ``expect`` lists the labels the caller is about to assert on; the probe
 * does not return until they are all present or the deadline passes, which
 * is what makes it deterministic under load.
 */
async function probeAt(
  uri: vscode.Uri,
  marker: string,
  insertion: string,
  expect: string[],
): Promise<vscode.CompletionItem[]> {
  const doc = await activate(uri);
  const completionPos = await insertProbe(uri, doc, marker, insertion);
  try {
    return await completionItemsAt(uri, completionPos, expect);
  } finally {
    await vscode.commands.executeCommand("workbench.action.files.revert", doc);
  }
}

/** One completion request at *position*, no polling. Only safe to call
 * against an already-settled document — see the "command contexts" suite's
 * ``suiteSetup``, which waits for the deep-diagnostics signal before any
 * test calls this. */
async function completionAt(
  uri: vscode.Uri,
  position: vscode.Position,
): Promise<vscode.CompletionItem[]> {
  const result = (await vscode.commands.executeCommand(
    "vscode.executeCompletionItemProvider",
    uri,
    position,
  )) as vscode.CompletionList;
  return result.items;
}

function insertedText(item: vscode.CompletionItem): string | undefined {
  if (typeof item.insertText === "string") return item.insertText;
  if (item.insertText) return item.insertText.value;
  return undefined;
}

function textEditOf(item: vscode.CompletionItem): vscode.TextEdit | undefined {
  // VS Code's CompletionItem.textEdit may be a TextEdit, an
  // InsertReplaceEdit, or undefined.  We only care about the simple
  // TextEdit form here.
  const te = (item as { textEdit?: unknown }).textEdit;
  if (te instanceof vscode.TextEdit) return te;
  return undefined;
}

suite("Variable Completion: command contexts", () => {
  const docUri = getDocUri("variableContexts.tcl");

  // Each command-form probe inserts ``\n        puts $`` (or, for the three
  // forms whose marker sits one indent level deeper, ``\n            puts
  // $``) after the marker line so the cursor lands right after a fresh
  // ``$`` inside the surrounding proc body.
  //
  // These ten probes used to each insert their own probe text and
  // independently poll ``completionItemsAt`` on a fresh 10s budget before
  // reverting — ten such polls stacked on top of Electron startup produced
  // the intermittent cumulative timeout under constrained containers
  // (issue #1059), even though any single probe settles in well under a
  // second once the server has caught up. Every probe marker sits in its
  // own, syntactically independent proc (see
  // ``testFixture/variableContexts.tcl``), so all ten insertions can
  // coexist in one edit batch: ``suiteSetup`` applies them all up front and
  // waits ONCE for the server's ``[timing] deep diagnostics`` marker for
  // this document (``waitForDeepDiagnostics``) — a real "the server
  // finished re-analysing the edited document" signal (the same one
  // ``helper.ts`` uses elsewhere), not a sleep. Every test below then
  // requests completion exactly once against that already-settled state,
  // so a genuine regression in any one probe still fails loudly on its own
  // assertion — the batching only removes the redundant repeated waiting,
  // it does not merge the ten checks into one.
  const PROBES = [
    { key: "variable", marker: "# PROBE_VARIABLE", insertion: "\n        puts $" },
    { key: "nsUpvar", marker: "# PROBE_NS_UPVAR", insertion: "\n        puts $" },
    { key: "lassign", marker: "# PROBE_LASSIGN", insertion: "\n        puts $" },
    { key: "regexp", marker: "# PROBE_REGEXP", insertion: "\n        puts $" },
    { key: "regsub", marker: "# PROBE_REGSUB", insertion: "\n        puts $" },
    { key: "scan", marker: "# PROBE_SCAN", insertion: "\n        puts $" },
    { key: "catch", marker: "# PROBE_CATCH", insertion: "\n        puts $" },
    { key: "tryOnError", marker: "# PROBE_TRY", insertion: "\n            puts $" },
    { key: "dictUpdate", marker: "# PROBE_DICT_UPDATE", insertion: "\n            puts $" },
    { key: "uplevelOne", marker: "# PROBE_UPLEVEL_ONE", insertion: "\n            puts $" },
  ] as const;
  const positions = new Map<string, vscode.Position>();
  let doc: vscode.TextDocument;

  function positionFor(key: (typeof PROBES)[number]["key"]): vscode.Position {
    const pos = positions.get(key);
    assert.ok(pos, `no probe position recorded for '${key}' — did suiteSetup run?`);
    return pos as vscode.Position;
  }

  suiteSetup(async function () {
    this.timeout(scaledTimeout(30_000));
    doc = await activate(docUri);
    for (let i = 0; i < PROBES.length; i++) {
      const p = PROBES[i];
      const isLast = i === PROBES.length - 1;
      // Capture the log-size baseline right before the LAST edit only: it
      // must be taken after every earlier edit (so an intermediate deep
      // pass that happens to land between two of our edits does not
      // satisfy the wait prematurely) but before this edit is applied (so
      // the wait cannot miss a publish that lands between capturing the
      // baseline and awaiting it).
      const since = isLast ? getServerLogSize() : undefined;
      positions.set(p.key, await insertProbe(docUri, doc, p.marker, p.insertion));
      if (isLast) {
        await waitForDeepDiagnostics(docUri, { since, timeout: 20_000 });
      }
    }
  });

  suiteTeardown(async () => {
    await vscode.commands.executeCommand("workbench.action.files.revert", doc);
  });

  test("variable: namespace var alias is in scope inside the proc", async () => {
    const items = await completionAt(docUri, positionFor("variable"));
    const labels = items.map(labelOf);
    assert.ok(
      labels.includes("$namespace_var"),
      `Expected $namespace_var: ${labels.slice(0, 30).join(", ")}`,
    );
  });

  test("namespace upvar: local alias is in scope", async () => {
    const items = await completionAt(docUri, positionFor("nsUpvar"));
    const labels = items.map(labelOf);
    assert.ok(
      labels.includes("$local_ns_var"),
      `Expected $local_ns_var: ${labels.slice(0, 30).join(", ")}`,
    );
  });

  test("lassign: each destructured var is in scope", async () => {
    const items = await completionAt(docUri, positionFor("lassign"));
    const labels = items.map(labelOf);
    for (const v of ["$la", "$lb", "$lc"]) {
      assert.ok(
        labels.includes(v),
        `Expected ${v} in lassign body: ${labels.slice(0, 30).join(", ")}`,
      );
    }
  });

  test("regexp: capture-group vars are in scope", async () => {
    const items = await completionAt(docUri, positionFor("regexp"));
    const labels = items.map(labelOf);
    for (const v of ["$rx_full", "$rx_key", "$rx_value"]) {
      assert.ok(
        labels.includes(v),
        `Expected ${v} after regexp: ${labels.slice(0, 30).join(", ")}`,
      );
    }
  });

  test("regsub: output var is in scope", async () => {
    const items = await completionAt(docUri, positionFor("regsub"));
    const labels = items.map(labelOf);
    assert.ok(labels.includes("$rs_out"), `Expected $rs_out: ${labels.slice(0, 30).join(", ")}`);
  });

  test("scan: each output var is in scope", async () => {
    const items = await completionAt(docUri, positionFor("scan"));
    const labels = items.map(labelOf);
    for (const v of ["$sx", "$sy", "$sz"]) {
      assert.ok(labels.includes(v), `Expected ${v} after scan: ${labels.slice(0, 30).join(", ")}`);
    }
  });

  test("catch: resultVar and optionsVar are in scope", async () => {
    const items = await completionAt(docUri, positionFor("catch"));
    const labels = items.map(labelOf);
    assert.ok(
      labels.includes("$catch_result"),
      `Expected $catch_result: ${labels.slice(0, 30).join(", ")}`,
    );
    assert.ok(
      labels.includes("$catch_opts"),
      `Expected $catch_opts: ${labels.slice(0, 30).join(", ")}`,
    );
  });

  test("try on error: handler binding-list vars are in scope inside body", async () => {
    const items = await completionAt(docUri, positionFor("tryOnError"));
    const labels = items.map(labelOf);
    for (const v of ["$try_msg", "$try_opts"]) {
      assert.ok(
        labels.includes(v),
        `Expected ${v} in try-on-error body: ${labels.slice(0, 30).join(", ")}`,
      );
    }
  });

  test("dict update: alias vars are in scope inside body", async () => {
    const items = await completionAt(docUri, positionFor("dictUpdate"));
    const labels = items.map(labelOf);
    for (const v of ["$alias_a", "$alias_b"]) {
      assert.ok(
        labels.includes(v),
        `Expected ${v} in dict-update body: ${labels.slice(0, 30).join(", ")}`,
      );
    }
  });

  test("uplevel 1 abstains: the caller proc's locals are not offered", async () => {
    // ``uplevel 1`` runs its body in the dynamic caller's frame, so the
    // enclosing proc's ``up1_local`` is not in scope there — offering it would
    // suggest a variable that does not exist at runtime. Completion abstains on
    // the proc's locals (matching the server-side
    // ``dollar_completion_uplevel_one_abstains_from_proc_scope``) while still
    // offering the body's own declarations.
    const items = await completionAt(docUri, positionFor("uplevelOne"));
    const labels = items.map(labelOf);
    assert.ok(
      labels.includes("$body_var"),
      `the uplevel body's own var should be offered: ${labels.slice(0, 30).join(", ")}`,
    );
    assert.ok(
      !labels.includes("$up1_local"),
      `$up1_local must NOT be offered inside an uplevel 1 body: ${labels.slice(0, 30).join(", ")}`,
    );
  });
});

suite("Variable Completion: TextEdit shape", () => {
  const docUri = getDocUri("variableContexts.tcl");

  test("issue #357 -- bare $ insertion does not delete the typed dollar", async () => {
    // Probe inside ``test_partials {greeting}`` proc -- ``$greeting``
    // is in scope.  Type a single ``$`` and check that selecting the
    // ``$greeting`` completion produces a TextEdit whose range starts
    // at the typed ``$``, and whose new text starts with ``$``.
    const items = await probeAt(docUri, "# PROBE_PARTIAL", "\n        puts $", ["$greeting"]);
    const item = items.find((i) => labelOf(i) === "$greeting");
    assert.ok(item, "Expected $greeting in completion list");
    const te = textEditOf(item);
    assert.ok(te, "Expected TextEdit on $greeting completion item");
    assert.ok(
      te.newText.startsWith("$"),
      `TextEdit new text must keep the leading $: ${te.newText}`,
    );
    assert.strictEqual(te.newText, "$greeting");
    // Range must cover the ``$`` so applying the edit replaces it
    // (rather than inserting at cursor and leaving a stranded $).
    assert.strictEqual(
      te.range.start.character,
      te.range.end.character - 1,
      `Range should cover exactly the '$' (col ${te.range.start.character}..${te.range.end.character})`,
    );
  });

  test("$gre partial -- TextEdit replaces the partial with $greeting", async () => {
    const items = await probeAt(docUri, "# PROBE_PARTIAL", "\n        puts $gre", ["$greeting"]);
    const item = items.find((i) => labelOf(i) === "$greeting");
    assert.ok(item, "Expected $greeting in completion list");
    const te = textEditOf(item);
    assert.ok(te, "Expected TextEdit on $greeting completion item");
    assert.strictEqual(te.newText, "$greeting");
    // Range should cover ``$gre`` (4 cols).
    assert.strictEqual(
      te.range.end.character - te.range.start.character,
      4,
      `Range should span 4 chars covering '$gre': got ${te.range.end.character - te.range.start.character}`,
    );
  });

  test("${gre partial -- TextEdit emits ${greeting} with closing brace", async () => {
    const items = await probeAt(docUri, "# PROBE_PARTIAL", "\n        puts ${gre", ["$greeting"]);
    const item = items.find((i) => labelOf(i) === "$greeting");
    assert.ok(item, "Expected $greeting in completion list");
    const te = textEditOf(item);
    assert.ok(te, "Expected TextEdit on $greeting completion item");
    assert.strictEqual(te.newText, "${greeting}");
  });

  test("${gre|} -- TextEdit consumes the existing close brace, no double }", async () => {
    // Insert ``puts ${gre}`` and place cursor between ``e`` and ``}``.
    const doc = await activate(docUri);
    const text = doc.getText();
    const markerIdx = text.indexOf("# PROBE_PARTIAL");
    assert.ok(markerIdx >= 0);
    const markerLine = doc.positionAt(markerIdx).line;
    const eolPos = doc.lineAt(markerLine).range.end;
    const eolOffset = doc.offsetAt(eolPos);
    const insertion = "\n        puts ${gre}";
    const edit = new vscode.WorkspaceEdit();
    edit.insert(docUri, eolPos, insertion);
    await vscode.workspace.applyEdit(edit);
    // Cursor between ``e`` and ``}`` -- one char before the end of insertion.
    const completionPos = doc.positionAt(eolOffset + insertion.length - 1);
    try {
      const items = await completionItemsAt(docUri, completionPos, ["$greeting"]);
      const item = items.find((i) => labelOf(i) === "$greeting");
      assert.ok(item, "Expected $greeting");
      const te = textEditOf(item);
      assert.ok(te, "Expected TextEdit");
      assert.strictEqual(te.newText, "${greeting}");
      // Range must cover the existing ``}`` (so replacement = original).
      assert.ok(
        te.range.end.character > completionPos.character,
        `Range end (${te.range.end.character}) must cover the trailing '}' past the cursor (${completionPos.character})`,
      );
    } finally {
      await vscode.commands.executeCommand("workbench.action.files.revert", doc);
    }
  });

  test("midword $gre|eting -- TextEdit replaces the whole token", async () => {
    // Insert ``puts $greeting`` and place cursor between ``gre`` and
    // ``eting`` (after 4 chars of the var name including ``$``).
    const doc = await activate(docUri);
    const text = doc.getText();
    const markerIdx = text.indexOf("# PROBE_PARTIAL");
    const markerLine = doc.positionAt(markerIdx).line;
    const eolPos = doc.lineAt(markerLine).range.end;
    const eolOffset = doc.offsetAt(eolPos);
    const insertion = "\n        puts $greeting";
    const edit = new vscode.WorkspaceEdit();
    edit.insert(docUri, eolPos, insertion);
    await vscode.workspace.applyEdit(edit);
    // Cursor 9 chars from start of insertion (after newline + 8 spaces +
    // ``puts $gre`` -> position the cursor mid-token at offset 18 from
    // the newline; ie. eolOffset + 1 newline + 8 spaces + ``puts $gre``
    // = eolOffset + 18).
    const cursorOffset = eolOffset + 1 + 8 + 6 + 4; // \n + spaces + "puts $" + "gre"
    const completionPos = doc.positionAt(cursorOffset);
    try {
      const items = await completionItemsAt(docUri, completionPos, ["$greeting"]);
      const item = items.find((i) => labelOf(i) === "$greeting");
      assert.ok(item, "Expected $greeting in completion list");
      const te = textEditOf(item);
      assert.ok(te, "Expected TextEdit");
      assert.strictEqual(te.newText, "$greeting");
      // Range should extend past cursor to include the ``eting`` suffix
      // (so replacement covers the whole token, not just up to cursor).
      assert.ok(
        te.range.end.character > completionPos.character,
        `Range end (${te.range.end.character}) must extend past cursor (${completionPos.character}) to cover suffix`,
      );
    } finally {
      await vscode.commands.executeCommand("workbench.action.files.revert", doc);
    }
  });

  test("brace-form auto-promote: hyphenated name inserts ${foo-bar}", async () => {
    // ``foo-bar`` is in scope at the global level (set in
    // testFixture/variableCompletion.tcl).  We use that fixture so the
    // hyphenated var is visible.
    const otherUri = getDocUri("variableCompletion.tcl");
    const doc = await activate(otherUri);
    const text = doc.getText();
    // Insert a probe at the very end of the file -- valid Tcl since
    // we're at the top level (not inside a brace block).
    const eolOffset = text.length;
    const eolPos = doc.positionAt(eolOffset);
    const insertion = "\nputs $";
    const edit = new vscode.WorkspaceEdit();
    edit.insert(otherUri, eolPos, insertion);
    await vscode.workspace.applyEdit(edit);
    const completionPos = doc.positionAt(eolOffset + insertion.length);
    try {
      const items = await completionItemsAt(otherUri, completionPos, ["$foo-bar"]);
      const item = items.find((i) => labelOf(i) === "$foo-bar");
      assert.ok(item, "Expected $foo-bar in completion list");
      assert.strictEqual(
        insertedText(item),
        "${foo-bar}",
        `Expected '${"${foo-bar}"}' for hyphenated name, got '${insertedText(item)}'`,
      );
    } finally {
      await vscode.commands.executeCommand("workbench.action.files.revert", doc);
    }
  });
});

suite("Variable Completion: W215 unreachable-name diagnostic", () => {
  const docUri = getDocUri("variableContexts.tcl");

  test("W215 fires on a variable name that contains '}'", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 1 });
    const w215 = diagnostics.filter((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "W215";
    });
    assert.ok(w215.length >= 2, `Expected >=2 W215 diagnostics, got ${w215.length}`);
    // One W215 should mention the var-name (with ``}``), the other
    // should mention the array-element-index case (with ``)``).
    const messages = w215.map((d) => d.message);
    assert.ok(
      messages.some((m) => m.includes("not reachable") && m.includes("weird}name")),
      `Expected a var-name W215 message, got: ${messages.join(" / ")}`,
    );
    assert.ok(
      messages.some((m) => m.includes("array element index contains ')'")),
      `Expected an array-index W215 message, got: ${messages.join(" / ")}`,
    );
  });

  test("W216 fires on `${arr}(foo)` (parsed as scalar+literal)", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 1 });
    const w216 = diagnostics.filter((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "W216";
    });
    assert.ok(w216.length >= 2, `Expected >=2 W216 diagnostics, got ${w216.length}`);
    const messages = w216.map((d) => d.message);
    assert.ok(
      messages.some((m) => m.includes("scalar") && m.includes("literal")),
      `Expected the ${"${arr}(foo)"} message, got: ${messages.join(" / ")}`,
    );
    assert.ok(
      messages.some((m) => m.includes("does not substitute")),
      `Expected the ${"${arr($foo)}"} message, got: ${messages.join(" / ")}`,
    );
  });

  test("W216 quick fix offers the bare-form replacement", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 1 });
    // Pick the ``${arr}(name)`` (pattern 1) diagnostic specifically.
    const w216 = diagnostics.find((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "W216" && d.message.includes("scalar");
    });
    assert.ok(w216, "Expected a ${arr}(foo) W216 diagnostic in fixture");
    const matchesW216Fix = (a: vscode.CodeAction) =>
      a.kind !== undefined &&
      a.kind.value === vscode.CodeActionKind.QuickFix.value &&
      ((typeof a.title === "string" && a.title.includes("$arr(name)")) ||
        (a.edit !== undefined &&
          [...a.edit.entries()].some(([, edits]) =>
            edits.some((e) => e.newText === "$arr(name)"),
          )));
    const actions = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeCodeActionProvider", docUri, w216.range),
      (r) => Array.isArray(r) && (r as vscode.CodeAction[]).some(matchesW216Fix),
      { timeout: 10_000, label: "W216 bare-form quick fix" },
    )) as vscode.CodeAction[];
    const w216Fix = actions.find(matchesW216Fix);
    assert.ok(
      w216Fix,
      `Expected a W216 quick-fix offering $arr(name); got titles: ${actions.map((a) => a.title).join(" / ")}`,
    );
  });

  test('W216 falls back to [set "..."] when the array name needs braces', async () => {
    // ``funny name`` has a space, so bare ``$funny name(...)`` would
    // tokenise as scalar ``$funny`` followed by literal text.  The
    // quick fix uses the ``[set "name(idx)"]`` indirection form which
    // works for any name, with index substitution preserved.
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 1 });
    const w216 = diagnostics.find((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "W216" && d.message.includes("funny name");
    });
    assert.ok(w216, "Expected a W216 diagnostic for the funny-name fixture line");
    const expected = '[set "funny name($foo)"]';
    const matchesFix = (a: vscode.CodeAction) =>
      a.kind !== undefined &&
      a.kind.value === vscode.CodeActionKind.QuickFix.value &&
      ((typeof a.title === "string" && a.title.includes(expected)) ||
        (a.edit !== undefined &&
          [...a.edit.entries()].some(([, edits]) => edits.some((e) => e.newText === expected))));
    const actions = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeCodeActionProvider", docUri, w216.range),
      (r) => Array.isArray(r) && (r as vscode.CodeAction[]).some(matchesFix),
      { timeout: 10_000, label: "W216 [set ...] fallback quick fix" },
    )) as vscode.CodeAction[];
    const fix = actions.find(matchesFix);
    assert.ok(
      fix,
      `Expected a W216 quick-fix offering ${expected}; got titles: ${actions
        .map((a) => a.title)
        .join(" / ")}`,
    );
  });

  test("W215 has Warning severity", async () => {
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, { minCount: 1 });
    const w215 = diagnostics.find((d) => {
      const code = typeof d.code === "object" ? d.code.value : d.code;
      return code === "W215";
    });
    assert.ok(w215, "W215 diagnostic not found");
    assert.strictEqual(
      w215.severity,
      vscode.DiagnosticSeverity.Warning,
      "W215 should have Warning severity",
    );
  });
});
