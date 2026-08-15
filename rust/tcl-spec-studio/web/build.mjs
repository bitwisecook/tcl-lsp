// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Bundle the spec studio's TypeScript front-end. Two outputs, on purpose:
//
//   dist/studio.js               the controller — a plain browser IIFE, inlined
//                                into `dist/index.html` by build-wasm.sh
//   dist/assets/monaco-host.js   Monaco plus the language client, an ES module
//   dist/assets/monaco-host.css  the editor's stylesheet, emitted beside it
//
// The split is the point. The controller is ~120 KB and every visitor loads it;
// the editor is ~2.5 MB and only an author who opens an editor tab ever does,
// via a dynamic `import()` of the second file. esbuild's code splitting cannot
// do this for us — it needs `format: "esm"` for the whole build, and the
// controller has to stay a classic script so the page keeps working when
// opened from `file://` — so the second bundle is simply a second build with
// its own entry point.
//
// Both are BUILD OUTPUTS, not checked in.
//
// Usage: `npm run build` (from rust/tcl-spec-studio/web).

import * as esbuild from "esbuild";
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const distDir = join(here, "dist");
const assetsDir = join(distDir, "assets");
const testsDir = join(distDir, "tests");
const version =
  process.env.TCL_LSP_VERSION ??
  process.env.TCL_LSP_GIT_DESCRIBE ??
  execFileSync("git", ["describe", "--tags", "--always", "--dirty"], {
    cwd: here,
    encoding: "utf8",
  }).trim();

// `node build.mjs --tests` bundles the unit tests instead of the app: they
// import the same TypeScript sources the browser bundle does, and node's test
// runner reads plain JavaScript. Kept in the one build script so there is a
// single place that knows how this project's TypeScript becomes JavaScript.
if (process.argv.includes("--tests")) {
  rmSync(testsDir, { recursive: true, force: true });
  mkdirSync(testsDir, { recursive: true });
  await esbuild.build({
    entryPoints: [join(here, "test", "units.test.ts")],
    outfile: join(testsDir, "units.test.mjs"),
    bundle: true,
    format: "esm",
    platform: "node",
    target: "node22",
    define: { __SPEC_STUDIO_FRONTEND_VERSION__: JSON.stringify(version) },
    // node's own modules stay external — bundling them would be nonsense.
    packages: "external",
    logLevel: "info",
  });
  process.exit(0);
}

const banner = `// SPDX-License-Identifier: AGPL-3.0-or-later
// Generated from rust/tcl-spec-studio/web/src — DO NOT EDIT; edit the .ts source.`;

/**
 * The editor bundle carries third-party code. Its licences travel with it —
 * see web/THIRD-PARTY-NOTICES.md for the full texts and versions.
 */
const vendorBanner = `${banner}
//
// This file bundles third-party MIT-licensed software:
//   monaco-editor                  (c) Microsoft Corporation      — MIT
//   vscode-jsonrpc                 (c) Microsoft Corporation      — MIT
//   vscode-languageserver-protocol (c) Microsoft Corporation      — MIT
//   vscode-languageserver-types    (c) Microsoft Corporation      — MIT
//   vscode-oniguruma                (c) Microsoft Corporation      — MIT/BSD-2-Clause
//   vscode-textmate                 (c) Microsoft Corporation      — MIT
// Full licence texts: rust/tcl-spec-studio/web/THIRD-PARTY-NOTICES.md`;

rmSync(distDir, { recursive: true, force: true });
mkdirSync(assetsDir, { recursive: true });

// The controller. Nothing from node_modules reaches this bundle — the dynamic
// import of the editor chunk is built from a runtime URL, so esbuild leaves it
// alone rather than pulling Monaco back in here.
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
  define: { __SPEC_STUDIO_FRONTEND_VERSION__: JSON.stringify(version) },
  logLevel: "info",
});

// Editor integrations inject a tiny host bridge and use their ordinary native
// editors for Pack DSL, Test Tcl, generated Rust, and the generated stub.
// Keep this separate from Monaco so embedding Studio does not embed an editor
// inside an editor.
await esbuild.build({
  entryPoints: [join(here, "src", "nativeEditorHost.ts")],
  outfile: join(assetsDir, "native-editor-host.js"),
  bundle: true,
  format: "esm",
  target: "es2020",
  platform: "browser",
  minify: true,
  legalComments: "none",
  banner: { js: banner },
  define: { __SPEC_STUDIO_FRONTEND_VERSION__: JSON.stringify(version) },
  logLevel: "info",
});

// Monaco has no bundled Tcl syntax. Ship the exact TextMate grammar used by
// the VS Code extension, plus the same Oniguruma engine that executes it, so
// the Studio's pre-semantic paint cannot drift into a second Tcl grammar.
copyFileSync(
  join(here, "..", "..", "..", "editors", "vscode", "syntaxes", "tcl.tmLanguage.json"),
  join(assetsDir, "tcl.tmLanguage.json"),
);
copyFileSync(join(here, "node_modules", "vscode-oniguruma", "release", "onig.wasm"), join(assetsDir, "onig.wasm"));

// The editor chunk. Minified, unlike the controller: this is vendored
// third-party code whose readable form is upstream's repository, not ours, and
// 2.5 MB unminified would be 9 MB of dist for no reader's benefit.
const monacoBuild = await esbuild.build({
  entryPoints: [join(here, "src", "monacoHost.ts")],
  outfile: join(assetsDir, "monaco-host.js"),
  bundle: true,
  format: "esm",
  target: "es2020",
  platform: "browser",
  minify: true,
  legalComments: "none",
  banner: { js: vendorBanner },
  metafile: true,
  define: { __SPEC_STUDIO_FRONTEND_VERSION__: JSON.stringify(version) },
  // Monaco's CSS pulls in the codicon font; inlining it as a data URI is what
  // keeps the dist to files that can be copied anywhere without a loader.
  loader: { ".ttf": "dataurl", ".woff": "dataurl", ".woff2": "dataurl", ".svg": "dataurl" },
  logLevel: "info",
});

// `editor.api` lets code register an LSP provider even when the controller
// which asks that provider for data was tree-shaken out. That failure looks
// healthy to direct transport probes and leaves the user with TextMate-only
// colouring. Make the controllers part of the build contract.
for (const feature of ["format", "hover", "parameterHints", "semanticTokens", "suggest"]) {
  const registration = `/features/${feature}/register.js`;
  if (!Object.keys(monacoBuild.metafile.inputs).some((input) => input.includes(registration))) {
    throw new Error(`Monaco's ${feature} controller is missing from the editor bundle`);
  }
}

console.log(
  `spec studio front-end ${version}: built dist/studio.js + native/Monaco editor hosts`,
);
