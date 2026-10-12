# naming.regex.jim-compiled-pattern-lifetime

Kind: `native-observation`

## Problem statement

Removing a Jim pattern string, changing flags or duplicating a compiled pattern exercises cache and lifetime paths that are separate from successful pattern matching. A process abort cannot be replaced by an invented successful getter result.

## Question

Which original Jim compiled-pattern invalidation and duplicate attempts complete, and which retain an abnormal process exit?

## Conclusion

The same-flag invalidation control completes and reuses the compiled pointer without resident bytes. Changed flags and direct-worker use after string invalidation exit -11 after their initial rows; duplicate cleanup exits -6 with a double-free diagnostic. These retained attempts certify no successful replacement or cleanup beyond the reported windows.

## Scope

Only the exact retained Jim direct-worker/object probe and selected case windows are covered. The library/header/source identities in this manifest are original; a captured version label does not certify an unqueried patch, checkout or UTF configuration. Internal object invalidation/duplication and cleanup are harness-specific operations. Printed rows preceding an abnormal exit remain partial observations, not successful lifecycle or adapter completion. C Tcl, BIG-IP and fresh replay/Rust outcomes are not measured here.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

### jim

Status: `observed`. Version: Jim0.84 (patch/revision not queried). Build: Original libjim.a SHA256 75c74eef0c9d1ff2ff8972e6339678564a3439d6332f66032eb4db79004937ff; original source/header/executable metadata in original-capture.. Channel: Original native Jim object/direct-worker arguments and manually inspected internal representation; no document source parser is used.. Dialect: Jim Tcl.

Original selected native attempts: [{"case":1,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncache\t1\nafter\tregexp\t0\t1\n","stderr":""},{"case":2,"exit":-11,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\n","stderr":""},{"case":3,"exit":-6,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\nduplicate\tregexp\t1\t1\ncache\t1\n","stderr":"free(): double free detected in tcache 2\n"},{"case":8,"exit":-11,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\n","stderr":""}]. Process status is independent of printed command return and object rows.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

## Exact evidence

- `original-capture` (provider): [runtime/rust/tests/data/native_regexp_jim/manifest.json](../../../../runtime/rust/tests/data/native_regexp_jim/manifest.json). SHA-256 `9ffd57acfc7ab547fcefbec91c4b1215b1792607e674392f53ce9f89af8340ad`. Original source/library/header/build context and every process status/stream, including incomplete attempts.
- `original-probe` (input): [runtime/rust/tests/data/native_regexp_jim/probe.c](../../../../runtime/rust/tests/data/native_regexp_jim/probe.c). SHA-256 `dbd3c02c4d9b11f3f8cf91cefbffef96ca8a128ed6d84fd20fcec46315372f2e`. Exact direct Jim worker invocation, object construction and physical reporting order.
- `jim-case-1` (observation): [runtime/rust/tests/data/native_regexp_jim/manifest.json](../../../../runtime/rust/tests/data/native_regexp_jim/manifest.json). SHA-256 `9ffd57acfc7ab547fcefbec91c4b1215b1792607e674392f53ce9f89af8340ad`. JSON pointer `/captures/1`. Exact original process status and raw output/error of this case.
- `jim-case-2` (observation): [runtime/rust/tests/data/native_regexp_jim/manifest.json](../../../../runtime/rust/tests/data/native_regexp_jim/manifest.json). SHA-256 `9ffd57acfc7ab547fcefbec91c4b1215b1792607e674392f53ce9f89af8340ad`. JSON pointer `/captures/2`. Exact original process status and raw output/error of this case.
- `jim-case-3` (observation): [runtime/rust/tests/data/native_regexp_jim/manifest.json](../../../../runtime/rust/tests/data/native_regexp_jim/manifest.json). SHA-256 `9ffd57acfc7ab547fcefbec91c4b1215b1792607e674392f53ce9f89af8340ad`. JSON pointer `/captures/3`. Exact original process status and raw output/error of this case.
- `jim-case-8` (observation): [runtime/rust/tests/data/native_regexp_jim/manifest.json](../../../../runtime/rust/tests/data/native_regexp_jim/manifest.json). SHA-256 `9ffd57acfc7ab547fcefbec91c4b1215b1792607e674392f53ce9f89af8340ad`. JSON pointer `/captures/8`. Exact original process status and raw output/error of this case.
- `exact-source-0` (source-anchor): [docs/design/analysis/name-resolution-proofs/regexp-retained-source-anchors.json](../../../../docs/design/analysis/name-resolution-proofs/regexp-retained-source-anchors.json). SHA-256 `f36a41d7658d9b17accf0b4bf93dc17a1c11008bd8d1261ff9c10ce83f58678c`. JSON pointer `/0/snippet`. Exact current source excerpt whose complete source SHA equals the original captured source; independent of interpreter output and build revision query.

