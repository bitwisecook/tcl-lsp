# naming.regex.jim-regsub-precompiled-cleanup

Kind: `native-observation`

## Problem statement

A manually precompiled original Jim pattern can print a successful regsub result and then fail while the harness releases objects. Treating the printed code as full process completion would hide the distinct cleanup boundary.

## Question

Does the precompiled Jim regsub control complete after its result and original object windows are printed?

## Conclusion

The retained attempt prints code zero, RegExp original pattern and result XbX, then exits -6 with a double-free diagnostic. It establishes the partial result window and the abnormal completion only; it supplies no successful object cleanup, reproduction guarantee or safe adapter behavior.

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

Status: `observed`. Version: not queried by this probe; Jim source/library identities retained (patch/revision not queried). Build: Original libjim.a SHA256 75c74eef0c9d1ff2ff8972e6339678564a3439d6332f66032eb4db79004937ff; original source/header/executable metadata in original-capture.. Channel: Original native Jim object/direct-worker arguments and manually inspected internal representation; no document source parser is used.. Dialect: Jim Tcl.

Original selected native attempts: [{"case":9,"exit":-6,"stdout":"code\t0\nobject\tpattern\tregexp\t1\t1\nobject\tsubject\tnone\t1\t1\nobject\treplacement\tnone\t1\t1\nobject\tresult\tstring\t1\t1\nbytes\t586258\n","stderr":"free(): double free detected in tcache 2\n"}]. Process status is independent of printed command return and object rows.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

## Exact evidence

- `original-capture` (provider): [runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json](../../../../runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json). SHA-256 `6f5e36c0589ba3c4e5b40e21f880bdd5afaba2c54c488f03e7deedc01079ee8e`. Original source/library/header/build context and every process status/stream, including incomplete attempts.
- `original-probe` (input): [runtime/rust/tests/data/native_regexp_jim_regsub/probe.c](../../../../runtime/rust/tests/data/native_regexp_jim_regsub/probe.c). SHA-256 `b427c8677cb8dcf5d7343c9cda1bae6d93e492fbe964a5c2de49c88b8fa5a803`. Exact direct Jim worker invocation, object construction and physical reporting order.
- `jim-case-9` (observation): [runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json](../../../../runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json). SHA-256 `6f5e36c0589ba3c4e5b40e21f880bdd5afaba2c54c488f03e7deedc01079ee8e`. JSON pointer `/captures/9`. Exact original process status and raw output/error of this case.
- `exact-source-0` (source-anchor): [docs/design/analysis/name-resolution-proofs/regexp-retained-source-anchors.json](../../../../docs/design/analysis/name-resolution-proofs/regexp-retained-source-anchors.json). SHA-256 `f36a41d7658d9b17accf0b4bf93dc17a1c11008bd8d1261ff9c10ce83f58678c`. JSON pointer `/0/snippet`. Exact current source excerpt whose complete source SHA equals the original captured source; independent of interpreter output and build revision query.
- `exact-source-1` (source-anchor): [docs/design/analysis/name-resolution-proofs/regexp-retained-source-anchors.json](../../../../docs/design/analysis/name-resolution-proofs/regexp-retained-source-anchors.json). SHA-256 `f36a41d7658d9b17accf0b4bf93dc17a1c11008bd8d1261ff9c10ce83f58678c`. JSON pointer `/1/snippet`. Exact current source excerpt whose complete source SHA equals the original captured source; independent of interpreter output and build revision query.

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

jim Jim0.84 captured source label; no captured patch query, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03 (current source inspection; full-file digest equals captured source)`, `jim-regexp.c`, function `Jim_RegsubCmd`, lines 452–483. Full-source SHA-256 `7b0f5acc1cb334b3d372d0a4ce991d7553941983fc7be5ee5026216fa6053ddd`; snippet SHA-256 `60d1754027bfbe5928e2189ffd553ac28fab8a6d907b8874249f81775ff5b5c3`; retained evidence `exact-source-1`.

```text
    }
    if (argc - i != 3 && argc - i != 4) {
        return JIM_USAGE;
    }

    /* Need to ensure that this is unshared, so just duplicate it always */
    regcomp_obj = Jim_DuplicateObj(interp, argv[i]);
    Jim_IncrRefCount(regcomp_obj);
    regex = SetRegexpFromAny(interp, regcomp_obj, regcomp_flags);
    if (!regex) {
        Jim_DecrRefCount(interp, regcomp_obj);
        return JIM_ERR;
    }
    pattern = Jim_String(argv[i]);

    source_str = Jim_GetString(argv[i + 1], &source_len);
    if (opt_command) {
        cmd_prefix = argv[i + 2];
        if (Jim_ListLength(interp, cmd_prefix) == 0) {
            Jim_SetResultString(interp, "command prefix must be a list of at least one element", -1);
            Jim_DecrRefCount(interp, regcomp_obj);
            return JIM_ERR;
        }
        Jim_IncrRefCount(cmd_prefix);
    }
    else {
        replace_str = Jim_GetString(argv[i + 2], &replace_len);
    }
    varname = argv[i + 3];

    /* Create the result string */
    resultObj = Jim_NewStringObj(interp, "", 0);

```


## Consumer bindings

- [runtime/rust/src/cmd_regex.rs](../../../../runtime/rust/src/cmd_regex.rs), `regsub_cmd`: Separate Rust regex consumer; these physical native rows provide no executed Rust or object-manufacturer grant.

A named test is a coverage binding, not a claim that it executed.

## Replay

No replay runner or new execution is asserted. Rebuild the retained probe only with independently verified matching source/header/library identities, run each selected case in a fresh process and preserve stdout, stderr and process status separately. Original abnormal exits are retained observations of incomplete attempts; they are not successful completions and are not requested to be rerun.
