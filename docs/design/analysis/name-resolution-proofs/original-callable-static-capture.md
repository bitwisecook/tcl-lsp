# naming.variable.original-callable-static-capture

Kind: `implementation-contract`

## Problem statement

A String-facing static-list consumer can merge counted names or lose opaque initialiser bytes. Treating reference capture as a value read also confuses wrapper presence with link-target contents, while publishing declarations incrementally can leave partial bindings after a later error.

## Question

How does source callable-static capture retain the actual native operand and distinguish copied contents from retained raw wrappers without fabricating completion or lifetime authority?

## Conclusion

The original static-list operand and its native list children select counted static names, literal initialisers, copied values and reference wrappers independently. Callable allocation identity owns the retained slots; copied values retain their own retained producer graph, while references retain the original wrapper and follow its logical-level name link. Raw wrapper presence is separate from target contents: a reference can retain an installed name link whose target is missing, whereas a value copy fails. Preparation publishes atomically and does not supply Normal completion; unresolved capture errors cannot close the source definition. Current input, naming policy, observer and cell checks remain independent of representation caches. Original unset uses the actual named slot separately from its retained wrapper: direct callable-static fallback contents remain intact, while detaching a pinned named cell preserves held contents across later name recreation.

## Scope

Rust source-cell model for the selected Jim084 callable static grammar and actual definition operand. Tests use original NativeValue words with counted zero/opaque bytes and explicitly supplied conditional frames. The inspected pinned Jim source distinguishes raw wrapper selection from target evaluation. No new native guest run, physical VarVal pointer, release-hook closure, successful opaque body execution, edit or BIG-IP authority is claimed.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance execution for this question is attached.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance execution for this question is attached.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance execution for this question is attached.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance execution for this question is attached.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance execution for this question is attached.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This is a Rust implementation contract; no native or appliance execution for this question is attached.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This is a Rust implementation contract; no native or appliance execution for this question is attached.

## Exact evidence

- `jim-static-lookup-source-0` (source-anchor): [rust/tcl-compiler/tests/data/native_callable_static_source/jim-statics.json](../../../../rust/tcl-compiler/tests/data/native_callable_static_source/jim-statics.json). SHA-256 `088b41b3abb7ec1bb8e04f258bc17c6e8286a89f9aa030acdb5b29b673b4d172`. JSON pointer `/source_anchors/0/snippet`. Exact pinned JimCreateProcedureStatics snippet; source inspection only.
- `jim-static-lookup-source-1` (source-anchor): [rust/tcl-compiler/tests/data/native_callable_static_source/jim-statics.json](../../../../rust/tcl-compiler/tests/data/native_callable_static_source/jim-statics.json). SHA-256 `088b41b3abb7ec1bb8e04f258bc17c6e8286a89f9aa030acdb5b29b673b4d172`. JSON pointer `/source_anchors/1/snippet`. Exact pinned SetVariableFromAny snippet; source inspection only.
- `jim-static-lookup-source-2` (source-anchor): [rust/tcl-compiler/tests/data/native_callable_static_source/jim-statics.json](../../../../rust/tcl-compiler/tests/data/native_callable_static_source/jim-statics.json). SHA-256 `088b41b3abb7ec1bb8e04f258bc17c6e8286a89f9aa030acdb5b29b673b4d172`. JSON pointer `/source_anchors/2/snippet`. Exact pinned Jim_GetVariable snippet; source inspection only.

## Source inspection

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `jim.c`, function `JimCreateProcedureStatics`, lines 4211–4310. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `a9c9c664c37ca9e060c887aef048d5d3ceffd094181073a79eef24635e24ba10`; retained evidence `jim-static-lookup-source-0`.

