# naming.event.original-callback-source-entry

Kind: `source-anchor`

## Problem statement

C84 AfterProc uses a counted public source entry distinct from later retained-object evaluation.

## Question

Which exact source/object evaluation entry and global flag do the pinned AfterProc/Jim background functions select?

## Conclusion

Pinned C84 AfterProc obtains the counted script bytes and calls Tcl_EvalEx with TCL_EVAL_GLOBAL. C85 through C91 call Tcl_EvalObjEx with TCL_EVAL_GLOBAL on the retained script object. Jim_EvalObjBackground selects the top global frame for Jim_EvalObj and restores the caller frame before reporting an error. These source windows select a recipe; they do not grant native preparation, current handler or normal completion.

## Scope

Exact pinned full-file and LF function-window inspection only, with source/header association from the capture receipts. No fresh source inspection replay launch or runtime object/cache equivalence claim. C84 Tcl_GetReturnOptions is absent; Jim C return-options API is not queried; corresponding rows remain explicitly not-tested. Raw receipts and prior attempts remain immutable.

## Provider answers

### tcl8.4

Status: `inspected`. Version: 8.4.20. Build: Pinned complete source translation unit inspection; matching capture source associations retained, no inspection launch claim.. Channel: Exact retained LF function windows in the full pinned translation units.. Dialect: tcl8.4.

Pinned C84 AfterProc obtains the counted script bytes and calls Tcl_EvalEx with TCL_EVAL_GLOBAL. C85 through C91 call Tcl_EvalObjEx with TCL_EVAL_GLOBAL on the retained script object. Jim_EvalObjBackground selects the top global frame for Jim_EvalObj and restores the caller frame before reporting an error. These source windows select a recipe; they do not grant native preparation, current handler or normal completion.

### tcl8.5

Status: `inspected`. Version: 8.5.19. Build: Pinned complete source translation unit inspection; matching capture source associations retained, no inspection launch claim.. Channel: Exact retained LF function windows in the full pinned translation units.. Dialect: tcl8.5.

Pinned C84 AfterProc obtains the counted script bytes and calls Tcl_EvalEx with TCL_EVAL_GLOBAL. C85 through C91 call Tcl_EvalObjEx with TCL_EVAL_GLOBAL on the retained script object. Jim_EvalObjBackground selects the top global frame for Jim_EvalObj and restores the caller frame before reporting an error. These source windows select a recipe; they do not grant native preparation, current handler or normal completion.

### tcl8.6

Status: `inspected`. Version: 8.6.18. Build: Pinned complete source translation unit inspection; matching capture source associations retained, no inspection launch claim.. Channel: Exact retained LF function windows in the full pinned translation units.. Dialect: tcl8.6.

Pinned C84 AfterProc obtains the counted script bytes and calls Tcl_EvalEx with TCL_EVAL_GLOBAL. C85 through C91 call Tcl_EvalObjEx with TCL_EVAL_GLOBAL on the retained script object. Jim_EvalObjBackground selects the top global frame for Jim_EvalObj and restores the caller frame before reporting an error. These source windows select a recipe; they do not grant native preparation, current handler or normal completion.

### tcl9.0

Status: `inspected`. Version: 9.0.4. Build: Pinned complete source translation unit inspection; matching capture source associations retained, no inspection launch claim.. Channel: Exact retained LF function windows in the full pinned translation units.. Dialect: tcl9.0.

Pinned C84 AfterProc obtains the counted script bytes and calls Tcl_EvalEx with TCL_EVAL_GLOBAL. C85 through C91 call Tcl_EvalObjEx with TCL_EVAL_GLOBAL on the retained script object. Jim_EvalObjBackground selects the top global frame for Jim_EvalObj and restores the caller frame before reporting an error. These source windows select a recipe; they do not grant native preparation, current handler or normal completion.

### tcl9.1

