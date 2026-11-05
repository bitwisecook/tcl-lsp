# naming.variable.trace-callback-direct-source

Kind: `source-anchor`

## Problem statement

A copied variable trace script is bytes evaluated by a C source entry, not an original caller script object. Treating it as an ordinary ControlBody can attach native Bytecode or literal-pool authority that the callback never received. The reached callback callsite and independently selected source evaluation purpose must be kept separate from namespace/frame and prefix storage proof.

## Question

Which counted source entry do the five retained C variable trace callsites invoke, and can that purpose reuse an ordinary object-body compilation grant?

## Conclusion

The inspected TraceVarProc callsites in C8.4 through C9.1 append the retained prefix with its counted length and pass the assembled DString plus its full length to Tcl_EvalEx with flags0. The retained evaluator excerpts show direct command parsing and object-vector dispatch: TclEvalObjvInternal in C8.4/C8.5 and Tcl_EvalObjv in C8.6/C9. The shared TraceCallback purpose therefore selects fresh direct operands and rejects ordinary List dispatch, Bytecode cache donation and foreign string protocols. Source inspection and Rust admission assertions remain distinct from actual native prefix observations.

## Scope

Exact pinned generic source files for Tcl8.4.20/8.5.19/8.6.18/9.0.4/9.1.0. Independently retained full-file SHA and original line numbers identify excerpts. Current Jim and BIG-IP are not inspected for this C TraceVarProc entry; no arbitrary object body or callback result cache is granted.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `inspected`. Channel: Copied counted DString source supplied by TraceVarProc to Tcl_EvalEx(flags0). Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Inspected counted callback EvalEx and original source object-vector dispatch callsites; no original ControlBody object is passed.

### jim

Status: `not-tested`.

No observation for this question.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Source inspection

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclCmdMZ.c`, function `TraceVarProc`, lines 4927–4944. Retained reference `e0`.

```text

	    Tcl_SaveResult(interp, &state);
	    if ((flags & TCL_TRACE_DESTROYED)
		    && !(tvarPtr->flags & TCL_TRACE_DESTROYED)) {
		destroy = 1;
		tvarPtr->flags |= TCL_TRACE_DESTROYED;
	    }

	    code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		    Tcl_DStringLength(&cmd), 0);
	    if (code != TCL_OK) {	     /* copy error msg to result */
		register Tcl_Obj *errMsgObj = Tcl_GetObjResult(interp);
		Tcl_IncrRefCount(errMsgObj);
		result = (char *) errMsgObj;
	    }

	    Tcl_RestoreResult(interp, &state);

```

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclBasic.c`, function `EvalEx`, lines 4256–4273. Retained reference `e1`.

```text
	    eeFrame.cmd.str.len = parse.commandSize;

	    if (parse.term == parse.commandStart + parse.commandSize - 1) {
		eeFrame.cmd.str.len --;
	    }

	    TclArgumentEnter (interp, objv, objectsUsed, &eeFrame);
	    iPtr->cmdFramePtr = &eeFrame;
#endif
	    iPtr->numLevels++;    
	    code = TclEvalObjvInternal(interp, objectsUsed, objv, 
	            parse.commandStart, parse.commandSize, 0);
	    iPtr->numLevels--;
#ifdef TCL_TIP280
	    iPtr->cmdFramePtr = iPtr->cmdFramePtr->nextPtr;
	    TclArgumentRelease (interp, objv, objectsUsed);

	    ckfree ((char*) eeFrame.line);

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclTrace.c`, function `TraceVarProc`, lines 2045–2062. Retained reference `e2`.

```text
	     * double-free might occur depending on what the eval does.
	     */

	    if ((flags & TCL_TRACE_DESTROYED)
		    && !(tvarPtr->flags & TCL_TRACE_DESTROYED)) {
		destroy = 1;
		tvarPtr->flags |= TCL_TRACE_DESTROYED;
	    }
	    code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		    Tcl_DStringLength(&cmd), 0);
	    if (code != TCL_OK) {		/* copy error msg to result */
		Tcl_Obj *errMsgObj = Tcl_GetObjResult(interp);
		Tcl_IncrRefCount(errMsgObj);
		result = (char *) errMsgObj;
	    }
	    Tcl_DStringFree(&cmd);
	}
    }

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclBasic.c`, function `TclEvalEx`, lines 4461–4478. Retained reference `e3`.

```text
		    parsePtr->commandStart + parsePtr->commandSize - 1) {
		eeFramePtr->cmd.str.len--;
	    }

	    eeFramePtr->nline = objectsUsed;
	    eeFramePtr->line = lines;

	    TclArgumentEnter (interp, objv, objectsUsed, eeFramePtr);
	    iPtr->cmdFramePtr = eeFramePtr;
	    iPtr->numLevels++;
	    code = TclEvalObjvInternal(interp, objectsUsed, objv,
		    parsePtr->commandStart, parsePtr->commandSize, 0);
	    iPtr->numLevels--;
	    iPtr->cmdFramePtr = iPtr->cmdFramePtr->nextPtr;
	    TclArgumentRelease (interp, objv, objectsUsed);

	    eeFramePtr->line = NULL;
	    eeFramePtr->nline = 0;

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclTrace.c`, function `TraceVarProc`, lines 2056–2073. Retained reference `e4`.

```text
	    /*
	     * Make sure that unset traces are rune even if the execEnv is
	     * rewinding (coroutine deletion, [Bug 2093947]
	     */

	    if (rewind && (flags & TCL_TRACE_UNSETS)) {
		((Interp *)interp)->execEnvPtr->rewind = 0;
	    }
	    code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		    Tcl_DStringLength(&cmd), 0);
	    if (rewind) {
		((Interp *)interp)->execEnvPtr->rewind = rewind;
	    }
	    if (code != TCL_OK) {		/* copy error msg to result */
		Tcl_Obj *errMsgObj = Tcl_GetObjResult(interp);

		Tcl_IncrRefCount(errMsgObj);
		result = (char *) errMsgObj;

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclBasic.c`, function `TclEvalEx`, lines 5448–5465. Retained reference `e5`.

