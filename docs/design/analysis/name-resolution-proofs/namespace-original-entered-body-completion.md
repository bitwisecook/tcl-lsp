# naming.namespace.original-entered-body-completion

Kind: `implementation-contract`

## Problem statement

A completed child script can lose the enclosing namespace handler completion, while a body descriptor alone cannot establish that the actual body or restored frame is represented.

## Question

Does an exact namespace body handoff retain independent enclosing completion only after the real common child walker completes and restores the original parent frame?

## Conclusion

The private handoff retains the selected intrinsic handler, exact original single body, namespace Ensure geometry and actual entered frame. It declines unexplained mutable-name accesses and callback residuals. Completion requires the common child outcomes to be complete with no abrupt alternative, the same body/handler/site/config, a known entered namespace and the actual parent restored. No world is reseeded and no body descriptor or callback footprint grants Normal.

## Scope

C Tcl source-model naming under Tcl 8.4, 8.5, 8.6, 9.0 and 9.1 selected policies, genuine original operands and independently retained ASCII namespace geometry. Opaque command/pattern units remain counted data under the shared selected native purposes. Jim export is a selected no-op; its source-name import helper and unimplemented forget do not receive C token or completion authority. Supplied native entries, generated opaque namespace allocation, arbitrary preload helpers, replacement cleanup, unknown callbacks and physical compiler/cache/namespace authority are excluded.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No native execution of this Rust implementation question is claimed.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No native execution of this Rust implementation question is claimed.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No native execution of this Rust implementation question is claimed.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No native execution of this Rust implementation question is claimed.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No native execution of this Rust implementation question is claimed.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No native execution of this Rust implementation question is claimed.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No native execution of this Rust implementation question is claimed.

## Exact evidence

- `tcl8.4-NamespaceEvalCmd-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.4/NamespaceEvalCmd.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.4/NamespaceEvalCmd.c). SHA-256 `38546ad3276e65d8196298f65165afc76af82032447f9e2ada35a43a151e9d13`. tcl8.4 retained NamespaceEvalCmd source; this is independent of Rust test or native invocation authority.
- `tcl8.5-NamespaceEvalCmd-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.5/NamespaceEvalCmd.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.5/NamespaceEvalCmd.c). SHA-256 `32b696430b740ea88259ac16d87ee17cf5b23553369e81f8c842b15592b6377d`. tcl8.5 retained NamespaceEvalCmd source; this is independent of Rust test or native invocation authority.
- `tcl8.6-NamespaceEvalCmd-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.6/NamespaceEvalCmd.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.6/NamespaceEvalCmd.c). SHA-256 `dbd0c261be810b44ffeb76d00f628adcb9e54d9aeb976b6607bef02078c7ba12`. tcl8.6 retained NamespaceEvalCmd source; this is independent of Rust test or native invocation authority.
- `tcl9.0-NamespaceEvalCmd-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.0/NamespaceEvalCmd.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.0/NamespaceEvalCmd.c). SHA-256 `65e9878ce7116591f12364671bb91ed295874c33e7d32a2b2992358487521fbb`. tcl9.0 retained NamespaceEvalCmd source; this is independent of Rust test or native invocation authority.
- `tcl9.1-NamespaceEvalCmd-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.1/NamespaceEvalCmd.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.1/NamespaceEvalCmd.c). SHA-256 `dbfdfb3de02c30dadc6f216e9559b82508daf20ed5867bf79668b14a109733a6`. tcl9.1 retained NamespaceEvalCmd source; this is independent of Rust test or native invocation authority.
- `jim-JimNamespaceCmd-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/jim/JimNamespaceCmd.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/jim/JimNamespaceCmd.c). SHA-256 `68cf42ac5f1a1fadf45a6ef3275144f466cbfd548737a300cb791d25f30a07b5`. jim retained JimNamespaceCmd source; this is independent of Rust test or native invocation authority.

## Source inspection

