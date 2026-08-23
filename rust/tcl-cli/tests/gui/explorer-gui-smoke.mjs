// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

/*
 * Headless smoke test for the compiler-explorer GUI (`rust/tcl-cli/gui/`).
 *
 * Driven by `rust/tcl-cli/tests/explorer_gui.rs`, which compiles a Tcl
 * snippet with the real explorer pipeline and hands the resulting contract
 * JSON to this script:
 *
 *     node explorer-gui-smoke.mjs <gui-dir> <payload.json>
 *
 * The script serves a copy of the GUI over loopback with a stub in place of
 * `tcl_explorer_wasm.js` (so no `wasm-pack` build is needed — the stub
 * returns the payload the Rust side just produced, which is byte-identical
 * to what `tcl-explorer-wasm`'s `compile` would return), drives it in
 * headless Chromium, and prints a JSON report on stdout. Everything else —
 * `index.html`, `explorer-core.js`, `worker.js` — is the shipped code.
 *
 * It asserts what issues #1182 / #1183 got wrong:
 *   - the WASM tab actually renders a disassembly,
 *   - the compile spinner stops,
 *   - the dialect dropdown is populated before the first result,
 *   - typing before the WASM module is ready still produces a compile,
 *   - the Compile button forces a recompile,
 *   - no uncaught page errors along the way.
 */

import { createServer } from 'node:http';
import { readFile, mkdtemp, cp, writeFile, rm, mkdir } from 'node:fs/promises';
import { join, extname } from 'node:path';
import { tmpdir } from 'node:os';

const [guiDir, payloadPath] = process.argv.slice(2);
if (!guiDir || !payloadPath) {
  console.error('usage: explorer-gui-smoke.mjs <gui-dir> <payload.json>');
  process.exit(2);
}

const playwrightModule = process.env.TCL_EXPLORER_PLAYWRIGHT ?? 'playwright';
const { chromium } = await import(playwrightModule);
const launchOptions = process.env.TCL_EXPLORER_CHROMIUM
  ? { executablePath: process.env.TCL_EXPLORER_CHROMIUM }
  : {};

const payload = await readFile(payloadPath, 'utf8');
const root = await mkdtemp(join(tmpdir(), 'explorer-gui-smoke-'));
await cp(guiDir, root, { recursive: true });

// `wasm-pack --target no-modules` defines a global `wasm_bindgen` init
// function with `compile` / `meta` attached; stub exactly that surface.
const meta = JSON.stringify(JSON.parse(payload).meta ?? {});
await writeFile(
  join(root, 'tcl_explorer_wasm.js'),
  `self.__PAYLOAD = ${JSON.stringify(payload)};\n` +
    `self.__META = ${JSON.stringify(meta)};\n` +
    'self.__COMPILES = 0;\n' +
    'self.wasm_bindgen = function () { return Promise.resolve(); };\n' +
    'self.wasm_bindgen.meta = function () { return self.__META; };\n' +
    'self.wasm_bindgen.compile = function () { self.__COMPILES += 1; return self.__PAYLOAD; };\n',
);
await writeFile(join(root, 'tcl_explorer_wasm_bg.wasm'), '');
// Mermaid is vendored by `make explorer-wasm` and not checked in; the GUI
// only uses it for the iRules event-flow diagram, so a stub keeps the page
// free of unrelated load errors.
await writeFile(
  join(root, 'mermaid.min.js'),
  'globalThis.mermaid={initialize(){},render(){return Promise.resolve({svg:""})}};',
);
await writeFile(join(root, 'build_info.json'), '{"version":"smoke-test"}');
// This harness tests the compiler result renderer, not Monaco (the assembled
// browser/native editor bundle has its own boot tests). Supply the same module
// boundary with a tiny test surface so the production page still has exactly
// one editor path and makes no failed asset requests here.
await mkdir(join(root, 'editor', 'assets'), { recursive: true });
await writeFile(
  join(root, 'editor', 'build-info.json'),
  '{"version":"smoke-test","assets":[{"name":"editor-controller","sha256":"smoke"}]}',
);
await writeFile(
  join(root, 'editor', 'assets', 'monaco-host.js'),
  'export async function mountTclEditor(options){' +
    'options.container.classList.add("monaco-mounted");' +
    'options.container.tabIndex=0;' +
    'options.container.addEventListener("keydown",event=>{' +
      'if(event.key==="Enter"&&(event.ctrlKey||event.metaKey)){event.preventDefault();options.onCompile?.();}' +
    '});' +
    'return {setDialect(){},highlightRanges(){},layout(){},lspReady:true};' +
  '}',
);

