# naming.interpreter.deleted-child-evaluation-and-recreation

Kind: `native-observation`

## Problem statement

A strong activation lease preserves an old interpreter allocation while its deleted flag still forbids later evaluation.

## Question

What completion does a deleted active child produce, and does a same-name recreated child retain the old allocation or variables?

## Conclusion

All five C providers return error 1 and attempt to call eval in deleted interpreter when the retired child continues. The parent owns a distinct recreated child whose before variable is absent. The preserved C public API control independently records old deleted=1, replacement present=1, different allocation=1 and after-return result 1 0. C8.4 exposes no public GetReturnOptions here;85+ retain the exact error options. An activation lease grants storage lifetime only.

## Scope

Exact counted ASCII source passed to Tcl_EvalEx or original Jim_NewStringObj/Jim_EvalObj on a fresh interpreter per source control. C public active-allocation control preserves Tcl_GetSlave actual interpreter through deletion. Finite cases: v1:active-child-script-recreation, v2:active-child-continuation-errors, v2:active-child-replacement-survives-error, v1:active-original-allocation, v2:active-original-allocation. Result primary/resident fields precede the public getter; retained fields do not independently prove object release/callback closure. No Rust execution or whole lifecycle Normal is inferred.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual executable SHA256 509624f56dea942795cd07b532ea4613cd0363997110bcfd30985424b2ffc013 linked to library SHA256 532be0a794ca8277c5ad979a227c38600f27c3c91d436e5d3731d661c271fe47 and header SHA256 824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf; compiled/exited0.. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Dialect: Tcl.

All five C providers return error 1 and attempt to call eval in deleted interpreter when the retired child continues. The parent owns a distinct recreated child whose before variable is absent. The preserved C public API control independently records old deleted=1, replacement present=1, different allocation=1 and after-return result 1 0. C8.4 exposes no public GetReturnOptions here;85+ retain the exact error options. An activation lease grants storage lifetime only.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual executable SHA256 41a74a750080e2791043e268b93669648560ddf9aa2725980106f78d1b4f34b3 linked to library SHA256 94313727ea507b1b2b00018062a0f57587458a3509c37f2e4b351dd58d0592dc and header SHA256 c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5; compiled/exited0.. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Dialect: Tcl.

All five C providers return error 1 and attempt to call eval in deleted interpreter when the retired child continues. The parent owns a distinct recreated child whose before variable is absent. The preserved C public API control independently records old deleted=1, replacement present=1, different allocation=1 and after-return result 1 0. C8.4 exposes no public GetReturnOptions here;85+ retain the exact error options. An activation lease grants storage lifetime only.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable SHA256 f0538fc24be66e978ea8415681f801cce18ab296144df4bfe2e4cda86e506071 linked to library SHA256 980630e3a7d37f82322fda1a7e45a85b86d8d905cb395a2aa0c083b3c7eaa8cb and header SHA256 aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245; compiled/exited0.. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Dialect: Tcl.

All five C providers return error 1 and attempt to call eval in deleted interpreter when the retired child continues. The parent owns a distinct recreated child whose before variable is absent. The preserved C public API control independently records old deleted=1, replacement present=1, different allocation=1 and after-return result 1 0. C8.4 exposes no public GetReturnOptions here;85+ retain the exact error options. An activation lease grants storage lifetime only.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable SHA256 785e48c945aab19ea93ade52750dabf662265ba78c1b3a8f8a62997589eb5496 linked to library SHA256 dace08db925224497714ba86969fabb51c05a78f2bda22db6c05f795c3a70ae4 and header SHA256 eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a; compiled/exited0.. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Dialect: Tcl.

All five C providers return error 1 and attempt to call eval in deleted interpreter when the retired child continues. The parent owns a distinct recreated child whose before variable is absent. The preserved C public API control independently records old deleted=1, replacement present=1, different allocation=1 and after-return result 1 0. C8.4 exposes no public GetReturnOptions here;85+ retain the exact error options. An activation lease grants storage lifetime only.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable SHA256 9c84565330cbb35ae4b38b92e5d3caf11c5815a720f2a16ca8a3ebb361eb29de linked to library SHA256 513028850edf096856a29e4e16869aac91cba2ef75c3924aaaaeeb6e45e167db and header SHA256 30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950; compiled/exited0.. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Dialect: Tcl.

