# naming.tcloo.original-forward-registration-prefix

Kind: `implementation-contract`

## Problem statement

A stock constant forward declaration captures a command prefix and defers its execution. Requiring a Body role loses the class world and later authentic source member context; treating an absent script entry as an absent method could instead select a hidden inherited script.

## Question

Can a fresh selected instance forward retain original prefix data and occupied method identity separately from future target dispatch?

## Conclusion

The selected Registry layout admits a nonempty original prefix vector. The common class declaration closure validates the actual stock worker and complete static source operands; a separate prefix inventory follows source-order replacement and one superclass, blocks hidden script and builtin advice, and preserves the existing fresh factory completion checks. Forward execution remains unproved.

## Scope

C8.6/C9.0/C9.1 fresh source class instance declarations with complete static original Word values, exact selected policy/config/source, stock workers and quiet factory premises. No computed/expanded prefix, class-object wrapper, custom worker, arbitrary old callback, forward-mixin method chain, target resolution, Native runtime entry, physical prefix storage/cache/pointer or successful forward invocation claim. Native question measurements are separate; all implementation provider statuses stay not-tested. Both readonly builtin candidate advice and the actual stock self-variable linker use the same current occupied override query; absence of a script body alone cannot establish stock variable-linker eligibility. One superclass is joined only through its current unaliased class allocation, matching implementation generation, complete lifecycle inventory and exact independently current selected factory/grammar receipt; the compatibility script-only rescan is not an additional authority. Inherited class eligibility joins the independently selected Registry class-system family, never a const descriptor address, and separately rechecks current factory allocation, implementation generation, lifecycle, method inventory and dependencies.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

No native execution of this source transfer implementation question.

## Source inspection

tcl8.6 8.6.18, revision `Exact full-file SHA retained; source revision not separately recorded by this probe.`, `tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOODefineForwardObjCmd`, lines 1488–1535. Retained reference `forward-source-0`.

```text
TclOODefineForwardObjCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    int isInstanceForward = (clientData != NULL);
    Object *oPtr;
    Method *mPtr;
    int isPublic;
    Tcl_Obj *prefixObj;

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "name cmdName ?arg ...?");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceForward && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }
    isPublic = Tcl_StringMatch(TclGetString(objv[1]), PUBLIC_PATTERN)
	    ? PUBLIC_METHOD : 0;

    /*
     * Create the method structure.
     */

    prefixObj = Tcl_NewListObj(objc - 2, objv + 2);
    if (isInstanceForward) {
	mPtr = TclOONewForwardInstanceMethod(interp, oPtr, isPublic, objv[1],
		prefixObj);
    } else {
	mPtr = TclOONewForwardMethod(interp, oPtr->classPtr, isPublic,
		objv[1], prefixObj);
    }
    if (mPtr == NULL) {
	Tcl_DecrRefCount(prefixObj);
	return TCL_ERROR;
    }
    return TCL_OK;
}
```

tcl8.6 8.6.18, revision `Exact full-file SHA retained; source revision not separately recorded by this probe.`, `tmp/tcl8.6.18/generic/tclOOMethod.c`, function `TclOONewForwardMethod`, lines 1356–1382. Retained reference `forward-source-1`.

```text
TclOONewForwardMethod(
    Tcl_Interp *interp,		/* Interpreter for error reporting. */
    Class *clsPtr,		/* The class to attach the method to. */
    int flags,			/* Whether the method is public or not. */
    Tcl_Obj *nameObj,		/* The name of the method. */
    Tcl_Obj *prefixObj)		/* List of arguments that form the command
				 * prefix to forward to. */
{
    int prefixLen;
    ForwardMethod *fmPtr;

    if (TclListObjLength(interp, prefixObj, &prefixLen) != TCL_OK) {
	return NULL;
    }
    if (prefixLen < 1) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"method forward prefix must be non-empty", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "BAD_FORWARD", (char *)NULL);
	return NULL;
    }

    fmPtr = (ForwardMethod *)ckalloc(sizeof(ForwardMethod));
    fmPtr->prefixObj = prefixObj;
    Tcl_IncrRefCount(prefixObj);
    return (Method *) Tcl_NewMethod(interp, (Tcl_Class) clsPtr, nameObj,
	    flags, &fwdMethodType, fmPtr);
}
```

