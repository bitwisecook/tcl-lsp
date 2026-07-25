// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Bundle the spec studio's TypeScript front-end (`src/studio.ts` and the
// modules it imports) to one plain browser IIFE at `dist/studio.js`.
//
// That bundle is a BUILD OUTPUT, not checked in: `build-wasm.sh` inlines it
// into the single-file `dist/index.html` alongside the wasm payload, the
// stylesheet, and the project mark. Run this before `make spec-studio-wasm`
// (the Make target does it for you).
//
// Usage: `npm run build` (from rust/tcl-spec-studio/web).

import * as esbuild from "esbuild";
import { mkdirSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const distDir = join(here, "dist");

const banner = `// SPDX-License-Identifier: AGPL-3.0-or-later
// Generated from rust/tcl-spec-studio/web/src — DO NOT EDIT; edit the .ts source.`;

rmSync(distDir, { recursive: true, force: true });
mkdirSync(distDir, { recursive: true });

await esbuild.build({
  entryPoints: [join(here, "src", "studio.ts")],
  outfile: join(distDir, "studio.js"),
  bundle: true,
  format: "iife",
  target: "es2020",
  platform: "browser",
  // Readable, diffable output — the published page is already one big file, and
  // an AGPL app should ship source a reader can follow.
  minify: false,
  legalComments: "none",
  banner: { js: banner },
  logLevel: "info",
});

console.log("spec studio front-end: built dist/studio.js");
