# naming.dictionary.original-with-write-error-object-lifetime

Kind: `native-observation`

## Problem statement

The compiled-with write-error cleanup has a distinct dictionary allocation/release owner. Captured later caller fields cannot establish defined storage; a precise conclusion requires live selection/refcount checks, reached free evidence and separately preserved later exit/signal outcomes.

## Question

For the unchanged original compiled-with write-error source on independently pinned current C8.6/C9.0/C9.1 builds, does TclDictWithFinish allocate a refcount0 header, leave the Var selecting a defined scalar after failed set, and reach final free before later reporting; how do plain-with and independent update controls differ?

## Conclusion

On each independently built C8.6.18/C9.0.4/C9.1.0 observer, original compiled-with-write-error case19 reaches TclDictWithFinish with refcount=0 and allocated=1. Immediately after failed TclPtrSetVarIdx, the Var still selects the same defined scalar (same_cell=1, defined_scalar=1), with no watched free yet. The unchanged allocated-dictionary decrement then reaches watched TclFreeObj: final_free=1 before the body-return integer report. Every later original guest catch/result/diagnostic/read-status/outer field is a captured post-free process observation, not a defined storage or completion/read/effect guarantee. C8.6 baseline and observer case19 exit0 with byte-identical whole stdout; C9.0/C9.1 baseline and observer case19 retain that stdout but later terminate by signal4 with their original allocation diagnostics. Plain with case10 records failed=0 and final_free=0 at body return, then a later cleanup free; all three providers exit0. Independent update-error case9 records entered=0 for this With observer: C8.6 exits0, C9.0/C9.1 later signal4, without a With selection/lifetime conclusion. The39 exact closed operations comprise18 build/compile steps and21 processes (18 case runs plus3 independently successful version queries);31 exit0 and8 signal4. No successful aggregate result, generic dictionary lifetime, safe freed-byte reproduction, old executable private-header identity, Native frame/handler/compiler permission or Rust pass follows.

Two current VM software bindings retain the unchanged329 observations with326 whole result comparisons and three bounded captured post-free diagnostic/read-status windows. The independent With host-storage control checks the exact405 integer/raw C8.6 witness and plain/update controls, then checks its own retained live BASE dictionary header/member under the same unchanged guest. That software storage policy is independent of native freed contents.

## Scope

The exact original probe and compiled-with-write-error guest from Native329 are unchanged; each fresh process receives only original case index19, plain-with10 or independent update-error9. Original counted Tcl_EvalObjEx uses Tcl_CreateInterp without Tcl_Init; this does not measure full-distribution initialisation. Original SDK sources/headers/configured Makefiles/archives and baseline/version executables remain separate from independently compiled observed tclDictObj.o/tclObj.o, patched archive and observer executable. The observer reads the live actual refcount and allocdict before set, then compares the Var pointer and scalar/undefined flags after set failure; it does not dereference a released header, retain the watched header, or modify Tcl ownership. The free-entry hook increments an integer and clears the watch before ordinary freeing. The probe reports saved integers before source-object release and interpreter teardown. Complete requests, original/observed sources and patches, build templates, original/observer objects/archives/ELFs, command receipts, versions, raw streams and signal diagnostics are retained byte-for-byte. Source inspection explains this exact current observer path and does not certify the private state of the older archived329 executable. C8.4, C8.5, Jim and BIG-IP are not tested by this request. UpdateEnd398/399 and their watched counters are independent questions; no result is transferred between cleanup owners.

These current definitions have no fresh Rust assertion receipt. The C9 baseline/observer signals remain original capture outcomes without invented software expectations. Retaining outer/diagnostic/caller-read fields does not turn post-free output into defined Native completion/storage/read/effect or another executable's private lifetime.

## Provider answers

### tcl8.4, tcl8.5, jim, bigip

Status: `not-tested`.

No process or WithFinish lifetime observation for this provider is executed by request405.

### tcl8.6

