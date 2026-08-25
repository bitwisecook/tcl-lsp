// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import * as assert from "assert";
import { getTkPreviewHtml } from "../tkPreviewPanelHtml";

suite("Tk preview webview", () => {
  const nonce = "0123456789abcdef";
  const html = getTkPreviewHtml("vscode-webview://preview", nonce);

  test("locks scripts to the per-panel nonce", () => {
    assert.ok(html.includes(`script-src 'nonce-${nonce}'`));
    assert.ok(html.includes(`<script nonce="${nonce}">`));
    assert.ok(!html.includes("script-src 'unsafe-inline'"));
  });

  test("validates every source-derived CSS number before interpolation", () => {
    assert.ok(html.includes("function finiteNumber"));
    assert.ok(html.includes("if (x !== undefined) parts.push('left:' + x + 'px')"));
    assert.ok(!html.includes("parts.push('left:' + pl.x + 'px')"));
    assert.ok(!html.includes("parts.push('padding-left:' + g.padx"));
    assert.ok(html.includes("Number.isSafeInteger(source.start)"));
  });

  test("allow-lists source-derived CSS classes", () => {
    assert.ok(html.includes("requested === 'horizontal' ? 'horizontal' : 'vertical'"));
    assert.ok(html.includes("['top', 'bottom', 'left', 'right'].includes(requested)"));
    assert.ok(html.includes("replace(/[^a-z0-9_-]/g, '-')"));
    assert.ok(html.includes("typeof type === 'string' ? type : ''"));
  });

  test("uses the versioned model field names", () => {
    assert.ok(html.includes("widget.path"));
    assert.ok(!html.includes("child.pathname"));
  });

  test("does not present uncertain or -in facts as a verified lexical layout", () => {
    assert.ok(html.includes("widget.certainty === 'potential'"));
    assert.ok(html.includes("placement.certainty !== 'certain'"));
    assert.ok(html.includes("typeof container === 'string' && container === parentPath"));
    assert.ok(html.includes("layout not statically asserted"));
    assert.ok(html.includes("unresolved geometry container"));
    assert.ok(html.includes("managed in "));
    assert.ok(html.includes("placementAppliesIn(child, widget.path)"));
    assert.ok(html.includes("data.uncertainties"));
    assert.ok(html.includes("sourceAttributes(item && item.source"));
  });

  test("does not invent notebook tabs, menu entries, or treeview rows", () => {
    assert.ok(html.includes("Tab membership requires a notebook add/insert fact"));
    assert.ok(html.includes("Menu entries are not statically modeled"));
    assert.ok(!html.includes('tk-menu-item">File'));
    assert.ok(!html.includes('tk-ttk-treeview-row">Row 1'));
    assert.ok(html.includes("items are not statically modeled"));
    assert.ok(!html.includes("Item 1"));
    assert.ok(html.includes("value is not statically modeled"));
    assert.ok(!html.includes("width: 40%"));
  });

  test("links geometry evidence and reports bounded model truncation", () => {
    assert.ok(html.includes("Reveal geometry source for "));
    assert.ok(html.includes("Reveal conflicting geometry source for "));
    assert.ok(html.includes("data.widgets_truncated"));
  });

  test("renders orphan nodes with one tree assignment and validates widget count", () => {
    assert.ok(html.includes("const widgetCount = Number.isSafeInteger(data.widget_count)"));
    assert.ok(html.includes("let treeHtml = renderTreeNode(root, true, '')"));
    assert.ok(html.includes("treeHtml += data.orphan_widgets"));
    assert.ok(html.includes("treeTab.innerHTML = treeHtml"));
    assert.ok(!html.includes("treeTab.innerHTML +="));
    assert.ok(html.includes("let visualHtml = renderWidget(root)"));
    assert.ok(html.includes("visualTab.innerHTML = visualHtml"));
    assert.ok(!html.includes("visualTab.innerHTML +="));
  });

  test("the tree renderer owns the geometry source it links", () => {
    const start = html.indexOf("function renderTreeNode");
    const end = html.indexOf("/* ── HTML escaping utility", start);
    assert.ok(start >= 0 && end > start, "tree renderer should be present");
    const treeRenderer = html.slice(start, end);
    assert.ok(
      treeRenderer.includes("const geometrySource = widget.geometry && widget.geometry.source;"),
      "renderTreeNode must not reference childStyle's local geometrySource",
    );
  });
});
