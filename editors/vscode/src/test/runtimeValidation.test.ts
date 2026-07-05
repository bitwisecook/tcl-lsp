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
  buildRuntimeValidationChecker,
  resolveRuntimeValidationAdapter,
  runtimeValidationAdapterLabel,
} from "../runtimeValidation";

suite("Runtime Validation", () => {
  test("auto adapter selects iRules mode for f5-irules dialect", () => {
    const adapter = resolveRuntimeValidationAdapter("auto", "f5-irules");
    assert.strictEqual(adapter, "irules-stub");
  });

  test("auto adapter selects Tcl syntax mode for Tcl dialects", () => {
    const adapter = resolveRuntimeValidationAdapter("auto", "tcl8.6");
    assert.strictEqual(adapter, "tcl-syntax");
  });

  test("explicit adapter mode overrides dialect", () => {
    const adapter = resolveRuntimeValidationAdapter("tcl-syntax", "f5-irules");
    assert.strictEqual(adapter, "tcl-syntax");
  });

  test("iRules checker script contains when stub validation", () => {
    const script = buildRuntimeValidationChecker("irules-stub");
    assert.ok(script.includes("proc when {event args}"), "Expected when stub");
    assert.ok(script.includes("uplevel #0 $script"), "Expected top-level evaluation");
  });

  test("Tcl syntax checker script performs completeness check only", () => {
    const script = buildRuntimeValidationChecker("tcl-syntax");
    assert.ok(script.includes("info complete $script"), "Expected info complete guard");
    assert.ok(!script.includes("proc when {event args}"), "Unexpected when stub");
  });

  test("adapter labels are user friendly", () => {
    assert.strictEqual(runtimeValidationAdapterLabel("tcl-syntax"), "Tcl syntax adapter");
    assert.strictEqual(runtimeValidationAdapterLabel("irules-stub"), "iRules stub adapter");
  });
});