Status: `observed`. Channel: Unchanged original counted C API source Tcl_CreateInterp/Tcl_EvalObjEx without Tcl_Init; distinct live integer observer counters, exact case processes and public streams. Version: tcl8.6 8.6.18.

Case19 live pre-set refcount0/allocated1; failed set retains same defined scalar with free0, then watched free1 before body-return report. Baseline/observer case19 exit0 and whole stdout match; all later fields are post-free process observations. Plain-with10 succeeds with free0 at body return and later cleanupfree1. Independent update9 does not enter With; it exits0.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Unchanged original counted C API source Tcl_CreateInterp/Tcl_EvalObjEx without Tcl_Init; distinct live integer observer counters, exact case processes and public streams. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Case19 live pre-set refcount0/allocated1; failed set retains same defined scalar with free0, then watched free1 before body-return report. Baseline/observer case19 whole stdout match but both later terminate by signal4; raw allocation diagnostics remain exact, not a successful completion. Plain-with10 succeeds with free0 at body return and later cleanupfree1. Independent update9 does not enter With; it later signals4.

## Source inspection

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/8.6.18/original-tclDictObj.c`, function `TclDictWithFinish original dictionary selection and allocation`, lines 3655–3764. Retained reference `native_dict_with_write_error_lifetime405-request-8.6.18-original-tclDictObj.c`.

```text
TclDictWithFinish(
    Tcl_Interp *interp,		/* Command interpreter in which variable
				 * exists. Used for state management, traces
				 * and error reporting. */
    Var *varPtr,		/* Reference to the variable holding the
				 * dictionary. */
    Var *arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Name of an array (if part2 is non-NULL) or
				 * the name of a variable. NULL if the 'index'
				 * parameter is >= 0 */
    Tcl_Obj *part2Ptr,		/* If non-NULL, gives the name of an element
				 * in the array part1. */
    int index,			/* Index into the local variable table of the
				 * variable, or -1. Only used when part1Ptr is
				 * NULL. */
    int pathc,			/* The number of elements in the path into the
				 * dictionary. */
    Tcl_Obj *const pathv[],	/* The elements of the path to the subdict. */
    Tcl_Obj *keysPtr)		/* List of keys to be synchronized. This is
				 * the result value from TclDictWithInit. */
{
    Tcl_Obj *dictPtr, *leafPtr, *valPtr;
    int i, allocdict, keyc;
    Tcl_Obj **keyv;

    /*
     * If the dictionary variable doesn't exist, drop everything silently.
     */

    dictPtr = TclPtrGetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    TCL_LEAVE_ERR_MSG, index);
    if (dictPtr == NULL) {
	return TCL_OK;
    }

    /*
     * Double-check that it is still a dictionary.
     */

    if (Tcl_DictObjSize(interp, dictPtr, &i) != TCL_OK) {
	return TCL_ERROR;
    }

    if (Tcl_IsShared(dictPtr)) {
	dictPtr = Tcl_DuplicateObj(dictPtr);
	allocdict = 1;
    } else {
	allocdict = 0;
    }

    if (pathc > 0) {
	/*
	 * Want to get to the dictionary which we will update; need to do
	 * prepare-for-update unsharing along the path *but* avoid generating
	 * an error on a non-extant path (we'll treat that the same as a
	 * non-extant variable. Luckily, the unsharing operation isn't
	 * deeply damaging if we don't go on to update; it's just less than
	 * perfectly efficient (but no memory should be leaked).
	 */

	leafPtr = TclTraceDictPath(interp, dictPtr, pathc, pathv,
		DICT_PATH_EXISTS | DICT_PATH_UPDATE);
	if (leafPtr == NULL) {
	    if (allocdict) {
		TclDecrRefCount(dictPtr);
	    }
	    return TCL_ERROR;
	}
	if (leafPtr == DICT_PATH_NON_EXISTENT) {
	    if (allocdict) {
		TclDecrRefCount(dictPtr);
	    }
	    return TCL_OK;
	}
    } else {
	leafPtr = dictPtr;
    }

    /*
     * Now process our updates on the leaf dictionary.
     */

    TclListObjGetElements(NULL, keysPtr, &keyc, &keyv);
    for (i=0 ; i<keyc ; i++) {
	valPtr = Tcl_ObjGetVar2(interp, keyv[i], NULL, 0);
	if (valPtr == NULL) {
	    Tcl_DictObjRemove(NULL, leafPtr, keyv[i]);
	} else if (leafPtr == valPtr) {
	    /*
	     * Someone is messing us around, trying to build a recursive
	     * structure. [Bug 1786481]
	     */

	    Tcl_DictObjPut(NULL, leafPtr, keyv[i], Tcl_DuplicateObj(valPtr));
	} else {
	    Tcl_DictObjPut(NULL, leafPtr, keyv[i], valPtr);
	}
    }

    /*
     * Ensure that none of the dictionaries in the chain still have a string
     * rep.
     */

    if (pathc > 0) {
	InvalidateDictChain(leafPtr);
    }

