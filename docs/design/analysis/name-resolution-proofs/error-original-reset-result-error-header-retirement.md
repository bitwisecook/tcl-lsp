# naming.error.original-reset-result-error-header-retirement

Kind: `native-observation`

## Problem statement

A result reset retires selected error-header references while callbacks may still see original private or global fields. Clearing fields before their decrement, eagerly formatting a lazy error header, or treating a freeIntRep hook as header death would change the reached stages and reference roles. Original C8.4 globals and later C private fields require distinct observations.

## Question

For five fresh original-header cases on each selected C release, what result/code/info fields and reference roles are visible during public Tcl_ResetResult, its pure freeIntRep and compatibility write callbacks, and which caller-held headers remain lazy until independently requested counted String getters and caller release?

## Conclusion

All five own original C producers complete16 closed0 processes and25 cases:100 ROW,78 EVENT and25 separate GETTER records. C8.4 retains original global code/info headers and clears its explicit-code flag without reset write traces. C8.5–C9.1 decrement code then info while each corresponding old private field is still selected, then clear it; held caller/global roles survive independently. The explicit COPY fixture reaches code then info global traces. Every result R reaches freeIntRep at refs1 while its same current header becomes empty/typeNone; this is not header death. No code/info String updater runs during reset; each later caller getter yields the same14 NUL/non-UTF8-containing bytes. Deliberately installed private info and COPY inputs are not public lazy-info/flag producer proofs. Jim/BIG-IP and matching Rust assertions are not tested; mutating reentry, Host/ABA, teardown callbacks and general header/VM equivalence remain unmeasured.

## Scope

Five fresh own C compilations select original actual SDK Makefile flags/cwd, strict actual header/link/runtime dependency admission and unchanged own executable identities. Five cases per provider select absent error fields, sole selected code/info holds, separate caller holds, explicit compatibility COPY flag with pure global traces, and one same original C header in both code/info fields with two selected holds plus one caller. Original result is independent and result-only held. Modern errorInfo uses an explicitly installed authentic private field/native +1 rather than a public lazy-info producer; COPY is an explicit input flag. C84 uses actual global storage and no modern private flag. Observer callbacks perform no variable worker, getter, shell creation or interpreter mutation; trace registration is its own setup worker. No original pointer is dereferenced after a freeIntRep callback, whose occurrence alone gives no general object-death claim. Caller String presentation and release follow reset; observers are disabled before interpreter teardown. There is no result/error header alias, mutating reentry, Host/ABA, physical frame/ABI, Jim/BIG-IP, Rust/VM, whole-suite or performance grant.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Public Tcl_ResetResult on five authentic prepared Interp/result/code/info header roles; pure original freeIntRep/compatibility variable trace observation followed by separate caller-held Tcl_GetStringFromObj and caller release. Version: tcl8.4 8.4.20.

Five original cases complete with20 ROW/five separate counted GETTER/10 EVENT. Reset calls R freeIntRep at passedrefs1 while current result remains that same R header and becomes empty/typeNone. The original global code/info headers persist: sole global refs1 in case1, caller-plus-global refs2 in cases2/3, and shared code/info C refs3 in case4. The explicit-code flag goes1 to0; even with registered case3 write traces, reset reaches no global write trace. Later caller getters alone update the original headers and return14 bytes; caller release leaves the independently held globals. C84 has no selected modern private/COPY-flag route. All selected code/info headers stay lazy with updater0 through reset; the five later counted getters each return14 bytes containing NUL/non-UTF8. Modern errorInfo and COPY flag are explicit original-field input fixtures, not observed public producers. Pure observers perform no interpreter mutation. FreeIntRep alone does not prove header death; no teardown callback, Host/ABA, general reentry, safe pointer lifetime or Rust/VM parity follows.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Public Tcl_ResetResult on five authentic prepared Interp/result/code/info header roles; pure original freeIntRep/compatibility variable trace observation followed by separate caller-held Tcl_GetStringFromObj and caller release. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Five original cases complete with20 ROW/five separate counted GETTER/17 EVENT. Reset calls R freeIntRep at passedrefs1 while current result remains that same R header and becomes empty/typeNone. With sole private holds, free-C sees codeC/infoI at passedrefs0, then free-I sees codeNONE/infoI before its own field is cleared. Caller-held case2 removes only each private hold, refs2 to1; the later caller releases trigger their frees. Explicit COPY-flag case3 reaches code then info global write traces while their respective private headers are still selected; caller-plus-global refs2 remain after private clear and become1 after caller release. Shared code/info C in case4 loses two private holds, refs3 to1, then a separate getter and caller release occur. All selected code/info headers stay lazy with updater0 through reset; the five later counted getters each return14 bytes containing NUL/non-UTF8. Modern errorInfo and COPY flag are explicit original-field input fixtures, not observed public producers. Pure observers perform no interpreter mutation. FreeIntRep alone does not prove header death; no teardown callback, Host/ABA, general reentry, safe pointer lifetime or Rust/VM parity follows.