tcl8.6 8.6.18, revision `Exact full-file SHA retained; source revision not separately recorded by this probe.`, `tmp/tcl8.6.18/generic/tclOOMethod.c`, function `InvokeForwardMethod`, lines 1396–1427. Retained reference `forward-source-2`.

```text
InvokeForwardMethod(
    void *clientData,	/* Pointer to some per-method context. */
    Tcl_Interp *interp,
    Tcl_ObjectContext context,	/* The method calling context. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Arguments as actually seen. */
{
    CallContext *contextPtr = (CallContext *) context;
    ForwardMethod *fmPtr = (ForwardMethod *)clientData;
    Tcl_Obj **argObjs, **prefixObjs;
    int numPrefixes, len, skip = contextPtr->skip;

    /*
     * Build the real list of arguments to use. Note that we know that the
     * prefixObj field of the ForwardMethod structure holds a reference to a
     * non-empty list, so there's a whole class of failures ("not a list") we
     * can ignore here.
     */

    TclListObjGetElements(NULL, fmPtr->prefixObj, &numPrefixes, &prefixObjs);
    argObjs = InitEnsembleRewrite(interp, objc, objv, skip,
	    numPrefixes, prefixObjs, &len);
    Tcl_NRAddCallback(interp, FinalizeForwardCall, argObjs, NULL, NULL, NULL);
    /*
     * NOTE: The combination of direct set of iPtr->lookupNsPtr and the use
     * of the TCL_EVAL_NOERR flag results in an evaluation configuration
     * very much like TCL_EVAL_INVOKE.
     */
    ((Interp *)interp)->lookupNsPtr
	    = (Namespace *) contextPtr->oPtr->namespacePtr;
    return TclNREvalObjv(interp, len, argObjs, TCL_EVAL_NOERR, NULL);
}
```

tcl8.6 8.6.18, revision `Exact full-file SHA retained; source revision not separately recorded by this probe.`, `tmp/tcl8.6.18/generic/tclOOMethod.c`, function `DeleteForwardMethod`, lines 1452–1459. Retained reference `forward-source-3`.

```text
DeleteForwardMethod(
    void *clientData)
{
    ForwardMethod *fmPtr = (ForwardMethod *)clientData;

    Tcl_DecrRefCount(fmPtr->prefixObj);
    ckfree(fmPtr);
}
```

tcl9.0 9.0.4, revision `Exact full-file SHA retained; source revision not separately recorded by this probe.`, `tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOODefineForwardObjCmd`, lines 2166–2216. Retained reference `forward-source-4`.

```text
TclOODefineForwardObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    int isInstanceForward = (clientData != NULL);
    Object *oPtr;
    Method *mPtr;
    int isPublic;
    Tcl_Obj *prefixObj;

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "name cmdName ?arg ...?");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceForward && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return TCL_ERROR;
    }
    isPublic = Tcl_StringMatch(TclGetString(objv[1]), PUBLIC_PATTERN)
	    ? PUBLIC_METHOD : 0;
    if (IsPrivateDefine(interp)) {
	isPublic = TRUE_PRIVATE_METHOD;
    }

    /*
     * Create the method structure.
     */

    prefixObj = Tcl_NewListObj(objc - 2, objv + 2);
    if (isInstanceForward) {
	mPtr = TclOONewForwardInstanceMethod(interp, oPtr, isPublic, objv[1],
		prefixObj);
    } else {
	mPtr = TclOONewForwardMethod(interp, oPtr->classPtr, isPublic,
		objv[1], prefixObj);
    }
    if (mPtr == NULL) {
	Tcl_DecrRefCount(prefixObj);
	return TCL_ERROR;
    }
    return TCL_OK;
}
```

tcl9.0 9.0.4, revision `Exact full-file SHA retained; source revision not separately recorded by this probe.`, `tmp/tcl9.0.4/generic/tclOOMethod.c`, function `TclOONewForwardMethod`, lines 1530–1556. Retained reference `forward-source-5`.

