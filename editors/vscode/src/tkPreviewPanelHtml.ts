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

/**
 * Returns the full HTML content for the Tk Preview webview panel.
 *
 * The webview renders a visual approximation of Tk widgets using HTML/CSS,
 * supporting grid, pack, and place geometry managers. It provides two tabs:
 * "Visual Preview" (rendered widgets) and "Widget Tree" (hierarchical view).
 *
 * Message protocol (extension → webview):
 *   { type: "layout", data: TkUiModel }   — render schema version 1
 *   { type: "status", text: string }       — show a status message
 *   { type: "error",  message: string }    — show an error message
 *   { type: "empty" }                      — show "no Tk content" placeholder
 *   { type: "unavailable", message }       — active editor cannot be previewed
 *
 * Message protocol (webview → extension):
 *   { type: "ready" }                      — webview has finished loading
 *   { type: "revealSource", start, end }   — reveal a model byte span
 */
export function getTkPreviewHtml(cspSource: string, nonce: string): string {
  return /* html */ `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=device-width, initial-scale=1.0" />
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src ${cspSource} 'unsafe-inline'; script-src 'nonce-${nonce}';" />
<title>Tk Preview</title>
<style>
  /* ── Reset & base ──────────────────────────────────────────────── */
  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

  body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
                 Helvetica, Arial, sans-serif;
    font-size: 13px;
    color: var(--vscode-foreground, #333);
    background: var(--vscode-editor-background, #fff);
    overflow: auto;
  }

  /* ── Tab bar ───────────────────────────────────────────────────── */
  .tab-bar {
    display: flex;
    border-bottom: 1px solid var(--vscode-panel-border, #ccc);
    background: var(--vscode-editorGroupHeader-tabsBackground, #f3f3f3);
    user-select: none;
  }
  .tab-bar button {
    padding: 6px 16px;
    border: none;
    background: transparent;
    cursor: pointer;
    font-size: 12px;
    color: var(--vscode-foreground, #333);
    border-bottom: 2px solid transparent;
  }
  .tab-bar button.active {
    border-bottom-color: var(--vscode-focusBorder, #007acc);
    font-weight: 600;
  }
  .tab-bar button:hover {
    background: var(--vscode-list-hoverBackground, #e8e8e8);
  }

  /* ── Tab content ───────────────────────────────────────────────── */
  .tab-content { display: none; padding: 12px; }
  .tab-content.active { display: block; }

  /* ── Status / error overlays ───────────────────────────────────── */
  #overlay {
    display: none;
    padding: 24px;
    text-align: center;
    color: var(--vscode-descriptionForeground, #888);
  }
  #overlay.visible { display: block; }
  #overlay.error { color: var(--vscode-errorForeground, #f44); }

  /* ── Visual preview: Tk widget styles ──────────────────────────── */
  .tk-toplevel {
    border: 1px solid #999;
    background: #d9d9d9;
    padding: 2px;
    min-width: 200px;
    min-height: 100px;
    position: relative;
  }
  .tk-toplevel-title {
    background: linear-gradient(to right, #0058a3, #3a8fd4);
    color: #fff;
    font-size: 12px;
    padding: 3px 8px;
    margin: -2px -2px 2px -2px;
    user-select: none;
  }
  .tk-layout-abstained {
    border: 1px dashed var(--vscode-descriptionForeground, #888);
    padding: 3px;
    margin: 3px;
  }

  .tk-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 2px 10px;
    border: 2px outset #d9d9d9;
    background: #d9d9d9;
    font-size: 12px;
    cursor: default;
    min-height: 24px;
  }

  .tk-entry, .tk-ttk-entry {
    border: 2px inset #d9d9d9;
    background: #fff;
    font-family: monospace;
    font-size: 12px;
    padding: 2px 4px;
    min-width: 100px;
    min-height: 20px;
  }

  .tk-label, .tk-ttk-label {
    padding: 2px 4px;
    font-size: 12px;
    background: transparent;
  }

  .tk-frame, .tk-ttk-frame {
    border: 1px groove #d9d9d9;
    background: #d9d9d9;
    padding: 4px;
    min-height: 20px;
  }

  .tk-labelframe, .tk-ttk-labelframe {
    border: 2px groove #d9d9d9;
    background: #d9d9d9;
    padding: 8px 4px 4px 4px;
    position: relative;
    margin-top: 8px;
  }
  .tk-labelframe-text, .tk-ttk-labelframe-text {
    position: absolute;
    top: -9px;
    left: 10px;
    background: #d9d9d9;
    padding: 0 4px;
    font-size: 12px;
  }

  .tk-text {
    border: 2px inset #d9d9d9;
    background: #fff;
    font-family: monospace;
    font-size: 12px;
    padding: 4px;
    min-width: 120px;
    min-height: 60px;
    overflow: auto;
  }

  .tk-listbox {
    border: 2px inset #d9d9d9;
    background: #fff;
    font-size: 12px;
    padding: 2px;
    min-width: 100px;
    min-height: 60px;
    overflow: auto;
  }
  .tk-listbox-item {
    padding: 1px 4px;
  }
  .tk-listbox-item:nth-child(odd) {
    background: #f0f0f0;
  }

  .tk-canvas {
    border: 1px solid #999;
    background: #e8e8e8;
    min-width: 100px;
    min-height: 80px;
  }

  .tk-scrollbar {
    background: #c0c0c0;
    border: 1px solid #999;
  }
  .tk-scrollbar.vertical {
    width: 16px;
    min-height: 40px;
  }
  .tk-scrollbar.horizontal {
    height: 16px;
    min-width: 40px;
  }

  .tk-checkbutton {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    padding: 2px 4px;
    cursor: default;
  }
  .tk-checkbutton::before {
    content: "";
    display: inline-block;
    width: 13px;
    height: 13px;
    border: 1px inset #d9d9d9;
    background: #fff;
    flex-shrink: 0;
  }

  .tk-radiobutton {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    padding: 2px 4px;
    cursor: default;
  }
  .tk-radiobutton::before {
    content: "";
    display: inline-block;
    width: 13px;
    height: 13px;
    border: 1px inset #d9d9d9;
    background: #fff;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .tk-scale {
    display: flex;
    align-items: center;
    padding: 4px;
    min-width: 100px;
  }
  .tk-scale-track {
    flex: 1;
    height: 4px;
    background: #c0c0c0;
    border: 1px inset #d9d9d9;
    position: relative;
  }
  .tk-scale-thumb {
    position: absolute;
    width: 12px;
    height: 20px;
    background: #d9d9d9;
    border: 2px outset #d9d9d9;
    top: -9px;
    left: 30%;
  }

  .tk-ttk-combobox {
    display: inline-flex;
    align-items: center;
    border: 1px solid #999;
    background: #fff;
    font-size: 12px;
    min-width: 100px;
    min-height: 22px;
  }
  .tk-ttk-combobox-text {
    flex: 1;
    padding: 2px 4px;
    font-family: monospace;
  }
  .tk-ttk-combobox-arrow {
    width: 18px;
    height: 100%;
    background: #d9d9d9;
    border-left: 1px solid #999;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 10px;
  }

  .tk-ttk-treeview {
    border: 1px solid #999;
    background: #fff;
    font-size: 12px;
    min-width: 120px;
    min-height: 60px;
  }
  .tk-ttk-treeview-heading {
    background: #e8e8e8;
    border-bottom: 1px solid #999;
    padding: 2px 6px;
    font-weight: 600;
    font-size: 11px;
  }
  .tk-ttk-treeview-row {
    padding: 1px 6px;
    border-bottom: 1px solid #eee;
  }

  .tk-ttk-notebook {
    border: 1px solid #999;
    background: #d9d9d9;
    min-height: 60px;
  }
  .tk-ttk-notebook-tabs {
    display: flex;
    background: #c0c0c0;
    border-bottom: 1px solid #999;
  }
  .tk-ttk-notebook-tab {
    padding: 3px 12px;
    font-size: 12px;
    border-right: 1px solid #999;
    cursor: default;
  }
  .tk-ttk-notebook-tab:first-child {
    background: #d9d9d9;
    font-weight: 600;
  }
  .tk-ttk-notebook-body {
    padding: 4px;
  }

  .tk-ttk-progressbar {
    border: 1px solid #999;
    background: #e8e8e8;
    height: 20px;
    min-width: 100px;
    position: relative;
    overflow: hidden;
  }
  .tk-ttk-progressbar-fill {
    position: absolute;
    top: 0;
    left: 0;
    height: 100%;
    width: 0;
    background: #0078d7;
  }

  .tk-ttk-separator {
    background: #999;
  }
  .tk-ttk-separator.horizontal {
    height: 1px;
    min-width: 40px;
  }
  .tk-ttk-separator.vertical {
    width: 1px;
    min-height: 40px;
  }

  .tk-menu {
    display: flex;
    background: #f0f0f0;
    border-bottom: 1px solid #999;
    padding: 0;
    min-height: 22px;
  }
  .tk-menu-item {
    padding: 2px 10px;
    font-size: 12px;
    cursor: default;
  }
  .tk-menu-item:hover {
    background: #0078d7;
    color: #fff;
  }

  /* ── Geometry: grid container ──────────────────────────────────── */
  .geo-grid {
    display: grid;
    gap: 2px;
  }

  /* ── Geometry: pack container ──────────────────────────────────── */
  .geo-pack {
    display: flex;
    gap: 2px;
  }
  .geo-pack.pack-top    { flex-direction: column; }
  .geo-pack.pack-bottom { flex-direction: column-reverse; }
  .geo-pack.pack-left   { flex-direction: row; }
  .geo-pack.pack-right  { flex-direction: row-reverse; }

  /* ── Geometry: place container ──────────────────────────────────── */
  .geo-place {
    position: relative;
  }
  .geo-place > * {
    position: absolute;
  }

  /* ── Widget Tree tab ───────────────────────────────────────────── */
  .tree-node {
    margin-left: 16px;
    border-left: 1px solid var(--vscode-panel-border, #ccc);
    padding-left: 8px;
    margin-top: 2px;
  }
  .tree-node-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 0;
    cursor: default;
  }
  .tree-node-type {
    font-weight: 600;
    color: var(--vscode-symbolIcon-classForeground, #d67e00);
  }
  .tree-node-path {
    color: var(--vscode-descriptionForeground, #888);
    font-family: monospace;
    font-size: 11px;
  }
  .tree-node-geo {
    color: var(--vscode-debugTokenExpression-number, #098658);
    font-size: 11px;
  }
  .tree-node-opts {
    color: var(--vscode-descriptionForeground, #888);
    font-size: 11px;
    font-style: italic;
  }
  .tree-root {
    margin-left: 0;
    border-left: none;
    padding-left: 0;
  }
  .tk-fact-badge {
    display: inline-block;
    margin: 2px 4px;
    padding: 1px 5px;
    border: 1px dashed var(--vscode-editorWarning-foreground, #b89500);
    border-radius: 3px;
    color: var(--vscode-editorWarning-foreground, #8a6d00);
    background: var(--vscode-editorWarning-background, rgba(255, 193, 7, 0.12));
    font-size: 10px;
  }
  .uncertainty-list { margin: 4px 0 0 18px; }
  .uncertainty-list li { margin: 3px 0; }
  .source-link { cursor: pointer; text-decoration: underline; }
</style>
</head>
<body>

<div class="tab-bar" role="tablist" aria-label="Tk preview views">
  <button id="tab-button-visual" class="active" data-tab="visual" role="tab" aria-selected="true" aria-controls="tab-visual">Visual Preview</button>
  <button id="tab-button-tree" data-tab="tree" role="tab" aria-selected="false" aria-controls="tab-tree">Widget Tree</button>
</div>

<div id="overlay" role="status" aria-live="polite"></div>

<div id="tab-visual" class="tab-content active" role="tabpanel" aria-labelledby="tab-button-visual" aria-label="Static visual approximation"></div>
<div id="tab-tree" class="tab-content" role="tabpanel" aria-labelledby="tab-button-tree"></div>

<script nonce="${nonce}">
(function () {
  const vscode = acquireVsCodeApi();

  /* ── Tab switching ─────────────────────────────────────────────── */
  const tabBar = document.querySelector('.tab-bar');
  const tabs = document.querySelectorAll('.tab-content');

  function activateTab(btn) {
    if (!btn) return;
    tabBar.querySelectorAll('button').forEach(b => {
      b.classList.remove('active');
      b.setAttribute('aria-selected', 'false');
    });
    btn.classList.add('active');
    btn.setAttribute('aria-selected', 'true');
    tabs.forEach(t => t.classList.remove('active'));
    document.getElementById('tab-' + btn.dataset.tab).classList.add('active');
    hideOverlay();
  }

  tabBar.addEventListener('click', (e) => {
    const btn = e.target.closest('button[data-tab]');
    activateTab(btn);
  });
  tabBar.addEventListener('keydown', (e) => {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(e.key)) return;
    const buttons = Array.from(tabBar.querySelectorAll('button[data-tab]'));
    const current = buttons.indexOf(document.activeElement);
    if (current < 0) return;
    e.preventDefault();
    let next = current;
    if (e.key === 'Home') next = 0;
    else if (e.key === 'End') next = buttons.length - 1;
    else if (e.key === 'ArrowRight') next = (current + 1) % buttons.length;
    else next = (current - 1 + buttons.length) % buttons.length;
    buttons[next].focus();
    activateTab(buttons[next]);
  });

  /* ── Overlay helpers ───────────────────────────────────────────── */
  const overlay = document.getElementById('overlay');

  function showOverlay(text, isError) {
    overlay.textContent = text;
    overlay.className = 'visible' + (isError ? ' error' : '');
    overlay.style.display = 'block';
    overlay.setAttribute('role', isError ? 'alert' : 'status');
  }

  function hideOverlay() {
    overlay.style.display = 'none';
    overlay.className = '';
  }

  /* ── Normalise widget type to a CSS class suffix ───────────────── */
  function normaliseCssType(type) {
    const text = typeof type === 'string' ? type : '';
    return text.replace(/::/g, '-').toLowerCase().replace(/[^a-z0-9_-]/g, '-');
  }

  function finiteNumber(value, minimum, maximum) {
    const text = String(value ?? '').trim();
    if (!/^[+-]?(?:\\d+(?:\\.\\d*)?|\\.\\d+)$/.test(text)) return undefined;
    const number = Number(text);
    if (!Number.isFinite(number) || number < minimum || number > maximum) return undefined;
    return number;
  }

  function finiteInteger(value, minimum, maximum) {
    const number = finiteNumber(value, minimum, maximum);
    return number !== undefined && Number.isInteger(number) ? number : undefined;
  }

  function literalValue(option) {
    if (option && typeof option === 'object' && typeof option.value === 'string') {
      return option.value;
    }
    return typeof option === 'string' ? option : '';
  }

  function optionValue(options, name, fallback) {
    const value = literalValue((options || {})[name]);
    return value === '' ? (fallback || '') : value;
  }

  function placementOptions(widget) {
    const values = {};
    const options = (widget.geometry && widget.geometry.options) || {};
    for (const [name, option] of Object.entries(options)) {
      values[name.replace(/^-/, '')] = literalValue(option);
    }
    return values;
  }

  function placementManager(widget) {
    const manager = widget && widget.geometry && widget.geometry.manager;
    return typeof manager === 'string' ? manager.toLowerCase() : '';
  }

  function lexicalParent(pathname) {
    if (typeof pathname !== 'string' || pathname === '.') return '';
    const index = pathname.lastIndexOf('.');
    return index > 0 ? pathname.slice(0, index) : '.';
  }

  function placementContainer(widget) {
    const container = widget && widget.geometry && widget.geometry.container;
    return typeof container === 'string' ? container : undefined;
  }

  function placementAppliesIn(widget, parentPath) {
    const placement = widget && widget.geometry;
    if (!placement || widget.certainty !== 'certain' || placement.certainty !== 'certain') return false;
    const container = placementContainer(widget);
    // A missing container means an explicit -in value was dynamic or invalid.
    // The model always supplies the lexical parent for an implicit/default
    // container, so absence is an abstention rather than permission to guess.
    return typeof container === 'string' && container === parentPath;
  }

  function factBadges(widget, parentPath) {
    let html = '';
    if (widget.certainty === 'potential') {
      html += '<span class="tk-fact-badge" title="Execution or final widget state is not statically proven">potential</span>';
    }
    if (widget.geometry && widget.geometry.certainty === 'potential') {
      html += '<span class="tk-fact-badge" title="This geometry-manager call may never execute; no visual placement is asserted">potential geometry</span>';
    }
    const container = placementContainer(widget);
    if (container && parentPath && container !== parentPath) {
      html += '<span class="tk-fact-badge" title="The -in geometry container differs from the pathname parent">managed in '
        + escapeHtml(container) + '</span>';
    }
    if (widget.geometry && typeof container !== 'string') {
      html += '<span class="tk-fact-badge" title="The effective -in container is dynamic or invalid; lexical placement is not inferred">unresolved geometry container</span>';
    }
    return html;
  }

  /* ── Determine the display text for a widget ───────────────────── */
  function widgetText(widget) {
    return optionValue(widget.options, '-text', optionValue(widget.options, 'text', ''));
  }

  /* ── Build styled HTML for a single widget (no children yet) ──── */
  function renderWidgetContent(widget) {
    const type = (widget.constructor || '').toLowerCase();
    const text = widgetText(widget);

    switch (type) {
      case 'button':
      case 'ttk::button':
        return '<div class="tk-button">' + escapeHtml(text || 'Button') + '</div>';

      case 'entry':
      case 'ttk::entry':
        return '<div class="tk-' + normaliseCssType(type) + '">' + escapeHtml(text) + '</div>';

      case 'label':
      case 'ttk::label':
        return '<div class="tk-' + normaliseCssType(type) + '">' + escapeHtml(text || 'Label') + '</div>';

      case 'frame':
      case 'ttk::frame':
        return '';  // frame is just a container; children rendered separately

      case 'labelframe':
      case 'ttk::labelframe':
        return '<span class="tk-' + normaliseCssType(type) + '-text">'
             + escapeHtml(text || '') + '</span>';

      case 'text':
        return '<div class="tk-text"></div>';

      case 'listbox':
        return '<div class="tk-listbox"><span class="tk-fact-badge">items are not statically modeled</span></div>';

      case 'canvas':
        return '<div class="tk-canvas"></div>';

      case 'scrollbar':
      case 'ttk::scrollbar': {
        const requested = optionValue(widget.options, '-orient', 'vertical').toLowerCase();
        const orient = requested === 'horizontal' ? 'horizontal' : 'vertical';
        return '<div class="tk-scrollbar ' + orient + '"></div>';
      }

      case 'checkbutton':
      case 'ttk::checkbutton':
        return '<div class="tk-checkbutton">' + escapeHtml(text || 'Check') + '</div>';

      case 'radiobutton':
      case 'ttk::radiobutton':
        return '<div class="tk-radiobutton">' + escapeHtml(text || 'Radio') + '</div>';

      case 'scale':
      case 'ttk::scale':
        return '<div class="tk-scale">'
             + '<div class="tk-scale-track"><div class="tk-scale-thumb"></div></div>'
             + '</div>';

      case 'ttk::combobox':
        return '<div class="tk-ttk-combobox">'
             + '<span class="tk-ttk-combobox-text">' + escapeHtml(text) + '</span>'
             + '<span class="tk-ttk-combobox-arrow">&#9660;</span>'
             + '</div>';

      case 'ttk::treeview':
        return '<div class="tk-ttk-treeview">'
             + '<div class="tk-ttk-treeview-heading">Treeview</div>'
             + '<div class="tk-fact-badge">columns and items are not statically modeled</div>'
             + '</div>';

      case 'ttk::notebook':
        return '';  // handled specially in renderWidget

      case 'ttk::progressbar':
        return '<div class="tk-ttk-progressbar">'
             + '<span class="tk-fact-badge">value is not statically modeled</span>'
             + '</div>';

      case 'ttk::separator': {
        const requested = optionValue(widget.options, '-orient', 'horizontal').toLowerCase();
        const orient2 = requested === 'vertical' ? 'vertical' : 'horizontal';
        return '<div class="tk-ttk-separator ' + orient2 + '"></div>';
      }

      case 'menu':
        return '';  // rendered as menu bar

      default:
        return '<div class="tk-label">' + escapeHtml(type + (text ? ': ' + text : '')) + '</div>';
    }
  }

  /* ── Build sticky / alignment style string for grid items ──────── */
  function stickyStyle(sticky) {
    if (!sticky) return '';
    const s = sticky.toLowerCase();
    if (!/^[nsew]*$/.test(s)) return '';
    const styles = [];

    const hasN = s.includes('n');
    const hasS = s.includes('s');
    const hasE = s.includes('e');
    const hasW = s.includes('w');

    /* Vertical alignment */
    if (hasN && hasS) styles.push('align-self:stretch');
    else if (hasN)    styles.push('align-self:start');
    else if (hasS)    styles.push('align-self:end');
    else              styles.push('align-self:center');

    /* Horizontal alignment */
    if (hasW && hasE) styles.push('justify-self:stretch');
    else if (hasW)    styles.push('justify-self:start');
    else if (hasE)    styles.push('justify-self:end');
    else              styles.push('justify-self:center');

    return styles.join(';');
  }

  /* ── Compute inline style for a child based on geometry ────────── */
  function childStyle(widget, parentPath) {
    const parts = [];
    if (!placementAppliesIn(widget, parentPath)) return '';
    const manager = placementManager(widget);
    const geometry = placementOptions(widget);
    const geometrySource = widget.geometry && widget.geometry.source;

    /* Grid placement */
    if (manager === 'grid') {
      const g = geometry;
      const row = finiteInteger(g.row, 0, 100000);
      const column = finiteInteger(g.column, 0, 100000);
      const rowspan = finiteInteger(g.rowspan, 1, 100000);
      const columnspan = finiteInteger(g.columnspan, 1, 100000);
      if (row !== undefined)       parts.push('grid-row:' + (row + 1));
      if (column !== undefined)    parts.push('grid-column:' + (column + 1));
      if (rowspan !== undefined && rowspan > 1)
        parts.push('grid-row-end:span ' + rowspan);
      if (columnspan !== undefined && columnspan > 1)
        parts.push('grid-column-end:span ' + columnspan);
      const stk = stickyStyle(g.sticky);
      if (stk) parts.push(stk);
      const padx = finiteNumber(g.padx, 0, 100000);
      const pady = finiteNumber(g.pady, 0, 100000);
      if (padx !== undefined) parts.push('padding-left:' + padx + 'px;padding-right:' + padx + 'px');
      if (pady !== undefined) parts.push('padding-top:' + pady + 'px;padding-bottom:' + pady + 'px');
    }

    /* Pack placement */
    if (manager === 'pack') {
      const p = geometry;
      const side = ['top', 'bottom', 'left', 'right'].includes((p.side || '').toLowerCase())
        ? p.side.toLowerCase() : 'top';
      const horizontal = side === 'left' || side === 'right';
      if (p.expand && p.expand !== '0' && p.expand !== 'false') parts.push('flex:1');
      if (p.fill === 'both' || (horizontal && p.fill === 'y') || (!horizontal && p.fill === 'x')) {
        parts.push('align-self:stretch');
      }
      if ((horizontal && (p.fill === 'x' || p.fill === 'both'))
          || (!horizontal && (p.fill === 'y' || p.fill === 'both'))) {
        parts.push('flex-grow:1');
      }
      const padx = finiteNumber(p.padx, 0, 100000);
      const pady = finiteNumber(p.pady, 0, 100000);
      if (padx !== undefined) parts.push('margin-left:' + padx + 'px;margin-right:' + padx + 'px');
      if (pady !== undefined) parts.push('margin-top:' + pady + 'px;margin-bottom:' + pady + 'px');
    }

    /* Place positioning */
    if (manager === 'place') {
      const pl = geometry;
      const x = finiteNumber(pl.x, -1000000, 1000000);
      const y = finiteNumber(pl.y, -1000000, 1000000);
      const width = finiteNumber(pl.width, 0, 1000000);
      const height = finiteNumber(pl.height, 0, 1000000);
      const relx = finiteNumber(pl.relx, -1000, 1000);
      const rely = finiteNumber(pl.rely, -1000, 1000);
      const relwidth = finiteNumber(pl.relwidth, 0, 1000);
      const relheight = finiteNumber(pl.relheight, 0, 1000);
      if (relx !== undefined && x !== undefined)
        parts.push('left:calc(' + (relx * 100) + '% + ' + x + 'px)');
      else if (relx !== undefined) parts.push('left:' + (relx * 100) + '%');
      else if (x !== undefined) parts.push('left:' + x + 'px');
      if (rely !== undefined && y !== undefined)
        parts.push('top:calc(' + (rely * 100) + '% + ' + y + 'px)');
      else if (rely !== undefined) parts.push('top:' + (rely * 100) + '%');
      else if (y !== undefined) parts.push('top:' + y + 'px');
      if (relwidth !== undefined && width !== undefined)
        parts.push('width:calc(' + (relwidth * 100) + '% + ' + width + 'px)');
      else if (relwidth !== undefined) parts.push('width:' + (relwidth * 100) + '%');
      else if (width !== undefined) parts.push('width:' + width + 'px');
      if (relheight !== undefined && height !== undefined)
        parts.push('height:calc(' + (relheight * 100) + '% + ' + height + 'px)');
      else if (relheight !== undefined) parts.push('height:' + (relheight * 100) + '%');
      else if (height !== undefined) parts.push('height:' + height + 'px');
      const anchors = {
        center: '-50%,-50%', n: '-50%,0', ne: '-100%,0', e: '-100%,-50%',
        se: '-100%,-100%', s: '-50%,-100%', sw: '0,-100%', w: '0,-50%', nw: '0,0'
      };
      const anchor = anchors[(pl.anchor || 'nw').toLowerCase()];
      if (anchor && anchor !== '0,0') parts.push('transform:translate(' + anchor + ')');
    }

    return parts.join(';');
  }

  function renderChildInContainer(child, parentPath) {
    if (placementAppliesIn(child, parentPath)) {
      return '<div style="' + childStyle(child, parentPath) + '">'
        + renderWidget(child, parentPath) + '</div>';
    }
    return '<div class="tk-layout-abstained">'
      + '<span class="tk-fact-badge">layout not statically asserted</span>'
      + renderWidget(child, parentPath) + '</div>';
  }

  /* ── Determine the geometry container class for a widget ────────── */
  function geoContainerClass(widget) {
    const managers = new Set((widget.children || [])
      .filter(child => placementAppliesIn(child, widget.path))
      .map(placementManager).filter(Boolean));
    const gm = managers.size === 1 ? Array.from(managers)[0] : '';
    if (gm === 'grid')  return 'geo-grid';
    if (gm === 'pack')  return 'geo-pack';
    if (gm === 'place') return 'geo-place';
    return '';
  }

  /* ── Determine pack direction class ────────────────────────────── */
  function packDirectionClass(widget) {
    const children = (widget.children || []).filter(child => placementAppliesIn(child, widget.path));
    for (const child of children) {
      if (placementManager(child) === 'pack') {
        const requested = (placementOptions(child).side || 'top').toLowerCase();
        const side = ['top', 'bottom', 'left', 'right'].includes(requested) ? requested : 'top';
        return 'pack-' + side;
      }
    }
    return 'pack-top';
  }

  /* ── Render a widget and its children recursively ──────────────── */
  function renderWidget(widget, parentPath) {
    const type = typeof widget.constructor === 'string' ? widget.constructor.toLowerCase() : '';
    const cssType = normaliseCssType(type);
    const children = widget.children || [];
    const geoClass = geoContainerClass(widget);
    const packDir = packDirectionClass(widget);
    const badges = factBadges(widget, parentPath);

    /* Toplevel window */
    if (type === 'root' || type === 'toplevel') {
      let html = '<div class="tk-toplevel">';
      html += '<div class="tk-toplevel-title">' + escapeHtml(widget.path || 'Tk root')
        + ' — title not statically modeled</div>';
      html += badges;
      html += '<div class="' + [geoClass, packDir].filter(Boolean).join(' ') + '">';
      for (const child of children) {
        html += renderChildInContainer(child, widget.path);
      }
      html += '</div></div>';
      return html;
    }

    /* Menu bar */
    if (type === 'menu') {
      let html = '<div class="tk-menu">';
      html += badges;
      html += '<div class="tk-menu-item">Menu entries are not statically modeled</div>';
      html += '</div>';
      return html;
    }

    /* Notebook (tabbed container) */
    if (type === 'ttk::notebook') {
      let html = '<div class="tk-ttk-notebook">';
      html += badges;
      html += '<div class="tk-ttk-notebook-tabs"><div class="tk-ttk-notebook-tab">'
        + 'Tab membership requires a notebook add/insert fact</div></div>';
      html += '<div class="tk-ttk-notebook-body">';
      if (children.length > 0) {
        html += '<span class="tk-fact-badge">pathname children, not asserted tabs</span>';
        for (const child of children) {
          html += renderWidget(child, widget.path);
        }
      }
      html += '</div></div>';
      return html;
    }

    /* Frame / labelframe containers */
    if (type === 'frame' || type === 'ttk::frame' ||
        type === 'labelframe' || type === 'ttk::labelframe') {
      let html = '<div class="tk-' + cssType + '">';
      html += badges;
      if (type === 'labelframe' || type === 'ttk::labelframe') {
        html += renderWidgetContent(widget);
      }
      if (children.length > 0) {
        html += '<div class="' + [geoClass, packDir].filter(Boolean).join(' ') + '">';
        for (const child of children) {
          html += renderChildInContainer(child, widget.path);
        }
        html += '</div>';
      }
      html += '</div>';
      return html;
    }

    /* Leaf widgets (no children expected, but handle gracefully) */
    let html = badges + renderWidgetContent(widget);
    if (children.length > 0) {
      html += '<div class="' + [geoClass, packDir].filter(Boolean).join(' ') + '">';
      for (const child of children) {
        html += renderChildInContainer(child, widget.path);
      }
      html += '</div>';
    }
    return html;
  }

  /* ── Render the widget tree tab (hierarchical text view) ────────── */
  function sourceAttributes(source, label) {
    if (!source || !Number.isSafeInteger(source.start) || !Number.isSafeInteger(source.end)
        || source.start < 0 || source.end <= source.start) return '';
    return ' role="button" tabindex="0" data-source-start="' + source.start
      + '" data-source-end="' + source.end + '" aria-label="'
      + escapeHtml(label) + '"';
  }

  function renderTreeNode(widget, isRoot, parentPath) {
    const type = typeof widget.constructor === 'string' ? widget.constructor : 'unknown';
    const pathname = widget.path || '';
    const children = widget.children || [];
    const opts = widget.options || {};
    const manager = placementManager(widget);
    const geometry = placementOptions(widget);
    const geometrySource = widget.geometry && widget.geometry.source;
    const pathSource = (widget.source && widget.source.path) || {};

    let html = '<div class="tree-node' + (isRoot ? ' tree-root' : '') + '">';
    const sourceAttrs = sourceAttributes(pathSource, 'Reveal source for ' + (pathname || type));
    html += '<div class="tree-node-header"' + sourceAttrs + '>';
    html += '<span class="tree-node-type">' + escapeHtml(type) + '</span>';
    html += '<span class="tree-node-path">' + escapeHtml(pathname) + '</span>';
    html += factBadges(widget, parentPath);

    /* Show geometry info */
    const geometryAttrs = sourceAttributes(geometrySource, 'Reveal geometry source for ' + pathname);
    if (manager === 'grid') {
      const g = geometry;
      let geo = 'grid(' + (g.row || 0) + ',' + (g.column || 0) + ')';
      if (g.sticky) geo += ' sticky=' + g.sticky;
      html += '<span class="tree-node-geo source-link"' + geometryAttrs + '>' + escapeHtml(geo) + '</span>';
    } else if (manager === 'pack') {
      const p = geometry;
      let geo = 'pack';
      if (p.side) geo += ' side=' + p.side;
      if (p.fill) geo += ' fill=' + p.fill;
      html += '<span class="tree-node-geo source-link"' + geometryAttrs + '>' + escapeHtml(geo) + '</span>';
    } else if (manager === 'place') {
      const pl = geometry;
      let geo = 'place';
      if (pl.x !== undefined) geo += ' x=' + pl.x;
      if (pl.y !== undefined) geo += ' y=' + pl.y;
      html += '<span class="tree-node-geo source-link"' + geometryAttrs + '>' + escapeHtml(geo) + '</span>';
    }
    const container = placementContainer(widget);
    if (container) {
      html += '<span class="tree-node-geo">in=' + escapeHtml(container) + '</span>';
    }

    /* Show a few key options */
    const interestingOpts = ['-text', '-textvariable', '-command', '-variable', '-width', '-height'];
    const shown = [];
    for (const key of interestingOpts) {
      if (opts[key] !== undefined) {
        shown.push(key + '=' + literalValue(opts[key]));
      }
    }
    if (shown.length > 0) {
      html += '<span class="tree-node-opts">' + escapeHtml(shown.join(', ')) + '</span>';
    }

    html += '</div>';  // header

    for (const child of children) {
      html += renderTreeNode(child, false, pathname);
    }
    html += '</div>';
    return html;
  }

  /* ── HTML escaping utility ─────────────────────────────────────── */
  function escapeHtml(str) {
    if (!str) return '';
    return String(str)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  function revealSourceTarget(target) {
    if (!target) return;
    const start = Number(target.getAttribute('data-source-start'));
    const end = Number(target.getAttribute('data-source-end'));
    if (Number.isFinite(start) && Number.isFinite(end) && end >= start) {
      vscode.postMessage({ type: 'revealSource', start, end });
    }
  }

  document.addEventListener('click', (event) => {
    const target = event.target instanceof Element
      ? event.target.closest('[data-source-start][data-source-end]')
      : null;
    revealSourceTarget(target);
  });
  document.addEventListener('keydown', (event) => {
    if (event.key !== 'Enter' && event.key !== ' ') return;
    const target = event.target instanceof Element
      ? event.target.closest('[data-source-start][data-source-end]')
      : null;
    if (!target) return;
    event.preventDefault();
    revealSourceTarget(target);
  });

  /* ── Message handler ───────────────────────────────────────────── */
  window.addEventListener('message', (event) => {
    const msg = event.data;
    if (!msg || !msg.type) return;

    switch (msg.type) {
      case 'layout': {
        hideOverlay();
        const data = msg.data;
        const root = data.root;
        if (!data.tk_active || !root) {
          showOverlay('No statically active Tk application was found in this document.', false);
          return;
        }

        /* Visual preview.  Build the complete fragment before assigning it;
         * repeated innerHTML += reparses the entire widget tree for each
         * warning/count block and makes large generated UIs quadratic. */
        const visualTab = document.getElementById('tab-visual');
        let visualHtml = renderWidget(root);

        /* Show geometry conflicts if any */
        if (data.geometry_conflicts && data.geometry_conflicts.length > 0) {
          let warnings = '<div style="margin-top:12px;padding:8px;background:#fff3cd;border:1px solid #ffc107;border-radius:4px;font-size:12px;">';
          warnings += '<strong>Geometry conflicts:</strong><ul style="margin:4px 0 0 16px;">';
          for (const conflict of data.geometry_conflicts) {
            const text = conflict && typeof conflict === 'object'
              ? 'Container ' + (conflict.container || '?') + ' is claimed by '
                + ((conflict.managers || []).join(' and ') || 'multiple managers')
              : String(conflict);
            warnings += '<li>' + escapeHtml(text);
            if (conflict && Array.isArray(conflict.placements)) {
              warnings += '<ul>';
              for (const placement of conflict.placements) {
                const label = (placement.widget || '?') + ' via ' + (placement.manager || '?');
                const attrs = sourceAttributes(placement.source, 'Reveal conflicting geometry source for ' + label);
                warnings += '<li><span class="source-link"' + attrs + '>'
                  + escapeHtml(label) + '</span></li>';
              }
              warnings += '</ul>';
            }
            warnings += '</li>';
          }
          warnings += '</ul></div>';
          visualHtml += warnings;
        }

        if (data.uncertainties && data.uncertainties.length > 0) {
          const omitted = Number.isSafeInteger(data.uncertainties_truncated)
            && data.uncertainties_truncated > 0
            ? data.uncertainties_truncated
            : 0;
          const total = data.uncertainties.length + omitted;
          let unresolved = '<div style="margin-top:8px;font-size:11px;color:var(--vscode-descriptionForeground,#888);">'
            + total
            + ' dynamic or unsupported layout fact(s) were left unresolved; see the widget tree and diagnostics.'
            + (omitted > 0 ? ' Showing the first ' + data.uncertainties.length + '.' : '')
            + '</div>';
          unresolved += '<ul class="uncertainty-list">';
          for (const item of data.uncertainties) {
            const kind = item && item.kind ? String(item.kind).replace(/_/g, ' ') : 'unresolved fact';
            const message = item && item.message ? String(item.message) : kind;
            const attrs = sourceAttributes(item && item.source, 'Reveal source for ' + kind);
            unresolved += '<li><span class="source-link"' + attrs + '><strong>'
              + escapeHtml(kind) + ':</strong> ' + escapeHtml(message) + '</span></li>';
          }
          unresolved += '</ul>';
          visualHtml += unresolved;
        }

        /* Widget count */
        const widgetCount = Number.isSafeInteger(data.widget_count) && data.widget_count >= 0
          ? data.widget_count
          : undefined;
        if (widgetCount !== undefined) {
          const truncatedWidgets = Number.isSafeInteger(data.widgets_truncated)
            && data.widgets_truncated > 0 ? data.widgets_truncated : 0;
          visualHtml += '<div style="margin-top:8px;font-size:11px;color:var(--vscode-descriptionForeground,#888);">'
            + widgetCount + ' statically recognised widget(s)'
            + (truncatedWidgets > 0 ? '; ' + truncatedWidgets + ' omitted from this bounded preview' : '')
            + '</div>';
        }
        visualTab.innerHTML = visualHtml;

        /* Tree view */
        const treeTab = document.getElementById('tab-tree');
        let treeHtml = renderTreeNode(root, true, '');
        if (Array.isArray(data.orphan_widgets) && data.orphan_widgets.length > 0) {
          treeHtml += '<h3 style="margin:12px 0 4px">Unresolved parents</h3>';
          treeHtml += data.orphan_widgets
            .map((orphan) => renderTreeNode(orphan, false, lexicalParent(orphan.path)))
            .join('');
        }
        treeTab.innerHTML = treeHtml;
        break;
      }

      case 'status':
        showOverlay(msg.text || 'Loading...', false);
        break;

      case 'error':
        showOverlay(msg.message || 'An error occurred.', true);
        break;

      case 'empty':
        showOverlay('No Tk widgets detected. Ensure the file contains "package require Tk".', false);
        break;

      case 'unavailable':
        showOverlay(msg.message || 'Tk preview is unavailable for this editor.', false);
        break;
    }
  });

  /* ── Signal readiness to the extension host ────────────────────── */
  vscode.postMessage({ type: 'ready' });
})();
</script>
</body>
</html>`;
}
