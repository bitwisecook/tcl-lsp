# naming.procedure-static.jim-original-link-frame-storage

Kind: `implementation-contract`

## Problem statement

An original mutable Jim link can reach recycled frame storage without retaining the same activation. Storage reuse, live ownership and fresh activation epochs need independent checks.

## Question

How do both ports retain exact original Jim link-frame storage while keeping activation/source/cache identity and teardown order independent?

## Conclusion

Both actual Jim backends use the shared opaque LIFO storage pool and original target-name getter. Departed local-command/argument/body/namespace/variable owners release before recycling. Current activation owners and variable epochs remain independently fresh; C consumers do not use this pool.

## Scope

One actual interpreter storage pool, exact original link target and selected live matching frame. Inactive or foreign storage still supplies no receiver. No entered source or Native compilation/name/observer/Normal permission follows from storage reuse. Interpreter final teardown does not mint future call-frame authority.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

No native execution receipt for this separate storage-owner implementation contract; finite guest controls are the independent native question.

## Source inspection

jim 0.84-9-g5bac7c9, revision `Source revision not recorded by this frame probe.`, `rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim/jim.c`, function `Jim_GetVariable`, lines 4937–4974. Retained reference `eff22ac40e9a48ac6eeb4`.

```text
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

jim 0.84-9-g5bac7c9, revision `Source revision not recorded by this frame probe.`, `rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim/jim.c`, function `JimCreateCallFrame`, lines 5260–5292. Retained reference `ed7cf30ba937eadca0e2a`.

```text
static Jim_CallFrame *JimCreateCallFrame(Jim_Interp *interp, Jim_CallFrame *parent, Jim_Obj *nsObj)
{
    Jim_CallFrame *cf;

    if (interp->freeFramesList) {
        cf = interp->freeFramesList;
        interp->freeFramesList = cf->next;

        cf->argv = NULL;
        cf->argc = 0;
        cf->procArgsObjPtr = NULL;
        cf->procBodyObjPtr = NULL;
        cf->next = NULL;
        cf->staticVars = NULL;
        cf->localCommands = NULL;
        cf->tailcallObj = NULL;
        cf->tailcallCmd = NULL;
    }
    else {
        cf = Jim_Alloc(sizeof(*cf));
        memset(cf, 0, sizeof(*cf));

        Jim_InitHashTable(&cf->vars, &JimVariablesHashTableType, interp);
    }

    cf->id = interp->callFrameEpoch++;
    cf->parent = parent;
    cf->level = parent ? parent->level + 1 : 0;
    cf->nsObj = nsObj;
    Jim_IncrRefCount(nsObj);

    return cf;
}
```

jim 0.84-9-g5bac7c9, revision `Source revision not recorded by this frame probe.`, `rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim/jim.c`, function `JimFreeCallFrame`, lines 5387–5403. Retained reference `ee128a2f09a96f35b50dc`.

```text
static void JimFreeCallFrame(Jim_Interp *interp, Jim_CallFrame *cf, int action)
 {
    JimDeleteLocalProcs(interp, cf->localCommands);

    if (cf->procArgsObjPtr)
        Jim_DecrRefCount(interp, cf->procArgsObjPtr);
    if (cf->procBodyObjPtr)
        Jim_DecrRefCount(interp, cf->procBodyObjPtr);
    Jim_DecrRefCount(interp, cf->nsObj);
    if (action == JIM_FCF_FULL || cf->vars.size != JIM_HT_INITIAL_SIZE)
        Jim_FreeHashTable(&cf->vars);
    else {
        Jim_ClearHashTable(&cf->vars);
    }
    cf->next = interp->freeFramesList;
    interp->freeFramesList = cf;
}
```

## Consumer bindings

- [rust/tcl-runtime-api/src/jim_call_frame.rs](../../../../rust/tcl-runtime-api/src/jim_call_frame.rs), `JimCallFrameStoragePool::acquire`: Shared concrete storage selection; no activation/cache or native receipt donor.
- [rust/tcl-runtime-api/src/jim_call_frame.rs](../../../../rust/tcl-runtime-api/src/jim_call_frame.rs), `JimCallFrameStoragePool::recycle`: Shared storage publication after separately completed original owner release.
- [runtime/rust/src/cmd_proc/native_static_original_tests.rs](../../../../runtime/rust/src/cmd_proc/native_static_original_tests.rs), `cmd_proc::native_static_original_tests::original_jim_static_links_match_native_frame_storage_reuse_boundaries` (linked): Original six source/code/result rows per provider, with actual native backend installed; no host refusal is substituted for Jim guest error.
- [rust/tcl-vm/src/command/native_static_original_tests.rs](../../../../rust/tcl-vm/src/command/native_static_original_tests.rs), `command::native_static_original_tests::original_jim_static_links_match_native_frame_storage_reuse_boundaries` (linked): Original six source/code/result rows per provider, with actual native backend installed; no host refusal is substituted for Jim guest error.
- [rust/tcl-runtime-api/src/jim_call_frame.rs](../../../../rust/tcl-runtime-api/src/jim_call_frame.rs), `jim_call_frame::tests::original_jim_storage_reuse_keeps_live_and_foreign_slots_separate` (linked): Distinct active slots and foreign pools do not match; only retired LIFO storage can be reused, expired storage cannot match.

## Evidence

- [rust/tcl-registry/tests/data/native_jim_link_frame_storage/source-anchors.json](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/source-anchors.json)
