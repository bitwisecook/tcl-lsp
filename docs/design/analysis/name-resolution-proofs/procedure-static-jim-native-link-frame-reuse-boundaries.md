# naming.procedure-static.jim-native-link-frame-reuse-boundaries

Kind: `native-observation`

## Problem statement

An original Jim static captures the mutable link cell; retiring its initial frame does not by itself establish the result of later recursive target-name access.

## Question

What are the exact source return-code/result boundaries for active links, retired/self/nested frame-slot reuse, writes, scalar captures and global links on the six captured providers?

## Conclusion

Jim yields KEEP, missing x, OTHER, missing x, KEEP and CHANGED for the six finite cases. Each C provider rejects the optional statics argv with its observed procedure arity error. Exact source windows separately show retained linkFramePtr and LIFO free storage with independently fresh callFrameEpoch; result rows do not measure pointers or activation equality.

## Scope

Six exact original ASCII scripts per provider, each in a fresh native interpreter, 36 cases/108 protocol rows total; actual per-case info patchlevel queried. All six original programs compiled and exited 0 with empty stderr. No raw encoded names, reference counts, physical addresses/layouts, callback teardown completeness, native entry/CPP capability, string/cache/storage outcome or general Jim static completeness claim.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII NUL-terminated original whole-script Tcl_Eval/Jim_Eval; six fresh interpreters, completed result getter then ASCII hex output. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

ACTIVE_LINK: code 1, bytes b'wrong # args: should be "proc name args body"'; RETIRED_SLOT_SELF_REUSE: code 1, bytes b'wrong # args: should be "proc name args body"'; NESTED_SLOT_REUSE: code 1, bytes b'wrong # args: should be "proc name args body"'; RETIRED_WRITE_THEN_READ: code 1, bytes b'wrong # args: should be "proc name args body"'; SCALAR_CELL_REFERENCE: code 1, bytes b'wrong # args: should be "proc name args body"'; GLOBAL_LINK: code 1, bytes b'wrong # args: should be "proc name args body"'

### jim

Status: `observed`. Channel: ASCII NUL-terminated original whole-script Tcl_Eval/Jim_Eval; six fresh interpreters, completed result getter then ASCII hex output. Version: jim 0.84-9-g5bac7c9.

ACTIVE_LINK: code 0, bytes b'KEEP'; RETIRED_SLOT_SELF_REUSE: code 1, bytes b'can\'t read "x": no such variable'; NESTED_SLOT_REUSE: code 0, bytes b'OTHER'; RETIRED_WRITE_THEN_READ: code 1, bytes b'can\'t read "x": no such variable'; SCALAR_CELL_REFERENCE: code 0, bytes b'KEEP'; GLOBAL_LINK: code 0, bytes b'CHANGED'

### bigip

Status: `not-tested`.

No observed BIG-IP guest process for this question.

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

- [runtime/rust/src/cmd_proc/native_static_original_tests.rs](../../../../runtime/rust/src/cmd_proc/native_static_original_tests.rs), `cmd_proc::native_static_original_tests::original_jim_static_links_match_native_frame_storage_reuse_boundaries` (linked): Original six source/code/result rows per provider, with actual native backend installed; no host refusal is substituted for Jim guest error.
- [rust/tcl-vm/src/command/native_static_original_tests.rs](../../../../rust/tcl-vm/src/command/native_static_original_tests.rs), `command::native_static_original_tests::original_jim_static_links_match_native_frame_storage_reuse_boundaries` (linked): Original six source/code/result rows per provider, with actual native backend installed; no host refusal is substituted for Jim guest error.

## Evidence

- [rust/tcl-registry/tests/data/native_jim_link_frame_storage](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage)
- [rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.4.20](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.4.20)
- [rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.5.19](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.5.19)
- [rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.6.18](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.6.18)
- [rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.0.4](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.0.4)
- [rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.1.0](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.1.0)
- [rust/tcl-registry/tests/data/native_jim_link_frame_storage/jim](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/jim)
