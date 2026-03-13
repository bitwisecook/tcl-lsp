#!/usr/bin/env node
// Generates coverage reports from V8 coverage data using c8's Report class directly.
// This bypasses c8's CLI (and its yargs dependency) which is incompatible with Node.js v25+.
"use strict";

const Report = require("c8/lib/report");
const path = require("path");

const rootDir = path.resolve(__dirname, "../../..");
const covDir = path.join(rootDir, "tmp", "coverage");

async function main() {
  const report = new Report({
    tempDirectory: path.join(covDir, ".v8-coverage-vscode"),
    reportsDirectory: path.join(covDir, "vscode"),
    reporter: ["html", "text"],
    src: [path.resolve(__dirname, "..", "src")],
    all: true,
    include: ["out/**/*.js"],
    exclude: ["out/test/**"],
    excludeNodeModules: true,
  });
  await report.run();
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