### jim

Status: `not-tested`. Channel: No original jim input reaches this selected Tcl_ResetResult C API question. Version: jim No jim process selected.

The selected worker is the original Tcl_ResetResult C API, and no Jim Tcl API process is selected or executed. This is not a measured Jim unavailable-worker response or an observation of Jim result APIs.

### bigip

Status: `not-tested`. Channel: No original bigip input reaches this selected Tcl_ResetResult C API question. Version: bigip No bigip process selected.

No BIG-IP process executes this reset-header question. C source inspection and finite C observations do not establish BIG-IP or iRules reset semantics.

## Source inspection

tcl8.4 8.4.20, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclResult.c`, function `Tcl_ResetResult`, lines 780–797. Retained reference `reset-native-86`.

```text
Tcl_ResetResult(interp)
    register Tcl_Interp *interp; /* Interpreter for which to clear result. */
{
    register Interp *iPtr = (Interp *) interp;

    ResetObjResult(iPtr);
    if (iPtr->freeProc != NULL) {
	if (iPtr->freeProc == TCL_DYNAMIC) {
	    ckfree(iPtr->result);
	} else {
	    (*iPtr->freeProc)(iPtr->result);
	}
	iPtr->freeProc = 0;
    }
    iPtr->result = iPtr->resultSpace;
    iPtr->resultSpace[0] = 0;
    iPtr->flags &= ~(ERR_ALREADY_LOGGED | ERR_IN_PROGRESS | ERROR_CODE_SET);
}

```

tcl8.4 8.4.20, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclResult.c`, function `ResetObjResult`, lines 818–842. Retained reference `reset-native-85`.

```text
ResetObjResult(iPtr)
    register Interp *iPtr;	/* Points to the interpreter whose result
				 * object should be reset. */
{
    register Tcl_Obj *objResultPtr = iPtr->objResultPtr;

    if (Tcl_IsShared(objResultPtr)) {
	TclDecrRefCount(objResultPtr);
	TclNewObj(objResultPtr);
	Tcl_IncrRefCount(objResultPtr);
	iPtr->objResultPtr = objResultPtr;
    } else {
	if ((objResultPtr->bytes != NULL)
	        && (objResultPtr->bytes != tclEmptyStringRep)) {
	    ckfree((char *) objResultPtr->bytes);
	}
	objResultPtr->bytes  = tclEmptyStringRep;
	objResultPtr->length = 0;
	if ((objResultPtr->typePtr != NULL)
	        && (objResultPtr->typePtr->freeIntRepProc != NULL)) {
	    objResultPtr->typePtr->freeIntRepProc(objResultPtr);
	}
	objResultPtr->typePtr = (Tcl_ObjType *) NULL;
    }
}

```

tcl8.5 8.5.19, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclResult.c`, function `Tcl_ResetResult`, lines 889–930. Retained reference `reset-native-88`.

```text
Tcl_ResetResult(
    register Tcl_Interp *interp)/* Interpreter for which to clear result. */
{
    register Interp *iPtr = (Interp *) interp;

    ResetObjResult(iPtr);
    if (iPtr->freeProc != NULL) {
	if (iPtr->freeProc == TCL_DYNAMIC) {
	    ckfree(iPtr->result);
	} else {
	    (*iPtr->freeProc)(iPtr->result);
	}
	iPtr->freeProc = 0;
    }
    iPtr->result = iPtr->resultSpace;
    iPtr->resultSpace[0] = 0;
    if (iPtr->errorCode) {
	/* Legacy support */
	if (iPtr->flags & ERR_LEGACY_COPY) {
	    Tcl_ObjSetVar2(interp, iPtr->ecVar, NULL,
		    iPtr->errorCode, TCL_GLOBAL_ONLY);
	}
	Tcl_DecrRefCount(iPtr->errorCode);
	iPtr->errorCode = NULL;
    }
    if (iPtr->errorInfo) {
	/* Legacy support */
	if (iPtr->flags & ERR_LEGACY_COPY) {
	    Tcl_ObjSetVar2(interp, iPtr->eiVar, NULL,
		    iPtr->errorInfo, TCL_GLOBAL_ONLY);
	}
	Tcl_DecrRefCount(iPtr->errorInfo);
	iPtr->errorInfo = NULL;
    }
    iPtr->returnLevel = 1;
    iPtr->returnCode = TCL_OK;
    if (iPtr->returnOpts) {
	Tcl_DecrRefCount(iPtr->returnOpts);
	iPtr->returnOpts = NULL;
    }
    iPtr->flags &= ~(ERR_ALREADY_LOGGED | ERR_LEGACY_COPY);
}