Status: `inspected`. Version: 9.1.0. Build: Pinned complete source translation unit inspection; matching capture source associations retained, no inspection launch claim.. Channel: Exact retained LF function windows in the full pinned translation units.. Dialect: tcl9.1.

Pinned C84 AfterProc obtains the counted script bytes and calls Tcl_EvalEx with TCL_EVAL_GLOBAL. C85 through C91 call Tcl_EvalObjEx with TCL_EVAL_GLOBAL on the retained script object. Jim_EvalObjBackground selects the top global frame for Jim_EvalObj and restores the caller frame before reporting an error. These source windows select a recipe; they do not grant native preparation, current handler or normal completion.

### jim

Status: `inspected`. Version: 0.84-9-g5bac7c9. Build: Pinned complete source translation unit inspection; matching capture source associations retained, no inspection launch claim.. Channel: Exact retained LF function windows in the full pinned translation units.. Dialect: jim.

Pinned C84 AfterProc obtains the counted script bytes and calls Tcl_EvalEx with TCL_EVAL_GLOBAL. C85 through C91 call Tcl_EvalObjEx with TCL_EVAL_GLOBAL on the retained script object. Jim_EvalObjBackground selects the top global frame for Jim_EvalObj and restores the caller frame before reporting an error. These source windows select a recipe; they do not grant native preparation, current handler or normal completion.

### bigip

Status: `not-tested`. Version: not tested. Build: No appliance build attached.. Channel: not exercised. Dialect: bigip.

No BIG-IP provider was executed or inspected for this Event question.

## Exact evidence

- `e7ca34f2d6fd9344d4610` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclTimer.c). SHA-256 `3a54a00f6b2037ce2afc039054dad9c2a7b5c80c77c0258c696a3c9f4ca15cfa`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `eb83569215d92fa37da10` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclTimer.c). SHA-256 `9c6e3aa2bad086fe1e94dadca7e99c22b5997b55b07065e6d9256a7a692255ff`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e5cad7b4101332ff96f7c` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclTimer.c). SHA-256 `648da0aebdcd2adde777ab81d12e4722c7ca8396ff5034e71c4e3f19e25ca7d5`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e4d5b46a62af04d177edb` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclTimer.c). SHA-256 `9be43ec6af72e0ba19422b277c6badb07fec3e604fbaeef8ee6147d3bc21284e`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `eaf331dbfda81fff6c4db` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclTimer.c). SHA-256 `580f163fb0bbd01a5a5ffbd9992cf7f4d2dec932355edb7fe6383b0352ab581b`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e4521e47b782c5165c471` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/jim/jim-eventloop.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/jim/jim-eventloop.c). SHA-256 `b9536f7021919026f8ee2cfcdb35a59305db07a23e60daa50d5da966d52a1a12`. Complete retained pinned translation unit; the exact LF function window is separately recorded.

## Source inspection

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclTimer.c`, function `AfterProc`, lines 1005–1052. Full-source SHA-256 `3a54a00f6b2037ce2afc039054dad9c2a7b5c80c77c0258c696a3c9f4ca15cfa`; snippet SHA-256 `b85d2e4ddfa43e470924a2b351b99d9c9b53ef9bdbd3472b0170fafe736cfe66`; retained evidence `e7ca34f2d6fd9344d4610`.

