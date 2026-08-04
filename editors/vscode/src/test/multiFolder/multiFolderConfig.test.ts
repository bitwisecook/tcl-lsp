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

// Multi-folder workspace tests for issue #230.  Verifies VS Code accepts
// folder-level tclLsp.* settings (no "This setting cannot be applied in
// this workspace" warning) and that the language server applies them
// per-folder when formatting files in each folder.
import * as assert from "assert";
import * as path from "path";
import * as vscode from "vscode";

interface EffectiveConfig {
  uri: string;
  folder_uri: string | null;
  // Resolved values: folder override → workspace fallback → process default.
  // ``dialect`` always concrete (e.g. "tcl8.6"), never null.
  // ``extra_commands`` / ``library_paths`` always arrays (empty when unset).
  dialect: string;
  extra_commands: string[];
  non_ascii_mode: string | null;
  library_paths: string[];
  line_length: number;
  dialect_explicitly_set: boolean;
  known_folder_uris: string[];
}

/**
 * Wait until the server reports that ``workspace/configuration`` has been
 * pulled and applied for every folder, by polling the
 * ``tcl-lsp.getEffectiveConfig`` command until each folder's resolved
 * dialect matches the value the fixture's ``.vscode/settings.json``
 * configured.  Replaces a brittle ``setTimeout(3000)`` that masked the
 * race issue #407 caught in real-world use.
 */
async function waitForConfigSettled(
  expected: Record<string, string>,
  timeoutMs: number = 15000,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  const last: Record<string, EffectiveConfig | null> = {};
  while (Date.now() < deadline) {
    let allMatch = true;
    for (const [folderName, wantDialect] of Object.entries(expected)) {
      const folder = vscode.workspace.workspaceFolders!.find((f) => f.name === folderName)!;
      const cfg = (await vscode.commands.executeCommand(
        "tcl-lsp.getEffectiveConfig",
        folder.uri.toString(),
      )) as EffectiveConfig | undefined;
      last[folderName] = cfg ?? null;
      if (!cfg || cfg.dialect !== wantDialect) {
        allMatch = false;
        break;
      }
    }
    if (allMatch) return;
    await new Promise((r) => setTimeout(r, 75));
  }
  throw new Error(
    `Timed out waiting for per-folder config to settle after ${timeoutMs}ms.  ` +
      `Expected: ${JSON.stringify(expected)}.  Last seen: ${JSON.stringify(last)}`,
  );
}

