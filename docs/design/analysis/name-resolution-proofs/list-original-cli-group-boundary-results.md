# naming.list.original-cli-group-boundary-results

Kind: `native-observation`

## Problem statement

Original-header measurements and initialized CLI source answer different questions. A failing leading group coordinate needs its own actual public code/errorCode/errorInfo output, and successful command routes expose a callback count without granting private getter or sort chronology.

## Question

For these 11 exact initialized CLI source invocations, which guest code/result/errorCode/errorInfo and genuine callback counts are exposed on each actual provider?

## Conclusion

C84/85 reject stride through their actual option tables. C86/9 leading2 produces BADINDEX; minimum and nonmultiple width produce BADSTRIDE. C86 rejects stride4294967296 at its Int boundary, while C9/Jim reach the group-size error. Jim leading2 fails on the genuine group sublist. Every original process completes; these public fields grant no private chronology.

## Scope

66 fresh actual provider CLI processes, six providers by 11 exact cases, all exit0 with complete ASCII projections. The complete probe source has explicit backslash-LF syntax, no NUL or non-ASCII input. Software controls execute the exact capture prefix with separately initialised argv and compare exact selected script/code/result/errorCode/errorInfo/presence/count; they do not claim CLI startup, print/hex helper or load/build equivalence. Callback count is only an observed public field. Original651 Jim case12 is a different input and remains unavailable. No private header/cache/getter/teardown or libc schedule is inferred. Simultaneous size/leading failure has no capture here.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Fresh initialized CLI ASCII Tcl source; actual request/probe/script bytes and startup selection are recorded. This is separate from original651 native-created argv/header roles. Version: tcl8.4 8.4.20.

Every stride spelling is unsupported, with observed NONE and exact option text; all callback counts0.

### tcl8.5

Status: `observed`. Channel: Fresh initialized CLI ASCII Tcl source; actual request/probe/script bytes and startup selection are recorded. This is separate from original651 native-created argv/header roles. Version: tcl8.5 8.5.19.

Every stride spelling is unsupported, with observed TCL LOOKUP INDEX option -stride and exact option text; all callback counts0.

### tcl8.6

Status: `observed`. Channel: Fresh initialized CLI ASCII Tcl source; actual request/probe/script bytes and startup selection are recorded. This is separate from original651 native-created argv/header roles. Version: tcl8.6 8.6.18.

Leading2 gives BADINDEX; negative index is INDEXOUTOFRANGE with its -index context; minimum/nonmultiple width BADSTRIDE; wide stride overflows Int. Valid command case6 callback count1.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Fresh initialized CLI ASCII Tcl source; actual request/probe/script bytes and startup selection are recorded. This is separate from original651 native-created argv/header roles. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Leading2 gives BADINDEX; negative index is TCL VALUE INDEX OUTOFRANGE; minimum/nonmultiple and wide stride BADSTRIDE. Valid command case6 callback count1.

### jim

Status: `observed`. Channel: Fresh initialized CLI ASCII Tcl source; actual request/probe/script bytes and startup selection are recorded. This is separate from original651 native-created argv/header roles. Version: jim 0.84-9-g5bac7c9.

Leading2 reports element2 missing from group sublist; negative index, minimum/nonmultiple and wide stride report their primitive messages with NONE and no errorInfo global. Valid command case6 callback count1.

### bigip

Status: `not-tested`. Channel: No appliance source/result channel retained.

No public appliance code/errorInfo/callback result is inferred.

## Source inspection

C Tcl 8.4.20, revision `selected release source; revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdIL.c`, function `Tcl_LsortObjCmd, MergeSort/MergeLists and SortCompare: original prefix/operands, bucket merging and Int result`, lines 3693–4172. Line unit: physical LF only. Original byte range [105199, 118437). Retained reference `selected-producer-0`.

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

/*
 *----------------------------------------------------------------------
 *
 * MergeSort -
 *
 *	This procedure sorts a linked list of SortElement structures
 *	use the merge-sort algorithm.
 *
 * Results:
 *      A pointer to the head of the list after sorting is returned.
 *
 * Side effects:
 *	None, unless a user-defined comparison command does something
 *	weird.
 *
 *----------------------------------------------------------------------
 */

