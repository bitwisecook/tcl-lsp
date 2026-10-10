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

### tcl8.4

Status: `inspected`. Version: 8.4.20. Build: No new guest execution for this source-anchor question; exact source-file hashes/lines are retained independently of native probe builds.. Channel: Copied counted DString source supplied by TraceVarProc to Tcl_EvalEx(flags0).. Dialect: Tcl.

Inspected counted callback EvalEx and original source object-vector dispatch callsites; no original ControlBody object is passed.

### tcl8.5

Status: `inspected`. Version: 8.5.19. Build: No new guest execution for this source-anchor question; exact source-file hashes/lines are retained independently of native probe builds.. Channel: Copied counted DString source supplied by TraceVarProc to Tcl_EvalEx(flags0).. Dialect: Tcl.

Inspected counted callback EvalEx and original source object-vector dispatch callsites; no original ControlBody object is passed.

### tcl8.6

Status: `inspected`. Version: 8.6.18. Build: No new guest execution for this source-anchor question; exact source-file hashes/lines are retained independently of native probe builds.. Channel: Copied counted DString source supplied by TraceVarProc to Tcl_EvalEx(flags0).. Dialect: Tcl.

Inspected counted callback EvalEx and original source object-vector dispatch callsites; no original ControlBody object is passed.

### tcl9.0

Status: `inspected`. Version: 9.0.4. Build: No new guest execution for this source-anchor question; exact source-file hashes/lines are retained independently of native probe builds.. Channel: Copied counted DString source supplied by TraceVarProc to Tcl_EvalEx(flags0).. Dialect: Tcl.

Inspected counted callback EvalEx and original source object-vector dispatch callsites; no original ControlBody object is passed.

### tcl9.1

Status: `inspected`. Version: 9.1.0. Build: No new guest execution for this source-anchor question; exact source-file hashes/lines are retained independently of native probe builds.. Channel: Copied counted DString source supplied by TraceVarProc to Tcl_EvalEx(flags0).. Dialect: Tcl.

Inspected counted callback EvalEx and original source object-vector dispatch callsites; no original ControlBody object is passed.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (source-anchor): [rust/tcl-registry/tests/data/native_variable_trace_prefix/source-anchors/8.4.20-TraceVarProc.txt](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/source-anchors/8.4.20-TraceVarProc.txt). SHA-256 `618870335b0cc62363ca52dab51e139c86c693754a521bd68393d1a95e22e742`. Exact TraceVarProc counted evaluation callsite excerpt; source full-file hash and original lines separate.
- `e1` (source-anchor): [rust/tcl-registry/tests/data/native_variable_trace_prefix/source-anchors/8.4.20-source-object-dispatch.txt](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/source-anchors/8.4.20-source-object-dispatch.txt). SHA-256 `6e9e91803affdfa4a238cb96b062404da2d83d275b0391b55fd28d1b46dec217`. Exact counted source evaluator reached object-vector dispatch: TclEvalObjvInternal on C84/85, Tcl_EvalObjv on C86/90/91; excludes the unrelated List-body branch.
- `e2` (source-anchor): [rust/tcl-registry/tests/data/native_variable_trace_prefix/source-anchors/8.5.19-TraceVarProc.txt](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/source-anchors/8.5.19-TraceVarProc.txt). SHA-256 `4ef99a3dacfa9511cef6379a2f79b577f90622abbe791978866606c25802d682`. Exact TraceVarProc counted evaluation callsite excerpt; source full-file hash and original lines separate.
- `e3` (source-anchor): [rust/tcl-registry/tests/data/native_variable_trace_prefix/source-anchors/8.5.19-source-object-dispatch.txt](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/source-anchors/8.5.19-source-object-dispatch.txt). SHA-256 `25b8dbbfb632e54e6cd57fac4c4ed181158a70566f7ad8c9beca212750e83c61`. Exact counted source evaluator reached object-vector dispatch: TclEvalObjvInternal on C84/85, Tcl_EvalObjv on C86/90/91; excludes the unrelated List-body branch.
- `e4` (source-anchor): [rust/tcl-registry/tests/data/native_variable_trace_prefix/source-anchors/8.6.18-TraceVarProc.txt](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/source-anchors/8.6.18-TraceVarProc.txt). SHA-256 `6e0697a1677220d78ca1db76fd16057f61024abb332bd54ad39d4af738e1bb5a`. Exact TraceVarProc counted evaluation callsite excerpt; source full-file hash and original lines separate.
- `e5` (source-anchor): [rust/tcl-registry/tests/data/native_variable_trace_prefix/source-anchors/8.6.18-source-object-dispatch.txt](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/source-anchors/8.6.18-source-object-dispatch.txt). SHA-256 `a25037c1d9ed2c3bc155a6d877a78067c448205c3093686f5424d7992d4f60d2`. Exact counted source evaluator reached object-vector dispatch: TclEvalObjvInternal on C84/85, Tcl_EvalObjv on C86/90/91; excludes the unrelated List-body branch.
- `e6` (source-anchor): [rust/tcl-registry/tests/data/native_variable_trace_prefix/source-anchors/9.0.4-TraceVarProc.txt](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/source-anchors/9.0.4-TraceVarProc.txt). SHA-256 `6e0697a1677220d78ca1db76fd16057f61024abb332bd54ad39d4af738e1bb5a`. Exact TraceVarProc counted evaluation callsite excerpt; source full-file hash and original lines separate.
- `e7` (source-anchor): [rust/tcl-registry/tests/data/native_variable_trace_prefix/source-anchors/9.0.4-source-object-dispatch.txt](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/source-anchors/9.0.4-source-object-dispatch.txt). SHA-256 `a25037c1d9ed2c3bc155a6d877a78067c448205c3093686f5424d7992d4f60d2`. Exact counted source evaluator reached object-vector dispatch: TclEvalObjvInternal on C84/85, Tcl_EvalObjv on C86/90/91; excludes the unrelated List-body branch.
- `e8` (source-anchor): [rust/tcl-registry/tests/data/native_variable_trace_prefix/source-anchors/9.1.0-TraceVarProc.txt](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/source-anchors/9.1.0-TraceVarProc.txt). SHA-256 `6e0697a1677220d78ca1db76fd16057f61024abb332bd54ad39d4af738e1bb5a`. Exact TraceVarProc counted evaluation callsite excerpt; source full-file hash and original lines separate.
- `e9` (source-anchor): [rust/tcl-registry/tests/data/native_variable_trace_prefix/source-anchors/9.1.0-source-object-dispatch.txt](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/source-anchors/9.1.0-source-object-dispatch.txt). SHA-256 `a25037c1d9ed2c3bc155a6d877a78067c448205c3093686f5424d7992d4f60d2`. Exact counted source evaluator reached object-vector dispatch: TclEvalObjvInternal on C84/85, Tcl_EvalObjv on C86/90/91; excludes the unrelated List-body branch.

