/**
 * Returns the full HTML content for the Tk Preview webview panel.
 *
 * The webview renders a visual approximation of Tk widgets using HTML/CSS,
 * supporting grid, pack, and place geometry managers. It provides two tabs:
 * "Visual Preview" (rendered widgets) and "Widget Tree" (hierarchical view).
 *
 * Message protocol (extension → webview):
 *   { type: "layout", data: WidgetTree }  — render the widget tree
 *   { type: "status", text: string }       — show a status message
 *   { type: "error",  message: string }    — show an error message
 *   { type: "empty" }                      — show "no Tk content" placeholder
 *
 * Message protocol (webview → extension):
 *   { type: "ready" }                      — webview has finished loading
 */
export function getTkPreviewHtml(): string {
  return /* html */ `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=device-width, initial-scale=1.0" />
<title>Tk Preview</title>
<style>
  /* ── Reset & base ──────────────────────────────────────────────── */
  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

  body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
                 Helvetica, Arial, sans-serif;
    font-size: 13px;
    colour: var(--vscode-foreground, #333);
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
    width: 40%;
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
</style>
</head>
<body>

<div class="tab-bar">
  <button class="active" data-tab="visual">Visual Preview</button>
  <button data-tab="tree">Widget Tree</button>
</div>

<div id="overlay"></div>

<div id="tab-visual" class="tab-content active"></div>
<div id="tab-tree" class="tab-content"></div>

<script>
(function () {
  const vscode = acquireVsCodeApi();

  /* ── Tab switching ─────────────────────────────────────────────── */
  const tabBar = document.querySelector('.tab-bar');
  const tabs = document.querySelectorAll('.tab-content');

  tabBar.addEventListener('click', (e) => {
    const btn = e.target.closest('button[data-tab]');
    if (!btn) return;
    tabBar.querySelectorAll('button').forEach(b => b.classList.remove('active'));
    btn.classList.add('active');
    tabs.forEach(t => t.classList.remove('active'));
    document.getElementById('tab-' + btn.dataset.tab).classList.add('active');
    hideOverlay();
  });

  /* ── Overlay helpers ───────────────────────────────────────────── */
  const overlay = document.getElementById('overlay');

  function showOverlay(text, isError) {
    overlay.textContent = text;
    overlay.className = 'visible' + (isError ? ' error' : '');
    overlay.style.display = 'block';
  }

  function hideOverlay() {
    overlay.style.display = 'none';
    overlay.className = '';
  }

  /* ── Normalise widget type to a CSS class suffix ───────────────── */
  function normaliseCssType(type) {
    return type.replace(/::/g, '-').toLowerCase();
  }

  /* ── Determine the display text for a widget ───────────────────── */
  function widgetText(widget) {
    const opts = widget.options || {};
    return opts['-text'] || opts['text'] || '';
  }

  /* ── Build styled HTML for a single widget (no children yet) ──── */
  function renderWidgetContent(widget) {
    const type = (widget.type || '').toLowerCase();
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

      case 'listbox': {
        let items = '';
        for (let i = 1; i <= 3; i++) {
          items += '<div class="tk-listbox-item">Item ' + i + '</div>';
        }
        return '<div class="tk-listbox">' + items + '</div>';
      }

      case 'canvas':
        return '<div class="tk-canvas"></div>';

      case 'scrollbar':
      case 'ttk::scrollbar': {
        const orient = (widget.options || {})['-orient'] || 'vertical';
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
             + '<div class="tk-ttk-treeview-heading">Column</div>'
             + '<div class="tk-ttk-treeview-row">Row 1</div>'
             + '<div class="tk-ttk-treeview-row">Row 2</div>'
             + '</div>';

      case 'ttk::notebook':
        return '';  // handled specially in renderWidget

      case 'ttk::progressbar':
        return '<div class="tk-ttk-progressbar">'
             + '<div class="tk-ttk-progressbar-fill"></div>'
             + '</div>';

      case 'ttk::separator': {
        const orient2 = (widget.options || {})['-orient'] || 'horizontal';
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
  function childStyle(widget) {
    const parts = [];

    /* Grid placement */
    if (widget.grid) {
      const g = widget.grid;
      if (g.row !== undefined)       parts.push('grid-row:' + (parseInt(g.row, 10) + 1));
      if (g.column !== undefined)    parts.push('grid-column:' + (parseInt(g.column, 10) + 1));
      if (g.rowspan && g.rowspan > 1)
        parts.push('grid-row-end:span ' + g.rowspan);
      if (g.columnspan && g.columnspan > 1)
        parts.push('grid-column-end:span ' + g.columnspan);
      const stk = stickyStyle(g.sticky);
      if (stk) parts.push(stk);
      if (g.padx) parts.push('padding-left:' + g.padx + 'px;padding-right:' + g.padx + 'px');
      if (g.pady) parts.push('padding-top:' + g.pady + 'px;padding-bottom:' + g.pady + 'px');
    }

    /* Pack placement */
    if (widget.pack) {
      const p = widget.pack;
      if (p.expand) parts.push('flex:1');
      if (p.fill === 'both')      parts.push('align-self:stretch');
      else if (p.fill === 'x')    parts.push('align-self:stretch');
      else if (p.fill === 'y')    parts.push('align-self:stretch');
      if (p.padx) parts.push('margin-left:' + p.padx + 'px;margin-right:' + p.padx + 'px');
      if (p.pady) parts.push('margin-top:' + p.pady + 'px;margin-bottom:' + p.pady + 'px');
    }

    /* Place positioning */
    if (widget.place) {
      const pl = widget.place;
      if (pl.x !== undefined) parts.push('left:' + pl.x + 'px');
      if (pl.y !== undefined) parts.push('top:' + pl.y + 'px');
      if (pl.width)  parts.push('width:' + pl.width + 'px');
      if (pl.height) parts.push('height:' + pl.height + 'px');
      if (pl.relx !== undefined) parts.push('left:' + (pl.relx * 100) + '%');
      if (pl.rely !== undefined) parts.push('top:' + (pl.rely * 100) + '%');
      if (pl.relwidth !== undefined)  parts.push('width:' + (pl.relwidth * 100) + '%');
      if (pl.relheight !== undefined) parts.push('height:' + (pl.relheight * 100) + '%');
    }

    return parts.join(';');
  }

  /* ── Determine the geometry container class for a widget ────────── */
  function geoContainerClass(widget) {
    const gm = (widget.geometry_manager || '').toLowerCase();
    if (gm === 'grid')  return 'geo-grid';
    if (gm === 'pack')  return 'geo-pack';
    if (gm === 'place') return 'geo-place';
    return '';
  }

  /* ── Determine pack direction class ────────────────────────────── */
  function packDirectionClass(widget) {
    if ((widget.geometry_manager || '').toLowerCase() !== 'pack') return '';
    const children = widget.children || [];
    for (const child of children) {
      if (child.pack) {
        const side = (child.pack.side || 'top').toLowerCase();
        return 'pack-' + side;
      }
    }
    return 'pack-top';
  }

  /* ── Render a widget and its children recursively ──────────────── */
  function renderWidget(widget) {
    const type = (widget.type || '').toLowerCase();
    const cssType = normaliseCssType(type);
    const children = widget.children || [];
    const geoClass = geoContainerClass(widget);
    const packDir = packDirectionClass(widget);

    /* Toplevel window */
    if (type === 'toplevel') {
      const title = widget.title || 'Tk Window';
      let html = '<div class="tk-toplevel">';
      html += '<div class="tk-toplevel-title">' + escapeHtml(title) + '</div>';
      html += '<div class="' + [geoClass, packDir].filter(Boolean).join(' ') + '">';
      for (const child of children) {
        html += '<div style="' + childStyle(child) + '">' + renderWidget(child) + '</div>';
      }
      html += '</div></div>';
      return html;
    }

    /* Menu bar */
    if (type === 'menu') {
      let html = '<div class="tk-menu">';
      for (const child of children) {
        const label = widgetText(child) || child.pathname;
        html += '<div class="tk-menu-item">' + escapeHtml(label) + '</div>';
      }
      if (children.length === 0) {
        html += '<div class="tk-menu-item">File</div>';
        html += '<div class="tk-menu-item">Edit</div>';
        html += '<div class="tk-menu-item">Help</div>';
      }
      html += '</div>';
      return html;
    }

    /* Notebook (tabbed container) */
    if (type === 'ttk::notebook') {
      let html = '<div class="tk-ttk-notebook">';
      html += '<div class="tk-ttk-notebook-tabs">';
      for (const child of children) {
        const tabText = widgetText(child) || child.pathname || 'Tab';
        html += '<div class="tk-ttk-notebook-tab">' + escapeHtml(tabText) + '</div>';
      }
      if (children.length === 0) {
        html += '<div class="tk-ttk-notebook-tab">Tab 1</div>';
      }
      html += '</div>';
      html += '<div class="tk-ttk-notebook-body">';
      if (children.length > 0) {
        html += renderWidget(children[0]);
      }
      html += '</div></div>';
      return html;
    }

    /* Frame / labelframe containers */
    if (type === 'frame' || type === 'ttk::frame' ||
        type === 'labelframe' || type === 'ttk::labelframe') {
      let html = '<div class="tk-' + cssType + '">';
      if (type === 'labelframe' || type === 'ttk::labelframe') {
        html += renderWidgetContent(widget);
      }
      if (children.length > 0) {
        html += '<div class="' + [geoClass, packDir].filter(Boolean).join(' ') + '">';
        for (const child of children) {
          html += '<div style="' + childStyle(child) + '">' + renderWidget(child) + '</div>';
        }
        html += '</div>';
      }
      html += '</div>';
      return html;
    }

    /* Leaf widgets (no children expected, but handle gracefully) */
    let html = renderWidgetContent(widget);
    if (children.length > 0) {
      html += '<div class="' + [geoClass, packDir].filter(Boolean).join(' ') + '">';
      for (const child of children) {
        html += '<div style="' + childStyle(child) + '">' + renderWidget(child) + '</div>';
      }
      html += '</div>';
    }
    return html;
  }

  /* ── Render the widget tree tab (hierarchical text view) ────────── */
  function renderTreeNode(widget, isRoot) {
    const type = widget.type || 'unknown';
    const pathname = widget.pathname || '';
    const children = widget.children || [];
    const gm = widget.geometry_manager || '';
    const opts = widget.options || {};

    let html = '<div class="tree-node' + (isRoot ? ' tree-root' : '') + '">';
    html += '<div class="tree-node-header">';
    html += '<span class="tree-node-type">' + escapeHtml(type) + '</span>';
    html += '<span class="tree-node-path">' + escapeHtml(pathname) + '</span>';

    /* Show geometry info */
    if (widget.grid) {
      const g = widget.grid;
      let geo = 'grid(' + (g.row || 0) + ',' + (g.column || 0) + ')';
      if (g.sticky) geo += ' sticky=' + g.sticky;
      html += '<span class="tree-node-geo">' + escapeHtml(geo) + '</span>';
    } else if (widget.pack) {
      const p = widget.pack;
      let geo = 'pack';
      if (p.side) geo += ' side=' + p.side;
      if (p.fill) geo += ' fill=' + p.fill;
      html += '<span class="tree-node-geo">' + escapeHtml(geo) + '</span>';
    } else if (widget.place) {
      const pl = widget.place;
      let geo = 'place';
      if (pl.x !== undefined) geo += ' x=' + pl.x;
      if (pl.y !== undefined) geo += ' y=' + pl.y;
      html += '<span class="tree-node-geo">' + escapeHtml(geo) + '</span>';
    } else if (gm) {
      html += '<span class="tree-node-geo">manages: ' + escapeHtml(gm) + '</span>';
    }

    /* Show a few key options */
    const interestingOpts = ['-text', '-textvariable', '-command', '-variable', '-width', '-height'];
    const shown = [];
    for (const key of interestingOpts) {
      if (opts[key] !== undefined) {
        shown.push(key + '=' + opts[key]);
      }
    }
    if (shown.length > 0) {
      html += '<span class="tree-node-opts">' + escapeHtml(shown.join(', ')) + '</span>';
    }

    html += '</div>';  // header

    for (const child of children) {
      html += renderTreeNode(child, false);
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

  /* ── Message handler ───────────────────────────────────────────── */
  window.addEventListener('message', (event) => {
    const msg = event.data;
    if (!msg || !msg.type) return;

    switch (msg.type) {
      case 'layout': {
        hideOverlay();
        const data = msg.data;
        const root = data.root;
        if (!root) {
          showOverlay('No root widget found in the layout data.', false);
          return;
        }

        /* Visual preview */
        const visualTab = document.getElementById('tab-visual');
        visualTab.innerHTML = renderWidget(root);

        /* Show geometry conflicts if any */
        if (data.geometry_conflicts && data.geometry_conflicts.length > 0) {
          let warnings = '<div style="margin-top:12px;padding:8px;background:#fff3cd;border:1px solid #ffc107;border-radius:4px;font-size:12px;">';
          warnings += '<strong>Geometry conflicts:</strong><ul style="margin:4px 0 0 16px;">';
          for (const conflict of data.geometry_conflicts) {
            warnings += '<li>' + escapeHtml(conflict) + '</li>';
          }
          warnings += '</ul></div>';
          visualTab.innerHTML += warnings;
        }

        /* Widget count */
        if (data.widget_count !== undefined) {
          visualTab.innerHTML += '<div style="margin-top:8px;font-size:11px;color:var(--vscode-descriptionForeground,#888);">'
            + data.widget_count + ' widget(s)</div>';
        }

        /* Tree view */
        const treeTab = document.getElementById('tab-tree');
        treeTab.innerHTML = renderTreeNode(root, true);
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
    }
  });

  /* ── Signal readiness to the extension host ────────────────────── */
  vscode.postMessage({ type: 'ready' });
})();
</script>
</body>
</html>`;
}