suite("Multi-folder workspace configuration (#230)", () => {
  suiteSetup(async () => {
    // Wait for the extension to activate so the LSP client is ready.
    const ext = vscode.extensions.getExtension("bitwisecook.tcl-lsp");
    assert.ok(ext, "tcl-lsp extension not found");
    if (!ext.isActive) {
      await ext.activate();
    }
    // Poll the server for its resolved per-folder dialect rather than
    // sleeping on wall-clock time -- the previous setTimeout(3000) masked
    // the race that issue #407 actually reports.
    await waitForConfigSettled({ "proj-a": "tcl8.4", "proj-b": "f5-irules" });
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

  test("folder A and folder B produce different diagnostics for identical source (#407)", async () => {
    // The source must carry **no** in-file dialect signal.  The server's
    // resolution order is: explicit ``languageId`` > in-source detection
    // (``# tcl-dialect:`` directive, shebang, ``package require Tcl``, content
    // signatures) > per-folder override > session default (issue #805).  A
    // body of ``when EVENT { … }`` handlers *is* an iRules content signature,
    // so such a source resolves to ``f5-irules`` in **both** folders and proves
    // nothing about the folder override.
    //
    // ``puts`` is the discriminator instead: a core Tcl command in every Tcl
    // dialect, and absent from the (far smaller) iRules command surface.  So
    // identical source is silent in proj-a (tcl8.4) and raises W123 in proj-b
    // (f5-irules).
    const source = `proc greet {who} {\n    puts "hi $who"\n}\ngreet world\n`;

    async function unknownPutsIn(folderName: string, expected: boolean): Promise<boolean> {
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
      // Poll for the *expected* state, not merely "some diagnostics exist" —
      // this file carries the previous test's diagnostics until the server
      // republishes, so a bare length check reads the stale set.
      const has = () =>
        vscode.languages
          .getDiagnostics(fileUri)
          .some((d) => d.code === "W123" && d.message.includes("puts"));
      for (let i = 0; i < 30 && has() !== expected; i++) {
        await new Promise((r) => setTimeout(r, 200));
      }
      return has();
    }

    assert.strictEqual(
      await unknownPutsIn("proj-a", false),
      false,
      "folder A (tcl8.4) knows 'puts' and must not flag it",
    );
    assert.strictEqual(
      await unknownPutsIn("proj-b", true),
      true,
      "folder B (f5-irules) has no 'puts' and must flag it",
    );
  });

  test("VS Code accepts folder-level tclLsp.extraCommands (#407)", () => {
    const folderA = vscode.workspace.workspaceFolders!.find((f) => f.name === "proj-a")!;
    const folderB = vscode.workspace.workspaceFolders!.find((f) => f.name === "proj-b")!;

    const a = vscode.workspace
      .getConfiguration("tclLsp", folderA.uri)
      .get<string[]>("extraCommands");
    const b = vscode.workspace
      .getConfiguration("tclLsp", folderB.uri)
      .get<string[]>("extraCommands");

    assert.deepStrictEqual(
      a,
      ["folder-a-helper", "shared-util"],
      "folder A should report its own extraCommands list",
    );
    assert.deepStrictEqual(
      b,
      ["folder-b-helper"],
      "folder B should report its own extraCommands list",
    );
  });

  test("VS Code accepts folder-level tclLsp.libraryPaths (#407)", () => {
    const folderA = vscode.workspace.workspaceFolders!.find((f) => f.name === "proj-a")!;
    const folderB = vscode.workspace.workspaceFolders!.find((f) => f.name === "proj-b")!;

    const a = vscode.workspace
      .getConfiguration("tclLsp", folderA.uri)
      .get<string[]>("libraryPaths");
    const b = vscode.workspace
      .getConfiguration("tclLsp", folderB.uri)
      .get<string[]>("libraryPaths");

    assert.deepStrictEqual(a, ["/opt/proj-a/tcl-lib"]);
    assert.deepStrictEqual(b, ["/opt/proj-b/tcl-lib", "/usr/local/share/tcl"]);
  });

  test("folder A and folder B differ on W108 non-ASCII diagnostics (#407)", async () => {
    // proj-a configures ``style.nonAscii=strict`` — every non-ASCII
    // character is flagged.
    // proj-b configures ``style.nonAscii=off`` — W108 is disabled.
    // Identical source: a single smart-quote literal that "strict" flags
    // and "off" ignores.  The dialect mode the W108 check reads from must
    // be scoped to the folder so the same source produces different
    // diagnostics in each folder.
    const source = `set greeting "“hello”"\n`;

    async function w108DiagsFor(
      folderName: string,
      wantW108: boolean,
    ): Promise<vscode.Diagnostic[]> {
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
      // Poll for the *expected* W108 state (up to 6s), not merely "some
      // diagnostics exist": this file still carries the previous test's
      // diagnostics until the server republishes, so a bare length check
      // returns the stale set immediately.
      const w108 = () => vscode.languages.getDiagnostics(fileUri).filter((d) => d.code === "W108");
      for (let i = 0; i < 30 && w108().length > 0 !== wantW108; i++) {
        await new Promise((r) => setTimeout(r, 200));
      }
      return w108();
    }

    const w108A = await w108DiagsFor("proj-a", true);
    const w108B = await w108DiagsFor("proj-b", false);

    // strict mode in folder A → at least one W108 for each smart quote.
    assert.ok(
      w108A.length >= 1,
      `folder A (strict) should flag the smart quote.  Got ${w108A.length} W108 diags.`,
    );
    // "off" in folder B → zero W108.
    assert.strictEqual(
      w108B.length,
      0,
      `folder B (off) should suppress W108.  Got ${w108B.length} W108 diags: ${JSON.stringify(w108B.map((d) => d.message))}`,
    );
  });

  test("folder A and folder B differ on W123 for extraCommands names (#407)", async () => {
    // proj-a registers ``folder-a-helper`` via tclLsp.extraCommands.
    // proj-b does not.  Identical source calling ``folder-a-helper`` must
    // be silent in folder A but emit W123 (Unknown command) in folder B.
    // This proves the per-folder extra_commands list actually reaches the
    // analyser's unknown-command emitter rather than just sitting in the
    // FeatureConfig.
    const source = `folder-a-helper foo bar\n`;

    async function w123DiagsFor(
      folderName: string,
      wantW123: boolean,
    ): Promise<vscode.Diagnostic[]> {
      const folder = vscode.workspace.workspaceFolders!.find((f) => f.name === folderName)!;
      // ``extra.tcl`` is materialised by ``runMultiFolderTest.ts`` alongside
      // the folder settings; the edit below supplies its real content.
      const fileUri = vscode.Uri.file(path.join(folder.uri.fsPath, "extra.tcl"));
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
      // Poll for the expected state.  Folder A's answer is "none", which is
      // also what an un-analysed document reports, so it drains the full
      // budget before concluding — that is the price of asserting an absence.
      const w123 = () => vscode.languages.getDiagnostics(fileUri).filter((d) => d.code === "W123");
      for (let i = 0; i < 30 && w123().length > 0 !== wantW123; i++) {
        await new Promise((r) => setTimeout(r, 200));
      }
      return w123();
    }

    const w123A = await w123DiagsFor("proj-a", false);
    const w123B = await w123DiagsFor("proj-b", true);

    assert.strictEqual(
      w123A.length,
      0,
      `folder A (extraCommands includes folder-a-helper) should not emit W123.  Got: ${JSON.stringify(w123A.map((d) => d.message))}`,
    );
    assert.ok(
      w123B.some((d) => String(d.message).includes("folder-a-helper")),
      `folder B (no extraCommands for folder-a-helper) should emit W123 for it.  Got: ${JSON.stringify(w123B.map((d) => d.message))}`,
    );
  });

  test("tcl-lsp.getEffectiveConfig reports the resolved per-folder dialect", async () => {
    // Sanity check the helper the suite setup uses -- if this regresses
    // the rest of the suite will hit the 3-second-wait situation issue
    // #407 reports, just by a different path.
    const folderA = vscode.workspace.workspaceFolders!.find((f) => f.name === "proj-a")!;
    const folderB = vscode.workspace.workspaceFolders!.find((f) => f.name === "proj-b")!;

    const cfgA = (await vscode.commands.executeCommand(
      "tcl-lsp.getEffectiveConfig",
      folderA.uri.toString(),
    )) as EffectiveConfig;
    const cfgB = (await vscode.commands.executeCommand(
      "tcl-lsp.getEffectiveConfig",
      folderB.uri.toString(),
    )) as EffectiveConfig;

    assert.strictEqual(cfgA.dialect, "tcl8.4");
    assert.strictEqual(cfgB.dialect, "f5-irules");
    assert.strictEqual(cfgA.non_ascii_mode, "strict");
    assert.strictEqual(cfgB.non_ascii_mode, "off");
    assert.deepStrictEqual(cfgA.extra_commands, ["folder-a-helper", "shared-util"]);
    assert.deepStrictEqual(cfgB.extra_commands, ["folder-b-helper"]);
  });
});

// Race-detection harness for issue #407: when a per-folder dialect
// arrives *after* a document's first analyse the cached AnalysisResult
// keeps its stale dialect-baked W002 / W108 / W123 diagnostics.  Direct
// startup-race reproduction would need a fresh VS Code process per test
// (mocha shares one process), so this suite drives the same code path
// by flipping the folder's ``tclLsp.dialect`` at runtime via
// ``WorkspaceConfiguration.update()`` and asserting the in-cache
// diagnostics catch up.
suite("Multi-folder per-folder dialect change after open (#407)", () => {
  test("flipping a folder's dialect re-analyses already-open files", async () => {
    // Suite ``Multi-folder workspace configuration (#230)`` ran first
    // and already waited for the initial config to settle, so proj-b
    // is on f5-irules at this point.
    const folderB = vscode.workspace.workspaceFolders!.find((f) => f.name === "proj-b")!;
    const fileUri = vscode.Uri.file(path.join(folderB.uri.fsPath, "race.tcl"));
    // ``active_members`` is the only dialect-sensitive name here: an iRules
    // command, disabled under tcl8.6.  Everything else (``if``, ``set``) is in
    // both command surfaces, so the W002 count is a clean signal for "which
    // dialect was this analysed under" — a body of ``puts`` would raise its own
    // W002 under f5-irules and mask the answer.
    const source = "if { [active_members http_pool] >= 2 } {\n    set flag 1\n}\n";

    // 1. Switch folder B off iRules and wait for the server to settle.
    //    ``active_members`` becomes a disallowed builtin under tcl8.6,
    //    so the analyser should bake W002 into the cached analysis.
    const cfgB = vscode.workspace.getConfiguration("tclLsp", folderB.uri);
    try {
      await cfgB.update("dialect", "tcl8.6", vscode.ConfigurationTarget.WorkspaceFolder);
      await waitForConfigSettled({ "proj-a": "tcl8.4", "proj-b": "tcl8.6" });

      // Open the file under tcl8.6 -- W002 must fire.
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
      const w002Initial = await waitForDiagnostic(fileUri, "W002", true);
      assert.ok(
        w002Initial.length > 0,
        `precondition: W002 should fire under tcl8.6.  Got: ${JSON.stringify(
          vscode.languages.getDiagnostics(fileUri).map((d) => [d.code, d.message]),
        )}`,
      );

      // 2. Flip the folder back to f5-irules and wait for the pull to
      //    settle.  The fix in ``_apply_merged_settings_now`` must
      //    detect the per-folder dialect change and re-analyse the
      //    already-open file, clearing the now-stale W002.
      await cfgB.update("dialect", "f5-irules", vscode.ConfigurationTarget.WorkspaceFolder);
      await waitForConfigSettled({ "proj-a": "tcl8.4", "proj-b": "f5-irules" });
      const w002Cleared = await waitForDiagnostic(fileUri, "W002", false);
      assert.strictEqual(
        w002Cleared.length,
        0,
        `W002 must clear once the per-folder dialect resolves to f5-irules.  ` +
          `Got: ${JSON.stringify(w002Cleared.map((d) => d.message))}`,
      );
    } finally {
      // Restore the fixture state so subsequent tests see the original
      // .vscode/settings.json values.  Write the value back explicitly:
      // ``update(…, undefined)`` *removes* the folder-scoped key, and the
      // updates above have already overwritten the fixture file, so clearing it
      // would leave proj-b inheriting the workspace default (tcl8.6) rather
      // than restoring its ``f5-irules``.
      await cfgB.update("dialect", "f5-irules", vscode.ConfigurationTarget.WorkspaceFolder);
      await waitForConfigSettled({ "proj-a": "tcl8.4", "proj-b": "f5-irules" });
    }
  });
});

// Wait up to *timeoutMs* for a diagnostic with the given *code* to be
// present (or absent if *wantPresent* is false) on *uri*.  Polls every
// 100ms; returns the matching diagnostics for the final state.
async function waitForDiagnostic(
  uri: vscode.Uri,
  code: string,
  wantPresent: boolean,
  timeoutMs: number = 6000,
): Promise<vscode.Diagnostic[]> {
  const deadline = Date.now() + timeoutMs;
  let matches: vscode.Diagnostic[] = [];
  while (Date.now() < deadline) {
    matches = vscode.languages.getDiagnostics(uri).filter((d) => d.code === code);
    if (wantPresent ? matches.length > 0 : matches.length === 0) return matches;
    await new Promise((r) => setTimeout(r, 100));
  }
  return matches;
}
