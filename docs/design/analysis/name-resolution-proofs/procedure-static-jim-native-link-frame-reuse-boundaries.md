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

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Original provider linked by exact source/header/library/build/executable hashes in the retained receipts.. Channel: ASCII NUL-terminated original whole-script Tcl_Eval/Jim_Eval; six fresh interpreters, completed result getter then ASCII hex output.. Dialect: tcl8.4.

ACTIVE_LINK: code 1, bytes b'wrong # args: should be "proc name args body"'; RETIRED_SLOT_SELF_REUSE: code 1, bytes b'wrong # args: should be "proc name args body"'; NESTED_SLOT_REUSE: code 1, bytes b'wrong # args: should be "proc name args body"'; RETIRED_WRITE_THEN_READ: code 1, bytes b'wrong # args: should be "proc name args body"'; SCALAR_CELL_REFERENCE: code 1, bytes b'wrong # args: should be "proc name args body"'; GLOBAL_LINK: code 1, bytes b'wrong # args: should be "proc name args body"'

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Original provider linked by exact source/header/library/build/executable hashes in the retained receipts.. Channel: ASCII NUL-terminated original whole-script Tcl_Eval/Jim_Eval; six fresh interpreters, completed result getter then ASCII hex output.. Dialect: tcl8.5.

ACTIVE_LINK: code 1, bytes b'wrong # args: should be "proc name args body"'; RETIRED_SLOT_SELF_REUSE: code 1, bytes b'wrong # args: should be "proc name args body"'; NESTED_SLOT_REUSE: code 1, bytes b'wrong # args: should be "proc name args body"'; RETIRED_WRITE_THEN_READ: code 1, bytes b'wrong # args: should be "proc name args body"'; SCALAR_CELL_REFERENCE: code 1, bytes b'wrong # args: should be "proc name args body"'; GLOBAL_LINK: code 1, bytes b'wrong # args: should be "proc name args body"'

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Original provider linked by exact source/header/library/build/executable hashes in the retained receipts.. Channel: ASCII NUL-terminated original whole-script Tcl_Eval/Jim_Eval; six fresh interpreters, completed result getter then ASCII hex output.. Dialect: tcl8.6.

ACTIVE_LINK: code 1, bytes b'wrong # args: should be "proc name args body"'; RETIRED_SLOT_SELF_REUSE: code 1, bytes b'wrong # args: should be "proc name args body"'; NESTED_SLOT_REUSE: code 1, bytes b'wrong # args: should be "proc name args body"'; RETIRED_WRITE_THEN_READ: code 1, bytes b'wrong # args: should be "proc name args body"'; SCALAR_CELL_REFERENCE: code 1, bytes b'wrong # args: should be "proc name args body"'; GLOBAL_LINK: code 1, bytes b'wrong # args: should be "proc name args body"'

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original provider linked by exact source/header/library/build/executable hashes in the retained receipts.. Channel: ASCII NUL-terminated original whole-script Tcl_Eval/Jim_Eval; six fresh interpreters, completed result getter then ASCII hex output.. Dialect: tcl9.0.

ACTIVE_LINK: code 1, bytes b'wrong # args: should be "proc name args body"'; RETIRED_SLOT_SELF_REUSE: code 1, bytes b'wrong # args: should be "proc name args body"'; NESTED_SLOT_REUSE: code 1, bytes b'wrong # args: should be "proc name args body"'; RETIRED_WRITE_THEN_READ: code 1, bytes b'wrong # args: should be "proc name args body"'; SCALAR_CELL_REFERENCE: code 1, bytes b'wrong # args: should be "proc name args body"'; GLOBAL_LINK: code 1, bytes b'wrong # args: should be "proc name args body"'

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original provider linked by exact source/header/library/build/executable hashes in the retained receipts.. Channel: ASCII NUL-terminated original whole-script Tcl_Eval/Jim_Eval; six fresh interpreters, completed result getter then ASCII hex output.. Dialect: tcl9.1.

