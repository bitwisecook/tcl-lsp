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
import { detectQueryType } from "../chat/commands/event";
import { extractCodeBlock } from "../chat/codeUtils";
import { detectIruleEvents, extractSourceSymbolDefinitions } from "../chat/contextPack";
import { buildTclSystemPrompt } from "../chat/tclSystemPrompt";
import { activate, getDocUri } from "./helper";

suite("Chat Utilities", () => {
  suiteSetup(async () => {
    await activate(getDocUri("procs.tcl"));
  });

  test("detectQueryType identifies iRules command queries", () => {
    const result = detectQueryType("What does HTTP::header do?");
    assert.strictEqual(result.type, "command");
    assert.strictEqual(result.value, "HTTP::header");
  });

  test("detectQueryType identifies event queries", () => {
    const result = detectQueryType("Use HTTP_REQUEST to route traffic");
    assert.strictEqual(result.type, "event");
    assert.strictEqual(result.value, "HTTP_REQUEST");
  });

  test("extractCodeBlock returns Tcl code payload", () => {
    const text = "Here is code:\n```tcl\nset x 1\nputs $x\n```\n";
    assert.strictEqual(extractCodeBlock(text), "set x 1\nputs $x");
  });

  test("detectIruleEvents uses shared current event cards", async () => {
    const code = [
      "when HTTP_REQUEST {",
      "    return",
      "}",
      "when HTTP_RESPONSE {",
      "    return",
      "}",
      "when HTTP_REQUEST {",
      "    return",
      "}",
    ].join("\n");

    const definitions = await extractSourceSymbolDefinitions(
      code,
      Number.POSITIVE_INFINITY,
      undefined,
      "tcl-irule",
    );
    assert.ok(definitions, "shared event provider is available");
    assert.deepStrictEqual(detectIruleEvents(definitions), ["HTTP_REQUEST", "HTTP_RESPONSE"]);
  });

  test("extractSourceSymbolDefinitions uses shared namespace and procedure owners", async () => {
    const code = [
      "namespace eval app {",
      "    proc route {host path} {",
      "        return /",
      "    }",
      "}",
    ].join("\n");

    const defs = await extractSourceSymbolDefinitions(
      code,
      Number.POSITIVE_INFINITY,
      undefined,
      "tcl86",
    );
    assert.ok(defs, "shared symbol provider is available");
    assert.ok(defs.some((entry) => entry.kind === "Namespace" && entry.name === "app"));
    assert.ok(defs.some((entry) => entry.kind === "Function" && entry.name === "route"));
    assert.ok(!defs.some((entry) => entry.kind === "Event"));
  });

  test("shared source cards keep repeated same-line names and exclude inert text", async () => {
    const code = "proc p {first} {}; proc p {second} {}; set inert {proc fake {} {}}";
    const definitions = await extractSourceSymbolDefinitions(
      code,
      Number.POSITIVE_INFINITY,
      undefined,
      "tcl86",
    );
    assert.ok(definitions, "shared symbol provider is available");
    const procedures = definitions.filter(
      (entry) => entry.kind === "Function" && entry.name === "p",
    );
    assert.strictEqual(procedures.length, 2);
    assert.ok(procedures.some((entry) => entry.detail?.includes("first")));
    assert.ok(procedures.some((entry) => entry.detail?.includes("second")));
    assert.ok(!definitions.some((entry) => entry.name === "fake"));
  });

  test("source symbol context does not borrow a Markdown provider", async () => {
    const code = "proc p {value} {return $value}";
    const document = await vscode.workspace.openTextDocument({
      language: "markdown",
      content: code,
    });
    const definitions = await extractSourceSymbolDefinitions(
      code,
      Number.POSITIVE_INFINITY,
      document,
      "tcl86",
    );
    assert.ok(definitions?.some((entry) => entry.kind === "Function" && entry.name === "p"));
    assert.strictEqual(document.languageId, "markdown");
    assert.strictEqual(document.getText(), code);
  });

  test("an explicit source dialect does not borrow the original document language", async () => {
    const code = "when HTTP_REQUEST {}";
    const document = await vscode.workspace.openTextDocument({ language: "tcl86", content: code });
    const definitions = await extractSourceSymbolDefinitions(
      code,
      Number.POSITIVE_INFINITY,
      document,
      "tcl-irule",
    );
    assert.ok(definitions, "shared iRules source provider is available");
    assert.deepStrictEqual(detectIruleEvents(definitions), ["HTTP_REQUEST"]);
    assert.strictEqual(document.languageId, "tcl86");
    assert.strictEqual(document.getText(), code);
  });

  test("Tcl system prompt is Tcl-focused", () => {
    const prompt = buildTclSystemPrompt();
    assert.ok(prompt.includes("Tcl"));
    assert.ok(prompt.includes("braced expressions"));
  });
});
