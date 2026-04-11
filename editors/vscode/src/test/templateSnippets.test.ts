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