All five C providers return error 1 and attempt to call eval in deleted interpreter when the retired child continues. The parent owns a distinct recreated child whose before variable is absent. The preserved C public API control independently records old deleted=1, replacement present=1, different allocation=1 and after-return result 1 0. C8.4 exposes no public GetReturnOptions here;85+ retain the exact error options. An activation lease grants storage lifetime only.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual executable SHA256 fd015db448138b2bdfe3a5375caf4694ec48ca3851042ba63f6043333384431a linked to library SHA256 a692b833f5b8f3351c1dc5911810aadf4c5f0ff4ca795ee6b802e6550e57b3da and header SHA256 d9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d; compiled/exited0.. Channel: Original counted Jim string object passed to Jim_EvalObj; C API explicitly not tested. Dialect: Jim Tcl.

Actual C-shaped interp source controls each return1/wrong # args: should be "interp". The C public active-allocation API is explicitly not tested; no corresponding child behavior is inferred.

### bigip

Status: `not-tested`. Version: not recorded. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No appliance observation.

## Historical references

- `v1-protocol-probe.c` (input): [runtime/rust/tests/data/native_child_command_lifetime/v1/protocol/probe.c](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/protocol/probe.c). Exact retained v1 protocol/probe.c; each guest code is independent of compile/process exit.
- `v1-protocol-inputs.json` (input): [runtime/rust/tests/data/native_child_command_lifetime/v1/protocol/inputs.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/protocol/inputs.json). Exact retained v1 protocol/inputs.json; each guest code is independent of compile/process exit.
- `v1-protocol-cases.h` (input): [runtime/rust/tests/data/native_child_command_lifetime/v1/protocol/cases.h](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/protocol/cases.h). Exact retained v1 protocol/cases.h; each guest code is independent of compile/process exit.
- `v1-capture-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/receipt.json). Exact retained v1 capture/receipt.json; each guest code is independent of compile/process exit.
- `v2-protocol-probe.c` (input): [runtime/rust/tests/data/native_child_command_lifetime/v2/protocol/probe.c](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/protocol/probe.c). Exact retained v2 protocol/probe.c; each guest code is independent of compile/process exit.
- `v2-protocol-inputs.json` (input): [runtime/rust/tests/data/native_child_command_lifetime/v2/protocol/inputs.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/protocol/inputs.json). Exact retained v2 protocol/inputs.json; each guest code is independent of compile/process exit.
- `v2-protocol-cases.h` (input): [runtime/rust/tests/data/native_child_command_lifetime/v2/protocol/cases.h](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/protocol/cases.h). Exact retained v2 protocol/cases.h; each guest code is independent of compile/process exit.
- `v2-capture-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/receipt.json). Exact retained v2 capture/receipt.json; each guest code is independent of compile/process exit.
- `v1-tcl8.4-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.4.20/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.4.20/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.4-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.4.20/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.4.20/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.4-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.4.20/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.4.20/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.4-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.4.20/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.4.20/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.4-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.4.20/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.4.20/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.4-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.4.20/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.4.20/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.5-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.5.19/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.5.19/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.5-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.5.19/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.5.19/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.5-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.5.19/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.5.19/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.5-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.5.19/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.5.19/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.5-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.5.19/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.5.19/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.5-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.5.19/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.5.19/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.6-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.6.18/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.6.18/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.6-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.6.18/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.6.18/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.6-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.6.18/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.6.18/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.6-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.6.18/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.6.18/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.6-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.6.18/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.6.18/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.6-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.6.18/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.6.18/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl9.0-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.0.4/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.0.4/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl9.0-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.0.4/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.0.4/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl9.0-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.0.4/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.0.4/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl9.0-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.0.4/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.0.4/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl9.0-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.0.4/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.0.4/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl9.0-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.0.4/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.0.4/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl9.1-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.1.0/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.1.0/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl9.1-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.1.0/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.1.0/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl9.1-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.1.0/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.1.0/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl9.1-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.1.0/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.1.0/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl9.1-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.1.0/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.1.0/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl9.1-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.1.0/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.1.0/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-jim-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/jim/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/jim/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-jim-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/jim/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/jim/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-jim-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/jim/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/jim/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-jim-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/jim/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/jim/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-jim-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/jim/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/jim/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-jim-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/jim/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/jim/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `tcl8.4-deleted-eval` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl8.4-basic-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl8.4-basic-source.json). JSON pointer `/windows/1/snippet`. Pinned source-only C API publication or deleted evaluator ready check; no extra execution is inferred from this excerpt.
- `tcl8.5-deleted-eval` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl8.5-basic-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl8.5-basic-source.json). JSON pointer `/windows/1/snippet`. Pinned source-only C API publication or deleted evaluator ready check; no extra execution is inferred from this excerpt.
- `tcl8.6-deleted-eval` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl8.6-basic-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl8.6-basic-source.json). JSON pointer `/windows/1/snippet`. Pinned source-only C API publication or deleted evaluator ready check; no extra execution is inferred from this excerpt.
- `tcl9.0-deleted-eval` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl9.0-basic-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl9.0-basic-source.json). JSON pointer `/windows/1/snippet`. Pinned source-only C API publication or deleted evaluator ready check; no extra execution is inferred from this excerpt.
- `tcl9.1-deleted-eval` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl9.1-basic-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl9.1-basic-source.json). JSON pointer `/windows/1/snippet`. Pinned source-only C API publication or deleted evaluator ready check; no extra execution is inferred from this excerpt.

## Source inspection

tcl8.4 8.4.20, revision `8.4.20`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclBasic.c`, function `TclInterpReady`, lines 3062–3072. Retained reference `tcl8.4-deleted-eval`.

