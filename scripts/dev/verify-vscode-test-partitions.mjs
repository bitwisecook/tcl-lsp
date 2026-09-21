#!/usr/bin/env node
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
// SPDX-License-Identifier: AGPL-3.0-or-later

import fs from "node:fs";
import path from "node:path";
import { parseJsonNoDuplicateKeys } from "./json-no-duplicate-keys.mjs";

const root = path.resolve(process.argv[2] ?? ".");
const extension = path.join(root, "editors", "vscode");
const manifest = parseJsonNoDuplicateKeys(
  fs.readFileSync(path.join(extension, "test-partitions.json"), "utf8"),
  "VS Code test partition manifest",
);
if (manifest.version !== 1 || manifest.canonical_glob !== "**/*.test.js" || JSON.stringify(manifest.exclude) !== JSON.stringify(["multiFolder/**"])) {
  throw new Error("unsupported VS Code test partition manifest schema");
}
if (JSON.stringify(Object.keys(manifest.partitions).sort()) !== JSON.stringify(["1", "2", "3"])) {
  throw new Error("manifest must define exactly partitions 1, 2, and 3");
}
function listFiles(root, prefix = "") {
  if (!fs.existsSync(root)) return [];
  const files = [];
  for (const entry of fs.readdirSync(root, { withFileTypes: true })) {
    const relative = prefix ? `${prefix}/${entry.name}` : entry.name;
    if (entry.isDirectory()) {
      files.push(...listFiles(path.join(root, entry.name), relative));
    } else if (entry.isFile()) {
      files.push(relative);
    }
  }
  return files;
}
const sourceRoot = path.join(extension, "src", "test");
const compiledRoot = path.join(extension, "out", "test");
const sourceCanonical = listFiles(sourceRoot)
  .filter((file) => file.endsWith(".test.ts") && !file.startsWith("multiFolder/"))
  .map((file) => file.replace(/\.ts$/, ".js"));
const canonicalMulti = listFiles(path.join(sourceRoot, "multiFolder"))
  .filter((file) => file.endsWith(".test.ts"))
  .map((file) => file.replace(/\.ts$/, ".js"))
  .sort();
if (JSON.stringify(manifest.multi_folder_files) !== JSON.stringify(canonicalMulti)) {
  throw new Error("manifest multi-folder inventory differs from tracked src/test inventory");
}
const compiledCanonical = listFiles(compiledRoot).filter(
  (file) => file.endsWith(".test.js") && !file.startsWith("multiFolder/"),
);
const canonical = sourceCanonical.sort();
if (compiledCanonical.length > 0 && JSON.stringify(compiledCanonical.sort()) !== JSON.stringify(canonical)) {
  throw new Error("compiled out/test inventory differs from tracked src/test inventory");
}
const seen = new Map();
for (const [partition, files] of Object.entries(manifest.partitions)) {
  if (!Array.isArray(files) || files.length === 0) {
    throw new Error(`partition ${partition} is empty`);
  }
  for (const file of files) {
    if (!canonical.includes(file)) throw new Error(`non-canonical file: ${file}`);
    if (seen.has(file)) throw new Error(`duplicate file ${file}`);
    seen.set(file, partition);
  }
}
if (seen.size !== canonical.length || canonical.some((file) => !seen.has(file))) {
  throw new Error("partition manifest does not cover the canonical glob exactly once");
}
if (seen.get("serverHealth.test.js") !== "1") {
  throw new Error("serverHealth.test.js must run exactly once in partition 1");
}
if (
  manifest.expected_tests?.single_root?.identities !== 978 ||
  manifest.expected_tests?.single_root?.passed !== 977 ||
  manifest.expected_tests?.single_root?.pending !== 1 ||
  manifest.expected_tests?.multi_folder?.identities !== 14 ||
  manifest.expected_tests?.multi_folder?.passed !== 14 ||
  manifest.expected_tests?.multi_folder?.pending !== 0
) {
  throw new Error("manifest expected test counts drifted");
}
console.log(JSON.stringify({
  partitions: manifest.partitions,
  files: canonical.length,
  exact_once: true,
}));
