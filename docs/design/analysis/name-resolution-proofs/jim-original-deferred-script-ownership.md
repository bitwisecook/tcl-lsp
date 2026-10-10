# naming.source.jim-original-deferred-script-ownership

Kind: `implementation-contract`

## Problem statement

A fresh eval/uplevel concat can lose its genuine original script owner while a pending execution request retains only a lifetime view. Preparation clones and equal script bytes cannot establish that object remains live through actual Script activation.

## Question

How does a deferred Jim Script activation retain the exact worker-produced original script owner, consume it once at reached entry and release it at exit without extending authority through dormant preparation clones?

## Conclusion

The pinned Jim_EvalCoreCommand and Jim_UplevelCoreCommand pass a single original argv script directly, or their fresh Jim_ConcatObj result, to Jim_EvalObj. On the ordinary Script branch, Jim_EvalObj increases the original script reference before JimGetScript; its parse-error path releases that reference and its final Script restoration releases it after popping the evaluation frame. The VM NativeJimScriptEntry retains one actual original Value in a shared pending capsule. CompiledUnit::with_deferred_script_original accepts only the same live original object; pending eval/uplevel requests carry that owner into NativeJimScriptState::new, which consumes it once for actual activation. Transport clones share the capsule and donate no extra execution reference; dormant views cannot extend Native object life after exit. Foreign same-byte objects and retired headers refuse. Three linked software definitions cover these ownership obligations, including independent eval/uplevel controls; no completed assertion result is attached.

The independently observed public current-Jim code/result and helper declaration are retained in [mathop-original-jim-source-and-helper-context.md](mathop-original-jim-source-and-helper-context.md). Their successful processes answer a separate public question.

## Scope

Four exact Jim source excerpts are interpretations of the retained revision 0.84-9-g5bac7c9, not executed native refcount/header or frame observations. The specialised original List/no-String branch, C preparation and Jim Subst acquisition order are separate purposes. The whole failed Source288 software diagnostic records Value::drop under eval_original_uplevel before later NativeJimScriptState::new retired access; it remains 0 passed, 1 failed and predates these source controls. Failed Source286 is independently archived under its own exact ELF. Neither failed diagnostic nor equal bytes proves original Jim private state, Native handler/Normal/body/frame authority, current command table or arbitrary clone/replay permission. The 24 original public Jim mathop/helper results435 remain an independent measured question. All seven external provider answers remain not tested for this deferred ownership contract.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No external input for this deferred ownership question. Dialect: tcl8.4.

No original external provider process or source comparison answers this Jim deferred-script ownership purpose.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No external input for this deferred ownership question. Dialect: tcl8.5.

No original external provider process or source comparison answers this Jim deferred-script ownership purpose.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No external input for this deferred ownership question. Dialect: tcl8.6.

No original external provider process or source comparison answers this Jim deferred-script ownership purpose.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No external input for this deferred ownership question. Dialect: tcl9.0.

No original external provider process or source comparison answers this Jim deferred-script ownership purpose.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No external input for this deferred ownership question. Dialect: tcl9.1.

No original external provider process or source comparison answers this Jim deferred-script ownership purpose.

### jim

Status: `not-tested`. Version: 0.84-9-g5bac7c9. Build: Pinned source interpretation only; no provider process for deferred ownership. Channel: No external input for this deferred ownership question. Dialect: jim.

No original external provider process answers this software deferred-script ownership question. Exact Jim source excerpts explain the ordinary Script reference boundary; they are not private header/refcount or successful native execution observations.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No external input for this deferred ownership question. Dialect: bigip.

No original external provider process or source comparison answers this Jim deferred-script ownership purpose.

## Exact evidence

