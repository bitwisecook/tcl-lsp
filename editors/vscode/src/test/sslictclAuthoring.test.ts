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

// Authoring a `.sslictcl` document in the editor host (#1543, epic #1524).
//
// The native e2e suite (`rust/tcl-lsp-server/tests/e2e/sslictcl.rs`) proves
// the *server* answers a `.sslictcl` document. What it cannot prove is the
// half a user experiences: that VS Code recognises the extension, activates
// the extension for it, and routes its requests to the language client at
// all. A language contribution the manifest gets wrong fails silently —
// the file opens as plain text, no client attaches, and every server-side
// assertion in the world stays true while the feature is invisible.
//
// So this suite opens the shipped sample through the real extension host and
// asserts the three things that can only be observed there: the language id
// VS Code chose, the diagnostics the client received, and a hover the client
// routed and rendered.

import * as assert from "assert";
import * as vscode from "vscode";

import { activate, getDocUri, waitForDiagnostics } from "./helper";

/** A copy of `samples/sslictcl/example.sslictcl`, the vocabulary document's
 *  worked example. Its own header states which notices it deliberately
 *  raises, so it is the right fixture for "what does a correct document
 *  look like in the editor". */
const docUri = getDocUri("sslictcl/example.sslictcl");

/** The code of a diagnostic, whatever shape the client gave it. */
function codeOf(diagnostic: vscode.Diagnostic): string {
  const { code } = diagnostic;
  if (typeof code === "object" && code !== null) return String(code.value);
  return String(code ?? "");
}

suite("SslicTcl authoring", () => {
  suiteSetup(async function () {
    this.timeout(120_000);
    await activate(docUri);
  });

  test("the sample opens as the sslictcl language", async () => {
    const document = vscode.workspace.textDocuments.find(
      (candidate) => candidate.uri.toString() === docUri.toString(),
    );
    assert.ok(document, "the sample must be open");
    assert.strictEqual(
      document.languageId,
      "sslictcl",
      "the `.sslictcl` extension is contributed by the manifest, so VS Code " +
        "must choose the dialect's own language id rather than plain text",
    );
  });

  test("the loader's notices arrive as diagnostics, and nothing is an error", async function () {
    this.timeout(120_000);
    const diagnostics = await waitForDiagnostics(docUri, {
      timeout: 60_000,
      predicate: (diags) => diags.some((d) => codeOf(d) === "SSLIC1101"),
    });
    const notices = diagnostics.filter((d) => codeOf(d) === "SSLIC1101");
    assert.ok(
      notices.length > 0,
      `expected an SSLIC1101 hint: ${diagnostics.map(codeOf).join(", ")}`,
    );
    assert.strictEqual(
      notices[0].severity,
      vscode.DiagnosticSeverity.Hint,
      "a preserved extension is a hint, not a warning",
    );
    const errors = diagnostics.filter((d) => d.severity === vscode.DiagnosticSeverity.Error);
    assert.deepStrictEqual(
      errors.map((d) => `${codeOf(d)}@${d.range.start.line}`),
      [],
      "the sample loads with no errors at all",
    );
    // The loader owns the verdict on an unrecognised word in a document that
    // is never evaluated, so the analyser's unknown-command hint must not
    // double up on one.
    assert.ok(
      !diagnostics.some((d) => codeOf(d) === "W123"),
      `an extension word must not also draw W123: ${diagnostics.map(codeOf).join(", ")}`,
    );
  });

  test("hovering a declaration returns the vocabulary's own documentation", async function () {
    this.timeout(120_000);
    const document = await vscode.workspace.openTextDocument(docUri);
    const line = document
      .getText()
      .split("\n")
      .findIndex((text) => text.startsWith("endpoint /Common/www"));
    assert.ok(line >= 0, "the sample declares `endpoint /Common/www`");

    const hovers = (await vscode.commands.executeCommand(
      "vscode.executeHoverProvider",
      docUri,
      new vscode.Position(line, 2),
    )) as vscode.Hover[];
    const text = hovers
      .flatMap((hover) => hover.contents)
      .map((content) => (typeof content === "string" ? content : content.value))
      .join("\n");
    assert.ok(text.includes("endpoint"), `hover must name the declaration: ${text}`);
    assert.ok(
      text.includes("Declare a TLS endpoint"),
      `hover must be the SslicTcl pack's own text: ${text}`,
    );
  });
});