```

tcl8.5 8.5.19, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclResult.c`, function `ResetObjResult`, lines 951–973. Retained reference `reset-native-87`.

```text
ResetObjResult(
    register Interp *iPtr)	/* Points to the interpreter whose result
				 * object should be reset. */
{
    register Tcl_Obj *objResultPtr = iPtr->objResultPtr;

    if (Tcl_IsShared(objResultPtr)) {
	TclDecrRefCount(objResultPtr);
	TclNewObj(objResultPtr);
	Tcl_IncrRefCount(objResultPtr);
	iPtr->objResultPtr = objResultPtr;
    } else {
	if (objResultPtr->bytes != tclEmptyStringRep) {
	    if (objResultPtr->bytes) {
		ckfree((char *) objResultPtr->bytes);
	    }
	    objResultPtr->bytes = tclEmptyStringRep;
	    objResultPtr->length = 0;
	}
	TclFreeIntRep(objResultPtr);
	objResultPtr->typePtr = NULL;
    }
}

```

tcl8.6 8.6.18, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclResult.c`, function `Tcl_ResetResult`, lines 919–961. Retained reference `reset-native-90`.

```text
Tcl_ResetResult(
    Tcl_Interp *interp)		/* Interpreter for which to clear result. */
{
    Interp *iPtr = (Interp *) interp;

    ResetObjResult(iPtr);
    if (iPtr->freeProc != NULL) {
	if (iPtr->freeProc == TCL_DYNAMIC) {
	    ckfree(iPtr->result);
	} else {
	    iPtr->freeProc(iPtr->result);
	}
	iPtr->freeProc = 0;
    }
    iPtr->result = iPtr->resultSpace;
    iPtr->resultSpace[0] = 0;
    if (iPtr->errorCode) {
	/* Legacy support */
	if (iPtr->flags & ERR_LEGACY_COPY) {
	    Tcl_ObjSetVar2(interp, iPtr->ecVar, NULL,
		    iPtr->errorCode, TCL_GLOBAL_ONLY);
	}
	Tcl_DecrRefCount(iPtr->errorCode);
	iPtr->errorCode = NULL;
    }
    if (iPtr->errorInfo) {
	/* Legacy support */
	if (iPtr->flags & ERR_LEGACY_COPY) {
	    Tcl_ObjSetVar2(interp, iPtr->eiVar, NULL,
		    iPtr->errorInfo, TCL_GLOBAL_ONLY);
	}
	Tcl_DecrRefCount(iPtr->errorInfo);
	iPtr->errorInfo = NULL;
    }
    iPtr->resetErrorStack = 1;
    iPtr->returnLevel = 1;
    iPtr->returnCode = TCL_OK;
    if (iPtr->returnOpts) {
	Tcl_DecrRefCount(iPtr->returnOpts);
	iPtr->returnOpts = NULL;
    }
    iPtr->flags &= ~(ERR_ALREADY_LOGGED | ERR_LEGACY_COPY);
}

```

tcl8.6 8.6.18, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclResult.c`, function `ResetObjResult`, lines 982–1003. Retained reference `reset-native-89`.

```text
ResetObjResult(
    Interp *iPtr)		/* Points to the interpreter whose result
				 * object should be reset. */
{
    Tcl_Obj *objResultPtr = iPtr->objResultPtr;

    if (Tcl_IsShared(objResultPtr)) {
	TclDecrRefCount(objResultPtr);
	TclNewObj(objResultPtr);
	Tcl_IncrRefCount(objResultPtr);
	iPtr->objResultPtr = objResultPtr;
    } else {
	if (objResultPtr->bytes != tclEmptyStringRep) {
	    if (objResultPtr->bytes) {
		ckfree(objResultPtr->bytes);
	    }
	    objResultPtr->bytes = tclEmptyStringRep;
	    objResultPtr->length = 0;
	}
	TclFreeIntRep(objResultPtr);
    }
}

```