```text
/* Create the named static variables declared for a proc body. */
static int JimCreateProcedureStatics(Jim_Interp *interp, Jim_Cmd *cmdPtr, Jim_Obj *staticsListObjPtr)
{
    int len, i;

    len = Jim_ListLength(interp, staticsListObjPtr);
    if (len == 0) {
        return JIM_OK;
    }

    cmdPtr->u.proc.staticVars = Jim_Alloc(sizeof(Jim_HashTable));
    Jim_InitHashTable(cmdPtr->u.proc.staticVars, &JimVariablesHashTableType, interp);
    for (i = 0; i < len; i++) {
        Jim_Obj *initObjPtr = NULL;
        Jim_Obj *nameObjPtr;
        Jim_VarVal *vv = NULL;
        Jim_Obj *objPtr = Jim_ListGetIndex(interp, staticsListObjPtr, i);
        int subLen = Jim_ListLength(interp, objPtr);
        int byref = 0;

        /* Check if it's composed of two elements. */
        if (subLen != 1 && subLen != 2) {
            Jim_SetResultFormatted(interp, "too many fields in static specifier \"%#s\"",
                objPtr);
            return JIM_ERR;
        }

        nameObjPtr = Jim_ListGetIndex(interp, objPtr, 0);

        /* How to intialise or link? */
        if (subLen == 1) {
            int len;
            const char *pt = Jim_GetString(nameObjPtr, &len);
            if (*pt == '&') {
                /* Create as a reference */
                nameObjPtr = Jim_NewStringObj(interp, pt + 1, len - 1);
                byref = 1;
            }
        }
        Jim_IncrRefCount(nameObjPtr);

        if (subLen == 1) {
            switch (SetVariableFromAny(interp, nameObjPtr)) {
                case JIM_DICT_SUGAR:
                    /* XXX: This message seem unnecessarily verbose, but it matches Tcl */
                    if (byref) {
                        Jim_SetResultFormatted(interp, "Can't link to array element \"%#s\"", nameObjPtr);
                    }
                    else {
                        Jim_SetResultFormatted(interp, "Can't initialise array element \"%#s\"", nameObjPtr);
                    }
                    Jim_DecrRefCount(interp, nameObjPtr);
                    return JIM_ERR;

                case JIM_OK:
                    if (byref) {
                        vv = nameObjPtr->internalRep.varValue.vv;
                    }
                    else {
                        initObjPtr = Jim_GetVariable(interp, nameObjPtr, JIM_NONE);
                        if (!initObjPtr) {
                            Jim_SetResultFormatted(interp, "Could not resolve upvar \"%#s\"'s value", nameObjPtr);
                            Jim_DecrRefCount(interp, nameObjPtr);
                            return JIM_ERR;
                        }
                    }
                    break;

                case JIM_ERR:
                    /* Doesn't exist */
                    Jim_SetResultFormatted(interp,
                        "variable for initialization of static \"%#s\" not found in the local context",
                        nameObjPtr);
                    Jim_DecrRefCount(interp, nameObjPtr);
                    return JIM_ERR;
            }
        }
        else {
            initObjPtr = Jim_ListGetIndex(interp, objPtr, 1);
        }

        if (vv == NULL) {
            vv = Jim_Alloc(sizeof(*vv));
            vv->objPtr = initObjPtr;
            Jim_IncrRefCount(vv->objPtr);
            vv->linkFramePtr = NULL;
            vv->refCount = 0;
        }

        if (JimSetNewVariable(cmdPtr->u.proc.staticVars, nameObjPtr, vv) != JIM_OK) {
            Jim_SetResultFormatted(interp,
                "static variable name \"%#s\" duplicated in statics list", nameObjPtr);
            JimIncrVarRef(vv);
            JimDecrVarRef(interp, vv);
            Jim_DecrRefCount(interp, nameObjPtr);
            return JIM_ERR;
        }

        Jim_DecrRefCount(interp, nameObjPtr);
    }

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `jim.c`, function `SetVariableFromAny`, lines 4620–4689. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `0535e59eed4cc5585c36111e7b595effdc66168d9721f138f859d26a2c6cb1d6`; retained evidence `jim-static-lookup-source-1`.

```text
 * JIM_ERR if it does not exist, JIM_DICT_SUGAR if it's not
 * a variable name, but syntax glue for [dict] i.e. the last
 * character is ')' */
