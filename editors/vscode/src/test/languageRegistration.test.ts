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

  test("tcl-apl gets a document selector entry", () => {
    // Mirror the documentSelector construction in extension.ts so the client
    // attaches to APL buffers and provides LSP features.
    const documentSelector = [...TCL_LANGUAGE_IDS].flatMap((lang) => [
      { scheme: "file", language: lang },
      { scheme: "untitled", language: lang },
    ]);
    assert.ok(
      documentSelector.some((s) => s.scheme === "file" && s.language === "tcl-apl"),
      "documentSelector should include a file entry for tcl-apl",
    );
    assert.ok(
      documentSelector.some((s) => s.scheme === "untitled" && s.language === "tcl-apl"),
      "documentSelector should include an untitled entry for tcl-apl",
    );
  });
});
