# naming.tcloo.object-eval-original-script

Kind: `source-anchor`

## Problem statement

Stock object eval currently decodes a single script as Rust String or reconstructs multiple scripts with an ad hoc join. That can reject accepted counted bytes and replace original object/list children before the selected evaluation owner.

## Question

Does stock TclOO object eval preserve a single original script operand, and which operation constructs a multi-operand script?

## Conclusion

All inspected C86/C90/C91 TclOO_Object_Eval bodies retain objv[skip] for exactly one script operand. Multiple operands are passed to Tcl_ConcatObj. The handler enters the actual object namespace/method frame and passes the resulting original script object to flags0 NRE object evaluation. The adapters use existing concat_selected and original ControlBody object evaluation; actual method selection, namespace/frame capability, native cache/list rules, compiler admission, observers and completion remain independent.

## Scope

Pinned source inspection of Tcl8.6.18/9.0.4/9.1.0 stock object eval handler only. No native launch, actual byte/header/cache observation, selected method replacement/dispatch proof, Normal/body equivalence, custom refcount/release or error-options equality. Tcl84/85, Jim and BIG-IP uninspected for this exact operation.

## Provider answers

### tcl8.4, tcl8.5, jim

Status: `not-tested`.

No observation for this question.

### tcl8.6, tcl9.0, tcl9.1

