import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate } from "./helper";

/**
 * Marker-based completion probing: find ``# PROBE_X`` in the fixture,
 * insert the supplied probe text on the line after the marker, position
 * the cursor at the end of the inserted text, request completion, then
 * revert the in-memory edit so the next test sees the canonical fixture.
 */
async function probeAt(
  uri: vscode.Uri,
  marker: string,
  insertion: string,
): Promise<vscode.CompletionItem[]> {
  const doc = await activate(uri);
  const text = doc.getText();
  const idx = text.indexOf(marker);
  assert.ok(idx >= 0, `Marker '${marker}' not found in fixture`);
  // Insert at the end of the marker line so the probe sits on the
  // following line (matching the comment indentation).
  const markerLine = doc.positionAt(idx).line;
  const eolPos = doc.lineAt(markerLine).range.end;
  const eolOffset = doc.offsetAt(eolPos);
  const edit = new vscode.WorkspaceEdit();
  edit.insert(uri, eolPos, insertion);
  await vscode.workspace.applyEdit(edit);
  const completionPos = doc.positionAt(eolOffset + insertion.length);
  try {
    const result = (await vscode.commands.executeCommand(
      "vscode.executeCompletionItemProvider",
      uri,
      completionPos,
    )) as vscode.CompletionList;
    return result.items;
  } finally {
    await vscode.commands.executeCommand("workbench.action.files.revert", doc);
  }
}

function labelOf(item: vscode.CompletionItem): string {
  return typeof item.label === "string" ? item.label : item.label.label;
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

  // Each command-form probe inserts ``\n        puts $`` after the
  // marker line so the cursor lands right after a fresh ``$`` inside
  // the surrounding proc body.

  test("variable: namespace var alias is in scope inside the proc", async () => {
    const items = await probeAt(docUri, "# PROBE_VARIABLE", "\n        puts $");
    const labels = items.map(labelOf);
    assert.ok(
      labels.includes("$namespace_var"),
      `Expected $namespace_var: ${labels.slice(0, 30).join(", ")}`,
    );
  });

  test("namespace upvar: local alias is in scope", async () => {
    const items = await probeAt(docUri, "# PROBE_NS_UPVAR", "\n        puts $");
    const labels = items.map(labelOf);
    assert.ok(
      labels.includes("$local_ns_var"),
      `Expected $local_ns_var: ${labels.slice(0, 30).join(", ")}`,
    );
  });

  test("lassign: each destructured var is in scope", async () => {
    const items = await probeAt(docUri, "# PROBE_LASSIGN", "\n        puts $");
    const labels = items.map(labelOf);
    for (const v of ["$la", "$lb", "$lc"]) {
      assert.ok(
        labels.includes(v),
        `Expected ${v} in lassign body: ${labels.slice(0, 30).join(", ")}`,
      );
    }
  });

  test("regexp: capture-group vars are in scope", async () => {
    const items = await probeAt(docUri, "# PROBE_REGEXP", "\n        puts $");
    const labels = items.map(labelOf);
    for (const v of ["$rx_full", "$rx_key", "$rx_value"]) {
      assert.ok(
        labels.includes(v),
        `Expected ${v} after regexp: ${labels.slice(0, 30).join(", ")}`,
      );
    }
  });

  test("regsub: output var is in scope", async () => {
    const items = await probeAt(docUri, "# PROBE_REGSUB", "\n        puts $");
    const labels = items.map(labelOf);
    assert.ok(labels.includes("$rs_out"), `Expected $rs_out: ${labels.slice(0, 30).join(", ")}`);
  });

  test("scan: each output var is in scope", async () => {
    const items = await probeAt(docUri, "# PROBE_SCAN", "\n        puts $");
    const labels = items.map(labelOf);
    for (const v of ["$sx", "$sy", "$sz"]) {
      assert.ok(labels.includes(v), `Expected ${v} after scan: ${labels.slice(0, 30).join(", ")}`);
    }
  });

  test("catch: resultVar and optionsVar are in scope", async () => {
    const items = await probeAt(docUri, "# PROBE_CATCH", "\n        puts $");
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
    const items = await probeAt(docUri, "# PROBE_TRY", "\n            puts $");
    const labels = items.map(labelOf);
    for (const v of ["$try_msg", "$try_opts"]) {
      assert.ok(
        labels.includes(v),
        `Expected ${v} in try-on-error body: ${labels.slice(0, 30).join(", ")}`,
      );
    }
  });

  test("dict update: alias vars are in scope inside body", async () => {
    const items = await probeAt(docUri, "# PROBE_DICT_UPDATE", "\n            puts $");
    const labels = items.map(labelOf);
    for (const v of ["$alias_a", "$alias_b"]) {
      assert.ok(
        labels.includes(v),
        `Expected ${v} in dict-update body: ${labels.slice(0, 30).join(", ")}`,
      );
    }
  });

  test("uplevel 1: best-effort behaviour leaves caller proc's locals in scope", async () => {
    // ``uplevel 1`` body's effective scope is the runtime caller, which
    // we can't resolve statically.  We keep best-effort behaviour: the
    // calling proc's locals stay visible.  Just check that completion
    // works (returns at least one item) and includes the proc-local.
    const items = await probeAt(docUri, "# PROBE_UPLEVEL_ONE", "\n            puts $");
    const labels = items.map(labelOf);
    assert.ok(
      labels.includes("$up1_local"),
      `Expected $up1_local in uplevel 1 body: ${labels.slice(0, 30).join(", ")}`,
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
    const items = await probeAt(docUri, "# PROBE_PARTIAL", "\n        puts $");
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
    const items = await probeAt(docUri, "# PROBE_PARTIAL", "\n        puts $gre");
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
    const items = await probeAt(docUri, "# PROBE_PARTIAL", "\n        puts ${gre");
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
      const result = (await vscode.commands.executeCommand(
        "vscode.executeCompletionItemProvider",
        docUri,
        completionPos,
      )) as vscode.CompletionList;
      const item = result.items.find((i) => labelOf(i) === "$greeting");
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
      const result = (await vscode.commands.executeCommand(
        "vscode.executeCompletionItemProvider",
        docUri,
        completionPos,
      )) as vscode.CompletionList;
      const item = result.items.find((i) => labelOf(i) === "$greeting");
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
      const result = (await vscode.commands.executeCommand(
        "vscode.executeCompletionItemProvider",
        otherUri,
        completionPos,
      )) as vscode.CompletionList;
      const item = result.items.find((i) => labelOf(i) === "$foo-bar");
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
