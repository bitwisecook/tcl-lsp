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
  catalogueChoices,
  dialectFromEffectiveConfig,
  groupDialectChoices,
  parseListedDialects,
} from "../dialectChoices";
import { buildClientOptions, dropForeignViews } from "../clientCore";
import { convertShowReferencesArgs } from "../showReferences";
import { DiffDiagnosticsSuppressor } from "../diffAnalysis";
import { LANGUAGE_ID_DIALECTS, TCL_LANGUAGE_IDS, isTclLanguage } from "../languageIds";

interface Manifest {
  activationEvents: string[];
  contributes: { languages: { id: string }[] };
}

/** The extension's own manifest: the list of languages every check below derives from. */
function manifest(): Manifest {
  return vscode.extensions.getExtension("bitwisecook.tcl-lsp")!.packageJSON as Manifest;
}

suite("Language Registration", () => {
  let registeredLanguages: string[];

  suiteSetup(async () => {
    registeredLanguages = await vscode.languages.getLanguages();
  });

  test("every language the manifest contributes is registered", () => {
    const contributed = manifest().contributes.languages.map((language) => language.id);
    assert.ok(contributed.includes("tcl-jim"), "the manifest contributes tcl-jim");
    const missing = contributed.filter((id) => !registeredLanguages.includes(id));
    assert.deepStrictEqual(missing, [], `contributed but not registered: ${missing.join(", ")}`);
  });

  test("every contributed language is a Tcl language id that activates the extension", () => {
    const contributed = manifest().contributes.languages.map((language) => language.id);
    assert.deepStrictEqual([...TCL_LANGUAGE_IDS].sort(), [...contributed].sort());
    for (const id of contributed) {
      assert.ok(
        manifest().activationEvents.includes(`onLanguage:${id}`),
        `${id} has no onLanguage activation event`,
      );
      assert.ok(
        id === "tcl" || Object.prototype.hasOwnProperty.call(LANGUAGE_ID_DIALECTS, id),
        `${id} implies no dialect in LANGUAGE_ID_DIALECTS`,
      );
    }
  });

  test("the dialect picker lists languages before Tcl-plus-packages environments", () => {
    const choices = catalogueChoices();
    const groups = groupDialectChoices(choices);
    assert.deepStrictEqual(
      groups.map((group) => group.title),
      ["Dialects", "Tcl + packages"],
    );
    const [languages, packages] = groups;
    assert.ok(languages.choices.some((choice) => choice.name === "jim"));
    assert.ok(
      packages.choices.some(
        (choice) =>
          choice.name === "xilinx-eda-tcl" && choice.description.includes("Tcl 8.5 + vivado"),
      ),
    );
    for (const choice of choices) {
      assert.ok(choice.shortLabel && choice.description, `${choice.name} lacks a label`);
    }
  });

  test("the status bar label comes from the server's labels, with the catalogue as fallback", () => {
    const served = dialectFromEffectiveConfig({
      dialect: "jim",
      dialect_id: "jim",
      dialect_display_name: "Jim Tcl",
      dialect_short_name: "Jim",
      dialect_kind: "language",
      dialect_description: "Jim Tcl",
    });
    assert.strictEqual(served?.name, "jim");
    assert.strictEqual(served?.shortLabel, "Jim");
    assert.strictEqual(
      dialectFromEffectiveConfig({ dialect: "xilinx-eda-tcl" })?.shortLabel,
      "Vivado",
    );
    assert.strictEqual(dialectFromEffectiveConfig(null), undefined);
    assert.strictEqual(parseListedDialects([{ name: "incomplete" }]), undefined);
  });

  test("tcl files are associated with the tcl language", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "tcl",
      content: "set x 1\n",
    });
    assert.strictEqual(doc.languageId, "tcl");
  });

  test("iRule content can be opened with tcl-irule language", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "tcl-irule",
      content: 'when HTTP_REQUEST {\n    log local0. "test"\n}\n',
    });
    assert.strictEqual(doc.languageId, "tcl-irule");
  });

  test("iApp content can be opened with tcl-iapp language", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "tcl-iapp",
      content: "set x 1\n",
    });
    assert.strictEqual(doc.languageId, "tcl-iapp");
  });

  test("BIG-IP config can be opened with tcl-bigip language", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "tcl-bigip",
      content: "ltm virtual /Common/test {\n}\n",
    });
    assert.strictEqual(doc.languageId, "tcl-bigip");
  });

  test("Jim content can be opened with tcl-jim language", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "tcl-jim",
      content: "alias greet puts\n",
    });
    assert.strictEqual(doc.languageId, "tcl-jim");
  });

  test("APL content can be opened with tcl-apl language", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "tcl-apl",
      content: "section networking {\n}\n",
    });
    assert.strictEqual(doc.languageId, "tcl-apl");
  });

  test("tcl-apl is a recognised Tcl language id", () => {
    assert.ok(
      TCL_LANGUAGE_IDS.has("tcl-apl"),
      "tcl-apl should be in TCL_LANGUAGE_IDS so it drives the dialect status bar and save-triggered validation",
    );
    assert.ok(isTclLanguage("tcl-apl"), "isTclLanguage('tcl-apl') should be true");
  });

  test("tclLsp.viewOnlySchemes drives the desktop selector and drops diagnostics", async () => {
    const cfg = vscode.workspace.getConfiguration("tclLsp");
    assert.deepStrictEqual(cfg.get("viewOnlySchemes"), ["git"], "built-in Git is on by default");
    // `file` must be ignored, or one stray entry would hide every real diagnostic.
    await cfg.update(
      "viewOnlySchemes",
      ["git", "gl-review", "file"],
      vscode.ConfigurationTarget.Global,
    );
    const suppressor = new DiffDiagnosticsSuppressor();
    try {
      const options = buildClientOptions(suppressor);
      const selector = options.documentSelector as { scheme?: string; language: string }[];
      const has = (scheme: string, language: string) =>
        selector.some((s) => s.scheme === scheme && s.language === language);
      for (const scheme of ["file", "untitled", "git", "gl-review"]) {
        for (const language of ["tcl", "tcl-apl"]) {
          assert.ok(has(scheme, language), `documentSelector should include ${scheme}:${language}`);
        }
      }
      assert.ok(!has("gitlab-remote", "tcl"), "an unlisted scheme stays out of the selector");

      // A view-only document gets highlighting, never diagnostics.
      const diagnostic = new vscode.Diagnostic(new vscode.Range(0, 0, 0, 1), "x");
      const applied = new Map<string, number>();
      const sink = (uri: vscode.Uri, diags: vscode.Diagnostic[]) =>
        applied.set(uri.scheme, diags.length);
      const handle = options.middleware?.handleDiagnostics;
      assert.ok(handle, "handleDiagnostics middleware should be installed");
      handle(vscode.Uri.parse("git:/w/a.tcl?ref"), [diagnostic], sink);
      handle(vscode.Uri.parse("gl-review:/w/a.tcl"), [diagnostic], sink);
      handle(vscode.Uri.parse("gitlab-remote:/w/a.tcl"), [diagnostic], sink);
      handle(vscode.Uri.file("/w/a.tcl"), [diagnostic], sink);
      assert.strictEqual(applied.get("git"), 0);
      assert.strictEqual(applied.get("gl-review"), 0);
      assert.strictEqual(applied.get("gitlab-remote"), 1);
      assert.strictEqual(applied.get("file"), 1);
    } finally {
      suppressor.dispose();
      await cfg.update("viewOnlySchemes", undefined, vscode.ConfigurationTarget.Global);
    }
  });

  test("navigation drops view-only documents other than the origin", () => {
    const range = new vscode.Range(0, 0, 0, 1);
    const working = vscode.Uri.file("/w/a.tcl");
    const oldRev = vscode.Uri.parse("git:/w/a.tcl?old");
    const mrRev = vscode.Uri.parse("gl-review:/w/a.tcl");
    const at = (uri: vscode.Uri) => new vscode.Location(uri, range);
    const uris = (locs: vscode.Location[] | undefined) => locs?.map((l) => l.uri.toString());

    const views = ["git", "gl-review"];

    // From the working tree, every view-only copy is a ghost.
    assert.deepStrictEqual(
      uris(dropForeignViews([at(working), at(oldRev), at(mrRev)], working, views)),
      [working.toString()],
    );
    // From inside a view, that view's own hits stay.
    assert.deepStrictEqual(
      uris(dropForeignViews([at(working), at(oldRev), at(mrRev)], oldRev, views)),
      [working.toString(), oldRev.toString()],
    );
    // A single-location definition pointing into a foreign view vanishes.
    assert.strictEqual(dropForeignViews(at(oldRev), working, views), undefined);
    // An unlisted scheme is left alone.
    assert.deepStrictEqual(uris(dropForeignViews([at(mrRev)], working, ["git"])), [
      mrRev.toString(),
    ]);
    // LocationLink targets are judged by targetUri.
    const link: vscode.LocationLink = { targetUri: mrRev, targetRange: range };
    assert.deepStrictEqual(dropForeignViews([link], working, views), []);
    // Code-lens "show references" goes through the same filter (default: git).
    const lens = convertShowReferencesArgs(working.toString(), { line: 0, character: 0 }, [
      { uri: working.toString(), range: { start: range.start, end: range.end } },
      { uri: oldRev.toString(), range: { start: range.start, end: range.end } },
    ]);
    assert.deepStrictEqual(uris(lens.locations), [working.toString()]);
  });
});
