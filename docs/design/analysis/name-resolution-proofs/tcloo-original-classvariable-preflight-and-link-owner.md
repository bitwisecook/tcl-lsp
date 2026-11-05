# naming.tcloo.original-classvariable-preflight-and-link-owner

Kind: `implementation-contract`

## Problem statement

A selected ClassVariable name getter can change operation currency, retire its actual method frame or replace the declaring Class facet while its Object survives. Linking each name before validating the remaining original names can publish effects that the selected worker’s all-name preflight should prevent.

## Question

Does the selected C9 ClassVariable continuation retain the actual method activation, original declaring Class facet and namespace owner across release-specific head/name getters, validate every original local name before any target lookup or link, and stop later effects after first typed refusal or reached owner withdrawal?

## Conclusion

The current source/API continuation retains the original activation/Weak frame, selected declaring Class facet and actual namespace token through preflight and each original target/link result. Exact C9.0/C9.1 worker sources place all local-name validation before namespace lookup and local linking, with distinct head/context getter stages. Five linked finite software controls have no attached matching owned assertion outcome. All seven external execution statuses are not tested; source inspection and software callbacks establish no Native frame/Class/Var pointer lifetime, link refcount or callback parity.

## Scope

Selected original C9 ClassVariable operand count, current method context and declaring Class namespace; all-name local CString validation; actual original namespace target preparation and local-link continuation. Three exact physical-LF C9.0/C9.1 source joins are static facts, separate from software Host/context-ABA/Class-rebirth and metadata-edit controls. C8.6/authored compatibility, namespace upvar, arbitrary class/method reports, Foundation role headers, entered-chain payload/report ownership, Class-facet allocation lifetime and Native frame or execution admission retain their independent purposes. Initial missing context is distinct from withdrawal after original selection.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, jim

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: tcl8.4 not tested for this source/API question; tcl8.5 not tested for this source/API question; tcl8.6 not tested for this source/API question; jim not tested for this source/API question.

No original process tests this selected C9 ClassVariable source/API purpose. The current pure schedule leaves this profile unselected; an older/authored compatibility route and Jim behavior are independent and receive no unavailable-worker or callback-parity observation from that software selection.

### tcl9.0

Status: `not-tested`. Channel: Static selected original worker and method-frame API source; authored software callbacks remain independent. Version: tcl9.0 9.0.4.

Exact C9.0 source checks operand count and current method context before any ordinary name getters, reaching the head only to report missing context. Exact C9.1 source obtains the command head before its current-method-frame helper. The inspected selected worker gets and validates every original local name before namespace-only target lookup and local-link publication. Source order establishes no safe frame/Class/Var pointer lifetime, original callback continuation, link refcount or externally executed Host/ABA parity.

### tcl9.1

Status: `not-tested`. Channel: Static selected original worker and method-frame API source; authored software callbacks remain independent. Version: tcl9.1 9.1.0.

Exact C9.1 source obtains the command head before selecting current method context through TclOOGetContextFromCurrentStackFrame. The inspected selected worker gets and validates every original local name before namespace-only target lookup and local-link publication. Source order establishes no safe frame/Class/Var pointer lifetime, original callback continuation, link refcount or externally executed Host/ABA parity.

### bigip

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: bigip not tested for this source/API question.

No original bigip process tests this selected ClassVariable preflight and original link owner contract. Retained source inspection and unrun authored controls establish neither Native callback outcomes nor physical context, frame, record identity or execution authority.

## Source inspection

