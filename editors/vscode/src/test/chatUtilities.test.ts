import * as assert from "assert";
import { detectQueryType } from "../chat/commands/event";
import { extractCodeBlock } from "../chat/codeUtils";
import { detectIruleEvents, extractSourceSymbolDefinitions } from "../chat/contextPack";
import { buildTclSystemPrompt } from "../chat/tclSystemPrompt";

suite("Chat Utilities", () => {
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

  test("detectIruleEvents finds unique event handlers", () => {
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

    assert.deepStrictEqual(detectIruleEvents(code), ["HTTP_REQUEST", "HTTP_RESPONSE"]);
  });

  test("extractSourceSymbolDefinitions detects procs and events", () => {
    const code = [
      "namespace eval app {",
      "    proc route {host path} {",
      "        return /",
      "    }",
      "}",
      "when HTTP_REQUEST {",
      "    app::route [HTTP::host] [HTTP::path]",
      "}",
    ].join("\n");

    const defs = extractSourceSymbolDefinitions(code);
    assert.ok(defs.some((entry) => entry.kind === "Namespace" && entry.name === "app"));
    assert.ok(defs.some((entry) => entry.kind === "Function" && entry.name === "route"));
    assert.ok(defs.some((entry) => entry.kind === "Event" && entry.name === "HTTP_REQUEST"));
  });

  test("Tcl system prompt is Tcl-focused", () => {
    const prompt = buildTclSystemPrompt();
    assert.ok(prompt.includes("Tcl"));
    assert.ok(prompt.includes("braced expressions"));
  });
});
