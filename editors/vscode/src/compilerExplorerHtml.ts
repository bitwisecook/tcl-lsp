/**
 * Generates the HTML content for the Tcl Compiler Explorer webview panel.
 *
 * This is the standalone explorer index.html adapted for VS Code webview:
 * - Pyodide web worker replaced with VS Code postMessage API
 * - Textarea is read-only (source comes from the active editor)
 * - Compilation routed through the extension to the LSP server
 *
 * Shared rendering logic lives in explorer-core.js, which is read at runtime
 * and inlined into the HTML (VS Code CSP does not allow external scripts).
 */
import { existsSync, readFileSync } from "fs";
import { join } from "path";

function findCoreJs(): string {
  // When built via Makefile, the file is copied next to the bundle.
  const bundled = join(__dirname, "explorer-core.js");
  if (existsSync(bundled)) {
    return bundled;
  }
  // Fallback: resolve from the source tree (dev / tsc-watch mode).
  // __dirname is  editors/vscode/out  →  walk up to repo root.
  const source = join(__dirname, "..", "..", "..", "explorer", "static", "explorer-core.js");
  if (existsSync(source)) {
    return source;
  }
  throw new Error(
    `explorer-core.js not found at ${bundled} or ${source}. ` +
      "Run 'make compile' or copy explorer/static/explorer-core.js to editors/vscode/out/.",
  );
}

