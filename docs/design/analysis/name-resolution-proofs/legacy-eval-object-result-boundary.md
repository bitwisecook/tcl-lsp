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

## Exact evidence

- `v5-probe.c` (input): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/probe.c](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/probe.c). SHA-256 `25edafe870b033baf0e17f59ddaf9800db285b34025964027ab1811e8a6ed249`. Exact retained input/runner or aggregate native build/process/stream association. Prior variants are immutable and are not interchangeable.
- `v5-capture.py` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/capture.py](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/capture.py). SHA-256 `606a6e897917f55b5a4dfb9091de2fc53e830bb4ce74ac8775b1cf9a999c3a5e`. Exact retained input/runner or aggregate native build/process/stream association. Prior variants are immutable and are not interchangeable.
- `v5-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/receipt.json). SHA-256 `98a91bd5e621163b911eba8eb4191f3d908cd0f86eb336d142e3b1bc343f717a`. Exact retained input/runner or aggregate native build/process/stream association. Prior variants are immutable and are not interchangeable.
- `v5-tcl8.4-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.4.20/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.4.20/receipt.json). SHA-256 `45df0b43cb29f055401c4899429e5ce37f8473b438a347845d186ac8e4312283`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl8.4-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.4.20/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.4.20/stdout.tsv). SHA-256 `46f05ae01d71cf78a58b0557e6cab7ddc56fe225a9872ef14387c1c9227f2fc3`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl8.4-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.4.20/stderr](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl8.5-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.5.19/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.5.19/receipt.json). SHA-256 `c5ac0fb1cf13a5f4b57acbed242afb3f5a00719b69af707875c8b80b6eb4641c`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl8.5-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.5.19/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.5.19/stdout.tsv). SHA-256 `6763de318120d961bc97b57f9f5476dfdd1a9bc433a782898d4c618ca28564d0`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl8.5-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.5.19/stderr](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl8.6-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.6.18/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.6.18/receipt.json). SHA-256 `c271f1ce05a3d3ff97058d4d5f0e1fbac96acc84dac7f669746b04b664321beb`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl8.6-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.6.18/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.6.18/stdout.tsv). SHA-256 `cc03f55a9c1be04f39b9d5f5059cd53a7450ff1d8e337673501402335547eb5e`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl8.6-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.6.18/stderr](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl9.0-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.0.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.0.4/receipt.json). SHA-256 `ee81546200eaf14316446ebaab4847d60b72b236da40678e9a2b01cbeb66c638`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl9.0-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.0.4/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.0.4/stdout.tsv). SHA-256 `86b6e7570ffae48cdd4dc09b9c079815f49999e719469e255a1b0e4cfa27d1d1`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl9.0-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.0.4/stderr](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl9.1-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.1.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.1.0/receipt.json). SHA-256 `26de69e0e7dbf69d3a8d7cc78fd494f23fe7fdde0e1096daf6d07ce177274c9a`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl9.1-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.1.0/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.1.0/stdout.tsv). SHA-256 `f977a2fdbf22fba72b128f5c88a87fd61336ffa219de648e4b5d48de3cedffdd`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-tcl9.1-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.1.0/stderr](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-jim-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/jim/receipt.json). SHA-256 `a17e0930bf0d234b8aa2f5e1b78f713b9796853c5789c49e2f89d7fe001f194e`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-jim-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/jim/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/jim/stdout.tsv). SHA-256 `5707d1b91c4f468cfe79a38218291016830110fb0bc37c8d942c2c7eff73518a`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v5-jim-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/jim/stderr](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `source-tcl8.4` (source-anchor): [rust/tcl-vm/tests/data/native_oo_resident_keys/source-anchors/8.4.20.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/source-anchors/8.4.20.json). SHA-256 `04726504ae24c17a4f32e699f4f1f3f81fec9f8d2536924ffaf7092261cf9aa5`. JSON pointer `/snippet`. Exact pinned C wrapper or macro excerpt; source inspection is separate from native output.
- `source-tcl8.5` (source-anchor): [rust/tcl-vm/tests/data/native_oo_resident_keys/source-anchors/8.5.19.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/source-anchors/8.5.19.json). SHA-256 `c3c7db42c1c6840c9adb144a80d280fc2e5fe4993c6007a38ec8f77f95f4fc24`. JSON pointer `/snippet`. Exact pinned C wrapper or macro excerpt; source inspection is separate from native output.
- `source-tcl8.6` (source-anchor): [rust/tcl-vm/tests/data/native_oo_resident_keys/source-anchors/8.6.18.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/source-anchors/8.6.18.json). SHA-256 `b7a12b4ce1ba3f1f02e52e7873497313c37ec93840c408e208a6ccac6bdaf777`. JSON pointer `/snippet`. Exact pinned C wrapper or macro excerpt; source inspection is separate from native output.
- `source-tcl9.0` (source-anchor): [rust/tcl-vm/tests/data/native_oo_resident_keys/source-anchors/9.0.4.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/source-anchors/9.0.4.json). SHA-256 `9bb8f2376460d986c1171851da989e7a591ffbb5a8ebab52e11abd6ae857c79c`. JSON pointer `/snippet`. Exact pinned C wrapper or macro excerpt; source inspection is separate from native output.
- `source-tcl9.1` (source-anchor): [rust/tcl-vm/tests/data/native_oo_resident_keys/source-anchors/9.1.0.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/source-anchors/9.1.0.json). SHA-256 `5abab9933dd9a6155e916667566808495ca26e6ffd6c99d0bc5110ebf0700fd1`. JSON pointer `/snippet`. Exact pinned C wrapper or macro excerpt; source inspection is separate from native output.

## Source inspection

tcl8.4 8.4.20, revision `Pinned Tcl 8.4.20`, `tmp/tcl8.4.20/generic/tclBasic.c`, function `Tcl_Eval`, lines 4907–4923. Full-source SHA-256 `cab6d4decd1a3e365fe3cef49a2d5430dfef01cd775dd7d34f98064032f3a81a`; snippet SHA-256 `811e2735c35e1bf034ba30b536da52a0d31023f093c8bb6708c94c4805d2e83d`; retained evidence `source-tcl8.4`.

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

tcl8.5 8.5.19, revision `Pinned Tcl 8.5.19`, `tmp/tcl8.5.19/generic/tclBasic.c`, function `Tcl_Eval`, lines 5000–5015. Full-source SHA-256 `d09c16386ea0376dc3590a74ded2c6e2bcef5079f2b1a3785545bad37c7ead63`; snippet SHA-256 `677517a21d7196b16bc9d8f8139f9b5df9cd3106b55909dfd029c13b641471c0`; retained evidence `source-tcl8.5`.

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

tcl8.6 8.6.18, revision `Pinned Tcl 8.6.18`, `tmp/tcl8.6.18/generic/tclBasic.c`, function `Tcl_Eval`, lines 5997–6012. Full-source SHA-256 `19e28c2c9fbcd27e7e538a06960f0d806d04f8c25d94499cf9eb9031f8440198`; snippet SHA-256 `3ea3db03130a6e6cda21be9789421f318eef1cb3c1e7cede5d9ec1fb796de5f8`; retained evidence `source-tcl8.6`.

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

tcl9.0 9.0.4, revision `Pinned Tcl 9.0.4`, `tmp/tcl9.0.4/generic/tclDecls.h`, function `Tcl_Eval macro`, lines 3966–3967. Full-source SHA-256 `28ad708ea0738fbf8bfb13dcde87ea8064220982d29b8c082df87c7987ff2c70`; snippet SHA-256 `b59b8f3c6608f4e792cd283abde65ab1cff1f414be3f8f124d194ac4d56d18bb`; retained evidence `source-tcl9.0`.

```text
#define Tcl_Eval(interp, objPtr) \
	Tcl_EvalEx(interp, objPtr, -1, 0)

```

tcl9.1 9.1.0, revision `Pinned Tcl 9.1.0`, `tmp/tcl9.1.0/generic/tclDecls.h`, function `Tcl_Eval macro`, lines 4085–4086. Full-source SHA-256 `590a2f27f095e7150bfb648f43d00c9a9ec5dbb2b936e413b551f663dc972142`; snippet SHA-256 `50cde600478b3551463746d96b83ba951a98a33fb5793926b165d8ae364f9342`; retained evidence `source-tcl9.1`.

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
