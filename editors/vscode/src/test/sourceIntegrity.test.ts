// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import * as assert from "assert";
import * as fs from "fs";
import * as vscode from "vscode";
import { activate, getDocUri, waitForDiagnostics } from "./helper";

function codeOf(diagnostic: vscode.Diagnostic): string {
  const code = diagnostic.code;
  return String(typeof code === "object" && code !== null ? code.value : code);
}

suite("Source-integrity diagnostics", () => {
  const malformedUri = getDocUri("sourceIntegrityMalformed.tcl");
  const bigipUri = getDocUri("sourceIntegrity.scf");
  const aplUri = getDocUri("sourceIntegrity.apl");
  const generatedUris = [malformedUri, bigipUri, aplUri];

  suiteSetup(() => {
    // The first U+FFFD is authored, valid UTF-8. The invalid byte on the next
    // line causes the decoder to insert a second U+FFFD. W107 must select the
    // insertion backed by DecodeReport, not search for the earlier literal.
    fs.writeFileSync(
      malformedUri.fsPath,
      Buffer.concat([
        Buffer.from('puts "literal \uFFFD"\nputs ', "utf8"),
        Buffer.from([0xff, 0x0a]),
      ]),
    );
    fs.writeFileSync(bigipUri.fsPath, "ltm pool /Common/a\u202Eb {\n}\n", "utf8");
    fs.writeFileSync(aplUri.fsPath, "# \u2066hidden\nsection presentation {\n}\n", "utf8");
  });

  suiteTeardown(() => {
    for (const uri of generatedUris) {
      try {
        fs.unlinkSync(uri.fsPath);
      } catch {
        // A failed setup may not have created every fixture.
      }
    }
  });

  test("W107 selects the decoder insertion after an authored replacement character", async () => {
    await activate(malformedUri);
    const diagnostics = await waitForDiagnostics(malformedUri, {
      predicate: (items) => items.some((item) => codeOf(item) === "W107"),
    });
    const w107 = diagnostics.filter((item) => codeOf(item) === "W107");
    assert.strictEqual(w107.length, 1, `expected one W107; got [${diagnostics.map(codeOf)}]`);
    assert.deepStrictEqual(w107[0].range, new vscode.Range(1, 5, 1, 6));
  });

  test("BIG-IP and APL documents receive the canonical W305 finding", async () => {
    for (const uri of [bigipUri, aplUri]) {
      const document = await activate(uri);
      assert.ok(
        document.languageId === "tcl-bigip" || document.languageId === "tcl-apl",
        `${uri.fsPath} opened as ${document.languageId}`,
      );
      const diagnostics = await waitForDiagnostics(uri, {
        predicate: (items) => items.some((item) => codeOf(item) === "W305"),
      });
      assert.strictEqual(
        diagnostics.filter((item) => codeOf(item) === "W305").length,
        1,
        `${uri.fsPath}: expected one W305; got [${diagnostics.map(codeOf)}]`,
      );
      assert.ok(
        !diagnostics.some((item) => codeOf(item) === "W108"),
        `${uri.fsPath}: W108 must not duplicate W305`,
      );
    }
  });
});
