// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

// Finite wire/render control, not a browser or Tcl provider execution proof.
// node explorer-core-retained-views.mjs <gui-dir> <vscode-shell.ts> <payload.json>
import vm from 'node:vm';
import fs from 'node:fs';
import path from 'node:path';

const [guiDir, vscodeShell, payloadPath] = process.argv.slice(2);
if (!guiDir || !vscodeShell || !payloadPath) {
  throw new Error('usage: explorer-core-retained-views.mjs <gui-dir> <vscode-shell.ts> <payload.json>');
}
const source = fs.readFileSync(path.join(guiDir, 'explorer-core.js'), 'utf8');
const payload = JSON.parse(fs.readFileSync(payloadPath, 'utf8'));
const shellNames = ['standalone', 'vscode'];
const shellSources = [
  fs.readFileSync(path.join(guiDir, 'index.html'), 'utf8'),
  fs.readFileSync(vscodeShell, 'utf8'),
];
function absorbing(state) {
  return new Proxy(function () {}, {
    get(_target, key) {
      if (key === 'innerHTML') return state.html;
      if (Object.prototype.hasOwnProperty.call(state, key)) return state[key];
      if (key === 'querySelectorAll') return () => [];
      if (key === 'classList') return { contains: () => false, add() {}, remove() {}, toggle() {} };
      return absorbing({ html: '' });
    },
    set(_target, key, value) {
      if (key === 'innerHTML') state.html = String(value);
      else state[key] = value;
      return true;
    },
    apply: () => absorbing({ html: '' }),
  });
}
function shippedFunction(shell, name) {
  const match = shell.match(new RegExp('function ' + name + '\\([^)]*\\) \\{[\\s\\S]*?\\n\\}'));
  if (!match) throw new Error('missing shipped function ' + name);
  return match[0];
}
function text(html) {
  return html.replace(/<[^>]*>/g, ' ').replace(/&lt;/g, '<').replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"').replace(/&#39;/g, "'").replace(/&amp;/g, '&')
    .replace(/\s+/g, ' ').trim();
}
const report = {};
for (let i = 0; i < shellNames.length; i++) {
  const panes = new Map();
  const badges = {};
  const tabs = ['opt', 'callouts', 'event-order', 'optimiser-passes', 'irules-flow', 'cfg-post', 'gvn', 'shimmer', 'taint', 'bounds', 'asm', 'wasm'].map(id => ({
    dataset: { tab: id }, querySelector: () => null,
    appendChild: badge => { badges[id] = Number(badge.textContent); },
  }));
  const nodeFor = selector => {
    if (!panes.has(selector)) {
      const state = { html: '' };
      panes.set(selector, { state, node: absorbing(state) });
    }
    return panes.get(selector).node;
  };
  const context = vm.createContext({
    document: absorbing({ html: '' }), navigator: { platform: 'Linux' },
    window: {}, console, data: payload, compiledSource: payload.source || '',
    compiledDialect: 'f5-irules', $: nodeFor, $$: () => tabs,
    requestAnimationFrame: () => {}, setTimeout, clearTimeout,
    getSource: () => payload.source || '', setupHoverHighlighting: () => {},
    buildOptDiffView: () => '', setupOptDiffHover: () => {}, setupOptItemDiffScroll: () => {},
    instrCount: () => 0, renderEventFlowGraph: () => {}, eventRangeMap: {},
  });
  vm.runInContext(source, context, { filename: 'explorer-core.js' });
  vm.runInContext(shippedFunction(shellSources[i], 'renderShimmer'), context);
  vm.runInContext(shippedFunction(shellSources[i], 'renderIrulesFlow'), context);
  if (shellNames[i] === 'vscode') vm.runInContext(shippedFunction(shellSources[i], 'warningRow'), context);
  for (const render of ['renderStats', 'renderOpt', 'renderOptimiserPasses', 'renderEventOrder', 'renderCallouts', 'renderCfgPost', 'renderGvn', 'renderTaint', 'renderBounds', 'renderAsm', 'renderWasm', 'renderShimmer']) {
    context[render]();
  }
  vm.runInContext(shippedFunction(shellSources[i], 'updateBadges'), context);
  context.updateBadges();
  context.renderIrulesFlow();
  context.renderGenericExplorerView({ id: 'liveness' });
  report[shellNames[i]] = {
    panes: Object.fromEntries([...panes].map(([selector, pane]) => [selector, text(pane.state.html)])),
    badges,
    arrayCounts: Object.fromEntries(['optimisations', 'annotations', 'eventOrder', 'asm', 'wasm', 'cfgPostSsa', 'shimmer', 'gvn', 'taintWarnings', 'liveness', 'irulesFlow', 'bounds'].map(
      field => [field, context.explorerArrayCount(payload[field]) ?? null])),
  };
}
process.stdout.write(JSON.stringify(report));