static SortElement *
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

/*
 *----------------------------------------------------------------------
 *
 * MergeLists -
 *
 *	This procedure combines two sorted lists of SortElement structures
 *	into a single sorted list.
 *
 * Results:
 *      The unified list of SortElement structures.
 *
 * Side effects:
 *	None, unless a user-defined comparison command does something
 *	weird.
 *
 *----------------------------------------------------------------------
 */

static SortElement *
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

/*
 *----------------------------------------------------------------------
 *
 * SortCompare --
 *
 *	This procedure is invoked by MergeLists to determine the proper
 *	ordering between two elements.
 *
 * Results:
 *      A negative results means the the first element comes before the
 *      second, and a positive results means that the second element
 *      should come first.  A result of zero means the two elements
 *      are equal and it doesn't matter which comes first.
 *
 * Side effects:
 *	None, unless a user-defined comparison command does something
 *	weird.
 *
 *----------------------------------------------------------------------
 */

static int
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

/*
 *----------------------------------------------------------------------
 *
 * DictionaryCompare
 *
 *	This function compares two strings as if they were being used in
 *	an index or card catalog.  The case of alphabetic characters is

```

C Tcl 8.5.19, revision `selected release source; revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdIL.c`, function `Tcl_LsortObjCmd, MergeSort/MergeLists and SortCompare: original prefix/operands, bucket merging and Int result`, lines 3463–4024. Line unit: physical LF only. Original byte range [92458, 107735). Retained reference `selected-producer-1`.

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

/*
 *----------------------------------------------------------------------
 *
 * MergeLists -
 *
 *	This procedure combines two sorted lists of SortElement structures
 *	into a single sorted list.
 *
 * Results:
 *	The unified list of SortElement structures.
 *
 * Side effects:
 *      If infoPtr->unique is set then infoPtr->numElements may be updated.
 *	Possibly others, if a user-defined comparison command does something
 *      weird. 
 *
 * Note:
 *      If infoPtr->unique is set, the merge assumes that there are no
 *	"repeated" elements in each of the left and right lists. In that case,
 *	if any element of the left list is equivalent to one in the right list
 *	it is omitted from the merged list.
 *      This simplified mechanism works because of the special way
 *	our MergeSort creates the sublists to be merged and will fail to
 *	eliminate all repeats in the general case where they are already
 *	present in either the left or right list. A general code would need to
 *	skip adjacent initial repeats in the left and right lists before
 *	comparing their initial elements, at each step. 
 *----------------------------------------------------------------------
 */

static SortElement *
MergeLists(
    SortElement *leftPtr,	/* First list to be merged; may be NULL. */
    SortElement *rightPtr,	/* Second list to be merged; may be NULL. */
    SortInfo *infoPtr)		/* Information needed by the comparison
				 * operator. */
{
    SortElement *headPtr, *tailPtr;
    int cmp;

    if (leftPtr == NULL) {
	return rightPtr;
    }
    if (rightPtr == NULL) {
	return leftPtr;
    }
    cmp = SortCompare(leftPtr, rightPtr, infoPtr);
    if (cmp > 0 || (cmp == 0 && infoPtr->unique)) {
	if (cmp == 0) {
	    infoPtr->numElements--;
	    leftPtr = leftPtr->nextPtr;
	}
	tailPtr = rightPtr;
	rightPtr = rightPtr->nextPtr;
    } else {
	tailPtr = leftPtr;
	leftPtr = leftPtr->nextPtr;
    }
    headPtr = tailPtr;
    if (!infoPtr->unique) {
	while ((leftPtr != NULL) && (rightPtr != NULL)) {
	    cmp = SortCompare(leftPtr, rightPtr, infoPtr);
	    if (cmp > 0) {
		tailPtr->nextPtr = rightPtr;
		tailPtr = rightPtr;
		rightPtr = rightPtr->nextPtr;
	    } else {
		tailPtr->nextPtr = leftPtr;
		tailPtr = leftPtr;
		leftPtr = leftPtr->nextPtr;
	    }
	}
    } else {
	while ((leftPtr != NULL) && (rightPtr != NULL)) {
	    cmp = SortCompare(leftPtr, rightPtr, infoPtr);
	    if (cmp >= 0) {
		if (cmp == 0) {
		    infoPtr->numElements--;
		    leftPtr = leftPtr->nextPtr;
		}
		tailPtr->nextPtr = rightPtr;
		tailPtr = rightPtr;
		rightPtr = rightPtr->nextPtr;
	    } else {
		tailPtr->nextPtr = leftPtr;
		tailPtr = leftPtr;
		leftPtr = leftPtr->nextPtr;
	    }
	}
    }
    if (leftPtr != NULL) {
	tailPtr->nextPtr = leftPtr;
    } else {
	tailPtr->nextPtr = rightPtr;
    }
    return headPtr;
}

