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

suite("Language Registration", () => {
  const expectedLanguageIds = [
    "tcl",
    "tcl-irule",
    "tcl-iapp",
    "tcl-apl",
    "tcl-bigip",
    "tcl8.4",
    "tcl8.5",
    "tcl9.0",
    "tcl-synopsys",
    "tcl-cadence",
    "tcl-xilinx",
    "tcl-quartus",
    "tcl-mentor",
    "tcl-expect",
  ];

  let registeredLanguages: string[];

  suiteSetup(async () => {
    registeredLanguages = await vscode.languages.getLanguages();
  });

  for (const langId of expectedLanguageIds) {
    test(`language '${langId}' is registered`, () => {
      assert.ok(registeredLanguages.includes(langId), `Language '${langId}' should be registered`);
    });
  }

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
});
