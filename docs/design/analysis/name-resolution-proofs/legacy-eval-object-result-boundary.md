# naming.interpreter.legacy-eval-object-result-boundary

Kind: `native-observation`

## Problem statement

After a raw counted-zero variable name is reported through info object vars, a shortened list element can mean either that storage clipped the key or that the evaluator changed the result representation. Comparing Tcl_Eval results with object-vector dispatch would confuse those interpretations. This check compares the same reporter through three public evaluation APIs; it applies to the captured TclOO cases and does not attest unknown object getters or resident key identity.

## Question

For the captured counted TclOO variable names, does Tcl_Eval followed by Tcl_GetObjResult preserve the same reported list-element bytes as Tcl_EvalEx and Tcl_EvalObjv?

## Conclusion

C8.6 reports raw-zero k through the legacy Tcl_Eval result boundary but retains full k<00>tail through Tcl_EvalEx and Tcl_EvalObjv. C9.0/C9.1 retain the full counted bytes through all three APIs. Encoded C080 is distinct from raw zero. Source inspection separately selects the C8 legacy CString mirror and C9 direct EvalEx macro. Old-C/Jim TclOO-unavailable controls do not measure this API comparison there; no production namespace-key clipping or general object-getter permission follows.

## Scope

Six fixed declaration/primary arrangements (plain, raw zero, qualifier-after-zero, D800, FF, prefix negative), exact original v1..v5 inputs/observer chronology and API windows. C86/C90/C91 supported OO, C84/C85/Jim explicit unavailable OO controls. v3 C86 failed compile is retained separately.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: 8.4.20. Build: Actual executable SHA256 eab6a1a03b47e6b2eb864d67b9a84d7478dd7b3233a5bd97bffb58f6c152c2ff; retained compile command, header/static-library/Makefile/source-owner hashes and process streams.. Channel: Original counted declaration/method objects and Tcl_EvalObjv calls; ASCII Tcl_Eval/Tcl_EvalEx reporter source; actual ListObjIndex child bytes.. Dialect: Tcl.

Captured OO_UNAVAILABLE control; no TclOO resident-key/API comparison executed for this provider.

### tcl8.5

Status: `not-tested`. Version: 8.5.19. Build: Actual executable SHA256 97f9da400f3e8a3101acb93e046586278ff17750345925668421cd4951bfc8bb; retained compile command, header/static-library/Makefile/source-owner hashes and process streams.. Channel: Original counted declaration/method objects and Tcl_EvalObjv calls; ASCII Tcl_Eval/Tcl_EvalEx reporter source; actual ListObjIndex child bytes.. Dialect: Tcl.

Captured OO_UNAVAILABLE control; no TclOO resident-key/API comparison executed for this provider.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable SHA256 18ce6fb3d479bf22364b8f9a43c3fb4604f0811d5cc096724ad1cd8c5e396f98; retained compile command, header/static-library/Makefile/source-owner hashes and process streams.. Channel: Original counted declaration/method objects and Tcl_EvalObjv calls; ASCII Tcl_Eval/Tcl_EvalEx reporter source; actual ListObjIndex child bytes.. Dialect: Tcl.

Legacy Tcl_Eval raw-zero report leaf is 6b; EvalEx and object-vector leaf are 6b007461696c.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable SHA256 b4392c79823d600b58213f4b0f8bba51e8eba7de8d075806c2cd324cb4e4be6b; retained compile command, header/static-library/Makefile/source-owner hashes and process streams.. Channel: Original counted declaration/method objects and Tcl_EvalObjv calls; ASCII Tcl_Eval/Tcl_EvalEx reporter source; actual ListObjIndex child bytes.. Dialect: Tcl.

All three raw-zero report leaves are 6b007461696c.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable SHA256 cf17f9d47b36183f64068dc69a39bd653b95c1c470ffc9e77e1cdbe45d628fa6; retained compile command, header/static-library/Makefile/source-owner hashes and process streams.. Channel: Original counted declaration/method objects and Tcl_EvalObjv calls; ASCII Tcl_Eval/Tcl_EvalEx reporter source; actual ListObjIndex child bytes.. Dialect: Tcl.

All three raw-zero report leaves are 6b007461696c.

### jim