/*
 *----------------------------------------------------------------------
 *
 * SortCompare --
 *
 *	This procedure is invoked by MergeLists to determine the proper
 *	ordering between two elements.
 *
 * Results:
 *	A negative results means the the first element comes before the
 *	second, and a positive results means that the second element should
 *	come first. A result of zero means the two elements are equal and it
 *	doesn't matter which comes first.
 *
 * Side effects:
 *	None, unless a user-defined comparison command does something weird.
 *
 *----------------------------------------------------------------------
 */

static int
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

/*
 *----------------------------------------------------------------------
 *
 * DictionaryCompare
 *
 *	This function compares two strings as if they were being used in an
 *	index or card catalog. The case of alphabetic characters is ignored,

```

C Tcl 8.6.18, revision `selected release source; revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdIL.c`, function `Tcl_LsortObjCmd, MergeSort/MergeLists and SortCompare: original prefix/operands, bucket merging and Int result`, lines 3651–4357. Line unit: physical LF only. Original byte range [98374, 118325). Retained reference `selected-producer-2`.

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

/*
 *----------------------------------------------------------------------
 *
 * MergeLists -
 *
 *	This procedure combines two sorted lists of SortElement structures
 *	into a single sorted list.
 *
 * Results:
 *	The unified list of SortElement structures.
 *
 * Side effects:
 *	If infoPtr->unique is set then infoPtr->numElements may be updated.
 *	Possibly others, if a user-defined comparison command does something
 *	weird.
 *
 * Note:
 *	If infoPtr->unique is set, the merge assumes that there are no
 *	"repeated" elements in each of the left and right lists. In that case,
 *	if any element of the left list is equivalent to one in the right list
 *	it is omitted from the merged list.
 *
 *	This simplified mechanism works because of the special way our
 *	MergeSort creates the sublists to be merged and will fail to eliminate
 *	all repeats in the general case where they are already present in
 *	either the left or right list. A general code would need to skip
 *	adjacent initial repeats in the left and right lists before comparing
 *	their initial elements, at each step.
 *
 *----------------------------------------------------------------------
 */

static SortElement *
MergeLists(
    SortElement *leftPtr,	/* First list to be merged; may be NULL. */
    SortElement *rightPtr,	/* Second list to be merged; may be NULL. */
    SortInfo *infoPtr)		/* Information needed by the comparison
				 * operator. */
{
    SortElement *headPtr, *tailPtr;
    int cmp;

    if (leftPtr == NULL) {
	return rightPtr;
    }
    if (rightPtr == NULL) {
	return leftPtr;
    }
    cmp = SortCompare(leftPtr, rightPtr, infoPtr);
    if (cmp > 0 || (cmp == 0 && infoPtr->unique)) {
	if (cmp == 0) {
	    infoPtr->numElements--;
	    leftPtr = leftPtr->nextPtr;
	}
	tailPtr = rightPtr;
	rightPtr = rightPtr->nextPtr;
    } else {
	tailPtr = leftPtr;
	leftPtr = leftPtr->nextPtr;
    }
    headPtr = tailPtr;
    if (!infoPtr->unique) {
	while ((leftPtr != NULL) && (rightPtr != NULL)) {
	    cmp = SortCompare(leftPtr, rightPtr, infoPtr);
	    if (cmp > 0) {
		tailPtr->nextPtr = rightPtr;
		tailPtr = rightPtr;
		rightPtr = rightPtr->nextPtr;
	    } else {
		tailPtr->nextPtr = leftPtr;
		tailPtr = leftPtr;
		leftPtr = leftPtr->nextPtr;
	    }
	}
    } else {
	while ((leftPtr != NULL) && (rightPtr != NULL)) {
	    cmp = SortCompare(leftPtr, rightPtr, infoPtr);
	    if (cmp >= 0) {
		if (cmp == 0) {
		    infoPtr->numElements--;
		    leftPtr = leftPtr->nextPtr;
		}
		tailPtr->nextPtr = rightPtr;
		tailPtr = rightPtr;
		rightPtr = rightPtr->nextPtr;
	    } else {
		tailPtr->nextPtr = leftPtr;
		tailPtr = leftPtr;
		leftPtr = leftPtr->nextPtr;
	    }
	}
    }
    if (leftPtr != NULL) {
	tailPtr->nextPtr = leftPtr;
    } else {
	tailPtr->nextPtr = rightPtr;
    }
    return headPtr;
}

