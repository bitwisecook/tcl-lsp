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
import { getDocUri, activate, waitForDiagnostics } from "./helper";

// A nested, cross-namespace proc self-redefinition (issue #923 audit idx 45),
// end to end in the editor.  The fixture nestedSelfRedefinition.tcl:
//
//   15: namespace eval ticklecharts {}
//   16: proc ticklecharts::activate {bool} {
//   17:     if {$bool} {
//   18:         proc activate {bool} {puts "already activated"}
//   19:         puts "activating for the first time"
//   20:     }
//   21: }
//   22: ticklecharts::activate 1
//   23: ticklecharts::activate 1
//
// tclsh 9.0.4 and 8.6.16 agree byte-for-byte: the calls print `activating for
// the first time` then `already activated`, and afterwards `info commands
// ::activate` is empty while `info commands ::ticklecharts::activate` reports
// the command.  Both `proc` headers therefore declare ONE command, so a
// reference or rename query from either must answer with the same call sites.
//
// The mirror shape (two verbatim-identical top-level declarations, idx 31) is
// covered at the server tier; this is the namespace-aware half.

/** Start lines of *locations*, sorted and deduplicated. */
function startLines(locations: vscode.Location[]): number[] {
  return [...new Set(locations.map((l) => l.range.start.line))].sort((a, b) => a - b);
}

/** The `code` string of a diagnostic, whatever shape VS Code hands us. */
function codeOf(d: vscode.Diagnostic): string | undefined {
  const code = typeof d.code === "object" && d.code !== null ? d.code.value : d.code;
  return code === undefined ? undefined : String(code);
}

suite("Nested cross-namespace self-redefinition", () => {
  const docUri = getDocUri("nestedSelfRedefinition.tcl");

  // Line 16 col 20 is the `activate` of the outer `ticklecharts::activate`
  // header; line 18 col 13 is the nested, unqualified one.
  const declarations: [number, number, string][] = [
    [16, 20, "outer"],
    [18, 13, "nested"],
  ];

  for (const [line, character, which] of declarations) {
    test(`references from the ${which} declaration reach both call sites`, async () => {
      await activate(docUri);

      const locations = (await vscode.commands.executeCommand(
        "vscode.executeReferenceProvider",
        docUri,
        new vscode.Position(line, character),
      )) as vscode.Location[];

      const lines = startLines(locations);
      assert.ok(
        lines.includes(22) && lines.includes(23),
        `Both callers of the one command must be reported from the ${which} ` +
          `declaration, got ${lines}`,
      );
    });

    test(`rename from the ${which} declaration rewrites both call sites`, async () => {
      await activate(docUri);

      const edit = (await vscode.commands.executeCommand(
        "vscode.executeDocumentRenameProvider",
        docUri,
        new vscode.Position(line, character),
        "enable",
      )) as vscode.WorkspaceEdit | undefined;

      assert.ok(edit, `Rename should succeed on the ${which} declaration`);
      const entry = edit.entries().find(([uri]) => uri.toString() === docUri.toString());
      assert.ok(entry, "Expected edits for the fixture document");
      const lines = [...new Set(entry[1].map((e) => e.range.start.line))].sort((a, b) => a - b);
      assert.ok(
        lines.includes(22) && lines.includes(23),
        `A rename that leaves a caller behind silently rebinds it to whichever ` +
          `declaration was not renamed; got ${lines}`,
      );
    });
  }

  test("the unused-parameter hint names the fully-qualified command", async () => {
    // The lowering bug this finding also carried named the nested proc as the
    // global `::activate`; the nested `proc` really redefines
    // `::ticklecharts::activate` (proved by `info commands` above).
    await activate(docUri);
    const diagnostics = await waitForDiagnostics(docUri, {
      predicate: (d) => d.some((x) => codeOf(x) === "W214"),
    });
    const w214 = diagnostics.filter((d) => codeOf(d) === "W214");
    assert.ok(
      w214.every((d) => d.message.includes("::ticklecharts::activate")),
      `W214 must name the enclosing namespace's command, got: ${w214
        .map((d) => d.message)
        .join("; ")}`,
    );
    assert.ok(
      w214.every((d) => !d.message.includes("'::activate'")),
      `No global ::activate is ever created, got: ${w214.map((d) => d.message).join("; ")}`,
    );
  });
});
