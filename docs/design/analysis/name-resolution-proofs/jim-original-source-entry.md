# naming.source.jim-original-source-entry

Kind: `source-anchor`

## Problem statement

A sourced Jim script evaluated through a String-only tree-walk loses its original Source token filename and starting line, so a nested procedure error can report line1 instead of its actual declaration line. Rebuilding diagnostic filenames as reporting strings also discards the original filename object.

## Question

How does the pinned Jim source entry retain filename/line through original Script tokens and automatic error-stack fields, and what happens to an unlocated original object?

## Conclusion

Jim_EvalSource installs Source metadata before evaluating its fresh original C-string script object; Jim_EvalFile appends counted file-read buffers to an original object then installs Source filename/line1. JimSetScriptFromAny reads that original metadata and its parser emits tokens that retain the original filename and token lines. Ordinary objects with no Source/Script primary use empty filename/line1. Automatic error-stack list fields retain the Script filename object. The Runtime sourced entry therefore installs actual Source before parsing and borrows the authentic filename until error capture; normal execution need not stringify or retain a separate diagnostic filename. This closes source location transport only, not arbitrary evaluation, cache/current compiler context, function dispatch, Normal, or raw/binary filename equivalence. The inspected Jim catch helper passes its original script argv object directly to Jim_EvalObj; selected Runtime Jim control bodies use the existing common original-object door instead of reconstructing unlocated bytes. Original Jim_EvalObj captures parse failures from the retained Script location before entering an evaluation frame, resets automatic capture on a validated Script entry, and captures an Error before popping its current evaluation frame. Runtime retains the actual procedure level, filename and empty invocation on failed lookup until that capture; explicit traces and completed earlier captures remain first-capture owned. No procedure frame is reconstructed after unwinding.

## Scope

Pinned Jim commit5bac7c99ad65864c87da513e22e2f01703fa4e03 source inspection, exact Source/Script/filename/line/token/error-stack object purposes. File-buffer count and C-string APIs are stated separately. No new native or Rust execution.

## Provider answers

### jim

Status: `inspected`. Version: 0.84-9-g5bac7c9. Build: Pinned source inspection only; no executed build association.. Channel: Inspected Jim_EvalSource C-string source and Jim_EvalFile counted read buffers, original Source/Script/token and error-stack objects; no input supplied.. Dialect: Jim Tcl.

Located source installs actual Source filename/line before parsing. Script compilation copies that original filename and per-token lines into original token objects. Plain unlocated objects default to empty filename and line1. Error frames append the original Script filename object. Jim_EvalFile reads counted buffers into its original object; Jim_EvalSource uses C-string length.

### tcl8.4

Status: `not-tested`. Version: 8.4.20. Build: not recorded. Channel: No input supplied for this Jim Source/Script purpose.. Dialect: Tcl.

No C/appliance source or execution comparison for this Jim object-primary question.

### tcl8.5

Status: `not-tested`. Version: 8.5.19. Build: not recorded. Channel: No input supplied for this Jim Source/Script purpose.. Dialect: Tcl.

No C/appliance source or execution comparison for this Jim object-primary question.

### tcl8.6

Status: `not-tested`. Version: 8.6.18. Build: not recorded. Channel: No input supplied for this Jim Source/Script purpose.. Dialect: Tcl.

No C/appliance source or execution comparison for this Jim object-primary question.

### tcl9.0

Status: `not-tested`. Version: 9.0.4. Build: not recorded. Channel: No input supplied for this Jim Source/Script purpose.. Dialect: Tcl.

No C/appliance source or execution comparison for this Jim object-primary question.

### tcl9.1

Status: `not-tested`. Version: 9.1.0. Build: not recorded. Channel: No input supplied for this Jim Source/Script purpose.. Dialect: Tcl.

No C/appliance source or execution comparison for this Jim object-primary question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: No input supplied for this Jim Source/Script purpose.. Dialect: F5 iRules.

No C/appliance source or execution comparison for this Jim object-primary question.

## Exact evidence