Status: `not-tested`. Version: 0.84-9-g5bac7c9. Build: Actual executable SHA256 a949f19cd5443db661617c44a2cd057be0d666de21a9d28b2bef728ba16cd7bf; retained compile command, header/static-library/Makefile/source-owner hashes and process streams.. Channel: Original counted declaration/method objects and Tcl_EvalObjv calls; ASCII Tcl_Eval/Tcl_EvalEx reporter source; actual ListObjIndex child bytes.. Dialect: Jim Tcl.

Captured OO_UNAVAILABLE control; no TclOO resident-key/API comparison executed for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation for this question.

## Historical references

- `v5-probe.c` (input): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/probe.c](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/probe.c). Exact retained input/runner or aggregate native build/process/stream association. Prior variants are immutable and are not interchangeable.
- `v5-capture.py` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/capture.py](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/capture.py). Exact retained input/runner or aggregate native build/process/stream association. Prior variants are immutable and are not interchangeable.
- `v5-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/receipt.json). Exact retained input/runner or aggregate native build/process/stream association. Prior variants are immutable and are not interchangeable.
- `v5-tcl8.4-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.4.20/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.4.20/receipt.json). Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl8.4-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.4.20/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.4.20/stdout.tsv). Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl8.4-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.4.20/stderr](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.4.20/stderr). Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl8.5-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.5.19/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.5.19/receipt.json). Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl8.5-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.5.19/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.5.19/stdout.tsv). Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl8.5-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.5.19/stderr](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.5.19/stderr). Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl8.6-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.6.18/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.6.18/receipt.json). Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl8.6-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.6.18/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.6.18/stdout.tsv). Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl8.6-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.6.18/stderr](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.6.18/stderr). Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl9.0-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.0.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.0.4/receipt.json). Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl9.0-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.0.4/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.0.4/stdout.tsv). Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl9.0-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.0.4/stderr](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.0.4/stderr). Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl9.1-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.1.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.1.0/receipt.json). Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl9.1-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.1.0/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.1.0/stdout.tsv). Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl9.1-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.1.0/stderr](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.1.0/stderr). Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-jim-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/jim/receipt.json). Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-jim-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/jim/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/jim/stdout.tsv). Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-jim-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/jim/stderr](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/jim/stderr). Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `source-tcl8.4` (source-anchor): [rust/tcl-vm/tests/data/native_oo_resident_keys/source-anchors/8.4.20.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/source-anchors/8.4.20.json). JSON pointer `/snippet`. Exact pinned C wrapper or macro excerpt; source inspection is separate from native output.
- `source-tcl8.5` (source-anchor): [rust/tcl-vm/tests/data/native_oo_resident_keys/source-anchors/8.5.19.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/source-anchors/8.5.19.json). JSON pointer `/snippet`. Exact pinned C wrapper or macro excerpt; source inspection is separate from native output.
- `source-tcl8.6` (source-anchor): [rust/tcl-vm/tests/data/native_oo_resident_keys/source-anchors/8.6.18.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/source-anchors/8.6.18.json). JSON pointer `/snippet`. Exact pinned C wrapper or macro excerpt; source inspection is separate from native output.
- `source-tcl9.0` (source-anchor): [rust/tcl-vm/tests/data/native_oo_resident_keys/source-anchors/9.0.4.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/source-anchors/9.0.4.json). JSON pointer `/snippet`. Exact pinned C wrapper or macro excerpt; source inspection is separate from native output.
- `source-tcl9.1` (source-anchor): [rust/tcl-vm/tests/data/native_oo_resident_keys/source-anchors/9.1.0.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/source-anchors/9.1.0.json). JSON pointer `/snippet`. Exact pinned C wrapper or macro excerpt; source inspection is separate from native output.

## Source inspection

tcl8.4 8.4.20, revision `Pinned Tcl 8.4.20`, `tmp/tcl8.4.20/generic/tclBasic.c`, function `Tcl_Eval`, lines 4907–4923. Retained reference `source-tcl8.4`.

```text
Tcl_Eval(interp, string)
    Tcl_Interp *interp;		/* Token for command interpreter (returned
				 * by previous call to Tcl_CreateInterp). */
    CONST char *string;		/* Pointer to TCL command to execute. */
{
    int code = Tcl_EvalEx(interp, string, -1, 0);

    /*
     * For backwards compatibility with old C code that predates the
     * object system in Tcl 8.0, we have to mirror the object result
     * back into the string result (some callers may expect it there).
     */

    Tcl_SetResult(interp, TclGetString(Tcl_GetObjResult(interp)),
	    TCL_VOLATILE);
    return code;
}
```

