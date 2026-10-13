// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { runInNewContext } from "node:vm";
import test from "node:test";

const source = await readFile(new URL("../explorer-core.js", import.meta.url), "utf8");
const context = {};
for (const [start, end] of [
  ["function pairDisassemblyEntries(", "function openOptDiffView("],
  ["function irToLines(", "function cfgToLines("],
  ["function selectionRowsForFunction(", "function renderSelections("],
  ["function drawAllCfgEdges(", "var edgeRedrawTimer"],
]) {
  runInNewContext(source.slice(source.indexOf(start), source.indexOf(end)), context);
}
const { pairDisassemblyEntries, irToLines, selectionRowsForFunction, drawAllCfgEdges } = context;

function row(bodyKey, name = "same display", bodyId = 0) {
  return { kind: "proc", bodyKey, bodyId, name };
}

test("disassembly pairs use full body coordinates despite equal display names", () => {
  const original = [row("module-a:0"), row("module-a:1")];
  const optimised = [row("module-a:1", "renamed label"), row("module-a:0")];
  const pairs = pairDisassemblyEntries(original, optimised);
  assert.equal(pairs.length, 2);
  assert.equal(pairs[0].orig, original[0]);
  assert.equal(pairs[0].opt, optimised[1]);
  assert.equal(pairs[1].opt, optimised[0]);
});

test("different modules never pair through equal ordinals or names", () => {
  const pairs = pairDisassemblyEntries([row("module-a:0")], [row("module-b:0")]);
  assert.equal(pairs.length, 2);
  assert.equal(pairs[0].opt, undefined);
  assert.equal(pairs[1].orig, undefined);
});

test("unidentified root and helper rows remain separate", () => {
  const original = [{ kind: "top", name: "::top" }, { kind: "helper", name: "helper" }];
  const pairs = pairDisassemblyEntries(original, original.map((entry) => ({ ...entry })));
  assert.equal(pairs.length, 4);
  assert.ok(pairs.every((pair) => !(pair.orig && pair.opt)));
});

test("duplicate full keys on either side refuse arbitrary pairing", () => {
  for (const [original, optimised] of [
    [[row("module-a:0"), row("module-a:0")], [row("module-a:0")]],
    [[row("module-a:0")], [row("module-a:0"), row("module-a:0")]],
  ]) {
    const pairs = pairDisassemblyEntries(original, optimised);
    assert.equal(pairs.length, 3);
    assert.ok(pairs.every((pair) => !(pair.orig && pair.opt)));
  }
});

test("IR text renders retained labels while preserving equal-name rows", () => {
  const lines = irToLines({
    topLevel: [],
    procedures: {
      "body:0": { name: "::p", body: [{ summary: "FIRST" }] },
      "body:1": { name: "::p", body: [{ summary: "SECOND" }] },
    },
  });
  assert.deepEqual(Array.from(lines), ["top-level", "proc ::p", "  FIRST", "proc ::p", "  SECOND"]);
});

test("SCCP selections join exact body coordinates without borrowing equal names", () => {
  const first = row("module-a:0");
  const second = row("module-a:1");
  const rows = [first, second];
  assert.equal(selectionRowsForFunction(second, rows)[0], second);
  assert.equal(selectionRowsForFunction(row("module-b:1"), rows).length, 0);
  assert.equal(selectionRowsForFunction({ name: "same display", kind: "proc" }, rows).length, 0);
  assert.equal(selectionRowsForFunction(first, [first, { ...first }]).length, 0);
});

test("CFG edge redraw uses only the retained current view and preserves equal-name containers", () => {
  const containers = [0, 1].map((index) => ({
    index,
    querySelectorAll: () => ["new-from", "new-to"].map((block) => ({ dataset: { block } })),
  }));
  const calls = [];
  context.drawOrthogonalEdges = (container, edges, options) => calls.push({ container, edges, options });
  const current = [0, 1].map(() => ({ name: "same display", edges: [{ from: "new-from", to: "new-to" }] }));
  const old = [{ name: "same display", edges: [{ from: "old-from", to: "old-to" }] }];
  const pane = { id: "pane-cfg-post", cfgGraphRows: current, querySelectorAll: () => containers };
  drawAllCfgEdges(pane, old);
  assert.equal(calls.length, 2);
  assert.equal(calls[0].container, containers[0]);
  assert.equal(calls[1].container, containers[1]);
  assert.equal(calls[0].edges.length, 1);
  assert.notEqual(calls[0].options.arrowheadIdPrefix, calls[1].options.arrowheadIdPrefix);
});

test("CFG edge redraw declines a pane with no current graph rows", () => {
  context.drawOrthogonalEdges = () => assert.fail("unavailable view must not draw graph edges");
  drawAllCfgEdges({ cfgGraphRows: null }, [{ name: "stale" }]);
});
