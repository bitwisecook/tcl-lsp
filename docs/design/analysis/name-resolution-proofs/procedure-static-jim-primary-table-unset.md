# naming.procedure-static.jim-primary-table-unset

Kind: `implementation-contract`

## Problem statement

A static fallback cell is visible to Jim variable lookup but is not an entry in the current activation primary table. Plain unset needs the primary-table removal purpose independently of successful fallback lookup.

## Question

Does original Jim non-link unset remove only the actual selected primary-table entry while preserving static fallback and independent linked/dictionary targets?

## Conclusion

The Runtime plain unset path consumes the current selected Jim name policy and primary-table owner. Static-only names fail without mutating their retained cell; successful primary removal updates the actual selected frame incarnation. Linked original target recursion and dictionary-member removal remain independent.

## Scope

Selected Jim084 Runtime plain non-link variable removal. No C Tcl removal, static-copy/reference creation, dictionary member ownership, observers or arbitrary cache authority is inferred.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No guest execution of this Rust implementation question is claimed.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No guest execution of this Rust implementation question is claimed.

## Exact evidence

- `jim-source` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim/jim.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim/jim.c). SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`. Pinned source already associated with the independently retained Info capture; those observations do not measure static unset.
- `unset-windows` (source-anchor): [rust/tcl-registry/tests/data/native_procedure_static_original/jim-primary-unset-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/jim-primary-unset-source-anchors.json). SHA-256 `96fec322d5375fee8e5b5ae37cc3964a2039c1e4946f871df662031b3150e686`. Exact LF source windows, separate from execution.
- `historical-source` (input): [rust/tcl-syntax/tests/data/body_execution/native-static-storage.tcl](../../../../rust/tcl-syntax/tests/data/body_execution/native-static-storage.tcl). SHA-256 `0575fe1f1d9006c7f3fc6e648d5cb6cadf0868ab7e3573af50051f2406557f52`. Retained finite source input; no complete native process receipt is attached.
- `historical-result` (limitation): [rust/tcl-syntax/tests/data/body_execution/native-static-storage-jim.txt](../../../../rust/tcl-syntax/tests/data/body_execution/native-static-storage-jim.txt). SHA-256 `4bed326918283bff9de9c0ef6d9861cab113335572c3cbbf2cc709618b455341`. Retained finite output includes unsetstatic error; absent full process provenance prevents an interpreter-outcome claim.
- `unset-window-0` (source-anchor): [rust/tcl-registry/tests/data/native_procedure_static_original/jim-primary-unset-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/jim-primary-unset-source-anchors.json). SHA-256 `96fec322d5375fee8e5b5ae37cc3964a2039c1e4946f871df662031b3150e686`. JSON pointer `/source_anchors/0/snippet`. Exact LF source window for SetVariableFromAny; no native execution claim.
- `unset-window-1` (source-anchor): [rust/tcl-registry/tests/data/native_procedure_static_original/jim-primary-unset-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/jim-primary-unset-source-anchors.json). SHA-256 `96fec322d5375fee8e5b5ae37cc3964a2039c1e4946f871df662031b3150e686`. JSON pointer `/source_anchors/1/snippet`. Exact LF source window for Jim_UnsetVariable; no native execution claim.

## Source inspection

jim source version not independently recorded, revision `Source revision unrecorded; exact retained full-file SHA identifies the inspected bytes.`, `rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim/jim.c`, function `SetVariableFromAny`, lines 4623–4686. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `3408b0521d97ced6a6b6bcb0cf0cbe7d01a722c2a2a570e575f0f18a9107bf9b`; retained evidence `unset-window-0`.

```text
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


```

jim source version not independently recorded, revision `Source revision unrecorded; exact retained full-file SHA identifies the inspected bytes.`, `rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim/jim.c`, function `Jim_UnsetVariable`, lines 5017–5067. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `b727ed5e4c28b20bd009514e28d78794aa412149bfce0c68ec8eda70c8ad3158`; retained evidence `unset-window-1`.

```text
int Jim_UnsetVariable(Jim_Interp *interp, Jim_Obj *nameObjPtr, int flags)
{
    Jim_VarVal *vv;
    int retval;
    Jim_CallFrame *framePtr;

    retval = SetVariableFromAny(interp, nameObjPtr);
    if (retval == JIM_DICT_SUGAR) {
        /* [dict] syntax sugar. */
        return JimDictSugarSet(interp, nameObjPtr, NULL);
    }
    else if (retval == JIM_OK) {
        vv = nameObjPtr->internalRep.varValue.vv;

        /* If it's a link call UnsetVariable recursively */
        if (vv->linkFramePtr) {
            framePtr = interp->framePtr;
            interp->framePtr = vv->linkFramePtr;
            retval = Jim_UnsetVariable(interp, vv->objPtr, JIM_NONE);
            interp->framePtr = framePtr;
        }
        else {
            if (nameObjPtr->internalRep.varValue.global) {
                int len;
                const char *name = Jim_GetString(nameObjPtr, &len);
                while (*name == ':') {
                    name++;
                    len--;
                }
                framePtr = interp->topFramePtr;
                Jim_Obj *tempObj = Jim_NewStringObj(interp, name, len);
                retval = JimUnsetVariable(&framePtr->vars, tempObj);
                Jim_FreeNewObj(interp, tempObj);
            }
            else {
                framePtr = interp->framePtr;
                retval = JimUnsetVariable(&framePtr->vars, nameObjPtr);
            }

            if (retval == JIM_OK) {
                /* Change the callframe id, invalidating var lookup caching */
                framePtr->id = interp->callFrameEpoch++;
            }
        }
    }
    if (retval != JIM_OK && (flags & JIM_ERRMSG)) {
        Jim_SetResultFormatted(interp, "can't unset \"%#s\": no such variable", nameObjPtr);
    }
    return retval;
}


```


## Consumer bindings

- [runtime/rust/src/frame/jim_lookup.rs](../../../../runtime/rust/src/frame/jim_lookup.rs), `remove_jim_primary`: Actual primary variable-table membership before removal; static fallback is not primary membership.
- [runtime/rust/src/vars.rs](../../../../runtime/rust/src/vars.rs), `unset_jim_primary_variable`: Selected Jim policy, actual resolved home and successful frame-incarnation update.
- [runtime/rust/src/interp/native_jim_links.rs](../../../../runtime/rust/src/interp/native_jim_links.rs), `unset_original_jim_variable`: Original target recursion and dictionary sugar remain independent before plain primary removal.
- [runtime/rust/src/cmd_proc.rs](../../../../runtime/rust/src/cmd_proc.rs), `cmd_proc::tests::jim_static_cells_match_native_definition_and_retirement` (linked): Static fallback unset refuses without destroying retained storage, while the existing literal/copy/raw-link/formal/array-member and command-retirement cases retain their separate outcomes.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact linked Rust selector has no execution receipt in this question. Retained finite static source/output has no complete provider process provenance and cannot establish a native interpreter outcome.
