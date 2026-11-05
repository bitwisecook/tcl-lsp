# naming.list.original-numeric-pattern-key-getters-and-results

Kind: `native-observation`

## Problem statement

Which primitive owner must numeric lsearch patterns and reached keys, and lsort keys, use to preserve original typed values, dialect widths, cache effects, early exits and complete Guest results? Byte parsing can select the wrong grammar, materialise String too early, omit original caches or reach keys the selected producer never reads. The search mode determines whether numeric conversion occurs at all.

## Question

For these original counted/typed32 inputs per provider, what do independent defaultGlob645 and explicitExact/Sorted647 invocations expose before command, at reached result, and after result materialisation?

## Conclusion

Numeric conversion is a separate selected original scalar purpose. C84/85 Exact/Sorted search uses Int; C86/9 uses Wide. C84/85 sort uses Long and C86/9 Wide; real uses Double. Default Glob search does not select those numeric getters. The measured zero/one-member numeric sorts in C84 and Jim avoid key conversion; C85+ prepares numeric keys. Jim numeric search options are unavailable, while numeric sort uses original Wide/Double and genuine local qsort. Public cache/result observations and pinned producer source are distinct proof bounds.

## Scope

384 fresh original processes,192 per independent series, all compile/CLI/case exits0 and all input pins unchanged. Command Guest results are recorded independently from process exit. All24 original probe/version ELFs and12 same-static-library diagnostics retained. No observer child Native hold; true typed/C-created original roles retained. Original public descriptor/residency/length/reference observations stay at their captured points. Private cache payloads, callback/teardown chronology, transient native references, universal platform/ABI, software assertion and BIG-IP grants absent. Linked Rust definitions are software coverage obligations. Their source assertions require independent execution receipts; they add no provider observation, feature availability, native object/frame or edit authority.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Two independent32-case fresh EvalObjv matrices. Counted String or genuine typed Wide/Double children have sole containing List holds and no observer refs; original argv each callerhold1. C Tcl_CreateInterp without Tcl_Init; Jim core registered. Public result snapshots before and after materialisation remain separate. Version: tcl8.4 8.4.20.

Default Glob645 searches leave numeric pattern/key caches untouched except explicit Sorted case9. Explicit Exact647 converts Int pattern before reached keys, including empty input; 4294967296 receives int before integer-too-large Guest. Counted NUL integer/real words reject without numeric cache. Long/Double sort lazily compares, accepts invalid or NaN singleton without getter, and accepts the measured NaN two-member case. Original typed wide constructor exposes wideInt, separately from parsed int. Source independently holds every SortElement member before its bucket merge; public snapshots do not measure those temporary references.

### tcl8.5

Status: `observed`. Channel: Two independent32-case fresh EvalObjv matrices. Counted String or genuine typed Wide/Double children have sole containing List holds and no observer refs; original argv each callerhold1. C Tcl_CreateInterp without Tcl_Init; Jim core registered. Public result snapshots before and after materialisation remain separate. Version: tcl8.5 8.5.19.

Default Glob645 is independently retained as nonnumeric search control. Exact647 uses Int pattern and keys; the measured 4294967296 reaches int before integer-too-large Guest. Counted-NUL integer/real words are accepted. Long/Double sort prepares numeric keys in input order even for singleton; NaN produces the recorded Guest with double primary. First/second invalid keys show only recorded reached cache fields, not internal callback chronology.

### tcl8.6

Status: `observed`. Channel: Two independent32-case fresh EvalObjv matrices. Counted String or genuine typed Wide/Double children have sole containing List holds and no observer refs; original argv each callerhold1. C Tcl_CreateInterp without Tcl_Init; Jim core registered. Public result snapshots before and after materialisation remain separate. Version: tcl8.6 8.6.18.

Exact647 uses Wide pattern/key purposes and accepts the measured outside-Int-width4294967296 inputs. Default Glob645 leaves numeric search caches untouched except Sorted. Sort prepares Wide/Double originals in input order, accepts counted NUL primitives and rejects NaN with recorded double primary. Invalid second key preserves the recorded numeric first key cache; no private cache value is inferred.

### tcl9.0

Status: `observed`. Channel: Two independent32-case fresh EvalObjv matrices. Counted String or genuine typed Wide/Double children have sole containing List holds and no observer refs; original argv each callerhold1. C Tcl_CreateInterp without Tcl_Init; Jim core registered. Public result snapshots before and after materialisation remain separate. Version: tcl9.0 9.0.4.

Independent Exact647 and defaultGlob645 captures distinguish selected primitive search from String matching. Wide/Double search and sort have the recorded counted-NUL, width, early-match/all and NaN Guest/cache/result fields. Numeric typed Double remains without String residency before result materialisation. These finite observations do not establish all private storage or lifecycle behaviour.

### tcl9.1

Status: `observed`. Channel: Two independent32-case fresh EvalObjv matrices. Counted String or genuine typed Wide/Double children have sole containing List holds and no observer refs; original argv each callerhold1. C Tcl_CreateInterp without Tcl_Init; Jim core registered. Public result snapshots before and after materialisation remain separate. Version: tcl9.1 9.1.0.

Both independently captured32-case series have their original Guest/result/header fields. Exact/Sorted uses Wide/Double, default Glob does not choose numeric conversion. Counted-NUL primitives accept, early match avoids the invalid second key while all fails there, and NaN has recorded rejection/cache fields. Matching finite9.0/9.1 outcomes do not establish general equivalence.

### jim

Status: `observed`. Channel: Two independent32-case fresh EvalObjv matrices. Counted String or genuine typed Wide/Double children have sole containing List holds and no observer refs; original argv each callerhold1. C Tcl_CreateInterp without Tcl_Init; Jim core registered. Public result snapshots before and after materialisation remain separate. Version: jim 0.84-9-g5bac7c9.

Both series reject integer/real lsearch options before numeric conversion. Numeric lsort accepts invalid/NaN singleton with original alias and no key getter; nonempty pairs reach original Wide/Double primitives. Counted-NUL values succeed; NaN pair is accepted and typed NaN String spelling is NaN. Source selects actual qsort and signed-wide subtraction; finite captured values do not prove overflow behaviour, libc callback chronology or a foreign ABI.

### bigip

Status: `not-tested`. Channel: No BIG-IP original-object process for these inputs.

These C Tcl/Jim measured original objects do not establish a BIG-IP/TMM scalar, reference or sort producer.