ACTIVE_LINK: code 1, bytes b'wrong # args: should be "proc name args body"'; RETIRED_SLOT_SELF_REUSE: code 1, bytes b'wrong # args: should be "proc name args body"'; NESTED_SLOT_REUSE: code 1, bytes b'wrong # args: should be "proc name args body"'; RETIRED_WRITE_THEN_READ: code 1, bytes b'wrong # args: should be "proc name args body"'; SCALAR_CELL_REFERENCE: code 1, bytes b'wrong # args: should be "proc name args body"'; GLOBAL_LINK: code 1, bytes b'wrong # args: should be "proc name args body"'

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Original provider linked by exact source/header/library/build/executable hashes in the retained receipts.. Channel: ASCII NUL-terminated original whole-script Tcl_Eval/Jim_Eval; six fresh interpreters, completed result getter then ASCII hex output.. Dialect: jim.

ACTIVE_LINK: code 0, bytes b'KEEP'; RETIRED_SLOT_SELF_REUSE: code 1, bytes b'can\'t read "x": no such variable'; NESTED_SLOT_REUSE: code 0, bytes b'OTHER'; RETIRED_WRITE_THEN_READ: code 1, bytes b'can\'t read "x": no such variable'; SCALAR_CELL_REFERENCE: code 0, bytes b'KEEP'; GLOBAL_LINK: code 0, bytes b'CHANGED'

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No observed BIG-IP guest process for this question.

## Historical references