export function getWebviewHtml(): string {
  const coreJs = readFileSync(findCoreJs(), "utf-8").replace(
    /[^\x00-\x7F]/g,
    (char) => `\\u${char.charCodeAt(0).toString(16).padStart(4, "0")}`,
  );
  const coreJsDataUri = `data:text/javascript;base64,${Buffer.from(coreJs, "utf8").toString("base64")}`;
  const nonce = String(Math.random()).replace(".", "") + String(Date.now());

  return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta http-equiv="Content-Security-Policy"
  content="default-src 'none'; style-src 'unsafe-inline'; script-src 'nonce-${nonce}' data:;">
<title>Tcl Compiler Explorer</title>
<style>
:root {
  --bg: #1e1e2e;
  --bg-surface: #181825;
  --bg-hover: #313244;
  --text: #cdd6f4;
  --text-dim: #6c7086;
  --text-bright: #f5e0dc;
  --accent: #89b4fa;
  --green: #a6e3a1;
  --yellow: #f9e2af;
  --red: #f38ba8;
  --magenta: #cba6f7;
  --cyan: #94e2d5;
  --blue: #89b4fa;
  --orange: #fab387;
  --border: #313244;
  --gutter: #45475a;
  --highlight: rgba(137, 180, 250, 0.08);
  --highlight-strong: rgba(137, 180, 250, 0.18);
  --font-mono: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', 'Consolas', monospace;
  --font-size: 13px;
}
* { margin: 0; padding: 0; box-sizing: border-box; }
html, body { height: 100%; overflow: hidden; }
body {
  background: var(--bg);
  color: var(--text);
  font-family: var(--font-mono);
  font-size: var(--font-size);
}
.app {
  display: grid;
  grid-template-rows: auto 1fr;
  grid-template-columns: minmax(0, 1fr);
  height: 100vh;
  width: 100%;
}
.toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
  padding: 8px 16px;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  min-height: 42px;
  min-width: 0;
}
.toolbar h1 {
  font-size: 14px;
  font-weight: 600;
  color: var(--accent);
  white-space: nowrap;
}
.toolbar .stats {
  font-size: 11px;
  color: var(--text-dim);
  margin-left: auto;
  white-space: nowrap;
}
.toolbar .stats .stat-value { color: var(--text); }
.toolbar select {
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 3px 8px;
  font-family: var(--font-mono);
  font-size: 11px;
  cursor: pointer;
  outline: none;
}
.toolbar select:focus { border-color: var(--accent); }
.toolbar .status-msg {
  font-size: 11px;
  color: var(--yellow);
  white-space: nowrap;
}
.main {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;
}
.output-panel {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
  min-width: 0;
}
.tab-bar {
  display: flex;
  flex-wrap: wrap;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  min-width: 0;
}
.tab {
  padding: 6px 10px;
  font-size: 12px;
  color: var(--text-dim);
  cursor: pointer;
  border-bottom: 2px solid transparent;
  white-space: nowrap;
  transition: color 0.15s, border-color 0.15s;
  user-select: none;
}
.tab:hover { color: var(--text); background: var(--bg-hover); }
.tab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}
.tab .badge {
  display: inline-block;
  background: var(--bg-hover);
  color: var(--text-dim);
  font-size: 10px;
  padding: 1px 5px;
  border-radius: 8px;
  margin-left: 4px;
  vertical-align: middle;
}
.tab.active .badge { background: rgba(137, 180, 250, 0.2); color: var(--accent); }
.output-content {
  flex: 1;
  overflow: auto;
  padding: 12px 16px;
}
.output-content::-webkit-scrollbar { width: 8px; }
.output-content::-webkit-scrollbar-track { background: transparent; }
.output-content::-webkit-scrollbar-thumb { background: var(--border); border-radius: 4px; }
.tab-pane { display: none; }
.tab-pane.active { display: block; }
.ir-tree { font-size: 12px; }
.ir-node { margin-left: 16px; }
.ir-node-header {
  padding: 2px 4px;
  border-radius: 3px;
  cursor: pointer;
  display: flex;
  align-items: baseline;
  gap: 6px;
}
.ir-node-header:hover { background: var(--highlight); }
.ir-node-header.highlighted { background: var(--highlight-strong); }
.ir-node-header .toggle {
  display: inline-block;
  width: 12px;
  font-size: 10px;
  color: var(--text-dim);
  flex-shrink: 0;
}
.ir-node-header .summary { flex: 1; }
.ir-node-header .span-label {
  font-size: 10px;
  color: var(--text-dim);
  flex-shrink: 0;
}
.ir-node-children { margin-left: 8px; border-left: 1px solid var(--border); }
.ir-node-children.collapsed { display: none; }
.ir-child-label {
  font-size: 11px;
  color: var(--blue);
  padding: 1px 4px;
  margin-left: 16px;
}
.ir-assign { color: var(--green); }
.ir-call { color: var(--cyan); }
.ir-barrier { color: var(--yellow); }
.ir-return { color: var(--magenta); }
.ir-control { color: var(--blue); }
.ir-other { color: var(--text-dim); }
.section-header {
  font-size: 13px;
  font-weight: 600;
  color: var(--accent);
  margin: 12px 0 6px;
  padding-bottom: 4px;
  border-bottom: 1px solid var(--border);
}
.section-header:first-child { margin-top: 0; }
.cfg-function { margin-bottom: 16px; }
.cfg-func-header {
  font-size: 12px;
  font-weight: 600;
  color: var(--cyan);
  margin-bottom: 4px;
}
.cfg-block {
  margin-left: 8px;
  margin-bottom: 8px;
  border-left: 2px solid var(--border);
  padding-left: 8px;
}
.cfg-block.unreachable { opacity: 0.5; border-left-color: var(--red); }
.cfg-block-header {
  font-weight: 600;
  font-size: 12px;
  color: var(--text);
  margin-bottom: 2px;
}
.cfg-block-header .tag {
  font-weight: 400;
  font-size: 10px;
  padding: 1px 4px;
  border-radius: 3px;
  margin-left: 4px;
}
.tag-entry { background: rgba(166, 227, 161, 0.15); color: var(--green); }
.tag-unreachable { background: rgba(243, 139, 168, 0.15); color: var(--red); }
.cfg-stmt {
  font-size: 12px;
  padding: 1px 4px;
  border-radius: 3px;
  cursor: pointer;
}
.cfg-stmt:hover { background: var(--highlight); }
.cfg-stmt.highlighted { background: var(--highlight-strong); }
.cfg-stmt .idx { color: var(--text-dim); min-width: 24px; display: inline-block; }
.cfg-ssa-info {
  font-size: 10px;
  color: var(--text-dim);
  margin-left: 28px;
}
.cfg-phi {
  font-size: 11px;
  color: var(--magenta);
  padding: 1px 4px;
}
.cfg-terminator {
  font-size: 11px;
  color: var(--blue);
  padding: 1px 4px;
  margin-top: 2px;
}
.proc-card {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 10px 12px;
  margin-bottom: 8px;
}
.proc-card .proc-name { font-weight: 600; color: var(--cyan); font-size: 13px; }
.proc-card .proc-detail { font-size: 11px; color: var(--text-dim); margin-top: 4px; }
.proc-card .proc-detail .val { color: var(--text); }
.proc-card .pure-badge {
  display: inline-block;
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 8px;
  margin-left: 6px;
}
.pure-yes { background: rgba(166, 227, 161, 0.15); color: var(--green); }
.pure-no { background: rgba(249, 226, 175, 0.15); color: var(--yellow); }
.opt-item {
  display: flex;
  gap: 8px;
  padding: 4px 6px;
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
  align-items: baseline;
}
.opt-item:hover { background: var(--highlight); }
.opt-item.highlighted { background: var(--highlight-strong); }
.opt-item .opt-code { font-weight: 600; color: var(--green); flex-shrink: 0; min-width: 32px; }
.opt-item .opt-msg { color: var(--text); flex: 1; }
.opt-item .opt-repl { color: var(--green); font-size: 11px; }
.shimmer-item {
  padding: 4px 6px;
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
}
.shimmer-item:hover { background: var(--highlight); }
.shimmer-item.highlighted { background: var(--highlight-strong); }
.shimmer-item .shimmer-code { font-weight: 600; min-width: 32px; display: inline-block; }
.shimmer-S100 .shimmer-code, .shimmer-S101 .shimmer-code { color: var(--yellow); }
.shimmer-S102 .shimmer-code { color: var(--red); }
.gvn-item {
  padding: 4px 6px;
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
}
.gvn-item:hover { background: var(--highlight); }
.gvn-item.highlighted { background: var(--highlight-strong); }
.gvn-item .gvn-code { font-weight: 600; min-width: 32px; display: inline-block; color: var(--green); }
.gvn-item .gvn-expr { color: var(--cyan); font-size: 11px; }
.gvn-item .gvn-first { color: var(--text-dim); font-size: 10px; }
.taint-item {
  padding: 4px 6px;
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
}
.taint-item:hover { background: var(--highlight); }
.taint-item.highlighted { background: var(--highlight-strong); }
.taint-item .taint-code { font-weight: 600; min-width: 52px; display: inline-block; }
.taint-item.taint-danger .taint-code { color: var(--red); }
.taint-item.taint-warn .taint-code { color: var(--yellow); }
.taint-item.taint-info .taint-code { color: var(--blue); }
.taint-tracking-var {
  font-size: 11px;
  padding: 1px 4px;
  color: var(--text-dim);
}
.taint-tracking-var .taint-val { color: var(--orange); }
.type-entry {
  font-size: 11px;
  padding: 1px 4px;
  display: flex;
  gap: 8px;
}
.type-entry .type-var { color: var(--text); min-width: 120px; }
.type-entry .type-val { font-weight: 600; }
.type-entry .type-val.type-known { color: var(--green); }
.type-entry .type-val.type-shimmered { color: var(--yellow); }
.type-entry .type-val.type-overdefined { color: var(--text-dim); }
.irules-flow-item {
  padding: 4px 6px;
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
}
.irules-flow-item:hover { background: var(--highlight); }
.irules-flow-item.highlighted { background: var(--highlight-strong); }
.irules-flow-item .irules-code { font-weight: 600; min-width: 72px; display: inline-block; color: var(--yellow); }
.callout-line {
  display: flex;
  font-size: 12px;
  line-height: 1.5;
}
.callout-line .gutter {
  color: var(--gutter);
  text-align: right;
  min-width: 32px;
  padding-right: 8px;
  flex-shrink: 0;
  user-select: none;
}
.callout-line .code-text {
  white-space: pre;
  flex: 1;
  overflow-x: hidden;
}
.callout-annotation {
  margin-left: 40px;
  font-size: 11px;
  line-height: 1.4;
  white-space: pre;
}
.callout-annotation.kind-barrier { color: var(--yellow); }
.callout-annotation.kind-deadStore { color: var(--yellow); }
.callout-annotation.kind-constantBranch { color: var(--blue); }
.callout-annotation.kind-unreachable { color: var(--magenta); }
.callout-annotation.kind-optimisation { color: var(--green); }
.callout-annotation.kind-shimmer { color: var(--yellow); }
.callout-annotation.kind-thunking { color: var(--red); }
.callout-annotation.kind-gvn { color: var(--green); }
.callout-annotation.kind-taint { color: var(--orange); }
.callout-annotation.kind-irulesFlow { color: var(--yellow); }
.source-listing { font-size: 12px; line-height: 1.5; }
.asm-listing { font-size: 12px; line-height: 1.5; white-space: pre; overflow-x: auto; margin: 0; padding: 8px 12px; }
.source-line { display: flex; }
.source-line .gutter {
  color: var(--gutter);
  text-align: right;
  min-width: 32px;
  padding-right: 8px;
  flex-shrink: 0;
  user-select: none;
}
.source-line .code-text { white-space: pre; flex: 1; }
.opt-diff-container { position: relative; padding-left: 36px; }
.opt-diff-svg {
  position: absolute;
  top: 0; left: 0;
  width: 36px; height: 100%;
  pointer-events: none;
  z-index: 1;
  overflow: visible;
}
.opt-diff-line {
  display: flex;
  font-size: 12px;
  line-height: 1.5;
  position: relative;
  z-index: 2;
}
.opt-diff-line .gutter {
  color: var(--gutter);
  text-align: right;
  min-width: 32px;
  padding-right: 8px;
  flex-shrink: 0;
  user-select: none;
}
.opt-diff-line .code-text { white-space: pre; flex: 1; overflow-x: hidden; }
.opt-diff-line.opt-original { background: rgba(243, 139, 168, 0.06); opacity: 0.45; }
.opt-diff-line.opt-original .code-text { color: var(--text-dim); text-decoration: line-through; }
.opt-diff-line.opt-replacement { background: rgba(166, 227, 161, 0.08); }
.opt-diff-line.opt-replacement .code-text { color: var(--green); }
.opt-diff-line.opt-replacement .gutter { color: var(--green); }
.opt-diff-line.opt-input-highlight { background: rgba(137, 180, 250, 0.18) !important; opacity: 1 !important; }
.opt-diff-line.opt-output-highlight { background: rgba(166, 227, 161, 0.18) !important; }
.opt-bracket { fill: none; stroke: var(--text-dim); stroke-width: 1.5; opacity: 0.4; transition: opacity 0.15s, stroke 0.15s; }
.opt-bracket.highlighted { stroke: var(--accent); opacity: 1; }
.analysis-card {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 10px 12px;
  margin-bottom: 8px;
}
.analysis-card h3 { font-size: 12px; font-weight: 600; color: var(--cyan); margin-bottom: 6px; }
.analysis-entry { font-size: 11px; color: var(--text-dim); padding: 1px 0; }
.analysis-entry .val { color: var(--text); }
.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-dim);
  font-size: 13px;
  text-align: center;
  padding: 32px;
}
.error-box {
  background: rgba(243, 139, 168, 0.1);
  border: 1px solid var(--red);
  border-radius: 6px;
  padding: 12px 16px;
  color: var(--red);
  font-size: 12px;
  white-space: pre-wrap;
}
.spinner {
  display: none;
  width: 16px; height: 16px;
  border: 2px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }
.status-light {
  width: 10px; height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
  transition: background 0.25s, box-shadow 0.25s;
}
.status-light.synced     { background: var(--green);  box-shadow: 0 0 6px var(--green); }
.status-light.compiling  { background: var(--yellow); box-shadow: 0 0 6px var(--yellow); }
.status-light.dirty      { background: var(--red);    box-shadow: 0 0 6px var(--red); }
.status-light.loading    { background: var(--text-dim); box-shadow: none; }
.cfg-edges-container { position: relative; padding-left: 40px; }
.cfg-edges-svg {
  position: absolute;
  top: 0; left: 0;
  width: 100%; height: 100%;
  pointer-events: none;
  z-index: 1;
  overflow: visible;
}
.cfg-edge { fill: none; stroke-width: 1.5; opacity: 0.45; transition: opacity 0.15s, stroke-width 0.15s; }
.cfg-edge-true { stroke: var(--green); }
.cfg-edge-false { stroke: var(--red); }
.cfg-edge-goto { stroke: var(--text-dim); }
.cfg-edge.highlighted { opacity: 1; stroke-width: 2; }
.cfg-arrowhead { opacity: 0.45; transition: opacity 0.15s; }
.cfg-arrowhead-true { fill: var(--green); }
.cfg-arrowhead-false { fill: var(--red); }
.cfg-arrowhead-goto { fill: var(--text-dim); }
.cfg-arrowhead.highlighted { opacity: 1; }
.cfg-block[data-block] { position: relative; z-index: 2; }
.var-tooltip {
  position: fixed;
  background: var(--bg-surface);
  border: 1px solid var(--accent);
  border-radius: 6px;
  padding: 8px 12px;
  font-size: 11px;
  color: var(--text);
  z-index: 1000;
  pointer-events: none;
  box-shadow: 0 4px 12px rgba(0,0,0,0.4);
  max-width: 320px;
  line-height: 1.5;
  white-space: nowrap;
}
.var-tooltip .tt-name { font-weight: 600; color: var(--accent); font-size: 12px; }
.var-tooltip .tt-row { display: flex; gap: 6px; }
.var-tooltip .tt-label { color: var(--text-dim); min-width: 48px; }
.var-tooltip .tt-val { color: var(--text); }
.var-tooltip .tt-type { color: var(--green); }
.var-tooltip .tt-lattice { color: var(--yellow); }
.ssa-var { cursor: default; border-radius: 2px; padding: 0 2px; transition: background 0.1s; }
.ssa-var:hover { background: var(--highlight-strong); }
.ssa-var-def { color: var(--green); }
.ssa-var-use { color: var(--text); }
/* Selection for copy-to-Claude */
[data-start].selected {
  background: rgba(137, 180, 250, 0.12) !important;
  border-left: 2px solid var(--accent);
  padding-left: 6px;
  opacity: 1 !important;
}
.copy-fab {
  position: absolute;
  bottom: 12px;
  right: 12px;
  background: var(--accent);
  color: var(--bg);
  border: none;
  border-radius: 8px;
  padding: 8px 14px;
  font-family: var(--font-mono);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  z-index: 100;
  box-shadow: 0 4px 12px rgba(0,0,0,0.4);
  transition: opacity 0.15s, transform 0.15s;
  display: flex;
  align-items: center;
  gap: 6px;
}
.copy-fab:hover { transform: translateY(-1px); box-shadow: 0 6px 16px rgba(0,0,0,0.5); }
.copy-fab.copied { background: var(--green); }
.copy-fab .key-hint {
  font-size: 10px;
  opacity: 0.7;
  font-weight: 400;
}
/* WASM disassembly view */
.wasm-module { margin-bottom: 16px; }
.wasm-module-details { margin-left: 16px; font-size: 11px; color: var(--text-dim); }
.wasm-module-details summary { cursor: pointer; color: var(--text); }
.wasm-module-details summary:hover { color: var(--accent); }
.wasm-import-entry { padding: 1px 0; white-space: pre; display: flex; gap: 6px; align-items: baseline; }
.wasm-function { margin-bottom: 16px; border: 1px solid var(--border); border-radius: 4px; padding: 6px 8px; background: var(--bg-surface); }
.wasm-function[data-func-idx] { position: relative; }
.wasm-func-header { font-weight: 600; color: var(--cyan); margin-bottom: 4px; padding: 2px 4px; cursor: pointer; border-radius: 3px; transition: background 0.15s; font-size: 12px; }
.wasm-func-header:hover { background: var(--highlight); }
.wasm-func-header.wasm-func-flash { background: rgba(137, 180, 250, 0.3); animation: wasm-flash 1.2s ease-out; }
.wasm-func-header .wasm-kind-badge { font-size: 10px; background: var(--bg-hover); color: var(--text-dim); padding: 1px 5px; border-radius: 8px; margin-left: 2px; font-weight: 400; }
.wasm-func-name { color: var(--accent); }
.wasm-func-locals { margin-left: 40px; padding: 2px 0; color: var(--text-dim); font-size: 11px; }
.wasm-local { padding: 0 4px; }
.wasm-func-body { font-size: 12px; line-height: 1.5; position: relative; padding-left: 36px; }
.wasm-instr, .wasm-src-comment { display: flex; align-items: baseline; gap: 4px; padding: 1px 0; border-radius: 2px; position: relative; z-index: 2; white-space: pre; }
.wasm-instr { cursor: pointer; }
.wasm-instr:hover { background: var(--highlight); }
.wasm-instr.highlighted { background: var(--highlight-strong); }
.wasm-instr.wasm-instr-flash { background: rgba(249, 226, 175, 0.3); animation: wasm-flash 1.2s ease-out; }
.wasm-src-comment { color: var(--green); cursor: pointer; padding-top: 4px; font-size: 11px; opacity: 0.85; }
.wasm-src-comment:hover { background: var(--highlight); opacity: 1; }
.wasm-src-comment .wasm-src-text { color: var(--green); }
.wasm-gutter-pad { width: 0; flex-shrink: 0; }
.wasm-idx { color: var(--gutter); min-width: 36px; user-select: none; text-align: right; font-size: 10px; padding-right: 8px; flex-shrink: 0; }
.wasm-code { display: inline; flex: 1; min-width: 0; }
.wasm-indent { color: transparent; }
.wasm-mnemonic { color: var(--text-bright); font-weight: 500; }
.wasm-operand { color: var(--text); }
.wasm-operand-name { color: var(--green); }
.wasm-block-label { color: var(--magenta); opacity: 0.6; font-size: 10px; margin-left: 2px; }
.wasm-call-target, .wasm-branch-target { cursor: pointer; text-decoration: underline dotted; text-underline-offset: 2px; padding: 0 2px; border-radius: 2px; }
.wasm-call-target { color: var(--cyan); }
.wasm-call-target:hover { color: var(--accent); background: rgba(137, 180, 250, 0.15); }
.wasm-branch-target { color: var(--yellow); }
.wasm-branch-target:hover { color: var(--orange); background: rgba(250, 179, 135, 0.15); }
.wasm-comment { color: var(--text-dim); font-style: italic; }
.wasm-edges-svg {
  position: absolute;
  top: 0; left: 0;
  width: 36px; height: 100%;
  pointer-events: none;
  z-index: 1;
  overflow: visible;
}
.wasm-edge { fill: none; stroke-width: 1.2; opacity: 0.4; transition: opacity 0.15s, stroke-width 0.15s; }
.wasm-edge-forward { stroke: var(--blue); }
.wasm-edge-back { stroke: var(--yellow); stroke-dasharray: 3,2; }
.wasm-edge.highlighted { opacity: 1; stroke-width: 2; }
.wasm-arrowhead { fill: var(--blue); }
@keyframes wasm-flash {
  0% { background: rgba(249, 226, 175, 0.5); }
  100% { background: transparent; }
}
/* Disassembly toolbar + opt diff */
.disasm-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 6px 4px 10px;
  margin-bottom: 8px;
  border-bottom: 1px solid var(--border);
  position: sticky;
  top: 0;
  background: var(--bg);
  z-index: 3;
}
.disasm-toolbar-opt {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text);
  cursor: pointer;
  user-select: none;
}
.disasm-toolbar-opt input[type="checkbox"] { accent-color: var(--accent); cursor: pointer; }
.disasm-toolbar-opt input:disabled { cursor: not-allowed; opacity: 0.4; }
.disasm-toolbar-diff {
  background: var(--bg-surface);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 4px 10px;
  font-family: var(--font-mono);
  font-size: 11px;
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;
}
.disasm-toolbar-diff:hover { border-color: var(--accent); background: var(--bg-hover); }
.disasm-toolbar-diff:disabled { opacity: 0.4; cursor: not-allowed; }
.disasm-toolbar-hint { font-size: 11px; color: var(--text-dim); font-style: italic; }
.disasm-tables { margin-left: 40px; margin-bottom: 4px; color: var(--text-dim); font-size: 11px; }
.disasm-tables summary { cursor: pointer; padding: 2px 0; }
.disasm-tables summary:hover { color: var(--accent); }
.disasm-tables-section { color: var(--text-dim); font-weight: 600; margin-top: 4px; }
.disasm-label-row { cursor: default !important; }
.disasm-label-row:hover { background: transparent; }
.disasm-label-anchor { color: var(--magenta); font-weight: 500; }
.disasm-diff-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 6px 4px;
  margin-bottom: 6px;
  border-bottom: 1px solid var(--border);
}
.disasm-diff-title { font-weight: 600; color: var(--accent); font-size: 12px; }
.disasm-diff-legend { font-size: 10px; display: flex; gap: 10px; }
.disasm-diff-legend-removed { color: var(--red); }
.disasm-diff-legend-added { color: var(--green); }
.disasm-diff-legend-dim { color: var(--text-dim); font-style: italic; }
.disasm-diff-close {
  margin-left: auto;
  background: var(--bg-surface);
  color: var(--text-dim);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 3px 10px;
  font-family: var(--font-mono);
  font-size: 11px;
  cursor: pointer;
}
.disasm-diff-close:hover { border-color: var(--accent); color: var(--text); }
.disasm-diff-block { border: 1px solid var(--border); background: var(--bg-surface); }
.disasm-diff-badge {
  font-size: 10px;
  font-weight: 400;
  padding: 1px 6px;
  border-radius: 8px;
  margin-left: 6px;
  vertical-align: middle;
}
.disasm-diff-added { background: rgba(166, 227, 161, 0.15); color: var(--green); }
.disasm-diff-removed { background: rgba(243, 139, 168, 0.15); color: var(--red); }
.disasm-diff-same { background: rgba(108, 112, 134, 0.15); color: var(--text-dim); }
.disasm-diff-modified { background: rgba(249, 226, 175, 0.18); color: var(--yellow); }
.disasm-diff-rows { padding-left: 4px; }
.disasm-diff-row {
  display: flex;
  gap: 6px;
  padding: 1px 6px;
  border-radius: 2px;
  white-space: pre;
  font-size: 12px;
  line-height: 1.5;
}
.disasm-diff-row.disasm-diff-removed { background: rgba(243, 139, 168, 0.1); color: var(--text); }
.disasm-diff-row.disasm-diff-removed .disasm-diff-sigil { color: var(--red); }
.disasm-diff-row.disasm-diff-removed .disasm-diff-text { text-decoration: line-through; opacity: 0.75; }
.disasm-diff-row.disasm-diff-added { background: rgba(166, 227, 161, 0.1); color: var(--text); }
.disasm-diff-row.disasm-diff-added .disasm-diff-sigil { color: var(--green); }
.disasm-diff-row.disasm-diff-same { background: transparent; color: var(--text-dim); opacity: 0.7; }
.disasm-diff-row.disasm-diff-same .disasm-diff-sigil { color: var(--text-dim); }
.disasm-diff-sigil { min-width: 14px; text-align: center; user-select: none; font-weight: 600; flex-shrink: 0; }
.disasm-diff-text { flex: 1; white-space: pre; }
.disasm-diff-elide {
  padding: 2px 22px;
  color: var(--text-dim);
  font-size: 10px;
  font-style: italic;
}
.disasm-diff-unchanged-note { padding: 4px 8px; color: var(--text-dim); font-size: 11px; font-style: italic; }
</style>
</head>
<body>
<div class="app">
  <div class="toolbar">
    <div class="status-light synced" id="statusLight" title="In sync"></div>
    <h1>Tcl Compiler Explorer</h1>
    <select id="dialect">
      <option value="synopsys-eda-tcl">Synopsys EDA</option>
      <option value="cadence-eda-tcl">Cadence EDA</option>
      <option value="xilinx-eda-tcl">Xilinx EDA</option>
      <option value="intel-quartus-eda-tcl">Intel Quartus</option>
      <option value="mentor-eda-tcl">Mentor EDA</option>
      <option value="f5-iapps">F5 iapps</option>
      <option value="f5-irules">F5 irules</option>
      <option value="tcl8.4">Tcl 8.4</option>
      <option value="tcl8.5">Tcl 8.5</option>
      <option value="tcl8.6" selected>Tcl 8.6</option>
      <option value="tcl9.0">Tcl 9.0</option>
    </select>
    <div class="spinner" id="spinner"></div>
    <span class="status-msg" id="statusMsg" style="display:none"></span>
    <div class="stats" id="stats"></div>
  </div>
  <div class="main" id="main">
    <div class="output-panel" id="outputPanel" style="position:relative">
      <div class="tab-bar" id="tabBar">
        <div class="tab active" data-tab="ir">IR</div>
        <div class="tab" data-tab="cfg-pre">CFG</div>
        <div class="tab" data-tab="cfg-post">SSA+Analysis</div>
        <div class="tab" data-tab="interproc">Interproc</div>
        <div class="tab" data-tab="types">Types</div>
        <div class="tab" data-tab="opt">Optimiser</div>
        <div class="tab" data-tab="gvn">GVN</div>
        <div class="tab" data-tab="shimmer">Shimmer</div>
        <div class="tab" data-tab="taint">Taint</div>
        <div class="tab" data-tab="irules-flow">iRules Flow</div>
        <div class="tab" data-tab="callouts">Callouts</div>
        <div class="tab" data-tab="asm">Tcl ASM</div>
        <div class="tab" data-tab="wasm">WASM</div>
      </div>
      <div class="output-content" id="outputContent">
        <div class="tab-pane active" id="pane-ir">
          <div class="empty-state" id="emptyState">Waiting for source from editor...</div>
        </div>
        <div class="tab-pane" id="pane-cfg-pre"></div>
        <div class="tab-pane" id="pane-cfg-post"></div>
        <div class="tab-pane" id="pane-interproc"></div>
        <div class="tab-pane" id="pane-types"></div>
        <div class="tab-pane" id="pane-opt"></div>
        <div class="tab-pane" id="pane-gvn"></div>
        <div class="tab-pane" id="pane-shimmer"></div>
        <div class="tab-pane" id="pane-taint"></div>
        <div class="tab-pane" id="pane-irules-flow"></div>
        <div class="tab-pane" id="pane-callouts"></div>
        <div class="tab-pane" id="pane-asm"></div>
        <div class="tab-pane" id="pane-wasm"></div>
      </div>
    </div>
  </div>