tcl8.4 8.4.20, revision `retained release source`, `tmp/tcl8.4.20/generic/tclNamesp.c`, function `NamespaceEvalCmd`, lines 2946–3038. Full-source SHA-256 `cd23865ee8d4b0ce81874d519b24de5b84a16c116ed3593ead366946b0512cbb`; snippet SHA-256 `38546ad3276e65d8196298f65165afc76af82032447f9e2ada35a43a151e9d13`; retained evidence `tcl8.4-NamespaceEvalCmd-source`.

```text
NamespaceEvalCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    Tcl_Namespace *namespacePtr;
    CallFrame frame;
    Tcl_Obj *objPtr;
    char *name;
    int length, result;

    if (objc < 4) {
        Tcl_WrongNumArgs(interp, 2, objv, "name arg ?arg...?");
        return TCL_ERROR;
    }

    /*
     * Try to resolve the namespace reference, caching the result in the
     * namespace object along the way.
     */

    result = GetNamespaceFromObj(interp, objv[2], &namespacePtr);
    if (result != TCL_OK) {
        return result;
    }

    /*
     * If the namespace wasn't found, try to create it.
     */
    
    if (namespacePtr == NULL) {
	name = Tcl_GetStringFromObj(objv[2], &length);
	namespacePtr = Tcl_CreateNamespace(interp, name, (ClientData) NULL, 
                (Tcl_NamespaceDeleteProc *) NULL);
	if (namespacePtr == NULL) {
	    return TCL_ERROR;
	}
    }

    /*
     * Make the specified namespace the current namespace and evaluate
     * the command(s).
     */

    result = Tcl_PushCallFrame(interp, (Tcl_CallFrame *) &frame, 
            namespacePtr, /*isProcCallFrame*/ 0);
    if (result != TCL_OK) {
        return TCL_ERROR;
    }
    frame.objc = objc;
    frame.objv = objv;  /* ref counts do not need to be incremented here */

    if (objc == 4) {
#ifndef TCL_TIP280
        result = Tcl_EvalObjEx(interp, objv[3], 0);
#else
        /* TIP #280 : Make actual argument location available to eval'd script */
        Interp* iPtr      = (Interp*) interp;
	CmdFrame* invoker = iPtr->cmdFramePtr;
	int word          = 3;
	TclArgumentGet (interp, objv[3], &invoker, &word);
        result = TclEvalObjEx(interp, objv[3], 0, invoker, word);
#endif
    } else {
	/*
	 * More than one argument: concatenate them together with spaces
	 * between, then evaluate the result.  Tcl_EvalObjEx will delete
	 * the object when it decrements its refcount after eval'ing it.
	 */
        objPtr = Tcl_ConcatObj(objc-3, objv+3);
#ifndef TCL_TIP280
        result = Tcl_EvalObjEx(interp, objPtr, TCL_EVAL_DIRECT);
#else
	/* TIP #280. Make invoking context available to eval'd script */
	result = TclEvalObjEx(interp, objPtr, TCL_EVAL_DIRECT, NULL, 0);
#endif
    }
    if (result == TCL_ERROR) {
        char msg[256 + TCL_INTEGER_SPACE];
	
        sprintf(msg, "\n    (in namespace eval \"%.200s\" script line %d)",
            namespacePtr->fullName, interp->errorLine);
        Tcl_AddObjErrorInfo(interp, msg, -1);
    }

    /*
     * Restore the previous "current" namespace.
     */
    
    Tcl_PopCallFrame(interp);
    return result;
}

```

tcl8.5 8.5.19, revision `retained release source`, `tmp/tcl8.5.19/generic/tclNamesp.c`, function `NamespaceEvalCmd`, lines 3254–3350. Full-source SHA-256 `60f944eb9ab669e183a546297c5aa6c8b62af0f79cdab115c74d37497c1fd83f`; snippet SHA-256 `32b696430b740ea88259ac16d87ee17cf5b23553369e81f8c842b15592b6377d`; retained evidence `tcl8.5-NamespaceEvalCmd-source`.