- `eb95cf8aa2727d8f2a4fa` (input): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/probe.c](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/probe.c). Exact original ASCII native program; six fresh whole-script cases per provider.
- `eb2b16f682a5864a8d40f` (input): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/queue.json](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/queue.json). Original bounded source cases and no-pointer/no-activation limits.
- `eb76663fbccce0edbc43f` (input): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/inputs.json](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/inputs.json). Actual immutable input preparation and provider pins.
- `e00547b3fdcd5542e8efe` (input): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/capture.py](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/capture.py). Original fixed-path capture protocol; not a new native execution.
- `e85c1355435416bcb42b6` (provider): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/receipt.json). Aggregate original compile/process/SDK/probe/stream provenance.
- `ed145559997551b0406ea` (provider): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.4.20/receipt.json). Exact original provider compile/process/source/version/header/library/executable joins.
- `ed875389f9501a8b74efc` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.4.20/stdout.tsv). Six original VERSION/INPUT/result triples; full guest bytes only after source evaluation.
- `ee46331d22e6be8222e3b` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.4.20/stderr). Original empty process stderr.
- `e5222df488dc8a5745d36` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.4.20/compile.stdout](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.4.20/compile.stdout). Original compiler stdout.
- `eefd2a2e9e57a68536aa7` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.4.20/compile.stderr](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.4.20/compile.stderr). Original compiler stderr.
- `ef74a17e1e8610c123056` (provider): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.5.19/receipt.json). Exact original provider compile/process/source/version/header/library/executable joins.
- `e95bb8939969454f4065c` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.5.19/stdout.tsv). Six original VERSION/INPUT/result triples; full guest bytes only after source evaluation.
- `e5132d359ff216382221e` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.5.19/stderr). Original empty process stderr.
- `e5a392c0b302fa91415c0` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.5.19/compile.stdout](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.5.19/compile.stdout). Original compiler stdout.
- `ea951d23e7ab2fcd9600e` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.5.19/compile.stderr](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.5.19/compile.stderr). Original compiler stderr.
- `e90f229a73287bb57eb9b` (provider): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.6.18/receipt.json). Exact original provider compile/process/source/version/header/library/executable joins.
- `e7f3564b17e9c43a31f12` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.6.18/stdout.tsv). Six original VERSION/INPUT/result triples; full guest bytes only after source evaluation.
- `e01305f8b425353afe9d5` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.6.18/stderr). Original empty process stderr.
- `e417334d2dacf001c09a5` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.6.18/compile.stdout](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.6.18/compile.stdout). Original compiler stdout.
- `e9303ecb1245d4ea1d81e` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.6.18/compile.stderr](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.6.18/compile.stderr). Original compiler stderr.
- `e1b8708ee6fcce1339358` (provider): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.0.4/receipt.json). Exact original provider compile/process/source/version/header/library/executable joins.
- `ec2b03680062d672e51b0` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.0.4/stdout.tsv). Six original VERSION/INPUT/result triples; full guest bytes only after source evaluation.
- `e3abb03736de3734bff92` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.0.4/stderr). Original empty process stderr.
- `e4f5f3f35c746bed28c1e` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.0.4/compile.stdout](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.0.4/compile.stdout). Original compiler stdout.
- `ee557791c24a7c5870fcc` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.0.4/compile.stderr](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.0.4/compile.stderr). Original compiler stderr.
- `e6ea817ec8bc51b986519` (provider): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.1.0/receipt.json). Exact original provider compile/process/source/version/header/library/executable joins.
- `ea02d207fd2bf4aad757c` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.1.0/stdout.tsv). Six original VERSION/INPUT/result triples; full guest bytes only after source evaluation.
- `e87c465985c85f48f1bb2` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.1.0/stderr). Original empty process stderr.
- `e4d8f2ca77b3911262738` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.1.0/compile.stdout](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.1.0/compile.stdout). Original compiler stdout.
- `ea83fbaabfce780ca55a6` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.1.0/compile.stderr](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.1.0/compile.stderr). Original compiler stderr.
- `ecec11859d18142eb03bf` (provider): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/jim/receipt.json). Exact original provider compile/process/source/version/header/library/executable joins.
- `ecc10a83bfd184689c88e` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/jim/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/jim/stdout.tsv). Six original VERSION/INPUT/result triples; full guest bytes only after source evaluation.
- `e702f878174914f79ef0b` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/jim/stderr](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/jim/stderr). Original empty process stderr.
- `eb2af3996ab0a992854c3` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/jim/compile.stdout](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/jim/compile.stdout). Original compiler stdout.
- `eb80ee7557023e64cbe70` (observation): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/jim/compile.stderr](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/jim/compile.stderr). Original compiler stderr.
- `eff22ac40e9a48ac6eeb4` (source-anchor): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/source-anchors.json](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/source-anchors.json). JSON pointer `/source_anchors/0/snippet`. Exact pinned Jim_Obj *Jim_GetVariable LF function bytes; recursive getter/storage/free-order only.
- `ed7cf30ba937eadca0e2a` (source-anchor): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/source-anchors.json](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/source-anchors.json). JSON pointer `/source_anchors/1/snippet`. Exact pinned static Jim_CallFrame *JimCreateCallFrame LF function bytes; recursive getter/storage/free-order only.
- `ee128a2f09a96f35b50dc` (source-anchor): [rust/tcl-registry/tests/data/native_jim_link_frame_storage/source-anchors.json](../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/source-anchors.json). JSON pointer `/source_anchors/2/snippet`. Exact pinned static void JimFreeCallFrame LF function bytes; recursive getter/storage/free-order only.

## Source inspection

jim 0.84-9-g5bac7c9, revision `Source revision not recorded by this frame probe; exact full source SHA-256 retained.`, `rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim/jim.c`, function `Jim_GetVariable`, lines 4937–4974. Retained reference `eff22ac40e9a48ac6eeb4`.

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

jim 0.84-9-g5bac7c9, revision `Source revision not recorded by this frame probe; exact full source SHA-256 retained.`, `rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim/jim.c`, function `JimCreateCallFrame`, lines 5260–5292. Retained reference `ed7cf30ba937eadca0e2a`.

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

jim 0.84-9-g5bac7c9, revision `Source revision not recorded by this frame probe; exact full source SHA-256 retained.`, `rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim/jim.c`, function `JimFreeCallFrame`, lines 5387–5403. Retained reference `ee128a2f09a96f35b50dc`.

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

A named test is a coverage binding, not a claim that it executed.

## Replay

No launch or Rust pass inferred by attaching the existing receipts. C rows are unsupported optional-statics arity controls only. Original capture.py retains its fixed capture-machine paths. The exact original runner and inputs remain archived; replay requires independently verified SDK/executable/source pins and fresh output paths.
