# naming.regex.jim-original-regexp-range

Kind: `native-observation`

## Problem statement

A Jim compiled pattern, original subject and stored range have independent primary/resident fields. UTF-shaped input bytes and a malformed pattern must be observed in the Jim worker rather than reconstructed from the C RegExp recipe.

## Question

What pattern, subject, result and range windows do the original Jim regexp byte-unit and malformed-pattern controls retain?

## Conclusion

The a/FF/C080/F09F9880 controls complete with RegExp pattern, untyped original subject and integer result; their stored string ranges report the original selected byte sequences. The malformed ( control reports compile false and leaves untyped pattern. These exact inputs do not establish a C range policy, Jim name comparison or universal UTF validity.

## Scope

Only the exact retained Jim direct-worker/object probe and selected case windows are covered. The library/header/source identities in this manifest are original; a captured version label does not certify an unqueried patch, checkout or UTF configuration. Internal object invalidation/duplication and cleanup are harness-specific operations. Printed rows preceding an abnormal exit remain partial observations, not successful lifecycle or adapter completion. C Tcl, BIG-IP and fresh replay/Rust outcomes are not measured here.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, bigip

Status: `not-tested`.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

### jim

Status: `observed`. Channel: Original native Jim object/direct-worker arguments and manually inspected internal representation; no document source parser is used.

Original selected native attempts: [{"case":0,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncode\t0\npatternafter\tregexp\t1\t1\nsubject\tnone\t1\t1\nresult\tint\t0\t1\nrange\tstring\t1\t1\nbytes\t61\n","stderr":""},{"case":4,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncode\t0\npatternafter\tregexp\t1\t1\nsubject\tnone\t1\t1\nresult\tint\t0\t1\nrange\tstring\t1\t1\nbytes\tff\n","stderr":""},{"case":5,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncode\t0\npatternafter\tregexp\t1\t1\nsubject\tnone\t1\t1\nresult\tint\t0\t1\nrange\tstring\t1\t1\nbytes\tc080\n","stderr":""},{"case":6,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncode\t0\npatternafter\tregexp\t1\t1\nsubject\tnone\t1\t1\nresult\tint\t0\t1\nrange\tstring\t1\t1\nbytes\tf09f9880\n","stderr":""},{"case":7,"exit":0,"stdout":"pattern\tnone\t1\t1\ncompile\t0\n","stderr":""}]. Process status is independent of printed command return and object rows.

## Source inspection

jim Jim0.84 captured source label; no captured patch query, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03 (current source inspection; full-file digest equals captured source)`, `jim-regexp.c`, function `SetRegexpFromAny`, lines 64–118. Retained reference `exact-source-0`.

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

## Evidence

- [runtime/rust/tests/data/native_regexp_jim/manifest.json](../../../../runtime/rust/tests/data/native_regexp_jim/manifest.json)
- [runtime/rust/tests/data/native_regexp_jim/probe.c](../../../../runtime/rust/tests/data/native_regexp_jim/probe.c)
