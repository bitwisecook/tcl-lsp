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
});
