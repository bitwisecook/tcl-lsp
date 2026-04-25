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