```text

	    if (parsePtr->term ==
		    parsePtr->commandStart + parsePtr->commandSize - 1) {
		eeFramePtr->len--;
	    }

	    eeFramePtr->nline = objectsUsed;
	    eeFramePtr->line = lines;

	    TclArgumentEnter(interp, objv, objectsUsed, eeFramePtr);
	    code = Tcl_EvalObjv(interp, objectsUsed, objv,
		    TCL_EVAL_NOERR | TCL_EVAL_SOURCE_IN_FRAME);
	    TclArgumentRelease(interp, objv, objectsUsed);

	    eeFramePtr->line = NULL;
	    eeFramePtr->nline = 0;
	    if (eeFramePtr->cmdObj) {
		Tcl_DecrRefCount(eeFramePtr->cmdObj);

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclTrace.c`, function `TraceVarProc`, lines 1921–1938. Retained reference `e6`.

```text
	    /*
	     * Make sure that unset traces are rune even if the execEnv is
	     * rewinding (coroutine deletion, [Bug 2093947]
	     */

	    if (rewind && (flags & TCL_TRACE_UNSETS)) {
		((Interp *)interp)->execEnvPtr->rewind = 0;
	    }
	    code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		    Tcl_DStringLength(&cmd), 0);
	    if (rewind) {
		((Interp *)interp)->execEnvPtr->rewind = rewind;
	    }
	    if (code != TCL_OK) {		/* copy error msg to result */
		Tcl_Obj *errMsgObj = Tcl_GetObjResult(interp);

		Tcl_IncrRefCount(errMsgObj);
		result = (char *) errMsgObj;

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclBasic.c`, function `TclEvalEx`, lines 5621–5638. Retained reference `e7`.

```text

	    if (parsePtr->term ==
		    parsePtr->commandStart + parsePtr->commandSize - 1) {
		eeFramePtr->len--;
	    }

	    eeFramePtr->nline = objectsUsed;
	    eeFramePtr->line = lines;

	    TclArgumentEnter(interp, objv, objectsUsed, eeFramePtr);
	    code = Tcl_EvalObjv(interp, objectsUsed, objv,
		    TCL_EVAL_NOERR | TCL_EVAL_SOURCE_IN_FRAME);
	    TclArgumentRelease(interp, objv, objectsUsed);

	    eeFramePtr->line = NULL;
	    eeFramePtr->nline = 0;
	    if (eeFramePtr->cmdObj) {
		Tcl_DecrRefCount(eeFramePtr->cmdObj);

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclTrace.c`, function `TraceVarProc`, lines 1926–1943. Retained reference `e8`.

```text
	    /*
	     * Make sure that unset traces are rune even if the execEnv is
	     * rewinding (coroutine deletion, [Bug 2093947]
	     */

	    if (rewind && (flags & TCL_TRACE_UNSETS)) {
		((Interp *)interp)->execEnvPtr->rewind = 0;
	    }
	    code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		    Tcl_DStringLength(&cmd), 0);
	    if (rewind) {
		((Interp *)interp)->execEnvPtr->rewind = rewind;
	    }
	    if (code != TCL_OK) {		/* copy error msg to result */
		Tcl_Obj *errMsgObj = Tcl_GetObjResult(interp);

		Tcl_IncrRefCount(errMsgObj);
		result = (char *) errMsgObj;

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclBasic.c`, function `TclEvalEx`, lines 5579–5596. Retained reference `e9`.

```text

	    if (parsePtr->term ==
		    parsePtr->commandStart + parsePtr->commandSize - 1) {
		eeFramePtr->len--;
	    }

	    eeFramePtr->nline = objectsUsed;
	    eeFramePtr->line = lines;

	    TclArgumentEnter(interp, objv, objectsUsed, eeFramePtr);
	    code = Tcl_EvalObjv(interp, objectsUsed, objv,
		    TCL_EVAL_NOERR | TCL_EVAL_SOURCE_IN_FRAME);
	    TclArgumentRelease(interp, objv, objectsUsed);

	    eeFramePtr->line = NULL;
	    eeFramePtr->nline = 0;
	    if (eeFramePtr->cmdObj) {
		Tcl_DecrRefCount(eeFramePtr->cmdObj);

```

## Consumer bindings

- [rust/tcl-registry/src/native_eval_object.rs](../../../../rust/tcl-registry/src/native_eval_object.rs), `NativeEvalObjectProtocol::permits_direct_source_operands`: Purpose-separated matching engine source entry only.
- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs), `NativeDirectSourceOperands`: Fresh source objects for a genuine plain direct control plan; no native registered literal objects.
- [rust/tcl-registry/src/native_variable_trace.rs](../../../../rust/tcl-registry/src/native_variable_trace.rs), `NativeVariableTraceProtocol::callback_compilation`: Independently selected Direct ScriptCode trace source entry with loop0/catchSome0; triggering physical frame and copied-prefix lifetime remain separate.
- [rust/tcl-registry/src/native_eval_object.rs](../../../../rust/tcl-registry/src/native_eval_object.rs), `native_eval_object::trace_source_tests::copied_trace_scripts_select_direct_operands_without_borrowing_control_body_compilation` (linked): TraceCallback selects matching C direct operands, declines Jim/foreign version strings, and leaves ControlBody compilation independent.

## Evidence

- [rust/tcl-registry/tests/data/native_variable_trace_prefix/source-anchors](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/source-anchors)