```text
TclOONewForwardMethod(
    Tcl_Interp *interp,		/* Interpreter for error reporting. */
    Class *clsPtr,		/* The class to attach the method to. */
    int flags,			/* Whether the method is public or not. */
    Tcl_Obj *nameObj,		/* The name of the method. */
    Tcl_Obj *prefixObj)		/* List of arguments that form the command
				 * prefix to forward to. */
{
    Tcl_Size prefixLen;
    ForwardMethod *fmPtr;

    if (TclListObjLength(interp, prefixObj, &prefixLen) != TCL_OK) {
	return NULL;
    }
    if (prefixLen < 1) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"method forward prefix must be non-empty", TCL_AUTO_LENGTH));
	OO_ERROR(interp, BAD_FORWARD);
	return NULL;
    }

    fmPtr = (ForwardMethod *) Tcl_Alloc(sizeof(ForwardMethod));
    fmPtr->prefixObj = prefixObj;
    Tcl_IncrRefCount(prefixObj);
    return (Method *) TclNewMethod((Tcl_Class) clsPtr, nameObj,
	    flags, &fwdMethodType, fmPtr);
}
```

tcl9.0 9.0.4, revision `Exact full-file SHA retained; source revision not separately recorded by this probe.`, `tmp/tcl9.0.4/generic/tclOOMethod.c`, function `InvokeForwardMethod`, lines 1570–1602. Retained reference `forward-source-6`.

```text
InvokeForwardMethod(
    void *clientData,		/* Pointer to some per-method context. */
    Tcl_Interp *interp,
    Tcl_ObjectContext context,	/* The method calling context. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Arguments as actually seen. */
{
    CallContext *contextPtr = (CallContext *) context;
    ForwardMethod *fmPtr = (ForwardMethod *) clientData;
    Tcl_Obj **argObjs, **prefixObjs;
    Tcl_Size numPrefixes, skip = contextPtr->skip;
    int len;

    /*
     * Build the real list of arguments to use. Note that we know that the
     * prefixObj field of the ForwardMethod structure holds a reference to a
     * non-empty list, so there's a whole class of failures ("not a list") we
     * can ignore here.
     */

    TclListObjGetElements(NULL, fmPtr->prefixObj, &numPrefixes, &prefixObjs);
    argObjs = InitEnsembleRewrite(interp, objc, objv, skip,
	    numPrefixes, prefixObjs, &len);
    TclNRAddCallback(interp, FinalizeForwardCall, argObjs, NULL, NULL, NULL);
    /*
     * NOTE: The combination of direct set of iPtr->lookupNsPtr and the use
     * of the TCL_EVAL_NOERR flag results in an evaluation configuration
     * very much like TCL_EVAL_INVOKE.
     */
    ((Interp *) interp)->lookupNsPtr = (Namespace *)
	    contextPtr->oPtr->namespacePtr;
    return TclNREvalObjv(interp, len, argObjs, TCL_EVAL_NOERR, NULL);
}
```

tcl9.0 9.0.4, revision `Exact full-file SHA retained; source revision not separately recorded by this probe.`, `tmp/tcl9.0.4/generic/tclOOMethod.c`, function `DeleteForwardMethod`, lines 1627–1634. Retained reference `forward-source-7`.

```text
DeleteForwardMethod(
    void *clientData)
{
    ForwardMethod *fmPtr = (ForwardMethod *) clientData;

    Tcl_DecrRefCount(fmPtr->prefixObj);
    Tcl_Free(fmPtr);
}
```

tcl9.1 9.1.0, revision `Exact full-file SHA retained; source revision not separately recorded by this probe.`, `tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOODefineForwardObjCmd`, lines 2129–2176. Retained reference `forward-source-8`.