- `jim-owner-anchor-index` (source-anchor): [rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/source-anchors/manifest.json](../../../../rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/source-anchors/manifest.json). SHA-256 `5e5332ff65e334a5c149e6fed8bb02c8a9bab905bdf1588800a14e8c95ca3503`. Exact immutable source-window index and retained source-version recipe; source inspection, not a process or private object observation.
- `jim-owner-source-0` (source-anchor): [rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/source-anchors/jim-evalobj-entry.c](../../../../rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/source-anchors/jim-evalobj-entry.c). SHA-256 `870fa0a2c577e0008525a845ff4b1a8f77428ca3a0e73dd99e11160a7dad2c92`. Exact counted LF source excerpt from the pinned whole jim.c; retained lines and snippet bytes are independently checked. No native header/refcount/process observation.
- `jim-owner-source-1` (source-anchor): [rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/source-anchors/jim-evalobj-release.c](../../../../rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/source-anchors/jim-evalobj-release.c). SHA-256 `68ee2734c0b5c05849b275dee752c9f5730ef239c7990221bbffbb24d45d7ecd`. Exact counted LF source excerpt from the pinned whole jim.c; retained lines and snippet bytes are independently checked. No native header/refcount/process observation.
- `jim-owner-source-2` (source-anchor): [rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/source-anchors/jim-eval-core-command.c](../../../../rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/source-anchors/jim-eval-core-command.c). SHA-256 `44bc2ffd0806d75f49e784f78ceeff4e992ee933ad81858c46cf402c795f6e3f`. Exact counted LF source excerpt from the pinned whole jim.c; retained lines and snippet bytes are independently checked. No native header/refcount/process observation.
- `jim-owner-source-3` (source-anchor): [rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/source-anchors/jim-uplevel-core-command.c](../../../../rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/source-anchors/jim-uplevel-core-command.c). SHA-256 `879080829a0cfd71eb731eee1b5a3490bcb2dd46752c70136fa3c56efbbce05a`. Exact counted LF source excerpt from the pinned whole jim.c; retained lines and snippet bytes are independently checked. No native header/refcount/process observation.
- `jim-owner-whole-source` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim/jim.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim/jim.c). SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`. Existing exact retained whole Jim source shared without mutation; source interpretation is distinct from original public completions from experiment435.
- `jim-owner-version-recipe` (source-anchor): [rust/tcl-registry/tests/data/native_mathop_jim_original_source/request.json](../../../../rust/tcl-registry/tests/data/native_mathop_jim_original_source/request.json). SHA-256 `8228eae6950ab50c2ee30c2789e055dc6755ec8fe8f610bff88887b2d8534c19`. Existing unchanged original current-Jim source/helper request ties the retained source revision/version recipe. Its 24 public results remain an independent question and confer no private lifecycle outcome.
- `jim-owner-failed-software-receipt` (limitation): [rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/diagnostics/integration-vm-jim-retirement-diagnostic288/receipt.json.gz](../../../../rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/diagnostics/integration-vm-jim-retirement-diagnostic288/receipt.json.gz). SHA-256 `10487ddbb755c42fc59ef782a3e433aa4d43eb3d7ea73a57dceebbd9904037f3`. Unchanged failed Source288 VM software diagnostic receipt, compressed losslessly. No new Native process or current418 assertion result; exact original receipt SHA256 9f69d8f355e2ed5abb58c09f3d8256ac06b3e1641988dc11b367a20f3d223583.
- `jim-owner-failed-software-stack` (limitation): [rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/diagnostics/integration-vm-jim-retirement-diagnostic288/tests.log](../../../../rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/diagnostics/integration-vm-jim-retirement-diagnostic288/tests.log). SHA-256 `dba2bc74121f661596e2fb2920c73aab881990c21f97667c80a340ffb9d4495d`. Whole original failed Source288 software stack: Value::drop in eval_original_uplevel releases before NativeJimScriptState::new accesses a retired header. 0 passed, 1 failed, 751 filtered; no native-provider or 418 success inference.

## Source inspection

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_EvalObj`, lines 11554–11580. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `870fa0a2c577e0008525a845ff4b1a8f77428ca3a0e73dd99e11160a7dad2c92`; retained evidence `jim-owner-source-0`.

