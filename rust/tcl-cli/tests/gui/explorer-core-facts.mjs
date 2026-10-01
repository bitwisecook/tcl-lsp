// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

/*
 * Renders the post-SSA CFG pane of `rust/tcl-cli/gui/explorer-core.js` over a
 * contract payload and prints the pane's text as JSON:
 *
 *     node explorer-core-facts.mjs <gui-dir> <payload.json> [optimised]
 *
 * `optimised` renders the pane's optimised view (the CFG of the optimiser's
 * rewritten source) instead of the compiled source's.
 *
 * Driven by `rust/tcl-cli/tests/explorer_gui.rs`. The shipped script runs in a
 * `vm` context whose DOM is a stand-in that absorbs every call and keeps only
 * what is assigned to `innerHTML`, so the test needs `node` and no browser.
 */

import vm from 'node:vm';
import fs from 'node:fs';
import path from 'node:path';

const [guiDir, payloadPath, view] = process.argv.slice(2);
if (!guiDir || !payloadPath) {
  console.error('usage: explorer-core-facts.mjs <gui-dir> <payload.json> [optimised]');
  process.exit(2);
}
const source = fs.readFileSync(path.join(guiDir, 'explorer-core.js'), 'utf8');
const payload = JSON.parse(fs.readFileSync(payloadPath, 'utf8'));

function absorbing(state) {
  return new Proxy(function () {}, {
    get(_target, key) {
      if (key === 'innerHTML') return state.html;
      if (key === 'querySelectorAll') return () => [];
      if (key === 'classList') return { contains: () => false, add() {}, remove() {}, toggle() {} };
      return absorbing({ html: '' });
    },
    set(_target, key, value) {
      if (key === 'innerHTML') state.html = String(value);
      return true;
    },
    apply: () => absorbing({ html: '' }),
  });
}

const pane = { state: { html: '' } };
pane.node = absorbing(pane.state);
const context = vm.createContext({
  document: absorbing({ html: '' }),
  navigator: { platform: 'Linux' },
  window: {},
  console,
  data: payload,
  compiledSource: '',
  compiledDialect: 'tcl8.6',
  $: (selector) => (selector === '#pane-cfg-post' ? pane.node : absorbing({ html: '' })),
  $$: () => [],
  requestAnimationFrame: () => {},
  setTimeout,
  clearTimeout,
  setupHoverHighlighting: () => {},
});
vm.runInContext(source, context, { filename: 'explorer-core.js' });
if (view === 'optimised') context.structOptState['pane-cfg-post'] = 'on';
context.renderCfgPost();
const text = pane.state.html
  .replace(/<[^>]*>/g, ' ')
  .replace(/&lt;/g, '<')
  .replace(/&gt;/g, '>')
  .replace(/&quot;/g, '"')
  .replace(/&#39;/g, "'")
  .replace(/&amp;/g, '&')
  .replace(/\s+/g, ' ');
process.stdout.write(JSON.stringify({ text }));
