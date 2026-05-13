// Multi-folder workspace tests for issue #230.  Verifies VS Code accepts
// folder-level tclLsp.* settings (no "This setting cannot be applied in
// this workspace" warning) and that the language server applies them
// per-folder when formatting files in each folder.
import * as assert from "assert";
import * as path from "path";
import * as vscode from "vscode";

suite("Multi-folder workspace configuration (#230)", () => {
  suiteSetup(async () => {
    // Wait for the extension to activate so the LSP client is ready.
    const ext = vscode.extensions.getExtension("bitwisecook.tcl-lsp");
    assert.ok(ext, "tcl-lsp extension not found");
    if (!ext.isActive) {
      await ext.activate();
    }
    // The server pulls per-folder config asynchronously after
    // ``initialized``; give it a moment to settle so format requests
    // see the resolved per-folder formatter config.
    await new Promise((r) => setTimeout(r, 3000));
  });

  test("workspace is opened in multi-folder mode with two folders", () => {
    const folders = vscode.workspace.workspaceFolders;
    assert.ok(folders, "workspaceFolders is undefined");
    assert.strictEqual(folders.length, 2, "expected exactly 2 workspace folders");
    const names = folders.map((f) => f.name).sort();
    assert.deepStrictEqual(names, ["proj-a", "proj-b"]);
  });

  test("VS Code accepts folder-level tclLsp.formatting.maxLineLength (#230 reproduction)", () => {
    // The fixture's folder-level .vscode/settings.json sets:
    //   proj-a -> tclLsp.formatting.maxLineLength = 160
    //   proj-b -> tclLsp.formatting.maxLineLength = 60
    //
    // If the setting still had window scope (the original bug), VS Code
    // would silently ignore the folder-level value and getConfiguration
    // would return the default 120 for both folders.  The scope fix in
    // PR A makes folder-level configuration take effect.
    const folders = vscode.workspace.workspaceFolders!;
    const folderA = folders.find((f) => f.name === "proj-a")!;
    const folderB = folders.find((f) => f.name === "proj-b")!;

    const valueA = vscode.workspace
      .getConfiguration("tclLsp.formatting", folderA.uri)
      .get<number>("maxLineLength");
    const valueB = vscode.workspace
      .getConfiguration("tclLsp.formatting", folderB.uri)
      .get<number>("maxLineLength");

    assert.strictEqual(valueA, 160, "folder A should see its folder-level override");
    assert.strictEqual(valueB, 60, "folder B should see its folder-level override");
  });

  test("VS Code accepts folder-level tclLsp.diagnostics.W111 toggle", () => {
    const folderA = vscode.workspace.workspaceFolders!.find((f) => f.name === "proj-a")!;
    const folderB = vscode.workspace.workspaceFolders!.find((f) => f.name === "proj-b")!;

    const a = vscode.workspace
      .getConfiguration("tclLsp.diagnostics", folderA.uri)
      .get<boolean>("W111");
    const b = vscode.workspace
      .getConfiguration("tclLsp.diagnostics", folderB.uri)
      .get<boolean>("W111");

    // proj-a explicitly disabled W111; proj-b inherits the default (true).
    assert.strictEqual(a, false, "folder A should report W111 disabled");
    assert.strictEqual(b, true, "folder B should report W111 enabled (default)");
  });

  // Set known content into the document and format it.  Returns the
  // formatted text (post-edits applied) so callers can inspect the
  // result.  Mirrors the flow in src/test/formatting.test.ts.
  async function formatWithContent(folderName: string, content: string): Promise<string> {
    const folder = vscode.workspace.workspaceFolders!.find((f) => f.name === folderName)!;
    const fileUri = vscode.Uri.file(path.join(folder.uri.fsPath, "foo.tcl"));
    const doc = await vscode.workspace.openTextDocument(fileUri);
    const editor = await vscode.window.showTextDocument(doc);

    // Replace whatever's in the doc with the known content.
    await editor.edit((e) => {
      const lastLine = doc.lineCount - 1;
      const lastChar = doc.lineAt(lastLine).text.length;
      e.replace(
        new vscode.Range(new vscode.Position(0, 0), new vscode.Position(lastLine, lastChar)),
        content,
      );
    });

    // Lightweight LSP roundtrip so the server has analysed the doc.
    await vscode.commands.executeCommand(
      "vscode.executeHoverProvider",
      fileUri,
      new vscode.Position(0, 0),
    );

    const edits = await vscode.commands.executeCommand<vscode.TextEdit[]>(
      "vscode.executeFormatDocumentProvider",
      fileUri,
      { tabSize: 4, insertSpaces: true },
    );
    if (edits && edits.length > 0) {
      const wsEdit = new vscode.WorkspaceEdit();
      wsEdit.set(fileUri, edits);
      await vscode.workspace.applyEdit(wsEdit);
    }
    return doc.getText();
  }

  test("folder A and folder B produce different formatted output for identical source", async () => {
    // A long single-statement command line.  Folder A (160 col) should
    // keep it on one line; folder B (60 col) should wrap it across
    // multiple lines.  If the LSP server resolved per-folder formatter
    // configs correctly, the outputs differ.
    // Use a command call with many command-level args (wrappable at depth
    // 0).  Long enough that:
    //   folder A (160 col) wraps to a few lines around 160 chars,
    //   folder B (60 col)  wraps to many lines around 60 chars.
    // Different max widths => different wrap patterns => different line
    // counts in the output.
    const args = Array.from({ length: 30 }, (_, i) => `argument-number-${i + 1}`).join(" ");
    const source = `my_long_command_name ${args}\n`;

    const textA = await formatWithContent("proj-a", source);
    const textB = await formatWithContent("proj-b", source);

    // Source must have actually loaded — guards against silent empty docs.
    assert.ok(textA.length > 0 && textB.length > 0, "fixtures must have content");

    const linesA = textA.split("\n").filter((l) => l.length > 0).length;
    const linesB = textB.split("\n").filter((l) => l.length > 0).length;
    const longestA = Math.max(...textA.split("\n").map((l) => l.length));
    const longestB = Math.max(...textB.split("\n").map((l) => l.length));

    assert.notStrictEqual(
      textA,
      textB,
      `same source should format differently per folder.\n` +
        `A: lines=${linesA} longest=${longestA}\n` +
        `B: lines=${linesB} longest=${longestB}\n` +
        `--- A ---\n${textA}\n--- B ---\n${textB}`,
    );
    // Folder B (60 col) must wrap into more lines than folder A (160 col).
    assert.ok(
      linesB > linesA,
      `folder B (60-col) should wrap into more lines than folder A (160-col): A=${linesA}, B=${linesB}`,
    );
  });

  test("folder A formatter keeps long lines under 160 cols (not the 120 default)", async () => {
    // Construct a line >120 but <160 chars.  Default 120-col cap would
    // wrap it, but folder A's 160-col override should keep it intact.
    const filler = "x".repeat(120);
    const source = `set the_variable_name_here "${filler}"\n`;

    const text = await formatWithContent("proj-a", source);
    const longest = Math.max(...text.split("\n").map((l) => l.length));

    assert.ok(longest > 120, `expected a >120-char line in source, got longest=${longest}`);
    assert.ok(
      longest <= 160,
      `folder A formatter should respect 160-col limit (longest=${longest})`,
    );
  });

  // Per-folder dialect (issue #407) — VS Code must accept the
  // ``tclLsp.dialect`` setting at the folder scope and the language
  // server must apply it per-document rather than using a single
  // process-wide dialect.
  test("VS Code accepts folder-level tclLsp.dialect (#407 reproduction)", () => {
    const folderA = vscode.workspace.workspaceFolders!.find((f) => f.name === "proj-a")!;
    const folderB = vscode.workspace.workspaceFolders!.find((f) => f.name === "proj-b")!;

    const a = vscode.workspace.getConfiguration("tclLsp", folderA.uri).get<string>("dialect");
    const b = vscode.workspace.getConfiguration("tclLsp", folderB.uri).get<string>("dialect");

    // proj-a is tcl8.4, proj-b is f5-irules.  Before the per-folder
    // dialect refactor, only one of these could be honoured at a time
    // and the LSP server emitted a "Per-folder dialects are not yet
    // supported" warning for the loser.
    assert.strictEqual(a, "tcl8.4", "folder A should report dialect=tcl8.4");
    assert.strictEqual(b, "f5-irules", "folder B should report dialect=f5-irules");
  });

  test("VS Code accepts folder-level tclLsp.style.nonAscii", () => {
    const folderA = vscode.workspace.workspaceFolders!.find((f) => f.name === "proj-a")!;
    const folderB = vscode.workspace.workspaceFolders!.find((f) => f.name === "proj-b")!;

    const a = vscode.workspace
      .getConfiguration("tclLsp.style", folderA.uri)
      .get<string>("nonAscii");
    const b = vscode.workspace
      .getConfiguration("tclLsp.style", folderB.uri)
      .get<string>("nonAscii");

    assert.strictEqual(a, "strict");
    assert.strictEqual(b, "off");
  });

  test("folder A and folder B produce different diagnostics for {*}-expansion source (#407)", async () => {
    // proj-a is tcl8.4 — ``{*}`` is NOT recognised as the word-expansion
    // prefix, so ``cmd {*}$args`` lexes as a literal ``*$args`` word
    // (no E001/E007 — the source is well-formed under 8.4 rules).
    // proj-b is f5-irules — based on 8.4 syntax too, so it shares the
    // no-expansion behaviour but flags many vanilla Tcl commands as
    // unknown (iRules has a far smaller command surface).
    // The interesting cross-folder diff: open identical ``my-helper``
    // invocations — folder A accepts it as an unknown user command
    // (no E002 in tcl8.4 either, but tcl8.4 *does* flag iRules-only
    // commands).  Use ``when`` (iRules-only) which is a known command in
    // proj-b but unknown in proj-a, yielding divergent E002 counts.
    const source = `when CLIENT_ACCEPTED {\n    log local0. "hi"\n}\n`;

    async function diagsFor(folderName: string): Promise<vscode.Diagnostic[]> {
      const folder = vscode.workspace.workspaceFolders!.find((f) => f.name === folderName)!;
      const fileUri = vscode.Uri.file(path.join(folder.uri.fsPath, "foo.tcl"));
      const doc = await vscode.workspace.openTextDocument(fileUri);
      const editor = await vscode.window.showTextDocument(doc);
      await editor.edit((e) => {
        const lastLine = doc.lineCount - 1;
        const lastChar = doc.lineAt(lastLine).text.length;
        e.replace(
          new vscode.Range(new vscode.Position(0, 0), new vscode.Position(lastLine, lastChar)),
          source,
        );
      });
      // Wait for diagnostics to arrive; the server publishes them
      // asynchronously after the didChange roundtrip.
      for (let i = 0; i < 30; i++) {
        const diags = vscode.languages.getDiagnostics(fileUri);
        if (diags.length > 0) return diags;
        await new Promise((r) => setTimeout(r, 200));
      }
      return vscode.languages.getDiagnostics(fileUri);
    }

    const diagsA = await diagsFor("proj-a");
    const diagsB = await diagsFor("proj-b");

    // ``when`` is unknown under tcl8.4 (proj-a) so we expect at least
    // one E002 (Unknown command); it's a registered iRules command
    // under f5-irules (proj-b) so there should be none for ``when``
    // itself.  The exact diagnostic count is sensitive to many other
    // checks, so just assert the presence/absence of E002 on the ``when``
    // command name.
    const whenE002InA = diagsA.some(
      (d) => d.code === "E002" && d.message.toLowerCase().includes("when"),
    );
    const whenE002InB = diagsB.some(
      (d) => d.code === "E002" && d.message.toLowerCase().includes("when"),
    );
    assert.strictEqual(
      whenE002InA,
      true,
      `folder A (tcl8.4) should flag 'when' as unknown.  Diags: ${JSON.stringify(diagsA.map((d) => [d.code, d.message]))}`,
    );
    assert.strictEqual(
      whenE002InB,
      false,
      `folder B (f5-irules) should accept 'when' as a known command.  Diags: ${JSON.stringify(diagsB.map((d) => [d.code, d.message]))}`,
    );
  });
});