```text
int Jim_EvalObj(Jim_Interp *interp, Jim_Obj *scriptObjPtr)
{
    int i;
    ScriptObj *script;
    ScriptToken *token;
    int retcode = JIM_OK;
    Jim_Obj *sargv[JIM_EVAL_SARGV_LEN], **argv = NULL;
    Jim_EvalFrame frame;

    /* If the object is of type "list", with no string rep we can call
     * a specialized version of Jim_EvalObj() */
    if (Jim_IsList(scriptObjPtr) && scriptObjPtr->bytes == NULL) {
        return JimEvalObjList(interp, scriptObjPtr);
    }

    Jim_IncrRefCount(scriptObjPtr);     /* Make sure it's shared. */
    script = JimGetScript(interp, scriptObjPtr);
    if (JimParseCheckMissing(interp, script->missing) == JIM_ERR) {
        JimSetErrorStack(interp, script);
        Jim_DecrRefCount(interp, scriptObjPtr);
        return JIM_ERR;
    }

    /* Reset the interpreter result. This is useful to
     * return the empty result in the case of empty program. */
    Jim_SetEmptyResult(interp);


```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_EvalObj`, lines 11780–11796. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `68ee2734c0b5c05849b275dee752c9f5730ef239c7990221bbffbb24d45d7ecd`; retained evidence `jim-owner-source-1`.

```text
    /* Possibly add to the error stack trace */
    if (retcode == JIM_ERR) {
        JimSetErrorStack(interp, NULL);
    }

    JimPopEvalFrame(interp);

    /* Note that we don't have to decrement inUse, because the
     * following code transfers our use of the reference again to
     * the script object. */
    Jim_FreeIntRep(interp, scriptObjPtr);
    scriptObjPtr->typePtr = &scriptObjType;
    Jim_SetIntRepPtr(scriptObjPtr, script);
    Jim_DecrRefCount(interp, scriptObjPtr);

    return retcode;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_EvalCoreCommand`, lines 14202–14214. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `44bc2ffd0806d75f49e784f78ceeff4e992ee933ad81858c46cf402c795f6e3f`; retained evidence `jim-owner-source-2`.

```text
static int Jim_EvalCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int rc;

    if (argc == 2) {
        rc = Jim_EvalObj(interp, argv[1]);
    }
    else {
        rc = Jim_EvalObj(interp, Jim_ConcatObj(interp, argc - 1, argv + 1));
    }

    return rc;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_UplevelCoreCommand`, lines 14217–14252. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `879080829a0cfd71eb731eee1b5a3490bcb2dd46752c70136fa3c56efbbce05a`; retained evidence `jim-owner-source-3`.

```text
static int Jim_UplevelCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int retcode;
    Jim_CallFrame *savedCallFrame, *targetCallFrame;
    const char *str;

    /* Save the old callframe pointer */
    savedCallFrame = interp->framePtr;

    /* Lookup the target frame pointer */
    str = Jim_String(argv[1]);
    if ((str[0] >= '0' && str[0] <= '9') || str[0] == '#') {
        targetCallFrame = Jim_GetCallFrameByLevel(interp, argv[1]);
        argc--;
        argv++;
    }
    else {
        targetCallFrame = Jim_GetCallFrameByLevel(interp, NULL);
    }
    if (targetCallFrame == NULL) {
        return JIM_ERR;
    }
    if (argc < 2) {
        return JIM_USAGE;
    }
    /* Eval the code in the target callframe. */
    interp->framePtr = targetCallFrame;
    if (argc == 2) {
        retcode = Jim_EvalObj(interp, argv[1]);
    }
    else {
        retcode = Jim_EvalObj(interp, Jim_ConcatObj(interp, argc - 1, argv + 1));
    }
    interp->framePtr = savedCallFrame;
    return retcode;
}

```


## Consumer bindings

- [rust/tcl-vm/src/compiled.rs](../../../../rust/tcl-vm/src/compiled.rs), `CompiledUnit::with_deferred_script_original`: Move the actual same live worker-produced script into the pending Jim entry; this transport creates no distinct object or compiled-body permission.
- [rust/tcl-vm/src/native_jim_script.rs](../../../../rust/tcl-vm/src/native_jim_script.rs), `NativeJimScriptEntry::retain_activation_original`: Share one pending actual original owner across preparation transports; refuse foreign/retired/mismatched or duplicate owner rather than converting equal text.
- [rust/tcl-vm/src/native_jim_script.rs](../../../../rust/tcl-vm/src/native_jim_script.rs), `NativeJimScriptState::new`: Take the retained actual owner once before ordinary Script preparation; activation leases/empty-parent release remain separate from dormant preparation views and Subst order.
- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `cmd_eval`: Retain the actual eval worker-produced script through its pending Jim execution request.
- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `eval_original_uplevel`: Retain the actual uplevel worker-produced script through pending activation, independently of target-frame selection/admission.
- [rust/tcl-vm/src/value_script_execution_tests.rs](../../../../rust/tcl-vm/src/value_script_execution_tests.rs), `value::script::execution_tests::deferred_original_jim_script_owner_survives_transport_clones_only_until_activation_exit` (linked): One pending original reference survives transport cloning and is consumed by actual activation; successful software completion is an authored assertion, while exit retires the original and a dormant clone retains no execution reference. No completed run or native pointer observation is attached.
- [rust/tcl-vm/src/value_script_execution_tests.rs](../../../../rust/tcl-vm/src/value_script_execution_tests.rs), `value::script::execution_tests::deferred_original_jim_script_owner_refuses_foreign_and_retired_original_headers` (linked): A foreign same-byte script or retired lifetime view cannot supply the actual pending original owner; the independently live original remains unchanged. Software refusal control only.
- [rust/tcl-vm/src/value_script_execution_tests.rs](../../../../rust/tcl-vm/src/value_script_execution_tests.rs), `value::script::execution_tests::deferred_original_jim_eval_and_uplevel_keep_their_worker_produced_script` (linked): Actual software eval and uplevel workers retain their own fresh original script through deferred entry and compare authored result bytes. No original-provider completion or arbitrary frame/handler authority is inferred.

A named test is a coverage binding, not a claim that it executed.

## Replay

Source/API definitions and exact source interpretation only. Original Source288 diagnostic is a failed independent software run, not a replay command or 418 assertion pass. No new native or Rust execution is supplied.