```text
AfterProc(clientData)
    ClientData clientData;	/* Describes command to execute. */
{
    AfterInfo *afterPtr = (AfterInfo *) clientData;
    AfterAssocData *assocPtr = afterPtr->assocPtr;
    AfterInfo *prevPtr;
    int result;
    Tcl_Interp *interp;
    char *script;
    int numBytes;

    /*
     * First remove the callback from our list of callbacks;  otherwise
     * someone could delete the callback while it's being executed, which
     * could cause a core dump.
     */

    if (assocPtr->firstAfterPtr == afterPtr) {
	assocPtr->firstAfterPtr = afterPtr->nextPtr;
    } else {
	for (prevPtr = assocPtr->firstAfterPtr; prevPtr->nextPtr != afterPtr;
		prevPtr = prevPtr->nextPtr) {
	    /* Empty loop body. */
	}
	prevPtr->nextPtr = afterPtr->nextPtr;
    }

    /*
     * Execute the callback.
     */

    interp = assocPtr->interp;
    Tcl_Preserve((ClientData) interp);
    script = Tcl_GetStringFromObj(afterPtr->commandPtr, &numBytes);
    result = Tcl_EvalEx(interp, script, numBytes, TCL_EVAL_GLOBAL);
    if (result != TCL_OK) {
	Tcl_AddErrorInfo(interp, "\n    (\"after\" script)");
	Tcl_BackgroundError(interp);
    }
    Tcl_Release((ClientData) interp);
    
    /*
     * Free the memory for the callback.
     */

    Tcl_DecrRefCount(afterPtr->commandPtr);
    ckfree((char *) afterPtr);
}
```

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclTimer.c`, function `AfterProc`, lines 1114–1158. Full-source SHA-256 `9c6e3aa2bad086fe1e94dadca7e99c22b5997b55b07065e6d9256a7a692255ff`; snippet SHA-256 `b97779c183f626115a786a6ca0ddc79e6f97b68765168df34dcab2f2e1243e2b`; retained evidence `eb83569215d92fa37da10`.

```text
AfterProc(
    ClientData clientData)	/* Describes command to execute. */
{
    AfterInfo *afterPtr = (AfterInfo *) clientData;
    AfterAssocData *assocPtr = afterPtr->assocPtr;
    AfterInfo *prevPtr;
    int result;
    Tcl_Interp *interp;

    /*
     * First remove the callback from our list of callbacks; otherwise someone
     * could delete the callback while it's being executed, which could cause
     * a core dump.
     */

    if (assocPtr->firstAfterPtr == afterPtr) {
	assocPtr->firstAfterPtr = afterPtr->nextPtr;
    } else {
	for (prevPtr = assocPtr->firstAfterPtr; prevPtr->nextPtr != afterPtr;
		prevPtr = prevPtr->nextPtr) {
	    /* Empty loop body. */
	}
	prevPtr->nextPtr = afterPtr->nextPtr;
    }

    /*
     * Execute the callback.
     */

    interp = assocPtr->interp;
    Tcl_Preserve((ClientData) interp);
    result = Tcl_EvalObjEx(interp, afterPtr->commandPtr, TCL_EVAL_GLOBAL);
    if (result != TCL_OK) {
	Tcl_AddErrorInfo(interp, "\n    (\"after\" script)");
	TclBackgroundException(interp, result);
    }
    Tcl_Release((ClientData) interp);

    /*
     * Free the memory for the callback.
     */

    Tcl_DecrRefCount(afterPtr->commandPtr);
    ckfree((char *) afterPtr);
}
```

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclTimer.c`, function `AfterProc`, lines 1168–1212. Full-source SHA-256 `648da0aebdcd2adde777ab81d12e4722c7ca8396ff5034e71c4e3f19e25ca7d5`; snippet SHA-256 `e42c4864ba8f37bb0160602f5bbe7f3bf18ce07e1d90076d6f3a6b8ffbac2c7b`; retained evidence `e5cad7b4101332ff96f7c`.

