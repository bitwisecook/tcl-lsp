# naming.regex.jim-regsub-original-operands

Kind: `native-observation`

## Problem statement

Jim regsub can use a duplicated pattern for compilation while leaving its original pattern and subject untouched. Backreferences, zero-width matches, start-index parsing and callback prefixes use distinct original operands and should not inherit a C result-storage rule.

## Question

What original operand primaries and result bytes do the nine completed Jim regsub controls retain?

## Conclusion

The nine recorded attempts complete with code zero, unchanged untyped original pattern/subject and a string result. They cover literal/backreference/zero-width/start and callback replacements, with exact result bytes retained per case; only the callback replacement becomes List and the start index becomes index primary. No native input channel or ordinary interpreter call outside the exact direct-worker probe is inferred.

## Scope

Only the exact retained Jim direct-worker/object probe and selected case windows are covered. The library/header/source identities in this manifest are original; a captured version label does not certify an unqueried patch, checkout or UTF configuration. Internal object invalidation/duplication and cleanup are harness-specific operations. Printed rows preceding an abnormal exit remain partial observations, not successful lifecycle or adapter completion. C Tcl, BIG-IP and fresh replay/Rust outcomes are not measured here.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, bigip

Status: `not-tested`.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

### jim

Status: `observed`. Channel: Original native Jim object/direct-worker arguments and manually inspected internal representation; no document source parser is used.

Original selected native attempts: [{"case":0,"exit":0,"stdout":"code\t0\nobject\tpattern\tnone\t1\t1\nobject\tsubject\tnone\t1\t1\nobject\treplacement\tnone\t1\t1\nobject\tresult\tstring\t1\t1\nbytes\t586258\n","stderr":""},{"case":1,"exit":0,"stdout":"code\t0\nobject\tpattern\tnone\t1\t1\nobject\tsubject\tnone\t1\t1\nobject\treplacement\tnone\t1\t1\nobject\tresult\tstring\t1\t1\nbytes\t58\n","stderr":""},{"case":2,"exit":0,"stdout":"code\t0\nobject\tpattern\tnone\t1\t1\nobject\tsubject\tnone\t1\t1\nobject\treplacement\tnone\t1\t1\nobject\tresult\tstring\t1\t1\nbytes\t58\n","stderr":""},{"case":3,"exit":0,"stdout":"code\t0\nobject\tpattern\tnone\t1\t1\nobject\tsubject\tnone\t1\t1\nobject\treplacement\tnone\t1\t1\nobject\tresult\tstring\t1\t1\nbytes\t58\n","stderr":""},{"case":4,"exit":0,"stdout":"code\t0\nobject\tpattern\tnone\t1\t1\nobject\tsubject\tnone\t1\t1\nobject\treplacement\tnone\t1\t1\nobject\tresult\tstring\t1\t1\nbytes\t616261\n","stderr":""},{"case":5,"exit":0,"stdout":"code\t0\nobject\tpattern\tnone\t1\t1\nobject\tsubject\tnone\t1\t1\nobject\treplacement\tnone\t1\t1\nobject\tresult\tstring\t1\t1\nbytes\t6161626161\n","stderr":""},{"case":6,"exit":0,"stdout":"code\t0\nobject\tpattern\tnone\t1\t1\nobject\tsubject\tnone\t1\t1\nobject\treplacement\tnone\t1\t1\nobject\tresult\tstring\t1\t1\nbytes\t586158625861\n","stderr":""},{"case":7,"exit":0,"stdout":"code\t0\nobject\tpattern\tnone\t1\t1\nobject\tsubject\tnone\t1\t1\nobject\treplacement\tnone\t1\t1\nobject\tresult\tstring\t1\t1\nobject\tindex\tindex\t1\t1\nbytes\t616258\n","stderr":""},{"case":8,"exit":0,"stdout":"code\t0\nobject\tpattern\tnone\t1\t1\nobject\tsubject\tnone\t1\t1\nobject\treplacement\tlist\t1\t1\nobject\tresult\tstring\t1\t1\nbytes\t416261\n","stderr":""}]. Process status is independent of printed command return and object rows.

## Source inspection

jim Jim0.84 captured source label; no captured patch query, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03 (current source inspection; full-file digest equals captured source)`, `jim-regexp.c`, function `Jim_RegsubCmd`, lines 452–483. Retained reference `exact-source-1`.

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
- [runtime/rust/src/cmd_regex.rs](../../../../runtime/rust/src/cmd_regex.rs), `cmd_regex::tests::jim_regsub_original_objects_match_all_nine_completed_native_controls` (linked): Checks its scoped original input/object/result windows against retained native controls. Adapter withdrawal after an aborted native attempt is a separate Rust contract; no executed Rust result is attached.

## Evidence

- [runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json](../../../../runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json)
- [runtime/rust/tests/data/native_regexp_jim_regsub/probe.c](../../../../runtime/rust/tests/data/native_regexp_jim_regsub/probe.c)
