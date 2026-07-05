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
import * as fs from "fs";
import * as path from "path";

interface SnippetEntry {
  prefix: string;
  description?: string;
  body: string | string[];
}

function loadSnippets(): Record<string, SnippetEntry> {
  const snippetPath = path.resolve(__dirname, "../../snippets/tcl.code-snippets");
  const raw = fs.readFileSync(snippetPath, "utf8");
  return JSON.parse(raw) as Record<string, SnippetEntry>;
}

suite("Snippet Catalog", () => {
  test("includes core Tcl and iRules snippet prefixes", () => {
    const snippets = loadSnippets();
    const prefixes = new Set(Object.values(snippets).map((entry) => entry.prefix));

    const requiredPrefixes = [
      "tcl-proc",
      "tcl-switch",
      "tcl-catch",
      "tcl-try",
      "irule-rule-init",
      "irule-http-request",
      "irule-collect-release",
      "irule-class-lookup",
    ];

    for (const prefix of requiredPrefixes) {
      assert.ok(prefixes.has(prefix), `Missing snippet prefix: ${prefix}`);
    }
  });

  test("collect/release snippet includes both commands", () => {
    const snippets = loadSnippets();
    const collectSnippet = snippets["iRule Collect/Release Pair"];
    assert.ok(collectSnippet, "iRule Collect/Release Pair snippet not found");

    const body = Array.isArray(collectSnippet.body)
      ? collectSnippet.body.join("\n")
      : collectSnippet.body;

    assert.ok(body.includes("HTTP::collect"), "collect snippet should include HTTP::collect");
    assert.ok(body.includes("HTTP::release"), "collect snippet should include HTTP::release");
    assert.ok(
      body.includes("when HTTP_REQUEST_DATA"),
      "collect snippet should include HTTP_REQUEST_DATA",
    );
  });

  test("HTTP request snippet includes guarded debug logging", () => {
    const snippets = loadSnippets();
    const requestSnippet = snippets["iRule HTTP_REQUEST Skeleton"];
    assert.ok(requestSnippet, "iRule HTTP_REQUEST Skeleton snippet not found");

    const body = Array.isArray(requestSnippet.body)
      ? requestSnippet.body.join("\n")
      : requestSnippet.body;

    assert.ok(
      body.includes("if {\\$debug}"),
      "HTTP_REQUEST snippet should guard logging with $debug",
    );
    assert.ok(
      body.includes("log local0.debug"),
      "HTTP_REQUEST snippet should include debug logging",
    );
  });
});
