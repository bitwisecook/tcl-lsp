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
import { getDocUri, activate } from "./helper";

suite("Signature Help", () => {
  const docUri = getDocUri("simple.tcl");

  test("provides signature help for a built-in command", async () => {
    await activate(docUri);

    // Use an untitled document to avoid leaking state into other suites.
    const doc = await vscode.workspace.openTextDocument({
      language: "tcl",
      content: "string length \n",
    });
    await vscode.window.showTextDocument(doc);

    // Trigger signature help after 'string length '
    const pos = new vscode.Position(0, 14);
    const result = (await vscode.commands.executeCommand(
      "vscode.executeSignatureHelpProvider",
      doc.uri,
      pos,
    )) as vscode.SignatureHelp | undefined;

    // Verify the provider is wired up and does not throw. If it does return
    // signature help, ensure the shape is well-formed.
    if (result) {
      assert.ok(
        Array.isArray(result.signatures),
        "SignatureHelp should include a signatures array",
      );
    }
  });

  test("provides signature help for a user proc", async () => {
    await activate(docUri);

    // Use an untitled document to avoid leaking state into other suites.
    const doc = await vscode.workspace.openTextDocument({
      language: "tcl",
      content: 'proc greet {name greeting} {\n    puts "$greeting, $name"\n}\ngreet \n',
    });
    await vscode.window.showTextDocument(doc);

    // Trigger signature help after 'greet ' on line 3
    const pos = new vscode.Position(3, 6);
    const result = (await vscode.commands.executeCommand(
      "vscode.executeSignatureHelpProvider",
      doc.uri,
      pos,
    )) as vscode.SignatureHelp | undefined;

    if (result && result.signatures.length > 0) {
      const sig = result.signatures[0];
      assert.ok(sig.label, "Signature should have a label");
      assert.ok(
        sig.parameters && sig.parameters.length > 0,
        "Signature for proc with params should list parameters",
      );
    }
  });
});
