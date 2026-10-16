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
  captureDocumentTransform,
  requestDocumentTransform,
  applyDocumentTransform,
} from "../documentTransform";
import { activate, getDocUri, scaledTimeout } from "./helper";
import {
  applyDocumentWrite,
  documentWriteReceipt,
  isDocumentSourceSnapshotCurrent,
  resolveTwoIruleSources,
  snapshotDocumentSource,
} from "../chat/codeUtils";

suite("Command Registration", () => {
  const allCommands = [
    "tclLsp.restartServer",
    "tclLsp.selectDialect",
    "tclLsp.exportConfig",
    "tclLsp.optimiseDocument",
    "tclLsp.showOptimisations",
    "tclLsp.fixAllSafeIssues",
    "tclLsp.toggleDiagnostics",
    "tclLsp.toggleOptimiser",
    "tclLsp.toggleAi",
    "tclLsp.formatDocument",
    "tclLsp.minifyDocument",
    "tclLsp.unminifyError",
    "tclLsp.escapeSelection",
    "tclLsp.unescapeSelection",
    "tclLsp.base64EncodeSelection",
    "tclLsp.base64DecodeSelection",
    "tclLsp.copyFileAsBase64",
    "tclLsp.copyFileAsGzipBase64",
    "tclLsp.insertPackageRequire",
    "tclLsp.scaffoldPackageStarter",
    "tclLsp.insertIruleEventSkeleton",
    "tclLsp.insertTemplateSnippet",
    "tclLsp.runRuntimeValidation",
    "tclLsp.openCompilerExplorer",
    "tclLsp.openSpecStudio",
    "tclLsp.openTkPreview",
    "tclLsp.insertIrule",
    "tclLsp.applyFix",
    "tclLsp.translateXc",
    "tclLsp.extractRule",
    "tclLsp.extractRulePick",
    "tclLsp.extractAllRules",
    "tclLsp.extractLinkedObjects",
    "tclLsp.renamePartition",
    "tclLsp.renameSymbolAtPosition",
    "tclLsp.generateDocstring",
  ];

  let registeredCommands: string[];

  suiteSetup(async function () {
    this.timeout(scaledTimeout(60_000));
    // Ensure the extension is activated before querying commands.
    await activate(getDocUri("simple.tcl"));
    registeredCommands = await vscode.commands.getCommands(true);
  });

  for (const cmd of allCommands) {
    test(`${cmd} is registered`, () => {
      assert.ok(registeredCommands.includes(cmd), `Command '${cmd}' should be registered`);
    });
  }

  test("toggleDiagnostics toggles the feature setting", async () => {
    const before = vscode.workspace
      .getConfiguration("tclLsp.features")
      .get<boolean>("diagnostics", true);
    try {
      await vscode.commands.executeCommand("tclLsp.toggleDiagnostics");
      // Re-fetch configuration to see the updated value.
      const after = vscode.workspace
        .getConfiguration("tclLsp.features")
        .get<boolean>("diagnostics");
      assert.strictEqual(after, !before, "diagnostics should be toggled");
    } finally {
      // Toggle commands write with undefined target; restore at same scope.
      await vscode.workspace
        .getConfiguration("tclLsp.features")
        .update("diagnostics", before, undefined);
    }
  });

  test("source write refuses a changed document and accepts its current snapshot", async () => {
    await activate(getDocUri("simple.tcl"));
    const doc = await vscode.workspace.openTextDocument(getDocUri("simple.tcl"));
    const editor = await vscode.window.showTextDocument(doc);
    const original = doc.getText();
    const originalSnapshot = snapshotDocumentSource(doc);
    try {
      await editor.edit((edit) => {
        edit.insert(
          new vscode.Position(doc.lineCount - 1, doc.lineAt(doc.lineCount - 1).text.length),
          "\n# newer source",
        );
      });
      const changedSource = doc.getText();
      assert.strictEqual(
        await applyDocumentWrite(
          "# stale generated source",
          documentWriteReceipt(originalSnapshot),
        ),
        false,
      );
      assert.strictEqual(doc.getText(), changedSource, "stale action must not replace newer text");
      assert.strictEqual(
        await vscode.commands.executeCommand<boolean>(
          "tclLsp.applyFix",
          "# stale generated source",
          documentWriteReceipt(originalSnapshot),
        ),
        false,
        "the registered sink must report a refused write to immediate callers",
      );

      const currentSnapshot = snapshotDocumentSource(doc);
      assert.strictEqual(
        await applyDocumentWrite(
          "# current generated source",
          documentWriteReceipt(currentSnapshot),
        ),
        true,
      );
      assert.strictEqual(doc.getText(), "# current generated source");
    } finally {
      await editor.edit((edit) => {
        edit.replace(new vscode.Range(0, 0, doc.lineCount, 0), original);
      });
    }
  });

  test("comparison source resolution retains selected ranges and whole-document currency", async () => {
    await activate(getDocUri("simple.tcl"));
    const doc = await vscode.workspace.openTextDocument(getDocUri("simple.tcl"));
    const original = doc.getText();
    const firstLine = doc.lineAt(0).text;
    const selection = new vscode.Range(0, 0, 0, firstLine.length);
    const context = {
      request: {
        references: [{ value: new vscode.Location(doc.uri, selection) }, { value: doc.uri }],
        prompt: "",
      },
    } as unknown as import("../chat/types").CommandContext;
    const resolved = await resolveTwoIruleSources(context);
    assert.ok(resolved);
    const [selected, whole] = resolved;
    assert.strictEqual(selected.source, doc.getText(selection));
    assert.strictEqual(selected.snapshot?.fullSource, original);
    assert.strictEqual(selected.snapshot?.selection, selection);
    assert.strictEqual(whole.source, original);
    const selectedSnapshot = selected.snapshot;
    assert.ok(selectedSnapshot && isDocumentSourceSnapshotCurrent(selectedSnapshot));

    const editor = await vscode.window.showTextDocument(doc);
    try {
      const lastLine = doc.lineAt(doc.lineCount - 1);
      await editor.edit((edit) =>
        edit.insert(new vscode.Position(doc.lineCount - 1, lastLine.text.length), "\n# changed"),
      );
      assert.strictEqual(isDocumentSourceSnapshotCurrent(selectedSnapshot), false);
    } finally {
      await editor.edit((edit) => edit.replace(new vscode.Range(0, 0, doc.lineCount, 0), original));
    }
  });

  test("toggleOptimiser toggles the optimiser.enabled setting", async () => {
    const before = vscode.workspace
      .getConfiguration("tclLsp.optimiser")
      .get<boolean>("enabled", true);
    try {
      await vscode.commands.executeCommand("tclLsp.toggleOptimiser");
      const after = vscode.workspace.getConfiguration("tclLsp.optimiser").get<boolean>("enabled");
      assert.strictEqual(after, !before, "optimiser should be toggled");
    } finally {
      await vscode.workspace
        .getConfiguration("tclLsp.optimiser")
        .update("enabled", before, undefined);
    }
  });

  test("toggleAi toggles the ai.enabled setting", async () => {
    const before = vscode.workspace.getConfiguration("tclLsp.ai").get<boolean>("enabled", true);
    try {
      await vscode.commands.executeCommand("tclLsp.toggleAi");
      const after = vscode.workspace.getConfiguration("tclLsp.ai").get<boolean>("enabled");
      assert.strictEqual(after, !before, "AI features should be toggled");
    } finally {
      await vscode.workspace.getConfiguration("tclLsp.ai").update("enabled", before, undefined);
    }
  });

  test("formatDocument executes without error on a Tcl file", async () => {
    // Use an untitled document to avoid leaking state into other suites.
    const doc = await vscode.workspace.openTextDocument({
      language: "tcl",
      content: "set x 10\n",
    });
    await vscode.window.showTextDocument(doc);
    // Should not throw
    await vscode.commands.executeCommand("tclLsp.formatDocument");
  });
});

