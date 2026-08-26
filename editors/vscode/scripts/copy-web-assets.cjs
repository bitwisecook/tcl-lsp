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

// Stage the browser language server into `dist/web/` — the three files
// `make lsp-server-wasm` emits (`rust/tcl-lsp-server-wasm/dist/`), plus the
// bundled SpecTcl loadables under `dist/web/specs/`.
//
// Best-effort by design, exactly like `copy-wasm` for the explorer module: a
// checkout that has never run `make lsp-server-wasm` still compiles and still
// runs on the desktop, and the browser entry reports a clear error rather than
// half-working. The VSIX is what must never miss them, and the Makefile's
// `$(VSIX_FILE)` recipe stages them itself (with `verify-vsix` asserting the
// result), so packaging does not depend on this script having been run.
//
// `--require` makes a missing asset an error instead: that is what CI and the
// packaging path use when the assets are supposed to already be there.

const fs = require("fs");
const path = require("path");

const WORKER_ASSETS = ["worker.js", "tcl_lsp_server_wasm.js", "tcl_lsp_server_wasm_bg.wasm"];

const extensionRoot = path.resolve(__dirname, "..");
const repoRoot = path.resolve(extensionRoot, "..", "..");
const wasmDist = path.join(repoRoot, "rust", "tcl-lsp-server-wasm", "dist");
const specSrc = path.join(repoRoot, "specs");
const webDir = path.join(extensionRoot, "dist", "web");
const specDir = path.join(webDir, "specs");

const require_ = process.argv.includes("--require");

function fail(message) {
  if (require_) {
    console.error(`copy-web-assets: ${message}`);
    process.exit(1);
  }
  console.log(`copy-web-assets: ${message} — skipping (run 'make lsp-server-wasm')`);
  process.exit(0);
}

const missing = WORKER_ASSETS.filter((name) => !fs.existsSync(path.join(wasmDist, name)));
if (missing.length > 0) {
  fail(`${wasmDist} is missing ${missing.join(", ")}`);
}

fs.mkdirSync(webDir, { recursive: true });
for (const name of WORKER_ASSETS) {
  fs.copyFileSync(path.join(wasmDist, name), path.join(webDir, name));
}

// The `.tclspec` packs the native server finds in a `specs/` directory beside
// its executable. The browser server has no executable, so the host upserts
// them into the virtual pack mount at startup — see
// docs/design/contracts/lsp-source-store.md.
let packs = [];
if (fs.existsSync(specSrc)) {
  packs = fs.readdirSync(specSrc).filter((name) => name.endsWith(".tclspec"));
}
if (packs.length === 0) {
  fail(`no .tclspec packs in ${specSrc}`);
}
fs.mkdirSync(specDir, { recursive: true });
for (const name of packs) {
  fs.copyFileSync(path.join(specSrc, name), path.join(specDir, name));
}
// A manifest, because an installed web extension's files are served over http
// and VS Code's http filesystem provider answers `readFile` only — the browser
// entry cannot list this directory to discover what is in it.
fs.writeFileSync(path.join(specDir, "index.json"), `${JSON.stringify(packs.sort(), null, 2)}\n`);

console.log(
  `copy-web-assets: staged ${WORKER_ASSETS.length} worker asset(s) and ${packs.length} spec pack(s) into dist/web`,
);
