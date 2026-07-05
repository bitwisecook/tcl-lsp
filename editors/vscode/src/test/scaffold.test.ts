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

import * as assert from "assert";
import {
  buildPackageScaffold,
  namespaceFromPackageName,
  packageDirectoryName,
  validateCommandName,
  validatePackageName,
  validateVersion,
} from "../scaffold";

suite("Package Scaffold", () => {
  test("validates package names", () => {
    assert.strictEqual(validatePackageName("acme::tools"), undefined);
    assert.ok(validatePackageName("9bad"));
    assert.ok(validatePackageName("bad name"));
  });

  test("validates command names", () => {
    assert.strictEqual(validateCommandName("hello_world"), undefined);
    assert.ok(validateCommandName("hello-world"));
    assert.ok(validateCommandName("9hello"));
  });

  test("validates versions", () => {
    assert.strictEqual(validateVersion("1.0.0"), undefined);
    assert.ok(validateVersion("v1"));
    assert.ok(validateVersion("1.0-beta"));
  });

  test("derives namespace and directory names", () => {
    assert.strictEqual(namespaceFromPackageName("acme::http-utils"), "::acme::http_utils");
    assert.strictEqual(packageDirectoryName("acme::http-utils"), "acme__http-utils");
  });

  test("builds package scaffold files", () => {
    const plan = buildPackageScaffold({
      packageName: "acme::tools",
      version: "0.1.0",
      commandName: "hello",
      minimumTclVersion: "8.6",
    });
    const paths = plan.files.map((file) => file.relativePath);
    assert.ok(paths.includes("pkgIndex.tcl"));
    assert.ok(paths.includes("src/acme__tools.tcl"));
    assert.ok(paths.includes("tests/acme__tools.test.tcl"));
    assert.ok(paths.includes("tests/run.tcl"));
    assert.ok(paths.includes(".github/workflows/ci.yml"));
    assert.ok(paths.includes("README.md"));
  });

  test("includes CI test invocation and package metadata", () => {
    const plan = buildPackageScaffold({
      packageName: "acme::tools",
      version: "0.1.0",
      commandName: "hello",
    });
    const files = new Map(plan.files.map((file) => [file.relativePath, file.content]));
    const workflow = files.get(".github/workflows/ci.yml");
    const src = files.get("src/acme__tools.tcl");
    assert.ok(workflow && workflow.includes("tclsh tests/run.tcl"));
    assert.ok(src && src.includes("package provide acme::tools 0.1.0"));
  });
});