/*
 *----------------------------------------------------------------------
 *
 * SortCompare --
 *
 *	This procedure is invoked by MergeLists to determine the proper
 *	ordering between two elements.
 *
 * Results:
 *	A negative results means the first element comes before the
 *	second, and a positive results means that the second element should
 *	come first. A result of zero means the two elements are equal and it
 *	doesn't matter which comes first.
 *
 * Side effects:
 *	None, unless a user-defined comparison command does something weird.
 *
 *----------------------------------------------------------------------
 */

static int
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

/*
 *----------------------------------------------------------------------
 *
 * DictionaryCompare
 *
 *	This function compares two strings as if they were being used in an
 *	index or card catalog. The case of alphabetic characters is ignored,

```

C Tcl 9.0.4, revision `selected release source; revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdIL.c`, function `Tcl_LsortObjCmd, MergeSort/MergeLists and SortCompare: original prefix/operands, bucket merging and Int result`, lines 4502–5324. Line unit: physical LF only. Original byte range [118534, 141307). Retained reference `selected-producer-3`.

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

/*
 *----------------------------------------------------------------------
 *
 * Tcl_LeditObjCmd --
 *
 *	This procedure is invoked to process the "ledit" Tcl command. See the
 *	user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_LeditObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument values. */
{
    Tcl_Obj *listPtr;		/* Pointer to the list being altered. */
    Tcl_Obj *finalValuePtr;	/* Value finally assigned to the variable. */
    int createdNewObj;
    int result;
    Tcl_Size first;
    Tcl_Size last;
    Tcl_Size listLen;
    Tcl_Size numToDelete;

    if (objc < 4) {
	Tcl_WrongNumArgs(interp, 1, objv,
		"listVar first last ?element ...?");
	return TCL_ERROR;
    }

    listPtr = Tcl_ObjGetVar2(interp, objv[1], NULL, TCL_LEAVE_ERR_MSG);
    if (listPtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * TODO - refactor the index extraction into a common function shared
     * by Tcl_{Lrange,Lreplace,Ledit}ObjCmd
     */

    result = TclListObjLength(interp, listPtr, &listLen);
    if (result != TCL_OK) {
	return result;
    }

    result = TclGetIntForIndexM(interp, objv[2], /*end*/ listLen-1, &first);
    if (result != TCL_OK) {
	return result;
    }

    result = TclGetIntForIndexM(interp, objv[3], /*end*/ listLen-1, &last);
    if (result != TCL_OK) {
	return result;
    }

    if (first < 0) {
	first = 0;
    } else if (first > listLen) {
	first = listLen;
    }

    if (last >= listLen) {
	last = listLen - 1;
    }
    if (first <= last) {
	numToDelete = (size_t)last - (size_t)first + 1; /* See [3d3124d01d] */
    } else {
	numToDelete = 0;
    }

    if (Tcl_IsShared(listPtr)) {
	listPtr = TclListObjCopy(NULL, listPtr);
	createdNewObj = 1;
    } else {
	createdNewObj = 0;
    }

    result =
	Tcl_ListObjReplace(interp, listPtr, first, numToDelete, objc - 4, objv + 4);
    if (result != TCL_OK) {
	if (createdNewObj) {
	    Tcl_DecrRefCount(listPtr);
	}
	return result;
    }

    /*
     * Tcl_ObjSetVar2 may return a value different from listPtr in the
     * presence of traces etc.
     */
    finalValuePtr =
	Tcl_ObjSetVar2(interp, objv[1], NULL, listPtr, TCL_LEAVE_ERR_MSG);
    if (finalValuePtr == NULL) {
	return TCL_ERROR;
    }

    Tcl_SetObjResult(interp, finalValuePtr);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * MergeLists -
 *
 *	This procedure combines two sorted lists of SortElement structures
 *	into a single sorted list.
 *
 * Results:
 *	The unified list of SortElement structures.
 *
 * Side effects:
 *	If infoPtr->unique is set then infoPtr->numElements may be updated.
 *	Possibly others, if a user-defined comparison command does something
 *	weird.
 *
 * Note:
 *	If infoPtr->unique is set, the merge assumes that there are no
 *	"repeated" elements in each of the left and right lists. In that case,
 *	if any element of the left list is equivalent to one in the right list
 *	it is omitted from the merged list.
 *
 *	This simplified mechanism works because of the special way our
 *	MergeSort creates the sublists to be merged and will fail to eliminate
 *	all repeats in the general case where they are already present in
 *	either the left or right list. A general code would need to skip
 *	adjacent initial repeats in the left and right lists before comparing
 *	their initial elements, at each step.
 *
 *----------------------------------------------------------------------
 */

static SortElement *
MergeLists(
    SortElement *leftPtr,	/* First list to be merged; may be NULL. */
    SortElement *rightPtr,	/* Second list to be merged; may be NULL. */
    SortInfo *infoPtr)		/* Information needed by the comparison
				 * operator. */
{
    SortElement *headPtr, *tailPtr;
    int cmp;

    if (leftPtr == NULL) {
	return rightPtr;
    }
    if (rightPtr == NULL) {
	return leftPtr;
    }
    cmp = SortCompare(leftPtr, rightPtr, infoPtr);
    if (cmp > 0 || (cmp == 0 && infoPtr->unique)) {
	if (cmp == 0) {
	    infoPtr->numElements--;
	    leftPtr = leftPtr->nextPtr;
	}
	tailPtr = rightPtr;
	rightPtr = rightPtr->nextPtr;
    } else {
	tailPtr = leftPtr;
	leftPtr = leftPtr->nextPtr;
    }
    headPtr = tailPtr;
    if (!infoPtr->unique) {
	while ((leftPtr != NULL) && (rightPtr != NULL)) {
	    cmp = SortCompare(leftPtr, rightPtr, infoPtr);
	    if (cmp > 0) {
		tailPtr->nextPtr = rightPtr;
		tailPtr = rightPtr;
		rightPtr = rightPtr->nextPtr;
	    } else {
		tailPtr->nextPtr = leftPtr;
		tailPtr = leftPtr;
		leftPtr = leftPtr->nextPtr;
	    }
	}
    } else {
	while ((leftPtr != NULL) && (rightPtr != NULL)) {
	    cmp = SortCompare(leftPtr, rightPtr, infoPtr);
	    if (cmp >= 0) {
		if (cmp == 0) {
		    infoPtr->numElements--;
		    leftPtr = leftPtr->nextPtr;
		}
		tailPtr->nextPtr = rightPtr;
		tailPtr = rightPtr;
		rightPtr = rightPtr->nextPtr;
	    } else {
		tailPtr->nextPtr = leftPtr;
		tailPtr = leftPtr;
		leftPtr = leftPtr->nextPtr;
	    }
	}
    }
    if (leftPtr != NULL) {
	tailPtr->nextPtr = leftPtr;
    } else {
	tailPtr->nextPtr = rightPtr;
    }
    return headPtr;
}

/*
 *----------------------------------------------------------------------
 *
 * SortCompare --
 *
 *	This procedure is invoked by MergeLists to determine the proper
 *	ordering between two elements.
 *
 * Results:
 *	A negative results means the first element comes before the
 *	second, and a positive results means that the second element should
 *	come first. A result of zero means the two elements are equal and it
 *	doesn't matter which comes first.
 *
 * Side effects:
 *	None, unless a user-defined comparison command does something weird.
 *
 *----------------------------------------------------------------------
 */

static int
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

/*
 *----------------------------------------------------------------------
 *
 * DictionaryCompare
 *
 *	This function compares two strings as if they were being used in an
 *	index or card catalog. The case of alphabetic characters is ignored,

```

C Tcl 9.1.0, revision `selected release source; revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdIL.c`, function `Tcl_LsortObjCmd, MergeSort/MergeLists and SortCompare: original prefix/operands, bucket merging and Int result`, lines 4349–5173. Line unit: physical LF only. Original byte range [115160, 137975). Retained reference `selected-producer-4`.

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

/*
 *----------------------------------------------------------------------
 *
 * Tcl_LeditObjCmd --
 *
 *	This procedure is invoked to process the "ledit" Tcl command. See the
 *	user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_LeditObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument values. */
{
    Tcl_Obj *listPtr;		/* Pointer to the list being altered. */
    Tcl_Obj *finalValuePtr;	/* Value finally assigned to the variable. */
    Tcl_Size first;
    Tcl_Size last;
    Tcl_Size listLen;
    Tcl_Size numToDelete;
    int result;
    bool createdNewObj;

    if (objc < 4) {
	Tcl_WrongNumArgs(interp, 1, objv,
		"listVar first last ?element ...?");
	return TCL_ERROR;
    }

    listPtr = Tcl_ObjGetVar2(interp, objv[1], NULL, TCL_LEAVE_ERR_MSG);
    if (listPtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * TODO - refactor the index extraction into a common function shared
     * by Tcl_{Lrange,Lreplace,Ledit}ObjCmd
     */

    result = TclListObjLength(interp, listPtr, &listLen);
    if (result != TCL_OK) {
	return result;
    }

    result = TclGetIntForIndexM(interp, objv[2], /*end*/ listLen-1, &first);
    if (result != TCL_OK) {
	return result;
    }

    result = TclGetIntForIndexM(interp, objv[3], /*end*/ listLen-1, &last);
    if (result != TCL_OK) {
	return result;
    }

    if (first < 0) {
	first = 0;
    } else if (first > listLen) {
	first = listLen;
    }

    if (last >= listLen) {
	last = listLen - 1;
    }
    if (first <= last) {
	numToDelete = (size_t)last - (size_t)first + 1; /* See [3d3124d01d] */
    } else {
	numToDelete = 0;
    }

    if (Tcl_IsShared(listPtr)) {
	listPtr = TclListObjCopy(NULL, listPtr);
	createdNewObj = true;
    } else {
	createdNewObj = false;
    }

    result =
	Tcl_ListObjReplace(interp, listPtr, first, numToDelete, objc - 4, objv + 4);
    if (result != TCL_OK) {
	if (createdNewObj) {
	    Tcl_DecrRefCount(listPtr);
	}
	return result;
    }

    /*
     * Tcl_ObjSetVar2 may return a value different from listPtr in the
     * presence of traces etc.
     */
    finalValuePtr =
	Tcl_ObjSetVar2(interp, objv[1], NULL, listPtr, TCL_LEAVE_ERR_MSG);
    if (finalValuePtr == NULL) {
	return TCL_ERROR;
    }

    Tcl_SetObjResult(interp, finalValuePtr);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * MergeLists -
 *
 *	This procedure combines two sorted lists of SortElement structures
 *	into a single sorted list.
 *
 * Results:
 *	The unified list of SortElement structures.
 *
 * Side effects:
 *	If infoPtr->unique is set then infoPtr->numElements may be updated.
 *	Possibly others, if a user-defined comparison command does something
 *	weird.
 *
 * Note:
 *	If infoPtr->unique is set, the merge assumes that there are no
 *	"repeated" elements in each of the left and right lists. In that case,
 *	if any element of the left list is equivalent to one in the right list
 *	it is omitted from the merged list.
 *
 *	This simplified mechanism works because of the special way our
 *	MergeSort creates the sublists to be merged and will fail to eliminate
 *	all repeats in the general case where they are already present in
 *	either the left or right list. A general code would need to skip
 *	adjacent initial repeats in the left and right lists before comparing
 *	their initial elements, at each step.
 *
 *----------------------------------------------------------------------
 */

static SortElement *
MergeLists(
    SortElement *leftPtr,	/* First list to be merged; may be NULL. */
    SortElement *rightPtr,	/* Second list to be merged; may be NULL. */
    SortInfo *infoPtr)		/* Information needed by the comparison
				 * operator. */
{
    SortElement *headPtr, *tailPtr;
    int cmp;

    if (leftPtr == NULL) {
	return rightPtr;
    }
    if (rightPtr == NULL) {
	return leftPtr;
    }
    cmp = SortCompare(leftPtr, rightPtr, infoPtr);
    if (cmp > 0 || (cmp == 0 && infoPtr->unique)) {
	if (cmp == 0) {
	    infoPtr->numElements--;
	    leftPtr = leftPtr->nextPtr;
	}
	tailPtr = rightPtr;
	rightPtr = rightPtr->nextPtr;
    } else {
	tailPtr = leftPtr;
	leftPtr = leftPtr->nextPtr;
    }
    headPtr = tailPtr;
    if (!infoPtr->unique) {
	while ((leftPtr != NULL) && (rightPtr != NULL)) {
	    cmp = SortCompare(leftPtr, rightPtr, infoPtr);
	    if (cmp > 0) {
		tailPtr->nextPtr = rightPtr;
		tailPtr = rightPtr;
		rightPtr = rightPtr->nextPtr;
	    } else {
		tailPtr->nextPtr = leftPtr;
		tailPtr = leftPtr;
		leftPtr = leftPtr->nextPtr;
	    }
	}
    } else {
	while ((leftPtr != NULL) && (rightPtr != NULL)) {
	    cmp = SortCompare(leftPtr, rightPtr, infoPtr);
	    if (cmp >= 0) {
		if (cmp == 0) {
		    infoPtr->numElements--;
		    leftPtr = leftPtr->nextPtr;
		}
		tailPtr->nextPtr = rightPtr;
		tailPtr = rightPtr;
		rightPtr = rightPtr->nextPtr;
	    } else {
		tailPtr->nextPtr = leftPtr;
		tailPtr = leftPtr;
		leftPtr = leftPtr->nextPtr;
	    }
	}
    }
    if (leftPtr != NULL) {
	tailPtr->nextPtr = leftPtr;
    } else {
	tailPtr->nextPtr = rightPtr;
    }
    return headPtr;
}

/*
 *----------------------------------------------------------------------
 *
 * SortCompare --
 *
 *	This procedure is invoked by MergeLists to determine the proper
 *	ordering between two elements.
 *
 * Results:
 *	A negative results means the first element comes before the
 *	second, and a positive results means that the second element should
 *	come first. A result of zero means the two elements are equal and it
 *	doesn't matter which comes first.
 *
 * Side effects:
 *	None, unless a user-defined comparison command does something weird.
 *
 *----------------------------------------------------------------------
 */

static int
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

/*
 *----------------------------------------------------------------------
 *
 * DictionaryCompare
 *
 *	This function compares two strings as if they were being used in an
 *	index or card catalog. The case of alphabetic characters is ignored,

```

Jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `ListSortIndexHelper`, lines 7053–7062. Line unit: physical LF only. Original byte range [217634, 218070). Retained reference `selected-producer-5`.

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

Jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `ListSortCommand, ListRemoveDuplicates and ListSortElements`, lines 7150–7265. Line unit: physical LF only. Original byte range [221014, 224536). Retained reference `selected-producer-6`.

```text
    Jim_Obj *compare_script;
    int rc;

    jim_wide ret = 0;

    /* This must be a valid list */
    compare_script = Jim_DuplicateObj(sort_info->interp, sort_info->command);
    Jim_ListAppendElement(sort_info->interp, compare_script, *lhsObj);
    Jim_ListAppendElement(sort_info->interp, compare_script, *rhsObj);

    rc = Jim_EvalObj(sort_info->interp, compare_script);

    if (rc != JIM_OK || Jim_GetWide(sort_info->interp, Jim_GetResult(sort_info->interp), &ret) != JIM_OK) {
        longjmp(sort_info->jmpbuf, rc);
    }

    return JimSign(ret) * sort_info->order;
}

/* Remove duplicate elements from the (sorted) list in-place, according to the
 * comparison function, comp.
 *
 * Note that the last unique value is kept, not the first
 */
static void ListRemoveDuplicates(Jim_Obj *listObjPtr, int (*comp)(Jim_Obj **lhs, Jim_Obj **rhs))
{
    int src;
    int dst = 0;
    Jim_Obj **ele = listObjPtr->internalRep.listValue.ele;

    for (src = 1; src < listObjPtr->internalRep.listValue.len; src++) {
        if (comp(&ele[dst], &ele[src]) == 0) {
            /* Match, so replace the dest with the current source */
            Jim_DecrRefCount(sort_info->interp, ele[dst]);
        }
        else {
            /* No match, so keep the current source and move to the next destination */
            dst++;
        }
        ele[dst] = ele[src];
    }

    /* At end of list, keep the final element unless all elements were kept */
    dst++;
    if (dst < listObjPtr->internalRep.listValue.len) {
        ele[dst] = ele[src];
    }

    /* Set the new length */
    listObjPtr->internalRep.listValue.len = dst;
}

/* Sort a list *in place*. MUST be called with a non-shared list. */
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

Jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_LsortCoreCommand`, lines 13831–13970. Line unit: physical LF only. Original byte range [436968, 441634). Retained reference `selected-producer-7`.

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

## Consumer bindings

- [rust/tcl-cmd-core/src/lsort.rs](../../../../rust/tcl-cmd-core/src/lsort.rs), `GroupingFailure`: Typed group size and leading-coordinate diagnostics at the actual branch; no message reparsing.
- [rust/tcl-vm/src/cmd_list/native_original_group_command_controls.rs](../../../../rust/tcl-vm/src/cmd_list/native_original_group_command_controls.rs), `original_cli_group_fields_match_native656_source_capture` (linked): Exact66 fixed capture-prefix script/code/result/errorCode/errorInfo/presence/count fields match actual CLI656 streams. argv is separately initialised; native startup/private getters, print helper and callback/libc chronology are excluded.
- [runtime/rust/src/cmd_list/native_original_group_command_controls.rs](../../../../runtime/rust/src/cmd_list/native_original_group_command_controls.rs), `original_cli_group_fields_match_native656_source_capture` (linked): Exact66 fixed capture-prefix script/code/result/errorCode/errorInfo/presence/count fields match actual CLI656 streams. argv is separately initialised; native startup/private getters, print helper and callback/libc chronology are excluded.

## Evidence

- [runtime/rust/tests/data](../../../../runtime/rust/tests/data)
- [runtime/rust/tests/data/native_original_list_group_cli656/8.4.20](../../../../runtime/rust/tests/data/native_original_list_group_cli656/8.4.20)
- [runtime/rust/tests/data/native_original_list_group_cli656/8.5.19](../../../../runtime/rust/tests/data/native_original_list_group_cli656/8.5.19)
- [runtime/rust/tests/data/native_original_list_group_cli656/8.6.18](../../../../runtime/rust/tests/data/native_original_list_group_cli656/8.6.18)
- [runtime/rust/tests/data/native_original_list_group_cli656/9.0.4](../../../../runtime/rust/tests/data/native_original_list_group_cli656/9.0.4)
- [runtime/rust/tests/data/native_original_list_group_cli656/9.1.0](../../../../runtime/rust/tests/data/native_original_list_group_cli656/9.1.0)
- [runtime/rust/tests/data/native_original_list_group_cli656/Jim](../../../../runtime/rust/tests/data/native_original_list_group_cli656/Jim)
- [runtime/rust/tests/data/native_original_list_group_command651](../../../../runtime/rust/tests/data/native_original_list_group_command651)