```text
AfterProc(
    ClientData clientData)	/* Describes command to execute. */
{
    AfterInfo *afterPtr = clientData;
    AfterAssocData *assocPtr = afterPtr->assocPtr;
    AfterInfo *prevPtr;
    int result;
    Tcl_Interp *interp;

    /*
     * First remove the callback from our list of callbacks; otherwise someone
     * could delete the callback while it's being executed, which could cause
     * a core dump.
     */

    if (assocPtr->firstAfterPtr == afterPtr) {
	assocPtr->firstAfterPtr = afterPtr->nextPtr;
    } else {
	for (prevPtr = assocPtr->firstAfterPtr; prevPtr->nextPtr != afterPtr;
		prevPtr = prevPtr->nextPtr) {
	    /* Empty loop body. */
	}
	prevPtr->nextPtr = afterPtr->nextPtr;
    }

    /*
     * Execute the callback.
     */

    interp = assocPtr->interp;
    Tcl_Preserve(interp);
    result = Tcl_EvalObjEx(interp, afterPtr->commandPtr, TCL_EVAL_GLOBAL);
    if (result != TCL_OK) {
	Tcl_AddErrorInfo(interp, "\n    (\"after\" script)");
	Tcl_BackgroundException(interp, result);
    }
    Tcl_Release(interp);

    /*
     * Free the memory for the callback.
     */

    Tcl_DecrRefCount(afterPtr->commandPtr);
    ckfree(afterPtr);
}
```

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclTimer.c`, function `AfterProc`, lines 1147–1191. Full-source SHA-256 `9be43ec6af72e0ba19422b277c6badb07fec3e604fbaeef8ee6147d3bc21284e`; snippet SHA-256 `6728d838a062e26667d01ff42be2bf848b2ec127e60c0b8c787ba5ed3255c3fb`; retained evidence `e4d5b46a62af04d177edb`.

```text
AfterProc(
    void *clientData)		/* Describes command to execute. */
{
    AfterInfo *afterPtr = (AfterInfo *)clientData;
    AfterAssocData *assocPtr = afterPtr->assocPtr;
    AfterInfo *prevPtr;
    int result;
    Tcl_Interp *interp;

    /*
     * First remove the callback from our list of callbacks; otherwise someone
     * could delete the callback while it's being executed, which could cause
     * a core dump.
     */

    if (assocPtr->firstAfterPtr == afterPtr) {
	assocPtr->firstAfterPtr = afterPtr->nextPtr;
    } else {
	for (prevPtr = assocPtr->firstAfterPtr; prevPtr->nextPtr != afterPtr;
		prevPtr = prevPtr->nextPtr) {
	    /* Empty loop body. */
	}
	prevPtr->nextPtr = afterPtr->nextPtr;
    }

    /*
     * Execute the callback.
     */

    interp = assocPtr->interp;
    Tcl_Preserve(interp);
    result = Tcl_EvalObjEx(interp, afterPtr->commandPtr, TCL_EVAL_GLOBAL);
    if (result != TCL_OK) {
	Tcl_AddErrorInfo(interp, "\n    (\"after\" script)");
	Tcl_BackgroundException(interp, result);
    }
    Tcl_Release(interp);

    /*
     * Free the memory for the callback.
     */

    Tcl_DecrRefCount(afterPtr->commandPtr);
    Tcl_Free(afterPtr);
}
```

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclTimer.c`, function `AfterProc`, lines 1533–1577. Full-source SHA-256 `580f163fb0bbd01a5a5ffbd9992cf7f4d2dec932355edb7fe6383b0352ab581b`; snippet SHA-256 `6728d838a062e26667d01ff42be2bf848b2ec127e60c0b8c787ba5ed3255c3fb`; retained evidence `eaf331dbfda81fff6c4db`.