## Source inspection

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdIL.c`, function `Tcl_LsearchObjCmd: default table, selected original pattern and reached numeric keys`, lines 3179–3593. Line unit: physical LF only. Original byte range [92032, 102811). Retained reference `source-window-0`.

```text
Tcl_LsearchObjCmd(clientData, interp, objc, objv)
    ClientData clientData;	/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument values. */
{
    char *bytes, *patternBytes;
    int i, match, mode, index, result, listc, length, elemLen;
    int dataType, isIncreasing, lower, upper, patInt, objInt;
    int offset, allMatches, inlineReturn, negatedMatch;
    double patDouble, objDouble;
    Tcl_Obj *patObj, **listv, *listPtr, *startPtr;
    Tcl_RegExp regexp = NULL;
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
    enum datatypes {
	ASCII, DICTIONARY, INTEGER, REAL
    };
    enum modes {
	EXACT, GLOB, REGEXP, SORTED
    };

    mode = GLOB;
    dataType = ASCII;
    isIncreasing = 1;
    allMatches = 0;
    inlineReturn = 0;
    negatedMatch = 0;
    listPtr = NULL;
    startPtr = NULL;
    offset = 0;

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "?options? list pattern");
	return TCL_ERROR;
    }

    for (i = 1; i < objc-2; i++) {
	if (Tcl_GetIndexFromObj(interp, objv[i], options, "option", 0, &index)
		!= TCL_OK) {
	    if (startPtr) {
		Tcl_DecrRefCount(startPtr);
	    }
	    return TCL_ERROR;
	}
	switch ((enum options) index) {
	case LSEARCH_ALL:		/* -all */
	    allMatches = 1;
	    break;
	case LSEARCH_ASCII:		/* -ascii */
	    dataType = ASCII;
	    break;
	case LSEARCH_DECREASING:	/* -decreasing */
	    isIncreasing = 0;
	    break;
	case LSEARCH_DICTIONARY:	/* -dictionary */
	    dataType = DICTIONARY;
	    break;
	case LSEARCH_EXACT:		/* -increasing */
	    mode = EXACT;
	    break;
	case LSEARCH_GLOB:		/* -glob */
	    mode = GLOB;
	    break;
	case LSEARCH_INCREASING:	/* -increasing */
	    isIncreasing = 1;
	    break;
	case LSEARCH_INLINE:		/* -inline */
	    inlineReturn = 1;
	    break;
	case LSEARCH_INTEGER:		/* -integer */
	    dataType = INTEGER;
	    break;
	case LSEARCH_NOT:		/* -not */
	    negatedMatch = 1;
	    break;
	case LSEARCH_REAL:		/* -real */
	    dataType = REAL;
	    break;
	case LSEARCH_REGEXP:		/* -regexp */
	    mode = REGEXP;
	    break;
	case LSEARCH_SORTED:		/* -sorted */
	    mode = SORTED;
	    break;
	case LSEARCH_START:		/* -start */
	    /*
	     * If there was a previous -start option, release its saved
	     * index because it will either be replaced or there will be
	     * an error.
	     */
	    if (startPtr) {
		Tcl_DecrRefCount(startPtr);
	    }
	    if (i > objc-4) {
		Tcl_AppendResult(interp, "missing starting index", NULL);
		return TCL_ERROR;
	    }
	    i++;
	    if (objv[i] == objv[objc - 2]) {
		/*
		 * Take copy to prevent shimmering problems.  Note
		 * that it does not matter if the index obj is also a
		 * component of the list being searched.  We only need
		 * to copy where the list and the index are
		 * one-and-the-same.
		 */
		startPtr = Tcl_DuplicateObj(objv[i]);
	    } else {
		startPtr = objv[i];
		Tcl_IncrRefCount(startPtr);
	    }
	}
    }

    if ((enum modes) mode == REGEXP) {
	/*
	 * We can shimmer regexp/list if listv[i] == pattern, so get the
	 * regexp rep before the list rep. First time round, omit the interp
         * and hope that the compilation will succeed. If it fails, we'll
         * recompile in "expensive" mode with a place to put error messages.
	 */

	regexp = Tcl_GetRegExpFromObj(NULL, objv[objc - 1],
		TCL_REG_ADVANCED | TCL_REG_NOSUB);
	if (regexp == NULL) {
            /*
             * Failed to compile the RE. Try again without the TCL_REG_NOSUB
             * flag in case the RE had sub-expressions in it [Bug 1366683].
             * If this fails, an error message will be left in the
             * interpreter.
             */

            regexp = Tcl_GetRegExpFromObj(interp, objv[objc - 1],
		    TCL_REG_ADVANCED);
	}

	if (regexp == NULL) {
	    if (startPtr) {
		Tcl_DecrRefCount(startPtr);
	    }
	    return TCL_ERROR;
	}
    }

    /*
     * Make sure the list argument is a list object and get its length and
     * a pointer to its array of element pointers.
     */

    result = Tcl_ListObjGetElements(interp, objv[objc - 2], &listc, &listv);
    if (result != TCL_OK) {
	if (startPtr) {
	    Tcl_DecrRefCount(startPtr);
	}
	return result;
    }

    /*
     * Get the user-specified start offset.
     */
    if (startPtr) {
	result = TclGetIntForIndex(interp, startPtr, listc-1, &offset);
	Tcl_DecrRefCount(startPtr);
	if (result != TCL_OK) {
	    return result;
	}

	/*
	 * If the search started past the end of the list, we just return a
	 * "did not match anything at all" result straight away. [Bug 1374778]
	 */

	if (offset > listc-1) {
	    if (allMatches || inlineReturn) {
		Tcl_ResetResult(interp);
	    } else {
		Tcl_SetObjResult(interp, Tcl_NewIntObj(-1));
	    }
	    return TCL_OK;
	}
	if (offset < 0) {
	    offset = 0;
	}
    }

    patObj = objv[objc - 1];
    patternBytes = NULL;
    if ((enum modes) mode == EXACT || (enum modes) mode == SORTED) {
	switch ((enum datatypes) dataType) {
	case ASCII:
	case DICTIONARY:
	    patternBytes = Tcl_GetStringFromObj(patObj, &length);
	    break;
	case INTEGER:
	    result = Tcl_GetIntFromObj(interp, patObj, &patInt);
	    if (result != TCL_OK) {
		return result;
	    }
	    Tcl_ListObjGetElements(NULL, objv[objc - 2], &listc, &listv);
	    break;
	case REAL:
	    result = Tcl_GetDoubleFromObj(interp, patObj, &patDouble);
	    if (result != TCL_OK) {
		return result;
	    }
	    Tcl_ListObjGetElements(NULL, objv[objc - 2], &listc, &listv);
	    break;
	}
    } else {
	patternBytes = Tcl_GetStringFromObj(patObj, &length);
    }

    /*
     * Set default index value to -1, indicating failure; if we find the
     * item in the course of our search, index will be set to the correct
     * value.
     */
    index = -1;
    match = 0;

    if ((enum modes) mode == SORTED && !allMatches && !negatedMatch) {
	/*
	 * If the data is sorted, we can do a more intelligent search.
	 * Note that there is no point in being smart when -all was
	 * specified; in that case, we have to look at all items anyway,
	 * and there is no sense in doing this when the match sense is
	 * inverted.
	 */
	lower = offset - 1;
	upper = listc;
	while (lower + 1 != upper) {
	    i = (lower + upper)/2;
	    switch ((enum datatypes) dataType) {
	    case ASCII:
		bytes = Tcl_GetString(listv[i]);
		match = strcmp(patternBytes, bytes);
		break;
	    case DICTIONARY:
		bytes = Tcl_GetString(listv[i]);
		match = DictionaryCompare(patternBytes, bytes);
		break;
	    case INTEGER:
		result = Tcl_GetIntFromObj(interp, listv[i], &objInt);
		if (result != TCL_OK) {
		    return result;
		}
		if (patInt == objInt) {
		    match = 0;
		} else if (patInt < objInt) {
		    match = -1;
		} else {
		    match = 1;
		}
		break;
	    case REAL:
		result = Tcl_GetDoubleFromObj(interp, listv[i], &objDouble);
		if (result != TCL_OK) {
		    return result;
		}
		if (patDouble == objDouble) {
		    match = 0;
		} else if (patDouble < objDouble) {
		    match = -1;
		} else {
		    match = 1;
		}
		break;
	    }
	    if (match == 0) {
		/*
		 * Normally, binary search is written to stop when it
		 * finds a match.  If there are duplicates of an element in
		 * the list, our first match might not be the first occurance.
		 * Consider:  0 0 0 1 1 1 2 2 2
		 * To maintain consistancy with standard lsearch semantics,
		 * we must find the leftmost occurance of the pattern in the
		 * list.  Thus we don't just stop searching here.  This
		 * variation means that a search always makes log n
		 * comparisons (normal binary search might "get lucky" with
		 * an early comparison).
		 */
		index = i;
		upper = i;
	    } else if (match > 0) {
		if (isIncreasing) {
		    lower = i;
		} else {
		    upper = i;
		}
	    } else {
		if (isIncreasing) {
		    upper = i;
		} else {
		    lower = i;
		}
	    }
	}

    } else {
	/*
	 * We need to do a linear search, because (at least one) of:
	 *   - our matcher can only tell equal vs. not equal
	 *   - our matching sense is negated
	 *   - we're building a list of all matched items
	 */
	if (allMatches) {
	    listPtr = Tcl_NewListObj(0, (Tcl_Obj **) NULL);
	}
	for (i = offset; i < listc; i++) {
	    match = 0;
	    switch ((enum modes) mode) {
	    case SORTED:
	    case EXACT:
		switch ((enum datatypes) dataType) {
		case ASCII:
		    bytes = Tcl_GetStringFromObj(listv[i], &elemLen);
		    if (length == elemLen) {
			match = (memcmp(bytes, patternBytes,
				(size_t) length) == 0);
		    }
		    break;
		case DICTIONARY:
		    bytes = Tcl_GetString(listv[i]);
		    match = (DictionaryCompare(bytes, patternBytes) == 0);
		    break;
		case INTEGER:
		    result = Tcl_GetIntFromObj(interp, listv[i], &objInt);
		    if (result != TCL_OK) {
			if (listPtr) {
			    Tcl_DecrRefCount(listPtr);
			}
			return result;
		    }
		    match = (objInt == patInt);
		    break;
		case REAL:
		    result = Tcl_GetDoubleFromObj(interp, listv[i],
			    &objDouble);
		    if (result != TCL_OK) {
			if (listPtr) {
			    Tcl_DecrRefCount(listPtr);
			}
			return result;
		    }
		    match = (objDouble == patDouble);
		    break;
		}
		break;
	    case GLOB:
		match = Tcl_StringMatch(Tcl_GetString(listv[i]),
			patternBytes);
		break;
	    case REGEXP:
		match = Tcl_RegExpExecObj(interp, regexp, listv[i], 0, 0, 0);
		if (match < 0) {
		    Tcl_DecrRefCount(patObj);
		    if (listPtr) {
			Tcl_DecrRefCount(listPtr);
		    }
		    return TCL_ERROR;
		}
		break;
	    }
	    /*
	     * Invert match condition for -not
	     */
	    if (negatedMatch) {
		match = !match;
	    }
	    if (match != 0) {
		if (!allMatches) {
		    index = i;
		    break;
		} else if (inlineReturn) {
		    /*
		     * Note that these appends are not expected to fail.
		     */
		    Tcl_ListObjAppendElement(interp, listPtr, listv[i]);
		} else {
		    Tcl_ListObjAppendElement(interp, listPtr,
			    Tcl_NewIntObj(i));
		}
	    }
	}
    }

    /*
     * Return everything or a single value.
     */
    if (allMatches) {
	Tcl_SetObjResult(interp, listPtr);
    } else if (!inlineReturn) {
	Tcl_SetIntObj(Tcl_GetObjResult(interp), index);
    } else if (index < 0) {
	/*
	 * Is this superfluous?  The result should be a blank object
	 * by default...
	 */
	Tcl_SetObjResult(interp, Tcl_NewObj());
    } else {
	Tcl_SetObjResult(interp, listv[index]);
    }
    return TCL_OK;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdIL.c`, function `Tcl_LsortObjCmd: selected List ingress, all-member native SortElement holds and cleanup`, lines 3693–3860. Line unit: physical LF only. Original byte range [105199, 110020). Retained reference `source-window-1`.

```text
Tcl_LsortObjCmd(clientData, interp, objc, objv)
    ClientData clientData;	/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument values. */
{
    int i, index, unique;
    Tcl_Obj *resultPtr;
    int length;
    Tcl_Obj *cmdPtr, **listObjPtrs;
    SortElement *elementArray;
    SortElement *elementPtr;        
    SortInfo sortInfo;                  /* Information about this sort that
                                         * needs to be passed to the 
                                         * comparison function */
    static CONST char *switches[] = {
	"-ascii", "-command", "-decreasing", "-dictionary", "-increasing",
	"-index", "-integer", "-real", "-unique", (char *) NULL
    };

    resultPtr = Tcl_GetObjResult(interp);
    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "?options? list");
	return TCL_ERROR;
    }

    /*
     * Parse arguments to set up the mode for the sort.
     */

    sortInfo.isIncreasing = 1;
    sortInfo.sortMode = SORTMODE_ASCII;
    sortInfo.index = SORTIDX_NONE;
    sortInfo.interp = interp;
    sortInfo.resultCode = TCL_OK;
    cmdPtr = NULL;
    unique = 0;
    for (i = 1; i < objc-1; i++) {
	if (Tcl_GetIndexFromObj(interp, objv[i], switches, "option", 0, &index)
		!= TCL_OK) {
	    return TCL_ERROR;
	}
	switch (index) {
	    case 0:			/* -ascii */
		sortInfo.sortMode = SORTMODE_ASCII;
		break;
	    case 1:			/* -command */
		if (i == (objc-2)) {
		    Tcl_AppendToObj(resultPtr,
			    "\"-command\" option must be followed by comparison command",
			    -1);
		    return TCL_ERROR;
		}
		sortInfo.sortMode = SORTMODE_COMMAND;
		cmdPtr = objv[i+1];
		i++;
		break;
	    case 2:			/* -decreasing */
		sortInfo.isIncreasing = 0;
		break;
	    case 3:			/* -dictionary */
		sortInfo.sortMode = SORTMODE_DICTIONARY;
		break;
	    case 4:			/* -increasing */
		sortInfo.isIncreasing = 1;
		break;
	    case 5:			/* -index */
		if (i == (objc-2)) {
		    Tcl_AppendToObj(resultPtr,
			    "\"-index\" option must be followed by list index",
			    -1);
		    return TCL_ERROR;
		}
		if (TclGetIntForIndex(interp, objv[i+1], SORTIDX_END,
			&sortInfo.index) != TCL_OK) {
		    return TCL_ERROR;
		}
		i++;
		break;
	    case 6:			/* -integer */
		sortInfo.sortMode = SORTMODE_INTEGER;
		break;
	    case 7:			/* -real */
		sortInfo.sortMode = SORTMODE_REAL;
		break;
	    case 8:			/* -unique */
		unique = 1;
		break;
	}
    }
    if (sortInfo.sortMode == SORTMODE_COMMAND) {
	/*
	 * The existing command is a list. We want to flatten it, append
	 * two dummy arguments on the end, and replace these arguments
	 * later.
	 */

        Tcl_Obj *newCommandPtr = Tcl_DuplicateObj(cmdPtr);
	Tcl_Obj *newObjPtr = Tcl_NewObj();

	Tcl_IncrRefCount(newCommandPtr);
	if (Tcl_ListObjAppendElement(interp, newCommandPtr, newObjPtr)
		!= TCL_OK) {
	    Tcl_DecrRefCount(newCommandPtr);
	    Tcl_IncrRefCount(newObjPtr);
	    Tcl_DecrRefCount(newObjPtr);
	    return TCL_ERROR;
	}
	Tcl_ListObjAppendElement(interp, newCommandPtr, Tcl_NewObj());
	sortInfo.compareCmdPtr = newCommandPtr;
    }

    sortInfo.resultCode = Tcl_ListObjGetElements(interp, objv[objc-1],
	    &length, &listObjPtrs);
    if (sortInfo.resultCode != TCL_OK || length <= 0) {
	goto done;
    }
    elementArray = (SortElement *) ckalloc(length * sizeof(SortElement));
    for (i=0; i < length; i++){
	elementArray[i].objPtr = listObjPtrs[i];
	elementArray[i].count = 0;
	elementArray[i].nextPtr = &elementArray[i+1];

	/*
	 * When sorting using a command, we are reentrant and therefore might
	 * have the representation of the list being sorted shimmered out from
	 * underneath our feet. Increment the reference counts of the elements
	 * to sort to prevent this. [Bug 1675116]
	 */

	Tcl_IncrRefCount(elementArray[i].objPtr);
    }
    elementArray[length-1].nextPtr = NULL;
    elementPtr = MergeSort(elementArray, &sortInfo);
    if (sortInfo.resultCode == TCL_OK) {
	/*
	 * Note: must clear the interpreter's result object: it could
	 * have been set by the -command script.
	 */

	Tcl_ResetResult(interp);
	resultPtr = Tcl_GetObjResult(interp);
	if (unique) {
	    for (; elementPtr != NULL; elementPtr = elementPtr->nextPtr){
		if (elementPtr->count == 0) {
		    Tcl_ListObjAppendElement(interp, resultPtr,
			    elementPtr->objPtr);
		}
	    }
	} else {
	    for (; elementPtr != NULL; elementPtr = elementPtr->nextPtr){
		Tcl_ListObjAppendElement(interp, resultPtr,
			elementPtr->objPtr);
	    }
	}
    }
    for (i=0; i<length; i++) {
	Tcl_DecrRefCount(elementArray[i].objPtr);
    }
    ckfree((char*) elementArray);

    done:
    if (sortInfo.sortMode == SORTMODE_COMMAND) {
	Tcl_DecrRefCount(sortInfo.compareCmdPtr);
	sortInfo.compareCmdPtr = NULL;
    }
    return sortInfo.resultCode;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdIL.c`, function `SortCompare: reached original Long/Double keys`, lines 4014–4164. Line unit: physical LF only. Original byte range [114054, 118197). Retained reference `source-window-2`.

```text
SortCompare(objPtr1, objPtr2, infoPtr)
    Tcl_Obj *objPtr1, *objPtr2;		/* Values to be compared. */
    SortInfo *infoPtr;                  /* Information passed from the
                                         * top-level "lsort" command */
{
    int order, listLen, index;
    Tcl_Obj *objPtr;
    char buffer[TCL_INTEGER_SPACE];

    order = 0;
    if (infoPtr->resultCode != TCL_OK) {
	/*
	 * Once an error has occurred, skip any future comparisons
	 * so as to preserve the error message in sortInterp->result.
	 */

	return order;
    }
    if (infoPtr->index != SORTIDX_NONE) {
	/*
	 * The "-index" option was specified.  Treat each object as a
	 * list, extract the requested element from each list, and
	 * compare the elements, not the lists.  "end"-relative indices
	 * are signaled here with large negative values.
	 */

	if (Tcl_ListObjLength(infoPtr->interp, objPtr1, &listLen) != TCL_OK) {
	    infoPtr->resultCode = TCL_ERROR;
	    return order;
	}
	if (infoPtr->index < SORTIDX_NONE) {
	    index = listLen + infoPtr->index + 1;
	} else {
	    index = infoPtr->index;
	}

	if (Tcl_ListObjIndex(infoPtr->interp, objPtr1, index, &objPtr)
		!= TCL_OK) {
	    infoPtr->resultCode = TCL_ERROR;
	    return order;
	}
	if (objPtr == NULL) {
	    objPtr = objPtr1;
	    missingElement:
	    TclFormatInt(buffer, infoPtr->index);
	    Tcl_AppendStringsToObj(Tcl_GetObjResult(infoPtr->interp),
			"element ", buffer, " missing from sublist \"",
			Tcl_GetString(objPtr), "\"", (char *) NULL);
	    infoPtr->resultCode = TCL_ERROR;
	    return order;
	}
	objPtr1 = objPtr;

	if (Tcl_ListObjLength(infoPtr->interp, objPtr2, &listLen) != TCL_OK) {
	    infoPtr->resultCode = TCL_ERROR;
	    return order;
	}
	if (infoPtr->index < SORTIDX_NONE) {
	    index = listLen + infoPtr->index + 1;
	} else {
	    index = infoPtr->index;
	}

	if (Tcl_ListObjIndex(infoPtr->interp, objPtr2, index, &objPtr)
		!= TCL_OK) {
	    infoPtr->resultCode = TCL_ERROR;
	    return order;
	}
	if (objPtr == NULL) {
	    objPtr = objPtr2;
	    goto missingElement;
	}
	objPtr2 = objPtr;
    }
    if (infoPtr->sortMode == SORTMODE_ASCII) {
	order = strcmp(Tcl_GetString(objPtr1), Tcl_GetString(objPtr2));
    } else if (infoPtr->sortMode == SORTMODE_DICTIONARY) {
	order = DictionaryCompare(
		Tcl_GetString(objPtr1),	Tcl_GetString(objPtr2));
    } else if (infoPtr->sortMode == SORTMODE_INTEGER) {
	long a, b;

	if ((Tcl_GetLongFromObj(infoPtr->interp, objPtr1, &a) != TCL_OK)
		|| (Tcl_GetLongFromObj(infoPtr->interp, objPtr2, &b)
		!= TCL_OK)) {
	    infoPtr->resultCode = TCL_ERROR;
	    return order;
	}
	if (a > b) {
	    order = 1;
	} else if (b > a) {
	    order = -1;
	}
    } else if (infoPtr->sortMode == SORTMODE_REAL) {
	double a, b;

	if ((Tcl_GetDoubleFromObj(infoPtr->interp, objPtr1, &a) != TCL_OK)
	      || (Tcl_GetDoubleFromObj(infoPtr->interp, objPtr2, &b)
		      != TCL_OK)) {
	    infoPtr->resultCode = TCL_ERROR;
	    return order;
	}
	if (a > b) {
	    order = 1;
	} else if (b > a) {
	    order = -1;
	}
    } else {
	Tcl_Obj **objv, *paramObjv[2];
	int objc;

	paramObjv[0] = objPtr1;
	paramObjv[1] = objPtr2;

  	/*
 	 * We made space in the command list for the two things to
	 * compare. Replace them and evaluate the result.
	 */

	Tcl_ListObjLength(infoPtr->interp, infoPtr->compareCmdPtr, &objc);
	Tcl_ListObjReplace(infoPtr->interp, infoPtr->compareCmdPtr, objc - 2,
		2, 2, paramObjv);
   	Tcl_ListObjGetElements(infoPtr->interp, infoPtr->compareCmdPtr,
		&objc, &objv);

	infoPtr->resultCode = Tcl_EvalObjv(infoPtr->interp, objc, objv, 0);
  
  	if (infoPtr->resultCode != TCL_OK) {
	    Tcl_AddErrorInfo(infoPtr->interp,
		    "\n    (-compare command)");
	    return order;
	}

	/*
	 * Parse the result of the command.
	 */

	if (Tcl_GetIntFromObj(infoPtr->interp,
		Tcl_GetObjResult(infoPtr->interp), &order) != TCL_OK) {
	    Tcl_ResetResult(infoPtr->interp);
	    Tcl_AppendToObj(Tcl_GetObjResult(infoPtr->interp),
		    "-compare command returned non-integer result", -1);
	    infoPtr->resultCode = TCL_ERROR;
	    return order;
	}
    }
    if (!infoPtr->isIncreasing) {
	order = -order;
    }
    return order;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdIL.c`, function `MergeSort: actual bounded binary bucket producer`, lines 3881–3918. Line unit: physical LF only. Original byte range [110497, 111577). Retained reference `source-window-3`.

```text
MergeSort(headPtr, infoPtr)
    SortElement *headPtr;               /* First element on the list */
    SortInfo *infoPtr;                  /* Information needed by the
                                         * comparison operator */
{
    /*
     * The subList array below holds pointers to temporary lists built
     * during the merge sort.  Element i of the array holds a list of
     * length 2**i.
     */

#   define NUM_LISTS 30
    SortElement *subList[NUM_LISTS];
    SortElement *elementPtr;
    int i;

    for(i = 0; i < NUM_LISTS; i++){
        subList[i] = NULL;
    }
    while (headPtr != NULL) {
	elementPtr = headPtr;
	headPtr = headPtr->nextPtr;
	elementPtr->nextPtr = 0;
	for (i = 0; (i < NUM_LISTS) && (subList[i] != NULL); i++){
	    elementPtr = MergeLists(subList[i], elementPtr, infoPtr);
	    subList[i] = NULL;
	}
	if (i >= NUM_LISTS) {
	    i = NUM_LISTS-1;
	}
	subList[i] = elementPtr;
    }
    elementPtr = NULL;
    for (i = 0; i < NUM_LISTS; i++){
        elementPtr = MergeLists(subList[i], elementPtr, infoPtr);
    }
    return elementPtr;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdIL.c`, function `MergeLists: equal-left comparator duplicate flags`, lines 3939–3990. Line unit: physical LF only. Original byte range [112041, 113381). Retained reference `source-window-4`.

```text
MergeLists(leftPtr, rightPtr, infoPtr)
    SortElement *leftPtr;               /* First list to be merged; may be
					 * NULL. */
    SortElement *rightPtr;              /* Second list to be merged; may be
					 * NULL. */
    SortInfo *infoPtr;                  /* Information needed by the
                                         * comparison operator. */
{
    SortElement *headPtr;
    SortElement *tailPtr;
    int cmp;

    if (leftPtr == NULL) {
        return rightPtr;
    }
    if (rightPtr == NULL) {
        return leftPtr;
    }
    cmp = SortCompare(leftPtr->objPtr, rightPtr->objPtr, infoPtr);
    if (cmp > 0) {
	tailPtr = rightPtr;
	rightPtr = rightPtr->nextPtr;
    } else {
	if (cmp == 0) {
	    leftPtr->count++;
	}
	tailPtr = leftPtr;
	leftPtr = leftPtr->nextPtr;
    }
    headPtr = tailPtr;
    while ((leftPtr != NULL) && (rightPtr != NULL)) {
	cmp = SortCompare(leftPtr->objPtr, rightPtr->objPtr, infoPtr);
	if (cmp > 0) {
	    tailPtr->nextPtr = rightPtr;
	    tailPtr = rightPtr;
	    rightPtr = rightPtr->nextPtr;
	} else {
	    if (cmp == 0) {
		leftPtr->count++;
	    }
	    tailPtr->nextPtr = leftPtr;
	    tailPtr = leftPtr;
	    leftPtr = leftPtr->nextPtr;
	}
    }
    if (leftPtr != NULL) {
       tailPtr->nextPtr = leftPtr;
    } else {
       tailPtr->nextPtr = rightPtr;
    }
    return headPtr;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdIL.c`, function `Tcl_LsearchObjCmd: default table, selected original pattern and reached numeric keys`, lines 2714–3358. Line unit: physical LF only. Original byte range [73953, 90109). Retained reference `source-window-5`.

```text
Tcl_LsearchObjCmd(
    ClientData clientData,	/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument values. */
{
    char *bytes, *patternBytes;
    int i, match, mode, index, result, listc, length, elemLen;
    int dataType, isIncreasing, lower, upper, patInt, objInt, offset;
    int allMatches, inlineReturn, negatedMatch, returnSubindices, noCase;
    double patDouble, objDouble;
    SortInfo sortInfo;
    Tcl_Obj *patObj, **listv, *listPtr, *startPtr, *itemPtr;
    SortStrCmpFn_t strCmpFn = strcmp;
    Tcl_RegExp regexp = NULL;
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
    enum datatypes {
	ASCII, DICTIONARY, INTEGER, REAL
    };
    enum modes {
	EXACT, GLOB, REGEXP, SORTED
    };

    mode = GLOB;
    dataType = ASCII;
    isIncreasing = 1;
    allMatches = 0;
    inlineReturn = 0;
    returnSubindices = 0;
    negatedMatch = 0;
    listPtr = NULL;
    startPtr = NULL;
    offset = 0;
    noCase = 0;
    sortInfo.compareCmdPtr = NULL;
    sortInfo.isIncreasing = 1;
    sortInfo.sortMode = 0;
    sortInfo.interp = interp;
    sortInfo.resultCode = TCL_OK;
    sortInfo.indexv = NULL;
    sortInfo.indexc = 0;

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "?options? list pattern");
	return TCL_ERROR;
    }

    for (i = 1; i < objc-2; i++) {
	if (Tcl_GetIndexFromObj(interp, objv[i], options, "option", 0, &index)
		!= TCL_OK) {
	    if (startPtr != NULL) {
		Tcl_DecrRefCount(startPtr);
	    }
	    if (sortInfo.indexc > 1) {
		ckfree((char *) sortInfo.indexv);
	    }
	    return TCL_ERROR;
	}
	switch ((enum options) index) {
	case LSEARCH_ALL:		/* -all */
	    allMatches = 1;
	    break;
	case LSEARCH_ASCII:		/* -ascii */
	    dataType = ASCII;
	    break;
	case LSEARCH_DECREASING:	/* -decreasing */
	    isIncreasing = 0;
	    sortInfo.isIncreasing = 0;
	    break;
	case LSEARCH_DICTIONARY:	/* -dictionary */
	    dataType = DICTIONARY;
	    break;
	case LSEARCH_EXACT:		/* -increasing */
	    mode = EXACT;
	    break;
	case LSEARCH_GLOB:		/* -glob */
	    mode = GLOB;
	    break;
	case LSEARCH_INCREASING:	/* -increasing */
	    isIncreasing = 1;
	    sortInfo.isIncreasing = 1;
	    break;
	case LSEARCH_INLINE:		/* -inline */
	    inlineReturn = 1;
	    break;
	case LSEARCH_INTEGER:		/* -integer */
	    dataType = INTEGER;
	    break;
	case LSEARCH_NOCASE:		/* -nocase */
	    strCmpFn = TclUtfCasecmp;
	    noCase = 1;
	    break;
	case LSEARCH_NOT:		/* -not */
	    negatedMatch = 1;
	    break;
	case LSEARCH_REAL:		/* -real */
	    dataType = REAL;
	    break;
	case LSEARCH_REGEXP:		/* -regexp */
	    mode = REGEXP;
	    break;
	case LSEARCH_SORTED:		/* -sorted */
	    mode = SORTED;
	    break;
	case LSEARCH_SUBINDICES:	/* -subindices */
	    returnSubindices = 1;
	    break;
	case LSEARCH_START:		/* -start */
	    /*
	     * If there was a previous -start option, release its saved index
	     * because it will either be replaced or there will be an error.
	     */

	    if (startPtr != NULL) {
		Tcl_DecrRefCount(startPtr);
	    }
	    if (i > objc-4) {
		if (sortInfo.indexc > 1) {
		    ckfree((char *) sortInfo.indexv);
		}
		Tcl_AppendResult(interp, "missing starting index", NULL);
		return TCL_ERROR;
	    }
	    i++;
	    if (objv[i] == objv[objc - 2]) {
		/*
		 * Take copy to prevent shimmering problems. Note that it does
		 * not matter if the index obj is also a component of the list
		 * being searched. We only need to copy where the list and the
		 * index are one-and-the-same.
		 */

		startPtr = Tcl_DuplicateObj(objv[i]);
	    } else {
		startPtr = objv[i];
		Tcl_IncrRefCount(startPtr);
	    }
	    break;
	case LSEARCH_INDEX: {		/* -index */
	    Tcl_Obj **indices;
	    int j;

	    if (sortInfo.indexc > 1) {
		ckfree((char *) sortInfo.indexv);
	    }
	    if (i > objc-4) {
		if (startPtr != NULL) {
		    Tcl_DecrRefCount(startPtr);
		}
		Tcl_AppendResult(interp,
			"\"-index\" option must be followed by list index",
			NULL);
		return TCL_ERROR;
	    }

	    /*
	     * Store the extracted indices for processing by sublist
	     * extraction. Note that we don't do this using objects because
	     * that has shimmering problems.
	     */

	    i++;
	    if (TclListObjGetElements(interp, objv[i],
		    &sortInfo.indexc, &indices) != TCL_OK) {
		if (startPtr != NULL) {
		    Tcl_DecrRefCount(startPtr);
		}
		return TCL_ERROR;
	    }
	    switch (sortInfo.indexc) {
	    case 0:
		sortInfo.indexv = NULL;
		break;
	    case 1:
		sortInfo.indexv = &sortInfo.singleIndex;
		break;
	    default:
		sortInfo.indexv = (int *)
			ckalloc(sizeof(int) * sortInfo.indexc);
	    }

	    /*
	     * Fill the array by parsing each index. We don't know whether
	     * their scale is sensible yet, but we at least perform the
	     * syntactic check here.
	     */

	    for (j=0 ; j<sortInfo.indexc ; j++) {
		if (TclGetIntForIndexM(interp, indices[j], SORTIDX_END,
			&sortInfo.indexv[j]) != TCL_OK) {
		    if (sortInfo.indexc > 1) {
			ckfree((char *) sortInfo.indexv);
		    }
		    Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
			    "\n    (-index option item number %d)", j));
		    return TCL_ERROR;
		}
	    }
	    break;
	}
	}
    }

    /*
     * Subindices only make sense if asked for with -index option set.
     */

    if (returnSubindices && sortInfo.indexc==0) {
	if (startPtr != NULL) {
	    Tcl_DecrRefCount(startPtr);
	}
	Tcl_AppendResult(interp,
		"-subindices cannot be used without -index option", NULL);
	return TCL_ERROR;
    }

    if ((enum modes) mode == REGEXP) {
	/*
	 * We can shimmer regexp/list if listv[i] == pattern, so get the
	 * regexp rep before the list rep. First time round, omit the interp
	 * and hope that the compilation will succeed. If it fails, we'll
	 * recompile in "expensive" mode with a place to put error messages.
	 */

	regexp = Tcl_GetRegExpFromObj(NULL, objv[objc - 1],
		TCL_REG_ADVANCED | TCL_REG_NOSUB |
		(noCase ? TCL_REG_NOCASE : 0));
	if (regexp == NULL) {
	    /*
	     * Failed to compile the RE. Try again without the TCL_REG_NOSUB
	     * flag in case the RE had sub-expressions in it [Bug 1366683]. If
	     * this fails, an error message will be left in the interpreter.
	     */

	    regexp = Tcl_GetRegExpFromObj(interp, objv[objc - 1],
		    TCL_REG_ADVANCED | (noCase ? TCL_REG_NOCASE : 0));
	}

	if (regexp == NULL) {
	    if (startPtr != NULL) {
		Tcl_DecrRefCount(startPtr);
	    }
	    if (sortInfo.indexc > 1) {
		ckfree((char *) sortInfo.indexv);
	    }
	    return TCL_ERROR;
	}
    }

    /*
     * Make sure the list argument is a list object and get its length and a
     * pointer to its array of element pointers.
     */

    result = TclListObjGetElements(interp, objv[objc - 2], &listc, &listv);
    if (result != TCL_OK) {
	if (startPtr != NULL) {
	    Tcl_DecrRefCount(startPtr);
	}
	if (sortInfo.indexc > 1) {
	    ckfree((char *) sortInfo.indexv);
	}
	return result;
    }

    /*
     * Get the user-specified start offset.
     */

    if (startPtr) {
	result = TclGetIntForIndexM(interp, startPtr, listc-1, &offset);
	Tcl_DecrRefCount(startPtr);
	if (result != TCL_OK) {
	    if (sortInfo.indexc > 1) {
		ckfree((char *) sortInfo.indexv);
	    }
	    return result;
	}
	if (offset < 0) {
	    offset = 0;
	}

	/*
	 * If the search started past the end of the list, we just return a
	 * "did not match anything at all" result straight away. [Bug 1374778]
	 */

	if (offset > listc-1) {
	    if (sortInfo.indexc > 1) {
		ckfree((char *) sortInfo.indexv);
	    }
	    if (allMatches || inlineReturn) {
		Tcl_ResetResult(interp);
	    } else {
		Tcl_SetObjResult(interp, Tcl_NewIntObj(-1));
	    }
	    return TCL_OK;
	}
    }

    patObj = objv[objc - 1];
    patternBytes = NULL;
    if ((enum modes) mode == EXACT || (enum modes) mode == SORTED) {
	switch ((enum datatypes) dataType) {
	case ASCII:
	case DICTIONARY:
	    patternBytes = TclGetStringFromObj(patObj, &length);
	    break;
	case INTEGER:
	    result = TclGetIntFromObj(interp, patObj, &patInt);
	    if (result != TCL_OK) {
		if (sortInfo.indexc > 1) {
		    ckfree((char *) sortInfo.indexv);
		}
		return result;
	    }

	    /*
	     * List representation might have been shimmered; restore it. [Bug
	     * 1844789]
	     */

	    TclListObjGetElements(NULL, objv[objc - 2], &listc, &listv);
	    break;
	case REAL:
	    result = Tcl_GetDoubleFromObj(interp, patObj, &patDouble);
	    if (result != TCL_OK) {
		if (sortInfo.indexc > 1) {
		    ckfree((char *) sortInfo.indexv);
		}
		return result;
	    }

	    /*
	     * List representation might have been shimmered; restore it. [Bug
	     * 1844789]
	     */

	    TclListObjGetElements(NULL, objv[objc - 2], &listc, &listv);
	    break;
	}
    } else {
	patternBytes = TclGetStringFromObj(patObj, &length);
    }

    /*
     * Set default index value to -1, indicating failure; if we find the item
     * in the course of our search, index will be set to the correct value.
     */

    index = -1;
    match = 0;

    if ((enum modes) mode == SORTED && !allMatches && !negatedMatch) {
	/*
	 * If the data is sorted, we can do a more intelligent search. Note
	 * that there is no point in being smart when -all was specified; in
	 * that case, we have to look at all items anyway, and there is no
	 * sense in doing this when the match sense is inverted.
	 */

	lower = offset - 1;
	upper = listc;
	while (lower + 1 != upper && sortInfo.resultCode == TCL_OK) {
	    i = (lower + upper)/2;
	    if (sortInfo.indexc != 0) {
		itemPtr = SelectObjFromSublist(listv[i], &sortInfo);
		if (sortInfo.resultCode != TCL_OK) {
		    if (sortInfo.indexc > 1) {
			ckfree((char *) sortInfo.indexv);
		    }
		    return sortInfo.resultCode;
		}
	    } else {
		itemPtr = listv[i];
	    }
	    switch ((enum datatypes) dataType) {
	    case ASCII:
		bytes = TclGetString(itemPtr);
		match = strCmpFn(patternBytes, bytes);
		break;
	    case DICTIONARY:
		bytes = TclGetString(itemPtr);
		match = DictionaryCompare(patternBytes, bytes);
		break;
	    case INTEGER:
		result = TclGetIntFromObj(interp, itemPtr, &objInt);
		if (result != TCL_OK) {
		    if (sortInfo.indexc > 1) {
			ckfree((char *) sortInfo.indexv);
		    }
		    return result;
		}
		if (patInt == objInt) {
		    match = 0;
		} else if (patInt < objInt) {
		    match = -1;
		} else {
		    match = 1;
		}
		break;
	    case REAL:
		result = Tcl_GetDoubleFromObj(interp, itemPtr, &objDouble);
		if (result != TCL_OK) {
		    if (sortInfo.indexc > 1) {
			ckfree((char *) sortInfo.indexv);
		    }
		    return result;
		}
		if (patDouble == objDouble) {
		    match = 0;
		} else if (patDouble < objDouble) {
		    match = -1;
		} else {
		    match = 1;
		}
		break;
	    }
	    if (match == 0) {
		/*
		 * Normally, binary search is written to stop when it finds a
		 * match. If there are duplicates of an element in the list,
		 * our first match might not be the first occurance.
		 * Consider: 0 0 0 1 1 1 2 2 2
		 *
		 * To maintain consistancy with standard lsearch semantics, we
		 * must find the leftmost occurance of the pattern in the
		 * list. Thus we don't just stop searching here. This
		 * variation means that a search always makes log n
		 * comparisons (normal binary search might "get lucky" with an
		 * early comparison).
		 */

		index = i;
		upper = i;
	    } else if (match > 0) {
		if (isIncreasing) {
		    lower = i;
		} else {
		    upper = i;
		}
	    } else {
		if (isIncreasing) {
		    upper = i;
		} else {
		    lower = i;
		}
	    }
	}

    } else {
	/*
	 * We need to do a linear search, because (at least one) of:
	 *   - our matcher can only tell equal vs. not equal
	 *   - our matching sense is negated
	 *   - we're building a list of all matched items
	 */

	if (allMatches) {
	    listPtr = Tcl_NewListObj(0, NULL);
	}
	for (i = offset; i < listc; i++) {
	    match = 0;
	    if (sortInfo.indexc != 0) {	    
		itemPtr = SelectObjFromSublist(listv[i], &sortInfo);
		if (sortInfo.resultCode != TCL_OK) {
		    if (listPtr != NULL) {
			Tcl_DecrRefCount(listPtr);
		    }
		    if (sortInfo.indexc > 1) {
			ckfree((char *) sortInfo.indexv);
		    }
		    return sortInfo.resultCode;
		}
	    } else {
		itemPtr = listv[i];
	    }
		
	    switch ((enum modes) mode) {
	    case SORTED:
	    case EXACT:
		switch ((enum datatypes) dataType) {
		case ASCII:
		    bytes = TclGetStringFromObj(itemPtr, &elemLen);
		    if (length == elemLen) {
			/*
			 * This split allows for more optimal compilation of
			 * memcmp/strcasecmp.
			 */

			if (noCase) {
			    match = (TclUtfCasecmp(bytes, patternBytes) == 0);
			} else {
			    match = (memcmp(bytes, patternBytes,
				    (size_t) length) == 0);
			}
		    }
		    break;

		case DICTIONARY:
		    bytes = TclGetString(itemPtr);
		    match = (DictionaryCompare(bytes, patternBytes) == 0);
		    break;

		case INTEGER:
		    result = TclGetIntFromObj(interp, itemPtr, &objInt);
		    if (result != TCL_OK) {
			if (listPtr != NULL) {
			    Tcl_DecrRefCount(listPtr);
			}
			if (sortInfo.indexc > 1) {
			    ckfree((char *) sortInfo.indexv);
			}
			return result;
		    }
		    match = (objInt == patInt);
		    break;

		case REAL:
		    result = Tcl_GetDoubleFromObj(interp,itemPtr, &objDouble);
		    if (result != TCL_OK) {
			if (listPtr) {
			    Tcl_DecrRefCount(listPtr);
			}
			if (sortInfo.indexc > 1) {
			    ckfree((char *) sortInfo.indexv);
			}
			return result;
		    }
		    match = (objDouble == patDouble);
		    break;
		}
		break;

	    case GLOB:
		match = Tcl_StringCaseMatch(TclGetString(itemPtr),
			patternBytes, noCase);
		break;

	    case REGEXP:
		match = Tcl_RegExpExecObj(interp, regexp, itemPtr, 0, 0, 0);
		if (match < 0) {
		    Tcl_DecrRefCount(patObj);
		    if (listPtr != NULL) {
			Tcl_DecrRefCount(listPtr);
		    }
		    if (sortInfo.indexc > 1) {
			ckfree((char *) sortInfo.indexv);
		    }
		    return TCL_ERROR;
		}
		break;
	    }

	    /*
	     * Invert match condition for -not.
	     */

	    if (negatedMatch) {
		match = !match;
	    }
	    if (!match) {
		continue;
	    }
	    if (!allMatches) {
		index = i;
		break;
	    } else if (inlineReturn) {
		/*
		 * Note that these appends are not expected to fail.
		 */

		if (returnSubindices && (sortInfo.indexc != 0)) {
		    itemPtr = SelectObjFromSublist(listv[i], &sortInfo);
		} else {
		    itemPtr = listv[i];
		}
		Tcl_ListObjAppendElement(interp, listPtr, itemPtr);
	    } else if (returnSubindices) {
		int j;

		itemPtr = Tcl_NewIntObj(i);
		for (j=0 ; j<sortInfo.indexc ; j++) {
		    Tcl_ListObjAppendElement(interp, itemPtr,
			    Tcl_NewIntObj(sortInfo.indexv[j]));
		}
		Tcl_ListObjAppendElement(interp, listPtr, itemPtr);
	    } else {
		Tcl_ListObjAppendElement(interp, listPtr, Tcl_NewIntObj(i));
	    }
	}
    }

    /*
     * Return everything or a single value.
     */

    if (allMatches) {
	Tcl_SetObjResult(interp, listPtr);
    } else if (!inlineReturn) {
	if (returnSubindices) {
	    int j;

	    itemPtr = Tcl_NewIntObj(index);
	    for (j=0 ; j<sortInfo.indexc ; j++) {
		Tcl_ListObjAppendElement(interp, itemPtr,
			Tcl_NewIntObj(sortInfo.indexv[j]));
	    }
	    Tcl_SetObjResult(interp, itemPtr);
	} else {
	    Tcl_SetObjResult(interp, Tcl_NewIntObj(index));
	}
    } else if (index < 0) {
	/*
	 * Is this superfluous? The result should be a blank object by
	 * default...
	 */

	Tcl_SetObjResult(interp, Tcl_NewObj());
    } else {
	Tcl_SetObjResult(interp, listv[index]);
    }

    /*
     * Cleanup the index list array.
     */

    if (sortInfo.indexc > 1) {
	ckfree((char *) sortInfo.indexv);
    }
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdIL.c`, function `Tcl_LsortObjCmd: input-order original numeric preparation and list merge producer`, lines 3463–3809. Line unit: physical LF only. Original byte range [92458, 101651). Retained reference `source-window-6`.

```text
Tcl_LsortObjCmd(
    ClientData clientData,	/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument values. */
{
    int i, j, index, indices, length, nocase = 0, indexc;
    int sortMode = SORTMODE_ASCII;
    Tcl_Obj *resultPtr, *cmdPtr, **listObjPtrs, *listObj, *indexPtr;
    SortElement *elementArray, *elementPtr;
    SortInfo sortInfo;		/* Information about this sort that needs to
				 * be passed to the comparison function. */
    static const char *switches[] = {
	"-ascii", "-command", "-decreasing", "-dictionary", "-increasing",
	"-index", "-indices", "-integer", "-nocase", "-real", "-unique", NULL
    };
    enum Lsort_Switches {
	LSORT_ASCII, LSORT_COMMAND, LSORT_DECREASING, LSORT_DICTIONARY,
	LSORT_INCREASING, LSORT_INDEX, LSORT_INDICES, LSORT_INTEGER,
	LSORT_NOCASE, LSORT_REAL, LSORT_UNIQUE
    };

    /*
     * The subList array below holds pointers to temporary lists built during
     * the merge sort. Element i of the array holds a list of length 2**i.
     */
#   define NUM_LISTS 30
    SortElement *subList[NUM_LISTS+1];

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "?options? list");
	return TCL_ERROR;
    }

    /*
     * Parse arguments to set up the mode for the sort.
     */

    sortInfo.isIncreasing = 1;
    sortInfo.sortMode = SORTMODE_ASCII;
    sortInfo.indexv = NULL;
    sortInfo.indexc = 0;
    sortInfo.unique = 0;
    sortInfo.interp = interp;
    sortInfo.resultCode = TCL_OK;    
    cmdPtr = NULL;
    indices = 0;
    for (i = 1; i < objc-1; i++) {
	if (Tcl_GetIndexFromObj(interp, objv[i], switches, "option", 0,
		&index) != TCL_OK) {
	    if (sortInfo.indexc > 1) {
		ckfree((char *) sortInfo.indexv);
	    }
	    return TCL_ERROR;
	}
	switch ((enum Lsort_Switches) index) {
	case LSORT_ASCII:
	    sortInfo.sortMode = SORTMODE_ASCII;
	    break;
	case LSORT_COMMAND:
	    if (i == (objc-2)) {
		if (sortInfo.indexc > 1) {
		    ckfree((char *) sortInfo.indexv);
		}
		Tcl_AppendResult(interp,
			"\"-command\" option must be followed "
			"by comparison command", NULL);
		return TCL_ERROR;
	    }
	    sortInfo.sortMode = SORTMODE_COMMAND;
	    cmdPtr = objv[i+1];
	    i++;
	    break;
	case LSORT_DECREASING:
	    sortInfo.isIncreasing = 0;
	    break;
	case LSORT_DICTIONARY:
	    sortInfo.sortMode = SORTMODE_DICTIONARY;
	    break;
	case LSORT_INCREASING:
	    sortInfo.isIncreasing = 1;
	    break;
	case LSORT_INDEX: {
	    Tcl_Obj **indices;

	    if (sortInfo.indexc > 1) {
		ckfree((char *) sortInfo.indexv);
	    }
	    if (i == (objc-2)) {
		Tcl_AppendResult(interp, "\"-index\" option must be "
			"followed by list index", NULL);
		return TCL_ERROR;
	    }

	    /*
	     * Take copy to prevent shimmering problems.
	     */

	    if (TclListObjGetElements(interp, objv[i+1], &sortInfo.indexc,
		    &indices) != TCL_OK) {
		return TCL_ERROR;
	    }
	    switch (sortInfo.indexc) {
	    case 0:
		sortInfo.indexv = NULL;
		break;
	    case 1:
		sortInfo.indexv = &sortInfo.singleIndex;
		break;
	    default:
		sortInfo.indexv = (int *)
			ckalloc(sizeof(int) * sortInfo.indexc);
	    }

	    /*
	     * Fill the array by parsing each index. We don't know whether
	     * their scale is sensible yet, but we at least perform the
	     * syntactic check here.
	     */

	    for (j=0 ; j<sortInfo.indexc ; j++) {
		if (TclGetIntForIndexM(interp, indices[j], SORTIDX_END,
			&sortInfo.indexv[j]) != TCL_OK) {
		    if (sortInfo.indexc > 1) {
			ckfree((char *) sortInfo.indexv);
		    }
		    Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
			    "\n    (-index option item number %d)", j));
		    return TCL_ERROR;
		}
	    }
	    i++;
	    break;
	}
	case LSORT_INTEGER:
	    sortInfo.sortMode = SORTMODE_INTEGER;
	    break;
	case LSORT_NOCASE:
	    nocase = 1;
	    break;
	case LSORT_REAL:
	    sortInfo.sortMode = SORTMODE_REAL;
	    break;
	case LSORT_UNIQUE:
	    sortInfo.unique = 1;
	    break;
	case LSORT_INDICES:
	    indices = 1;
	    break;
	}
    }
    if (nocase && (sortInfo.sortMode == SORTMODE_ASCII)) {
	sortInfo.sortMode = SORTMODE_ASCII_NC;
    }

    listObj = objv[objc-1];

    if (sortInfo.sortMode == SORTMODE_COMMAND) {
	Tcl_Obj *newCommandPtr, *newObjPtr;

	/*
	 * When sorting using a command, we are reentrant and therefore might
	 * have the representation of the list being sorted shimmered out from
	 * underneath our feet. Take a copy (cheap) to prevent this. [Bug
	 * 1675116]
	 */

	listObj = TclListObjCopy(interp, listObj);
	if (listObj == NULL) {
	    if (sortInfo.indexc > 1) {
		ckfree((char *) sortInfo.indexv);
	    }
	    return TCL_ERROR;
	}

	/*
	 * The existing command is a list. We want to flatten it, append two
	 * dummy arguments on the end, and replace these arguments later.
	 */

	newCommandPtr = Tcl_DuplicateObj(cmdPtr);
	TclNewObj(newObjPtr);
	Tcl_IncrRefCount(newCommandPtr);
	if (Tcl_ListObjAppendElement(interp, newCommandPtr, newObjPtr)
		!= TCL_OK) {
	    TclDecrRefCount(newCommandPtr);
	    TclDecrRefCount(listObj);
	    Tcl_IncrRefCount(newObjPtr);
	    TclDecrRefCount(newObjPtr);
	    if (sortInfo.indexc > 1) {
		ckfree((char *) sortInfo.indexv);
	    }
	    return TCL_ERROR;
	}
	Tcl_ListObjAppendElement(interp, newCommandPtr, Tcl_NewObj());
	sortInfo.compareCmdPtr = newCommandPtr;
    }

    sortInfo.resultCode = TclListObjGetElements(interp, listObj,
	    &length, &listObjPtrs);
    if (sortInfo.resultCode != TCL_OK || length <= 0) {
	goto done;
    }
    sortInfo.numElements = length;
    
    indexc = sortInfo.indexc;
    sortMode = sortInfo.sortMode;
    if ((sortMode == SORTMODE_ASCII_NC)
	    || (sortMode == SORTMODE_DICTIONARY)) {
	/*
	 * For this function's purpose all string-based modes are equivalent
	 */
	
	sortMode = SORTMODE_ASCII;
    }

    /*
     * Initialize the sublists. After the following loop, subList[i] will
     * contain a sorted sublist of length 2**i. Use one extra subList at the
     * end, always at NULL, to indicate the end of the lists.
     */
    
    for (j=0 ; j<=NUM_LISTS ; j++) {
	subList[j] = NULL;
    }

    /*
     * The following loop creates a SortElement for each list element and
     * begins sorting it into the sublists as it appears.
     */

    elementArray = (SortElement *) ckalloc( length * sizeof(SortElement));

    for (i=0; i < length; i++){
	if (indexc) {
	    /*
	     * If this is an indexed sort, retrieve the corresponding element
	     */
	    indexPtr = SelectObjFromSublist(listObjPtrs[i], &sortInfo);
	    if (sortInfo.resultCode != TCL_OK) {
		goto done1;
	    }
	} else {
	    indexPtr = listObjPtrs[i];
	}

	/*
	 * Determine the "value" of this object for sorting purposes
	 */
	
	if (sortMode == SORTMODE_ASCII) {
	    elementArray[i].index.strValuePtr = TclGetString(indexPtr);
	} else if (sortMode == SORTMODE_INTEGER) {
	    long a;
	    if (TclGetLongFromObj(sortInfo.interp, indexPtr, &a) != TCL_OK) {
		sortInfo.resultCode = TCL_ERROR;
		goto done1;
	    }
	    elementArray[i].index.intValue = a;
	} else if (sortMode == SORTMODE_REAL) {
	    double a;
	    if (Tcl_GetDoubleFromObj(sortInfo.interp, indexPtr, &a) != TCL_OK) {
		sortInfo.resultCode = TCL_ERROR;
		goto done1;
	    }
	    elementArray[i].index.doubleValue = a;
	} else {
	    elementArray[i].index.objValuePtr = indexPtr;
	}

	/*
	 * Determine the representation of this element in the result: either
	 * the objPtr itself, or its index in the original list.
	 */
	
	elementArray[i].objPtr = (indices ? INT2PTR(i) : listObjPtrs[i]);

	/*
	 * Merge this element in the pre-existing sublists (and merge together
	 * sublists when we have two of the same size).
	 */
	
	elementArray[i].nextPtr = NULL;
	elementPtr = &elementArray[i];
	for (j=0 ; subList[j] ; j++) {
	    elementPtr = MergeLists(subList[j], elementPtr, &sortInfo);
	    subList[j] = NULL;
	}
	if (j >= NUM_LISTS) {
	    j = NUM_LISTS-1;
	}
	subList[j] = elementPtr;
    }

    /*
     * Merge all sublists
     */
    
    elementPtr = subList[0];
    for (j=1 ; j<NUM_LISTS ; j++) {
	elementPtr = MergeLists(subList[j], elementPtr, &sortInfo);
    }

    /*
     * Now store the sorted elements in the result list.
     */
    
    if (sortInfo.resultCode == TCL_OK) {
	List *listRepPtr;
	Tcl_Obj **newArray, *objPtr;
	int i;
	
	resultPtr = Tcl_NewListObj(sortInfo.numElements, NULL);
	listRepPtr = ListRepPtr(resultPtr);
	newArray = &listRepPtr->elements;
	if (indices) {
	    for (i = 0; elementPtr != NULL ; elementPtr = elementPtr->nextPtr){
		objPtr = Tcl_NewIntObj(PTR2INT(elementPtr->objPtr));
		newArray[i++] = objPtr;
		Tcl_IncrRefCount(objPtr);
	    }
	} else {
	    for (i = 0; elementPtr != NULL ; elementPtr = elementPtr->nextPtr){
		objPtr = elementPtr->objPtr;
		newArray[i++] = objPtr;
		Tcl_IncrRefCount(objPtr);
	    }
	}
	listRepPtr->elemCount = i;
	Tcl_SetObjResult(interp, resultPtr);
    }

  done1:
    ckfree((char *)elementArray);

  done:
    if (sortMode == SORTMODE_COMMAND) {
	TclDecrRefCount(sortInfo.compareCmdPtr);
	TclDecrRefCount(listObj);
	sortInfo.compareCmdPtr = NULL;
    }
    if (sortInfo.indexc > 1) {
	ckfree((char *) sortInfo.indexv);
    }
    return sortInfo.resultCode;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdIL.c`, function `SortCompare: comparison of prepared numeric values and command result primitive`, lines 3930–4016. Line unit: physical LF only. Original byte range [105119, 107487). Retained reference `source-window-7`.

```text
SortCompare(
    SortElement *elemPtr1, SortElement *elemPtr2,
				/* Values to be compared. */
    SortInfo *infoPtr)		/* Information passed from the top-level
				 * "lsort" command. */
{
    int order = 0;

    if (infoPtr->sortMode == SORTMODE_ASCII) {
	order = strcmp(elemPtr1->index.strValuePtr,
		elemPtr2->index.strValuePtr);
    } else if (infoPtr->sortMode == SORTMODE_ASCII_NC) {
	order = TclUtfCasecmp(elemPtr1->index.strValuePtr,
		elemPtr2->index.strValuePtr);
    } else if (infoPtr->sortMode == SORTMODE_DICTIONARY) {
	order = DictionaryCompare(elemPtr1->index.strValuePtr,
		elemPtr2->index.strValuePtr);
    } else if (infoPtr->sortMode == SORTMODE_INTEGER) {
	long a, b;

	a = elemPtr1->index.intValue;
	b = elemPtr2->index.intValue;
	order = ((a >= b) - (a <= b));
    } else if (infoPtr->sortMode == SORTMODE_REAL) {
	double a, b;

	a = elemPtr1->index.doubleValue;
	b = elemPtr2->index.doubleValue;
	order = ((a >= b) - (a <= b));
    } else {
	Tcl_Obj **objv, *paramObjv[2];
	int objc;
	Tcl_Obj *objPtr1, *objPtr2;

	if (infoPtr->resultCode != TCL_OK) {
	    /*
	     * Once an error has occurred, skip any future comparisons so as
	     * to preserve the error message in sortInterp->result.
	     */
	    
	    return 0;
	}

	objPtr1 = elemPtr1->index.objValuePtr;
	objPtr2 = elemPtr2->index.objValuePtr;
	
	paramObjv[0] = objPtr1;
	paramObjv[1] = objPtr2;

	/*
	 * We made space in the command list for the two things to compare.
	 * Replace them and evaluate the result.
	 */

	TclListObjLength(infoPtr->interp, infoPtr->compareCmdPtr, &objc);
	Tcl_ListObjReplace(infoPtr->interp, infoPtr->compareCmdPtr, objc - 2,
		2, 2, paramObjv);
	TclListObjGetElements(infoPtr->interp, infoPtr->compareCmdPtr,
		&objc, &objv);

	infoPtr->resultCode = Tcl_EvalObjv(infoPtr->interp, objc, objv, 0);

	if (infoPtr->resultCode != TCL_OK) {
	    Tcl_AddErrorInfo(infoPtr->interp,
		    "\n    (-compare command)");
	    return 0;
	}

	/*
	 * Parse the result of the command.
	 */

	if (TclGetIntFromObj(infoPtr->interp,
		Tcl_GetObjResult(infoPtr->interp), &order) != TCL_OK) {
	    Tcl_ResetResult(infoPtr->interp);
	    Tcl_AppendResult(infoPtr->interp,
		    "-compare command returned non-integer result", NULL);
	    infoPtr->resultCode = TCL_ERROR;
	    return 0;
	}
    }
    if (!infoPtr->isIncreasing) {
	order = -order;
    }
    return order;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdIL.c`, function `Tcl_LsearchObjCmd: default table, selected original pattern and reached numeric keys`, lines 2897–3546. Line unit: physical LF only. Original byte range [79518, 96039). Retained reference `source-window-8`.

```text
Tcl_LsearchObjCmd(
    ClientData clientData,	/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument values. */
{
    const char *bytes, *patternBytes;
    int i, match, index, result=TCL_OK, listc, length, elemLen, bisect;
    int dataType, isIncreasing, lower, upper, offset;
    Tcl_WideInt patWide, objWide;
    int allMatches, inlineReturn, negatedMatch, returnSubindices, noCase;
    double patDouble, objDouble;
    SortInfo sortInfo;
    Tcl_Obj *patObj, **listv, *listPtr, *startPtr, *itemPtr;
    SortStrCmpFn_t strCmpFn = strcmp;
    Tcl_RegExp regexp = NULL;
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
    enum datatypes {
	ASCII, DICTIONARY, INTEGER, REAL
    };
    enum modes {
	EXACT, GLOB, REGEXP, SORTED
    };
    enum modes mode;

    mode = GLOB;
    dataType = ASCII;
    isIncreasing = 1;
    allMatches = 0;
    inlineReturn = 0;
    returnSubindices = 0;
    negatedMatch = 0;
    bisect = 0;
    listPtr = NULL;
    startPtr = NULL;
    offset = 0;
    noCase = 0;
    sortInfo.compareCmdPtr = NULL;
    sortInfo.isIncreasing = 1;
    sortInfo.sortMode = 0;
    sortInfo.interp = interp;
    sortInfo.resultCode = TCL_OK;
    sortInfo.indexv = NULL;
    sortInfo.indexc = 0;

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-option value ...? list pattern");
	return TCL_ERROR;
    }

    for (i = 1; i < objc-2; i++) {
	if (Tcl_GetIndexFromObj(interp, objv[i], options, "option", 0, &index)
		!= TCL_OK) {
	    if (startPtr != NULL) {
		Tcl_DecrRefCount(startPtr);
	    }
	    result = TCL_ERROR;
	    goto done;
	}
	switch ((enum options) index) {
	case LSEARCH_ALL:		/* -all */
	    allMatches = 1;
	    break;
	case LSEARCH_ASCII:		/* -ascii */
	    dataType = ASCII;
	    break;
	case LSEARCH_BISECT:		/* -bisect */
	    mode = SORTED;
	    bisect = 1;
	    break;
	case LSEARCH_DECREASING:	/* -decreasing */
	    isIncreasing = 0;
	    sortInfo.isIncreasing = 0;
	    break;
	case LSEARCH_DICTIONARY:	/* -dictionary */
	    dataType = DICTIONARY;
	    break;
	case LSEARCH_EXACT:		/* -increasing */
	    mode = EXACT;
	    break;
	case LSEARCH_GLOB:		/* -glob */
	    mode = GLOB;
	    break;
	case LSEARCH_INCREASING:	/* -increasing */
	    isIncreasing = 1;
	    sortInfo.isIncreasing = 1;
	    break;
	case LSEARCH_INLINE:		/* -inline */
	    inlineReturn = 1;
	    break;
	case LSEARCH_INTEGER:		/* -integer */
	    dataType = INTEGER;
	    break;
	case LSEARCH_NOCASE:		/* -nocase */
	    strCmpFn = TclUtfCasecmp;
	    noCase = 1;
	    break;
	case LSEARCH_NOT:		/* -not */
	    negatedMatch = 1;
	    break;
	case LSEARCH_REAL:		/* -real */
	    dataType = REAL;
	    break;
	case LSEARCH_REGEXP:		/* -regexp */
	    mode = REGEXP;
	    break;
	case LSEARCH_SORTED:		/* -sorted */
	    mode = SORTED;
	    break;
	case LSEARCH_SUBINDICES:	/* -subindices */
	    returnSubindices = 1;
	    break;
	case LSEARCH_START:		/* -start */
	    /*
	     * If there was a previous -start option, release its saved index
	     * because it will either be replaced or there will be an error.
	     */

	    if (startPtr != NULL) {
		Tcl_DecrRefCount(startPtr);
	    }
	    if (i > objc-4) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"missing starting index", -1));
		Tcl_SetErrorCode(interp, "TCL", "ARGUMENT", "MISSING", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    i++;
	    if (objv[i] == objv[objc - 2]) {
		/*
		 * Take copy to prevent shimmering problems. Note that it does
		 * not matter if the index obj is also a component of the list
		 * being searched. We only need to copy where the list and the
		 * index are one-and-the-same.
		 */

		startPtr = Tcl_DuplicateObj(objv[i]);
	    } else {
		startPtr = objv[i];
		Tcl_IncrRefCount(startPtr);
	    }
	    break;
	case LSEARCH_INDEX: {		/* -index */
	    Tcl_Obj **indices;
	    int j;

	    if (sortInfo.indexc > 1) {
		TclStackFree(interp, sortInfo.indexv);
	    }
	    if (i > objc-4) {
		if (startPtr != NULL) {
		    Tcl_DecrRefCount(startPtr);
		}
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"\"-index\" option must be followed by list index",
			-1));
		Tcl_SetErrorCode(interp, "TCL", "ARGUMENT", "MISSING", (char *)NULL);
		return TCL_ERROR;
	    }

	    /*
	     * Store the extracted indices for processing by sublist
	     * extraction. Note that we don't do this using objects because
	     * that has shimmering problems.
	     */

	    i++;
	    if (TclListObjGetElements(interp, objv[i],
		    &sortInfo.indexc, &indices) != TCL_OK) {
		if (startPtr != NULL) {
		    Tcl_DecrRefCount(startPtr);
		}
		return TCL_ERROR;
	    }
	    switch (sortInfo.indexc) {
	    case 0:
		sortInfo.indexv = NULL;
		break;
	    case 1:
		sortInfo.indexv = &sortInfo.singleIndex;
		break;
	    default:
		sortInfo.indexv = (int *)
			TclStackAlloc(interp, sizeof(int) * sortInfo.indexc);
	    }

	    /*
	     * Fill the array by parsing each index. We don't know whether
	     * their scale is sensible yet, but we at least perform the
	     * syntactic check here.
	     */

	    for (j=0 ; j<sortInfo.indexc ; j++) {
		int encoded = 0;
		if (TclIndexEncode(interp, indices[j], TCL_INDEX_BEFORE,
			TCL_INDEX_AFTER, &encoded) != TCL_OK) {
		    result = TCL_ERROR;
		}
		if ((encoded == TCL_INDEX_BEFORE)
			|| (encoded == TCL_INDEX_AFTER)) {
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "index \"%s\" cannot select an element "
			    "from any list", TclGetString(indices[j])));
		    Tcl_SetErrorCode(interp, "TCL", "VALUE", "INDEX"
			    "OUTOFRANGE", (char *)NULL);
		    result = TCL_ERROR;
		}
		if (result == TCL_ERROR) {
		    Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
			    "\n    (-index option item number %d)", j));
		    goto done;
		}
		sortInfo.indexv[j] = encoded;
	    }
	    break;
	}
	}
    }

    /*
     * Subindices only make sense if asked for with -index option set.
     */

    if (returnSubindices && sortInfo.indexc==0) {
	if (startPtr != NULL) {
	    Tcl_DecrRefCount(startPtr);
	}
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"-subindices cannot be used without -index option", -1));
	Tcl_SetErrorCode(interp, "TCL", "OPERATION", "LSEARCH",
		"BAD_OPTION_MIX", (char *)NULL);
	return TCL_ERROR;
    }

    if (bisect && (allMatches || negatedMatch)) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"-bisect is not compatible with -all or -not", -1));
	Tcl_SetErrorCode(interp, "TCL", "OPERATION", "LSEARCH",
		"BAD_OPTION_MIX", (char *)NULL);
	return TCL_ERROR;
    }

    if (mode == REGEXP) {
	/*
	 * We can shimmer regexp/list if listv[i] == pattern, so get the
	 * regexp rep before the list rep. First time round, omit the interp
	 * and hope that the compilation will succeed. If it fails, we'll
	 * recompile in "expensive" mode with a place to put error messages.
	 */

	regexp = Tcl_GetRegExpFromObj(NULL, objv[objc - 1],
		TCL_REG_ADVANCED | TCL_REG_NOSUB |
		(noCase ? TCL_REG_NOCASE : 0));
	if (regexp == NULL) {
	    /*
	     * Failed to compile the RE. Try again without the TCL_REG_NOSUB
	     * flag in case the RE had sub-expressions in it [Bug 1366683]. If
	     * this fails, an error message will be left in the interpreter.
	     */

	    regexp = Tcl_GetRegExpFromObj(interp, objv[objc - 1],
		    TCL_REG_ADVANCED | (noCase ? TCL_REG_NOCASE : 0));
	}

	if (regexp == NULL) {
	    if (startPtr != NULL) {
		Tcl_DecrRefCount(startPtr);
	    }
	    result = TCL_ERROR;
	    goto done;
	}
    }

    /*
     * Make sure the list argument is a list object and get its length and a
     * pointer to its array of element pointers.
     */

    result = TclListObjGetElements(interp, objv[objc - 2], &listc, &listv);
    if (result != TCL_OK) {
	if (startPtr != NULL) {
	    Tcl_DecrRefCount(startPtr);
	}
	goto done;
    }

    /*
     * Get the user-specified start offset.
     */

    if (startPtr) {
	result = TclGetIntForIndexM(interp, startPtr, listc-1, &offset);
	Tcl_DecrRefCount(startPtr);
	if (result != TCL_OK) {
	    goto done;
	}
	if (offset < 0) {
	    offset = 0;
	}

	/*
	 * If the search started past the end of the list, we just return a
	 * "did not match anything at all" result straight away. [Bug 1374778]
	 */

	if (offset > listc-1) {
	    if (sortInfo.indexc > 1) {
		TclStackFree(interp, sortInfo.indexv);
	    }
	    if (allMatches || inlineReturn) {
		Tcl_ResetResult(interp);
	    } else {
		Tcl_SetObjResult(interp, Tcl_NewIntObj(-1));
	    }
	    return TCL_OK;
	}
    }

    patObj = objv[objc - 1];
    patternBytes = NULL;
    if (mode == EXACT || mode == SORTED) {
	switch ((enum datatypes) dataType) {
	case ASCII:
	case DICTIONARY:
	    patternBytes = TclGetStringFromObj(patObj, &length);
	    break;
	case INTEGER:
	    result = TclGetWideIntFromObj(interp, patObj, &patWide);
	    if (result != TCL_OK) {
		goto done;
	    }

	    /*
	     * List representation might have been shimmered; restore it. [Bug
	     * 1844789]
	     */

	    TclListObjGetElements(NULL, objv[objc - 2], &listc, &listv);
	    break;
	case REAL:
	    result = Tcl_GetDoubleFromObj(interp, patObj, &patDouble);
	    if (result != TCL_OK) {
		goto done;
	    }

	    /*
	     * List representation might have been shimmered; restore it. [Bug
	     * 1844789]
	     */

	    TclListObjGetElements(NULL, objv[objc - 2], &listc, &listv);
	    break;
	}
    } else {
	patternBytes = TclGetStringFromObj(patObj, &length);
    }

    /*
     * Set default index value to -1, indicating failure; if we find the item
     * in the course of our search, index will be set to the correct value.
     */

    index = -1;
    match = 0;

    if (mode == SORTED && !allMatches && !negatedMatch) {
	/*
	 * If the data is sorted, we can do a more intelligent search. Note
	 * that there is no point in being smart when -all was specified; in
	 * that case, we have to look at all items anyway, and there is no
	 * sense in doing this when the match sense is inverted.
	 */

	lower = offset - 1;
	upper = listc;
	while (lower + 1 != upper && sortInfo.resultCode == TCL_OK) {
	    i = (lower + upper)/2;
	    if (sortInfo.indexc != 0) {
		itemPtr = SelectObjFromSublist(listv[i], &sortInfo);
		if (sortInfo.resultCode != TCL_OK) {
		    result = sortInfo.resultCode;
		    goto done;
		}
	    } else {
		itemPtr = listv[i];
	    }
	    switch ((enum datatypes) dataType) {
	    case ASCII:
		bytes = TclGetString(itemPtr);
		match = strCmpFn(patternBytes, bytes);
		break;
	    case DICTIONARY:
		bytes = TclGetString(itemPtr);
		match = DictionaryCompare(patternBytes, bytes);
		break;
	    case INTEGER:
		result = TclGetWideIntFromObj(interp, itemPtr, &objWide);
		if (result != TCL_OK) {
		    goto done;
		}
		if (patWide == objWide) {
		    match = 0;
		} else if (patWide < objWide) {
		    match = -1;
		} else {
		    match = 1;
		}
		break;
	    case REAL:
		result = Tcl_GetDoubleFromObj(interp, itemPtr, &objDouble);
		if (result != TCL_OK) {
		    goto done;
		}
		if (patDouble == objDouble) {
		    match = 0;
		} else if (patDouble < objDouble) {
		    match = -1;
		} else {
		    match = 1;
		}
		break;
	    }
	    if (match == 0) {
		/*
		 * Normally, binary search is written to stop when it finds a
		 * match. If there are duplicates of an element in the list,
		 * our first match might not be the first occurrence.
		 * Consider: 0 0 0 1 1 1 2 2 2
		 *
		 * To maintain consistency with standard lsearch semantics, we
		 * must find the leftmost occurrence of the pattern in the
		 * list. Thus we don't just stop searching here. This
		 * variation means that a search always makes log n
		 * comparisons (normal binary search might "get lucky" with an
		 * early comparison).
		 *
		 * In bisect mode though, we want the last of equals.
		 */

		index = i;
		if (bisect) {
		    lower = i;
		} else {
		    upper = i;
		}
	    } else if (match > 0) {
		if (isIncreasing) {
		    lower = i;
		} else {
		    upper = i;
		}
	    } else {
		if (isIncreasing) {
		    upper = i;
		} else {
		    lower = i;
		}
	    }
	}
	if (bisect && index < 0) {
	    index = lower;
	}
    } else {
	/*
	 * We need to do a linear search, because (at least one) of:
	 *   - our matcher can only tell equal vs. not equal
	 *   - our matching sense is negated
	 *   - we're building a list of all matched items
	 */

	if (allMatches) {
	    listPtr = Tcl_NewListObj(0, NULL);
	}
	for (i = offset; i < listc; i++) {
	    match = 0;
	    if (sortInfo.indexc != 0) {
		itemPtr = SelectObjFromSublist(listv[i], &sortInfo);
		if (sortInfo.resultCode != TCL_OK) {
		    if (listPtr != NULL) {
			Tcl_DecrRefCount(listPtr);
		    }
		    result = sortInfo.resultCode;
		    goto done;
		}
	    } else {
		itemPtr = listv[i];
	    }

	    switch (mode) {
	    case SORTED:
	    case EXACT:
		switch ((enum datatypes) dataType) {
		case ASCII:
		    bytes = TclGetStringFromObj(itemPtr, &elemLen);
		    if (length == elemLen) {
			/*
			 * This split allows for more optimal compilation of
			 * memcmp/strcasecmp.
			 */

			if (noCase) {
			    match = (TclUtfCasecmp(bytes, patternBytes) == 0);
			} else {
			    match = (memcmp(bytes, patternBytes,
				    (size_t) length) == 0);
			}
		    }
		    break;

		case DICTIONARY:
		    bytes = TclGetString(itemPtr);
		    match = (DictionaryCompare(bytes, patternBytes) == 0);
		    break;

		case INTEGER:
		    result = TclGetWideIntFromObj(interp, itemPtr, &objWide);
		    if (result != TCL_OK) {
			if (listPtr != NULL) {
			    Tcl_DecrRefCount(listPtr);
			}
			goto done;
		    }
		    match = (objWide == patWide);
		    break;

		case REAL:
		    result = Tcl_GetDoubleFromObj(interp,itemPtr, &objDouble);
		    if (result != TCL_OK) {
			if (listPtr) {
			    Tcl_DecrRefCount(listPtr);
			}
			goto done;
		    }
		    match = (objDouble == patDouble);
		    break;
		}
		break;

	    case GLOB:
		match = Tcl_StringCaseMatch(TclGetString(itemPtr),
			patternBytes, noCase);
		break;

	    case REGEXP:
		match = Tcl_RegExpExecObj(interp, regexp, itemPtr, 0, 0, 0);
		if (match < 0) {
		    Tcl_DecrRefCount(patObj);
		    if (listPtr != NULL) {
			Tcl_DecrRefCount(listPtr);
		    }
		    result = TCL_ERROR;
		    goto done;
		}
		break;
	    }

	    /*
	     * Invert match condition for -not.
	     */

	    if (negatedMatch) {
		match = !match;
	    }
	    if (!match) {
		continue;
	    }
	    if (!allMatches) {
		index = i;
		break;
	    } else if (inlineReturn) {
		/*
		 * Note that these appends are not expected to fail.
		 */

		if (returnSubindices && (sortInfo.indexc != 0)) {
		    itemPtr = SelectObjFromSublist(listv[i], &sortInfo);
		} else {
		    itemPtr = listv[i];
		}
		Tcl_ListObjAppendElement(interp, listPtr, itemPtr);
	    } else if (returnSubindices) {
		int j;

		TclNewIntObj(itemPtr, i);
		for (j=0 ; j<sortInfo.indexc ; j++) {
		    Tcl_ListObjAppendElement(interp, itemPtr, Tcl_NewIntObj(
			    TclIndexDecode(sortInfo.indexv[j], listc)));
		}
		Tcl_ListObjAppendElement(interp, listPtr, itemPtr);
	    } else {
		Tcl_ListObjAppendElement(interp, listPtr, Tcl_NewIntObj(i));
	    }
	}
    }

    /*
     * Return everything or a single value.
     */

    if (allMatches) {
	Tcl_SetObjResult(interp, listPtr);
    } else if (!inlineReturn) {
	if (returnSubindices) {
	    int j;

	    TclNewIntObj(itemPtr, index);
	    for (j=0 ; j<sortInfo.indexc ; j++) {
		Tcl_ListObjAppendElement(interp, itemPtr, Tcl_NewIntObj(
			TclIndexDecode(sortInfo.indexv[j], listc)));
	    }
	    Tcl_SetObjResult(interp, itemPtr);
	} else {
	    Tcl_SetObjResult(interp, Tcl_NewIntObj(index));
	}
    } else if (index < 0) {
	/*
	 * Is this superfluous? The result should be a blank object by
	 * default...
	 */

	Tcl_SetObjResult(interp, Tcl_NewObj());
    } else {
	Tcl_SetObjResult(interp, listv[index]);
    }
    result = TCL_OK;

    /*
     * Cleanup the index list array.
     */

  done:
    if (sortInfo.indexc > 1) {
	TclStackFree(interp, sortInfo.indexv);
    }
    return result;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdIL.c`, function `Tcl_LsortObjCmd: input-order original numeric preparation and list merge producer`, lines 3651–4140. Line unit: physical LF only. Original byte range [98374, 112093). Retained reference `source-window-9`.

```text
Tcl_LsortObjCmd(
    ClientData clientData,	/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument values. */
{
    int i, j, index, indices, length, nocase = 0, indexc;
    int sortMode = SORTMODE_ASCII;
    int group, groupSize, groupOffset, idx, allocatedIndexVector = 0;
    Tcl_Obj *resultPtr, *cmdPtr, **listObjPtrs, *listObj, *indexPtr;
    size_t elmArrSize;
    SortElement *elementArray = NULL, *elementPtr;
    SortInfo sortInfo;		/* Information about this sort that needs to
				 * be passed to the comparison function. */
#   define MAXCALLOC 1024000
#   define NUM_LISTS 30
    SortElement *subList[NUM_LISTS+1];
				/* This array holds pointers to temporary
				 * lists built during the merge sort. Element
				 * i of the array holds a list of length
				 * 2**i. */
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

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-option value ...? list");
	return TCL_ERROR;
    }

    /*
     * Parse arguments to set up the mode for the sort.
     */

    sortInfo.isIncreasing = 1;
    sortInfo.sortMode = SORTMODE_ASCII;
    sortInfo.indexv = NULL;
    sortInfo.indexc = 0;
    sortInfo.unique = 0;
    sortInfo.interp = interp;
    sortInfo.resultCode = TCL_OK;
    cmdPtr = NULL;
    indices = 0;
    group = 0;
    groupSize = 1;
    groupOffset = 0;
    indexPtr = NULL;
    for (i = 1; i < objc-1; i++) {
	if (Tcl_GetIndexFromObj(interp, objv[i], switches, "option", 0,
		&index) != TCL_OK) {
	    sortInfo.resultCode = TCL_ERROR;
	    goto done;
	}
	switch ((enum Lsort_Switches) index) {
	case LSORT_ASCII:
	    sortInfo.sortMode = SORTMODE_ASCII;
	    break;
	case LSORT_COMMAND:
	    if (i == objc-2) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"\"-command\" option must be followed "
			"by comparison command", -1));
		Tcl_SetErrorCode(interp, "TCL", "ARGUMENT", "MISSING", (char *)NULL);
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    sortInfo.sortMode = SORTMODE_COMMAND;
	    cmdPtr = objv[i+1];
	    i++;
	    break;
	case LSORT_DECREASING:
	    sortInfo.isIncreasing = 0;
	    break;
	case LSORT_DICTIONARY:
	    sortInfo.sortMode = SORTMODE_DICTIONARY;
	    break;
	case LSORT_INCREASING:
	    sortInfo.isIncreasing = 1;
	    break;
	case LSORT_INDEX: {
	    int sortindex;
	    Tcl_Obj **indexv;

	    if (i == objc-2) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"\"-index\" option must be followed by list index",
			-1));
		Tcl_SetErrorCode(interp, "TCL", "ARGUMENT", "MISSING", (char *)NULL);
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    if (TclListObjGetElements(interp, objv[i+1], &sortindex,
		    &indexv) != TCL_OK) {
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }

	    /*
	     * Check each of the indices for syntactic correctness. Note that
	     * we do not store the converted values here because we do not
	     * know if this is the only -index option yet and so we can't
	     * allocate any space; that happens after the scan through all the
	     * options is done.
	     */

	    for (j=0 ; j<sortindex ; j++) {
		int encoded = 0;
		int result = TclIndexEncode(interp, indexv[j],
			TCL_INDEX_BEFORE, TCL_INDEX_AFTER, &encoded);

		if ((result == TCL_OK) && ((encoded == TCL_INDEX_BEFORE)
			|| (encoded == TCL_INDEX_AFTER))) {
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "index \"%s\" cannot select an element "
			    "from any list", TclGetString(indexv[j])));
		    Tcl_SetErrorCode(interp, "TCL", "VALUE", "INDEX"
			    "OUTOFRANGE", (char *)NULL);
		    result = TCL_ERROR;
		}
		if (result == TCL_ERROR) {
		    Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
			    "\n    (-index option item number %d)", j));
		    sortInfo.resultCode = TCL_ERROR;
		    goto done;
		}
	    }
	    indexPtr = objv[i+1];
	    i++;
	    break;
	}
	case LSORT_INTEGER:
	    sortInfo.sortMode = SORTMODE_INTEGER;
	    break;
	case LSORT_NOCASE:
	    nocase = 1;
	    break;
	case LSORT_REAL:
	    sortInfo.sortMode = SORTMODE_REAL;
	    break;
	case LSORT_UNIQUE:
	    sortInfo.unique = 1;
	    break;
	case LSORT_INDICES:
	    indices = 1;
	    break;
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
	}
    }
    if (nocase && (sortInfo.sortMode == SORTMODE_ASCII)) {
	sortInfo.sortMode = SORTMODE_ASCII_NC;
    }

    /*
     * Now extract the -index list for real, if present. No failures are
     * expected here; the values are all of the right type or convertible to
     * it.
     */

    if (indexPtr) {
	Tcl_Obj **indexv;

	TclListObjGetElements(interp, indexPtr, &sortInfo.indexc, &indexv);
	switch (sortInfo.indexc) {
	case 0:
	    sortInfo.indexv = NULL;
	    break;
	case 1:
	    sortInfo.indexv = &sortInfo.singleIndex;
	    break;
	default:
	    sortInfo.indexv = (int *)
		    TclStackAlloc(interp, sizeof(int) * sortInfo.indexc);
	    allocatedIndexVector = 1;	/* Cannot use indexc field, as it
					 * might be decreased by 1 later. */
	}
	for (j=0 ; j<sortInfo.indexc ; j++) {
	    /* Prescreened values, no errors or out of range possible */
	    TclIndexEncode(NULL, indexv[j], 0, 0, &sortInfo.indexv[j]);
	}
    }

    listObj = objv[objc-1];

    if (sortInfo.sortMode == SORTMODE_COMMAND) {
	Tcl_Obj *newCommandPtr, *newObjPtr;

	/*
	 * When sorting using a command, we are reentrant and therefore might
	 * have the representation of the list being sorted shimmered out from
	 * underneath our feet. Take a copy (cheap) to prevent this. [Bug
	 * 1675116]
	 */

	listObj = TclListObjCopy(interp, listObj);
	if (listObj == NULL) {
	    sortInfo.resultCode = TCL_ERROR;
	    goto done;
	}

	/*
	 * The existing command is a list. We want to flatten it, append two
	 * dummy arguments on the end, and replace these arguments later.
	 */

	newCommandPtr = Tcl_DuplicateObj(cmdPtr);
	TclNewObj(newObjPtr);
	Tcl_IncrRefCount(newCommandPtr);
	if (Tcl_ListObjAppendElement(interp, newCommandPtr, newObjPtr)
		!= TCL_OK) {
	    TclDecrRefCount(newCommandPtr);
	    TclDecrRefCount(listObj);
	    Tcl_IncrRefCount(newObjPtr);
	    TclDecrRefCount(newObjPtr);
	    sortInfo.resultCode = TCL_ERROR;
	    goto done;
	}
	Tcl_ListObjAppendElement(interp, newCommandPtr, Tcl_NewObj());
	sortInfo.compareCmdPtr = newCommandPtr;
    }

    sortInfo.resultCode = TclListObjGetElements(interp, listObj,
	    &length, &listObjPtrs);
    if (sortInfo.resultCode != TCL_OK || length <= 0) {
	goto done;
    }

    /*
     * Check for sanity when grouping elements of the overall list together
     * because of the -stride option. [TIP #326]
     */

    if (group) {
	if (length % groupSize) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "list size must be a multiple of the stride length",
		    -1));
	    Tcl_SetErrorCode(interp, "TCL", "OPERATION", "LSORT", "BADSTRIDE",
		    (char *)NULL);
	    sortInfo.resultCode = TCL_ERROR;
	    goto done;
	}
	length = length / groupSize;
	if (sortInfo.indexc > 0) {
	    /*
	     * Use the first value in the list supplied to -index as the
	     * offset of the element within each group by which to sort.
	     */

	    groupOffset = TclIndexDecode(sortInfo.indexv[0], groupSize - 1);
	    if (groupOffset < 0 || groupOffset >= groupSize) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"when used with \"-stride\", the leading \"-index\""
			" value must be within the group", -1));
		Tcl_SetErrorCode(interp, "TCL", "OPERATION", "LSORT",
			"BADINDEX", (char *)NULL);
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    if (sortInfo.indexc == 1) {
		sortInfo.indexc = 0;
		sortInfo.indexv = NULL;
	    } else {
		sortInfo.indexc--;

		/*
		 * Do not shrink the actual memory block used; that doesn't
		 * work with TclStackAlloc-allocated memory. [Bug 2918962]
		 *
		 * TODO: Consider a pointer increment to replace this
		 * array shift.
		 */

		for (i = 0; i < sortInfo.indexc; i++) {
		    sortInfo.indexv[i] = sortInfo.indexv[i+1];
		}
	    }
	}
    }

    sortInfo.numElements = length;

    indexc = sortInfo.indexc;
    sortMode = sortInfo.sortMode;
    if ((sortMode == SORTMODE_ASCII_NC)
	    || (sortMode == SORTMODE_DICTIONARY)) {
	/*
	 * For this function's purpose all string-based modes are equivalent
	 */

	sortMode = SORTMODE_ASCII;
    }

    /*
     * Initialize the sublists. After the following loop, subList[i] will
     * contain a sorted sublist of length 2**i. Use one extra subList at the
     * end, always at NULL, to indicate the end of the lists.
     */

    for (j=0 ; j<=NUM_LISTS ; j++) {
	subList[j] = NULL;
    }

    /*
     * The following loop creates a SortElement for each list element and
     * begins sorting it into the sublists as it appears.
     */

    elmArrSize = length * sizeof(SortElement);
    if (elmArrSize <= MAXCALLOC) {
	elementArray = (SortElement *)ckalloc(elmArrSize);
    } else {
	elementArray = (SortElement *)malloc(elmArrSize);
    }
    if (!elementArray) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"no enough memory to proccess sort of %d items", length));
	Tcl_SetErrorCode(interp, "TCL", "MEMORY", (char *)NULL);
	sortInfo.resultCode = TCL_ERROR;
	goto done;
    }

    for (i=0; i < length; i++){
	idx = groupSize * i + groupOffset;
	if (indexc) {
	    /*
	     * If this is an indexed sort, retrieve the corresponding element
	     */
	    indexPtr = SelectObjFromSublist(listObjPtrs[idx], &sortInfo);
	    if (sortInfo.resultCode != TCL_OK) {
		goto done;
	    }
	} else {
	    indexPtr = listObjPtrs[idx];
	}

	/*
	 * Determine the "value" of this object for sorting purposes
	 */

	if (sortMode == SORTMODE_ASCII) {
	    elementArray[i].collationKey.strValuePtr = TclGetString(indexPtr);
	} else if (sortMode == SORTMODE_INTEGER) {
	    Tcl_WideInt a;

	    if (TclGetWideIntFromObj(sortInfo.interp, indexPtr, &a) != TCL_OK) {
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    elementArray[i].collationKey.wideValue = a;
	} else if (sortMode == SORTMODE_REAL) {
	    double a;

	    if (Tcl_GetDoubleFromObj(sortInfo.interp, indexPtr,
		    &a) != TCL_OK) {
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    elementArray[i].collationKey.doubleValue = a;
	} else {
	    elementArray[i].collationKey.objValuePtr = indexPtr;
	}

	/*
	 * Determine the representation of this element in the result: either
	 * the objPtr itself, or its index in the original list.
	 */

	if (indices || group) {
	    elementArray[i].payload.index = idx;
	} else {
	    elementArray[i].payload.objPtr = listObjPtrs[idx];
	}

	/*
	 * Merge this element in the preexisting sublists (and merge together
	 * sublists when we have two of the same size).
	 */

	elementArray[i].nextPtr = NULL;
	elementPtr = &elementArray[i];
	for (j=0 ; subList[j] ; j++) {
	    elementPtr = MergeLists(subList[j], elementPtr, &sortInfo);
	    subList[j] = NULL;
	}
	if (j >= NUM_LISTS) {
	    j = NUM_LISTS-1;
	}
	subList[j] = elementPtr;
    }

    /*
     * Merge all sublists
     */

    elementPtr = subList[0];
    for (j=1 ; j<NUM_LISTS ; j++) {
	elementPtr = MergeLists(subList[j], elementPtr, &sortInfo);
    }

    /*
     * Now store the sorted elements in the result list.
     */

    if (sortInfo.resultCode == TCL_OK) {
	List *listRepPtr;
	Tcl_Obj **newArray, *objPtr;

	resultPtr = Tcl_NewListObj(sortInfo.numElements * groupSize, NULL);
	listRepPtr = ListRepPtr(resultPtr);
	newArray = &listRepPtr->elements;
	if (group) {
	    for (i=0; elementPtr!=NULL ; elementPtr=elementPtr->nextPtr) {
		idx = elementPtr->payload.index;
		for (j = 0; j < groupSize; j++) {
		    if (indices) {
			TclNewIntObj(objPtr, idx + j - groupOffset);
			newArray[i++] = objPtr;
			Tcl_IncrRefCount(objPtr);
		    } else {
			objPtr = listObjPtrs[idx + j - groupOffset];
			newArray[i++] = objPtr;
			Tcl_IncrRefCount(objPtr);
		    }
		}
	    }
	} else if (indices) {
	    for (i=0; elementPtr != NULL ; elementPtr = elementPtr->nextPtr) {
		TclNewIntObj(objPtr, elementPtr->payload.index);
		newArray[i++] = objPtr;
		Tcl_IncrRefCount(objPtr);
	    }
	} else {
	    for (i=0; elementPtr != NULL ; elementPtr = elementPtr->nextPtr) {
		objPtr = elementPtr->payload.objPtr;
		newArray[i++] = objPtr;
		Tcl_IncrRefCount(objPtr);
	    }
	}
	listRepPtr->elemCount = i;
	Tcl_SetObjResult(interp, resultPtr);
    }

  done:
    if (sortMode == SORTMODE_COMMAND) {
	TclDecrRefCount(sortInfo.compareCmdPtr);
	TclDecrRefCount(listObj);
	sortInfo.compareCmdPtr = NULL;
    }
    if (allocatedIndexVector) {
	TclStackFree(interp, sortInfo.indexv);
    }
    if (elementArray) {
	if (elmArrSize <= MAXCALLOC) {
	    ckfree((char *)elementArray);
	} else {
	    free((char *)elementArray);
	}
    }
    return sortInfo.resultCode;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdIL.c`, function `SortCompare: comparison of prepared numeric values and command result primitive`, lines 4263–4349. Line unit: physical LF only. Original byte range [115541, 118077). Retained reference `source-window-10`.

```text
SortCompare(
    SortElement *elemPtr1, SortElement *elemPtr2,
				/* Values to be compared. */
    SortInfo *infoPtr)		/* Information passed from the top-level
				 * "lsort" command. */
{
    int order = 0;

    if (infoPtr->sortMode == SORTMODE_ASCII) {
	order = strcmp(elemPtr1->collationKey.strValuePtr,
		elemPtr2->collationKey.strValuePtr);
    } else if (infoPtr->sortMode == SORTMODE_ASCII_NC) {
	order = TclUtfCasecmp(elemPtr1->collationKey.strValuePtr,
		elemPtr2->collationKey.strValuePtr);
    } else if (infoPtr->sortMode == SORTMODE_DICTIONARY) {
	order = DictionaryCompare(elemPtr1->collationKey.strValuePtr,
		elemPtr2->collationKey.strValuePtr);
    } else if (infoPtr->sortMode == SORTMODE_INTEGER) {
	Tcl_WideInt a, b;

	a = elemPtr1->collationKey.wideValue;
	b = elemPtr2->collationKey.wideValue;
	order = ((a >= b) - (a <= b));
    } else if (infoPtr->sortMode == SORTMODE_REAL) {
	double a, b;

	a = elemPtr1->collationKey.doubleValue;
	b = elemPtr2->collationKey.doubleValue;
	order = ((a >= b) - (a <= b));
    } else {
	Tcl_Obj **objv, *paramObjv[2];
	int objc;
	Tcl_Obj *objPtr1, *objPtr2;

	if (infoPtr->resultCode != TCL_OK) {
	    /*
	     * Once an error has occurred, skip any future comparisons so as
	     * to preserve the error message in sortInterp->result.
	     */

	    return 0;
	}

	objPtr1 = elemPtr1->collationKey.objValuePtr;
	objPtr2 = elemPtr2->collationKey.objValuePtr;

	paramObjv[0] = objPtr1;
	paramObjv[1] = objPtr2;

	/*
	 * We made space in the command list for the two things to compare.
	 * Replace them and evaluate the result.
	 */

	TclListObjLength(infoPtr->interp, infoPtr->compareCmdPtr, &objc);
	Tcl_ListObjReplace(infoPtr->interp, infoPtr->compareCmdPtr, objc - 2,
		2, 2, paramObjv);
	TclListObjGetElements(infoPtr->interp, infoPtr->compareCmdPtr,
		&objc, &objv);

	infoPtr->resultCode = Tcl_EvalObjv(infoPtr->interp, objc, objv, 0);

	if (infoPtr->resultCode != TCL_OK) {
	    Tcl_AddErrorInfo(infoPtr->interp, "\n    (-compare command)");
	    return 0;
	}

	/*
	 * Parse the result of the command.
	 */

	if (TclGetIntFromObj(infoPtr->interp,
		Tcl_GetObjResult(infoPtr->interp), &order) != TCL_OK) {
	    Tcl_SetObjResult(infoPtr->interp, Tcl_NewStringObj(
		    "-compare command returned non-integer result", -1));
	    Tcl_SetErrorCode(infoPtr->interp, "TCL", "OPERATION", "LSORT",
		    "COMPARISONFAILED", (char *)NULL);
	    infoPtr->resultCode = TCL_ERROR;
	    return 0;
	}
    }
    if (!infoPtr->isIncreasing) {
	order = -order;
    }
    return order;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdIL.c`, function `Tcl_LsearchObjCmd: default table, selected original pattern and reached numeric keys`, lines 3256–4017. Line unit: physical LF only. Original byte range [87244, 107092). Retained reference `source-window-11`.

```text
Tcl_LsearchObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument values. */
{
    const char *bytes, *patternBytes;
    int match, result=TCL_OK, bisect;
    Tcl_Size i, length = 0, listc, elemLen, start, index;
    Tcl_Size groupOffset, lower, upper;
    int allocatedIndexVector = 0;
    int isIncreasing;
    Tcl_WideInt patWide, objWide, wide, groupSize;
    int allMatches, inlineReturn, negatedMatch, returnSubindices, noCase;
    double patDouble, objDouble;
    SortInfo sortInfo;
    Tcl_Obj *patObj, **listv, *listPtr, *startPtr, *itemPtr = NULL;
    SortStrCmpFn_t strCmpFn = TclUtfCmp;
    Tcl_RegExp regexp = NULL;
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
    enum datatypes {
	ASCII, DICTIONARY, INTEGER, REAL
    } dataType;
    enum modes {
	EXACT, GLOB, REGEXP, SORTED
    };
    enum modes mode;

    mode = GLOB;
    dataType = ASCII;
    isIncreasing = 1;
    allMatches = 0;
    inlineReturn = 0;
    returnSubindices = 0;
    negatedMatch = 0;
    bisect = 0;
    listPtr = NULL;
    startPtr = NULL;
    groupSize = 1;
    groupOffset = 0;
    start = 0;
    noCase = 0;
    sortInfo.compareCmdPtr = NULL;
    sortInfo.isIncreasing = 1;
    sortInfo.sortMode = 0;
    sortInfo.interp = interp;
    sortInfo.resultCode = TCL_OK;
    sortInfo.indexv = NULL;
    sortInfo.indexc = 0;

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-option value ...? list pattern");
	return TCL_ERROR;
    }

    for (i = 1; i < objc-2; i++) {
	enum lsearchoptions idx;
	if (Tcl_GetIndexFromObj(interp, objv[i], options, "option", 0, &idx)
		!= TCL_OK) {
	    result = TCL_ERROR;
	    goto done;
	}
	switch (idx) {
	case LSEARCH_ALL:		/* -all */
	    allMatches = 1;
	    break;
	case LSEARCH_ASCII:		/* -ascii */
	    dataType = ASCII;
	    break;
	case LSEARCH_BISECT:		/* -bisect */
	    mode = SORTED;
	    bisect = 1;
	    break;
	case LSEARCH_DECREASING:	/* -decreasing */
	    isIncreasing = 0;
	    sortInfo.isIncreasing = 0;
	    break;
	case LSEARCH_DICTIONARY:	/* -dictionary */
	    dataType = DICTIONARY;
	    break;
	case LSEARCH_EXACT:		/* -increasing */
	    mode = EXACT;
	    break;
	case LSEARCH_GLOB:		/* -glob */
	    mode = GLOB;
	    break;
	case LSEARCH_INCREASING:	/* -increasing */
	    isIncreasing = 1;
	    sortInfo.isIncreasing = 1;
	    break;
	case LSEARCH_INLINE:		/* -inline */
	    inlineReturn = 1;
	    break;
	case LSEARCH_INTEGER:		/* -integer */
	    dataType = INTEGER;
	    break;
	case LSEARCH_NOCASE:		/* -nocase */
	    strCmpFn = TclUtfCasecmp;
	    noCase = 1;
	    break;
	case LSEARCH_NOT:		/* -not */
	    negatedMatch = 1;
	    break;
	case LSEARCH_REAL:		/* -real */
	    dataType = REAL;
	    break;
	case LSEARCH_REGEXP:		/* -regexp */
	    mode = REGEXP;
	    break;
	case LSEARCH_SORTED:		/* -sorted */
	    mode = SORTED;
	    break;
	case LSEARCH_SUBINDICES:	/* -subindices */
	    returnSubindices = 1;
	    break;
	case LSEARCH_START:		/* -start */
	    /*
	     * If there was a previous -start option, release its saved index
	     * because it will either be replaced or there will be an error.
	     */

	    if (startPtr != NULL) {
		Tcl_DecrRefCount(startPtr);
		startPtr = NULL;
	    }
	    if (i > objc-4) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"missing starting index", -1));
		Tcl_SetErrorCode(interp, "TCL", "ARGUMENT", "MISSING", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    i++;
	    if (objv[i] == objv[objc - 2]) {
		/*
		 * Take copy to prevent shimmering problems. Note that it does
		 * not matter if the index obj is also a component of the list
		 * being searched. We only need to copy where the list and the
		 * index are one-and-the-same.
		 */

		startPtr = Tcl_DuplicateObj(objv[i]);
	    } else {
		startPtr = objv[i];
	    }
	    Tcl_IncrRefCount(startPtr);
	    break;
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
	case LSEARCH_INDEX: {		/* -index */
	    Tcl_Obj **indices;
	    Tcl_Size j;

	    if (allocatedIndexVector) {
		TclStackFree(interp, sortInfo.indexv);
		allocatedIndexVector = 0;
	    }
	    if (i > objc-4) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"\"-index\" option must be followed by list index",
			-1));
		Tcl_SetErrorCode(interp, "TCL", "ARGUMENT", "MISSING", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }

	    /*
	     * Store the extracted indices for processing by sublist
	     * extraction. Note that we don't do this using objects because
	     * that has shimmering problems.
	     */

	    i++;
	    if (TclListObjGetElements(interp, objv[i],
		    &sortInfo.indexc, &indices) != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    switch (sortInfo.indexc) {
	    case 0:
		sortInfo.indexv = NULL;
		break;
	    case 1:
		sortInfo.indexv = &sortInfo.singleIndex;
		break;
	    default:
		sortInfo.indexv = (int *)
			TclStackAlloc(interp, sizeof(int) * sortInfo.indexc);
		allocatedIndexVector = 1; /* Cannot use indexc field, as it
					   * might be decreased by 1 later. */
	    }

	    /*
	     * Fill the array by parsing each index. We don't know whether
	     * their scale is sensible yet, but we at least perform the
	     * syntactic check here.
	     */

	    for (j=0 ; j<sortInfo.indexc ; j++) {
		int encoded = 0;
		if (TclIndexEncode(interp, indices[j], TCL_INDEX_NONE,
			TCL_INDEX_NONE, &encoded) != TCL_OK) {
		    result = TCL_ERROR;
		}
		if (encoded == (int)TCL_INDEX_NONE) {
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "index \"%s\" out of range",
			    TclGetString(indices[j])));
		    Tcl_SetErrorCode(interp, "TCL", "VALUE", "INDEX", "OUTOFRANGE", (char *)NULL);
		    result = TCL_ERROR;
		}
		if (result == TCL_ERROR) {
		    Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
			    "\n    (-index option item number %" TCL_Z_MODIFIER "u)", j));
		    goto done;
		}
		sortInfo.indexv[j] = encoded;
	    }
	    break;
	}
	}
    }

    /*
     * Subindices only make sense if asked for with -index option set.
     */

    if (returnSubindices && sortInfo.indexc==0) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"-subindices cannot be used without -index option", -1));
	Tcl_SetErrorCode(interp, "TCL", "OPERATION", "LSEARCH",
		"BAD_OPTION_MIX", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    if (bisect && (allMatches || negatedMatch)) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"-bisect is not compatible with -all or -not", -1));
	Tcl_SetErrorCode(interp, "TCL", "OPERATION", "LSEARCH",
		"BAD_OPTION_MIX", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    if (mode == REGEXP) {
	/*
	 * We can shimmer regexp/list if listv[i] == pattern, so get the
	 * regexp rep before the list rep. First time round, omit the interp
	 * and hope that the compilation will succeed. If it fails, we'll
	 * recompile in "expensive" mode with a place to put error messages.
	 */

	regexp = Tcl_GetRegExpFromObj(NULL, objv[objc - 1],
		TCL_REG_ADVANCED | TCL_REG_NOSUB |
		(noCase ? TCL_REG_NOCASE : 0));
	if (regexp == NULL) {
	    /*
	     * Failed to compile the RE. Try again without the TCL_REG_NOSUB
	     * flag in case the RE had sub-expressions in it [Bug 1366683]. If
	     * this fails, an error message will be left in the interpreter.
	     */

	    regexp = Tcl_GetRegExpFromObj(interp, objv[objc - 1],
		    TCL_REG_ADVANCED | (noCase ? TCL_REG_NOCASE : 0));
	}

	if (regexp == NULL) {
	    result = TCL_ERROR;
	    goto done;
	}
    }

    /*
     * Make sure the list argument is a list object and get its length and a
     * pointer to its array of element pointers.
     */

    result = TclListObjGetElements(interp, objv[objc - 2], &listc, &listv);
    if (result != TCL_OK) {
	goto done;
    }

    /*
     * Check for sanity when grouping elements of the overall list together
     * because of the -stride option. [TIP #351]
     */

    if (groupSize > 1) {
	if (listc % groupSize) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "list size must be a multiple of the stride length",
		    -1));
	    Tcl_SetErrorCode(interp, "TCL", "OPERATION", "LSEARCH", "BADSTRIDE",
		    (char *)NULL);
	    result = TCL_ERROR;
	    goto done;
	}
	if (sortInfo.indexc > 0) {
	    /*
	     * Use the first value in the list supplied to -index as the
	     * offset of the element within each group by which to sort.
	     */

	    groupOffset = TclIndexDecode(sortInfo.indexv[0], groupSize - 1);
	    if (groupOffset < 0 || groupOffset >= groupSize) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"when used with \"-stride\", the leading \"-index\""
			" value must be within the group", -1));
		Tcl_SetErrorCode(interp, "TCL", "OPERATION", "LSEARCH",
			"BADINDEX", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    if (sortInfo.indexc == 1) {
		sortInfo.indexc = 0;
		sortInfo.indexv = NULL;
	    } else {
		sortInfo.indexc--;

		for (i = 0; i < sortInfo.indexc; i++) {
		    sortInfo.indexv[i] = sortInfo.indexv[i+1];
		}
	    }
	}
    }

    /*
     * Get the user-specified start offset.
     */

    if (startPtr) {
	result = TclGetIntForIndexM(interp, startPtr, listc-1, &start);
	if (result != TCL_OK) {
	    goto done;
	}
	if (start == TCL_INDEX_NONE) {
	    start = TCL_INDEX_START;
	}

	/*
	 * If the search started past the end of the list, we just return a
	 * "did not match anything at all" result straight away. [Bug 1374778]
	 */

	if (start >= listc) {
	    if (allMatches || inlineReturn) {
		Tcl_ResetResult(interp);
	    } else {
		TclNewIntObj(itemPtr, -1);
		Tcl_SetObjResult(interp, itemPtr);
	    }
	    goto done;
	}

	/*
	 * If start points within a group, it points to the start of the group.
	 */

	if (groupSize > 1) {
	    start -= (start % groupSize);
	}
    }

    patObj = objv[objc - 1];
    patternBytes = NULL;
    if (mode == EXACT || mode == SORTED) {
	switch (dataType) {
	case ASCII:
	case DICTIONARY:
	    patternBytes = TclGetStringFromObj(patObj, &length);
	    break;
	case INTEGER:
	    result = TclGetWideIntFromObj(interp, patObj, &patWide);
	    if (result != TCL_OK) {
		goto done;
	    }

	    /*
	     * List representation might have been shimmered; restore it. [Bug
	     * 1844789]
	     */

	    TclListObjGetElements(NULL, objv[objc - 2], &listc, &listv);
	    break;
	case REAL:
	    result = Tcl_GetDoubleFromObj(interp, patObj, &patDouble);
	    if (result != TCL_OK) {
		goto done;
	    }

	    /*
	     * List representation might have been shimmered; restore it. [Bug
	     * 1844789]
	     */

	    TclListObjGetElements(NULL, objv[objc - 2], &listc, &listv);
	    break;
	}
    } else {
	patternBytes = TclGetStringFromObj(patObj, &length);
    }

    /*
     * Set default index value to -1, indicating failure; if we find the item
     * in the course of our search, index will be set to the correct value.
     */

    index = -1;
    match = 0;

    if (mode == SORTED && !allMatches && !negatedMatch) {
	/*
	 * If the data is sorted, we can do a more intelligent search. Note
	 * that there is no point in being smart when -all was specified; in
	 * that case, we have to look at all items anyway, and there is no
	 * sense in doing this when the match sense is inverted.
	 */

	/*
	 * With -stride, lower, upper and i are kept as multiples of groupSize.
	 */

	lower = start - groupSize;
	upper = listc;
	itemPtr = NULL;
	while (lower + groupSize != upper && sortInfo.resultCode == TCL_OK) {
	    i = (lower + upper)/2;
	    i -= i % groupSize;

	    Tcl_BounceRefCount(itemPtr);
	    itemPtr = NULL;

	    if (sortInfo.indexc != 0) {
		itemPtr = SelectObjFromSublist(listv[i+groupOffset], &sortInfo);
		if (sortInfo.resultCode != TCL_OK) {
		    result = sortInfo.resultCode;
		    goto done;
		}
	    } else {
		itemPtr = listv[i+groupOffset];
	    }
	    switch (dataType) {
	    case ASCII:
		bytes = TclGetString(itemPtr);
		match = strCmpFn(patternBytes, bytes);
		break;
	    case DICTIONARY:
		bytes = TclGetString(itemPtr);
		match = DictionaryCompare(patternBytes, bytes);
		break;
	    case INTEGER:
		result = TclGetWideIntFromObj(interp, itemPtr, &objWide);
		if (result != TCL_OK) {
		    goto done;
		}
		if (patWide == objWide) {
		    match = 0;
		} else if (patWide < objWide) {
		    match = -1;
		} else {
		    match = 1;
		}
		break;
	    case REAL:
		result = Tcl_GetDoubleFromObj(interp, itemPtr, &objDouble);
		if (result != TCL_OK) {
		    goto done;
		}
		if (patDouble == objDouble) {
		    match = 0;
		} else if (patDouble < objDouble) {
		    match = -1;
		} else {
		    match = 1;
		}
		break;
	    }
	    if (match == 0) {
		/*
		 * Normally, binary search is written to stop when it finds a
		 * match. If there are duplicates of an element in the list,
		 * our first match might not be the first occurrence.
		 * Consider: 0 0 0 1 1 1 2 2 2
		 *
		 * To maintain consistency with standard lsearch semantics, we
		 * must find the leftmost occurrence of the pattern in the
		 * list. Thus we don't just stop searching here. This
		 * variation means that a search always makes log n
		 * comparisons (normal binary search might "get lucky" with an
		 * early comparison).
		 *
		 * In bisect mode though, we want the last of equals.
		 */

		index = i;
		if (bisect) {
		    lower = i;
		} else {
		    upper = i;
		}
	    } else if (match > 0) {
		if (isIncreasing) {
		    lower = i;
		} else {
		    upper = i;
		}
	    } else {
		if (isIncreasing) {
		    upper = i;
		} else {
		    lower = i;
		}
	    }
	}
	if (bisect && index < 0) {
	    index = lower;
	}
    } else {
	/*
	 * We need to do a linear search, because (at least one) of:
	 *   - our matcher can only tell equal vs. not equal
	 *   - our matching sense is negated
	 *   - we're building a list of all matched items
	 */

	if (allMatches) {
	    listPtr = Tcl_NewListObj(0, NULL);
	}
	for (i = start; i < listc; i += groupSize) {
	    match = 0;
	    Tcl_BounceRefCount(itemPtr);
	    itemPtr = NULL;

	    if (sortInfo.indexc != 0) {
		itemPtr = SelectObjFromSublist(listv[i+groupOffset], &sortInfo);
		if (sortInfo.resultCode != TCL_OK) {
		    if (listPtr != NULL) {
			Tcl_DecrRefCount(listPtr);
		    }
		    result = sortInfo.resultCode;
		    goto done;
		}
	    } else {
		itemPtr = listv[i+groupOffset];
	    }

	    switch (mode) {
	    case SORTED:
	    case EXACT:
		switch (dataType) {
		case ASCII:
		    bytes = TclGetStringFromObj(itemPtr, &elemLen);
		    if (length == elemLen) {
			/*
			 * This split allows for more optimal compilation of
			 * memcmp/strcasecmp.
			 */

			if (noCase) {
			    match = (TclUtfCasecmp(bytes, patternBytes) == 0);
			} else {
			    match = (memcmp(bytes, patternBytes, length) == 0);
			}
		    }
		    break;

		case DICTIONARY:
		    bytes = TclGetString(itemPtr);
		    match = (DictionaryCompare(bytes, patternBytes) == 0);
		    break;

		case INTEGER:
		    result = TclGetWideIntFromObj(interp, itemPtr, &objWide);
		    if (result != TCL_OK) {
			if (listPtr != NULL) {
			    Tcl_DecrRefCount(listPtr);
			}
			goto done;
		    }
		    match = (objWide == patWide);
		    break;

		case REAL:
		    result = Tcl_GetDoubleFromObj(interp,itemPtr, &objDouble);
		    if (result != TCL_OK) {
			if (listPtr) {
			    Tcl_DecrRefCount(listPtr);
			}
			goto done;
		    }
		    match = (objDouble == patDouble);
		    break;
		}
		break;

	    case GLOB:
		match = Tcl_StringCaseMatch(TclGetString(itemPtr),
			patternBytes, noCase);
		break;

	    case REGEXP:
		match = Tcl_RegExpExecObj(interp, regexp, itemPtr, 0, 0, 0);
		if (match < 0) {
		    Tcl_DecrRefCount(patObj);
		    if (listPtr != NULL) {
			Tcl_DecrRefCount(listPtr);
		    }
		    result = TCL_ERROR;
		    goto done;
		}
		break;
	    }

	    /*
	     * Invert match condition for -not.
	     */

	    if (negatedMatch) {
		match = !match;
	    }
	    if (!match) {
		continue;
	    }
	    if (!allMatches) {
		index = i;
		break;
	    } else if (inlineReturn) {
		/*
		 * Note that these appends are not expected to fail.
		 */

		if (returnSubindices && (sortInfo.indexc != 0)) {
		    Tcl_BounceRefCount(itemPtr);
		    itemPtr = SelectObjFromSublist(listv[i+groupOffset],
			    &sortInfo);
		    Tcl_ListObjAppendElement(interp, listPtr, itemPtr);
		} else if (returnSubindices && (sortInfo.indexc == 0) && (groupSize > 1)) {
		    Tcl_BounceRefCount(itemPtr);
		    itemPtr = listv[i + groupOffset];
			Tcl_ListObjAppendElement(interp, listPtr, itemPtr);
		} else if (groupSize > 1) {
		    Tcl_ListObjReplace(interp, listPtr, LIST_MAX, 0,
			    groupSize, &listv[i]);
		} else {
		    Tcl_BounceRefCount(itemPtr);
		    itemPtr = listv[i];
		    Tcl_ListObjAppendElement(interp, listPtr, itemPtr);
		}
	    } else if (returnSubindices) {
		Tcl_Size j;

		TclNewIndexObj(itemPtr, i+groupOffset);
		for (j=0 ; j<sortInfo.indexc ; j++) {
		    Tcl_Obj *elObj;
		    size_t elValue = TclIndexDecode(sortInfo.indexv[j], listc);
		    TclNewIndexObj(elObj, elValue);
		    Tcl_ListObjAppendElement(interp, itemPtr, elObj);
		}
		Tcl_ListObjAppendElement(interp, listPtr, itemPtr);
	    } else {
		Tcl_ListObjAppendElement(interp, listPtr, Tcl_NewWideIntObj(i));
	    }
	}
    }

    Tcl_BounceRefCount(itemPtr);
    itemPtr = NULL;

    /*
     * Return everything or a single value.
     */

    if (allMatches) {
	Tcl_SetObjResult(interp, listPtr);
    } else if (!inlineReturn) {
	if (returnSubindices) {
	    Tcl_Size j;

	    TclNewIndexObj(itemPtr, index+groupOffset);
	    for (j=0 ; j<sortInfo.indexc ; j++) {
		Tcl_Obj *elObj;
		size_t elValue = TclIndexDecode(sortInfo.indexv[j], listc);
		TclNewIndexObj(elObj, elValue);
		Tcl_ListObjAppendElement(interp, itemPtr, elObj);
	    }
	    Tcl_SetObjResult(interp, itemPtr);
	} else {
		Tcl_Obj *elObj;
		TclNewIndexObj(elObj, index);
	    Tcl_SetObjResult(interp, elObj);
	}
    } else if (index < 0) {
	/*
	 * Is this superfluous? The result should be a blank object by
	 * default...
	 */

	Tcl_SetObjResult(interp, Tcl_NewObj());
    } else {
	if (returnSubindices) {
	    Tcl_SetObjResult(interp, SelectObjFromSublist(listv[i+groupOffset],
		    &sortInfo));
	} else if (groupSize > 1) {
	    Tcl_SetObjResult(interp, Tcl_NewListObj(groupSize, &listv[index]));
	} else {
	    Tcl_SetObjResult(interp, listv[index]);
	}
    }
    result = TCL_OK;

    /*
     * Cleanup the index list array.
     */

  done:
    /* potential lingering abstract list element */
    Tcl_BounceRefCount(itemPtr);

    if (startPtr != NULL) {
	Tcl_DecrRefCount(startPtr);
    }
    if (allocatedIndexVector) {
	TclStackFree(interp, sortInfo.indexv);
    }
    return result;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdIL.c`, function `Tcl_LsortObjCmd: input-order original numeric preparation and list merge producer`, lines 4502–4999. Line unit: physical LF only. Original byte range [118534, 132520). Retained reference `source-window-12`.

```text
Tcl_LsortObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument values. */
{
    int indices, nocase = 0, indexc;
    int sortMode = SORTMODE_ASCII;
    int group, allocatedIndexVector = 0;
    Tcl_Size j, idx, groupOffset, length;
    Tcl_WideInt wide, groupSize;
    Tcl_Obj *resultPtr, *cmdPtr, **listObjPtrs, *listObj, *indexPtr;
    Tcl_Size i, elmArrSize;
    SortElement *elementArray = NULL, *elementPtr;
    SortInfo sortInfo;		/* Information about this sort that needs to
				 * be passed to the comparison function. */
#   define MAXCALLOC 1024000
#   define NUM_LISTS 30
    SortElement *subList[NUM_LISTS+1];
				/* This array holds pointers to temporary
				 * lists built during the merge sort. Element
				 * i of the array holds a list of length
				 * 2**i. */
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

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-option value ...? list");
	return TCL_ERROR;
    }

    /*
     * Parse arguments to set up the mode for the sort.
     */

    sortInfo.isIncreasing = 1;
    sortInfo.sortMode = SORTMODE_ASCII;
    sortInfo.indexv = NULL;
    sortInfo.indexc = 0;
    sortInfo.unique = 0;
    sortInfo.interp = interp;
    sortInfo.resultCode = TCL_OK;
    cmdPtr = NULL;
    indices = 0;
    group = 0;
    groupSize = 1;
    groupOffset = 0;
    indexPtr = NULL;
    for (i = 1; i < objc-1; i++) {
	if (Tcl_GetIndexFromObj(interp, objv[i], switches, "option", 0,
		&index) != TCL_OK) {
	    sortInfo.resultCode = TCL_ERROR;
	    goto done;
	}
	switch (index) {
	case LSORT_ASCII:
	    sortInfo.sortMode = SORTMODE_ASCII;
	    break;
	case LSORT_COMMAND:
	    if (i == objc-2) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"\"-command\" option must be followed "
			"by comparison command", -1));
		Tcl_SetErrorCode(interp, "TCL", "ARGUMENT", "MISSING", (char *)NULL);
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    sortInfo.sortMode = SORTMODE_COMMAND;
	    cmdPtr = objv[i+1];
	    i++;
	    break;
	case LSORT_DECREASING:
	    sortInfo.isIncreasing = 0;
	    break;
	case LSORT_DICTIONARY:
	    sortInfo.sortMode = SORTMODE_DICTIONARY;
	    break;
	case LSORT_INCREASING:
	    sortInfo.isIncreasing = 1;
	    break;
	case LSORT_INDEX: {
	    Tcl_Size sortindex;
	    Tcl_Obj **indexv;

	    if (i == objc-2) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"\"-index\" option must be followed by list index",
			-1));
		Tcl_SetErrorCode(interp, "TCL", "ARGUMENT", "MISSING", (char *)NULL);
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    if (TclListObjGetElements(interp, objv[i+1], &sortindex,
		    &indexv) != TCL_OK) {
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }

	    /*
	     * Check each of the indices for syntactic correctness. Note that
	     * we do not store the converted values here because we do not
	     * know if this is the only -index option yet and so we can't
	     * allocate any space; that happens after the scan through all the
	     * options is done.
	     */

	    for (j=0 ; j<sortindex ; j++) {
		int encoded = 0;
		int result = TclIndexEncode(interp, indexv[j],
			TCL_INDEX_NONE, TCL_INDEX_NONE, &encoded);

		if ((result == TCL_OK) && (encoded == (int)TCL_INDEX_NONE)) {
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "index \"%s\" out of range",
			    TclGetString(indexv[j])));
		    Tcl_SetErrorCode(interp, "TCL", "VALUE", "INDEX", "OUTOFRANGE", (char *)NULL);
		    result = TCL_ERROR;
		}
		if (result == TCL_ERROR) {
		    Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
			    "\n    (-index option item number %" TCL_Z_MODIFIER "u)", j));
		    sortInfo.resultCode = TCL_ERROR;
		    goto done;
		}
	    }
	    indexPtr = objv[i+1];
	    i++;
	    break;
	}
	case LSORT_INTEGER:
	    sortInfo.sortMode = SORTMODE_INTEGER;
	    break;
	case LSORT_NOCASE:
	    nocase = 1;
	    break;
	case LSORT_REAL:
	    sortInfo.sortMode = SORTMODE_REAL;
	    break;
	case LSORT_UNIQUE:
	    sortInfo.unique = 1;
	    break;
	case LSORT_INDICES:
	    indices = 1;
	    break;
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
	}
    }
    if (nocase && (sortInfo.sortMode == SORTMODE_ASCII)) {
	sortInfo.sortMode = SORTMODE_ASCII_NC;
    }

    /*
     * Now extract the -index list for real, if present. No failures are
     * expected here; the values are all of the right type or convertible to
     * it.
     */

    if (indexPtr) {
	Tcl_Obj **indexv;

	TclListObjGetElements(interp, indexPtr, &sortInfo.indexc, &indexv);
	switch (sortInfo.indexc) {
	case 0:
	    sortInfo.indexv = NULL;
	    break;
	case 1:
	    sortInfo.indexv = &sortInfo.singleIndex;
	    break;
	default:
	    sortInfo.indexv = (int *)
		    TclStackAlloc(interp, sizeof(int) * sortInfo.indexc);
	    allocatedIndexVector = 1;	/* Cannot use indexc field, as it
					 * might be decreased by 1 later. */
	}
	for (j=0 ; j<sortInfo.indexc ; j++) {
	    /* Prescreened values, no errors or out of range possible */
	    TclIndexEncode(NULL, indexv[j], TCL_INDEX_NONE,
		    TCL_INDEX_NONE, &sortInfo.indexv[j]);
	}
    }

    listObj = objv[objc-1];

    if (sortInfo.sortMode == SORTMODE_COMMAND) {
	Tcl_Obj *newCommandPtr, *newObjPtr;

	/*
	 * When sorting using a command, we are reentrant and therefore might
	 * have the representation of the list being sorted shimmered out from
	 * underneath our feet. Take a copy (cheap) to prevent this. [Bug
	 * 1675116]
	 */

	listObj = TclListObjCopy(interp, listObj);
	if (listObj == NULL) {
	    sortInfo.resultCode = TCL_ERROR;
	    goto done;
	}

	/*
	 * The existing command is a list. We want to flatten it, append two
	 * dummy arguments on the end, and replace these arguments later.
	 */

	newCommandPtr = Tcl_DuplicateObj(cmdPtr);
	TclNewObj(newObjPtr);
	Tcl_IncrRefCount(newCommandPtr);
	if (Tcl_ListObjAppendElement(interp, newCommandPtr, newObjPtr)
		!= TCL_OK) {
	    TclDecrRefCount(newCommandPtr);
	    TclDecrRefCount(newObjPtr);
	    sortInfo.resultCode = TCL_ERROR;
	    goto done;
	}
	Tcl_ListObjAppendElement(interp, newCommandPtr, Tcl_NewObj());
	sortInfo.compareCmdPtr = newCommandPtr;
    }

    if (TclObjTypeHasProc(objv[1], getElementsProc)) {
	sortInfo.resultCode =
	    TclObjTypeGetElements(interp, listObj, &length, &listObjPtrs);
    } else {
	sortInfo.resultCode = TclListObjGetElements(interp, listObj,
	    &length, &listObjPtrs);
    }
    if (sortInfo.resultCode != TCL_OK || length <= 0) {
	goto done;
    }

    /*
     * Check for sanity when grouping elements of the overall list together
     * because of the -stride option. [TIP #326]
     */

    if (group) {
	if (length % groupSize) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "list size must be a multiple of the stride length",
		    -1));
	    Tcl_SetErrorCode(interp, "TCL", "OPERATION", "LSORT", "BADSTRIDE",
		    (char *)NULL);
	    sortInfo.resultCode = TCL_ERROR;
	    goto done;
	}
	length = length / groupSize;
	if (sortInfo.indexc > 0) {
	    /*
	     * Use the first value in the list supplied to -index as the
	     * offset of the element within each group by which to sort.
	     */

	    groupOffset = TclIndexDecode(sortInfo.indexv[0], groupSize - 1);
	    if (groupOffset < 0 || groupOffset >= groupSize) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"when used with \"-stride\", the leading \"-index\""
			" value must be within the group", -1));
		Tcl_SetErrorCode(interp, "TCL", "OPERATION", "LSORT",
			"BADINDEX", (char *)NULL);
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    if (sortInfo.indexc == 1) {
		sortInfo.indexc = 0;
		sortInfo.indexv = NULL;
	    } else {
		sortInfo.indexc--;

		/*
		 * Do not shrink the actual memory block used; that doesn't
		 * work with TclStackAlloc-allocated memory. [Bug 2918962]
		 *
		 * TODO: Consider a pointer increment to replace this
		 * array shift.
		 */

		for (i = 0; i < sortInfo.indexc; i++) {
		    sortInfo.indexv[i] = sortInfo.indexv[i+1];
		}
	    }
	}
    }

    sortInfo.numElements = length;

    indexc = sortInfo.indexc;
    sortMode = sortInfo.sortMode;
    if ((sortMode == SORTMODE_ASCII_NC)
	    || (sortMode == SORTMODE_DICTIONARY)) {
	/*
	 * For this function's purpose all string-based modes are equivalent
	 */

	sortMode = SORTMODE_ASCII;
    }

    /*
     * Initialize the sublists. After the following loop, subList[i] will
     * contain a sorted sublist of length 2**i. Use one extra subList at the
     * end, always at NULL, to indicate the end of the lists.
     */

    for (j=0 ; j<=NUM_LISTS ; j++) {
	subList[j] = NULL;
    }

    /*
     * The following loop creates a SortElement for each list element and
     * begins sorting it into the sublists as it appears.
     */

    elmArrSize = length * sizeof(SortElement);
    if (elmArrSize <= MAXCALLOC) {
	elementArray = (SortElement *)Tcl_Alloc(elmArrSize);
    } else {
	elementArray = (SortElement *)malloc(elmArrSize);
    }
    if (!elementArray) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"no enough memory to proccess sort of %" TCL_Z_MODIFIER "u items", length));
	Tcl_SetErrorCode(interp, "TCL", "MEMORY", (char *)NULL);
	sortInfo.resultCode = TCL_ERROR;
	goto done;
    }

    for (i=0; i < length; i++) {
	idx = groupSize * i + groupOffset;
	if (indexc) {
	    /*
	     * If this is an indexed sort, retrieve the corresponding element
	     */
	    indexPtr = SelectObjFromSublist(listObjPtrs[idx], &sortInfo);
	    if (sortInfo.resultCode != TCL_OK) {
		goto done;
	    }
	} else {
	    indexPtr = listObjPtrs[idx];
	}

	/*
	 * Determine the "value" of this object for sorting purposes
	 */

	if (sortMode == SORTMODE_ASCII) {
	    elementArray[i].collationKey.strValuePtr = TclGetString(indexPtr);
	} else if (sortMode == SORTMODE_INTEGER) {
	    Tcl_WideInt a;

	    if (TclGetWideIntFromObj(sortInfo.interp, indexPtr, &a) != TCL_OK) {
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    elementArray[i].collationKey.wideValue = a;
	} else if (sortMode == SORTMODE_REAL) {
	    double a;

	    if (Tcl_GetDoubleFromObj(sortInfo.interp, indexPtr, &a) != TCL_OK) {
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    elementArray[i].collationKey.doubleValue = a;
	} else {
	    elementArray[i].collationKey.objValuePtr = indexPtr;
	}

	/*
	 * Determine the representation of this element in the result: either
	 * the objPtr itself, or its index in the original list.
	 */

	if (indices || group) {
	    elementArray[i].payload.index = idx;
	} else {
	    elementArray[i].payload.objPtr = listObjPtrs[idx];
	}

	/*
	 * Merge this element in the preexisting sublists (and merge together
	 * sublists when we have two of the same size).
	 */

	elementArray[i].nextPtr = NULL;
	elementPtr = &elementArray[i];
	for (j=0 ; subList[j] ; j++) {
	    elementPtr = MergeLists(subList[j], elementPtr, &sortInfo);
	    subList[j] = NULL;
	}
	if (j >= NUM_LISTS) {
	    j = NUM_LISTS-1;
	}
	subList[j] = elementPtr;
    }

    /*
     * Merge all sublists
     */

    elementPtr = subList[0];
    for (j=1 ; j<NUM_LISTS ; j++) {
	elementPtr = MergeLists(subList[j], elementPtr, &sortInfo);
    }

    /*
     * Now store the sorted elements in the result list.
     */

    if (sortInfo.resultCode == TCL_OK) {
	ListRep listRep;
	Tcl_Obj **newArray, *objPtr;

	resultPtr = Tcl_NewListObj(sortInfo.numElements * groupSize, NULL);
	ListObjGetRep(resultPtr, &listRep);
	newArray = ListRepElementsBase(&listRep);
	if (group) {
	    for (i=0; elementPtr!=NULL ; elementPtr=elementPtr->nextPtr) {
		idx = elementPtr->payload.index;
		for (j = 0; j < groupSize; j++) {
		    if (indices) {
			TclNewIndexObj(objPtr, idx + j - groupOffset);
			newArray[i++] = objPtr;
			Tcl_IncrRefCount(objPtr);
		    } else {
			objPtr = listObjPtrs[idx + j - groupOffset];
			newArray[i++] = objPtr;
			Tcl_IncrRefCount(objPtr);
		    }
		}
	    }
	} else if (indices) {
	    for (i=0; elementPtr != NULL ; elementPtr = elementPtr->nextPtr) {
		TclNewIndexObj(objPtr, elementPtr->payload.index);
		newArray[i++] = objPtr;
		Tcl_IncrRefCount(objPtr);
	    }
	} else {
	    for (i=0; elementPtr != NULL ; elementPtr = elementPtr->nextPtr) {
		objPtr = elementPtr->payload.objPtr;
		newArray[i++] = objPtr;
		Tcl_IncrRefCount(objPtr);
	    }
	}
	listRep.storePtr->numUsed = i;
	if (listRep.spanPtr) {
	    listRep.spanPtr->spanStart = listRep.storePtr->firstUsed;
	    listRep.spanPtr->spanLength = listRep.storePtr->numUsed;
	}
	Tcl_SetObjResult(interp, resultPtr);
    }

  done:
    if (sortMode == SORTMODE_COMMAND) {
	TclDecrRefCount(sortInfo.compareCmdPtr);
	TclDecrRefCount(listObj);
	sortInfo.compareCmdPtr = NULL;
    }
    if (allocatedIndexVector) {
	TclStackFree(interp, sortInfo.indexv);
    }
    if (elementArray) {
	if (elmArrSize <= MAXCALLOC) {
	    Tcl_Free(elementArray);
	} else {
	    free((char *)elementArray);
	}
    }
    return sortInfo.resultCode;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdIL.c`, function `SortCompare: comparison of prepared numeric values and command result primitive`, lines 5231–5316. Line unit: physical LF only. Original byte range [138516, 141059). Retained reference `source-window-13`.

```text
SortCompare(
    SortElement *elemPtr1, SortElement *elemPtr2,
				/* Values to be compared. */
    SortInfo *infoPtr)		/* Information passed from the top-level
				 * "lsort" command. */
{
    int order = 0;

    if (infoPtr->sortMode == SORTMODE_ASCII) {
	order = TclUtfCmp(elemPtr1->collationKey.strValuePtr,
		elemPtr2->collationKey.strValuePtr);
    } else if (infoPtr->sortMode == SORTMODE_ASCII_NC) {
	order = TclUtfCasecmp(elemPtr1->collationKey.strValuePtr,
		elemPtr2->collationKey.strValuePtr);
    } else if (infoPtr->sortMode == SORTMODE_DICTIONARY) {
	order = DictionaryCompare(elemPtr1->collationKey.strValuePtr,
		elemPtr2->collationKey.strValuePtr);
    } else if (infoPtr->sortMode == SORTMODE_INTEGER) {
	Tcl_WideInt a, b;

	a = elemPtr1->collationKey.wideValue;
	b = elemPtr2->collationKey.wideValue;
	order = ((a >= b) - (a <= b));
    } else if (infoPtr->sortMode == SORTMODE_REAL) {
	double a, b;

	a = elemPtr1->collationKey.doubleValue;
	b = elemPtr2->collationKey.doubleValue;
	order = ((a >= b) - (a <= b));
    } else {
	Tcl_Obj **objv, *paramObjv[2];
	Tcl_Size objc;
	Tcl_Obj *objPtr1, *objPtr2;

	if (infoPtr->resultCode != TCL_OK) {
	    /*
	     * Once an error has occurred, skip any future comparisons so as
	     * to preserve the error message in sortInterp->result.
	     */

	    return 0;
	}

	objPtr1 = elemPtr1->collationKey.objValuePtr;
	objPtr2 = elemPtr2->collationKey.objValuePtr;

	paramObjv[0] = objPtr1;
	paramObjv[1] = objPtr2;

	/*
	 * We made space in the command list for the two things to compare.
	 * Replace them and evaluate the result.
	 */

	TclListObjLength(infoPtr->interp, infoPtr->compareCmdPtr, &objc);
	Tcl_ListObjReplace(infoPtr->interp, infoPtr->compareCmdPtr, objc - 2,
		2, 2, paramObjv);
	TclListObjGetElements(infoPtr->interp, infoPtr->compareCmdPtr,
		&objc, &objv);

	infoPtr->resultCode = Tcl_EvalObjv(infoPtr->interp, objc, objv, 0);

	if (infoPtr->resultCode != TCL_OK) {
	    Tcl_AddErrorInfo(infoPtr->interp, "\n    (-compare command)");
	    return 0;
	}

	/*
	 * Parse the result of the command.
	 */

	if (TclGetIntFromObj(infoPtr->interp,
		Tcl_GetObjResult(infoPtr->interp), &order) != TCL_OK) {
	    Tcl_SetObjResult(infoPtr->interp, Tcl_NewStringObj(
		    "-compare command returned non-integer result", -1));
	    Tcl_SetErrorCode(infoPtr->interp, "TCL", "OPERATION", "LSORT",
		    "COMPARISONFAILED", (char *)NULL);
	    infoPtr->resultCode = TCL_ERROR;
	    return 0;
	}
    }
    if (!infoPtr->isIncreasing) {
	order = -order;
    }
    return order;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdIL.c`, function `Tcl_LsearchObjCmd: default table, selected original pattern and reached numeric keys`, lines 3105–3868. Line unit: physical LF only. Original byte range [83869, 103763). Retained reference `source-window-14`.

```text
Tcl_LsearchObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument values. */
{
    const char *bytes, *patternBytes;
    int match, result=TCL_OK, bisect;
    Tcl_Size i, length = 0, listc, elemLen, start, index;
    Tcl_Size groupOffset, lower, upper;
    int allocatedIndexVector = 0;
    int isIncreasing;
    Tcl_WideInt patWide, objWide, wide, groupSize;
    int allMatches, inlineReturn, negatedMatch, returnSubindices, noCase;
    double patDouble, objDouble;
    SortInfo sortInfo;
    Tcl_Obj *patObj, **listv, *listPtr, *startPtr, *itemPtr = NULL;
    SortStrCmpFn_t strCmpFn = TclUtfCmp;
    Tcl_RegExp regexp = NULL;
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
    enum datatypes {
	ASCII, DICTIONARY, INTEGER, REAL
    } dataType;
    enum modes {
	EXACT, GLOB, REGEXP, SORTED
    };
    enum modes mode;

    mode = GLOB;
    dataType = ASCII;
    isIncreasing = 1;
    allMatches = 0;
    inlineReturn = 0;
    returnSubindices = 0;
    negatedMatch = 0;
    bisect = 0;
    listPtr = NULL;
    startPtr = NULL;
    groupSize = 1;
    groupOffset = 0;
    start = 0;
    noCase = 0;
    sortInfo.compareCmdPtr = NULL;
    sortInfo.isIncreasing = 1;
    sortInfo.sortMode = 0;
    sortInfo.interp = interp;
    sortInfo.resultCode = TCL_OK;
    sortInfo.indexv = NULL;
    sortInfo.indexc = 0;

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-option value ...? list pattern");
	return TCL_ERROR;
    }

    for (i = 1; i < objc-2; i++) {
	enum lsearchoptions idx;
	if (Tcl_GetIndexFromObj(interp, objv[i], options, "option", 0, &idx)
		!= TCL_OK) {
	    result = TCL_ERROR;
	    goto done;
	}
	switch (idx) {
	case LSEARCH_ALL:		/* -all */
	    allMatches = 1;
	    break;
	case LSEARCH_ASCII:		/* -ascii */
	    dataType = ASCII;
	    break;
	case LSEARCH_BISECT:		/* -bisect */
	    mode = SORTED;
	    bisect = 1;
	    break;
	case LSEARCH_DECREASING:	/* -decreasing */
	    isIncreasing = 0;
	    sortInfo.isIncreasing = 0;
	    break;
	case LSEARCH_DICTIONARY:	/* -dictionary */
	    dataType = DICTIONARY;
	    break;
	case LSEARCH_EXACT:		/* -increasing */
	    mode = EXACT;
	    break;
	case LSEARCH_GLOB:		/* -glob */
	    mode = GLOB;
	    break;
	case LSEARCH_INCREASING:	/* -increasing */
	    isIncreasing = 1;
	    sortInfo.isIncreasing = 1;
	    break;
	case LSEARCH_INLINE:		/* -inline */
	    inlineReturn = 1;
	    break;
	case LSEARCH_INTEGER:		/* -integer */
	    dataType = INTEGER;
	    break;
	case LSEARCH_NOCASE:		/* -nocase */
	    strCmpFn = TclUtfCasecmp;
	    noCase = 1;
	    break;
	case LSEARCH_NOT:		/* -not */
	    negatedMatch = 1;
	    break;
	case LSEARCH_REAL:		/* -real */
	    dataType = REAL;
	    break;
	case LSEARCH_REGEXP:		/* -regexp */
	    mode = REGEXP;
	    break;
	case LSEARCH_SORTED:		/* -sorted */
	    mode = SORTED;
	    break;
	case LSEARCH_SUBINDICES:	/* -subindices */
	    returnSubindices = 1;
	    break;
	case LSEARCH_START:		/* -start */
	    /*
	     * If there was a previous -start option, release its saved index
	     * because it will either be replaced or there will be an error.
	     */

	    if (startPtr != NULL) {
		Tcl_DecrRefCount(startPtr);
		startPtr = NULL;
	    }
	    if (i > objc-4) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"missing starting index", -1));
		Tcl_SetErrorCode(interp, "TCL", "ARGUMENT", "MISSING", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    i++;
	    if (objv[i] == objv[objc - 2]) {
		/*
		 * Take copy to prevent shimmering problems. Note that it does
		 * not matter if the index obj is also a component of the list
		 * being searched. We only need to copy where the list and the
		 * index are one-and-the-same.
		 */

		startPtr = Tcl_DuplicateObj(objv[i]);
	    } else {
		startPtr = objv[i];
	    }
	    Tcl_IncrRefCount(startPtr);
	    break;
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
	case LSEARCH_INDEX: {		/* -index */
	    Tcl_Obj **indices;
	    Tcl_Size j;

	    if (allocatedIndexVector) {
		TclStackFree(interp, sortInfo.indexv);
		allocatedIndexVector = 0;
	    }
	    if (i > objc-4) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"\"-index\" option must be followed by list index",
			-1));
		Tcl_SetErrorCode(interp, "TCL", "ARGUMENT", "MISSING", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }

	    /*
	     * Store the extracted indices for processing by sublist
	     * extraction. Note that we don't do this using objects because
	     * that has shimmering problems.
	     */

	    i++;
	    if (TclListObjGetElements(interp, objv[i],
		    &sortInfo.indexc, &indices) != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    switch (sortInfo.indexc) {
	    case 0:
		sortInfo.indexv = NULL;
		break;
	    case 1:
		sortInfo.indexv = &sortInfo.singleIndex;
		break;
	    default:
		sortInfo.indexv = (int *)
			TclStackAlloc(interp, sizeof(int) * sortInfo.indexc);
		allocatedIndexVector = 1; /* Cannot use indexc field, as it
					   * might be decreased by 1 later. */
	    }

	    /*
	     * Fill the array by parsing each index. We don't know whether
	     * their scale is sensible yet, but we at least perform the
	     * syntactic check here.
	     */

	    for (j=0 ; j<sortInfo.indexc ; j++) {
		int encoded = 0;
		if (TclIndexEncode(interp, indices[j], TCL_INDEX_NONE,
			TCL_INDEX_NONE, &encoded) != TCL_OK) {
		    result = TCL_ERROR;
		}
		if (encoded == (int)TCL_INDEX_NONE) {
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "index \"%s\" out of range",
			    TclGetString(indices[j])));
		    Tcl_SetErrorCode(interp, "TCL", "VALUE", "INDEX", "OUTOFRANGE", (char *)NULL);
		    result = TCL_ERROR;
		}
		if (result == TCL_ERROR) {
		    Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
			    "\n    (-index option item number %" TCL_Z_MODIFIER "u)", j));
		    goto done;
		}
		sortInfo.indexv[j] = encoded;
	    }
	    break;
	}
	default:
	    TCL_UNREACHABLE();
	}
    }

    /*
     * Subindices only make sense if asked for with -index option set.
     */

    if (returnSubindices && sortInfo.indexc==0) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"-subindices cannot be used without -index option", -1));
	Tcl_SetErrorCode(interp, "TCL", "OPERATION", "LSEARCH",
		"BAD_OPTION_MIX", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    if (bisect && (allMatches || negatedMatch)) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"-bisect is not compatible with -all or -not", -1));
	Tcl_SetErrorCode(interp, "TCL", "OPERATION", "LSEARCH",
		"BAD_OPTION_MIX", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    if (mode == REGEXP) {
	/*
	 * We can shimmer regexp/list if listv[i] == pattern, so get the
	 * regexp rep before the list rep. First time round, omit the interp
	 * and hope that the compilation will succeed. If it fails, we'll
	 * recompile in "expensive" mode with a place to put error messages.
	 */

	regexp = Tcl_GetRegExpFromObj(NULL, objv[objc - 1],
		TCL_REG_ADVANCED | TCL_REG_NOSUB |
		(noCase ? TCL_REG_NOCASE : 0));
	if (regexp == NULL) {
	    /*
	     * Failed to compile the RE. Try again without the TCL_REG_NOSUB
	     * flag in case the RE had sub-expressions in it [Bug 1366683]. If
	     * this fails, an error message will be left in the interpreter.
	     */

	    regexp = Tcl_GetRegExpFromObj(interp, objv[objc - 1],
		    TCL_REG_ADVANCED | (noCase ? TCL_REG_NOCASE : 0));
	}

	if (regexp == NULL) {
	    result = TCL_ERROR;
	    goto done;
	}
    }

    /*
     * Make sure the list argument is a list object and get its length and a
     * pointer to its array of element pointers.
     */

    result = TclListObjGetElements(interp, objv[objc - 2], &listc, &listv);
    if (result != TCL_OK) {
	goto done;
    }

    /*
     * Check for sanity when grouping elements of the overall list together
     * because of the -stride option. [TIP #351]
     */

    if (groupSize > 1) {
	if (listc % groupSize) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "list size must be a multiple of the stride length",
		    -1));
	    Tcl_SetErrorCode(interp, "TCL", "OPERATION", "LSEARCH", "BADSTRIDE",
		    (char *)NULL);
	    result = TCL_ERROR;
	    goto done;
	}
	if (sortInfo.indexc > 0) {
	    /*
	     * Use the first value in the list supplied to -index as the
	     * offset of the element within each group by which to sort.
	     */

	    groupOffset = TclIndexDecode(sortInfo.indexv[0], groupSize - 1);
	    if (groupOffset < 0 || groupOffset >= groupSize) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"when used with \"-stride\", the leading \"-index\""
			" value must be within the group", -1));
		Tcl_SetErrorCode(interp, "TCL", "OPERATION", "LSEARCH",
			"BADINDEX", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    if (sortInfo.indexc == 1) {
		sortInfo.indexc = 0;
		sortInfo.indexv = NULL;
	    } else {
		sortInfo.indexc--;

		for (i = 0; i < sortInfo.indexc; i++) {
		    sortInfo.indexv[i] = sortInfo.indexv[i+1];
		}
	    }
	}
    }

    /*
     * Get the user-specified start offset.
     */

    if (startPtr) {
	result = TclGetIntForIndexM(interp, startPtr, listc-1, &start);
	if (result != TCL_OK) {
	    goto done;
	}
	if (start == TCL_INDEX_NONE) {
	    start = TCL_INDEX_START;
	}

	/*
	 * If the search started past the end of the list, we just return a
	 * "did not match anything at all" result straight away. [Bug 1374778]
	 */

	if (start >= listc) {
	    if (allMatches || inlineReturn) {
		Tcl_ResetResult(interp);
	    } else {
		TclNewIntObj(itemPtr, -1);
		Tcl_SetObjResult(interp, itemPtr);
	    }
	    goto done;
	}

	/*
	 * If start points within a group, it points to the start of the group.
	 */

	if (groupSize > 1) {
	    start -= (start % groupSize);
	}
    }

    patObj = objv[objc - 1];
    patternBytes = NULL;
    if (mode == EXACT || mode == SORTED) {
	switch (dataType) {
	case ASCII:
	case DICTIONARY:
	    patternBytes = TclGetStringFromObj(patObj, &length);
	    break;
	case INTEGER:
	    result = TclGetWideIntFromObj(interp, patObj, &patWide);
	    if (result != TCL_OK) {
		goto done;
	    }

	    /*
	     * List representation might have been shimmered; restore it. [Bug
	     * 1844789]
	     */

	    TclListObjGetElements(NULL, objv[objc - 2], &listc, &listv);
	    break;
	case REAL:
	    result = Tcl_GetDoubleFromObj(interp, patObj, &patDouble);
	    if (result != TCL_OK) {
		goto done;
	    }

	    /*
	     * List representation might have been shimmered; restore it. [Bug
	     * 1844789]
	     */

	    TclListObjGetElements(NULL, objv[objc - 2], &listc, &listv);
	    break;
	}
    } else {
	patternBytes = TclGetStringFromObj(patObj, &length);
    }

    /*
     * Set default index value to -1, indicating failure; if we find the item
     * in the course of our search, index will be set to the correct value.
     */

    index = -1;
    match = 0;

    if (mode == SORTED && !allMatches && !negatedMatch) {
	/*
	 * If the data is sorted, we can do a more intelligent search. Note
	 * that there is no point in being smart when -all was specified; in
	 * that case, we have to look at all items anyway, and there is no
	 * sense in doing this when the match sense is inverted.
	 */

	/*
	 * With -stride, lower, upper and i are kept as multiples of groupSize.
	 */

	lower = start - groupSize;
	upper = listc;
	itemPtr = NULL;
	while (lower + groupSize != upper && sortInfo.resultCode == TCL_OK) {
	    i = (lower + upper)/2;
	    i -= i % groupSize;

	    Tcl_BounceRefCount(itemPtr);
	    itemPtr = NULL;

	    if (sortInfo.indexc != 0) {
		itemPtr = SelectObjFromSublist(listv[i+groupOffset], &sortInfo);
		if (sortInfo.resultCode != TCL_OK) {
		    result = sortInfo.resultCode;
		    goto done;
		}
	    } else {
		itemPtr = listv[i+groupOffset];
	    }
	    switch (dataType) {
	    case ASCII:
		bytes = TclGetString(itemPtr);
		match = strCmpFn(patternBytes, bytes);
		break;
	    case DICTIONARY:
		bytes = TclGetString(itemPtr);
		match = DictionaryCompare(patternBytes, bytes);
		break;
	    case INTEGER:
		result = TclGetWideIntFromObj(interp, itemPtr, &objWide);
		if (result != TCL_OK) {
		    goto done;
		}
		if (patWide == objWide) {
		    match = 0;
		} else if (patWide < objWide) {
		    match = -1;
		} else {
		    match = 1;
		}
		break;
	    case REAL:
		result = Tcl_GetDoubleFromObj(interp, itemPtr, &objDouble);
		if (result != TCL_OK) {
		    goto done;
		}
		if (patDouble == objDouble) {
		    match = 0;
		} else if (patDouble < objDouble) {
		    match = -1;
		} else {
		    match = 1;
		}
		break;
	    }
	    if (match == 0) {
		/*
		 * Normally, binary search is written to stop when it finds a
		 * match. If there are duplicates of an element in the list,
		 * our first match might not be the first occurrence.
		 * Consider: 0 0 0 1 1 1 2 2 2
		 *
		 * To maintain consistency with standard lsearch semantics, we
		 * must find the leftmost occurrence of the pattern in the
		 * list. Thus we don't just stop searching here. This
		 * variation means that a search always makes log n
		 * comparisons (normal binary search might "get lucky" with an
		 * early comparison).
		 *
		 * In bisect mode though, we want the last of equals.
		 */

		index = i;
		if (bisect) {
		    lower = i;
		} else {
		    upper = i;
		}
	    } else if (match > 0) {
		if (isIncreasing) {
		    lower = i;
		} else {
		    upper = i;
		}
	    } else {
		if (isIncreasing) {
		    upper = i;
		} else {
		    lower = i;
		}
	    }
	}
	if (bisect && index < 0) {
	    index = lower;
	}
    } else {
	/*
	 * We need to do a linear search, because (at least one) of:
	 *   - our matcher can only tell equal vs. not equal
	 *   - our matching sense is negated
	 *   - we're building a list of all matched items
	 */

	if (allMatches) {
	    listPtr = Tcl_NewListObj(0, NULL);
	}
	for (i = start; i < listc; i += groupSize) {
	    match = 0;
	    Tcl_BounceRefCount(itemPtr);
	    itemPtr = NULL;

	    if (sortInfo.indexc != 0) {
		itemPtr = SelectObjFromSublist(listv[i+groupOffset], &sortInfo);
		if (sortInfo.resultCode != TCL_OK) {
		    if (listPtr != NULL) {
			Tcl_DecrRefCount(listPtr);
		    }
		    result = sortInfo.resultCode;
		    goto done;
		}
	    } else {
		itemPtr = listv[i+groupOffset];
	    }

	    switch (mode) {
	    case SORTED:
	    case EXACT:
		switch (dataType) {
		case ASCII:
		    bytes = TclGetStringFromObj(itemPtr, &elemLen);
		    if (length == elemLen) {
			/*
			 * This split allows for more optimal compilation of
			 * memcmp/strcasecmp.
			 */

			if (noCase) {
			    match = (TclUtfCasecmp(bytes, patternBytes) == 0);
			} else {
			    match = (memcmp(bytes, patternBytes, length) == 0);
			}
		    }
		    break;

		case DICTIONARY:
		    bytes = TclGetString(itemPtr);
		    match = (DictionaryCompare(bytes, patternBytes) == 0);
		    break;

		case INTEGER:
		    result = TclGetWideIntFromObj(interp, itemPtr, &objWide);
		    if (result != TCL_OK) {
			if (listPtr != NULL) {
			    Tcl_DecrRefCount(listPtr);
			}
			goto done;
		    }
		    match = (objWide == patWide);
		    break;

		case REAL:
		    result = Tcl_GetDoubleFromObj(interp,itemPtr, &objDouble);
		    if (result != TCL_OK) {
			if (listPtr) {
			    Tcl_DecrRefCount(listPtr);
			}
			goto done;
		    }
		    match = (objDouble == patDouble);
		    break;
		}
		break;

	    case GLOB:
		match = Tcl_StringCaseMatch(TclGetString(itemPtr),
			patternBytes, noCase);
		break;

	    case REGEXP:
		match = Tcl_RegExpExecObj(interp, regexp, itemPtr, 0, 0, 0);
		if (match < 0) {
		    Tcl_DecrRefCount(patObj);
		    if (listPtr != NULL) {
			Tcl_DecrRefCount(listPtr);
		    }
		    result = TCL_ERROR;
		    goto done;
		}
		break;
	    }

	    /*
	     * Invert match condition for -not.
	     */

	    if (negatedMatch) {
		match = !match;
	    }
	    if (!match) {
		continue;
	    }
	    if (!allMatches) {
		index = i;
		break;
	    } else if (inlineReturn) {
		/*
		 * Note that these appends are not expected to fail.
		 */

		if (returnSubindices && (sortInfo.indexc != 0)) {
		    Tcl_BounceRefCount(itemPtr);
		    itemPtr = SelectObjFromSublist(listv[i+groupOffset],
			    &sortInfo);
		    Tcl_ListObjAppendElement(interp, listPtr, itemPtr);
		} else if (returnSubindices && (sortInfo.indexc == 0) && (groupSize > 1)) {
		    Tcl_BounceRefCount(itemPtr);
		    itemPtr = listv[i + groupOffset];
		    Tcl_ListObjAppendElement(interp, listPtr, itemPtr);
		} else if (groupSize > 1) {
		    Tcl_ListObjReplace(interp, listPtr, LIST_MAX, 0,
			    groupSize, &listv[i]);
		} else {
		    Tcl_BounceRefCount(itemPtr);
		    itemPtr = listv[i];
		    Tcl_ListObjAppendElement(interp, listPtr, itemPtr);
		}
	    } else if (returnSubindices) {
		Tcl_Size j;

		TclNewIndexObj(itemPtr, i+groupOffset);
		for (j=0 ; j<sortInfo.indexc ; j++) {
		    Tcl_Obj *elObj;
		    size_t elValue = TclIndexDecode(sortInfo.indexv[j], listc);
		    TclNewIndexObj(elObj, elValue);
		    Tcl_ListObjAppendElement(interp, itemPtr, elObj);
		}
		Tcl_ListObjAppendElement(interp, listPtr, itemPtr);
	    } else {
		Tcl_ListObjAppendElement(interp, listPtr, Tcl_NewWideIntObj(i));
	    }
	}
    }

    Tcl_BounceRefCount(itemPtr);
    itemPtr = NULL;

    /*
     * Return everything or a single value.
     */

    if (allMatches) {
	Tcl_SetObjResult(interp, listPtr);
    } else if (!inlineReturn) {
	if (returnSubindices) {
	    Tcl_Size j;

	    TclNewIndexObj(itemPtr, index+groupOffset);
	    for (j=0 ; j<sortInfo.indexc ; j++) {
		Tcl_Obj *elObj;
		size_t elValue = TclIndexDecode(sortInfo.indexv[j], listc);
		TclNewIndexObj(elObj, elValue);
		Tcl_ListObjAppendElement(interp, itemPtr, elObj);
	    }
	    Tcl_SetObjResult(interp, itemPtr);
	} else {
	    Tcl_Obj *elObj;
	    TclNewIndexObj(elObj, index);
	    Tcl_SetObjResult(interp, elObj);
	}
    } else if (index < 0) {
	/*
	 * Is this superfluous? The result should be a blank object by
	 * default...
	 */

	Tcl_SetObjResult(interp, Tcl_NewObj());
    } else {
	if (returnSubindices) {
	    Tcl_SetObjResult(interp, SelectObjFromSublist(listv[i+groupOffset],
		    &sortInfo));
	} else if (groupSize > 1) {
	    Tcl_SetObjResult(interp, Tcl_NewListObj(groupSize, &listv[index]));
	} else {
	    Tcl_SetObjResult(interp, listv[index]);
	}
    }
    result = TCL_OK;

    /*
     * Cleanup the index list array.
     */

  done:
    /* potential lingering abstract list element */
    Tcl_BounceRefCount(itemPtr);

    if (startPtr != NULL) {
	Tcl_DecrRefCount(startPtr);
    }
    if (allocatedIndexVector) {
	TclStackFree(interp, sortInfo.indexv);
    }
    return result;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdIL.c`, function `Tcl_LsortObjCmd: input-order original numeric preparation and list merge producer`, lines 4349–4848. Line unit: physical LF only. Original byte range [115160, 129177). Retained reference `source-window-15`.

```text
Tcl_LsortObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument values. */
{
    int indices, nocase = 0;
    int sortMode = SORTMODE_ASCII;
    int group, allocatedIndexVector = 0;
    Tcl_Size j, idx, groupOffset, length, indexc;
    Tcl_WideInt wide, groupSize;
    Tcl_Obj *resultPtr, *cmdPtr, **listObjPtrs, *listObj, *indexPtr;
    Tcl_Size i, elmArrSize;
    SortElement *elementArray = NULL, *elementPtr;
    SortInfo sortInfo;		/* Information about this sort that needs to
				 * be passed to the comparison function. */
#   define MAXCALLOC 1024000
#   define NUM_LISTS 30
    SortElement *subList[NUM_LISTS+1];
				/* This array holds pointers to temporary
				 * lists built during the merge sort. Element
				 * i of the array holds a list of length
				 * 2**i. */
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

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-option value ...? list");
	return TCL_ERROR;
    }

    /*
     * Parse arguments to set up the mode for the sort.
     */

    sortInfo.isIncreasing = 1;
    sortInfo.sortMode = SORTMODE_ASCII;
    sortInfo.indexv = NULL;
    sortInfo.indexc = 0;
    sortInfo.unique = 0;
    sortInfo.interp = interp;
    sortInfo.resultCode = TCL_OK;
    cmdPtr = NULL;
    indices = 0;
    group = 0;
    groupSize = 1;
    groupOffset = 0;
    indexPtr = NULL;
    for (i = 1; i < objc-1; i++) {
	if (Tcl_GetIndexFromObj(interp, objv[i], switches, "option", 0,
		&index) != TCL_OK) {
	    sortInfo.resultCode = TCL_ERROR;
	    goto done;
	}
	switch (index) {
	case LSORT_ASCII:
	    sortInfo.sortMode = SORTMODE_ASCII;
	    break;
	case LSORT_COMMAND:
	    if (i == objc-2) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"\"-command\" option must be followed "
			"by comparison command", -1));
		Tcl_SetErrorCode(interp, "TCL", "ARGUMENT", "MISSING", (char *)NULL);
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    sortInfo.sortMode = SORTMODE_COMMAND;
	    cmdPtr = objv[i+1];
	    i++;
	    break;
	case LSORT_DECREASING:
	    sortInfo.isIncreasing = 0;
	    break;
	case LSORT_DICTIONARY:
	    sortInfo.sortMode = SORTMODE_DICTIONARY;
	    break;
	case LSORT_INCREASING:
	    sortInfo.isIncreasing = 1;
	    break;
	case LSORT_INDEX: {
	    Tcl_Size sortindex;
	    Tcl_Obj **indexv;

	    if (i == objc-2) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"\"-index\" option must be followed by list index",
			-1));
		Tcl_SetErrorCode(interp, "TCL", "ARGUMENT", "MISSING", (char *)NULL);
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    if (TclListObjGetElements(interp, objv[i+1], &sortindex,
		    &indexv) != TCL_OK) {
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }

	    /*
	     * Check each of the indices for syntactic correctness. Note that
	     * we do not store the converted values here because we do not
	     * know if this is the only -index option yet and so we can't
	     * allocate any space; that happens after the scan through all the
	     * options is done.
	     */

	    for (j=0 ; j<sortindex ; j++) {
		int encoded = 0;
		int result = TclIndexEncode(interp, indexv[j],
			TCL_INDEX_NONE, TCL_INDEX_NONE, &encoded);

		if ((result == TCL_OK) && (encoded == (int)TCL_INDEX_NONE)) {
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "index \"%s\" out of range",
			    TclGetString(indexv[j])));
		    Tcl_SetErrorCode(interp, "TCL", "VALUE", "INDEX", "OUTOFRANGE", (char *)NULL);
		    result = TCL_ERROR;
		}
		if (result == TCL_ERROR) {
		    Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
			    "\n    (-index option item number %" TCL_Z_MODIFIER "u)", j));
		    sortInfo.resultCode = TCL_ERROR;
		    goto done;
		}
	    }
	    indexPtr = objv[i+1];
	    i++;
	    break;
	}
	case LSORT_INTEGER:
	    sortInfo.sortMode = SORTMODE_INTEGER;
	    break;
	case LSORT_NOCASE:
	    nocase = 1;
	    break;
	case LSORT_REAL:
	    sortInfo.sortMode = SORTMODE_REAL;
	    break;
	case LSORT_UNIQUE:
	    sortInfo.unique = 1;
	    break;
	case LSORT_INDICES:
	    indices = 1;
	    break;
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
	default:
	    TCL_UNREACHABLE();
	}
    }
    if (nocase && (sortInfo.sortMode == SORTMODE_ASCII)) {
	sortInfo.sortMode = SORTMODE_ASCII_NC;
    }

    /*
     * Now extract the -index list for real, if present. No failures are
     * expected here; the values are all of the right type or convertible to
     * it.
     */

    if (indexPtr) {
	Tcl_Obj **indexv;

	TclListObjGetElements(interp, indexPtr, &sortInfo.indexc, &indexv);
	switch (sortInfo.indexc) {
	case 0:
	    sortInfo.indexv = NULL;
	    break;
	case 1:
	    sortInfo.indexv = &sortInfo.singleIndex;
	    break;
	default:
	    sortInfo.indexv = (int *)
		    TclStackAlloc(interp, sizeof(int) * sortInfo.indexc);
	    allocatedIndexVector = 1;	/* Cannot use indexc field, as it
					 * might be decreased by 1 later. */
	}
	for (j=0 ; j<sortInfo.indexc ; j++) {
	    /* Prescreened values, no errors or out of range possible */
	    TclIndexEncode(NULL, indexv[j], TCL_INDEX_NONE,
		    TCL_INDEX_NONE, &sortInfo.indexv[j]);
	}
    }

    listObj = objv[objc-1];

    if (sortInfo.sortMode == SORTMODE_COMMAND) {
	Tcl_Obj *newCommandPtr, *newObjPtr;

	/*
	 * When sorting using a command, we are reentrant and therefore might
	 * have the representation of the list being sorted shimmered out from
	 * underneath our feet. Take a copy (cheap) to prevent this. [Bug
	 * 1675116]
	 */

	listObj = TclListObjCopy(interp, listObj);
	if (listObj == NULL) {
	    sortInfo.resultCode = TCL_ERROR;
	    goto done;
	}

	/*
	 * The existing command is a list. We want to flatten it, append two
	 * dummy arguments on the end, and replace these arguments later.
	 */

	newCommandPtr = Tcl_DuplicateObj(cmdPtr);
	TclNewObj(newObjPtr);
	Tcl_IncrRefCount(newCommandPtr);
	if (Tcl_ListObjAppendElement(interp, newCommandPtr, newObjPtr)
		!= TCL_OK) {
	    TclDecrRefCount(newCommandPtr);
	    TclDecrRefCount(newObjPtr);
	    sortInfo.resultCode = TCL_ERROR;
	    goto done;
	}
	Tcl_ListObjAppendElement(interp, newCommandPtr, Tcl_NewObj());
	sortInfo.compareCmdPtr = newCommandPtr;
    }

    if (TclObjTypeHasProc(objv[1], getElementsProc)) {
	sortInfo.resultCode = TclObjTypeGetElements(interp, listObj,
		&length, &listObjPtrs);
    } else {
	sortInfo.resultCode = TclListObjGetElements(interp, listObj,
		&length, &listObjPtrs);
    }
    if (sortInfo.resultCode != TCL_OK || length <= 0) {
	goto done;
    }

    /*
     * Check for sanity when grouping elements of the overall list together
     * because of the -stride option. [TIP #326]
     */

    if (group) {
	if (length % groupSize) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "list size must be a multiple of the stride length",
		    -1));
	    Tcl_SetErrorCode(interp, "TCL", "OPERATION", "LSORT", "BADSTRIDE",
		    (char *)NULL);
	    sortInfo.resultCode = TCL_ERROR;
	    goto done;
	}
	length = length / groupSize;
	if (sortInfo.indexc > 0) {
	    /*
	     * Use the first value in the list supplied to -index as the
	     * offset of the element within each group by which to sort.
	     */

	    groupOffset = TclIndexDecode(sortInfo.indexv[0], groupSize - 1);
	    if (groupOffset < 0 || groupOffset >= groupSize) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"when used with \"-stride\", the leading \"-index\""
			" value must be within the group", -1));
		Tcl_SetErrorCode(interp, "TCL", "OPERATION", "LSORT",
			"BADINDEX", (char *)NULL);
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    if (sortInfo.indexc == 1) {
		sortInfo.indexc = 0;
		sortInfo.indexv = NULL;
	    } else {
		sortInfo.indexc--;

		/*
		 * Do not shrink the actual memory block used; that doesn't
		 * work with TclStackAlloc-allocated memory. [Bug 2918962]
		 *
		 * TODO: Consider a pointer increment to replace this
		 * array shift.
		 */

		for (i = 0; i < sortInfo.indexc; i++) {
		    sortInfo.indexv[i] = sortInfo.indexv[i+1];
		}
	    }
	}
    }

    sortInfo.numElements = length;

    indexc = sortInfo.indexc;
    sortMode = sortInfo.sortMode;
    if ((sortMode == SORTMODE_ASCII_NC)
	    || (sortMode == SORTMODE_DICTIONARY)) {
	/*
	 * For this function's purpose all string-based modes are equivalent
	 */

	sortMode = SORTMODE_ASCII;
    }

    /*
     * Initialize the sublists. After the following loop, subList[i] will
     * contain a sorted sublist of length 2**i. Use one extra subList at the
     * end, always at NULL, to indicate the end of the lists.
     */

    for (j=0 ; j<=NUM_LISTS ; j++) {
	subList[j] = NULL;
    }

    /*
     * The following loop creates a SortElement for each list element and
     * begins sorting it into the sublists as it appears.
     */

    elmArrSize = length * sizeof(SortElement);
    if (elmArrSize <= MAXCALLOC) {
	elementArray = (SortElement *)Tcl_Alloc(elmArrSize);
    } else {
	elementArray = (SortElement *)malloc(elmArrSize);
    }
    if (!elementArray) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"no enough memory to proccess sort of %" TCL_Z_MODIFIER "u items", length));
	Tcl_SetErrorCode(interp, "TCL", "MEMORY", (char *)NULL);
	sortInfo.resultCode = TCL_ERROR;
	goto done;
    }

    for (i=0; i < length; i++) {
	idx = groupSize * i + groupOffset;
	if (indexc) {
	    /*
	     * If this is an indexed sort, retrieve the corresponding element
	     */
	    indexPtr = SelectObjFromSublist(listObjPtrs[idx], &sortInfo);
	    if (sortInfo.resultCode != TCL_OK) {
		goto done;
	    }
	} else {
	    indexPtr = listObjPtrs[idx];
	}

	/*
	 * Determine the "value" of this object for sorting purposes
	 */

	if (sortMode == SORTMODE_ASCII) {
	    elementArray[i].collationKey.strValuePtr = TclGetString(indexPtr);
	} else if (sortMode == SORTMODE_INTEGER) {
	    Tcl_WideInt a;

	    if (TclGetWideIntFromObj(sortInfo.interp, indexPtr, &a) != TCL_OK) {
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    elementArray[i].collationKey.wideValue = a;
	} else if (sortMode == SORTMODE_REAL) {
	    double a;

	    if (Tcl_GetDoubleFromObj(sortInfo.interp, indexPtr, &a) != TCL_OK) {
		sortInfo.resultCode = TCL_ERROR;
		goto done;
	    }
	    elementArray[i].collationKey.doubleValue = a;
	} else {
	    elementArray[i].collationKey.objValuePtr = indexPtr;
	}

	/*
	 * Determine the representation of this element in the result: either
	 * the objPtr itself, or its index in the original list.
	 */

	if (indices || group) {
	    elementArray[i].payload.index = idx;
	} else {
	    elementArray[i].payload.objPtr = listObjPtrs[idx];
	}

	/*
	 * Merge this element in the preexisting sublists (and merge together
	 * sublists when we have two of the same size).
	 */

	elementArray[i].nextPtr = NULL;
	elementPtr = &elementArray[i];
	for (j=0 ; subList[j] ; j++) {
	    elementPtr = MergeLists(subList[j], elementPtr, &sortInfo);
	    subList[j] = NULL;
	}
	if (j >= NUM_LISTS) {
	    j = NUM_LISTS-1;
	}
	subList[j] = elementPtr;
    }

    /*
     * Merge all sublists
     */

    elementPtr = subList[0];
    for (j=1 ; j<NUM_LISTS ; j++) {
	elementPtr = MergeLists(subList[j], elementPtr, &sortInfo);
    }

    /*
     * Now store the sorted elements in the result list.
     */

    if (sortInfo.resultCode == TCL_OK) {
	ListRep listRep;
	Tcl_Obj **newArray, *objPtr;

	resultPtr = Tcl_NewListObj(sortInfo.numElements * groupSize, NULL);
	ListObjGetRep(resultPtr, &listRep);
	newArray = ListRepElementsBase(&listRep);
	if (group) {
	    for (i=0; elementPtr!=NULL ; elementPtr=elementPtr->nextPtr) {
		idx = elementPtr->payload.index;
		for (j = 0; j < groupSize; j++) {
		    if (indices) {
			TclNewIndexObj(objPtr, idx + j - groupOffset);
			newArray[i++] = objPtr;
			Tcl_IncrRefCount(objPtr);
		    } else {
			objPtr = listObjPtrs[idx + j - groupOffset];
			newArray[i++] = objPtr;
			Tcl_IncrRefCount(objPtr);
		    }
		}
	    }
	} else if (indices) {
	    for (i=0; elementPtr != NULL ; elementPtr = elementPtr->nextPtr) {
		TclNewIndexObj(objPtr, elementPtr->payload.index);
		newArray[i++] = objPtr;
		Tcl_IncrRefCount(objPtr);
	    }
	} else {
	    for (i=0; elementPtr != NULL ; elementPtr = elementPtr->nextPtr) {
		objPtr = elementPtr->payload.objPtr;
		newArray[i++] = objPtr;
		Tcl_IncrRefCount(objPtr);
	    }
	}
	listRep.storePtr->numUsed = i;
	if (listRep.spanPtr) {
	    listRep.spanPtr->spanStart = listRep.storePtr->firstUsed;
	    listRep.spanPtr->spanLength = listRep.storePtr->numUsed;
	}
	Tcl_SetObjResult(interp, resultPtr);
    }

  done:
    if (sortMode == SORTMODE_COMMAND) {
	TclDecrRefCount(sortInfo.compareCmdPtr);
	TclDecrRefCount(listObj);
	sortInfo.compareCmdPtr = NULL;
    }
    if (allocatedIndexVector) {
	TclStackFree(interp, sortInfo.indexv);
    }
    if (elementArray) {
	if (elmArrSize <= MAXCALLOC) {
	    Tcl_Free(elementArray);
	} else {
	    free((char *)elementArray);
	}
    }
    return sortInfo.resultCode;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdIL.c`, function `SortCompare: comparison of prepared numeric values and command result primitive`, lines 5080–5165. Line unit: physical LF only. Original byte range [135184, 137727). Retained reference `source-window-16`.

```text
SortCompare(
    SortElement *elemPtr1, SortElement *elemPtr2,
				/* Values to be compared. */
    SortInfo *infoPtr)		/* Information passed from the top-level
				 * "lsort" command. */
{
    int order = 0;

    if (infoPtr->sortMode == SORTMODE_ASCII) {
	order = TclUtfCmp(elemPtr1->collationKey.strValuePtr,
		elemPtr2->collationKey.strValuePtr);
    } else if (infoPtr->sortMode == SORTMODE_ASCII_NC) {
	order = TclUtfCasecmp(elemPtr1->collationKey.strValuePtr,
		elemPtr2->collationKey.strValuePtr);
    } else if (infoPtr->sortMode == SORTMODE_DICTIONARY) {
	order = DictionaryCompare(elemPtr1->collationKey.strValuePtr,
		elemPtr2->collationKey.strValuePtr);
    } else if (infoPtr->sortMode == SORTMODE_INTEGER) {
	Tcl_WideInt a, b;

	a = elemPtr1->collationKey.wideValue;
	b = elemPtr2->collationKey.wideValue;
	order = ((a >= b) - (a <= b));
    } else if (infoPtr->sortMode == SORTMODE_REAL) {
	double a, b;

	a = elemPtr1->collationKey.doubleValue;
	b = elemPtr2->collationKey.doubleValue;
	order = ((a >= b) - (a <= b));
    } else {
	Tcl_Obj **objv, *paramObjv[2];
	Tcl_Size objc;
	Tcl_Obj *objPtr1, *objPtr2;

	if (infoPtr->resultCode != TCL_OK) {
	    /*
	     * Once an error has occurred, skip any future comparisons so as
	     * to preserve the error message in sortInterp->result.
	     */

	    return 0;
	}

	objPtr1 = elemPtr1->collationKey.objValuePtr;
	objPtr2 = elemPtr2->collationKey.objValuePtr;

	paramObjv[0] = objPtr1;
	paramObjv[1] = objPtr2;

	/*
	 * We made space in the command list for the two things to compare.
	 * Replace them and evaluate the result.
	 */

	TclListObjLength(infoPtr->interp, infoPtr->compareCmdPtr, &objc);
	Tcl_ListObjReplace(infoPtr->interp, infoPtr->compareCmdPtr, objc - 2,
		2, 2, paramObjv);
	TclListObjGetElements(infoPtr->interp, infoPtr->compareCmdPtr,
		&objc, &objv);

	infoPtr->resultCode = Tcl_EvalObjv(infoPtr->interp, objc, objv, 0);

	if (infoPtr->resultCode != TCL_OK) {
	    Tcl_AddErrorInfo(infoPtr->interp, "\n    (-compare command)");
	    return 0;
	}

	/*
	 * Parse the result of the command.
	 */

	if (TclGetIntFromObj(infoPtr->interp,
		Tcl_GetObjResult(infoPtr->interp), &order) != TCL_OK) {
	    Tcl_SetObjResult(infoPtr->interp, Tcl_NewStringObj(
		    "-compare command returned non-integer result", -1));
	    Tcl_SetErrorCode(infoPtr->interp, "TCL", "OPERATION", "LSORT",
		    "COMPARISONFAILED", (char *)NULL);
	    infoPtr->resultCode = TCL_ERROR;
	    return 0;
	}
    }
    if (!infoPtr->isIncreasing) {
	order = -order;
    }
    return order;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `SetIntFromAny: original selected integer primitive`, lines 6307–6336. Line unit: physical LF only. Original byte range [195501, 196500). Retained reference `source-window-17`.

```text
static int SetIntFromAny(Jim_Interp *interp, Jim_Obj *objPtr, int flags)
{
    jim_wide wideValue;
    const char *str;

    if (objPtr->typePtr == &coercedDoubleObjType) {
        /* Simple switch */
        objPtr->typePtr = &intObjType;
        return JIM_OK;
    }

    /* Get the string representation */
    str = Jim_String(objPtr);
    /* Try to convert into a jim_wide */
    if (Jim_StringToWide(str, &wideValue, 0) != JIM_OK) {
        if (flags & JIM_ERRMSG) {
            Jim_SetResultFormatted(interp, "expected integer but got \"%#s\"", objPtr);
        }
        return JIM_ERR;
    }
    if ((wideValue == JIM_WIDE_MIN || wideValue == JIM_WIDE_MAX) && errno == ERANGE) {
        Jim_SetResultString(interp, "Integer value too big to be represented", -1);
        return JIM_ERR;
    }
    /* Free the old internal repr and set the new one. */
    Jim_FreeIntRep(interp, objPtr);
    objPtr->typePtr = &intObjType;
    objPtr->internalRep.wideValue = wideValue;
    return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_GetWide: actual original Int getter`, lines 6346–6352. Line unit: physical LF only. Original byte range [196685, 196938). Retained reference `source-window-18`.

```text
int Jim_GetWide(Jim_Interp *interp, Jim_Obj *objPtr, jim_wide * widePtr)
{
    if (objPtr->typePtr != &intObjType && SetIntFromAny(interp, objPtr, JIM_ERRMSG) == JIM_ERR)
        return JIM_ERR;
    *widePtr = JimWideValue(objPtr);
    return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `SetDoubleFromAny: original selected real primitive`, lines 6495–6539. Line unit: physical LF only. Original byte range [200986, 202481). Retained reference `source-window-19`.

```text
static int SetDoubleFromAny(Jim_Interp *interp, Jim_Obj *objPtr)
{
    double doubleValue;
    jim_wide wideValue;
    const char *str;

#ifdef HAVE_LONG_LONG
    /* Assume a 53 bit mantissa */
#define MIN_INT_IN_DOUBLE -(1LL << 53)
#define MAX_INT_IN_DOUBLE -(MIN_INT_IN_DOUBLE + 1)

    if (objPtr->typePtr == &intObjType
        && JimWideValue(objPtr) >= MIN_INT_IN_DOUBLE
        && JimWideValue(objPtr) <= MAX_INT_IN_DOUBLE) {

        /* Direct conversion to coerced double */
        objPtr->typePtr = &coercedDoubleObjType;
        return JIM_OK;
    }
#endif
    /* Preserve the string representation.
     * Needed so we can convert back to int without loss
     */
    str = Jim_String(objPtr);

    if (Jim_StringToWide(str, &wideValue, 10) == JIM_OK) {
        /* Managed to convert to an int, so we can use this as a cooerced double */
        Jim_FreeIntRep(interp, objPtr);
        objPtr->typePtr = &coercedDoubleObjType;
        objPtr->internalRep.wideValue = wideValue;
        return JIM_OK;
    }
    else {
        /* Try to convert into a double */
        if (Jim_StringToDouble(str, &doubleValue) != JIM_OK) {
            Jim_SetResultFormatted(interp, "expected floating-point number but got \"%#s\"", objPtr);
            return JIM_ERR;
        }
        /* Free the old internal repr and set the new one. */
        Jim_FreeIntRep(interp, objPtr);
    }
    objPtr->typePtr = &doubleObjType;
    objPtr->internalRep.doubleValue = doubleValue;
    return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_GetDouble: actual original Double getter`, lines 6541–6557. Line unit: physical LF only. Original byte range [202482, 202990). Retained reference `source-window-20`.

```text
int Jim_GetDouble(Jim_Interp *interp, Jim_Obj *objPtr, double *doublePtr)
{
    if (objPtr->typePtr == &coercedDoubleObjType) {
        *doublePtr = JimWideValue(objPtr);
        return JIM_OK;
    }
    if (objPtr->typePtr != &doubleObjType && SetDoubleFromAny(interp, objPtr) == JIM_ERR)
        return JIM_ERR;

    if (objPtr->typePtr == &coercedDoubleObjType) {
        *doublePtr = JimWideValue(objPtr);
    }
    else {
        *doublePtr = objPtr->internalRep.doubleValue;
    }
    return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `ListSortIndexHelper: original left/right selected index drill`, lines 7053–7062. Line unit: physical LF only. Original byte range [217634, 218070). Retained reference `source-window-21`.

```text
static int ListSortIndexHelper(Jim_Obj **lhsObj, Jim_Obj **rhsObj)
{
    Jim_Obj *lObj, *rObj;

    if (Jim_ListIndices(sort_info->interp, *lhsObj, sort_info->indexv, sort_info->indexc, &lObj, JIM_ERRMSG) != JIM_OK ||
        Jim_ListIndices(sort_info->interp, *rhsObj, sort_info->indexv, sort_info->indexc, &rObj, JIM_ERRMSG) != JIM_OK) {
        longjmp(sort_info->jmpbuf, JIM_ERR);
    }
    return sort_info->subfn(&lObj, &rObj);
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `ListSortInteger: original Wide conversion and definedness-sensitive subtraction`, lines 7117–7127. Line unit: physical LF only. Original byte range [220071, 220406). Retained reference `source-window-22`.

```text
static int ListSortInteger(Jim_Obj **lhsObj, Jim_Obj **rhsObj)
{
    jim_wide lhs = 0, rhs = 0;

    if (Jim_GetWide(sort_info->interp, *lhsObj, &lhs) != JIM_OK ||
        Jim_GetWide(sort_info->interp, *rhsObj, &rhs) != JIM_OK) {
        longjmp(sort_info->jmpbuf, JIM_ERR);
    }

    return JimSign(lhs - rhs) * sort_info->order;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `ListSortReal: original Double conversion and NaN comparison relation`, lines 7130–7145. Line unit: physical LF only. Original byte range [220466, 220885). Retained reference `source-window-23`.

```text
static int ListSortReal(Jim_Obj **lhsObj, Jim_Obj **rhsObj)
{
    double lhs = 0, rhs = 0;

    if (Jim_GetDouble(sort_info->interp, *lhsObj, &lhs) != JIM_OK ||
        Jim_GetDouble(sort_info->interp, *rhsObj, &rhs) != JIM_OK) {
        longjmp(sort_info->jmpbuf, JIM_ERR);
    }
    if (lhs == rhs) {
        return 0;
    }
    if (lhs > rhs) {
        return sort_info->order;
    }
    return -sort_info->order;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `ListSortElements: genuine qsort, unique and original String invalidation`, lines 7203–7265. Line unit: physical LF only. Original byte range [222715, 224536). Retained reference `source-window-24`.

```text
static int ListSortElements(Jim_Interp *interp, Jim_Obj *listObjPtr, struct lsort_info *info)
{
    struct lsort_info *prev_info;

    typedef int (qsort_comparator) (const void *, const void *);
    int (*fn) (Jim_Obj **, Jim_Obj **);
    Jim_Obj **vector;
    int len;
    int rc;

    JimPanic((Jim_IsShared(listObjPtr), "ListSortElements called with shared object"));
    SetListFromAny(interp, listObjPtr);

    /* Allow lsort to be called reentrantly */
    prev_info = sort_info;
    sort_info = info;

    vector = listObjPtr->internalRep.listValue.ele;
    len = listObjPtr->internalRep.listValue.len;
    switch (info->type) {
        case JIM_LSORT_ASCII:
            fn = ListSortString;
            break;
        case JIM_LSORT_NOCASE:
            fn = ListSortStringNoCase;
            break;
        case JIM_LSORT_INTEGER:
            fn = ListSortInteger;
            break;
        case JIM_LSORT_REAL:
            fn = ListSortReal;
            break;
        case JIM_LSORT_COMMAND:
            fn = ListSortCommand;
            break;
        case JIM_LSORT_DICT:
            fn = ListSortDict;
            break;
        default:
            fn = NULL;          /* avoid warning */
            JimPanic((1, "ListSort called with invalid sort type"));
            return -1; /* Should not be run but keeps static analysers happy */
    }

    if (info->indexc) {
        /* Need to interpose a "list index" function */
        info->subfn = fn;
        fn = ListSortIndexHelper;
    }

    if ((rc = setjmp(info->jmpbuf)) == 0) {
        qsort(vector, len, sizeof(Jim_Obj *), (qsort_comparator *) fn);

        if (info->unique && len > 1) {
            ListRemoveDuplicates(listObjPtr, fn);
        }

        Jim_InvalidateStringRep(listObjPtr);
    }
    sort_info = prev_info;

    return rc;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_LsortCoreCommand: option table, <=1 alias and group/List producer`, lines 13831–13970. Line unit: physical LF only. Original byte range [436968, 441634). Retained reference `source-window-25`.

```text
static int Jim_LsortCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const argv[])
{
    static const char * const options[] = {
        "-ascii", "-nocase", "-increasing", "-decreasing", "-command", "-integer", "-real", "-index", "-unique",
        "-stride", "-dictionary", NULL
    };
    enum {
        OPT_ASCII, OPT_NOCASE, OPT_INCREASING, OPT_DECREASING, OPT_COMMAND, OPT_INTEGER, OPT_REAL, OPT_INDEX, OPT_UNIQUE,
        OPT_STRIDE, OPT_DICT
    };
    Jim_Obj *resObj;
    int i;
    int retCode;
    int shared;
    long stride = 1;
    Jim_Obj **elements;
    int listlen;

    struct lsort_info info;

    info.type = JIM_LSORT_ASCII;
    info.order = 1;
    info.indexc = 0;
    info.unique = 0;
    info.command = NULL;
    info.interp = interp;

    for (i = 1; i < (argc - 1); i++) {
        int option;

        if (Jim_GetEnum(interp, argv[i], options, &option, NULL, JIM_ENUM_ABBREV | JIM_ERRMSG)
            != JIM_OK)
            return JIM_ERR;
        switch (option) {
            case OPT_ASCII:
                info.type = JIM_LSORT_ASCII;
                break;
            case OPT_DICT:
                info.type = JIM_LSORT_DICT;
                break;
            case OPT_NOCASE:
                info.type = JIM_LSORT_NOCASE;
                break;
            case OPT_INTEGER:
                info.type = JIM_LSORT_INTEGER;
                break;
            case OPT_REAL:
                info.type = JIM_LSORT_REAL;
                break;
            case OPT_INCREASING:
                info.order = 1;
                break;
            case OPT_DECREASING:
                info.order = -1;
                break;
            case OPT_UNIQUE:
                info.unique = 1;
                break;
            case OPT_COMMAND:
                if (i >= (argc - 2)) {
                    Jim_SetResultString(interp, "\"-command\" option must be followed by comparison command", -1);
                    return JIM_ERR;
                }
                info.type = JIM_LSORT_COMMAND;
                info.command = argv[i + 1];
                i++;
                break;
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
            case OPT_INDEX:
                if (i >= (argc - 2)) {
badindex:
                    Jim_SetResultString(interp, "\"-index\" option must be followed by list index", -1);
                    return JIM_ERR;
                }
                JimListGetElements(interp, argv[i + 1], &info.indexc, &info.indexv);
                if (info.indexc == 0) {
                    goto badindex;
                }
                i++;
                break;
        }
    }
    resObj = argv[argc - 1];
    JimListGetElements(interp, resObj, &listlen, &elements);
    if (listlen <= 1) {
        /* Nothing to do */
        Jim_SetResult(interp, resObj);
        return JIM_OK;
    }

    if (stride > 1) {
        Jim_Obj *tmpListObj;
        int i;

        if (listlen % stride) {
            Jim_SetResultString(interp, "list size must be a multiple of the stride length", -1);
            return JIM_ERR;
        }
        /* Need to create a new list of lists for sorting */
        tmpListObj = Jim_NewListObj(interp, NULL, 0);
        Jim_IncrRefCount(tmpListObj);
        for (i = 0; i < listlen; i += stride) {
            Jim_ListAppendElement(interp, tmpListObj, Jim_NewListObj(interp, elements + i, stride));
        }
        retCode = ListSortElements(interp, tmpListObj, &info);
        if (retCode == JIM_OK) {
            resObj = Jim_NewListObj(interp, NULL, 0);
            /* Now we need to unpack the result back into a flat list */
            for (i = 0; i < listlen; i += stride) {
                Jim_ListAppendList(interp, resObj, Jim_ListGetIndex(interp, tmpListObj, i / stride));
            }
            Jim_SetResult(interp, resObj);
        }
        Jim_DecrRefCount(interp, tmpListObj);
    }
    else {
        if ((shared = Jim_IsShared(resObj))) {
            resObj = Jim_DuplicateObj(interp, resObj);
        }
        retCode = ListSortElements(interp, resObj, &info);
        if (retCode == JIM_OK) {
            Jim_SetResult(interp, resObj);
        }
        else if (shared) {
            Jim_FreeNewObj(interp, resObj);
        }
    }
    return retCode;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_LsearchCoreCommand: actual table rejects integer/real options`, lines 13502–13723. Line unit: physical LF only. Original byte range [426846, 433891). Retained reference `source-window-26`.

```text
static int Jim_LsearchCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    static const char * const options[] = {
        "-bool", "-not", "-nocase", "-exact", "-glob", "-regexp", "-all", "-inline", "-command",
        "-stride", "-index", NULL
    };
    enum
    { OPT_BOOL, OPT_NOT, OPT_NOCASE, OPT_EXACT, OPT_GLOB, OPT_REGEXP, OPT_ALL, OPT_INLINE,
            OPT_COMMAND, OPT_STRIDE, OPT_INDEX };
    int i;
    int opt_bool = 0;
    int opt_not = 0;
    int opt_all = 0;
    int opt_inline = 0;
    int opt_match = OPT_EXACT;
    int listlen;
    int rc = JIM_OK;
    Jim_Obj *listObjPtr = NULL;
    Jim_Obj *commandObj = NULL;
    Jim_Obj *indexObj = NULL;
    int match_flags = 0;
    long stride = 1;

    for (i = 1; i < argc - 2; i++) {
        int option;

        if (Jim_GetEnum(interp, argv[i], options, &option, NULL, JIM_ERRMSG) != JIM_OK) {
            return JIM_ERR;
        }
        switch (option) {
            case OPT_BOOL:
                opt_bool = 1;
                opt_inline = 0;
                break;
            case OPT_NOT:
                opt_not = 1;
                break;
            case OPT_NOCASE:
                match_flags |= JIM_NOCASE;
                break;
            case OPT_INLINE:
                opt_inline = 1;
                opt_bool = 0;
                break;
            case OPT_ALL:
                opt_all = 1;
                break;
            case OPT_REGEXP:
                opt_match = option;
                match_flags |= JIM_OPT_END;
                break;
            case OPT_COMMAND:
                if (i >= argc - 2) {
                    return JIM_USAGE;
                }
                commandObj = argv[++i];
                /* fallthru */
            case OPT_EXACT:
            case OPT_GLOB:
                opt_match = option;
                break;
            case OPT_INDEX:
                if (i >= argc - 2) {
                    return JIM_USAGE;
                }
                indexObj = argv[++i];
                break;
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
        }
    }

    argc -= i;
    if (argc < 2) {
        return JIM_USAGE;
    }
    argv += i;

    listlen = Jim_ListLength(interp, argv[0]);
    if (listlen % stride) {
        Jim_SetResultString(interp, "list size must be a multiple of the stride length", -1);
        return JIM_ERR;
    }

    if (opt_all) {
        listObjPtr = Jim_NewListObj(interp, NULL, 0);
    }
    if (opt_match == OPT_REGEXP) {
        commandObj = Jim_NewStringObj(interp, "regexp", -1);
    }
    if (commandObj) {
        Jim_IncrRefCount(commandObj);
    }

    for (i = 0; i < listlen; i += stride) {
        int eq = 0;
        Jim_Obj *searchListObj;
        Jim_Obj *objPtr;
        int offset;

        if (indexObj) {
            int indexlen = Jim_ListLength(interp, indexObj);
            if (stride == 1) {
                searchListObj = Jim_ListGetIndex(interp, argv[0], i);
            }
            else {
                searchListObj = Jim_NewListObj(interp, argv[0]->internalRep.listValue.ele + i, stride);
            }
            Jim_IncrRefCount(searchListObj);
            rc = Jim_ListIndices(interp, searchListObj, indexObj->internalRep.listValue.ele, indexlen, &objPtr, JIM_ERRMSG);
            if (rc != JIM_OK) {
                Jim_DecrRefCount(interp, searchListObj);
                rc = JIM_ERR;
                goto done;
            }
            /* now indexObj is the object to compare */
            offset = 0;
        }
        else {
            /* No -index, so we have an implicit {0} as indexObj */
            searchListObj = argv[0];
            offset = i;
            objPtr = Jim_ListGetIndex(interp, searchListObj, i);
            Jim_IncrRefCount(searchListObj);
        }
        /* At this point objPtr represents the object to search against and
         * searchListObj represents the list we search in (offset .. offset + stride - 1)
         * both need to have reference counts decremented when done
         */

        switch (opt_match) {
            case OPT_EXACT:
                eq = Jim_StringCompareObj(interp, argv[1], objPtr, match_flags) == 0;
                break;

            case OPT_GLOB:
                eq = Jim_StringMatchObj(interp, argv[1], objPtr, match_flags);
                break;

            case OPT_REGEXP:
            case OPT_COMMAND:
                eq = JimCommandMatchObj(interp, commandObj, argv[1], objPtr, match_flags);
                if (eq < 0) {
                    Jim_DecrRefCount(interp, searchListObj);
                    rc = JIM_ERR;
                    goto done;
                }
                break;
        }

        /* Got a match (or non-match for opt_not), or (opt_bool && opt_all) */
        if ((!opt_bool && eq == !opt_not) || (opt_bool && (eq || opt_all))) {
            Jim_Obj *resultObj;

            if (opt_bool) {
                resultObj = Jim_NewIntObj(interp, eq ^ opt_not);
            }
            else if (!opt_inline) {
                resultObj = Jim_NewIntObj(interp, i);
            }
            else if (stride == 1) {
                resultObj = objPtr;
            }
            else if (opt_all) {
                /* Add the entire sublist directly for -all -stride > 1 */
                ListInsertElements(listObjPtr, -1, stride,
                    searchListObj->internalRep.listValue.ele + offset);
                /* Not necessary, but some compilers can't figure that out */
                resultObj = NULL;
            }
            else {
                resultObj = Jim_NewListObj(interp, searchListObj->internalRep.listValue.ele + offset, stride);
            }

            if (opt_all) {
                /* The stride > 1 case has already been handled above */
                if (stride == 1) {
                    Jim_ListAppendElement(interp, listObjPtr, resultObj);
                }
            }
            else {
                Jim_SetResult(interp, resultObj);
                Jim_DecrRefCount(interp, searchListObj);
                goto done;
            }
        }
        Jim_DecrRefCount(interp, searchListObj);
    }

    if (opt_all) {
        Jim_SetResult(interp, listObjPtr);
        listObjPtr = NULL;
    }
    else {
        /* No match */
        if (opt_bool) {
            Jim_SetResultBool(interp, opt_not);
        }
        else if (!opt_inline) {
            Jim_SetResultInt(interp, -1);
        }
    }

  done:
    if (listObjPtr) {
        Jim_FreeNewObj(interp, listObjPtr);
    }
    if (commandObj) {
        Jim_DecrRefCount(interp, commandObj);
    }
    return rc;
}

```

## Consumer bindings

- [rust/tcl-cmd-core/src/native_list_numeric.rs](../../../../rust/tcl-cmd-core/src/native_list_numeric.rs), `sort_order`: The same pure numeric comparison kernel keeps source-selected NaN search/sort outcomes separate from checked integer-subtraction definedness. Signed extreme overflow refuses instead of inventing a wrapping native comparator result; this source control provides no extreme Native observation, getter/cache authority or passed assertion.
- [rust/tcl-cmd-core/src/native_list_numeric.rs](../../../../rust/tcl-cmd-core/src/native_list_numeric.rs), `native_list_numeric::bounded_comparison_controls::source_selected_unordered_and_defined_subtraction_boundaries_are_explicit` (linked): The same pure numeric comparison kernel keeps source-selected NaN search/sort outcomes separate from checked integer-subtraction definedness. Signed extreme overflow refuses instead of inventing a wrapping native comparator result; this source control provides no extreme Native observation, getter/cache authority or passed assertion.
- [rust/tcl-registry/src/native_numeric_error/command.rs](../../../../rust/tcl-registry/src/native_numeric_error/command.rs), `list_numeric_recipes_select_original_search_sort_widths_and_jim_option_support` (linked): Software source definition checks list numeric recipes select original search sort widths and jim option support.
- [rust/tcl-vm/src/cmd_list/native_original_numeric_controls.rs](../../../../rust/tcl-vm/src/cmd_list/native_original_numeric_controls.rs), `original_numeric_search_sort_public_predicates_match_independent_native645_and647` (linked): Software source definition checks numeric search sort public predicates match independent native645 and647.
- [rust/tcl-host-c-abi/src/native_list_sort.rs](../../../../rust/tcl-host-c-abi/src/native_list_sort.rs), `actual_local_sort_has_no_comparator_for_zero_or_one_and_refuses_invalid_permutations` (linked): Software source definition checks actual local sort has no comparator for zero or one and refuses invalid permutations.
- [rust/tcl-host-c-abi/src/native_list_sort.rs](../../../../rust/tcl-host-c-abi/src/native_list_sort.rs), `actual_local_sort_stops_callback_effects_and_classifies_the_real_error_frontier` (linked): Software source definition checks actual local sort stops callback effects and classifies the real error frontier.
- [runtime/rust/src/cmd_list/native_original_numeric_controls.rs](../../../../runtime/rust/src/cmd_list/native_original_numeric_controls.rs), `original_numeric_search_sort_public_predicates_match_independent_native645_and647` (linked): Software source definition checks numeric search sort public predicates match independent native645 and647.

## Evidence

- [runtime/rust/tests/data/native_original_list_numeric645_647](../../../../runtime/rust/tests/data/native_original_list_numeric645_647)
- [runtime/rust/tests/data/native_original_list_numeric645_647/glob645](../../../../runtime/rust/tests/data/native_original_list_numeric645_647/glob645)
- [runtime/rust/tests/data/native_original_list_numeric645_647/exact647](../../../../runtime/rust/tests/data/native_original_list_numeric645_647/exact647)
- [rust/tcl-cmd-core](../../../../rust/tcl-cmd-core)