```

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/8.6.18/original-tclDictObj.c`, function `TclDictWithFinish original failed writeback cleanup`, lines 3765–3785. Retained reference `native_dict_with_write_error_lifetime405-request-8.6.18-original-tclDictObj.c`.

```text
    /*
     * Write back the outermost dictionary to the variable.
     */

    if (TclPtrSetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    dictPtr, TCL_LEAVE_ERR_MSG, index) == NULL) {
	if (allocdict) {
	    TclDecrRefCount(dictPtr);
	}
	return TCL_ERROR;
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * TclInitDictCmd --
 *
 *	This function is create the "dict" Tcl command. See the user
 *	documentation for details on what it does, and TIP#111 for the formal

```

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/8.6.18/observed-tclDictObj.c`, function `TclDictWithFinish live pre-set refcount/allocdict and post-failure Var-only observer`, lines 3773–3792. Retained reference `native_dict_with_write_error_lifetime405-request-8.6.18-observed-tclDictObj.c`.

```text
    tclLspWithEntered++;
    tclLspWithBeforeRefcount = dictPtr->refCount;
    tclLspWithAllocated = allocdict;
    tclLspWithWatch = dictPtr;
    fprintf(stderr,"WITH_BEFORE_SET|refcount=%d|allocated=%d\n",tclLspWithBeforeRefcount,tclLspWithAllocated);
    if (TclPtrSetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    dictPtr, TCL_LEAVE_ERR_MSG, index) == NULL) {
        tclLspWithFailed++;
        tclLspWithSameCell = varPtr->value.objPtr == dictPtr;
        tclLspWithDefinedScalar = TclIsVarScalar(varPtr) && !TclIsVarUndefined(varPtr);
        fprintf(stderr,"WITH_SET_ERROR|same_cell=%d|defined_scalar=%d|final_free=%d\n",tclLspWithSameCell,tclLspWithDefinedScalar,tclLspWithFinalFree);
	if (allocdict) {
	    TclDecrRefCount(dictPtr);
	}
	return TCL_ERROR;
    }
    return TCL_OK;
}

