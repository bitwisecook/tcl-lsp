/**
 * Generates the HTML content for the iRule diagram webview panel.
 *
 * Renders a Mermaid flowchart inside a VS Code webview tab using the
 * Mermaid library loaded from CDN.  Falls back to showing the raw
 * Mermaid source if the library cannot be loaded (e.g. offline).
 */
export function getDiagramPanelHtml(): string {
  return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta http-equiv="Content-Security-Policy"
  content="default-src 'none';
           style-src 'unsafe-inline';
           script-src 'unsafe-inline' https://cdn.jsdelivr.net;
           img-src data:;">
<title>iRule Diagram</title>
<style>
:root {
  --bg: var(--vscode-editor-background, #1e1e2e);
  --text: var(--vscode-editor-foreground, #cdd6f4);
  --text-dim: var(--vscode-descriptionForeground, #6c7086);
  --border: var(--vscode-panel-border, #313244);
  --accent: var(--vscode-textLink-foreground, #89b4fa);
}
* { margin: 0; padding: 0; box-sizing: border-box; }
html, body {
  height: 100%;
  background: var(--bg);
  color: var(--text);
  font-family: var(--vscode-font-family, system-ui, sans-serif);
  font-size: var(--vscode-font-size, 13px);
  overflow: hidden;
}
#toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--border);
  background: var(--bg);
}
#toolbar button {
  background: var(--vscode-button-background, #45475a);
  color: var(--vscode-button-foreground, #cdd6f4);
  border: none;
  border-radius: 3px;
  padding: 4px 10px;
  cursor: pointer;
  font-size: inherit;
}
#toolbar button:hover {
  background: var(--vscode-button-hoverBackground, #585b70);
}
#toolbar .spacer { flex: 1; }
#toolbar .title {
  font-weight: 600;
  font-size: 1.05em;
}
#container {
  height: calc(100% - 37px);
  overflow: auto;
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding: 16px;
}
#diagram {
  max-width: 100%;
}
#diagram svg {
  max-width: 100%;
  height: auto;
}
#error {
  display: none;
  padding: 16px;
  color: var(--vscode-errorForeground, #f38ba8);
}
#raw {
  display: none;
  padding: 16px;
  white-space: pre-wrap;
  font-family: var(--vscode-editor-font-family, monospace);
  font-size: var(--vscode-editor-font-size, 13px);
  background: var(--vscode-textCodeBlock-background, #181825);
  border-radius: 4px;
  margin: 16px;
  overflow: auto;
}
.loading {
  color: var(--text-dim);
  padding: 32px;
  text-align: center;
}
</style>
</head>
<body>
<div id="toolbar">
  <span class="title">iRule Diagram</span>
  <span class="spacer"></span>
  <button id="btnZoomIn" title="Zoom in">+</button>
  <button id="btnZoomReset" title="Reset zoom">100%</button>
  <button id="btnZoomOut" title="Zoom out">&minus;</button>
  <button id="btnCopy" title="Copy Mermaid source">Copy</button>
</div>
<div id="container">
  <div id="diagram" class="loading">Loading diagram&hellip;</div>
  <div id="error"></div>
  <pre id="raw"></pre>
</div>
<script type="module">
const vscode = acquireVsCodeApi();
const diagramEl = document.getElementById('diagram');
const errorEl = document.getElementById('error');
const rawEl = document.getElementById('raw');
const btnCopy = document.getElementById('btnCopy');
const btnZoomIn = document.getElementById('btnZoomIn');
const btnZoomOut = document.getElementById('btnZoomOut');
const btnZoomReset = document.getElementById('btnZoomReset');

let currentSource = '';
let scale = 1;

function applyZoom() {
  diagramEl.style.transform = 'scale(' + scale + ')';
  diagramEl.style.transformOrigin = 'top center';
  btnZoomReset.textContent = Math.round(scale * 100) + '%';
}
btnZoomIn.addEventListener('click', () => { scale = Math.min(scale + 0.25, 4); applyZoom(); });
btnZoomOut.addEventListener('click', () => { scale = Math.max(scale - 0.25, 0.25); applyZoom(); });
btnZoomReset.addEventListener('click', () => { scale = 1; applyZoom(); });
btnCopy.addEventListener('click', () => {
  if (currentSource) {
    navigator.clipboard.writeText(currentSource).then(
      () => { btnCopy.textContent = 'Copied!'; setTimeout(() => { btnCopy.textContent = 'Copy'; }, 1500); },
      () => {}
    );
  }
});

async function renderDiagram(source) {
  currentSource = source;
  diagramEl.className = '';
  diagramEl.innerHTML = '';
  errorEl.style.display = 'none';
  rawEl.style.display = 'none';

  try {
    const { default: mermaid } = await import(
      'https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs'
    );

    // Detect VS Code theme.
    const isDark = document.body.classList.contains('vscode-dark') ||
                   document.body.classList.contains('vscode-high-contrast');
    mermaid.initialize({
      startOnLoad: false,
      theme: isDark ? 'dark' : 'default',
      securityLevel: 'strict',
      flowchart: { useMaxWidth: true, htmlLabels: true, curve: 'basis' },
    });

    const { svg } = await mermaid.render('mermaid-graph', source);
    diagramEl.innerHTML = svg;
  } catch (err) {
    // Fallback: show raw Mermaid source.
    errorEl.textContent = 'Failed to render diagram: ' + (err.message || err);
    errorEl.style.display = 'block';
    rawEl.textContent = source;
    rawEl.style.display = 'block';
  }
}

window.addEventListener('message', (event) => {
  const msg = event.data;
  if (msg.type === 'setDiagram' && msg.source) {
    renderDiagram(msg.source);
  }
});

// Request the diagram source from the extension host.
vscode.postMessage({ type: 'ready' });
</script>
</body>
</html>`;
}
