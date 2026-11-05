# naming.list.original-stride-primitive-and-results

Kind: `native-observation`

## Problem statement

Which actual list commands accept -stride in C Tcl 8.4–9.1 and current Jim, which original scalar getter and minimum precede empty input conversion, and which Guest and public header fields result from counted, wide and arithmetic-looking operands? Textual parsing can choose the wrong primitive width or grammar, omit original cache effects, or lose complete errors.

## Question

For these exact 18 original counted stride words and empty input Lists per provider, which command table, Int/Wide/Jim Long getter, minimum and original public cache/result stage is reached?

## Conclusion

Stride has an independent scalar command purpose. Actual tables reject it in C 8.4/8.5 and C 8.6 search. C 8.6 sort uses Int, C 9 uses Wide, and Jim uses Long through its actual target ABI. Preserve original integer cache effects before primitive or minimum Guest errors. Counted NUL and 1+0 follow primitive syntax, rather than Index expressions. The public rows apply to these measured builds and empty input invocations.

## Scope

108 fresh case processes, 18 per selected library. All process exits are zero: 31 successful commands and 77 Guest errors, without skips. The original counted String has one argument-owner native reference and no additional observer hold; the List is genuinely empty. Before-command, reached-before-result-getter and materialised-result fields remain distinct. No private cache payload, callback/teardown chronology, temporary argv reference donation, universal platform or Long ABI, software assertion or BIG-IP outcome follows. CLI and independent same-library version queries run in separate processes. Linked Rust definitions are software coverage obligations. Their source assertions require independent execution receipts; they add no provider observation, feature availability, native object/frame or edit authority.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Fresh counted C-created original EvalObjv objects. Each stride word has one explicit argument-owner native reference and no additional observer hold. The input is a genuine empty native List. EvalObjv transient and error-stack references are not inferred. C uses Tcl_CreateInterp without Tcl_Init; Jim uses Jim_CreateInterp and Jim_RegisterCoreCommands. Version: tcl8.4 8.4.20.

Eighteen commands: zero successes and 18 Guest errors. Neither command accepts -stride. Every original counted stride retains a NULL primary and resident bytes. Exact unsupported-option result, code and header fields remain in each case.

### tcl8.5

Status: `observed`. Channel: Fresh counted C-created original EvalObjv objects. Each stride word has one explicit argument-owner native reference and no additional observer hold. The input is a genuine empty native List. EvalObjv transient and error-stack references are not inferred. C uses Tcl_CreateInterp without Tcl_Init; Jim uses Jim_CreateInterp and Jim_RegisterCoreCommands. Version: tcl8.5 8.5.19.

Eighteen commands: zero successes and 18 Guest errors. Neither actual command table accepts -stride. Every original stride retains a NULL primary. Exact failed-command fields are independently retained.

### tcl8.6

Status: `observed`. Channel: Fresh counted C-created original EvalObjv objects. Each stride word has one explicit argument-owner native reference and no additional observer hold. The input is a genuine empty native List. EvalObjv transient and error-stack references are not inferred. C uses Tcl_CreateInterp without Tcl_Init; Jim uses Jim_CreateInterp and Jim_RegisterCoreCommands. Version: tcl8.6 8.6.18.

Eighteen commands: one success and 17 Guest errors. Search rejects -stride before conversion. Sort reaches Int: 2 succeeds; 1, 0, -1 and counted 1-NUL-tail reject minimum 2. The measured 2147483648 and 4294967295 produce the minimum diagnostic. The measured 4294967296 receives an int primary before the integer-too-large Guest error. The word 1+0 remains uncached and fails integer conversion. These are empty-input public observations, without private cache payload claims.

### tcl9.0

Status: `observed`. Channel: Fresh counted C-created original EvalObjv objects. Each stride word has one explicit argument-owner native reference and no additional observer hold. The input is a genuine empty native List. EvalObjv transient and error-stack references are not inferred. C uses Tcl_CreateInterp without Tcl_Init; Jim uses Jim_CreateInterp and Jim_RegisterCoreCommands. Version: tcl9.0 9.0.4.