/*

```

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/8.6.18/observed-tclObj.c`, function `TclFreeObj watched free-entry integer counter; debug/non-debug branch`, lines 1295–1308. Retained reference `native_dict_with_write_error_lifetime405-request-8.6.18-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)	/* The object to be freed. */
{
    if (objPtr == tclLspWithWatch) {
        tclLspWithFinalFree++;
        tclLspWithWatch = NULL;
        fprintf(stderr,"WITH_FINAL_FREE|count=%d\n",tclLspWithFinalFree);
    }
    const Tcl_ObjType *typePtr = objPtr->typePtr;

    /*
     * This macro declares a variable, so must come here...
     */

```

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/8.6.18/observed-tclObj.c`, function `TclFreeObj watched free-entry integer counter; debug/non-debug branch`, lines 1425–1438. Retained reference `native_dict_with_write_error_lifetime405-request-8.6.18-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)	/* The object to be freed. */
{
    if (objPtr == tclLspWithWatch) {
        tclLspWithFinalFree++;
        tclLspWithWatch = NULL;
        fprintf(stderr,"WITH_FINAL_FREE|count=%d\n",tclLspWithFinalFree);
    }
    /*
     * Invalidate the string rep first so we can use the bytes value for our
     * pointer chain, and signal an obj deletion (as opposed to shimmering)
     * with 'length == -1'.
     */

```

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/observed-probe.c`, function `Unchanged source evaluation with integer-only body-return report before public report/source release/teardown`, lines 124–124. Retained reference `native_dict_with_write_error_lifetime405-request-observed-probe.c`.

```text
static void probe_bytes(const char*id,const char*src,int source_len){caseid=id;pathid="fixed-grammar";active=create();Obj*o=NEW(active,src,source_len);INC(o);int code=SCRIPT(active,o);fprintf(stderr,"WITH_BODY_RETURN|entered=%d|before_refcount=%d|allocated=%d|failed=%d|same_cell=%d|defined_scalar=%d|final_free=%d\n",tclLspWithEntered,tclLspWithBeforeRefcount,tclLspWithAllocated,tclLspWithFailed,tclLspWithSameCell,tclLspWithDefinedScalar,tclLspWithFinalFree);report("native-result",code);DEC(active,o);DESTROY(active);}

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.0.4/original-tclDictObj.c`, function `TclDictWithFinish original dictionary selection and allocation`, lines 3842–3951. Retained reference `native_dict_with_write_error_lifetime405-request-9.0.4-original-tclDictObj.c`.

```text
TclDictWithFinish(
    Tcl_Interp *interp,		/* Command interpreter in which variable
				 * exists. Used for state management, traces
				 * and error reporting. */
    Var *varPtr,		/* Reference to the variable holding the
				 * dictionary. */
    Var *arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Name of an array (if part2 is non-NULL) or
				 * the name of a variable. NULL if the 'index'
				 * parameter is >= 0 */
    Tcl_Obj *part2Ptr,		/* If non-NULL, gives the name of an element
				 * in the array part1. */
    int index,			/* Index into the local variable table of the
				 * variable, or -1. Only used when part1Ptr is
				 * NULL. */
    int pathc,			/* The number of elements in the path into the
				 * dictionary. */
    Tcl_Obj *const pathv[],	/* The elements of the path to the subdict. */
    Tcl_Obj *keysPtr)		/* List of keys to be synchronized. This is
				 * the result value from TclDictWithInit. */
{
    Tcl_Obj *dictPtr, *leafPtr, *valPtr;
    Tcl_Size i, allocdict, keyc;
    Tcl_Obj **keyv;

    /*
     * If the dictionary variable doesn't exist, drop everything silently.
     */

    dictPtr = TclPtrGetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    TCL_LEAVE_ERR_MSG, index);
    if (dictPtr == NULL) {
	return TCL_OK;
    }

    /*
     * Double-check that it is still a dictionary.
     */

    if (Tcl_DictObjSize(interp, dictPtr, &i) != TCL_OK) {
	return TCL_ERROR;
    }

    if (Tcl_IsShared(dictPtr)) {
	dictPtr = Tcl_DuplicateObj(dictPtr);
	allocdict = 1;
    } else {
	allocdict = 0;
    }

    if (pathc > 0) {
	/*
	 * Want to get to the dictionary which we will update; need to do
	 * prepare-for-update unsharing along the path *but* avoid generating
	 * an error on a non-extant path (we'll treat that the same as a
	 * non-extant variable. Luckily, the unsharing operation isn't
	 * deeply damaging if we don't go on to update; it's just less than
	 * perfectly efficient (but no memory should be leaked).
	 */

	leafPtr = TclTraceDictPath(interp, dictPtr, pathc, pathv,
		DICT_PATH_EXISTS | DICT_PATH_UPDATE);
	if (leafPtr == NULL) {
	    if (allocdict) {
		TclDecrRefCount(dictPtr);
	    }
	    return TCL_ERROR;
	}
	if (leafPtr == DICT_PATH_NON_EXISTENT) {
	    if (allocdict) {
		TclDecrRefCount(dictPtr);
	    }
	    return TCL_OK;
	}
    } else {
	leafPtr = dictPtr;
    }

    /*
     * Now process our updates on the leaf dictionary.
     */

    TclListObjGetElements(NULL, keysPtr, &keyc, &keyv);
    for (i=0 ; i<keyc ; i++) {
	valPtr = Tcl_ObjGetVar2(interp, keyv[i], NULL, 0);
	if (valPtr == NULL) {
	    Tcl_DictObjRemove(NULL, leafPtr, keyv[i]);
	} else if (leafPtr == valPtr) {
	    /*
	     * Someone is messing us around, trying to build a recursive
	     * structure. [Bug 1786481]
	     */

	    Tcl_DictObjPut(NULL, leafPtr, keyv[i], Tcl_DuplicateObj(valPtr));
	} else {
	    Tcl_DictObjPut(NULL, leafPtr, keyv[i], valPtr);
	}
    }

    /*
     * Ensure that none of the dictionaries in the chain still have a string
     * rep.
     */

    if (pathc > 0) {
	InvalidateDictChain(leafPtr);
    }

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.0.4/original-tclDictObj.c`, function `TclDictWithFinish original failed writeback cleanup`, lines 3952–3972. Retained reference `native_dict_with_write_error_lifetime405-request-9.0.4-original-tclDictObj.c`.

```text
    /*
     * Write back the outermost dictionary to the variable.
     */

    if (TclPtrSetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    dictPtr, TCL_LEAVE_ERR_MSG, index) == NULL) {
	if (allocdict) {
	    TclDecrRefCount(dictPtr);
	}
	return TCL_ERROR;
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * TclInitDictCmd --
 *
 *	This function is create the "dict" Tcl command. See the user
 *	documentation for details on what it does, and TIP#111 for the formal

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.0.4/observed-tclDictObj.c`, function `TclDictWithFinish live pre-set refcount/allocdict and post-failure Var-only observer`, lines 3960–3979. Retained reference `native_dict_with_write_error_lifetime405-request-9.0.4-observed-tclDictObj.c`.

```text
    tclLspWithEntered++;
    tclLspWithBeforeRefcount = dictPtr->refCount;
    tclLspWithAllocated = allocdict;
    tclLspWithWatch = dictPtr;
    fprintf(stderr,"WITH_BEFORE_SET|refcount=%d|allocated=%d\n",tclLspWithBeforeRefcount,tclLspWithAllocated);
    if (TclPtrSetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    dictPtr, TCL_LEAVE_ERR_MSG, index) == NULL) {
        tclLspWithFailed++;
        tclLspWithSameCell = varPtr->value.objPtr == dictPtr;
        tclLspWithDefinedScalar = TclIsVarScalar(varPtr) && !TclIsVarUndefined(varPtr);
        fprintf(stderr,"WITH_SET_ERROR|same_cell=%d|defined_scalar=%d|final_free=%d\n",tclLspWithSameCell,tclLspWithDefinedScalar,tclLspWithFinalFree);
	if (allocdict) {
	    TclDecrRefCount(dictPtr);
	}
	return TCL_ERROR;
    }
    return TCL_OK;
}

