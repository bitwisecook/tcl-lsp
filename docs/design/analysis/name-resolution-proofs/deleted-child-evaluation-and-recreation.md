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

Status: `observed`. Version: 8.4.20. Build: Actual executable linked to library and header; compiled/exited0.. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Dialect: Tcl.

All five C providers return error 1 and attempt to call eval in deleted interpreter when the retired child continues. The parent owns a distinct recreated child whose before variable is absent. The preserved C public API control independently records old deleted=1, replacement present=1, different allocation=1 and after-return result 1 0. C8.4 exposes no public GetReturnOptions here;85+ retain the exact error options. An activation lease grants storage lifetime only.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual executable linked to library and header; compiled/exited0.. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Dialect: Tcl.

All five C providers return error 1 and attempt to call eval in deleted interpreter when the retired child continues. The parent owns a distinct recreated child whose before variable is absent. The preserved C public API control independently records old deleted=1, replacement present=1, different allocation=1 and after-return result 1 0. C8.4 exposes no public GetReturnOptions here;85+ retain the exact error options. An activation lease grants storage lifetime only.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable linked to library and header; compiled/exited0.. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Dialect: Tcl.

All five C providers return error 1 and attempt to call eval in deleted interpreter when the retired child continues. The parent owns a distinct recreated child whose before variable is absent. The preserved C public API control independently records old deleted=1, replacement present=1, different allocation=1 and after-return result 1 0. C8.4 exposes no public GetReturnOptions here;85+ retain the exact error options. An activation lease grants storage lifetime only.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable linked to library and header; compiled/exited0.. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Dialect: Tcl.

All five C providers return error 1 and attempt to call eval in deleted interpreter when the retired child continues. The parent owns a distinct recreated child whose before variable is absent. The preserved C public API control independently records old deleted=1, replacement present=1, different allocation=1 and after-return result 1 0. C8.4 exposes no public GetReturnOptions here;85+ retain the exact error options. An activation lease grants storage lifetime only.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable linked to library and header; compiled/exited0.. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Dialect: Tcl.

All five C providers return error 1 and attempt to call eval in deleted interpreter when the retired child continues. The parent owns a distinct recreated child whose before variable is absent. The preserved C public API control independently records old deleted=1, replacement present=1, different allocation=1 and after-return result 1 0. C8.4 exposes no public GetReturnOptions here;85+ retain the exact error options. An activation lease grants storage lifetime only.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual executable linked to library and header; compiled/exited0.. Channel: Original counted Jim string object passed to Jim_EvalObj; C API explicitly not tested. Dialect: Jim Tcl.

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
