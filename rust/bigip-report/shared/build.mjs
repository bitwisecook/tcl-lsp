// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Build the shared BIG-IP report front-end: bundle the TypeScript in `ts/` to
// plain browser IIFE scripts in `dist/`, then sync the built JS + styles +
// vendor + the Jinja2 template into the Python `f5report` package so its wheel
// stays self-contained (no Node needed at wheel-build time — same precedent as
// the vendored wasm). The Rust generator embeds `dist/`, `styles/`, `vendor/`,
// and `templates/report.minijinja.html.j2` directly via `include_str!`.
//
// Usage: `npm run build` (from rust/bigip-report/shared).

import * as esbuild from "esbuild";
import { mkdirSync, copyFileSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const distDir = join(here, "dist");
const stylesDir = join(here, "styles");
const vendorDir = join(here, "vendor");
const templatesDir = join(here, "templates");
const pyPkg = join(here, "..", "py", "python", "f5report");
const pyTemplates = join(pyPkg, "templates");
const pyVendor = join(pyPkg, "vendor");

// Every front-end script, keyed by its dist basename. All are bundled so a
// module can `import` a sibling (e.g. topology.ts pulls in ts/search/*).
const ENTRIES = [
  "report",
  "topology",
  "console",
  "certs",
  "secrets",
  "forensics",
  "irule-flow",
  "apm",
  "elk-graph",
];

const banner = `// SPDX-License-Identifier: AGPL-3.0-or-later
// Generated from rust/bigip-report/shared/ts — DO NOT EDIT; edit the .ts source.`;

async function build() {
  rmSync(distDir, { recursive: true, force: true });
  mkdirSync(distDir, { recursive: true });
  for (const name of ENTRIES) {
    await esbuild.build({
      entryPoints: [join(here, "ts", `${name}.ts`)],
      outfile: join(distDir, `${name}.js`),
      bundle: true,
      format: "iife",
      target: "es2020",
      platform: "browser",
      // Readable, diffable output — the page is already one big file, and the
      // Rust/Python builds embed these verbatim, so keep them human-legible.
      minify: false,
      legalComments: "none",
      banner: { js: banner },
      logLevel: "info",
    });
  }
}

// ---- sync built assets into the self-contained Python package --------------
// Only the assets the Jinja2 template (`report.jinja2.html.j2`) references are
// synced; APM / elk-graph stay Rust-only, as they always have been.
const PY_JS = ["report", "topology", "console", "certs", "secrets", "forensics", "irule-flow"];
const PY_CSS = ["report", "topology", "certs", "secrets", "forensics"];
const PY_VENDOR = ["mermaid.min.js", "mermaid.LICENSE", "f5query_wasm.js", "f5query_wasm_bg.wasm"];

function sync() {
  mkdirSync(pyTemplates, { recursive: true });
  mkdirSync(pyVendor, { recursive: true });
  for (const name of PY_JS) copyFileSync(join(distDir, `${name}.js`), join(pyTemplates, `${name}.js`));
  for (const name of PY_CSS) copyFileSync(join(stylesDir, `${name}.css`), join(pyTemplates, `${name}.css`));
  for (const name of PY_VENDOR) copyFileSync(join(vendorDir, name), join(pyVendor, name));
  // The Jinja2 template lives under its engine-specific name in shared/; the
  // Python PackageLoader still asks for "report.html.j2".
  copyFileSync(join(templatesDir, "report.jinja2.html.j2"), join(pyTemplates, "report.html.j2"));
}

await build();
sync();
console.log("shared report front-end: built dist/ and synced into py/f5report");