const MIME = {
  '.html': 'text/html',
  '.js': 'text/javascript',
  '.json': 'application/json',
  '.wasm': 'application/wasm',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
};
const server = createServer(async (req, res) => {
  const name = decodeURIComponent(req.url.split('?')[0]).replace(/^\/+/, '') || 'index.html';
  if (name.includes('..')) {
    res.writeHead(400);
    res.end('bad path');
    return;
  }
  try {
    const body = await readFile(join(root, name));
    res.writeHead(200, { 'content-type': MIME[extname(name)] ?? 'application/octet-stream' });
    res.end(body);
  } catch {
    res.writeHead(404);
    res.end('not found');
  }
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const base = `http://127.0.0.1:${server.address().port}/`;

const SOURCE = 'proc add {a b} {\n    return [expr {$a + $b}]\n}\nputs [add 1 2]\n';
const pageErrors = [];
const browser = await chromium.launch(launchOptions);
let report;
try {
  // Registry metadata arrives on the worker's ready message, independently of
  // compilation. A blank explorer must therefore have a usable trait reference.
  const referencePage = await browser.newPage();
  referencePage.on('pageerror', (err) => pageErrors.push(err.stack || String(err)));
  await referencePage.goto(base);
  await referencePage.waitForFunction(
    () => document.querySelectorAll('.trait-reference-row').length > 90,
    null,
    { timeout: 30_000 },
  );
  const traitRowsBeforeCompile = await referencePage.locator('.trait-reference-row').count();
  await referencePage.close();

  const page = await browser.newPage();
  page.on('pageerror', (err) => pageErrors.push(err.stack || String(err)));

  await page.goto(base);
  // Deliberately type *before* the worker signals ready: a compile queued
  // during module load must still happen (it used to be dropped, leaving
  // the GUI blank forever).
  await page.fill('#source', SOURCE);
  await page.waitForFunction(() => !!window.data, null, { timeout: 30_000 });

  const afterFirst = await snapshot(page);

  // The Compile button must force a fresh compile even though neither the
  // source nor the dialect changed.
  const before = await compileCount(page);
  await page.click('#compileBtn');
  let after = before;
  for (let i = 0; i < 100 && after === before; i += 1) {
    await page.waitForTimeout(100);
    after = await compileCount(page);
  }

  // Monaco owns focus in the shipped UI. Its Ctrl/Cmd+Enter command must
  // reach the same force-compile path as the toolbar button.
  const beforeShortcut = after;
  await page.focus('#monacoSource');
  await page.keyboard.press('Control+Enter');
  let afterShortcut = beforeShortcut;
  for (let i = 0; i < 100 && afterShortcut === beforeShortcut; i += 1) {
    await page.waitForTimeout(100);
    afterShortcut = await compileCount(page);
  }

  report = {
    ok: true,
    first: afterFirst,
    traitRowsBeforeCompile,
    compilesBeforeButton: before,
    compilesAfterButton: after,
    compilesBeforeShortcut: beforeShortcut,
    compilesAfterShortcut: afterShortcut,
    pageErrors,
  };
} finally {
  await browser.close();
  server.close();
  await rm(root, { recursive: true, force: true });
}

console.log(JSON.stringify(report, null, 2));

async function snapshot(page) {
  return page.evaluate(() => {
    const wasm = document.querySelector('#pane-wasm');
    const asm = document.querySelector('#pane-asm');
    return {
      spinnerDisplay: getComputedStyle(document.querySelector('#spinner')).display,
      statusLight: document.querySelector('#statusLight').className,
      wasmText: wasm.textContent.replace(/\s+/g, ' ').trim().slice(0, 400),
      wasmModuleHeaders: wasm.querySelectorAll('.wasm-module').length,
      wasmFunctions: wasm.querySelectorAll('.wasm-function').length,
      wasmInstructions: wasm.querySelectorAll('.wasm-instr').length,
      asmFunctions: asm.querySelectorAll('.wasm-function').length,
      monacoMounted: !!document.querySelector('#monacoSource.monaco-mounted'),
      stateEditorDisplay: getComputedStyle(document.querySelector('#editorContainer')).display,
      traitRows: document.querySelectorAll('.trait-reference-row').length,
      traitGroups: document.querySelectorAll('.trait-group').length,
      traitText: document.querySelector('#pane-trait-reference').textContent
        .replace(/\s+/g, ' ').trim().slice(0, 8000),
      errorBoxes: Array.from(document.querySelectorAll('.error-box')).map((e) =>
        e.textContent.slice(0, 200),
      ),
      dialects: Array.from(document.querySelectorAll('#dialect option')).map((o) => o.value),
      hasCompileButton: !!document.querySelector('#compileBtn'),
    };
  });
}

// The worker owns the stub, so read its counter through a round-trip-free
// worker evaluation.
async function compileCount(page) {
  const workers = page.workers();
  if (!workers.length) return null;
  return workers[0].evaluate(() => self.__COMPILES);
}