</div>

<script nonce="${nonce}">
// Consumer globals (required by explorer-core.js)
var data = null;
var compiledSource = '';
var compiledDialect = '';
var currentSource = '';
var $ = function(s) { return document.querySelector(s); };
var $$ = function(s) { return document.querySelectorAll(s); };
function getSource() { return compiledSource || currentSource; }
var __bootStatus = document.getElementById('statusMsg');
var __vscodeApi =
  typeof acquireVsCodeApi === 'function'
    ? acquireVsCodeApi()
    : undefined;
window.__vscodeApi = __vscodeApi;
function __escapeHtml(value) {
  return String(value == null ? '' : value)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}
function __displayScriptFilename(filename) {
  if (!filename) return '';
  if (filename.indexOf('data:text/javascript;base64,') === 0) {
    return 'explorer-core.js (embedded data URI)';
  }
  return filename;
}
function __formatScriptLocation(info) {
  var filename = __displayScriptFilename(info && info.filename ? info.filename : '');
  var lineno = info && info.lineno ? info.lineno : 0;
  var colno = info && info.colno ? info.colno : 0;
  if (!filename && !lineno && !colno) {
    return '';
  }
  if (filename && (lineno || colno)) {
    return filename + ':' + (lineno || 0) + ':' + (colno || 0);
  }
  if (filename) {
    return filename;
  }
  return 'line ' + (lineno || 0) + ':' + (colno || 0);
}
window.__showExplorerScriptIssue = function(detail, info) {
  var problem = detail || 'Unknown script error';
  var location = __formatScriptLocation(info || {});
  var statusText = 'compiler explorer script error: ' + problem + (location ? ' @ ' + location : '');
  if (__bootStatus) {
    __bootStatus.textContent = statusText;
    __bootStatus.style.display = '';
  }
  var spinner = document.getElementById('spinner');
  if (spinner) {
    spinner.style.display = 'none';
  }
  var statusLight = document.getElementById('statusLight');
  if (statusLight) {
    statusLight.className = 'status-light dirty';
    statusLight.title = 'Script error';
  }
  var pane = document.getElementById('pane-ir');
  if (!pane) {
    return;
  }
  var html = '<div class="error-box">';
  html += '<div style="font-weight:600">Compiler explorer script failed to load.</div>';
  html += '<div style="margin-top:6px">' + __escapeHtml(problem) + '</div>';
  if (location) {
    html += '<div style="margin-top:6px; color:var(--text-dim)">Location: ' + __escapeHtml(location) + '</div>';
  }
  var stack = info && info.stack ? String(info.stack) : '';
  if (stack) {
    html += '<pre style="margin-top:8px; white-space:pre-wrap; font-size:11px; color:var(--text-dim)">' + __escapeHtml(stack) + '</pre>';
  }
  html += '</div>';
  pane.innerHTML = html;
};
if (__bootStatus) {
  __bootStatus.textContent = 'script-1 loaded';
  __bootStatus.style.display = '';
}
window.addEventListener('error', function(event) {
  var detail = event.message || 'Unknown script error';
  window.__coreLoadError = event.error || new Error(detail);
  var filename = event.filename || '';
  var lineno = event.lineno || 0;
  var colno = event.colno || 0;
  var stack = event.error && event.error.stack ? event.error.stack : '';
  var payload = {
    type: 'scriptError',
    message: detail,
    filename: filename,
    lineno: lineno,
    colno: colno,
    stack: stack,
  };
  window.__showExplorerScriptIssue(detail, {
    filename: filename,
    lineno: lineno,
    colno: colno,
    stack: stack,
  });
  if (__vscodeApi) {
    __vscodeApi.postMessage(payload);
  }
});
window.addEventListener('unhandledrejection', function(event) {
  var reason = event.reason && event.reason.message ? event.reason.message : String(event.reason);
  window.__coreLoadError = event.reason || new Error(reason || 'Unhandled rejection');
  var stack = event.reason && event.reason.stack ? event.reason.stack : '';
  window.__showExplorerScriptIssue(reason || 'Unhandled promise rejection', {
    stack: stack,
  });
  if (__vscodeApi) {
    __vscodeApi.postMessage({
      type: 'scriptRejection',
      message: reason,
      stack: stack,
    });
  }
});
</script>
<script nonce="${nonce}" src="${coreJsDataUri}"></script>
<script nonce="${nonce}">

