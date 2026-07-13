// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Build the shared BIG-IP report front-end: bundle the page entry scripts in
// `src/pages/` (which pull in `src/search/` and are styled by `src/styles/`) to
// plain browser IIFE scripts in `dist/`, then sync the built JS + styles +
// vendor + the single template into the Python `f5report` package. Those synced
// copies are BUILD OUTPUTS, not checked in (see py/.gitignore); every wheel /
// `maturin develop` build runs this first to populate them on disk, and the
// maturin `include` in py/pyproject.toml pulls them into the wheel. Both
// backends render the SAME template: Python via the `minijinja` binding, Rust
// via `include_str!` of `public/templates/report.html.j2` (plus `dist/`,
// `src/styles/`, `public/vendor/`). The rendered report is always ONE
// self-contained HTML file with every asset inlined.
//
// Usage: `npm run build` (from rust/bigip-report-gen/frontend).

import * as esbuild from "esbuild";
import { mkdirSync, copyFileSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const distDir = join(here, "dist");
const pagesDir = join(here, "src", "pages");
const stylesDir = join(here, "src", "styles");
const vendorDir = join(here, "..", "assets");
const templatesDir = join(here, "..", "templates");
const pyPkg = join(here, "..", "python", "python", "f5report");
const pyTemplates = join(pyPkg, "templates");
const pyVendor = join(pyPkg, "vendor");

// Every front-end script, keyed by its dist basename. All are bundled so a
// module can `import` a sibling (e.g. topology.ts pulls in ts/search/*).
const ENTRIES = [
  "input",
  "report",
  "topology",
  "console",
  "certs",
  "secrets",
  "forensics",
  "irule-flow",
  "irule-format",
  "print",
  "apm",
  "elk-graph",
];

const banner = `// SPDX-License-Identifier: AGPL-3.0-or-later
// Generated from rust/bigip-report-gen/frontend/src — DO NOT EDIT; edit the .ts source.`;

async function build() {
  rmSync(distDir, { recursive: true, force: true });
  mkdirSync(distDir, { recursive: true });
  for (const name of ENTRIES) {
    await esbuild.build({
      entryPoints: [join(pagesDir, `${name}.ts`)],
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
// Only the assets the template (`report.html.j2`) references are
// synced; APM / elk-graph stay Rust-only, as they always have been.
const PY_JS = ["input", "report", "topology", "console", "certs", "secrets", "forensics", "irule-flow", "irule-format", "print", "elk-graph", "apm"];
const PY_CSS = ["input", "report", "topology", "certs", "secrets", "forensics", "print", "apm"];
const PY_VENDOR = [
  "elk.bundled.js",
  "f5query_wasm.js",
  "f5query_wasm_bg.wasm",
  // Project marks the report inlines as <svg>; `make logo` refreshes these from
  // the canonical docs/*.svg.
  "logo-f5q.svg",
  "logo-tcl-lsp.svg",
  "logo-tcl-lsp-dark.svg",
];

function sync() {
  mkdirSync(pyTemplates, { recursive: true });
  mkdirSync(pyVendor, { recursive: true });
  for (const name of PY_JS) copyFileSync(join(distDir, `${name}.js`), join(pyTemplates, `${name}.js`));
  for (const name of PY_CSS) copyFileSync(join(stylesDir, `${name}.css`), join(pyTemplates, `${name}.css`));
  for (const name of PY_VENDOR) copyFileSync(join(vendorDir, name), join(pyVendor, name));
  // ONE template for both backends: the Python `f5report` package renders the
  // same MiniJinja template the Rust generator embeds (via the `minijinja`
  // Python binding), so the two can never drift.
  copyFileSync(join(templatesDir, "report.html.j2"), join(pyTemplates, "report.html.j2"));
  // The builder page the f5report web server serves.
  copyFileSync(join(templatesDir, "input.html"), join(pyTemplates, "input.html"));
}

await build();
sync();
console.log("shared report front-end: built dist/ and synced into py/f5report");