/*

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.0.4/observed-tclObj.c`, function `TclFreeObj watched free-entry integer counter; debug/non-debug branch`, lines 1271–1284. Retained reference `native_dict_with_write_error_lifetime405-request-9.0.4-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)	/* The object to be freed. */
{
    if (objPtr == tclLspWithWatch) {
        tclLspWithFinalFree++;
        tclLspWithWatch = NULL;
        fprintf(stderr,"WITH_FINAL_FREE|count=%d\n",tclLspWithFinalFree);
    }
    const Tcl_ObjType *typePtr = objPtr->typePtr;

    /*
     * This macro declares a variable, so must come here...
     */

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.0.4/observed-tclObj.c`, function `TclFreeObj watched free-entry integer counter; debug/non-debug branch`, lines 1401–1414. Retained reference `native_dict_with_write_error_lifetime405-request-9.0.4-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)	/* The object to be freed. */
{
    if (objPtr == tclLspWithWatch) {
        tclLspWithFinalFree++;
        tclLspWithWatch = NULL;
        fprintf(stderr,"WITH_FINAL_FREE|count=%d\n",tclLspWithFinalFree);
    }
    /*
     * Invalidate the string rep first so we can use the bytes value for our
     * pointer chain, and signal an obj deletion (as opposed to shimmering)
     * with 'length == -1'.
     */

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/observed-probe.c`, function `Unchanged source evaluation with integer-only body-return report before public report/source release/teardown`, lines 124–124. Retained reference `native_dict_with_write_error_lifetime405-request-observed-probe.c`.

```text
static void probe_bytes(const char*id,const char*src,int source_len){caseid=id;pathid="fixed-grammar";active=create();Obj*o=NEW(active,src,source_len);INC(o);int code=SCRIPT(active,o);fprintf(stderr,"WITH_BODY_RETURN|entered=%d|before_refcount=%d|allocated=%d|failed=%d|same_cell=%d|defined_scalar=%d|final_free=%d\n",tclLspWithEntered,tclLspWithBeforeRefcount,tclLspWithAllocated,tclLspWithFailed,tclLspWithSameCell,tclLspWithDefinedScalar,tclLspWithFinalFree);report("native-result",code);DEC(active,o);DESTROY(active);}

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.1.0/original-tclDictObj.c`, function `TclDictWithFinish original dictionary selection and allocation`, lines 3845–3954. Retained reference `native_dict_with_write_error_lifetime405-request-9.1.0-original-tclDictObj.c`.

```text
TclDictWithFinish(
    Tcl_Interp *interp,		/* Command interpreter in which variable
				 * exists. Used for state management, traces
				 * and error reporting. */
    Var *varPtr,		/* Reference to the variable holding the
				 * dictionary. */
    Var *arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Name of an array (if part2 is non-NULL) or
				 * the name of a variable. NULL if the 'index'
				 * parameter is >= 0 */
    Tcl_Obj *part2Ptr,		/* If non-NULL, gives the name of an element
				 * in the array part1. */
    Tcl_Size index,		/* Index into the local variable table of the
				 * variable, or -1. Only used when part1Ptr is
				 * NULL. */
    Tcl_Size pathc,		/* The number of elements in the path into the
				 * dictionary. */
    Tcl_Obj *const pathv[],	/* The elements of the path to the subdict. */
    Tcl_Obj *keysPtr)		/* List of keys to be synchronized. This is
				 * the result value from TclDictWithInit. */
{
    Tcl_Obj *dictPtr, *leafPtr, *valPtr;
    Tcl_Size i, allocdict, keyc;
    Tcl_Obj **keyv;

    /*
     * If the dictionary variable doesn't exist, drop everything silently.
     */

    dictPtr = TclPtrGetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    TCL_LEAVE_ERR_MSG, index);
    if (dictPtr == NULL) {
	return TCL_OK;
    }

    /*
     * Double-check that it is still a dictionary.
     */

    if (Tcl_DictObjSize(interp, dictPtr, &i) != TCL_OK) {
	return TCL_ERROR;
    }

    if (Tcl_IsShared(dictPtr)) {
	dictPtr = Tcl_DuplicateObj(dictPtr);
	allocdict = 1;
    } else {
	allocdict = 0;
    }

    if (pathc > 0) {
	/*
	 * Want to get to the dictionary which we will update; need to do
	 * prepare-for-update unsharing along the path *but* avoid generating
	 * an error on a non-extant path (we'll treat that the same as a
	 * non-extant variable. Luckily, the unsharing operation isn't
	 * deeply damaging if we don't go on to update; it's just less than
	 * perfectly efficient (but no memory should be leaked).
	 */

	leafPtr = TclTraceDictPath(interp, dictPtr, pathc, pathv,
		DICT_PATH_EXISTS | DICT_PATH_UPDATE);
	if (leafPtr == NULL) {
	    if (allocdict) {
		TclDecrRefCount(dictPtr);
	    }
	    return TCL_ERROR;
	}
	if (leafPtr == DICT_PATH_NON_EXISTENT) {
	    if (allocdict) {
		TclDecrRefCount(dictPtr);
	    }
	    return TCL_OK;
	}
    } else {
	leafPtr = dictPtr;
    }

    /*
     * Now process our updates on the leaf dictionary.
     */

    TclListObjGetElements(NULL, keysPtr, &keyc, &keyv);
    for (i=0 ; i<keyc ; i++) {
	valPtr = Tcl_ObjGetVar2(interp, keyv[i], NULL, 0);
	if (valPtr == NULL) {
	    Tcl_DictObjRemove(NULL, leafPtr, keyv[i]);
	} else if (leafPtr == valPtr) {
	    /*
	     * Someone is messing us around, trying to build a recursive
	     * structure. [Bug 1786481]
	     */

	    Tcl_DictObjPut(NULL, leafPtr, keyv[i], Tcl_DuplicateObj(valPtr));
	} else {
	    Tcl_DictObjPut(NULL, leafPtr, keyv[i], valPtr);
	}
    }

    /*
     * Ensure that none of the dictionaries in the chain still have a string
     * rep.
     */

    if (pathc > 0) {
	InvalidateDictChain(leafPtr);
    }

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.1.0/original-tclDictObj.c`, function `TclDictWithFinish original failed writeback cleanup`, lines 3955–3975. Retained reference `native_dict_with_write_error_lifetime405-request-9.1.0-original-tclDictObj.c`.

```text
    /*
     * Write back the outermost dictionary to the variable.
     */

    if (TclPtrSetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    dictPtr, TCL_LEAVE_ERR_MSG, index) == NULL) {
	if (allocdict) {
	    TclDecrRefCount(dictPtr);
	}
	return TCL_ERROR;
    }
    return TCL_OK;
}