## Source inspection

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclCmdMZ.c`, function `TraceVarProc`, lines 4927–4944. Full-source SHA-256 `b374cf4353b86235b189d432239a1eedf8e854eab266b701e9d9625b969d27dc`; snippet SHA-256 `618870335b0cc62363ca52dab51e139c86c693754a521bd68393d1a95e22e742`; retained evidence `e0`.

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

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclBasic.c`, function `EvalEx`, lines 4256–4273. Full-source SHA-256 `cab6d4decd1a3e365fe3cef49a2d5430dfef01cd775dd7d34f98064032f3a81a`; snippet SHA-256 `6e9e91803affdfa4a238cb96b062404da2d83d275b0391b55fd28d1b46dec217`; retained evidence `e1`.

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

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclTrace.c`, function `TraceVarProc`, lines 2045–2062. Full-source SHA-256 `d974ce7d57afd060edbc9a858975e17c9577c7905f5dfaf6df477fa409ef89e6`; snippet SHA-256 `4ef99a3dacfa9511cef6379a2f79b577f90622abbe791978866606c25802d682`; retained evidence `e2`.

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

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclBasic.c`, function `TclEvalEx`, lines 4461–4478. Full-source SHA-256 `d09c16386ea0376dc3590a74ded2c6e2bcef5079f2b1a3785545bad37c7ead63`; snippet SHA-256 `25b8dbbfb632e54e6cd57fac4c4ed181158a70566f7ad8c9beca212750e83c61`; retained evidence `e3`.

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

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclTrace.c`, function `TraceVarProc`, lines 2056–2073. Full-source SHA-256 `8034d0a6da9f529bebeedfa4ab8af88f944bf858ad8c498719a972ac81d503a9`; snippet SHA-256 `6e0697a1677220d78ca1db76fd16057f61024abb332bd54ad39d4af738e1bb5a`; retained evidence `e4`.

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

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclBasic.c`, function `TclEvalEx`, lines 5448–5465. Full-source SHA-256 `19e28c2c9fbcd27e7e538a06960f0d806d04f8c25d94499cf9eb9031f8440198`; snippet SHA-256 `a25037c1d9ed2c3bc155a6d877a78067c448205c3093686f5424d7992d4f60d2`; retained evidence `e5`.

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

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclTrace.c`, function `TraceVarProc`, lines 1921–1938. Full-source SHA-256 `b091fe603d801002a17d6d4c18de08f43e7aa7851abc64849ab2b1f7aac52904`; snippet SHA-256 `6e0697a1677220d78ca1db76fd16057f61024abb332bd54ad39d4af738e1bb5a`; retained evidence `e6`.

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

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclBasic.c`, function `TclEvalEx`, lines 5621–5638. Full-source SHA-256 `6d995c1af94fea92717183882ca41775fe8c0e088def163f6ac8fe699cbffc67`; snippet SHA-256 `a25037c1d9ed2c3bc155a6d877a78067c448205c3093686f5424d7992d4f60d2`; retained evidence `e7`.

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

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclTrace.c`, function `TraceVarProc`, lines 1926–1943. Full-source SHA-256 `b39de1869c493f1405355c9f92537bdd5d9839a42a10da78b18a725309d71847`; snippet SHA-256 `6e0697a1677220d78ca1db76fd16057f61024abb332bd54ad39d4af738e1bb5a`; retained evidence `e8`.

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

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclBasic.c`, function `TclEvalEx`, lines 5579–5596. Full-source SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`; snippet SHA-256 `a25037c1d9ed2c3bc155a6d877a78067c448205c3093686f5424d7992d4f60d2`; retained evidence `e9`.

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

A named test is a coverage binding, not a claim that it executed.

## Replay

Reconfirmation is source inspection against exact retained full-file SHA and original function/lines. The independent native prefix probe above checks reached callback bytes but does not observe bytecode-object or literal-pool internals. Rust test execution requires a separately captured current-source result; none is attached here.