suite("Source-bound document transforms", () => {
  test("actual transform result cannot overwrite a buffer changed during confirmation", async () => {
    await activate(getDocUri("simple.tcl"));
    const document = await vscode.workspace.openTextDocument(getDocUri("simple.tcl"));
    const editor = await vscode.window.showTextDocument(document);
    const original = document.getText();
    try {
      const snapshot = captureDocumentTransform(document);
      assert.ok(snapshot, "actual client session and resource configuration");
      const result = await requestDocumentTransform<{ source: string }>(
        snapshot,
        "tcl-lsp.minifyDocument",
      );
      assert.ok(result, "actual source-bound server result");
      await editor.edit((edit) =>
        edit.insert(new vscode.Position(0, 0), "# changed while confirming\n"),
      );
      const changed = document.getText();
      assert.strictEqual(await applyDocumentTransform(snapshot, result), false);
      assert.strictEqual(document.getText(), changed);
      const current = captureDocumentTransform(document);
      assert.ok(current);
      const fresh = await requestDocumentTransform<{ source: string }>(
        current,
        "tcl-lsp.minifyDocument",
      );
      assert.ok(fresh);
      assert.strictEqual(await applyDocumentTransform(current, fresh), true);
      assert.strictEqual(document.getText(), fresh.source);
    } finally {
      await editor.edit((edit) =>
        edit.replace(new vscode.Range(0, 0, document.lineCount, 0), original),
      );
    }
  });
});