- `jim-source-0` (source-anchor): [runtime/rust/tests/data/native_jim_source_entry/source-windows.json](../../../../runtime/rust/tests/data/native_jim_source_entry/source-windows.json). SHA-256 `9e1e7d02735ac9d4e32bbdab2a1c10241b1f71b2725b7d771bfe17cafe2964f1`. JSON pointer `/0/snippet`. Exact original Jim Source/Script/token/error-stack window with LF/full-source/snippet association. No execution claim.
- `jim-source-1` (source-anchor): [runtime/rust/tests/data/native_jim_source_entry/source-windows.json](../../../../runtime/rust/tests/data/native_jim_source_entry/source-windows.json). SHA-256 `9e1e7d02735ac9d4e32bbdab2a1c10241b1f71b2725b7d771bfe17cafe2964f1`. JSON pointer `/1/snippet`. Exact original Jim Source/Script/token/error-stack window with LF/full-source/snippet association. No execution claim.
- `jim-source-2` (source-anchor): [runtime/rust/tests/data/native_jim_source_entry/source-windows.json](../../../../runtime/rust/tests/data/native_jim_source_entry/source-windows.json). SHA-256 `9e1e7d02735ac9d4e32bbdab2a1c10241b1f71b2725b7d771bfe17cafe2964f1`. JSON pointer `/2/snippet`. Exact original Jim Source/Script/token/error-stack window with LF/full-source/snippet association. No execution claim.
- `jim-source-3` (source-anchor): [runtime/rust/tests/data/native_jim_source_entry/source-windows.json](../../../../runtime/rust/tests/data/native_jim_source_entry/source-windows.json). SHA-256 `9e1e7d02735ac9d4e32bbdab2a1c10241b1f71b2725b7d771bfe17cafe2964f1`. JSON pointer `/3/snippet`. Exact original Jim Source/Script/token/error-stack window with LF/full-source/snippet association. No execution claim.
- `jim-source-4` (source-anchor): [runtime/rust/tests/data/native_jim_source_entry/source-windows.json](../../../../runtime/rust/tests/data/native_jim_source_entry/source-windows.json). SHA-256 `9e1e7d02735ac9d4e32bbdab2a1c10241b1f71b2725b7d771bfe17cafe2964f1`. JSON pointer `/4/snippet`. Exact original Jim Source/Script/token/error-stack window with LF/full-source/snippet association. No execution claim.
- `jim-source-5` (source-anchor): [runtime/rust/tests/data/native_jim_source_entry/source-windows.json](../../../../runtime/rust/tests/data/native_jim_source_entry/source-windows.json). SHA-256 `9e1e7d02735ac9d4e32bbdab2a1c10241b1f71b2725b7d771bfe17cafe2964f1`. JSON pointer `/5/snippet`. Exact original Jim Source/Script/token/error-stack window with LF/full-source/snippet association. No execution claim.
- `jim-source-6` (source-anchor): [runtime/rust/tests/data/native_jim_source_entry/source-windows.json](../../../../runtime/rust/tests/data/native_jim_source_entry/source-windows.json). SHA-256 `9e1e7d02735ac9d4e32bbdab2a1c10241b1f71b2725b7d771bfe17cafe2964f1`. JSON pointer `/6/snippet`. Exact original Jim Source/Script/token/error-stack window with LF/full-source/snippet association. No execution claim.
- `jim-source-7` (source-anchor): [runtime/rust/tests/data/native_jim_source_entry/source-windows.json](../../../../runtime/rust/tests/data/native_jim_source_entry/source-windows.json). SHA-256 `9e1e7d02735ac9d4e32bbdab2a1c10241b1f71b2725b7d771bfe17cafe2964f1`. JSON pointer `/7/snippet`. Exact original Jim Source/Script/token/error-stack window with LF/full-source/snippet association. No execution claim.
- `jim-source-8` (source-anchor): [runtime/rust/tests/data/native_jim_source_entry/source-windows.json](../../../../runtime/rust/tests/data/native_jim_source_entry/source-windows.json). SHA-256 `9e1e7d02735ac9d4e32bbdab2a1c10241b1f71b2725b7d771bfe17cafe2964f1`. JSON pointer `/8/snippet`. Original Jim catch helper evaluates the retained argv script object; source inspection only.
- `jim-full` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim/jim.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim/jim.c). SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`. Exact pinned full Jim source supporting the selected source windows and full-file correspondence.
- `jim-error-frame-0` (source-anchor): [runtime/rust/tests/data/native_jim_source_entry/error-frame-windows.json](../../../../runtime/rust/tests/data/native_jim_source_entry/error-frame-windows.json). SHA-256 `3d9b96a5213219a9ba932288336c535479cb257ed0dedd9672a12cb87084e14c`. JSON pointer `/0/snippet`. Original Jim_EvalObj parser/evaluation capture boundary, exact LF/full-source/snippet association; source inspection only.
- `jim-error-frame-1` (source-anchor): [runtime/rust/tests/data/native_jim_source_entry/error-frame-windows.json](../../../../runtime/rust/tests/data/native_jim_source_entry/error-frame-windows.json). SHA-256 `3d9b96a5213219a9ba932288336c535479cb257ed0dedd9672a12cb87084e14c`. JSON pointer `/1/snippet`. Original Jim_EvalObj parser/evaluation capture boundary, exact LF/full-source/snippet association; source inspection only.
- `jim-error-frame-2` (source-anchor): [runtime/rust/tests/data/native_jim_source_entry/error-frame-windows.json](../../../../runtime/rust/tests/data/native_jim_source_entry/error-frame-windows.json). SHA-256 `3d9b96a5213219a9ba932288336c535479cb257ed0dedd9672a12cb87084e14c`. JSON pointer `/2/snippet`. Original Jim_EvalObj parser/evaluation capture boundary, exact LF/full-source/snippet association; source inspection only.

## Source inspection

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `ScriptObjAddTokens`, lines 3635–3647. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `daf4e2428d85f8cf71d5a54da0238010064b7d3cd5d6cef93a90d35e60ef4661`; retained evidence `jim-source-0`.

```text
            const ParseToken *t = &tokenlist->list[i++];

            token->type = t->type;
            token->objPtr = JimMakeScriptObj(interp, t);
            Jim_IncrRefCount(token->objPtr);

            /* Every object is initially a string of type 'source', but the
             * internal type may be specialized during execution of the
             * script. */
            Jim_SetSourceInfo(interp, token->objPtr, script->fileNameObj, t->line);
            token++;
        }
    }

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_GetSourceInfo`, lines 3727–3747. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `0bb79aca785137c131e8755b68d6dfd8924bf9535c586df5750058f240a4444b`; retained evidence `jim-source-1`.

```text
Jim_Obj *Jim_GetSourceInfo(Jim_Interp *interp, Jim_Obj *objPtr, int *lineptr)
{
    int line;
    Jim_Obj *fileNameObj;

    if (objPtr->typePtr == &sourceObjType) {
        fileNameObj = objPtr->internalRep.sourceValue.fileNameObj;
        line = objPtr->internalRep.sourceValue.lineNumber;
    }
    else if (objPtr->typePtr == &scriptObjType) {
        ScriptObj *script = JimGetScript(interp, objPtr);
        fileNameObj = script->fileNameObj;
        line = script->firstline;
    }
    else {
        fileNameObj = interp->emptyObj;
        line = 1;
    }
    *lineptr = line;
    return fileNameObj;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_SetSourceInfo`, lines 3749–3758. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `35e9e861f6e1c593cf6fc3909275de4d7ff5661badba6828f3af6f17efc169b3`; retained evidence `jim-source-2`.

```text
void Jim_SetSourceInfo(Jim_Interp *interp, Jim_Obj *objPtr,
    Jim_Obj *fileNameObj, int lineNumber)
{
    JimPanic((Jim_IsShared(objPtr), "Jim_SetSourceInfo called with shared object"));
    Jim_FreeIntRep(interp, objPtr);
    Jim_IncrRefCount(fileNameObj);
    objPtr->internalRep.sourceValue.fileNameObj = fileNameObj;
    objPtr->internalRep.sourceValue.lineNumber = lineNumber;
    objPtr->typePtr = &sourceObjType;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimSetScriptFromAny`, lines 3791–3844. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `f6c402b64a7486ae6b886b7307c31f8ed4cd10ef688a8ad808ba2e140b678f44`; retained evidence `jim-source-3`.

```text
static void JimSetScriptFromAny(Jim_Interp *interp, struct Jim_Obj *objPtr)
{
    int scriptTextLen;
    const char *scriptText = Jim_GetString(objPtr, &scriptTextLen);
    struct JimParserCtx parser;
    struct ScriptObj *script;
    ParseTokenList tokenlist;
    Jim_Obj *fileNameObj;
    int line;
    int oldtaint;

    /* Try to get information about filename / line number */
    fileNameObj = Jim_GetSourceInfo(interp, objPtr, &line);

    /* Initially parse the script into tokens (in tokenlist) */
    ScriptTokenListInit(&tokenlist);

    JimParserInit(&parser, scriptText, scriptTextLen, line);
    while (!parser.eof) {
        JimParseScript(&parser);
        ScriptAddToken(&tokenlist, parser.tstart, parser.tend - parser.tstart + 1, parser.tt,
            parser.tline);
    }

    /* Add a final EOF token */
    ScriptAddToken(&tokenlist, scriptText + scriptTextLen, 0, JIM_TT_EOF, 0);

    /* Create the "real" script tokens from the parsed tokens */

    /* Set the correct taint on the objects in the script */
    oldtaint = interp->taint;
    interp->taint = objPtr->taint;

    script = Jim_Alloc(sizeof(*script));
    memset(script, 0, sizeof(*script));
    script->inUse = 1;
    script->fileNameObj = fileNameObj;
    Jim_IncrRefCount(script->fileNameObj);
    script->missing = parser.missing.ch;
    script->linenr = parser.missing.line;

    ScriptObjAddTokens(interp, script, &tokenlist);

    /* No longer need the token list */
    ScriptTokenListFree(&tokenlist);

    /* Free the old internal rep and set the new one. */
    Jim_FreeIntRep(interp, objPtr);
    Jim_SetIntRepPtr(objPtr, script);
    objPtr->typePtr = &scriptObjType;

    interp->taint = oldtaint;
}


```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimAddStackFrame`, lines 6151–6168. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `4ba750a4d923b20a06b846475ed11b4608fcbbcb6374a65de857a011178179b3`; retained evidence `jim-source-4`.

```text
static void JimAddStackFrame(Jim_Interp *interp, Jim_EvalFrame *frame, Jim_Obj *listObj)
{
    Jim_Obj *procNameObj = JimProcForEvalFrame(interp, frame);
    Jim_Obj *fileNameObj = interp->emptyObj;
    int linenr = 1;

    if (frame->scriptObj) {
        ScriptObj *script = JimGetScript(interp, frame->scriptObj);
        fileNameObj = script->fileNameObj;
        linenr = script->linenr;
    }

    Jim_ListAppendElement(interp, listObj, procNameObj ? procNameObj : interp->emptyObj);
    Jim_ListAppendElement(interp, listObj, fileNameObj);
    Jim_ListAppendElement(interp, listObj, Jim_NewIntObj(interp, linenr));
    Jim_ListAppendElement(interp, listObj, Jim_NewListObj(interp, frame->argv, frame->argc));
}


```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimSetErrorStack`, lines 6180–6205. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `db70fab854b7998d42a9790361b714bec939a259756b19d46c6cd8e07134b6fe`; retained evidence `jim-source-5`.

```text
static void JimSetErrorStack(Jim_Interp *interp, ScriptObj *script)
{
    if (!interp->hasErrorStackTrace) {
        int i;
        Jim_Obj *stackTrace = Jim_NewListObj(interp, NULL, 0);

        if (interp->procLevel == 0 && script) {
            /* If this is at the top level and there is a script, use the script info
             * rather than the proc info
             */
            Jim_ListAppendElement(interp, stackTrace, interp->emptyObj);
            Jim_ListAppendElement(interp, stackTrace, script->fileNameObj);
            Jim_ListAppendElement(interp, stackTrace, Jim_NewIntObj(interp, script->linenr));
            Jim_ListAppendElement(interp, stackTrace, interp->emptyObj);
        }
        else {
            for (i = 0; i <= interp->procLevel; i++) {
                Jim_EvalFrame *frame = JimGetEvalFrameByProcLevel(interp, -i);
                if (frame) {
                    JimAddStackFrame(interp, frame, stackTrace);
                }
            }
        }
        JimSetStackTrace(interp, stackTrace);
    }
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_EvalSource`, lines 12068–12081. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `852961cf3bad4c9297bb5b101aabb63420585bcc4aace3e4c52804c1db88d55b`; retained evidence `jim-source-6`.

```text
int Jim_EvalSource(Jim_Interp *interp, const char *filename, int lineno, const char *script)
{
    int retval;
    Jim_Obj *scriptObjPtr;

    scriptObjPtr = Jim_NewStringObj(interp, script, -1);
    Jim_IncrRefCount(scriptObjPtr);
    if (filename) {
        Jim_SetSourceInfo(interp, scriptObjPtr, Jim_NewStringObj(interp, filename, -1), lineno);
    }
    retval = Jim_EvalObj(interp, scriptObjPtr);
    Jim_DecrRefCount(interp, scriptObjPtr);
    return retval;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_EvalFile`, lines 12120–12168. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `4dcbe0c82d53aff2760380ea9daed26891e9400aea9c272d8510940734f8df8b`; retained evidence `jim-source-7`.

```text
int Jim_EvalFile(Jim_Interp *interp, const char *filename)
{
    FILE *fp;
    char *buf;
    Jim_Obj *scriptObjPtr;
    Jim_Obj *filenameObj, *oldFilenameObj;
    int retcode = JIM_ERR;
    int readlen;
#define READ_BUF_SIZE 256

   if ((fp = fopen(filename, "rt")) == NULL) {
        Jim_SetResultFormatted(interp, "couldn't read file \"%s\": %s", filename, strerror(errno));
        return JIM_ERR;
    }
    scriptObjPtr = Jim_NewStringObj(interp, NULL, 0);

    buf = Jim_Alloc(READ_BUF_SIZE);
    while ((readlen = fread(buf, 1, READ_BUF_SIZE, fp)) > 0) {
        Jim_AppendString(interp, scriptObjPtr, buf, readlen);
    }
    Jim_Free(buf);
    if (ferror(fp)) {
        fclose(fp);
        Jim_SetResultFormatted(interp, "failed to load file \"%s\": %s", filename, strerror(errno));
        Jim_FreeNewObj(interp, scriptObjPtr);
        return retcode;
    }
    fclose(fp);

    /* Convert the stringObjType to a sourceObjType with filename and line */
    filenameObj = Jim_NewStringObj(interp, filename, -1);
    Jim_SetSourceInfo(interp, scriptObjPtr, filenameObj, 1);
    oldFilenameObj = JimPushInterpObj(interp->currentFilenameObj, filenameObj);

    retcode = Jim_EvalObj(interp, scriptObjPtr);

    JimPopInterpObj(interp, interp->currentFilenameObj, oldFilenameObj);

    /* Handle the JIM_RETURN return code */
    if (retcode == JIM_RETURN) {
        if (--interp->returnLevel <= 0) {
            retcode = interp->returnCode;
            interp->returnCode = JIM_OK;
            interp->returnLevel = 0;
        }
    }

    return retcode;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimCatchTryHelper`, lines 15319–15330. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `0c9bb7938a26a9b55db95c6aa641511d40ec1f10dfe5faea798c8e2b13905f46`; retained evidence `jim-source-8`.

```text

    interp->signal_level += sig;
    if (Jim_CheckSignal(interp)) {
        /* If a signal is set, don't even try to execute the body */
        exitCode = JIM_SIGNAL;
    }
    else {
        exitCode = Jim_EvalObj(interp, argv[idx]);
        /* Once caught, a new error will set a stack trace again */
        interp->hasErrorStackTrace = 0;
    }
    interp->signal_level -= sig;

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_EvalObj`, lines 11568–11575. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `f2df84187410d03f1d7f3d867fe7f6a1d4c178ad93b0d746dc5bdf92e7ef9ddb`; retained evidence `jim-error-frame-0`.

```text

    Jim_IncrRefCount(scriptObjPtr);     /* Make sure it's shared. */
    script = JimGetScript(interp, scriptObjPtr);
    if (JimParseCheckMissing(interp, script->missing) == JIM_ERR) {
        JimSetErrorStack(interp, script);
        Jim_DecrRefCount(interp, scriptObjPtr);
        return JIM_ERR;
    }

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_EvalObj`, lines 11626–11636. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `36c4ba3eb9ba119dab9896a94105b57f623e8000bc30c331d5afb374e71966ca`; retained evidence `jim-error-frame-1`.

```text
    script->inUse++;

    JimPushEvalFrame(interp, &frame, scriptObjPtr);

    /* Collect a new error stack trace if an error occurs */
    interp->hasErrorStackTrace = 0;
    argv = sargv;

    /* Execute every command sequentially until the end of the script
     * or an error occurs.
     */

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_EvalObj`, lines 11780–11786. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `8987c30883d1d34fb12f5c5ab407b3d4e9d9e47d3874ab942ec49fe803c90ffe`; retained evidence `jim-error-frame-2`.

```text
    /* Possibly add to the error stack trace */
    if (retcode == JIM_ERR) {
        JimSetErrorStack(interp, NULL);
    }

    JimPopEvalFrame(interp);


```


## Consumer bindings

- [runtime/rust/src/interp/native_script.rs](../../../../runtime/rust/src/interp/native_script.rs), `Interp::eval_native_jim_source_unpublished`: Install independently selected actual Source before original Script parsing and retain source-entry completion boundary.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::enter_original_jim_evaluation`: Borrow authentic current Script filename alongside original evaluation line without a normal-time string getter.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::materialized_jim_evaluation_frames`: Retain the authentic borrowed filename object only during actual diagnostic snapshot, independently of reporting text.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::eval_control_body`: Selected Jim evaluates the original retained Source/Script control object through the common original-body door; C compilation and TIP280 guards unchanged.
- [runtime/rust/src/interp/native_script.rs](../../../../runtime/rust/src/interp/native_script.rs), `Interp::eval_native_jim_script`: Capture original command failures before the actual Script evaluation scope and procedure level unwind; preserve first-capture and actual empty failed-lookup argv.
- [runtime/rust/src/interp/native_script.rs](../../../../runtime/rust/src/interp/native_script.rs), `Interp::capture_original_jim_parse_failure`: Use the retained original Script filename/completeness line at its pre-frame parser failure boundary, without synthetic evaluation frames.
- [runtime/rust/src/interp/native_script.rs](../../../../runtime/rust/src/interp/native_script.rs), `interp::native_script::tests::sourced_jim_body_tokens_keep_original_lines_and_unlocated_objects_stay_unlocated` (linked): Authentic sourced body token keeps original declaration line/file, while a separate direct unlocated original body stays line1/empty filename; linked obligation only.
- [runtime/rust/src/interp/native_script.rs](../../../../runtime/rust/src/interp/native_script.rs), `interp::native_script::tests::normal_original_script_does_not_materialise_its_borrowed_filename` (linked): Successful original Script with lazy numeric filename does not demand filename string materialisation, and borrowed evaluation stacks are released.
- [runtime/rust/src/cmd_error.rs](../../../../runtime/rust/src/cmd_error.rs), `cmd_error::tests::automatic_jim_error_stacks_match_the_actual_native_interpreter` (linked): Existing finite selected Jim sourced-script error-stack comparison includes the exact nested procedure line; source-record intake makes no Rust pass claim.
- [runtime/rust/src/interp/native_script.rs](../../../../runtime/rust/src/interp/native_script.rs), `interp::native_script::tests::jim_control_body_preserves_original_source_in_error_frames` (linked): Original located Jim control body preserves the same filename object and baseline7 in error frames without borrowing C location metadata.
- [runtime/rust/src/interp/native_script.rs](../../../../runtime/rust/src/interp/native_script.rs), `interp::native_script::tests::jim_original_lookup_error_keeps_live_procedure_and_script_frames` (linked): Selected original Jim constructor and sourced body retain actual procedure/source/caller records on a missing command, and diagnostic borrows unwind cleanly; no execution claimed here.

A named test is a coverage binding, not a claim that it executed.

## Replay

Verify retained full source SHA and exact LF-selected snippets. No executed provider/build or Rust validation is inferred from these source anchors.
