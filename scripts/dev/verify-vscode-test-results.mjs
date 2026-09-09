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
const results = path.resolve(process.argv[3] ?? root);
const manifest = parseJsonNoDuplicateKeys(
  fs.readFileSync(path.join(root, "editors/vscode/test-partitions.json"), "utf8"),
  "VS Code test partition manifest",
);
if (
  manifest.version !== 1 ||
  manifest.canonical_glob !== "**/*.test.js" ||
  JSON.stringify(manifest.exclude) !== JSON.stringify(["multiFolder/**"]) ||
  JSON.stringify(Object.keys(manifest.partitions).sort()) !== JSON.stringify(["1", "2", "3"])
) {
  throw new Error("unsupported VS Code test partition manifest schema");
}
const expected = new Set(Object.values(manifest.partitions).flat());
if (
  manifest.expected_tests?.single_root?.identities !== 976 ||
  manifest.expected_tests?.single_root?.passed !== 975 ||
  manifest.expected_tests?.single_root?.pending !== 1 ||
  manifest.expected_tests?.multi_folder?.identities !== 14 ||
  manifest.expected_tests?.multi_folder?.passed !== 14 ||
  manifest.expected_tests?.multi_folder?.pending !== 0
) {
  throw new Error("manifest expected test counts drifted");
}
const files = new Set();
const identities = new Set();
let tests = 0;
function validateIdentities(result, assigned, label) {
  if (!Array.isArray(result.testIdentities) || result.testIdentities.length !== result.testsCompleted) {
    throw new Error(`${label} test identity count does not match completed tests`);
  }
  const validate = (identity) => {
    if (typeof identity !== "string" || !identity.includes(":")) {
      throw new Error(`${label} has malformed test identity`);
    }
    const separator = identity.indexOf(":");
    const file = identity.slice(0, separator);
    const title = identity.slice(separator + 1);
    if (!file || !title || !assigned.has(file)) {
      throw new Error(`${label} has incomplete or wrong-file test identity: ${identity}`);
    }
    return identity;
  };
  for (const identity of result.testIdentities) validate(identity);
  if (!Array.isArray(result.discoveredTestIdentities) || result.discoveredTestIdentities.length !== result.testIdentities.length) {
    throw new Error(`${label} discovered test identity count does not match completed tests`);
  }
  for (const identity of result.discoveredTestIdentities) validate(identity);
  const discovered = new Set(result.discoveredTestIdentities);
  if (discovered.size !== result.discoveredTestIdentities.length || discovered.size !== result.testIdentities.length || [...discovered].some((identity) => !result.testIdentities.includes(identity))) {
    throw new Error(`${label} discovered tests do not exactly match completed tests`);
  }
}
function validateOutcomes(result, expected, label) {
  for (const field of ["failures", "testsStarted", "testsCompleted", "testsPassed", "testsPending"]) {
    if (!Number.isInteger(result[field]) || result[field] < 0) {
      throw new Error(`${label} has invalid ${field}`);
    }
  }
  if (
    result.failures !== 0 ||
    result.testsStarted !== result.testsCompleted ||
    result.testsPassed + result.testsPending + result.failures !== result.testsCompleted
  ) {
    throw new Error(`${label} outcome counts drifted or did not complete`);
  }
  if (
    expected &&
    (result.testsCompleted !== expected.identities ||
      result.testsPassed !== expected.passed ||
      result.testsPending !== expected.pending)
  ) {
    throw new Error(`${label} expected outcome counts drifted`);
  }
}
let testsPassed = 0;
let testsPending = 0;
for (const partition of Object.keys(manifest.partitions)) {
  const file = path.join(results, `partition-${partition}`, "mocha-result.json");
  const result = parseJsonNoDuplicateKeys(fs.readFileSync(file, "utf8"), `partition ${partition} result`);
  validateOutcomes(result, undefined, `partition ${partition}`);
  if (result.partition?.index !== Number(partition) || result.partition?.count !== Object.keys(manifest.partitions).length) {
    throw new Error(`partition ${partition} metadata does not identify its declared index/count`);
  }
  const assigned = new Set(manifest.partitions[partition]);
  const resultFiles = new Set(result.files);
  if (resultFiles.size !== assigned.size || [...assigned].some((name) => !resultFiles.has(name))) {
    throw new Error(`partition ${partition} result files do not match its manifest assignment`);
  }
  if (!result.fileDurationsMs || Object.entries(result.fileDurationsMs).some(([name, ms]) => !assigned.has(name) || !Number.isFinite(ms) || ms < 0)) {
    throw new Error(`partition ${partition} file duration metadata is invalid`);
  }
  if (JSON.stringify(Object.keys(result.fileDurationsMs).sort()) !== JSON.stringify([...assigned].sort())) {
    throw new Error(`partition ${partition} file duration metadata is incomplete`);
  }
  validateIdentities(result, assigned, `partition ${partition}`);
  for (const identity of result.testIdentities) {
    const separator = identity.indexOf(":");
    const file = identity.slice(0, separator);
    if (identities.has(identity)) throw new Error(`duplicate test identity: ${identity}`);
    identities.add(identity);
  }
  for (const name of result.files) {
    if (files.has(name)) throw new Error(`duplicate result file: ${name}`);
    files.add(name);
  }
  tests += result.testsCompleted;
  testsPassed += result.testsPassed;
  testsPending += result.testsPending;
}
validateOutcomes(
  {
    failures: 0,
    testsStarted: tests,
    testsCompleted: tests,
    testsPassed,
    testsPending,
  },
  manifest.expected_tests.single_root,
  "single-root aggregate",
);
const multiFile = path.join(results, "multi-folder", "mocha-result-multifolder.json");
const multi = parseJsonNoDuplicateKeys(fs.readFileSync(multiFile, "utf8"), "multi-folder result");
validateOutcomes(multi, manifest.expected_tests.multi_folder, "multi-folder suite");
const multiExpectedFiles = new Set(manifest.multi_folder_files);
if (!Array.isArray(multi.files) || multi.files.length !== multiExpectedFiles.size || JSON.stringify([...new Set(multi.files)].sort()) !== JSON.stringify([...multiExpectedFiles].sort())) {
  throw new Error("multi-folder result files do not match manifest inventory");
}
validateIdentities(multi, multiExpectedFiles, "multi-folder");
if (new Set(multi.testIdentities).size !== multi.testIdentities.length) throw new Error("duplicate multi-folder test identity");
if (!multi.fileDurationsMs || Object.entries(multi.fileDurationsMs).some(([, ms]) => !Number.isFinite(ms) || ms < 0)) {
  throw new Error("multi-folder file duration metadata is invalid");
}
if (JSON.stringify(Object.keys(multi.fileDurationsMs).sort()) !== JSON.stringify([...multiExpectedFiles].sort())) {
  throw new Error("multi-folder file duration metadata is incomplete");
}
if (files.size !== expected.size || [...expected].some((name) => !files.has(name))) {
  throw new Error("partition result files do not cover the manifest exactly once");
}
if (tests !== manifest.expected_tests.single_root.identities) {
  throw new Error(`single-root identity count drifted: expected ${manifest.expected_tests.single_root.identities}, got ${tests}`);
}
console.log(JSON.stringify({
  exact_once: true,
  files: files.size,
  single_root_identities: tests,
  single_root_passed: manifest.expected_tests.single_root.passed,
  single_root_pending: manifest.expected_tests.single_root.pending,
  multi_folder_identities: multi.testsCompleted,
  multi_folder_passed: multi.testsPassed,
  multi_folder_pending: multi.testsPending,
}));