```text
     * If the interpreter has been deleted, return an error.
     */
    
    if (iPtr->flags & DELETED) {
	Tcl_ResetResult(interp);
	Tcl_AppendToObj(Tcl_GetObjResult(interp),
	        "attempt to call eval in deleted interpreter", -1);
	Tcl_SetErrorCode(interp, "CORE", "IDELETE",
	        "attempt to call eval in deleted interpreter",
		(char *) NULL);
	return TCL_ERROR;

```

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclBasic.c`, function `TclInterpReady`, lines 3498–3507. Retained reference `tcl8.5-deleted-eval`.

```text
     * If the interpreter has been deleted, return an error.
     */

    if (iPtr->flags & DELETED) {
	Tcl_ResetResult(interp);
	Tcl_AppendResult(interp,
		"attempt to call eval in deleted interpreter", NULL);
	Tcl_SetErrorCode(interp, "TCL", "IDELETE",
		"attempt to call eval in deleted interpreter", NULL);
	return TCL_ERROR;

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `TclInterpReady`, lines 3935–3943. Retained reference `tcl8.6-deleted-eval`.

```text
     * If the interpreter has been deleted, return an error.
     */

    if (iPtr->flags & DELETED) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to call eval in deleted interpreter", -1));
	Tcl_SetErrorCode(interp, "TCL", "IDELETE",
		"attempt to call eval in deleted interpreter", (char *)NULL);
	return TCL_ERROR;

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `TclInterpReady`, lines 4161–4169. Retained reference `tcl9.0-deleted-eval`.

```text
     * If the interpreter has been deleted, return an error.
     */

    if (iPtr->flags & DELETED) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to call eval in deleted interpreter", TCL_INDEX_NONE));
	Tcl_SetErrorCode(interp, "TCL", "IDELETE",
		"attempt to call eval in deleted interpreter", (char *)NULL);
	return TCL_ERROR;

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `TclInterpReady`, lines 4131–4139. Retained reference `tcl9.1-deleted-eval`.

```text
     * If the interpreter has been deleted, return an error.
     */

    if (iPtr->flags & DELETED) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to call eval in deleted interpreter", TCL_INDEX_NONE));
	Tcl_SetErrorCode(interp, "TCL", "IDELETE",
		"attempt to call eval in deleted interpreter", (char *)NULL);
	return TCL_ERROR;

```


## Consumer bindings

- [rust/tcl-registry/src/native_eval_object.rs](../../../../rust/tcl-registry/src/native_eval_object.rs), `NativeEvalObjectProtocol::deleted_interpreter_error`: Select only the C ready-check error message/code for a independently deleted actual interpreter.
- [runtime/rust/src/interp/native_children.rs](../../../../runtime/rust/src/interp/native_children.rs), `Interp::evaluation_is_live`: Keep actual activation lifetime independent from permission for further evaluation.
- [runtime/rust/src/interp/native_children.rs](../../../../runtime/rust/src/interp/native_children.rs), `interp::native_children::tests::child_source_controls_match_native_creation_and_retirement_windows` (linked): Compare exact deleted active child code/result plus85+ errorcode options, and distinct recreated child liveness/variable absence.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "runtime/rust/tests/data/native_child_command_lifetime/replay.py",
  "--verify-only"
]
```

Offline exact source/input/stream/receipt verification only. For new native launches use the retained original compile commands with fresh output directory and the exact source/header/static-library association. C8.4 return-options getter unavailable; Jim C public child API not tested; no BIGIP execution. Original v1 errors remain separate from v2 intermediate observations.
