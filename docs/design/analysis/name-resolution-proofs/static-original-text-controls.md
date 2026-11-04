# naming.source.static-original-text-controls

Kind: `native-observation`

## Problem statement

An escaped source word can produce non-Unicode native bytes. Refusing its completed source evaluation because a Unicode reporting projection is absent prevents a following genuine procedure call from preserving its original publication world.

## Question

Do the exact unquoted, quoted and empty Text-only source words complete normally, and does the exact escaped opaque procedure head invoke the declared procedure, on the selected fresh C Tcl and Jim source drivers?

## Conclusion

The four positive cases complete with code0 on all five C releases in both explicit source modes and on Jim source. Unquoted and quoted p\uD800 produce counted bytes 70 ed a0 80; the empty quote produces zero bytes; the opaque-head procedure returns VALUE. The two negative controls complete with code1. These finite output observations do not establish a literal pool, object identity, custom-object closure or compiler admission. Separately inspected Jim source constructs parser-token bytes through JimEscape/Jim_NewStringObjNoAlloc and supplies Text tokens directly to evaluation; interpolation has its own rendering path and existing-object obligations.

## Scope

Six exact ASCII source strings, one fresh interpreter per case and source mode. C Tcl_EvalEx flags0 and TCL_EVAL_DIRECT are separate actual public arguments. Jim uses Jim_Eval and explicitly records that a C Direct recipe is unavailable. No raw NUL, encoded surrogate source, arbitrary host registration, observed callback, reused native script object or physical literal cache is tested.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual executable; exact original compile argv, header/library/build/source and stream digests are retained in the provider receipt. Channel: Original ASCII source passed to Tcl_EvalEx flags0 and TCL_EVAL_DIRECT in separate fresh interpreters. Dialect: Tcl.

Four positive cases complete0: escaped words 70 ed a0 80, empty quote zero bytes, opaque procedure call VALUE; both negative cases complete1. The same finite observations hold in each explicit C source mode.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual executable; exact original compile argv, header/library/build/source and stream digests are retained in the provider receipt. Channel: Original ASCII source passed to Tcl_EvalEx flags0 and TCL_EVAL_DIRECT in separate fresh interpreters. Dialect: Tcl.

Four positive cases complete0: escaped words 70 ed a0 80, empty quote zero bytes, opaque procedure call VALUE; both negative cases complete1. The same finite observations hold in each explicit C source mode.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable; exact original compile argv, header/library/build/source and stream digests are retained in the provider receipt. Channel: Original ASCII source passed to Tcl_EvalEx flags0 and TCL_EVAL_DIRECT in separate fresh interpreters. Dialect: Tcl.

Four positive cases complete0: escaped words 70 ed a0 80, empty quote zero bytes, opaque procedure call VALUE; both negative cases complete1. The same finite observations hold in each explicit C source mode.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable; exact original compile argv, header/library/build/source and stream digests are retained in the provider receipt. Channel: Original ASCII source passed to Tcl_EvalEx flags0 and TCL_EVAL_DIRECT in separate fresh interpreters. Dialect: Tcl.

Four positive cases complete0: escaped words 70 ed a0 80, empty quote zero bytes, opaque procedure call VALUE; both negative cases complete1. The same finite observations hold in each explicit C source mode.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable; exact original compile argv, header/library/build/source and stream digests are retained in the provider receipt. Channel: Original ASCII source passed to Tcl_EvalEx flags0 and TCL_EVAL_DIRECT in separate fresh interpreters. Dialect: Tcl.

Four positive cases complete0: escaped words 70 ed a0 80, empty quote zero bytes, opaque procedure call VALUE; both negative cases complete1. The same finite observations hold in each explicit C source mode.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual executable; exact original compile argv, header/library/build/source and stream digests are retained in the provider receipt. Channel: Original ASCII source passed to Jim_Eval; no C Direct recipe. Dialect: Jim Tcl.

Four positive cases complete0: escaped words 70 ed a0 80, empty quote zero bytes, opaque procedure call VALUE; both negative cases complete1. The Jim C Direct control is unavailable.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded Channel: not tested Dialect: F5 iRules.

No appliance capture is attached to these exact fresh-source scripts.

## Historical references