/*
 * Local Variables:
 * mode: c
 * c-basic-offset: 4
 * fill-column: 78
 * End:
 */

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.1.0/observed-tclDictObj.c`, function `TclDictWithFinish live pre-set refcount/allocdict and post-failure Var-only observer`, lines 3963–3982. Retained reference `native_dict_with_write_error_lifetime405-request-9.1.0-observed-tclDictObj.c`.

```text
    tclLspWithEntered++;
    tclLspWithBeforeRefcount = dictPtr->refCount;
    tclLspWithAllocated = allocdict;
    tclLspWithWatch = dictPtr;
    fprintf(stderr,"WITH_BEFORE_SET|refcount=%d|allocated=%d\n",tclLspWithBeforeRefcount,tclLspWithAllocated);
    if (TclPtrSetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    dictPtr, TCL_LEAVE_ERR_MSG, index) == NULL) {
        tclLspWithFailed++;
        tclLspWithSameCell = varPtr->value.objPtr == dictPtr;
        tclLspWithDefinedScalar = TclIsVarScalar(varPtr) && !TclIsVarUndefined(varPtr);
        fprintf(stderr,"WITH_SET_ERROR|same_cell=%d|defined_scalar=%d|final_free=%d\n",tclLspWithSameCell,tclLspWithDefinedScalar,tclLspWithFinalFree);
	if (allocdict) {
	    TclDecrRefCount(dictPtr);
	}
	return TCL_ERROR;
    }
    return TCL_OK;
}

