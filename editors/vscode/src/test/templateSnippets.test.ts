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
import * as path from "path";
import {
  loadTemplateSnippets,
  parseTemplateSnippetCatalog,
  renderTemplateSnippet,
} from "../templateSnippets";

suite("Template Snippets", () => {
  test("parses and sorts valid snippets", () => {
    const raw = JSON.stringify({
      Zeta: { prefix: "z", body: "puts z" },
      Alpha: { prefix: "a", body: ["puts a", "puts b"] },
      InvalidNoBody: { prefix: "i" },
      InvalidNoPrefix: { body: "puts x" },
    });

    const snippets = parseTemplateSnippetCatalog(raw);

    assert.strictEqual(snippets.length, 2);
    assert.strictEqual(snippets[0].name, "Alpha");
    assert.strictEqual(snippets[1].name, "Zeta");
    assert.strictEqual(renderTemplateSnippet(snippets[0]), "puts a\nputs b");
  });

  test("loads bundled snippet catalog", () => {
    const extensionRoot = path.resolve(__dirname, "../..");
    const snippets = loadTemplateSnippets(extensionRoot);

    assert.ok(snippets.length >= 10, `Expected bundled snippets, got ${snippets.length}`);
    assert.ok(
      snippets.some((entry) => entry.prefix === "tcl-proc"),
      "Expected tcl-proc snippet",
    );
    assert.ok(
      snippets.some((entry) => entry.prefix === "irule-http-request"),
      "Expected irule-http-request snippet",
    );
  });
});