Status: `inspected`. Channel: Inspected stock native TclOO object eval handler; no guest input. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Single original script operand is retained; multiple original operands go through Tcl_ConcatObj before flags0 original object evaluation in the object frame.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Source inspection

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOBasic.c`, function `TclOO_Object_Eval`, lines 392–451. Retained reference `tcl8.6-object-eval`.

```text
TclOO_Object_Eval(
    ClientData clientData,	/* Ignored. */
    Tcl_Interp *interp,		/* Interpreter in which to create the object;
				 * also used for error reporting. */
    Tcl_ObjectContext context,	/* The object/call context. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const *objv)	/* The actual arguments. */
{
    CallContext *contextPtr = (CallContext *) context;
    Tcl_Object object = Tcl_ObjectContextObject(context);
    const int skip = Tcl_ObjectContextSkippedArgs(context);
    CallFrame *framePtr, **framePtrPtr = &framePtr;
    Tcl_Obj *scriptPtr;
    CmdFrame *invoker;

    if (objc-1 < skip) {
	Tcl_WrongNumArgs(interp, skip, objv, "arg ?arg ...?");
	return TCL_ERROR;
    }

    /*
     * Make the object's namespace the current namespace and evaluate the
     * command(s).
     */

    (void) TclPushStackFrame(interp, (Tcl_CallFrame **) framePtrPtr,
	    Tcl_GetObjectNamespace(object), FRAME_IS_METHOD);
    framePtr->clientData = context;
    framePtr->objc = objc;
    framePtr->objv = objv;	/* Reference counts do not need to be
				 * incremented here. */

    if (!(contextPtr->callPtr->flags & PUBLIC_METHOD)) {
	object = NULL;		/* Now just for error mesage printing. */
    }

    /*
     * Work out what script we are actually going to evaluate.
     *
     * When there's more than one argument, we concatenate them together with
     * spaces between, then evaluate the result. Tcl_EvalObjEx will delete the
     * object when it decrements its refcount after eval'ing it.
     */

    if (objc != skip+1) {
	scriptPtr = Tcl_ConcatObj(objc-skip, objv+skip);
	invoker = NULL;
    } else {
	scriptPtr = objv[skip];
	invoker = ((Interp *) interp)->cmdFramePtr;
    }

    /*
     * Evaluate the script now, with FinalizeEval to do the processing after
     * the script completes.
     */

    TclNRAddCallback(interp, FinalizeEval, object, NULL, NULL, NULL);
    return TclNREvalObjEx(interp, scriptPtr, 0, invoker, skip);
}

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOBasic.c`, function `TclOO_Object_Eval`, lines 733–792. Retained reference `tcl9.0-object-eval`.

```text
TclOO_Object_Eval(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Interpreter in which to create the object;
				 * also used for error reporting. */
    Tcl_ObjectContext context,	/* The object/call context. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const *objv)	/* The actual arguments. */
{
    CallContext *contextPtr = (CallContext *) context;
    Tcl_Object object = Tcl_ObjectContextObject(context);
    size_t skip = Tcl_ObjectContextSkippedArgs(context);
    CallFrame *framePtr, **framePtrPtr = &framePtr;
    Tcl_Obj *scriptPtr;
    CmdFrame *invoker;

    if ((size_t) objc < skip + 1) {
	Tcl_WrongNumArgs(interp, skip, objv, "arg ?arg ...?");
	return TCL_ERROR;
    }

    /*
     * Make the object's namespace the current namespace and evaluate the
     * command(s).
     */

    (void) TclPushStackFrame(interp, (Tcl_CallFrame **) framePtrPtr,
	    Tcl_GetObjectNamespace(object), FRAME_IS_METHOD);
    framePtr->clientData = context;
    framePtr->objc = objc;
    framePtr->objv = objv;	/* Reference counts do not need to be
				 * incremented here. */

    if (!(contextPtr->callPtr->flags & PUBLIC_METHOD)) {
	object = NULL;		/* Now just for error mesage printing. */
    }

    /*
     * Work out what script we are actually going to evaluate.
     *
     * When there's more than one argument, we concatenate them together with
     * spaces between, then evaluate the result. Tcl_EvalObjEx will delete the
     * object when it decrements its refcount after eval'ing it.
     */

    if ((size_t) objc != skip+1) {
	scriptPtr = Tcl_ConcatObj(objc-skip, objv+skip);
	invoker = NULL;
    } else {
	scriptPtr = objv[skip];
	invoker = ((Interp *) interp)->cmdFramePtr;
    }

    /*
     * Evaluate the script now, with FinalizeEval to do the processing after
     * the script completes.
     */

    TclNRAddCallback(interp, FinalizeEval, object, NULL, NULL, NULL);
    return TclNREvalObjEx(interp, scriptPtr, 0, invoker, skip);
}

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOBasic.c`, function `TclOO_Object_Eval`, lines 789–848. Retained reference `tcl9.1-object-eval`.

```text
TclOO_Object_Eval(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Interpreter in which to create the object;
				 * also used for error reporting. */
    Tcl_ObjectContext context,	/* The object/call context. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* The actual arguments. */
{
    CallContext *contextPtr = (CallContext *) context;
    Tcl_Object object = Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    CallFrame *framePtr, **framePtrPtr = &framePtr;
    Tcl_Obj *scriptPtr;
    CmdFrame *invoker;

    if (objc < skip + 1) {
	Tcl_WrongNumArgs(interp, skip, objv, "arg ?arg ...?");
	return TCL_ERROR;
    }

    /*
     * Make the object's namespace the current namespace and evaluate the
     * command(s).
     */

    (void)TclPushStackFrame(interp, (Tcl_CallFrame **)framePtrPtr,
	    Tcl_GetObjectNamespace(object), FRAME_IS_METHOD);
    framePtr->clientData = context;
    framePtr->objc = objc;
    framePtr->objv = objv;	/* Reference counts do not need to be
				 * incremented here. */

    if (!(contextPtr->callPtr->flags & PUBLIC_METHOD)) {
	object = NULL;		/* Now just for error mesage printing. */
    }

    /*
     * Work out what script we are actually going to evaluate.
     *
     * When there's more than one argument, we concatenate them together with
     * spaces between, then evaluate the result. Tcl_EvalObjEx will delete the
     * object when it decrements its refcount after eval'ing it.
     */

    if (objc != skip+1) {
	scriptPtr = Tcl_ConcatObj(objc-skip, objv+skip);
	invoker = NULL;
    } else {
	scriptPtr = objv[skip];
	invoker = ((Interp *) interp)->cmdFramePtr;
    }

    /*
     * Evaluate the script now, with FinalizeEval to do the processing after
     * the script completes.
     */

    TclNRAddCallback(interp, FinalizeEval, object, NULL, NULL, NULL);
    return TclNREvalObjEx(interp, scriptPtr, 0, invoker, skip);
}

```

## Consumer bindings

- [rust/tcl-vm/src/cmd_oo.rs](../../../../rust/tcl-vm/src/cmd_oo.rs), `builtin_method`: Stock object eval arm retains one operand or delegates multi-operand construction, preserving actual frame/namespace and compilation refusal.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `Interp::oo_builtin_method`: Stock object eval arm retains original pointer or owned selected concat result and enters generic original object evaluation.
- [rust/tcl-cmd-core/src/list.rs](../../../../rust/tcl-cmd-core/src/list.rs), `concat_selected`: Independently selected original concat operation; this source record does not mint cache/renderer/materialization permission.
- [rust/tcl-vm/src/cmd_oo/native_eval_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_eval_tests.rs), `cmd_oo::native_eval_tests::object_eval_retains_original_script_and_concat_children` (linked): Implementation contract exercises current actual core object adapter with original List versus two original concat children, returns same opaque child and preserves source operand lack of string representation; no native/Rust launch claim.
- [runtime/rust/src/cmd_oo/native_eval_tests.rs](../../../../runtime/rust/src/cmd_oo/native_eval_tests.rs), `cmd_oo::native_eval_tests::object_eval_retains_original_script_and_concat_children` (linked): Independent Runtime implementation contract for original object/concat transport at current object frame; no Native object or completion authority inferred.

## Evidence

- [rust/tcl-cmd-core/tests/data/native_tcloo_object_eval_source/tcl8.6-source.json](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_object_eval_source/tcl8.6-source.json)
- [rust/tcl-cmd-core/tests/data/native_tcloo_object_eval_source/tcl9.0-source.json](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_object_eval_source/tcl9.0-source.json)
- [rust/tcl-cmd-core/tests/data/native_tcloo_object_eval_source/tcl9.1-source.json](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_object_eval_source/tcl9.1-source.json)