```text
NamespaceEvalCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Namespace *namespacePtr;
    CallFrame *framePtr, **framePtrPtr;
    Tcl_Obj *objPtr;
    int result;

    if (objc < 4) {
	Tcl_WrongNumArgs(interp, 2, objv, "name arg ?arg...?");
	return TCL_ERROR;
    }

    /*
     * Try to resolve the namespace reference, caching the result in the
     * namespace object along the way.
     */

    result = GetNamespaceFromObj(interp, objv[2], &namespacePtr);

    /*
     * If the namespace wasn't found, try to create it.
     */

    if (result == TCL_ERROR) {
	char *name = TclGetString(objv[2]);

	namespacePtr = Tcl_CreateNamespace(interp, name, NULL, NULL);
	if (namespacePtr == NULL) {
	    return TCL_ERROR;
	}
    }

    /*
     * Make the specified namespace the current namespace and evaluate the
     * command(s).
     */

    /* This is needed to satisfy GCC 3.3's strict aliasing rules */
    framePtrPtr = &framePtr;
    result = TclPushStackFrame(interp, (Tcl_CallFrame **) framePtrPtr,
	    namespacePtr, /*isProcCallFrame*/ 0);
    if (result != TCL_OK) {
	return TCL_ERROR;
    }

    framePtr->objc = objc;
    framePtr->objv = objv;

    if (objc == 4) {
	/*
	 * TIP #280: Make actual argument location available to eval'd script.
	 */

	Interp *iPtr      = (Interp *) interp;
	CmdFrame* invoker = iPtr->cmdFramePtr;
	int word          = 3;

	TclArgumentGet (interp, objv[3], &invoker, &word);
	result = TclEvalObjEx(interp, objv[3], 0, invoker, word);
    } else {
	/*
	 * More than one argument: concatenate them together with spaces
	 * between, then evaluate the result. Tcl_EvalObjEx will delete the
	 * object when it decrements its refcount after eval'ing it.
	 */

	objPtr = Tcl_ConcatObj(objc-3, objv+3);

	/*
	 * TIP #280: Make invoking context available to eval'd script.
	 */

	result = TclEvalObjEx(interp, objPtr, TCL_EVAL_DIRECT, NULL, 0);
    }

    if (result == TCL_ERROR) {
	int length = strlen(namespacePtr->fullName);
	int limit = 200;
	int overflow = (length > limit);

	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (in namespace eval \"%.*s%s\" script line %d)",
		(overflow ? limit : length), namespacePtr->fullName,
		(overflow ? "..." : ""), interp->errorLine));
    }

    /*
     * Restore the previous "current" namespace.
     */

    TclPopStackFrame(interp);
    return result;
}

```

tcl8.6 8.6.18, revision `retained release source`, `tmp/tcl8.6.18/generic/tclNamesp.c`, function `NamespaceEvalCmd`, lines 3262–3270. Full-source SHA-256 `b4b3095f69b2192f13bf355d70a0efeb9aa78b5e6ceccf6b17536cc5e9be1e24`; snippet SHA-256 `dbd0c261be810b44ffeb76d00f628adcb9e54d9aeb976b6607bef02078c7ba12`; retained evidence `tcl8.6-NamespaceEvalCmd-source`.

```text
NamespaceEvalCmd(
    ClientData clientData,	/* Arbitrary value passed to cmd. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    return Tcl_NRCallObjProc(interp, NRNamespaceEvalCmd, clientData, objc,
	    objv);
}

```

tcl9.0 9.0.4, revision `retained release source`, `tmp/tcl9.0.4/generic/tclNamesp.c`, function `NamespaceEvalCmd`, lines 3468–3476. Full-source SHA-256 `73e79904d662d12564dd7d35f922fd896566411524a6e14c6454325f2ce17b8d`; snippet SHA-256 `65e9878ce7116591f12364671bb91ed295874c33e7d32a2b2992358487521fbb`; retained evidence `tcl9.0-NamespaceEvalCmd-source`.

```text
NamespaceEvalCmd(
    void *clientData,		/* Arbitrary value passed to cmd. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    return Tcl_NRCallObjProc(interp, NRNamespaceEvalCmd, clientData, objc,
	    objv);
}

```