tcl9.0 9.0.4, revision `Exact retained complete source and physical-LF excerpt joined independently; source inspection only`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOBasic.c`, function `TclOOClassVariableObjCmd`, lines 1846–1937. Retained reference `classvariable-source-window-0`.

```text
TclOOClassVariableObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Interp *iPtr = (Interp *) interp;
    CallFrame *framePtr = iPtr->varFramePtr;
    CallContext *contextPtr;
    Class *clsPtr;
    Tcl_Namespace *clsNsPtr, *ourNsPtr;
    Var *arrayPtr, *otherPtr;
    int i;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ...");
	return TCL_ERROR;
    }

    /*
     * Start with sanity checks on the calling context to make sure that we
     * are invoked from a suitable method context. If so, we can safely
     * retrieve the handle to the object call context.
     */

    if (framePtr == NULL || !(framePtr->isProcCallFrame & FRAME_IS_METHOD)) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"%s may only be called from inside a method",
		TclGetString(objv[0])));
	OO_ERROR(interp, CONTEXT_REQUIRED);
	return TCL_ERROR;
    }

    /* Get a reference to the class's namespace */
    contextPtr = (CallContext *) framePtr->clientData;
    clsPtr = CurrentlyInvoked(contextPtr).mPtr->declaringClassPtr;
    if (clsPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"method not defined by a class", TCL_AUTO_LENGTH));
	OO_ERROR(interp, UNMATCHED_CONTEXT);
	return TCL_ERROR;
    }
    clsNsPtr = clsPtr->thisPtr->namespacePtr;

    /* Check the list of variable names */
    for (i = 1; i < objc; i++) {
	const char *varName = TclGetString(objv[i]);
	if (Tcl_StringMatch(varName, "*(*)")) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "bad variable name \"%s\": can't create a %s",
		    varName, "scalar variable that looks like an array element"));
	    Tcl_SetErrorCode(interp, "TCL", "UPVAR", "LOCAL_ELEMENT", (char *)NULL);
	    return TCL_ERROR;
	}
	if (Tcl_StringMatch(varName, "*::*")) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "bad variable name \"%s\": can't create a %s",
		    varName, "local variable with a namespace separator in it"));
	    Tcl_SetErrorCode(interp, "TCL", "UPVAR", "LOCAL_ELEMENT", (char *)NULL);
	    return TCL_ERROR;
	}
    }

    /* Lastly, link the caller's local variables to the class's variables */
    ourNsPtr = (Tcl_Namespace *) iPtr->varFramePtr->nsPtr;
    for (i = 1; i < objc; i++) {
	/* Locate the other variable. */
	iPtr->varFramePtr->nsPtr = (Namespace *) clsNsPtr;
	otherPtr = TclObjLookupVarEx(interp, objv[i], NULL,
		(TCL_NAMESPACE_ONLY|TCL_LEAVE_ERR_MSG|TCL_AVOID_RESOLVERS),
		"access", /*createPart1*/ 1, /*createPart2*/ 0, &arrayPtr);
	iPtr->varFramePtr->nsPtr = (Namespace *) ourNsPtr;
	if (otherPtr == NULL) {
	    return TCL_ERROR;
	}

	/* Create the new variable and link it to otherPtr. */
	if (TclPtrObjMakeUpvarIdx(interp, otherPtr, objv[i], 0,
		TCL_INDEX_NONE) != TCL_OK) {
	    return TCL_ERROR;
	}
    }

    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODelegateNameObjCmd --
 *
 *	Implementation of the [oo::DelegateName] command, which is a utility

```

tcl9.1 9.1.0, revision `Exact retained complete source and physical-LF excerpt joined independently; source inspection only`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOBasic.c`, function `TclOOClassVariableObjCmd`, lines 1863–1936. Retained reference `classvariable-source-window-1`.