Eighteen commands: ten successes and eight Guest errors. Both commands reach Wide. Each of 2147483648, 4294967295 and 4294967296 succeeds against empty input and receives an int primary. Search accepts 1 and counted 1-NUL-tail; sort rejects minimum 2. Zero and -1 reject the respective minima. The word 1+0 is invalid and receives no int primary. Exact result and public fields remain separate captures.

### tcl9.1

Status: `observed`. Channel: Fresh counted C-created original EvalObjv objects. Each stride word has one explicit argument-owner native reference and no additional observer hold. The input is a genuine empty native List. EvalObjv transient and error-stack references are not inferred. C uses Tcl_CreateInterp without Tcl_Init; Jim uses Jim_CreateInterp and Jim_RegisterCoreCommands. Version: tcl9.1 9.1.0.

Eighteen commands: ten successes and eight Guest errors. The independently measured cases have their recorded Wide, minimum, counted-NUL and arithmetic-looking word outcomes. The finite matching outcomes do not establish broader equivalence between 9.0 and 9.1.

### jim

Status: `observed`. Channel: Fresh counted C-created original EvalObjv objects. Each stride word has one explicit argument-owner native reference and no additional observer hold. The input is a genuine empty native List. EvalObjv transient and error-stack references are not inferred. C uses Tcl_CreateInterp without Tcl_Init; Jim uses Jim_CreateInterp and Jim_RegisterCoreCommands. Version: jim 0.84-9-g5bac7c9.

Eighteen commands: ten successes and eight Guest errors. Both actual tables accept -stride. Jim_GetLong calls Jim_GetWide and casts to the actual native long. This measured build accepts all three large widths and caches an int on the original. Counted 1-NUL-tail receives an int: search succeeds and sort rejects minimum 2. The word 1+0 fails without an int primary. These measured native-long conversion outcomes do not prove a foreign long ABI.

### bigip

Status: `not-tested`. Channel: No BIG-IP original object driver capture.

Measured C Tcl and Jim objects do not establish a BIG-IP/TMM storage or execution protocol.