tcl9.1 9.1.0, revision `retained release source`, `tmp/tcl9.1.0/generic/tclNamesp.c`, function `NamespaceEvalCmd`, lines 3440–3448. Full-source SHA-256 `fce8ca8ee9a21f2e87f7a1917e7a2e4109f9fadf72fd0e585673d1450409b815`; snippet SHA-256 `dbfdfb3de02c30dadc6f216e9559b82508daf20ed5867bf79668b14a109733a6`; retained evidence `tcl9.1-NamespaceEvalCmd-source`.

```text
NamespaceEvalCmd(
    void *clientData,		/* Arbitrary value passed to cmd. */
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    return Tcl_NRCallObjProc2(interp, NRNamespaceEvalCmd, clientData, objc,
	    objv);
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-namespace.c`, function `JimNamespaceCmd`, lines 193–328. Full-source SHA-256 `22bb6fb5fcea8ce8ec7e3eecbfe5ebd5ee290b9b725101ff4e4a1c0e9abed7f3`; snippet SHA-256 `68cf42ac5f1a1fadf45a6ef3275144f466cbfd548737a300cb791d25f30a07b5`; retained evidence `jim-JimNamespaceCmd-source`.

```text
static int JimNamespaceCmd(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    /* Must be kept in order with the array below */
    enum {
        OPT_CANONICAL,
        OPT_CODE,
        OPT_CURRENT,
        OPT_DELETE,
        OPT_ENSEMBLE,
        OPT_EVAL,
        OPT_EXPORT,
        OPT_IMPORT,
        OPT_INSCOPE,
        OPT_ORIGIN,
        OPT_PARENT,
        OPT_QUALIFIERS,
        OPT_TAIL,
        OPT_UPVAR,
        OPT_WHICH,
        OPT_COUNT,
    };
    static const jim_subcmd_type namespace_cmds[OPT_COUNT + 1] = {
        JIM_DEF_SUBCMD("canonical", "?current? ?name?", 0, 2),
        JIM_DEF_SUBCMD("code", "arg", 1, 1),
        JIM_DEF_SUBCMD("current", NULL, 0, 0),
        JIM_DEF_SUBCMD("delete", "?name ...?", 0, -1),
        JIM_DEF_SUBCMD("ensemble", "subcommand ?arg ...?", 1, -1),
        JIM_DEF_SUBCMD("eval", "name arg ?arg ...?", 2, -1),
        JIM_DEF_SUBCMD("export", "?...?", 0, -1),
        JIM_DEF_SUBCMD("import", "?...?", 0, -1),
        JIM_DEF_SUBCMD("inscope", "name arg ?arg ...?", 2, -1),
        JIM_DEF_SUBCMD("origin", "name", 1, 1),
        JIM_DEF_SUBCMD("parent", "?name?", 0, 1),
        JIM_DEF_SUBCMD("qualifiers", "string", 1, 1),
        JIM_DEF_SUBCMD("tail", "string", 1, 1),
        JIM_DEF_SUBCMD("upvar", "ns ?arg ...?", 1, -1),
        JIM_DEF_SUBCMD("which", "?-command|-variable? name", 1, 2),
        { NULL }
    };
    const jim_subcmd_type *ct = Jim_ParseSubCmd(interp, namespace_cmds, argc, argv);
    if (ct) {
        if (ct->function) {
            /* This is -help */
            return ct->function(interp, argc, argv);
        }

        /* (ct - namespace_cmds) is the index into the table */
        switch (ct - namespace_cmds) {
            case OPT_EVAL:
                {
                    Jim_Obj *nsObj;
                    Jim_Obj *objPtr;
                    if (argc == 4) {
                        objPtr = argv[3];
                    }
                    else {
                        objPtr = Jim_ConcatObj(interp, argc - 3, argv + 3);
                    }

                    nsObj = JimCanonicalNamespace(interp, interp->framePtr->nsObj, argv[2]);
                    return Jim_EvalNamespace(interp, objPtr, nsObj);
                }

            case OPT_CURRENT:
                Jim_SetResult(interp, JimNamespaceCurrent(interp));
                return JIM_OK;

            case OPT_CANONICAL:
                if (argc == 2) {
                    Jim_SetResult(interp, interp->framePtr->nsObj);
                }
                else if (argc == 3) {
                    Jim_SetResult(interp, JimCanonicalNamespace(interp, interp->framePtr->nsObj, argv[2]));
                }
                else {
                    Jim_SetResult(interp, JimCanonicalNamespace(interp, argv[2], argv[3]));
                }
                return JIM_OK;

            case OPT_QUALIFIERS:
                Jim_SetResult(interp, Jim_NamespaceQualifiers(interp, argv[2]));
                return JIM_OK;

            case OPT_EXPORT:
                return JIM_OK;

            case OPT_TAIL:
                if (argc != 3) {
                    Jim_WrongNumArgs(interp, 2, argv, "string");
                    return JIM_ERR;
                }
                Jim_SetResult(interp, Jim_NamespaceTail(interp, argv[2]));
                return JIM_OK;

            case OPT_PARENT:
                {
                    Jim_Obj *objPtr;
                    const char *name;

                    if (argc == 3) {
                        objPtr = argv[2];
                    }
                    else {
                        objPtr = interp->framePtr->nsObj;
                    }
                    if (Jim_Length(objPtr) == 0 || Jim_CompareStringImmediate(interp, objPtr, "::")) {
                        return JIM_OK;
                    }
                    objPtr = Jim_NamespaceQualifiers(interp, objPtr);

                    name = Jim_String(objPtr);

                    if (name[0] != ':' || name[1] != ':') {
                        /* Make it fully scoped */
                        Jim_SetResultString(interp, "::", 2);
                        Jim_AppendObj(interp, Jim_GetResult(interp), objPtr);
                        Jim_IncrRefCount(objPtr);
                        Jim_DecrRefCount(interp, objPtr);
                    }
                    else {
                        Jim_SetResult(interp, objPtr);
                    }
                }
                return JIM_OK;

                default:
                    /* Implemented as a Tcl helper proc.
                     * Note that calling a proc will change the current namespace,
                     * so helper procs must call [uplevel namespace canon] to get the callers
                     * namespace.
                     */
                    return Jim_EvalEnsemble(interp, "namespace", Jim_String(argv[1]), argc - 2, argv + 2);
            }
    }
    return JIM_ERR;
}

```