```text
TclOODefineForwardObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    bool isInstanceForward = (clientData != NULL);
    Object *oPtr;
    Method *mPtr;
    int flags;
    Tcl_Obj *prefixObj;

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "name cmdName ?arg ...?");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceForward && !oPtr->classPtr) {
	return ReportAbuse(interp);
    }
    flags = Tcl_StringMatch(TclGetString(objv[1]), PUBLIC_PATTERN)
	    ? PUBLIC_METHOD : 0;
    if (IsPrivateDefine(interp)) {
	flags = TRUE_PRIVATE_METHOD;
    }

    /*
     * Create the method structure.
     */

    prefixObj = Tcl_NewListObj(objc - 2, objv + 2);
    if (isInstanceForward) {
	mPtr = TclOONewForwardInstanceMethod(interp, oPtr, flags, objv[1],
		prefixObj);
    } else {
	mPtr = TclOONewForwardMethod(interp, oPtr->classPtr, flags,
		objv[1], prefixObj);
    }
    if (mPtr == NULL) {
	Tcl_DecrRefCount(prefixObj);
	return TCL_ERROR;
    }
    return TCL_OK;
}
```

tcl9.1 9.1.0, revision `Exact full-file SHA retained; source revision not separately recorded by this probe.`, `tmp/tcl9.1.0/generic/tclOOMethod.c`, function `TclOONewForwardMethod`, lines 1593–1619. Retained reference `forward-source-9`.

```text
TclOONewForwardMethod(
    Tcl_Interp *interp,		/* Interpreter for error reporting. */
    Class *clsPtr,		/* The class to attach the method to. */
    int flags,			/* Whether the method is public or not. */
    Tcl_Obj *nameObj,		/* The name of the method. */
    Tcl_Obj *prefixObj)		/* List of arguments that form the command
				 * prefix to forward to. */
{
    Tcl_Size prefixLen;
    ForwardMethod *fmPtr;

    if (TclListObjLength(interp, prefixObj, &prefixLen) != TCL_OK) {
	return NULL;
    }
    if (prefixLen < 1) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"method forward prefix must be non-empty", TCL_AUTO_LENGTH));
	OO_ERROR(interp, BAD_FORWARD);
	return NULL;
    }

    fmPtr = (ForwardMethod *) Tcl_Alloc(sizeof(ForwardMethod));
    fmPtr->prefixObj = prefixObj;
    Tcl_IncrRefCount(prefixObj);
    return (Method *) TclNewMethod((Tcl_Class) clsPtr, nameObj,
	    flags, &fwdMethodType, fmPtr);
}
```

tcl9.1 9.1.0, revision `Exact full-file SHA retained; source revision not separately recorded by this probe.`, `tmp/tcl9.1.0/generic/tclOOMethod.c`, function `InvokeForwardMethod`, lines 1633–1665. Retained reference `forward-source-10`.

```text
InvokeForwardMethod(
    void *clientData,		/* Pointer to some per-method context. */
    Tcl_Interp *interp,
    Tcl_ObjectContext context,	/* The method calling context. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Arguments as actually seen. */
{
    CallContext *contextPtr = (CallContext *) context;
    ForwardMethod *fmPtr = (ForwardMethod *) clientData;
    Tcl_Obj **argObjs, **prefixObjs;
    Tcl_Size numPrefixes, skip = contextPtr->skip;
    Tcl_Size len;

    /*
     * Build the real list of arguments to use. Note that we know that the
     * prefixObj field of the ForwardMethod structure holds a reference to a
     * non-empty list, so there's a whole class of failures ("not a list") we
     * can ignore here.
     */

    TclListObjGetElements(NULL, fmPtr->prefixObj, &numPrefixes, &prefixObjs);
    argObjs = InitEnsembleRewrite(interp, objc, objv, skip,
	    numPrefixes, prefixObjs, &len);
    TclNRAddCallback(interp, FinalizeForwardCall, argObjs, NULL, NULL, NULL);
    /*
     * NOTE: The combination of direct set of iPtr->lookupNsPtr and the use
     * of the TCL_EVAL_NOERR flag results in an evaluation configuration
     * very much like TCL_EVAL_INVOKE.
     */
    ((Interp *) interp)->lookupNsPtr = (Namespace *)
	    contextPtr->oPtr->namespacePtr;
    return TclNREvalObjv(interp, len, argObjs, TCL_EVAL_NOERR, NULL);
}
```

