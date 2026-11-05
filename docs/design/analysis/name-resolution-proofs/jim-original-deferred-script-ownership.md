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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, bigip

Status: `not-tested`. Channel: No external input for this deferred ownership question.

No original external provider process or source comparison answers this Jim deferred-script ownership purpose.

### jim

Status: `not-tested`. Channel: No external input for this deferred ownership question. Version: jim 0.84-9-g5bac7c9.

No original external provider process answers this software deferred-script ownership question. Exact Jim source excerpts explain the ordinary Script reference boundary; they are not private header/refcount or successful native execution observations.

## Source inspection

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_EvalObj`, lines 11554–11580. Retained reference `jim-owner-source-0`.

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

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_EvalObj`, lines 11780–11796. Retained reference `jim-owner-source-1`.

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

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_EvalCoreCommand`, lines 14202–14214. Retained reference `jim-owner-source-2`.

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

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_UplevelCoreCommand`, lines 14217–14252. Retained reference `jim-owner-source-3`.

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

## Evidence

- [rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/source-anchors/manifest.json](../../../../rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/source-anchors/manifest.json)
- [rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/source-anchors/jim-evalobj-entry.c](../../../../rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/source-anchors/jim-evalobj-entry.c)
- [rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/source-anchors/jim-evalobj-release.c](../../../../rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/source-anchors/jim-evalobj-release.c)
- [rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/source-anchors/jim-eval-core-command.c](../../../../rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/source-anchors/jim-eval-core-command.c)
- [rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/source-anchors/jim-uplevel-core-command.c](../../../../rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/source-anchors/jim-uplevel-core-command.c)
- [rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/diagnostics/integration-vm-jim-retirement-diagnostic288/tests.log](../../../../rust/tcl-vm/tests/data/jim_original_deferred_script_ownership/diagnostics/integration-vm-jim-retirement-diagnostic288/tests.log)