## Consumer bindings

- [rust/tcl-compiler/src/command_binding/original_namespace_body.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_body.rs), `OriginalNamespaceBodyDelegation::capture`: Exact selected handler and single original script/Ensure correspondence without completion.
- [rust/tcl-compiler/src/command_binding/original_namespace_body.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_body.rs), `OriginalNamespaceBody::capture`: Actual selected entered namespace and frame independent of the descriptor.
- [rust/tcl-compiler/src/command_binding/original_namespace_body.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_body.rs), `OriginalNamespaceBody::completed`: Common child completion and genuine parent restoration before the enclosing seal.
- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `SourceCommandBindings::walk_selected_native_body`: Use the existing common walker and frame restoration, then independently validate enclosing completion.
- [rust/tcl-compiler/src/command_binding/original_namespace_body.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_body.rs), `command_binding::original_namespace_body::tests::original_namespace_body_keeps_completed_child_and_restores_parent` (linked): A genuine completed nested procedure declaration survives in the root world and the parent global frame is restored.
- [rust/tcl-compiler/src/command_binding/original_namespace_body.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_body.rs), `command_binding::original_namespace_body::tests::original_namespace_body_refuses_unrepresented_and_abrupt_children` (linked): Unknown, abrupt and dynamically unrepresented children provide no enclosing completion; an independently complete shadowing procedure creates no namespace.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact named Rust selectors are implementation coverage obligations without an attached execution receipt. The independently retained source controls and source excerpts do not grant Rust handler selection, completion or physical native authority.