/*

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.1.0/observed-tclObj.c`, function `TclFreeObj watched free-entry integer counter; debug/non-debug branch`, lines 1274–1287. Retained reference `native_dict_with_write_error_lifetime405-request-9.1.0-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)		/* The object to be freed. */
{
    if (objPtr == tclLspWithWatch) {
        tclLspWithFinalFree++;
        tclLspWithWatch = NULL;
        fprintf(stderr,"WITH_FINAL_FREE|count=%d\n",tclLspWithFinalFree);
    }
    const Tcl_ObjType *typePtr = objPtr->typePtr;

    /*
     * This macro declares a variable, so must come here...
     */

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.1.0/observed-tclObj.c`, function `TclFreeObj watched free-entry integer counter; debug/non-debug branch`, lines 1404–1417. Retained reference `native_dict_with_write_error_lifetime405-request-9.1.0-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)		/* The object to be freed. */
{
    if (objPtr == tclLspWithWatch) {
        tclLspWithFinalFree++;
        tclLspWithWatch = NULL;
        fprintf(stderr,"WITH_FINAL_FREE|count=%d\n",tclLspWithFinalFree);
    }
    /*
     * Invalidate the string rep first so we can use the bytes value for our
     * pointer chain, and signal an obj deletion (as opposed to shimmering)
     * with 'length == -1'.
     */

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/observed-probe.c`, function `Unchanged source evaluation with integer-only body-return report before public report/source release/teardown`, lines 124–124. Retained reference `native_dict_with_write_error_lifetime405-request-observed-probe.c`.

```text
static void probe_bytes(const char*id,const char*src,int source_len){caseid=id;pathid="fixed-grammar";active=create();Obj*o=NEW(active,src,source_len);INC(o);int code=SCRIPT(active,o);fprintf(stderr,"WITH_BODY_RETURN|entered=%d|before_refcount=%d|allocated=%d|failed=%d|same_cell=%d|defined_scalar=%d|final_free=%d\n",tclLspWithEntered,tclLspWithBeforeRefcount,tclLspWithAllocated,tclLspWithFailed,tclLspWithSameCell,tclLspWithDefinedScalar,tclLspWithFinalFree);report("native-result",code);DEC(active,o);DESTROY(active);}

```

## Consumer bindings

No implementation binding is claimed by this observation record.

- [rust/tcl-vm/src/cmd_dict.rs](../../../../rust/tcl-vm/src/cmd_dict.rs), `cmd_dict::native_rmw_fixture_tests::c86_dictionary_with_write_error_retains_independent_defined_host_storage`: Keep original With405 counters and raw streams separate from the software backend's explicit retained live dictionary storage policy; no old archive private-header equivalence.
- [rust/tcl-vm/src/cmd_dict.rs](../../../../rust/tcl-vm/src/cmd_dict.rs), `cmd_dict::native_rmw_fixture_tests::dictionary_body_writeback_compares_original_native_observation_windows` (linked): Retain all329 captured outer process-code references,326 complete results and three bounded post-free observations: independent C8.5/C8.6 update-error and C8.6 with-error edges. Later fields remain captured process output, without defined storage/read/completion/effect or old executable lifetime guarantees.
- [rust/tcl-vm/src/cmd_dict.rs](../../../../rust/tcl-vm/src/cmd_dict.rs), `cmd_dict::native_rmw_fixture_tests::c86_dictionary_with_write_error_retains_independent_defined_host_storage` (linked): Read exact405 C8.6 baseline/observer stdout equality and reached integer rows with plain/update controls, then independently check the VM's own genuine live BASE dictionary header/member under unchanged original source; no native freed-content reproduction.

These source bindings establish no executed assertion result; exact software outcomes belong to the independently pinned Rust validation receipts.

## Evidence

- [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request)
- [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405)
- [rust/tcl-vm/tests/data/native_dictionary_body/probe.c](../../../../rust/tcl-vm/tests/data/native_dictionary_body/probe.c)
- [rust/tcl-vm/tests/data/native_dictionary_body/cases.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_body/cases.tsv)
- [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/8.6.18/unix/Makefile](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/8.6.18/unix/Makefile)
- [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.0.4/unix/Makefile](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.0.4/unix/Makefile)
- [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.1.0/unix/Makefile](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.1.0/unix/Makefile)