tcl9.0 9.0.4, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclResult.c`, function `Tcl_ResetResult`, lines 389–421. Retained reference `reset-native-92`.

```text
Tcl_ResetResult(
    Tcl_Interp *interp)		/* Interpreter for which to clear result. */
{
    Interp *iPtr = (Interp *) interp;

    ResetObjResult(iPtr);
    if (iPtr->errorCode) {
	/* Legacy support */
	if (iPtr->flags & ERR_LEGACY_COPY) {
	    Tcl_ObjSetVar2(interp, iPtr->ecVar, NULL,
		    iPtr->errorCode, TCL_GLOBAL_ONLY);
	}
	Tcl_DecrRefCount(iPtr->errorCode);
	iPtr->errorCode = NULL;
    }
    if (iPtr->errorInfo) {
	/* Legacy support */
	if (iPtr->flags & ERR_LEGACY_COPY) {
	    Tcl_ObjSetVar2(interp, iPtr->eiVar, NULL,
		    iPtr->errorInfo, TCL_GLOBAL_ONLY);
	}
	Tcl_DecrRefCount(iPtr->errorInfo);
	iPtr->errorInfo = NULL;
    }
    iPtr->resetErrorStack = 1;
    iPtr->returnLevel = 1;
    iPtr->returnCode = TCL_OK;
    if (iPtr->returnOpts) {
	Tcl_DecrRefCount(iPtr->returnOpts);
	iPtr->returnOpts = NULL;
    }
    iPtr->flags &= ~(ERR_ALREADY_LOGGED | ERR_LEGACY_COPY);
}

```

tcl9.0 9.0.4, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclResult.c`, function `ResetObjResult`, lines 442–463. Retained reference `reset-native-91`.

```text
ResetObjResult(
    Interp *iPtr)		/* Points to the interpreter whose result
				 * object should be reset. */
{
    Tcl_Obj *objResultPtr = iPtr->objResultPtr;

    if (Tcl_IsShared(objResultPtr)) {
	TclDecrRefCount(objResultPtr);
	TclNewObj(objResultPtr);
	Tcl_IncrRefCount(objResultPtr);
	iPtr->objResultPtr = objResultPtr;
    } else {
	if (objResultPtr->bytes != &tclEmptyString) {
	    if (objResultPtr->bytes) {
		Tcl_Free(objResultPtr->bytes);
	    }
	    objResultPtr->bytes = &tclEmptyString;
	    objResultPtr->length = 0;
	}
	TclFreeInternalRep(objResultPtr);
    }
}

```

tcl9.1 9.1.0, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclResult.c`, function `Tcl_ResetResult`, lines 390–422. Retained reference `reset-native-94`.

```text
Tcl_ResetResult(
    Tcl_Interp *interp)		/* Interpreter for which to clear result. */
{
    Interp *iPtr = (Interp *) interp;

    ResetObjResult(iPtr);
    if (iPtr->errorCode) {
	/* Legacy support */
	if (iPtr->flags & ERR_LEGACY_COPY) {
	    Tcl_ObjSetVar2(interp, iPtr->ecVar, NULL,
		    iPtr->errorCode, TCL_GLOBAL_ONLY);
	}
	Tcl_DecrRefCount(iPtr->errorCode);
	iPtr->errorCode = NULL;
    }
    if (iPtr->errorInfo) {
	/* Legacy support */
	if (iPtr->flags & ERR_LEGACY_COPY) {
	    Tcl_ObjSetVar2(interp, iPtr->eiVar, NULL,
		    iPtr->errorInfo, TCL_GLOBAL_ONLY);
	}
	Tcl_DecrRefCount(iPtr->errorInfo);
	iPtr->errorInfo = NULL;
    }
    iPtr->resetErrorStack = 1;
    iPtr->returnLevel = 1;
    iPtr->returnCode = TCL_OK;
    if (iPtr->returnOpts) {
	Tcl_DecrRefCount(iPtr->returnOpts);
	iPtr->returnOpts = NULL;
    }
    iPtr->flags &= ~(ERR_ALREADY_LOGGED | ERR_LEGACY_COPY);
}

```

tcl9.1 9.1.0, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclResult.c`, function `ResetObjResult`, lines 443–464. Retained reference `reset-native-93`.

```text
ResetObjResult(
    Interp *iPtr)		/* Points to the interpreter whose result
				 * object should be reset. */
{
    Tcl_Obj *objResultPtr = iPtr->objResultPtr;

    if (Tcl_IsShared(objResultPtr)) {
	TclDecrRefCount(objResultPtr);
	TclNewObj(objResultPtr);
	Tcl_IncrRefCount(objResultPtr);
	iPtr->objResultPtr = objResultPtr;
    } else {
	if (objResultPtr->bytes != &tclEmptyString) {
	    if (objResultPtr->bytes) {
		Tcl_Free(objResultPtr->bytes);
	    }
	    objResultPtr->bytes = &tclEmptyString;
	    objResultPtr->length = 0;
	}
	TclFreeInternalRep(objResultPtr);
    }
}

```

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [runtime/rust/tests/data/native_reset_result_error_header_retirement_original](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original)
- [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618)
- [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows)
- [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/static-review](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/static-review)
- [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/independent-interpretation](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/independent-interpretation)