// VS Code API
const bootStatus = document.getElementById('statusMsg');
if (bootStatus) {
  bootStatus.textContent = 'booting explorer UI...';
  bootStatus.style.display = '';
}
const vscodeApi = window.__vscodeApi || (typeof acquireVsCodeApi === 'function' ? acquireVsCodeApi() : undefined);
const vscode = vscodeApi || { postMessage: () => {} };
if (bootStatus) {
  if (window.__coreLoadError) {
    bootStatus.textContent =
      'explorer-core load failed: ' +
      (window.__coreLoadError.message || String(window.__coreLoadError));
  } else {
    bootStatus.textContent = vscodeApi ? 'explorer UI ready' : 'webview API unavailable';
  }
}
if (window.__coreLoadError && typeof window.__showExplorerScriptIssue === 'function') {
  window.__showExplorerScriptIssue(
    window.__coreLoadError.message || String(window.__coreLoadError),
    {
      stack: window.__coreLoadError.stack || '',
    },
  );
}
if (vscodeApi && window.__coreLoadError) {
  vscodeApi.postMessage({
    type: 'coreError',
    message: window.__coreLoadError.message || String(window.__coreLoadError),
    stack: window.__coreLoadError.stack || '',
  });
}

// Consumer state
let lastSource = '';
let lastDialect = '';

