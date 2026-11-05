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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

No guest execution of this Rust implementation question is claimed.

## Source inspection

jim source version not independently recorded, revision `Source revision unrecorded; exact retained full-file SHA identifies the inspected bytes.`, `rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim/jim.c`, function `SetVariableFromAny`, lines 4623–4686. Retained reference `unset-window-0`.

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

jim source version not independently recorded, revision `Source revision unrecorded; exact retained full-file SHA identifies the inspected bytes.`, `rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim/jim.c`, function `Jim_UnsetVariable`, lines 5017–5067. Retained reference `unset-window-1`.

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

- [runtime/rust/src/vars/entered_container.rs](../../../../runtime/rust/src/vars/entered_container.rs), `EnteredVariablePlace::remove_jim_primary`: Use the captured original table/frame/root/link observation and operation currency. The existing primary-table predicate excludes static-only membership; removal uses that same captured owner and updates its actual frame incarnation.
- [runtime/rust/src/interp/native_jim_links.rs](../../../../runtime/rust/src/interp/native_jim_links.rs), `Interp::unset_original_jim_variable_entered`: Retain selected original target recursion and dictionary-member separation, capture the actual plain primary-table place before removal, preserve the first full cause and release the captured owner before the final original currency check.
- [runtime/rust/src/cmd_proc.rs](../../../../runtime/rust/src/cmd_proc.rs), `cmd_proc::tests::jim_static_cells_match_native_definition_and_retirement` (linked): Static fallback unset refuses without destroying retained storage, while the existing literal/copy/raw-link/formal/array-member and command-retirement cases retain their separate outcomes.

## Evidence

- [rust/tcl-registry/tests/data/native_procedure_static_original/jim-primary-unset-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/jim-primary-unset-source-anchors.json)
- [rust/tcl-syntax/tests/data/body_execution/native-static-storage.tcl](../../../../rust/tcl-syntax/tests/data/body_execution/native-static-storage.tcl)
- [rust/tcl-syntax/tests/data/body_execution/native-static-storage-jim.txt](../../../../rust/tcl-syntax/tests/data/body_execution/native-static-storage-jim.txt)
- [runtime/rust/src/vars/entered_container.rs](../../../../runtime/rust/src/vars/entered_container.rs)
- [runtime/rust/src/interp/native_jim_links.rs](../../../../runtime/rust/src/interp/native_jim_links.rs)