suite("Source-bound transform configuration", () => {
  test("a settings change while confirming cannot reuse the earlier response", async () => {
    await activate(getDocUri("simple.tcl"));
    const document = await vscode.workspace.openTextDocument(getDocUri("simple.tcl"));
    const snapshot = captureDocumentTransform(document);
    assert.ok(snapshot);
    const result = await requestDocumentTransform<{ source: string }>(
      snapshot,
      "tcl-lsp.minifyDocument",
    );
    assert.ok(result);
    const source = document.getText();
    const configuration = vscode.workspace.getConfiguration("tclLsp.optimiser", document.uri);
    const before = configuration.get<boolean>("enabled", true);
    try {
      await configuration.update("enabled", !before, undefined);
      assert.strictEqual(await applyDocumentTransform(snapshot, result), false);
      assert.strictEqual(document.getText(), source);
    } finally {
      await configuration.update("enabled", before, undefined);
    }
  });
});

suite("Source-bound transform no-op acceptance", () => {
  test("an unchanged result still requires its actual server receipt", async () => {
    await activate(getDocUri("simple.tcl"));
    const document = await vscode.workspace.openTextDocument({
      language: "tcl",
      content: "puts hello\n",
    });
    await vscode.window.showTextDocument(document);
    const snapshot = captureDocumentTransform(document);
    assert.ok(snapshot);
    const result = await requestDocumentTransform<{ source: string }>(
      snapshot,
      "tcl-lsp.fixAllSafeIssues",
    );
    assert.ok(result);
    assert.strictEqual(result.source, snapshot.source.fullSource, "actual no-op safe-fix response");
    const foreign = {
      ...result,
      editReceipt: { ...result.editReceipt, id: `unissued-${result.editReceipt.id}` },
    };
    assert.strictEqual(
      await applyDocumentTransform(snapshot, foreign),
      false,
      "the actual server refuses an unissued handle even for unchanged text",
    );
    assert.strictEqual(document.getText(), snapshot.source.fullSource);
    assert.strictEqual(await applyDocumentTransform(snapshot, result), true);
    assert.strictEqual(document.getText(), snapshot.source.fullSource);
  });
});