// Message handler (replaces web worker)
window.addEventListener('message', function(event) {
  const msg = event.data;
  switch (msg.type) {
    case 'sourceUpdate':
      currentSource = msg.source;
      if (msg.dialect) {
        $('#dialect').value = msg.dialect;
      }
      compile();
      break;
    case 'result':
      data = msg.data;
      compiledSource = lastSource;
      compiledDialect = lastDialect;
      renderAll();
      $('#spinner').style.display = 'none';
      updateStatusLight();
      break;
    case 'error':
      showError(msg.data && msg.data.error || 'Unknown error', msg.data && msg.data.traceback);
      $('#spinner').style.display = 'none';
      setStatus('dirty');
      break;
    case 'status':
      $('#statusMsg').textContent = msg.text || msg.message || '';
      $('#statusMsg').style.display = '';
      break;
    case 'switchTab': {
      const tid = msg.tabId;
      $$('.tab').forEach(t => t.classList.remove('active'));
      $$('.tab-pane').forEach(p => p.classList.remove('active'));
      const tgt = $('.tab[data-tab="' + tid + '"]');
      if (tgt) {
        tgt.classList.add('active');
        $('#pane-' + tid).classList.add('active');
        if (['cfg-pre','cfg-post','opt'].includes(tid) && data)
          requestAnimationFrame(() => scheduleEdgeRedraw());
      }
      break;
    }
  }
});

