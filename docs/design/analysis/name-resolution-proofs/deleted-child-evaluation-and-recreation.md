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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

All five C providers return error 1 and attempt to call eval in deleted interpreter when the retired child continues. The parent owns a distinct recreated child whose before variable is absent. The preserved C public API control independently records old deleted=1, replacement present=1, different allocation=1 and after-return result 1 0. C8.4 exposes no public GetReturnOptions here;85+ retain the exact error options. An activation lease grants storage lifetime only.

### jim

Status: `unsupported`. Channel: Original counted Jim string object passed to Jim_EvalObj; C API explicitly not tested. Version: jim 0.84-9-g5bac7c9.

Actual C-shaped interp source controls each return1/wrong # args: should be "interp". The C public active-allocation API is explicitly not tested; no corresponding child behavior is inferred.

### bigip

Status: `not-tested`.

No appliance observation.

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

## Evidence

- [runtime/rust/tests/data/native_child_command_lifetime/v1](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1)
- [runtime/rust/tests/data/native_child_command_lifetime/v2](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2)
- [runtime/rust/tests/data/native_child_command_lifetime/v1/capture](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture)
- [runtime/rust/tests/data/native_child_command_lifetime/v2/capture](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture)
- [runtime/rust/tests/data](../../../../runtime/rust/tests/data)
