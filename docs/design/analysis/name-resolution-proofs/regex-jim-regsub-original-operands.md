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

Original selected native attempts: [{"case":0,"exit":0,"stdout":"code\t0\nobject\tpattern\tnone\t1\t1\nobject\tsubject\tnone\t1\t1\nobject\treplacement\tnone\t1\t1\nobject\tresult\tstring\t1\t1\nbytes\t586258\n","stderr":""},{"case":1,"exit":0,"stdout":"code\t0\nobject\tpattern\tnone\t1\t1\nobject\tsubject\tnone\t1\t1\nobject\treplacement\tnone\t1\t1\nobject\tresult\tstring\t1\t1\nbytes\t58\n","stderr":""},{"case":2,"exit":0,"stdout":"code\t0\nobject\tpattern\tnone\t1\t1\nobject\tsubject\tnone\t1\t1\nobject\treplacement\tnone\t1\t1\nobject\tresult\tstring\t1\t1\nbytes\t58\n","stderr":""},{"case":3,"exit":0,"stdout":"code\t0\nobject\tpattern\tnone\t1\t1\nobject\tsubject\tnone\t1\t1\nobject\treplacement\tnone\t1\t1\nobject\tresult\tstring\t1\t1\nbytes\t58\n","stderr":""},{"case":4,"exit":0,"stdout":"code\t0\nobject\tpattern\tnone\t1\t1\nobject\tsubject\tnone\t1\t1\nobject\treplacement\tnone\t1\t1\nobject\tresult\tstring\t1\t1\nbytes\t616261\n","stderr":""},{"case":5,"exit":0,"stdout":"code\t0\nobject\tpattern\tnone\t1\t1\nobject\tsubject\tnone\t1\t1\nobject\treplacement\tnone\t1\t1\nobject\tresult\tstring\t1\t1\nbytes\t6161626161\n","stderr":""},{"case":6,"exit":0,"stdout":"code\t0\nobject\tpattern\tnone\t1\t1\nobject\tsubject\tnone\t1\t1\nobject\treplacement\tnone\t1\t1\nobject\tresult\tstring\t1\t1\nbytes\t586158625861\n","stderr":""},{"case":7,"exit":0,"stdout":"code\t0\nobject\tpattern\tnone\t1\t1\nobject\tsubject\tnone\t1\t1\nobject\treplacement\tnone\t1\t1\nobject\tresult\tstring\t1\t1\nobject\tindex\tindex\t1\t1\nbytes\t616258\n","stderr":""},{"case":8,"exit":0,"stdout":"code\t0\nobject\tpattern\tnone\t1\t1\nobject\tsubject\tnone\t1\t1\nobject\treplacement\tlist\t1\t1\nobject\tresult\tstring\t1\t1\nbytes\t416261\n","stderr":""}]. Process status is independent of printed command return and object rows.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

## Exact evidence

- `original-capture` (provider): [runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json](../../../../runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json). SHA-256 `6f5e36c0589ba3c4e5b40e21f880bdd5afaba2c54c488f03e7deedc01079ee8e`. Original source/library/header/build context and every process status/stream, including incomplete attempts.
- `original-probe` (input): [runtime/rust/tests/data/native_regexp_jim_regsub/probe.c](../../../../runtime/rust/tests/data/native_regexp_jim_regsub/probe.c). SHA-256 `b427c8677cb8dcf5d7343c9cda1bae6d93e492fbe964a5c2de49c88b8fa5a803`. Exact direct Jim worker invocation, object construction and physical reporting order.
- `jim-case-0` (observation): [runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json](../../../../runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json). SHA-256 `6f5e36c0589ba3c4e5b40e21f880bdd5afaba2c54c488f03e7deedc01079ee8e`. JSON pointer `/captures/0`. Exact original process status and raw output/error of this case.
- `jim-case-1` (observation): [runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json](../../../../runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json). SHA-256 `6f5e36c0589ba3c4e5b40e21f880bdd5afaba2c54c488f03e7deedc01079ee8e`. JSON pointer `/captures/1`. Exact original process status and raw output/error of this case.
- `jim-case-2` (observation): [runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json](../../../../runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json). SHA-256 `6f5e36c0589ba3c4e5b40e21f880bdd5afaba2c54c488f03e7deedc01079ee8e`. JSON pointer `/captures/2`. Exact original process status and raw output/error of this case.
- `jim-case-3` (observation): [runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json](../../../../runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json). SHA-256 `6f5e36c0589ba3c4e5b40e21f880bdd5afaba2c54c488f03e7deedc01079ee8e`. JSON pointer `/captures/3`. Exact original process status and raw output/error of this case.
- `jim-case-4` (observation): [runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json](../../../../runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json). SHA-256 `6f5e36c0589ba3c4e5b40e21f880bdd5afaba2c54c488f03e7deedc01079ee8e`. JSON pointer `/captures/4`. Exact original process status and raw output/error of this case.
- `jim-case-5` (observation): [runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json](../../../../runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json). SHA-256 `6f5e36c0589ba3c4e5b40e21f880bdd5afaba2c54c488f03e7deedc01079ee8e`. JSON pointer `/captures/5`. Exact original process status and raw output/error of this case.
- `jim-case-6` (observation): [runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json](../../../../runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json). SHA-256 `6f5e36c0589ba3c4e5b40e21f880bdd5afaba2c54c488f03e7deedc01079ee8e`. JSON pointer `/captures/6`. Exact original process status and raw output/error of this case.
- `jim-case-7` (observation): [runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json](../../../../runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json). SHA-256 `6f5e36c0589ba3c4e5b40e21f880bdd5afaba2c54c488f03e7deedc01079ee8e`. JSON pointer `/captures/7`. Exact original process status and raw output/error of this case.
- `jim-case-8` (observation): [runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json](../../../../runtime/rust/tests/data/native_regexp_jim_regsub/manifest.json). SHA-256 `6f5e36c0589ba3c4e5b40e21f880bdd5afaba2c54c488f03e7deedc01079ee8e`. JSON pointer `/captures/8`. Exact original process status and raw output/error of this case.
- `exact-source-1` (source-anchor): [docs/design/analysis/name-resolution-proofs/regexp-retained-source-anchors.json](../../../../docs/design/analysis/name-resolution-proofs/regexp-retained-source-anchors.json). SHA-256 `f36a41d7658d9b17accf0b4bf93dc17a1c11008bd8d1261ff9c10ce83f58678c`. JSON pointer `/1/snippet`. Exact current source excerpt whose complete source SHA equals the original captured source; independent of interpreter output and build revision query.

## Source inspection

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
- [runtime/rust/src/cmd_regex.rs](../../../../runtime/rust/src/cmd_regex.rs), `cmd_regex::tests::jim_regsub_original_objects_match_all_nine_completed_native_controls` (linked): Checks its scoped original input/object/result windows against retained native controls. Adapter withdrawal after an aborted native attempt is a separate Rust contract; no executed Rust result is attached.

A named test is a coverage binding, not a claim that it executed.

## Replay

No replay runner or new execution is asserted. Rebuild the retained probe only with independently verified matching source/header/library identities, run each selected case in a fresh process and preserve stdout, stderr and process status separately. Original abnormal exits are retained observations of incomplete attempts; they are not successful completions and are not requested to be rerun.