## Source inspection

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdIL.c`, function `Tcl_LsearchObjCmd option table`, lines 3192–3203. Line unit: physical LF only. Original byte range [92599, 93074). Retained reference `source-window-0`.

```text
    static CONST char *options[] = {
	"-all",	    "-ascii", "-decreasing", "-dictionary",
	"-exact",   "-glob",  "-increasing", "-inline",
	"-integer", "-not",   "-real",	     "-regexp",
	"-sorted",  "-start", NULL
    };
    enum options {
	LSEARCH_ALL, LSEARCH_ASCII, LSEARCH_DECREASING, LSEARCH_DICTIONARY,
	LSEARCH_EXACT, LSEARCH_GLOB, LSEARCH_INCREASING, LSEARCH_INLINE,
	LSEARCH_INTEGER, LSEARCH_NOT, LSEARCH_REAL, LSEARCH_REGEXP,
	LSEARCH_SORTED, LSEARCH_START
    };

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdIL.c`, function `Tcl_LsortObjCmd option table`, lines 3708–3711. Line unit: physical LF only. Original byte range [105819, 105989). Retained reference `source-window-1`.

```text
    static CONST char *switches[] = {
	"-ascii", "-command", "-decreasing", "-dictionary", "-increasing",
	"-index", "-integer", "-real", "-unique", (char *) NULL
    };

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdIL.c`, function `Tcl_LsearchObjCmd option table`, lines 2729–2742. Line unit: physical LF only. Original byte range [74586, 75159). Retained reference `source-window-2`.

```text
    static const char *options[] = {
	"-all",	    "-ascii",   "-decreasing", "-dictionary",
	"-exact",   "-glob",    "-increasing", "-index",
	"-inline",  "-integer", "-nocase",     "-not",
	"-real",    "-regexp",  "-sorted",     "-start",
	"-subindices", NULL
    };
    enum options {
	LSEARCH_ALL, LSEARCH_ASCII, LSEARCH_DECREASING, LSEARCH_DICTIONARY,
	LSEARCH_EXACT, LSEARCH_GLOB, LSEARCH_INCREASING, LSEARCH_INDEX,
	LSEARCH_INLINE, LSEARCH_INTEGER, LSEARCH_NOCASE, LSEARCH_NOT,
	LSEARCH_REAL, LSEARCH_REGEXP, LSEARCH_SORTED, LSEARCH_START,
	LSEARCH_SUBINDICES
    };

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdIL.c`, function `Tcl_LsortObjCmd option table`, lines 3475–3483. Line unit: physical LF only. Original byte range [92988, 93372). Retained reference `source-window-3`.

```text
    static const char *switches[] = {
	"-ascii", "-command", "-decreasing", "-dictionary", "-increasing",
	"-index", "-indices", "-integer", "-nocase", "-real", "-unique", NULL
    };
    enum Lsort_Switches {
	LSORT_ASCII, LSORT_COMMAND, LSORT_DECREASING, LSORT_DICTIONARY,
	LSORT_INCREASING, LSORT_INDEX, LSORT_INDICES, LSORT_INTEGER,
	LSORT_NOCASE, LSORT_REAL, LSORT_UNIQUE
    };

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdIL.c`, function `Tcl_LsearchObjCmd option table`, lines 2913–2926. Line unit: physical LF only. Original byte range [80184, 80790). Retained reference `source-window-4`.

```text
    static const char *const options[] = {
	"-all",	    "-ascii",   "-bisect", "-decreasing", "-dictionary",
	"-exact",   "-glob",    "-increasing", "-index",
	"-inline",  "-integer", "-nocase",     "-not",
	"-real",    "-regexp",  "-sorted",     "-start",
	"-subindices", NULL
    };
    enum options {
	LSEARCH_ALL, LSEARCH_ASCII, LSEARCH_BISECT, LSEARCH_DECREASING,
	LSEARCH_DICTIONARY, LSEARCH_EXACT, LSEARCH_GLOB, LSEARCH_INCREASING,
	LSEARCH_INDEX, LSEARCH_INLINE, LSEARCH_INTEGER, LSEARCH_NOCASE,
	LSEARCH_NOT, LSEARCH_REAL, LSEARCH_REGEXP, LSEARCH_SORTED,
	LSEARCH_START, LSEARCH_SUBINDICES
    };

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdIL.c`, function `Tcl_LsortObjCmd option table`, lines 3672–3681. Line unit: physical LF only. Original byte range [99253, 99669). Retained reference `source-window-5`.

```text
    static const char *const switches[] = {
	"-ascii", "-command", "-decreasing", "-dictionary", "-increasing",
	"-index", "-indices", "-integer", "-nocase", "-real", "-stride",
	"-unique", NULL
    };
    enum Lsort_Switches {
	LSORT_ASCII, LSORT_COMMAND, LSORT_DECREASING, LSORT_DICTIONARY,
	LSORT_INCREASING, LSORT_INDEX, LSORT_INDICES, LSORT_INTEGER,
	LSORT_NOCASE, LSORT_REAL, LSORT_STRIDE, LSORT_UNIQUE
    };

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdIL.c`, function `Tcl_LsortObjCmd original stride primitive and minimum`, lines 3803–3826. Line unit: physical LF only. Original byte range [102940, 103649). Retained reference `source-window-6`.

```text
	case LSORT_STRIDE:
	    if (i == objc-2) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"\"-stride\" option must be "
			"followed by stride length", -1));
		Tcl_SetErrorCode(interp, "TCL", "ARGUMENT", "MISSING", (char *)NULL);
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    if (Tcl_GetIntFromObj(interp, objv[i+1], &groupSize) != TCL_OK) {
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    if (groupSize < 2) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"stride length must be at least 2", -1));
		Tcl_SetErrorCode(interp, "TCL", "OPERATION", "LSORT",
			"BADSTRIDE", (char *)NULL);
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    group = 1;
	    i++;
	    break;

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdIL.c`, function `Tcl_LsearchObjCmd option table`, lines 3275–3288. Line unit: physical LF only. Original byte range [87984, 88624). Retained reference `source-window-7`.

```text
    static const char *const options[] = {
	"-all",	    "-ascii",   "-bisect", "-decreasing", "-dictionary",
	"-exact",   "-glob",    "-increasing", "-index",
	"-inline",  "-integer", "-nocase",     "-not",
	"-real",    "-regexp",  "-sorted",     "-start", "-stride",
	"-subindices", NULL
    };
    enum lsearchoptions {
	LSEARCH_ALL, LSEARCH_ASCII, LSEARCH_BISECT, LSEARCH_DECREASING,
	LSEARCH_DICTIONARY, LSEARCH_EXACT, LSEARCH_GLOB, LSEARCH_INCREASING,
	LSEARCH_INDEX, LSEARCH_INLINE, LSEARCH_INTEGER, LSEARCH_NOCASE,
	LSEARCH_NOT, LSEARCH_REAL, LSEARCH_REGEXP, LSEARCH_SORTED,
	LSEARCH_START, LSEARCH_STRIDE, LSEARCH_SUBINDICES
    };

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdIL.c`, function `Tcl_LsortObjCmd option table`, lines 4525–4534. Line unit: physical LF only. Original byte range [119424, 119846). Retained reference `source-window-8`.

```text
    static const char *const switches[] = {
	"-ascii", "-command", "-decreasing", "-dictionary", "-increasing",
	"-index", "-indices", "-integer", "-nocase", "-real", "-stride",
	"-unique", NULL
    };
    enum Lsort_Switches {
	LSORT_ASCII, LSORT_COMMAND, LSORT_DECREASING, LSORT_DICTIONARY,
	LSORT_INCREASING, LSORT_INDEX, LSORT_INDICES, LSORT_INTEGER,
	LSORT_NOCASE, LSORT_REAL, LSORT_STRIDE, LSORT_UNIQUE
    } index;

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdIL.c`, function `Tcl_LsearchObjCmd original stride primitive and minimum`, lines 3416–3439. Line unit: physical LF only. Original byte range [91755, 92443). Retained reference `source-window-9`.

```text
	case LSEARCH_STRIDE:		/* -stride */
	    if (i > objc-4) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"\"-stride\" option must be "
			"followed by stride length", -1));
		Tcl_SetErrorCode(interp, "TCL", "ARGUMENT", "MISSING", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    if (TclGetWideIntFromObj(interp, objv[i+1], &wide) != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (wide < 1) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"stride length must be at least 1", -1));
		Tcl_SetErrorCode(interp, "TCL", "OPERATION", "LSEARCH",
			"BADSTRIDE", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    groupSize = wide;
	    i++;
	    break;

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdIL.c`, function `Tcl_LsortObjCmd original stride primitive and minimum`, lines 4654–4678. Line unit: physical LF only. Original byte range [123046, 123771). Retained reference `source-window-10`.

```text
	case LSORT_STRIDE:
	    if (i == objc-2) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"\"-stride\" option must be "
			"followed by stride length", -1));
		Tcl_SetErrorCode(interp, "TCL", "ARGUMENT", "MISSING", (char *)NULL);
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    if (TclGetWideIntFromObj(interp, objv[i+1], &wide) != TCL_OK) {
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    if (wide < 2) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"stride length must be at least 2", -1));
		Tcl_SetErrorCode(interp, "TCL", "OPERATION", "LSORT",
			"BADSTRIDE", (char *)NULL);
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    groupSize = wide;
	    group = 1;
	    i++;
	    break;

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdIL.c`, function `Tcl_LsearchObjCmd option table`, lines 3124–3137. Line unit: physical LF only. Original byte range [84612, 85252). Retained reference `source-window-11`.

```text
    static const char *const options[] = {
	"-all",	    "-ascii",   "-bisect", "-decreasing", "-dictionary",
	"-exact",   "-glob",    "-increasing", "-index",
	"-inline",  "-integer", "-nocase",     "-not",
	"-real",    "-regexp",  "-sorted",     "-start", "-stride",
	"-subindices", NULL
    };
    enum lsearchoptions {
	LSEARCH_ALL, LSEARCH_ASCII, LSEARCH_BISECT, LSEARCH_DECREASING,
	LSEARCH_DICTIONARY, LSEARCH_EXACT, LSEARCH_GLOB, LSEARCH_INCREASING,
	LSEARCH_INDEX, LSEARCH_INLINE, LSEARCH_INTEGER, LSEARCH_NOCASE,
	LSEARCH_NOT, LSEARCH_REAL, LSEARCH_REGEXP, LSEARCH_SORTED,
	LSEARCH_START, LSEARCH_STRIDE, LSEARCH_SUBINDICES
    };

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdIL.c`, function `Tcl_LsortObjCmd option table`, lines 4372–4381. Line unit: physical LF only. Original byte range [116053, 116475). Retained reference `source-window-12`.

```text
    static const char *const switches[] = {
	"-ascii", "-command", "-decreasing", "-dictionary", "-increasing",
	"-index", "-indices", "-integer", "-nocase", "-real", "-stride",
	"-unique", NULL
    };
    enum Lsort_Switches {
	LSORT_ASCII, LSORT_COMMAND, LSORT_DECREASING, LSORT_DICTIONARY,
	LSORT_INCREASING, LSORT_INDEX, LSORT_INDICES, LSORT_INTEGER,
	LSORT_NOCASE, LSORT_REAL, LSORT_STRIDE, LSORT_UNIQUE
    } index;

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdIL.c`, function `Tcl_LsearchObjCmd original stride primitive and minimum`, lines 3265–3288. Line unit: physical LF only. Original byte range [88383, 89071). Retained reference `source-window-13`.

```text
	case LSEARCH_STRIDE:		/* -stride */
	    if (i > objc-4) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"\"-stride\" option must be "
			"followed by stride length", -1));
		Tcl_SetErrorCode(interp, "TCL", "ARGUMENT", "MISSING", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    if (TclGetWideIntFromObj(interp, objv[i+1], &wide) != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (wide < 1) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"stride length must be at least 1", -1));
		Tcl_SetErrorCode(interp, "TCL", "OPERATION", "LSEARCH",
			"BADSTRIDE", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    groupSize = wide;
	    i++;
	    break;

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdIL.c`, function `Tcl_LsortObjCmd original stride primitive and minimum`, lines 4501–4525. Line unit: physical LF only. Original byte range [119675, 120400). Retained reference `source-window-14`.

```text
	case LSORT_STRIDE:
	    if (i == objc-2) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"\"-stride\" option must be "
			"followed by stride length", -1));
		Tcl_SetErrorCode(interp, "TCL", "ARGUMENT", "MISSING", (char *)NULL);
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    if (TclGetWideIntFromObj(interp, objv[i+1], &wide) != TCL_OK) {
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    if (wide < 2) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"stride length must be at least 2", -1));
		Tcl_SetErrorCode(interp, "TCL", "OPERATION", "LSORT",
			"BADSTRIDE", (char *)NULL);
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    groupSize = wide;
	    group = 1;
	    i++;
	    break;

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_LsearchCoreCommand actual Enum table`, lines 13504–13507. Line unit: physical LF only. Original byte range [426934, 427116). Retained reference `source-window-15`.

```text
    static const char * const options[] = {
        "-bool", "-not", "-nocase", "-exact", "-glob", "-regexp", "-all", "-inline", "-command",
        "-stride", "-index", NULL
    };

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_LsearchCoreCommand actual original Long and minimum`, lines 13569–13580. Line unit: physical LF only. Original byte range [428886, 429335). Retained reference `source-window-16`.

```text
            case OPT_STRIDE:
                if (i >= argc - 2) {
                    return JIM_USAGE;
                }
                if (Jim_GetLong(interp, argv[++i], &stride) != JIM_OK) {
                    return JIM_ERR;
                }
                if (stride < 1) {
                    Jim_SetResultString(interp, "stride length must be at least 1", -1);
                    return JIM_ERR;
                }
                break;

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_LsortCoreCommand actual Enum table`, lines 13833–13836. Line unit: physical LF only. Original byte range [437055, 437258). Retained reference `source-window-17`.

```text
    static const char * const options[] = {
        "-ascii", "-nocase", "-increasing", "-decreasing", "-command", "-integer", "-real", "-index", "-unique",
        "-stride", "-dictionary", NULL
    };

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_LsortCoreCommand actual original Long and minimum`, lines 13898–13909. Line unit: physical LF only. Original byte range [439080, 439529). Retained reference `source-window-18`.

```text
            case OPT_STRIDE:
                if (i >= argc - 2) {
                    return JIM_USAGE;
                }
                if (Jim_GetLong(interp, argv[++i], &stride) != JIM_OK) {
                    return JIM_ERR;
                }
                if (stride < 2) {
                    Jim_SetResultString(interp, "stride length must be at least 2", -1);
                    return JIM_ERR;
                }
                break;

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_GetWide exact primitive`, lines 6346–6352. Line unit: physical LF only. Original byte range [196685, 196938). Retained reference `source-window-19`.

```text
int Jim_GetWide(Jim_Interp *interp, Jim_Obj *objPtr, jim_wide * widePtr)
{
    if (objPtr->typePtr != &intObjType && SetIntFromAny(interp, objPtr, JIM_ERRMSG) == JIM_ERR)
        return JIM_ERR;
    *widePtr = JimWideValue(objPtr);
    return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_GetLong exact primitive`, lines 6395–6406. Line unit: physical LF only. Original byte range [198391, 198671). Retained reference `source-window-20`.

```text
int Jim_GetLong(Jim_Interp *interp, Jim_Obj *objPtr, long *longPtr)
{
    jim_wide wideValue;
    int retval;

    retval = Jim_GetWide(interp, objPtr, &wideValue);
    if (retval == JIM_OK) {
        *longPtr = (long)wideValue;
        return JIM_OK;
    }
    return JIM_ERR;
}

```

## Consumer bindings

- [rust/tcl-registry/src/native_numeric_error/command.rs](../../../../rust/tcl-registry/src/native_numeric_error/command.rs), `original_stride_recipes_keep_actual_command_tables_primitive_widths_and_minima` (linked): Software source definition checks stride recipes keep actual command tables primitive widths and minima.
- [rust/tcl-vm/src/cmd_list/native_original_stride_controls.rs](../../../../rust/tcl-vm/src/cmd_list/native_original_stride_controls.rs), `original_stride_guest_and_header_predicates_match_native644` (linked): Software source definition checks stride guest and header predicates match native644.
- [runtime/rust/src/cmd_list/native_original_stride_controls.rs](../../../../runtime/rust/src/cmd_list/native_original_stride_controls.rs), `original_stride_guest_and_header_predicates_match_native644` (linked): Software source definition checks stride guest and header predicates match native644.

## Evidence

- [runtime/rust/tests/data/native_original_list_stride644](../../../../runtime/rust/tests/data/native_original_list_stride644)
- [runtime/rust/tests/data/native_original_list_stride644/8.4.20/same-library-version](../../../../runtime/rust/tests/data/native_original_list_stride644/8.4.20/same-library-version)
- [runtime/rust/tests/data/native_original_list_stride644/8.5.19/same-library-version](../../../../runtime/rust/tests/data/native_original_list_stride644/8.5.19/same-library-version)
- [runtime/rust/tests/data/native_original_list_stride644/8.6.18/same-library-version](../../../../runtime/rust/tests/data/native_original_list_stride644/8.6.18/same-library-version)
- [runtime/rust/tests/data/native_original_list_stride644/9.0.4/same-library-version](../../../../runtime/rust/tests/data/native_original_list_stride644/9.0.4/same-library-version)
- [runtime/rust/tests/data/native_original_list_stride644/9.1.0/same-library-version](../../../../runtime/rust/tests/data/native_original_list_stride644/9.1.0/same-library-version)
- [runtime/rust/tests/data/native_original_list_stride644/Jim/same-library-version](../../../../runtime/rust/tests/data/native_original_list_stride644/Jim/same-library-version)