static int SetVariableFromAny(Jim_Interp *interp, struct Jim_Obj *objPtr)
{
    const char *varName;
    Jim_CallFrame *framePtr;
    int global;
    int len;
    Jim_VarVal *vv;

    /* Check if the object is already an uptodate variable */
    if (objPtr->typePtr == &variableObjType) {
        framePtr = objPtr->internalRep.varValue.global ? interp->topFramePtr : interp->framePtr;
        if (objPtr->internalRep.varValue.callFrameId == framePtr->id) {
            /* nothing to do */
            return JIM_OK;
        }
        /* Need to re-resolve the variable in the updated callframe */
    }
    else if (objPtr->typePtr == &dictSubstObjType) {
        return JIM_DICT_SUGAR;
    }

    varName = Jim_GetString(objPtr, &len);

    /* Make sure it's not syntax glue to get/set dict. */
    if (len && varName[len - 1] == ')' && strchr(varName, '(') != NULL) {
        return JIM_DICT_SUGAR;
    }

    if (varName[0] == ':' && varName[1] == ':') {
        while (*varName == ':') {
            varName++;
            len--;
        }
        global = 1;
        framePtr = interp->topFramePtr;
        /* XXX should use length */
        Jim_Obj *tempObj = Jim_NewStringObj(interp, varName, len);
        vv = JimFindVariable(&framePtr->vars, tempObj);
        Jim_FreeNewObj(interp, tempObj);
    }
    else {
        global = 0;
        framePtr = interp->framePtr;
        /* Resolve this name in the variables hash table */
        vv = JimFindVariable(&framePtr->vars, objPtr);
        if (vv == NULL && framePtr->staticVars) {
            /* Try with static vars. */
            vv = JimFindVariable(framePtr->staticVars, objPtr);
        }
    }

    if (vv == NULL) {
        return JIM_ERR;
    }

    /* Free the old internal repr and set the new one. */
    Jim_FreeIntRep(interp, objPtr);
    objPtr->typePtr = &variableObjType;
    objPtr->internalRep.varValue.callFrameId = framePtr->id;
    objPtr->internalRep.varValue.vv = vv;
    objPtr->internalRep.varValue.global = global;
    return JIM_OK;
}

/* -------------------- Variables related functions ------------------------- */
static int JimDictSugarSet(Jim_Interp *interp, Jim_Obj *ObjPtr, Jim_Obj *valObjPtr);
static Jim_Obj *JimDictSugarGet(Jim_Interp *interp, Jim_Obj *objPtr, int flags);

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `jim.c`, function `Jim_GetVariable`, lines 4933–4975. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `df8d47c3f0c411b74c7801f8b1b27b413f666af46f97bbdd5e0f2991ea680f5f`; retained evidence `jim-static-lookup-source-2`.

```text
 * in a dictionary which is shared, the array variable value is duplicated first.
 * This allows the array element to be updated (e.g. append, lappend) without
 * affecting other references to the dictionary.
 */
Jim_Obj *Jim_GetVariable(Jim_Interp *interp, Jim_Obj *nameObjPtr, int flags)
{
    if (interp->safeexpr) {
        return nameObjPtr;
    }
    switch (SetVariableFromAny(interp, nameObjPtr)) {
        case JIM_OK:{
                Jim_VarVal *vv = nameObjPtr->internalRep.varValue.vv;

                if (vv->linkFramePtr == NULL) {
                    return vv->objPtr;
                }
                else {
                    Jim_Obj *objPtr;

                    /* The variable is a link? Resolve it. */
                    Jim_CallFrame *savedCallFrame = interp->framePtr;

                    interp->framePtr = vv->linkFramePtr;
                    objPtr = Jim_GetVariable(interp, vv->objPtr, flags);
                    interp->framePtr = savedCallFrame;
                    if (objPtr) {
                        return objPtr;
                    }
                    /* Error, so fall through to the error message */
                }
            }
            break;

        case JIM_DICT_SUGAR:
            /* [dict] syntax sugar. */
            return JimDictSugarGet(interp, nameObjPtr, flags);
    }
    if (flags & JIM_ERRMSG) {
        Jim_SetResultFormatted(interp, "can't read \"%#s\": no such variable", nameObjPtr);
    }
    return NULL;
}


```