## Source inspection

jim Jim0.84 captured source label; no captured patch query, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03 (current source inspection; full-file digest equals captured source)`, `jim-regexp.c`, function `SetRegexpFromAny`, lines 64–118. Full-source SHA-256 `7b0f5acc1cb334b3d372d0a4ce991d7553941983fc7be5ee5026216fa6053ddd`; snippet SHA-256 `59666c07b5a37595ac1f946f1a627d8212ece3241bf8c95609abfbdd66380a5f`; retained evidence `exact-source-0`.

```text
static void FreeRegexpInternalRep(Jim_Interp *interp, Jim_Obj *objPtr)
{
    jim_regfree(objPtr->internalRep.ptrIntValue.ptr);
    Jim_Free(objPtr->internalRep.ptrIntValue.ptr);
}

/* internal rep is stored in ptrIntvalue
 *  ptr = compiled regex
 *  int1 = flags
 */
static const Jim_ObjType regexpObjType = {
    "regexp",
    FreeRegexpInternalRep,
    NULL,
    NULL,
    JIM_TYPE_NONE
};

static regex_t *SetRegexpFromAny(Jim_Interp *interp, Jim_Obj *objPtr, unsigned flags)
{
    regex_t *compre;
    const char *pattern;
    int ret;

    /* Check if the object is already an uptodate variable */
    if (objPtr->typePtr == &regexpObjType &&
        objPtr->internalRep.ptrIntValue.ptr && objPtr->internalRep.ptrIntValue.int1 == flags) {
        /* nothing to do */
        return objPtr->internalRep.ptrIntValue.ptr;
    }

    /* Not a regexp or the flags do not match */

    /* Get the string representation */
    pattern = Jim_String(objPtr);
    compre = Jim_Alloc(sizeof(regex_t));

    if ((ret = jim_regcomp(compre, pattern, REG_EXTENDED | flags)) != 0) {
        char buf[100];

        jim_regerror(ret, compre, buf, sizeof(buf));
        Jim_SetResultFormatted(interp, "couldn't compile regular expression pattern: %s", buf);
        jim_regfree(compre);
        Jim_Free(compre);
        return NULL;
    }

    Jim_FreeIntRep(interp, objPtr);

    objPtr->typePtr = &regexpObjType;
    objPtr->internalRep.ptrIntValue.int1 = flags;
    objPtr->internalRep.ptrIntValue.ptr = compre;

    return compre;
}

```


## Consumer bindings

- [runtime/rust/src/cmd_regex.rs](../../../../runtime/rust/src/cmd_regex.rs), `regexp_cmd`: Separate Rust regex consumer; these physical native rows provide no executed Rust or object-manufacturer grant.
- [runtime/rust/src/cmd_regex.rs](../../../../runtime/rust/src/cmd_regex.rs), `cmd_regex::tests::jim_original_regexp_cache_ranges_and_unsafe_lifecycles_match_native_controls` (linked): Checks its scoped original input/object/result windows against retained native controls. Adapter withdrawal after an aborted native attempt is a separate Rust contract; no executed Rust result is attached.

A named test is a coverage binding, not a claim that it executed.

## Replay

No replay runner or new execution is asserted. Rebuild the retained probe only with independently verified matching source/header/library identities, run each selected case in a fresh process and preserve stdout, stderr and process status separately. Original abnormal exits are retained observations of incomplete attempts; they are not successful completions and are not requested to be rerun.
