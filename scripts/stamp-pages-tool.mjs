// tcl-lsp — stamp one GitHub Pages tool with versioned asset URLs
// SPDX-License-Identifier: AGPL-3.0-or-later
import { createHash } from "node:crypto";
import { readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";

const [directory, version] = process.argv.slice(2);
if (!directory || !version)
  throw new Error("usage: stamp-pages-tool.mjs DIRECTORY VERSION");

const files = [];
function walk(dir) {
  for (const name of readdirSync(dir).sort()) {
    if (name === "build-info.json") continue;
    const path = join(dir, name);
    if (statSync(path).isDirectory()) walk(path);
    else files.push(path);
  }
}
walk(directory);

function describe(path) {
  const bytes = readFileSync(path);
  return {
    name: relative(directory, path).replaceAll("\\", "/"),
    sha256: createHash("sha256").update(bytes).digest("hex"),
    bytes: bytes.length,
  };
}
let assets = files.map(describe);
const hash = (name) =>
  assets.find((asset) => asset.name === name)?.sha256 ?? version;
const indexPath = join(directory, "index.html");
let html = readFileSync(indexPath, "utf8").replaceAll(
  "__TCL_LSP_BUILD_VERSION__",
  version,
);
for (const name of [
  "mermaid.min.js",
  "site-update.js",
  "explorer-core.js",
  "worker.js",
  "tcl-lsp-logo.svg",
  "favicon.png",
]) {
  // The worker's query propagates to the glue and WASM it imports, so key the
  // whole worker graph by the build rather than only by worker.js's bytes.
  const versioned = `${name}?v=${name === "worker.js" ? version : hash(name)}`;
  html = html.replaceAll(`"${name}"`, `"${versioned}"`);
  html = html.replaceAll(`'${name}'`, `'${versioned}'`);
}
writeFileSync(indexPath, html);
// Stamping changes index.html, so describe the deployed bytes rather than the
// input copy. This also makes the manifest useful as a provenance record.
assets = files.map(describe);
writeFileSync(
  join(directory, "build-info.json"),
  JSON.stringify({ version, assets }, null, 2) + "\n",
);