- `probe` (input): [rust/tcl-compiler/tests/data/native_original_jim_text/probe.c](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/probe.c). Exact original source scripts, fresh interpreter per case and explicit C source modes or Jim source.
- `inputs` (input): [rust/tcl-compiler/tests/data/native_original_jim_text/inputs.json](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/inputs.json). Exact ASCII source escapes and question scope; no raw surrogate or NUL source character.
- `aggregate` (provider): [rust/tcl-compiler/tests/data/native_original_jim_text/receipt.json](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/receipt.json). All six actual original compile/process captures and provider/input/executable/stream digests.
- `queue` (provider): [rust/tcl-compiler/tests/data/native_original_jim_text/queue.json](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/queue.json). Original input/provider/source/header/library/build pin correspondence.
- `capture-runner` (provider): [rust/tcl-compiler/tests/data/native_original_jim_text/capture.py](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/capture.py). Exact original capture driver; absolute paths are archival.
- `replay` (implementation): [rust/tcl-compiler/tests/data/native_original_jim_text/replay.py](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/replay.py). Offline retained-byte verification and optional independent fresh capture using exact provider pins.
- `tcl8.4-receipt.json` (provider): [rust/tcl-compiler/tests/data/native_original_jim_text/8.4.20/receipt.json](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/8.4.20/receipt.json). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl8.4-stdout.tsv` (observation): [rust/tcl-compiler/tests/data/native_original_jim_text/8.4.20/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/8.4.20/stdout.tsv). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl8.4-stderr` (observation): [rust/tcl-compiler/tests/data/native_original_jim_text/8.4.20/stderr](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/8.4.20/stderr). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl8.4-compile.stdout` (provider): [rust/tcl-compiler/tests/data/native_original_jim_text/8.4.20/compile.stdout](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/8.4.20/compile.stdout). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl8.4-compile.stderr` (provider): [rust/tcl-compiler/tests/data/native_original_jim_text/8.4.20/compile.stderr](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/8.4.20/compile.stderr). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl8.5-receipt.json` (provider): [rust/tcl-compiler/tests/data/native_original_jim_text/8.5.19/receipt.json](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/8.5.19/receipt.json). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl8.5-stdout.tsv` (observation): [rust/tcl-compiler/tests/data/native_original_jim_text/8.5.19/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/8.5.19/stdout.tsv). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl8.5-stderr` (observation): [rust/tcl-compiler/tests/data/native_original_jim_text/8.5.19/stderr](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/8.5.19/stderr). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl8.5-compile.stdout` (provider): [rust/tcl-compiler/tests/data/native_original_jim_text/8.5.19/compile.stdout](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/8.5.19/compile.stdout). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl8.5-compile.stderr` (provider): [rust/tcl-compiler/tests/data/native_original_jim_text/8.5.19/compile.stderr](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/8.5.19/compile.stderr). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl8.6-receipt.json` (provider): [rust/tcl-compiler/tests/data/native_original_jim_text/8.6.18/receipt.json](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/8.6.18/receipt.json). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl8.6-stdout.tsv` (observation): [rust/tcl-compiler/tests/data/native_original_jim_text/8.6.18/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/8.6.18/stdout.tsv). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl8.6-stderr` (observation): [rust/tcl-compiler/tests/data/native_original_jim_text/8.6.18/stderr](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/8.6.18/stderr). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl8.6-compile.stdout` (provider): [rust/tcl-compiler/tests/data/native_original_jim_text/8.6.18/compile.stdout](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/8.6.18/compile.stdout). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl8.6-compile.stderr` (provider): [rust/tcl-compiler/tests/data/native_original_jim_text/8.6.18/compile.stderr](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/8.6.18/compile.stderr). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl9.0-receipt.json` (provider): [rust/tcl-compiler/tests/data/native_original_jim_text/9.0.4/receipt.json](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/9.0.4/receipt.json). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl9.0-stdout.tsv` (observation): [rust/tcl-compiler/tests/data/native_original_jim_text/9.0.4/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/9.0.4/stdout.tsv). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl9.0-stderr` (observation): [rust/tcl-compiler/tests/data/native_original_jim_text/9.0.4/stderr](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/9.0.4/stderr). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl9.0-compile.stdout` (provider): [rust/tcl-compiler/tests/data/native_original_jim_text/9.0.4/compile.stdout](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/9.0.4/compile.stdout). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl9.0-compile.stderr` (provider): [rust/tcl-compiler/tests/data/native_original_jim_text/9.0.4/compile.stderr](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/9.0.4/compile.stderr). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl9.1-receipt.json` (provider): [rust/tcl-compiler/tests/data/native_original_jim_text/9.1.0/receipt.json](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/9.1.0/receipt.json). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl9.1-stdout.tsv` (observation): [rust/tcl-compiler/tests/data/native_original_jim_text/9.1.0/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/9.1.0/stdout.tsv). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl9.1-stderr` (observation): [rust/tcl-compiler/tests/data/native_original_jim_text/9.1.0/stderr](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/9.1.0/stderr). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl9.1-compile.stdout` (provider): [rust/tcl-compiler/tests/data/native_original_jim_text/9.1.0/compile.stdout](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/9.1.0/compile.stdout). Unchanged original per-provider receipt or actual compile/process stream.
- `tcl9.1-compile.stderr` (provider): [rust/tcl-compiler/tests/data/native_original_jim_text/9.1.0/compile.stderr](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/9.1.0/compile.stderr). Unchanged original per-provider receipt or actual compile/process stream.
- `jim-receipt.json` (provider): [rust/tcl-compiler/tests/data/native_original_jim_text/jim/receipt.json](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/jim/receipt.json). Unchanged original per-provider receipt or actual compile/process stream.
- `jim-stdout.tsv` (observation): [rust/tcl-compiler/tests/data/native_original_jim_text/jim/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/jim/stdout.tsv). Unchanged original per-provider receipt or actual compile/process stream.
- `jim-stderr` (observation): [rust/tcl-compiler/tests/data/native_original_jim_text/jim/stderr](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/jim/stderr). Unchanged original per-provider receipt or actual compile/process stream.
- `jim-compile.stdout` (provider): [rust/tcl-compiler/tests/data/native_original_jim_text/jim/compile.stdout](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/jim/compile.stdout). Unchanged original per-provider receipt or actual compile/process stream.
- `jim-compile.stderr` (provider): [rust/tcl-compiler/tests/data/native_original_jim_text/jim/compile.stderr](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/jim/compile.stderr). Unchanged original per-provider receipt or actual compile/process stream.
- `jim-source-2087` (source-anchor): [rust/tcl-compiler/tests/data/native_original_jim_text/source/jim-2087-2111.txt](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/source/jim-2087-2111.txt). Exact Jim source excerpt; full original source digest and line extent are retained separately.
- `jim-source-2549` (source-anchor): [rust/tcl-compiler/tests/data/native_original_jim_text/source/jim-2549-2558.txt](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/source/jim-2549-2558.txt). Exact Jim source excerpt; full original source digest and line extent are retained separately.
- `jim-source-11256` (source-anchor): [rust/tcl-compiler/tests/data/native_original_jim_text/source/jim-11256-11269.txt](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/source/jim-11256-11269.txt). Exact Jim source excerpt; full original source digest and line extent are retained separately.
- `jim-source-11663` (source-anchor): [rust/tcl-compiler/tests/data/native_original_jim_text/source/jim-11663-11679.txt](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/source/jim-11663-11679.txt). Exact Jim source excerpt; full original source digest and line extent are retained separately.
- `jim-source-11343` (source-anchor): [rust/tcl-compiler/tests/data/native_original_jim_text/source/jim-11343-11400.txt](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/source/jim-11343-11400.txt). Exact Jim source excerpt; full original source digest and line extent are retained separately.
- `source-anchors` (source-anchor): [rust/tcl-compiler/tests/data/native_original_jim_text/source-anchors.json](../../../../rust/tcl-compiler/tests/data/native_original_jim_text/source-anchors.json). Source-reading identity and exact function/line/snippet/full-source digests; source inspection is independent of output observations.

## Source inspection

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `jim.c`, function `JimParserGetTokenObj`, lines 2087–2111. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `31a91b47ddf67147dfb8ef80db497dc8bd3c1f1bf2c31d09da6e8f59ec3cc976`; evidence `jim-source-2087`.

```c
static Jim_Obj *JimParserGetTokenObj(Jim_Interp *interp, struct JimParserCtx *pc)
{
    const char *start, *end;
    char *token;
    int len;

    start = pc->tstart;
    end = pc->tend;
    len = (end - start) + 1;
    if (len < 0) {
        len = 0;
    }
    token = Jim_Alloc(len + 1);
    if (pc->tt != JIM_TT_ESC) {
        /* No escape conversion needed? Just copy it. */
        memcpy(token, start, len);
        token[len] = '\0';
    }
    else {
        /* Else convert the escape chars. */
        len = JimEscape(token, start, len);
    }

    return Jim_NewStringObjNoAlloc(interp, token, len);
}
```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `jim.c`, function `Jim_NewStringObjNoAlloc`, lines 2549–2558. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `293970b4f9c4577f796359cdd82ecc72db4f1b8a658cd0f4ea7ddddb271edb60`; evidence `jim-source-2549`.

```c
Jim_Obj *Jim_NewStringObjNoAlloc(Jim_Interp *interp, char *s, int len)
{
    Jim_Obj *objPtr = Jim_NewObj(interp);

    objPtr->bytes = s;
    objPtr->length = (len == -1) ? strlen(s) : len;
    objPtr->typePtr = NULL;
    return objPtr;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `jim.c`, function `JimSubstOneToken`, lines 11256–11269. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `22d884776010882e019ee91f9a113fc37edaece0915053e89da40fb139178305`; evidence `jim-source-11256`.

```c
static int JimSubstOneToken(Jim_Interp *interp, const ScriptToken *token, Jim_Obj **objPtrPtr)
{
    Jim_Obj *objPtr;
    int ret = JIM_ERR;

    switch (token->type) {
        case JIM_TT_STR:
        case JIM_TT_ESC:
            objPtr = token->objPtr;
            break;
        case JIM_TT_VAR:
            objPtr = Jim_GetVariable(interp, token->objPtr, JIM_ERRMSG);
            break;
        case JIM_TT_DICTSUGAR:
```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `jim.c`, function `Jim_EvalObj Text fast path`, lines 11663–11679. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `256ef9fd75fdf6354d3ce9460e741006673d80554b353639ee2ead54514635c9`; evidence `jim-source-11663`.

```c
                if (wordtokens < 0) {
                    expand = 1;
                    wordtokens = -wordtokens;
                }
            }

            if (wordtokens == 1) {
                /* Fast path if the token does not
                 * need interpolation */

                switch (token[i].type) {
                    case JIM_TT_ESC:
                    case JIM_TT_STR:
                        wordObjPtr = token[i].objPtr;
                        break;
                    case JIM_TT_VAR:
                        wordObjPtr = Jim_GetVariable(interp, token[i].objPtr, JIM_ERRMSG);
```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `jim.c`, function `JimInterpolateTokens text rendering and copy`, lines 11343–11400. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `996055d8c3bebda098ffcab8933ffca0fb30cc6edab3ed1ea0ee29d4506e22e7`; evidence `jim-source-11343`.

```c
                    error_action = "continue";
                }
                /* fall through to error */
            default:
                while (i--) {
                    Jim_DecrRefCount(interp, intv[i]);
                }
                if (intv != sintv) {
                    Jim_Free(intv);
                }
                if (error_action) {
                    Jim_SetResultFormatted(interp, "invoked \"%s\" outside of a loop", error_action);
                }
                return NULL;
        }
        taint |= intv[i]->taint;
        Jim_IncrRefCount(intv[i]);
        Jim_String(intv[i]);
        totlen += intv[i]->length;
    }

    /* Fast path return for a single token */
    if (tokens == 1 && intv[0] && intv == sintv) {
        /* Reverse the Jim_IncrRefCount() above, but don't free the object */
        intv[0]->refCount--;
        return intv[0];
    }

    /* Concatenate every token in an unique
     * object. */
    objPtr = Jim_NewStringObjNoAlloc(interp, NULL, 0);
    objPtr->taint = taint;

    if (tokens == 4 && token[0].type == JIM_TT_ESC && token[1].type == JIM_TT_ESC
        && token[2].type == JIM_TT_VAR) {
        /* May be able to do fast interpolated object -> dictSubst */
        objPtr->typePtr = &interpolatedObjType;
        objPtr->internalRep.dictSubstValue.varNameObjPtr = token[0].objPtr;
        objPtr->internalRep.dictSubstValue.indexObjPtr = intv[2];
        Jim_IncrRefCount(intv[2]);
    }
    else if (tokens && intv[0] && intv[0]->typePtr == &sourceObjType) {
        /* The first interpolated token is source, so preserve the source info */
        int line;
        Jim_Obj *fileNameObj = Jim_GetSourceInfo(interp, intv[0], &line);
        Jim_SetSourceInfo(interp, objPtr, fileNameObj, line);
    }


    s = objPtr->bytes = Jim_Alloc(totlen + 1);
    objPtr->length = totlen;
    for (i = 0; i < tokens; i++) {
        if (intv[i]) {
            memcpy(s, intv[i]->bytes, intv[i]->length);
            s += intv[i]->length;
            Jim_DecrRefCount(interp, intv[i]);
        }
    }
```

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/evaluated_word.rs](../../../../rust/tcl-compiler/src/command_binding/evaluated_word.rs), `command_binding::evaluated_word::tests::original_text_evaluation_matches_exact_native_source_controls` (linked): Compare current original frozen operand bytes and complete source worlds with the exact retained Direct C/Jim source result windows; negative source errors remain incomplete. No physical object or compiler equivalence is asserted.

A named selector is a coverage binding, not a claim that it executed.

## Replay

```json
[
  "python3",
  "rust/tcl-compiler/tests/data/native_original_jim_text/replay.py",
  "--verify-only"
]
```

Offline verification checks retained exact input/stream/hash and finite result controls; it makes no fresh execution claim. Optional --output requires an absent directory and every original provider pin at the archived path, then records fresh attempts separately. Jim C Direct is unavailable. No Rust execution result is inferred.