// Dialect change triggers recompile
$('#dialect').addEventListener('change', () => {
  lastSource = '';  // force recompile
  vscode.postMessage({ type: 'dialectChange', dialect: $('#dialect').value });
  compile();
});

// Tabs
$('#tabBar').addEventListener('click', e => {
  const tab = e.target.closest('.tab');
  if (!tab) return;
  $$('.tab').forEach(t => t.classList.remove('active'));
  $$('.tab-pane').forEach(p => p.classList.remove('active'));
  tab.classList.add('active');
  $(\`#pane-\${tab.dataset.tab}\`).classList.add('active');
  if (data && (tab.dataset.tab === 'cfg-pre' || tab.dataset.tab === 'cfg-post' || tab.dataset.tab === 'opt')) {
    requestAnimationFrame(() => scheduleEdgeRedraw());
  }
});

// Compile
function compile() {
  const source = currentSource;
  const dialect = $('#dialect').value;
  if (!source.trim()) return;
  if (source === lastSource && dialect === lastDialect) return;
  lastSource = source;
  lastDialect = dialect;

  $('#spinner').style.display = 'block';
  setStatus('compiling');
  vscode.postMessage({ type: 'compile', source, dialect });
}

function updateStatusLight() {
  const source = currentSource;
  const dialect = $('#dialect').value;
  if (source === compiledSource && dialect === compiledDialect) {
    setStatus('synced');
  } else {
    setStatus('dirty');
  }
}

// Consumer hooks: renderAll / updateBadges
function renderAll() {
  renderStats();
  renderIR();
  renderCfgPre();
  renderCfgPost();
  renderInterproc();
  renderTypes();
  renderOpt();
  renderGvn();
  renderShimmer();
  renderTaint();
  renderIrulesFlow();
  renderCallouts();
  renderAsm();
  renderWasm();
  updateBadges();
}

function updateBadges() {
  const counts = {
    'ir': data.ir.procedures ? Object.keys(data.ir.procedures).length + 1 : 1,
    'cfg-pre': data.cfgPreSsa.length,
    'cfg-post': data.cfgPostSsa.length,
    'interproc': data.interprocedural.length,
    'types': data.types.reduce((n, f) => n + f.entries.length, 0),
    'opt': data.optimisations.length,
    'gvn': data.gvn.length,
    'shimmer': data.shimmer.length,
    'taint': data.taintWarnings.length + data.taintTracking.reduce((n, f) => n + f.entries.length, 0),
    'irules-flow': data.irulesFlow.length,
    'callouts': data.annotations.length,
    'asm': instrCount(data.asm),
    'wasm': instrCount(data.wasm),
  };
  $$('.tab').forEach(tab => {
    const key = tab.dataset.tab;
    const existing = tab.querySelector('.badge');
    if (existing) existing.remove();
    if (counts[key]) {
      const badge = document.createElement('span');
      badge.className = 'badge';
      badge.textContent = counts[key];
      tab.appendChild(badge);
    }
  });
}

// Consumer hook: setupHoverHighlighting (VS Code version with postMessage)
function setupHoverHighlighting(container) {
  container.addEventListener('mouseover', e => {
    const el = e.target.closest('[data-start]');
    if (!el) return;
    const start = parseInt(el.dataset.start);
    const end = parseInt(el.dataset.end);
    vscode.postMessage({ type: 'highlightSource', start, end });
    if (currentHighlighted && currentHighlighted !== el) {
      currentHighlighted.classList.remove('highlighted');
    }
    el.classList.add('highlighted');
    currentHighlighted = el;
  });
  container.addEventListener('mouseleave', () => {
    vscode.postMessage({ type: 'clearHighlight' });
    if (currentHighlighted) {
      currentHighlighted.classList.remove('highlighted');
      currentHighlighted = null;
    }
  });
}

// Consumer hooks: optimiser diff
function setupOptItemDiffScroll(pane) {
  // No-op in VS Code webview (no source editor to scroll)
}

function buildOptDiffView() {
  const source = compiledSource || currentSource;
  const origLines = source.split('\\n');
  const optLines = data.optimisedSource.split('\\n');
  const lineStarts = [0];
  for (let i = 0; i < source.length; i++) { if (source[i] === '\\n') lineStarts.push(i + 1); }
  function lineEnd(n) { return n + 1 < lineStarts.length ? lineStarts[n + 1] - 1 : source.length; }
  const segments = computeDiffSegments(origLines, optLines);
  let html = '<div class="opt-diff-container">';
  let groupId = 0;
  for (const seg of segments) {
    if (seg.type === 'same') {
      for (let i = seg.origStart; i < seg.origEnd; i++) {
        html += \`<div class="opt-diff-line opt-unchanged" data-start="\${lineStarts[i]}" data-end="\${lineEnd(i)}"><span class="gutter">\${i + 1}</span><span class="code-text">\${esc(origLines[i])}</span></div>\`;
      }
    } else {
      const gid = groupId++;
      const gStart = lineStarts[seg.origStart];
      const gEnd = seg.origEnd > 0 ? lineEnd(seg.origEnd - 1) : gStart;
      for (let i = seg.origStart; i < seg.origEnd; i++) {
        html += \`<div class="opt-diff-line opt-original" data-opt-group="\${gid}" data-orig-line="\${i}" data-start="\${lineStarts[i]}" data-end="\${lineEnd(i)}"><span class="gutter">\${i + 1}</span><span class="code-text">\${esc(origLines[i])}</span></div>\`;
      }
      for (let i = seg.optStart; i < seg.optEnd; i++) {
        html += \`<div class="opt-diff-line opt-replacement" data-opt-group="\${gid}" data-start="\${gStart}" data-end="\${gEnd}"><span class="gutter">\u2192</span><span class="code-text">\${esc(optLines[i])}</span></div>\`;
      }
    }
  }
  html += '</div>';
  return html;
}

function setupOptDiffHover(pane) {
  const container=pane.querySelector('.opt-diff-container');
  if(!container)return;
  container.addEventListener('mouseover',e=>{
    const line=e.target.closest('.opt-diff-line[data-opt-group]');
    clearOptHighlights(container);
    if(!line)return;
    const gid=line.dataset.optGroup;
    container.querySelectorAll(\`.opt-original[data-opt-group="\${gid}"]\`).forEach(el=>el.classList.add('opt-input-highlight'));
    container.querySelectorAll(\`.opt-replacement[data-opt-group="\${gid}"]\`).forEach(el=>el.classList.add('opt-output-highlight'));
    container.querySelectorAll(\`.opt-bracket[data-opt-group="\${gid}"]\`).forEach(el=>el.classList.add('highlighted'));
    const origEls=container.querySelectorAll(\`.opt-original[data-opt-group="\${gid}"]\`);
    if(origEls.length){const start=parseInt(origEls[0].dataset.start);const end=parseInt(origEls[origEls.length-1].dataset.end);vscode.postMessage({type:'highlightSource',start,end});}
  });
  container.addEventListener('mouseleave',()=>{clearOptHighlights(container);vscode.postMessage({type:'clearHighlight'});});
}

// Consumer hook: renderShimmer (simplified — no involved lines view)
function renderShimmer() {
  const pane=$('#pane-shimmer');
  if(!data.shimmer.length){pane.innerHTML='<div class="empty-state">No shimmer warnings</div>';return;}
  let html='';
  for(const w of data.shimmer){
    html+=\`<div class="shimmer-item shimmer-\${w.code}"\${sourceRangeAttrs(w.range)}><span class="shimmer-code">\${esc(w.code)}</span> \${esc(w.message)} <span style="color:var(--text-dim); font-size:10px">[\${spanLabel(w.range)}]</span></div>\`;
  }
  pane.innerHTML=html;setupHoverHighlighting(pane);
}

// Consumer hook: renderIrulesFlow (simplified — no mermaid graph)
function renderIrulesFlow() {
  const pane=$('#pane-irules-flow');
  if(!data.irulesFlow.length){pane.innerHTML='<div class="empty-state">No iRules flow warnings (only active in f5-irules dialect)</div>';return;}
  let html='';
  for(const w of data.irulesFlow){
    html+=\`<div class="irules-flow-item"\${sourceRangeAttrs(w.range)}><span class="irules-code">\${esc(w.code)}</span> \${esc(w.message)} <span style="color:var(--text-dim); font-size:10px">[\${spanLabel(w.range)}]</span></div>\`;
  }
  pane.innerHTML=html;setupHoverHighlighting(pane);
}

// Edge redraw event listeners
const outputContent=$('#outputContent');
outputContent.addEventListener('scroll',scheduleEdgeRedraw);
window.addEventListener('resize',scheduleEdgeRedraw);

// Selection event handlers
$('#outputContent').addEventListener('click', e => {
  if (!(e.ctrlKey || e.metaKey)) return;
  const el = e.target.closest('[data-start]');
  if (!el) return;
  e.preventDefault(); e.stopPropagation();
  toggleSelection(el);
});

document.addEventListener('keydown', e => {
  if (e.key === 'Escape' && selectedItems.size > 0) { clearSelection(); e.preventDefault(); return; }
  if ((e.ctrlKey || e.metaKey) && e.key === 'c' && selectedItems.size > 0) { e.preventDefault(); copySelectionToClipboard(); return; }
  if ((e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === 'a' || e.key === 'A')) { e.preventDefault(); selectAllInActiveTab(); return; }
});

// Signal to the extension that the webview is ready to receive messages.
vscode.postMessage({ type: 'ready' });

</script>
</body>
</html>`;
}