tcl9.1 9.1.0, revision `Exact full-file SHA retained; source revision not separately recorded by this probe.`, `tmp/tcl9.1.0/generic/tclOOMethod.c`, function `DeleteForwardMethod`, lines 1690–1697. Retained reference `forward-source-11`.

```text
DeleteForwardMethod(
    void *clientData)
{
    ForwardMethod *fmPtr = (ForwardMethod *) clientData;

    Tcl_DecrRefCount(fmPtr->prefixObj);
    Tcl_Free(fmPtr);
}
```

## Consumer bindings

- [rust/tcl-registry/src/definer.rs](../../../../rust/tcl-registry/src/definer.rs), `native_deferred_forward_setter`: Select exact C release, admitted instance forward grammar and nonempty prefix layout without target dispatch.
- [rust/tcl-compiler/src/command_binding/deferred_forward.rs](../../../../rust/tcl-compiler/src/command_binding/deferred_forward.rs), `original_forward_method`: Retain immutable full-vector static name/prefix Inputs under the independently supplied selected source policy.
- [rust/tcl-compiler/src/command_binding/deferred_method.rs](../../../../rust/tcl-compiler/src/command_binding/deferred_method.rs), `closed_definition_member`: Use original selected worker and prefix closure at the common class declaration/factory boundary; source publication alone supplies no completion.
- [rust/tcl-compiler/src/command_binding/receiver_self.rs](../../../../rust/tcl-compiler/src/command_binding/receiver_self.rs), `retained_instance_method_entry`: A retained occupied forward cannot select a hidden script implementation.
- [rust/tcl-compiler/src/command_binding/receiver_self.rs](../../../../rust/tcl-compiler/src/command_binding/receiver_self.rs), `retained_instance_method_override`: Require actual current receiving allocation and selected method-name purpose before source script or forward occupancy excludes the stock variable linker.
- [rust/tcl-compiler/src/command_binding/deferred_method.rs](../../../../rust/tcl-compiler/src/command_binding/deferred_method.rs), `closed_inherited_member`: Select an actual current superclass through its sealed original class factory, implementation generation, local lifecycle and method inventory, without requiring a second script-only reporting body scan.
- [rust/tcl-compiler/src/command_binding/deferred_forward.rs](../../../../rust/tcl-compiler/src/command_binding/deferred_forward.rs), `command_binding::deferred_forward::tests::original_forward_registration_retains_prefix_without_entering_target` (linked): Static full original prefix values and counted distinct method names survive registration with unavailable future command target; no script entry or constructor callback is invented.
- [rust/tcl-compiler/src/command_binding/deferred_forward.rs](../../../../rust/tcl-compiler/src/command_binding/deferred_forward.rs), `command_binding::deferred_forward::tests::original_forward_masks_script_entries_and_future_dispatch_stays_unknown` (linked): Source-order replacement and one superclass preserve actual forward/script occupancy; forward invocation gets no selected script implementation or complete Normal world.
- [rust/tcl-compiler/src/command_binding/deferred_forward.rs](../../../../rust/tcl-compiler/src/command_binding/deferred_forward.rs), `command_binding::deferred_forward::tests::original_forward_registration_refuses_incomplete_or_unowned_prefixes` (linked): Missing command operand, substitutions/expansion, class-object wrapper and changed worker decline.
- [rust/tcl-registry/src/definer.rs](../../../../rust/tcl-registry/src/definer.rs), `definer::original_forward_tests::original_forward_layout_requires_selected_oo_and_nonempty_prefix` (linked): Selected C86/C90/C91 require at least name and one prefix value; C84/C85/Jim and other members decline.
- [rust/tcl-compiler/src/command_binding/receiver_self.rs](../../../../rust/tcl-compiler/src/command_binding/receiver_self.rs), `command_binding::receiver_self::tests::original_forward_override_cannot_select_the_stock_receiver_linker` (linked): A forward named variable blocks stock builtin candidate and the reached variable linker; source-only registration may complete while the unproved future call supplies no complete Normal world.

## Evidence

- [rust/tcl-registry/tests/data/native_tcloo_forward_registration/source-anchors.json](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/source-anchors.json)
