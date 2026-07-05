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
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

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