```text
AfterProc(
    void *clientData)		/* Describes command to execute. */
{
    AfterInfo *afterPtr = (AfterInfo *)clientData;
    AfterAssocData *assocPtr = afterPtr->assocPtr;
    AfterInfo *prevPtr;
    int result;
    Tcl_Interp *interp;

    /*
     * First remove the callback from our list of callbacks; otherwise someone
     * could delete the callback while it's being executed, which could cause
     * a core dump.
     */

    if (assocPtr->firstAfterPtr == afterPtr) {
	assocPtr->firstAfterPtr = afterPtr->nextPtr;
    } else {
	for (prevPtr = assocPtr->firstAfterPtr; prevPtr->nextPtr != afterPtr;
		prevPtr = prevPtr->nextPtr) {
	    /* Empty loop body. */
	}
	prevPtr->nextPtr = afterPtr->nextPtr;
    }

    /*
     * Execute the callback.
     */

    interp = assocPtr->interp;
    Tcl_Preserve(interp);
    result = Tcl_EvalObjEx(interp, afterPtr->commandPtr, TCL_EVAL_GLOBAL);
    if (result != TCL_OK) {
	Tcl_AddErrorInfo(interp, "\n    (\"after\" script)");
	Tcl_BackgroundException(interp, result);
    }
    Tcl_Release(interp);

    /*
     * Free the memory for the callback.
     */

    Tcl_DecrRefCount(afterPtr->commandPtr);
    Tcl_Free(afterPtr);
}
```

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `Jim_EvalObjBackground`, lines 112–148. Full-source SHA-256 `b9536f7021919026f8ee2cfcdb35a59305db07a23e60daa50d5da966d52a1a12`; snippet SHA-256 `7c1e41da33a7663e8745126d78eb7578f28301335188e4f1a7fa6f48e157c0b2`; retained evidence `e4521e47b782c5165c471`.

```text
int Jim_EvalObjBackground(Jim_Interp *interp, Jim_Obj *scriptObjPtr)
{
    Jim_EventLoop *eventLoop = Jim_GetAssocData(interp, "eventloop");
    Jim_CallFrame *savedFramePtr;
    int retval;

    savedFramePtr = interp->framePtr;
    interp->framePtr = interp->topFramePtr;
    retval = Jim_EvalObj(interp, scriptObjPtr);
    interp->framePtr = savedFramePtr;
    /* Try to report the error (if any) via the bgerror proc */
    if (retval != JIM_OK && retval != JIM_RETURN && !eventLoop->suppress_bgerror) {
        Jim_Obj *objv[2];
        int rc = JIM_ERR;

        objv[0] = Jim_NewStringObj(interp, "bgerror", -1);
        objv[1] = Jim_GetResult(interp);
        Jim_IncrRefCount(objv[0]);
        Jim_IncrRefCount(objv[1]);
        if (Jim_GetCommand(interp, objv[0], JIM_NONE) == NULL || (rc = Jim_EvalObjVector(interp, 2, objv)) != JIM_OK) {
            if (rc == JIM_BREAK) {
                /* No more bgerror calls */
                eventLoop->suppress_bgerror++;
            }
            else {
                /* Report the error to stderr. */
                Jim_MakeErrorMessage(interp);
                fprintf(stderr, "%s\n", Jim_String(Jim_GetResult(interp)));
                /* And reset the result */
                Jim_SetResultString(interp, "", -1);
            }
        }
        Jim_DecrRefCount(interp, objv[0]);
        Jim_DecrRefCount(interp, objv[1]);
    }
    return retval;
}
```


## Consumer bindings

- [rust/tcl-registry/src/native_eval_object.rs](../../../../rust/tcl-registry/src/native_eval_object.rs), `NativeEvalObjectProtocol::compiles_source`: Selected callback evaluation entry, independently of native artifact admission.
- [rust/tcl-registry/src/native_eval_object.rs](../../../../rust/tcl-registry/src/native_eval_object.rs), `NativeEvalObjectProtocol::permits_direct_source_operands`: Actual matching C84/trace direct-source entry only; foreign axes refuse.
- [rust/tcl-registry/src/native_eval_object.rs](../../../../rust/tcl-registry/src/native_eval_object.rs), `native_eval_object::after_source_tests::original_after_callback_keeps_counted_c84_entry_separate_from_object_dispatch` (linked): Version-selected C84 counted source versus modern object entry, foreign string recipe negatives and Jim direct-source refusal; pure recipe admission does not mint compiler preparation. No execution claim.

A named test is a coverage binding, not a claim that it executed.

## Replay

This record replays no interpreter. Original compile/run commands, input lengths, deadline status and source/header/library/executable hashes are retained in immutable receipts. Captured output directories and original absolute provisioning paths must not be overwritten. Source inspection can be reproduced from the attached full-file and exact LF/snippet hashes. No Rust execution claim.
