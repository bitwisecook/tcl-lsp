// tcl-lsp — diagram prompt contract test.

import * as assert from "assert";
import { buildDiagramPrompt } from "../chat/commands/diagram";

suite("Diagram prompt contract", () => {
  test("preserves the complete tcl-diagram payload shape for the model", () => {
    const payload = {
      events: [
        {
          name: "HTTP_REQUEST",
          priority: 400,
          multiplicity: "once",
          flow: [
            {
              kind: "if",
              branches: [{ condition: "path == /", body: [{ kind: "action", label: "pool web" }] }],
            },
          ],
        },
      ],
      procedures: [
        {
          name: "helper",
          params: ["request"],
          flow: [{ kind: "return", value: "request" }],
        },
      ],
    };
    const serialized = JSON.stringify(payload, null, 2);
    const prompt = buildDiagramPrompt("when HTTP_REQUEST { helper $r }", serialized, "");

    assert.ok(prompt.includes("## Structured flow data (authoritative"));
    assert.ok(prompt.includes(serialized), "prompt must embed the exact structured JSON payload");
    assert.ok(prompt.includes('"priority": 400'));
    assert.ok(prompt.includes('"multiplicity": "once"'));
    assert.ok(prompt.includes('"procedures": ['));
    assert.ok(prompt.includes('"params": [\n        "request"\n      ]'));
    assert.ok(prompt.includes('"branches": ['));
  });
});