```text
TclOOClassVariableObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ...");
	return TCL_ERROR;
    }

    /*
     * Start with sanity checks on the calling context to make sure that we
     * are invoked from a suitable method context. If so, we can safely
     * retrieve the handle to the object call context.
     */

    // Get a reference to the class's namespace
    CallContext *contextPtr = TclOOGetContextFromCurrentStackFrame(interp,
	    TclGetString(objv[0]));
    if (!contextPtr) {
	return TCL_ERROR;
    }
    Class *clsPtr = CurrentlyInvoked(contextPtr).mPtr->declaringClassPtr;
    if (clsPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"method not defined by a class", TCL_AUTO_LENGTH));
	OO_ERROR(interp, UNMATCHED_CONTEXT);
	return TCL_ERROR;
    }
    Tcl_Namespace *clsNsPtr = clsPtr->thisPtr->namespacePtr;

    // Check the list of variable names
    for (Tcl_Size i = 1; i < objc; i++) {
	const char *varName = TclGetString(objv[i]);
	if (Tcl_StringMatch(varName, "*(*)")) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "bad variable name \"%s\": can't create a %s",
		    varName, "scalar variable that looks like an array element"));
	    Tcl_SetErrorCode(interp, "TCL", "UPVAR", "LOCAL_ELEMENT", (char *)NULL);
	    return TCL_ERROR;
	}
	if (Tcl_StringMatch(varName, "*::*")) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "bad variable name \"%s\": can't create a %s",
		    varName, "local variable with a namespace separator in it"));
	    Tcl_SetErrorCode(interp, "TCL", "UPVAR", "LOCAL_ELEMENT", (char *)NULL);
	    return TCL_ERROR;
	}
    }

    // Lastly, link the caller's local variables to the class's variables
    Interp *iPtr = (Interp *) interp;
    Tcl_Namespace *ourNsPtr = (Tcl_Namespace *) iPtr->varFramePtr->nsPtr;
    for (Tcl_Size i = 1; i < objc; i++) {
	// Locate the other variable.
	iPtr->varFramePtr->nsPtr = (Namespace *) clsNsPtr;
	Var *arrayPtr, *otherPtr = TclObjLookupVarEx(interp, objv[i], NULL,
		(TCL_NAMESPACE_ONLY|TCL_LEAVE_ERR_MSG|TCL_AVOID_RESOLVERS),
		"access", /*createPart1*/ 1, /*createPart2*/ 0, &arrayPtr);
	iPtr->varFramePtr->nsPtr = (Namespace *) ourNsPtr;
	if (otherPtr == NULL) {
	    return TCL_ERROR;
	}

	// Create the new variable and link it to otherPtr.
	if (TclPtrObjMakeUpvarIdx(interp, otherPtr, objv[i], 0,
		TCL_INDEX_NONE) != TCL_OK) {
	    return TCL_ERROR;
	}
    }

    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `Exact retained complete source and physical-LF excerpt joined independently; source inspection only`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInt.h`, function `TclOOGetContextFromCurrentStackFrame`, lines 673–708. Retained reference `classvariable-source-window-2`.

```text
 * ----------------------------------------------------------------------
 *
 * TclOOGetContextFromCurrentStackFrame --
 *
 *	Get the object context from the stack frame that invoked the command
 *	that called this helper function. If a command name is supplied, will
 *	write an error message into the interpreter if the frame doesn't
 *	contain an object context.
 *
 * ----------------------------------------------------------------------
 */
static inline CallContext *
TclOOGetContextFromCurrentStackFrame(
    Tcl_Interp *interp,		/* How to get the stack frame and report
				 * problems. */
    const char *commandName)	/* If not NULL, used in problem reports. */
{
    CallFrame *framePtr = ((Interp *) interp)->varFramePtr;

    /*
     * Sanity checks on the calling context to make sure that we are invoked
     * from a suitable method context. If so, we can safely get the handle to
     * the object call context.
     */

    if (framePtr == NULL || !(framePtr->isProcCallFrame & FRAME_IS_METHOD)) {
	if (commandName != NULL) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "%s may only be called from inside a method",
		    commandName));
	    Tcl_SetErrorCode(interp, "TCL", "OO", "CONTEXT_REQUIRED",
		    (char *)NULL);
	}
	return NULL;
    }
    return (CallContext *) framePtr->clientData;

```

## Consumer bindings

- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `classvariable_cmd`: Select the explicit C9 ClassVariable schedule through authentic original arguments; the existing independent compatibility branch remains separate.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `classvariable_original`: Retain actual entered activation/Weak frame, original declaring Class facet and namespace token across every selected getter, preflight validation, target preparation and local-link result. All original name validation precedes any target lookup or local-link effect.
- [runtime/rust/src/cmd_oo/native_context.rs](../../../../runtime/rust/src/cmd_oo/native_context.rs), `OriginalOoTarget`: Check the actual selected declaring Class facet and instance namespace token, independently of Object display identity or unrelated method metadata changes.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `prepare_original_c_namespace_link_target`: Prepare the independently selected original namespace receiver and target before local-link publication; a caller must retain its own actual frame and namespace purpose.
- [rust/tcl-registry/src/native_tcloo_method_definition.rs](../../../../rust/tcl-registry/src/native_tcloo_method_definition.rs), `NativeTclooClassVariableProtocol`: Pure release-specific head stage and local CString name validation. Availability and grammar selection do not issue physical method context or variable/link ownership.
- [runtime/rust/src/cmd_oo/original_classvariable_tests.rs](../../../../runtime/rust/src/cmd_oo/original_classvariable_tests.rs), `cmd_oo::original_classvariable_tests::selected_classvariable_validates_all_names_before_any_lookup_or_link` (linked): An actual selected software C9 method frame reaches original name getters in release-specific head order, rejects the first invalid local name before any class-namespace cell/link, leaves the later lazy name unread and preserves exact caller references and selected Guest code. This does not execute the inspected original C worker. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.
- [runtime/rust/src/cmd_oo/original_classvariable_tests.rs](../../../../runtime/rust/src/cmd_oo/original_classvariable_tests.rs), `cmd_oo::original_classvariable_tests::selected_classvariable_name_callbacks_keep_original_frame_facet_and_first_cause` (linked): Original first-name software updater Host, context ABA or declaring-Class demotion/rebirth preserves reached result storage and stops later getters or links after retained frame/facet/namespace withdrawal. Displayed Object survival does not replace the original Class facet; original C pointer/callback safety is unmeasured. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.
- [runtime/rust/src/cmd_oo/original_classvariable_tests.rs](../../../../runtime/rust/src/cmd_oo/original_classvariable_tests.rs), `cmd_oo::original_classvariable_tests::selected_classvariable_links_original_names_after_legal_method_metadata_changes` (linked): Legal method metadata edits preserve the original frame, declaring Class facet and namespace token. All original names are validated before selected namespace targets and local links; actual original hash-key headers and caller-plus-namespace reference roles survive retired local frame cleanup. These are software ownership observations only. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.
- [runtime/rust/src/cmd_oo/original_classvariable_tests.rs](../../../../runtime/rust/src/cmd_oo/original_classvariable_tests.rs), `cmd_oo::original_classvariable_tests::selected_classvariable_missing_context_reads_only_its_original_head` (linked): Initial missing actual method context reaches only the original command head and selected context-required Guest presentation, leaving all name headers unread with caller holds. An initially absent frame is distinct from a reached frame withdrawal. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.
- [rust/tcl-registry/src/native_tcloo_method_definition.rs](../../../../rust/tcl-registry/src/native_tcloo_method_definition.rs), `native_tcloo_method_definition::classvariable_tests::selected_classvariable_schedule_keeps_availability_and_cstring_shapes_distinct` (linked): The pure Registry schedule selects the inspected C9 workers, keeps C9.0 missing-context and C9.1 before-context head stages distinct, validates local array/namespace shapes using selected CString extent and keeps Jim/older profiles unselected. Protocol selection issues no frame, Class facet, variable target or link authority. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.

## Evidence

- [runtime/rust/tests/data/original_classvariable_preflight_link_owner_source/9.0.4](../../../../runtime/rust/tests/data/original_classvariable_preflight_link_owner_source/9.0.4)
- [runtime/rust/tests/data/original_classvariable_preflight_link_owner_source/9.1.0](../../../../runtime/rust/tests/data/original_classvariable_preflight_link_owner_source/9.1.0)
- [runtime/rust/tests/data](../../../../runtime/rust/tests/data)
- [runtime/rust/src](../../../../runtime/rust/src)
- [runtime/rust/src/cmd_oo](../../../../runtime/rust/src/cmd_oo)
- [runtime/rust/src/interp](../../../../runtime/rust/src/interp)
- [rust/tcl-registry/src](../../../../rust/tcl-registry/src)