## Consumer bindings

- [rust/tcl-compiler/src/raw_binding.rs](../../../../rust/tcl-compiler/src/raw_binding.rs), `ResolveContext::capture_original_callable_statics`: Authenticate original list children and publish the complete counted capture atomically under the actual naming/cell context.
- [rust/tcl-compiler/src/raw_binding.rs](../../../../rust/tcl-compiler/src/raw_binding.rs), `RawBindingContents`: SelectedName retains a counted logical-level target name independently of its current resolved value or source label.
- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `prepare_source_callable_statics`: Consume definition.statics_at through the exact effective original input, current publication allocation and independent capture error boundary.
- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `parse_static_variables_bytes`: One selected native static-list grammar supplies literal, copy and reference roles without cell or completion authority.
- [rust/tcl-compiler/src/variable_bindings.rs](../../../../rust/tcl-compiler/src/variable_bindings.rs), `detach_retained_destructions`: Use the exact original Jim named slot for pinned-wrapper detachment, keeping static fallback and linked-target destruction separate.
- [rust/tcl-compiler/src/var_resolve/original_bytes.rs](../../../../rust/tcl-compiler/src/var_resolve/original_bytes.rs), `exact_root`: Preserve a receiver already selected by the current raw-wrapper or alias owner instead of rebinding it to the requesting activation; lazy OO candidates apply only to the actual direct current slot.
- [rust/tcl-compiler/src/raw_binding.rs](../../../../rust/tcl-compiler/src/raw_binding.rs), `raw_binding::original_callable_static_tests::original_static_literals_keep_counted_names_and_opaque_value_children` (linked): Two raw-zero names retain distinct static slots and opaque literal child bytes; later exact writes and frame restoration preserve the other slot.
- [rust/tcl-compiler/src/raw_binding.rs](../../../../rust/tcl-compiler/src/raw_binding.rs), `raw_binding::original_callable_static_tests::original_static_copy_and_reference_have_distinct_content_lifetimes` (linked): A copied static keeps OLD after the named source changes, while a retained reference sees NEW and later retargeting to another counted name. The retargeted read retains the exact maker target CellIdentity across the new call frame.
- [rust/tcl-compiler/src/raw_binding.rs](../../../../rust/tcl-compiler/src/raw_binding.rs), `raw_binding::original_callable_static_tests::original_reference_capture_owns_wrapper_presence_before_link_target_contents` (linked): An installed dangling name-link can be retained by reference before its target exists; copying its missing target fails without publication. Later resolution retains the exact original target CellIdentity across the called frame.
- [rust/tcl-compiler/src/raw_binding.rs](../../../../rust/tcl-compiler/src/raw_binding.rs), `raw_binding::original_callable_static_tests::original_static_capture_withdraws_atomically_for_unowned_or_observed_inputs` (linked): Malformed later declarations, unknown read observers, stale evaluated input and conflicting policies leave the entire prior state intact.
- [rust/tcl-compiler/src/raw_binding.rs](../../../../rust/tcl-compiler/src/raw_binding.rs), `raw_binding::original_callable_static_tests::original_static_unset_preserves_fallback_and_detaches_only_the_named_wrapper` (linked): An original counted Jim unset detaches only the pinned named entry, preserves the held OLD value after NEW recreation, and leaves direct callable-static fallback KEEP intact under both write projection and source transfer.

A named test is a coverage binding, not a claim that it executed.

## Replay

The linked Rust selectors exercise model inputs and lifetime distinctions; no Rust execution receipt is attached. Source windows can be checked against the retained pinned full-file digest. Native reference/copy admission is explained by inspected code, not asserted as a newly measured subprocess result.