tcl8.5 8.5.19, revision `Pinned Tcl 8.5.19`, `tmp/tcl8.5.19/generic/tclBasic.c`, function `Tcl_Eval`, lines 5000–5015. Retained reference `source-tcl8.5`.

```text
Tcl_Eval(
    Tcl_Interp *interp,		/* Token for command interpreter (returned by
				 * previous call to Tcl_CreateInterp). */
    const char *script)		/* Pointer to TCL command to execute. */
{
    int code = Tcl_EvalEx(interp, script, -1, 0);

    /*
     * For backwards compatibility with old C code that predates the object
     * system in Tcl 8.0, we have to mirror the object result back into the
     * string result (some callers may expect it there).
     */

    (void) Tcl_GetStringResult(interp);
    return code;
}
```

tcl8.6 8.6.18, revision `Pinned Tcl 8.6.18`, `tmp/tcl8.6.18/generic/tclBasic.c`, function `Tcl_Eval`, lines 5997–6012. Retained reference `source-tcl8.6`.

```text
Tcl_Eval(
    Tcl_Interp *interp,		/* Token for command interpreter (returned by
				 * previous call to Tcl_CreateInterp). */
    const char *script)		/* Pointer to TCL command to execute. */
{
    int code = Tcl_EvalEx(interp, script, -1, 0);

    /*
     * For backwards compatibility with old C code that predates the object
     * system in Tcl 8.0, we have to mirror the object result back into the
     * string result (some callers may expect it there).
     */

    (void)Tcl_GetStringResult(interp);
    return code;
}
```

tcl9.0 9.0.4, revision `Pinned Tcl 9.0.4`, `tmp/tcl9.0.4/generic/tclDecls.h`, function `Tcl_Eval macro`, lines 3966–3967. Retained reference `source-tcl9.0`.

```text
#define Tcl_Eval(interp, objPtr) \
	Tcl_EvalEx(interp, objPtr, -1, 0)

```

tcl9.1 9.1.0, revision `Pinned Tcl 9.1.0`, `tmp/tcl9.1.0/generic/tclDecls.h`, function `Tcl_Eval macro`, lines 4085–4086. Retained reference `source-tcl9.1`.

```text
#define Tcl_Eval(interp, objPtr) \
	Tcl_EvalEx(interp, objPtr, TCL_INDEX_NONE, 0)

```


## Consumer bindings

- [rust/tcl-syntax/src/native_string.rs](../../../../rust/tcl-syntax/src/native_string.rs), `NativeStringProtocol::legacy_eval_result_input`: Pure selected legacy C result extent; does not mutate counted results or stored variable keys.
- [rust/tcl-vm/src/cmd_oo/native_variables.rs](../../../../rust/tcl-vm/src/cmd_oo/native_variables.rs), `cmd_oo::native_variables::tests::compiled_and_dynamic_declared_variables_match_original_native_controls` (linked): Compares original counted dispatch results with the legacy result projection only for rows actually captured through Tcl_Eval, and independently asserts v5 object-vector bytes without projection. No execution is inferred.
- [runtime/rust/src/cmd_oo/native_variables.rs](../../../../runtime/rust/src/cmd_oo/native_variables.rs), `cmd_oo::native_variables::tests::compiled_and_dynamic_declared_variables_match_original_native_controls` (linked): Compares original counted dispatch results with the legacy result projection only for rows actually captured through Tcl_Eval, and independently asserts v5 object-vector bytes without projection. No execution is inferred.
- [rust/tcl-syntax/src/native_string.rs](../../../../rust/tcl-syntax/src/native_string.rs), `native_string::tests::legacy_eval_result_extent_keeps_encoded_zero_separate_from_counted_zero` (linked): Pure C API result extent selection distinguishes raw zero from encoded C080; Jim C API purpose is unavailable. Source-only old C recipe checks are not empirical old C TclOO comparisons.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original runners contain captured workspace paths and refuse output-directory reuse. Exact input/source/build/executable/stdout/stderr hashes remain in immutable receipts. No portable re-launch or Rust execution result is inferred; v3 failed compile produced no C86 guest rows.
