# naming.invocation.original-constant-captured-cell-after-write-trace

Kind: `native-observation`

## Problem statement

What public effects and original argument String stages does genuine C9 const produce when its actual write trace rejects, unsets, recreates, overwrites or nests const for the captured target? Public name queries do not attest the identity of the captured native Var.

## Question

Which original argument String stages and finite public constant/write-trace effects are reached by C9 const under rejection, unset, scalar/holder recreation, overwrite and nested const, and which public boundaries remain missing when an actual case terminates by signal?

## Conclusion

The isolated511b request records48 independent native executions:46 complete with markers and two C9 case4 SIGSEGV(-11). The seven completed C9 cases retain their exact public results, A-only worker getter stage and later independent public observations. Case4 records successful worker and post-query API returns before a missing scalar-result ROW and signal; it supplies no defined post-fatal state. Original511 separately retains two buffered batch signals after three complete cases. The pinned source captures a Var, calls TclPtrSetVar and marks that captured pointer constant after a non-null return, without same-name relookup; this inspected sequence does not prove pointer validity across a write trace. Same-spelling public scalar/constant queries establish no captured native Var identity or safe lifetime. Other providers retain availability/Guest observations only.

## Scope

Two complete original fixed request/probe/preparation manifests and their raw process captures remain independent:511 has19 process receipts and two partial buffered C9 batches;511b has61 closed receipts including48 fresh single-case executions,46 exit0 completions and two signal11 cases without markers. All compile/header/link/runtime/executable associations and unchanged inputs are retained; launcher exit0 does not establish native termination success. Original header/vector-return/eval-return/result/post-query/presentation stages retain separate meaning. Public evaluation error context is not a stock-worker-only getter witness. Neither buffered tail nor isolated final API return locates a fatal machine instruction or supplies unreported state. Source interpretation cannot authenticate captured Var liveness, an allocation/generation, safe recreated-cell behavior, Native application/handler/compiler/frame/read/store/edit authority or Rust assertions. SDK source/library inputs are retained; configure/build processes lack replay receipts.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6

Status: `observed`. Channel: Genuine original Tcl_EvalObjv/Jim_EvalObjVector with original lazy header arguments, C9 declared public write traces, independent public result/post-query phases; source facts are inspected separately. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18.

Eight original initialized processes retain empty const availability and actual invalid-command Guest results. The public C evaluation path reads A and B while reporting the absent command. These rows supply no const-worker, C9 write-trace, optional-extension absence or captured-cell semantics. Original511 retains its separate complete eight availability/error cases.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Genuine original Tcl_EvalObjv/Jim_EvalObjVector with original lazy header arguments, C9 declared public write traces, independent public result/post-query phases; source facts are inspected separately. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

The isolated eight fresh cases retain seven completed public observations and one case4 SIGSEGV(-11) without completion markers. Case0 writes VALUE/constant; case1 trace rejection retains VALUE/nonconstant; case2 unset leaves the name absent and permits a mutable write; case3 scalar recreation reports RECREATED/constant; case5 holder recreation reports RECREATED/nonconstant; case6 overwrite reports TRACE_VALUE/constant; case7 nested recreation reports INNER/constant. Supported worker vector-return paths reach A String only, with B stringless through worker result presentation. Case4 records worker VECTOR_RETURN0/EVAL_RETURN0 and post-exists-and-array result1 0; the last post-scalar-read EVAL_RETURN0/header has no public result ROW or final marker. No post-fatal state or captured Var identity/liveness follows. Original buffered511 separately terminates -11 after cases0–2; case3 is partial and4–7 absent, without an identified fatal site.

### jim

Status: `observed`. Channel: Genuine original Tcl_EvalObjv/Jim_EvalObjVector with original lazy header arguments, C9 declared public write traces, independent public result/post-query phases; source facts are inspected separately. Version: jim 0.84-9-g5bac7c9.

Eight original initialized processes retain empty const availability and actual invalid-command Guest results. The Jim public evaluation path reaches neither original lazy argument. These rows supply no const-worker, C9 write-trace, optional-extension absence or captured-cell semantics. Original511 retains its separate complete eight availability/error cases.

### bigip

Status: `not-tested`.

No original BIG-IP process for this finite const captured-cell/write-trace question.

## Source inspection

tcl8.4 8.4.20, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdIL.c`, function `InfoDefaultCmd(`, lines 927–1039. Retained reference `rust-tcl-registry-tests-data-native_info_inventory_original-sources-8.4.20-tclCmdIL.c`.

```text
static int
InfoDefaultCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    Interp *iPtr = (Interp *) interp;
    char *procName, *argName, *varName;
    Proc *procPtr;
    CompiledLocal *localPtr;
    Tcl_Obj *valueObjPtr;

    if (objc != 5) {
        Tcl_WrongNumArgs(interp, 2, objv, "procname arg varname");
        return TCL_ERROR;
    }

    procName = Tcl_GetString(objv[2]);
    argName = Tcl_GetString(objv[3]);

    procPtr = TclFindProc(iPtr, procName);
    if (procPtr == NULL) {
	Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
		"\"", procName, "\" isn't a procedure", (char *) NULL);
        return TCL_ERROR;
    }

    for (localPtr = procPtr->firstLocalPtr;  localPtr != NULL;
            localPtr = localPtr->nextPtr) {
        if (TclIsVarArgument(localPtr)
		&& (strcmp(argName, localPtr->name) == 0)) {
            if (localPtr->defValuePtr != NULL) {
		valueObjPtr = Tcl_ObjSetVar2(interp, objv[4], NULL,
			localPtr->defValuePtr, 0);
                if (valueObjPtr == NULL) {
                    defStoreError:
		    varName = Tcl_GetString(objv[4]);
		    Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
	                    "couldn't store default value in variable \"",
			    varName, "\"", (char *) NULL);
                    return TCL_ERROR;
                }
		Tcl_SetIntObj(Tcl_GetObjResult(interp), 1);
            } else {
                Tcl_Obj *nullObjPtr = Tcl_NewObj();
		Tcl_IncrRefCount(nullObjPtr);
                valueObjPtr = Tcl_ObjSetVar2(interp, objv[4], NULL,
			nullObjPtr, 0);
		Tcl_DecrRefCount(nullObjPtr); /* free unneeded obj */
                if (valueObjPtr == NULL) {
                    goto defStoreError;
                }
		Tcl_SetIntObj(Tcl_GetObjResult(interp), 0);
            }
            return TCL_OK;
        }
    }

    Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
	    "procedure \"", procName, "\" doesn't have an argument \"",
	    argName, "\"", (char *) NULL);
    return TCL_ERROR;
}

/*
 *----------------------------------------------------------------------
 *
 * InfoExistsCmd --
 *
 *      Called to implement the "info exists" command that determines
 *      whether a variable exists. Handles the following syntax:
 *
 *          info exists varName
 *
 * Results:
 *      Returns TCL_OK if successful and TCL_ERROR if there is an error.
 *
 * Side effects:
 *      Returns a result in the interpreter's result object. If there is
 *	an error, the result is an error message.
 *
 *----------------------------------------------------------------------
 */

static int
InfoExistsCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    char *varName;
    Var *varPtr;

    if (objc != 3) {
        Tcl_WrongNumArgs(interp, 2, objv, "varName");
        return TCL_ERROR;
    }

    varName = Tcl_GetString(objv[2]);
    varPtr = TclVarTraceExists(interp, varName);
    if ((varPtr != NULL) && !TclIsVarUndefined(varPtr)) {
        Tcl_SetIntObj(Tcl_GetObjResult(interp), 1);
    } else {
        Tcl_SetIntObj(Tcl_GetObjResult(interp), 0);
    }
    return TCL_OK;
}

#ifdef TCL_TIP280
/*
 *----------------------------------------------------------------------

```

tcl8.4 8.4.20, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdIL.c`, function `Tcl_IncrObjCmd(`, lines 327–447. Retained reference `rust-tcl-registry-tests-data-native_info_inventory_original-sources-8.4.20-tclCmdIL.c`.

```text
int
Tcl_IncrObjCmd(dummy, interp, objc, objv)
    ClientData dummy;			/* Not used. */
    Tcl_Interp *interp;			/* Current interpreter. */
    int objc;				/* Number of arguments. */
    Tcl_Obj *CONST objv[];		/* Argument objects. */
{
    long incrAmount;
    Tcl_Obj *newValuePtr;
    
    if ((objc != 2) && (objc != 3)) {
        Tcl_WrongNumArgs(interp, 1, objv, "varName ?increment?");
	return TCL_ERROR;
    }

    /*
     * Calculate the amount to increment by.
     */
    
    if (objc == 2) {
	incrAmount = 1;
    } else {
	if (Tcl_GetLongFromObj(interp, objv[2], &incrAmount) != TCL_OK) {
	    Tcl_AddErrorInfo(interp, "\n    (reading increment)");
	    return TCL_ERROR;
	}
	/*
	 * Need to be a bit cautious to ensure that [expr]-like rules
	 * are enforced for interpretation of wide integers, despite
	 * the fact that the underlying API itself is a 'long' only one.
	 */
	if (objv[2]->typePtr == &tclIntType) {
	    incrAmount = objv[2]->internalRep.longValue;
	} else if (objv[2]->typePtr == &tclWideIntType) {
	    TclGetLongFromWide(incrAmount,objv[2]);
	} else {
	    Tcl_WideInt wide;

	    if (Tcl_GetWideIntFromObj(interp, objv[2], &wide) != TCL_OK) {
		Tcl_AddErrorInfo(interp, "\n    (reading increment)");
		return TCL_ERROR;
	    }
	    incrAmount = Tcl_WideAsLong(wide);
	    if ((wide <= Tcl_LongAsWide(LONG_MAX))
		    && (wide >= Tcl_LongAsWide(LONG_MIN))) {
		objv[2]->typePtr = &tclIntType;
		objv[2]->internalRep.longValue = incrAmount;
	    }
	}
    }
    
    /*
     * Increment the variable's value.
     */

    newValuePtr = TclIncrVar2(interp, objv[1], (Tcl_Obj *) NULL, incrAmount,
	    TCL_LEAVE_ERR_MSG);
    if (newValuePtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * Set the interpreter's object result to refer to the variable's new
     * value object.
     */

    Tcl_SetObjResult(interp, newValuePtr);
    return TCL_OK; 
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_InfoObjCmd --
 *
 *	This procedure is invoked to process the "info" Tcl command.
 *	See the user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

	/* ARGSUSED */
int
Tcl_InfoObjCmd(clientData, interp, objc, objv)
    ClientData clientData;	/* Arbitrary value passed to the command. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    static CONST char *subCmds[] = {
	     "args", "body", "cmdcount", "commands",
	     "complete", "default", "exists",
#ifdef TCL_TIP280
	     "frame",
#endif
	     "functions",
	     "globals", "hostname", "level", "library", "loaded",
	     "locals", "nameofexecutable", "patchlevel", "procs",
	     "script", "sharedlibextension", "tclversion", "vars",
	     (char *) NULL};
    enum ISubCmdIdx {
	    IArgsIdx, IBodyIdx, ICmdCountIdx, ICommandsIdx,
	    ICompleteIdx, IDefaultIdx, IExistsIdx,
#ifdef TCL_TIP280
	    IFrameIdx,
#endif
	    IFunctionsIdx,
	    IGlobalsIdx, IHostnameIdx, ILevelIdx, ILibraryIdx, ILoadedIdx,
	    ILocalsIdx, INameOfExecutableIdx, IPatchLevelIdx, IProcsIdx,
	    IScriptIdx, ISharedLibExtensionIdx, ITclVersionIdx, IVarsIdx
    };
    int index, result;

    if (objc < 2) {
        Tcl_WrongNumArgs(interp, 1, objv, "option ?arg arg ...?");

```

tcl8.4 8.4.20, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclVar.c`, function `Tcl_UnsetObjCmd(`, lines 2596–2681. Retained reference `rust-tcl-registry-tests-data-native_error_code_metadata_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclVar.c`.

```text
int
Tcl_UnsetObjCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    register int i, flags = TCL_LEAVE_ERR_MSG;
    register char *name;

    if (objc < 1) {
	Tcl_WrongNumArgs(interp, 1, objv,
		"?-nocomplain? ?--? ?varName varName ...?");
	return TCL_ERROR;
    } else if (objc == 1) {
	/*
	 * Do nothing if no arguments supplied, so as to match
	 * command documentation.
	 */
	return TCL_OK;
    }

    /*
     * Simple, restrictive argument parsing.  The only options are --
     * and -nocomplain (which must come first and be given exactly to
     * be an option).
     */
    i = 1;
    name = TclGetString(objv[i]);
    if (name[0] == '-') {
 	if (strcmp("-nocomplain", name) == 0) {
	    i++;
 	    if (i == objc) {
		return TCL_OK;
	    }
 	    flags = 0;
 	    name = TclGetString(objv[i]);
 	}
 	if (strcmp("--", name) == 0) {
 	    i++;
 	}
    }

    for (; i < objc;  i++) {
	if ((TclObjUnsetVar2(interp, objv[i], NULL, flags) != TCL_OK)
		&& (flags == TCL_LEAVE_ERR_MSG)) {
	    return TCL_ERROR;
	}
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_AppendObjCmd --
 *
 *	This object-based procedure is invoked to process the "append" 
 *	Tcl command. See the user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl object result value.
 *
 * Side effects:
 *	A variable's value may be changed.
 *
 *----------------------------------------------------------------------
 */

	/* ARGSUSED */
int
Tcl_AppendObjCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    Var *varPtr, *arrayPtr;
    char *part1;

    register Tcl_Obj *varValuePtr = NULL;
    					/* Initialized to avoid compiler
				         * warning. */
    int i;

    if (objc < 2) {

```

tcl8.4 8.4.20, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclVar.c`, function `Tcl_ArrayObjCmd(`, lines 2920–3600. Retained reference `rust-tcl-registry-tests-data-native_error_code_metadata_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclVar.c`.

```text
int
Tcl_ArrayObjCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    /*
     * The list of constants below should match the arrayOptions string array
     * below.
     */

    enum {ARRAY_ANYMORE, ARRAY_DONESEARCH,  ARRAY_EXISTS, ARRAY_GET,
	  ARRAY_NAMES, ARRAY_NEXTELEMENT, ARRAY_SET, ARRAY_SIZE,
	  ARRAY_STARTSEARCH, ARRAY_STATISTICS, ARRAY_UNSET}; 
    static CONST char *arrayOptions[] = {
	"anymore", "donesearch", "exists", "get", "names", "nextelement",
	"set", "size", "startsearch", "statistics", "unset", (char *) NULL
    };

    Interp *iPtr = (Interp *) interp;
    Var *varPtr, *arrayPtr;
    Tcl_HashEntry *hPtr;
    Tcl_Obj *resultPtr, *varNamePtr;
    int notArray;
    char *varName;
    int index, result;

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "option arrayName ?arg ...?");
	return TCL_ERROR;
    }

    if (Tcl_GetIndexFromObj(interp, objv[1], arrayOptions, "option",
	    0, &index) != TCL_OK) {
    	return TCL_ERROR;
    }

    /*
     * Locate the array variable
     */
    
    varNamePtr = objv[2];
    varName = TclGetString(varNamePtr);
    varPtr = TclObjLookupVar(interp, varNamePtr, NULL, /*flags*/ 0,
            /*msg*/ 0, /*createPart1*/ 0, /*createPart2*/ 0, &arrayPtr);

    /*
     * Special array trace used to keep the env array in sync for
     * array names, array get, etc.
     */

    if (varPtr != NULL && varPtr->tracePtr != NULL
	    && (TclIsVarArray(varPtr) || TclIsVarUndefined(varPtr))) {
	if (TCL_ERROR == CallVarTraces(iPtr, arrayPtr, varPtr, varName, NULL,
		(TCL_LEAVE_ERR_MSG|TCL_NAMESPACE_ONLY|TCL_GLOBAL_ONLY|
		TCL_TRACE_ARRAY), /* leaveErrMsg */ 1)) {
	    return TCL_ERROR;
	}
    }

    /*
     * Verify that it is indeed an array variable. This test comes after
     * the traces - the variable may actually become an array as an effect 
     * of said traces.
     */

    notArray = 0;
    if ((varPtr == NULL) || !TclIsVarArray(varPtr)
	    || TclIsVarUndefined(varPtr)) {
	notArray = 1;
    }

    /*
     * We have to wait to get the resultPtr until here because
     * CallVarTraces can affect the result.
     */

    resultPtr = Tcl_GetObjResult(interp);

    switch (index) {
        case ARRAY_ANYMORE: {
	    ArraySearch *searchPtr;
	    
	    if (objc != 4) {
	        Tcl_WrongNumArgs(interp, 2, objv, 
                        "arrayName searchId");
		return TCL_ERROR;
	    }
	    if (notArray) {
	        goto error;
	    }
	    searchPtr = ParseSearchId(interp, varPtr, varName, objv[3]);
	    if (searchPtr == NULL) {
	        return TCL_ERROR;
	    }
	    while (1) {
	        Var *varPtr2;

		if (searchPtr->nextEntry != NULL) {
		    varPtr2 = (Var *) Tcl_GetHashValue(searchPtr->nextEntry);
		    if (!TclIsVarUndefined(varPtr2)) {
		        break;
		    }
		}
		searchPtr->nextEntry = Tcl_NextHashEntry(&searchPtr->search);
		if (searchPtr->nextEntry == NULL) {
		    Tcl_SetIntObj(resultPtr, 0);
		    return TCL_OK;
		}
	    }
	    Tcl_SetIntObj(resultPtr, 1);
	    break;
	}
        case ARRAY_DONESEARCH: {
	    ArraySearch *searchPtr, *prevPtr;

	    if (objc != 4) {
	        Tcl_WrongNumArgs(interp, 2, objv, 
                        "arrayName searchId");
		return TCL_ERROR;
	    }
	    if (notArray) {
	        goto error;
	    }
	    searchPtr = ParseSearchId(interp, varPtr, varName, objv[3]);
	    if (searchPtr == NULL) {
	        return TCL_ERROR;
	    }
	    if (varPtr->searchPtr == searchPtr) {
	        varPtr->searchPtr = searchPtr->nextPtr;
	    } else {
	        for (prevPtr = varPtr->searchPtr;  ;
		     prevPtr = prevPtr->nextPtr) {
		    if (prevPtr->nextPtr == searchPtr) {
		        prevPtr->nextPtr = searchPtr->nextPtr;
			break;
		    }
		}
	    }
	    ckfree((char *) searchPtr);
	    break;
	}
        case ARRAY_EXISTS: {
	    if (objc != 3) {
	        Tcl_WrongNumArgs(interp, 2, objv, "arrayName");
	        return TCL_ERROR;
	    }
	    Tcl_SetIntObj(resultPtr, !notArray);
	    break;
	}
        case ARRAY_GET: {
	    Tcl_HashSearch search;
	    Var *varPtr2;
	    char *pattern = NULL;
	    char *name;
	    Tcl_Obj *namePtr, *valuePtr, *nameLstPtr, *tmpResPtr, **namePtrPtr;
	    int i, count;
	    
	    if ((objc != 3) && (objc != 4)) {
	        Tcl_WrongNumArgs(interp, 2, objv, "arrayName ?pattern?");
		return TCL_ERROR;
	    }
	    if (notArray) {
	        return TCL_OK;
	    }
	    if (objc == 4) {
	        pattern = TclGetString(objv[3]);
	    }

	    /*
	     * Store the array names in a new object.
	     */

	    nameLstPtr = Tcl_NewObj();
	    Tcl_IncrRefCount(nameLstPtr);

	    for (hPtr = Tcl_FirstHashEntry(varPtr->value.tablePtr, &search);
		 hPtr != NULL;  hPtr = Tcl_NextHashEntry(&search)) {
	        varPtr2 = (Var *) Tcl_GetHashValue(hPtr);
		if (TclIsVarUndefined(varPtr2)) {
		    continue;
		}
		name = Tcl_GetHashKey(varPtr->value.tablePtr, hPtr);
		if ((objc == 4) && !Tcl_StringMatch(name, pattern)) {
		    continue;	/* element name doesn't match pattern */
		}
		
		namePtr = Tcl_NewStringObj(name, -1);
		result = Tcl_ListObjAppendElement(interp, nameLstPtr,
		        namePtr);
		if (result != TCL_OK) {
		    Tcl_DecrRefCount(namePtr); /* free unneeded name obj */
		    Tcl_DecrRefCount(nameLstPtr);
		    return result;
		}
	    }

	    /*
	     * Make sure the Var structure of the array is not removed by
	     * a trace while we're working.
	     */

	    varPtr->refCount++;

	    /*
	     * Get the array values corresponding to each element name 
	     */

	    tmpResPtr = Tcl_NewObj();
	    result = Tcl_ListObjGetElements(interp, nameLstPtr,
		    &count, &namePtrPtr);
	    if (result != TCL_OK) {
		goto errorInArrayGet;
	    }
	    
	    for (i = 0; i < count; i++) { 
		namePtr = *namePtrPtr++;
		valuePtr = Tcl_ObjGetVar2(interp, objv[2], namePtr,
	                TCL_LEAVE_ERR_MSG);
		if (valuePtr == NULL) {
		    /*
		     * Some trace played a trick on us; we need to diagnose to
		     * adapt our behaviour: was the array element unset, or did
		     * the modification modify the complete array?
		     */

		    if (TclIsVarArray(varPtr) && !TclIsVarUndefined(varPtr)) {
			/*
			 * The array itself looks OK, the variable was
			 * undefined: forget it.
			 */
			
			continue;
		    } else {
			result = TCL_ERROR;
			goto errorInArrayGet;
		    }
		}
		result = Tcl_ListObjAppendElement(interp, tmpResPtr, namePtr);
		if (result != TCL_OK) {
		    goto errorInArrayGet;
		}
		result = Tcl_ListObjAppendElement(interp, tmpResPtr, valuePtr);
		if (result != TCL_OK) {
		    goto errorInArrayGet;
		}
	    }
	    varPtr->refCount--;
	    Tcl_SetObjResult(interp, tmpResPtr);
	    Tcl_DecrRefCount(nameLstPtr);
	    break;

	    errorInArrayGet:
	    varPtr->refCount--;
	    Tcl_DecrRefCount(nameLstPtr);
	    Tcl_DecrRefCount(tmpResPtr); /* free unneeded temp result obj */
	    return result;
	}
        case ARRAY_NAMES: {
	    Tcl_HashSearch search;
	    Var *varPtr2;
	    char *pattern = NULL;
	    char *name;
	    Tcl_Obj *namePtr;
	    int mode, matched = 0;
	    static CONST char *options[] = {
		"-exact", "-glob", "-regexp", (char *) NULL
	    };
	    enum options { OPT_EXACT, OPT_GLOB, OPT_REGEXP };

	    mode = OPT_GLOB;
	    
	    if ((objc < 3) || (objc > 5)) {
  	        Tcl_WrongNumArgs(interp, 2, objv,
			"arrayName ?mode? ?pattern?");
		return TCL_ERROR;
	    }
	    if (notArray) {
	        return TCL_OK;
	    }
	    if (objc == 4) {
	        pattern = Tcl_GetString(objv[3]);
	    } else if (objc == 5) {
		pattern = Tcl_GetString(objv[4]);
		if (Tcl_GetIndexFromObj(interp, objv[3], options, "option",
			0, &mode) != TCL_OK) {
		    return TCL_ERROR;
		}
	    }       		
	    for (hPtr = Tcl_FirstHashEntry(varPtr->value.tablePtr, &search);
		 hPtr != NULL; hPtr = Tcl_NextHashEntry(&search)) {
	        varPtr2 = (Var *) Tcl_GetHashValue(hPtr);
		if (TclIsVarUndefined(varPtr2)) {
		    continue;
		}
		name = Tcl_GetHashKey(varPtr->value.tablePtr, hPtr);
		if (objc > 3) {
		    switch ((enum options) mode) {
			case OPT_EXACT:
			    matched = (strcmp(name, pattern) == 0);
			    break;
			case OPT_GLOB:
			    matched = Tcl_StringMatch(name, pattern);
			    break;
			case OPT_REGEXP:
			    matched = Tcl_RegExpMatch(interp, name,
				    pattern);
			    if (matched < 0) {
				return TCL_ERROR;
			    }
			    break;
		    }
		    if (matched == 0) {
			continue;
		    }
		}
		
		namePtr = Tcl_NewStringObj(name, -1);
		result = Tcl_ListObjAppendElement(interp, resultPtr, namePtr);
		if (result != TCL_OK) {
		    Tcl_DecrRefCount(namePtr); /* free unneeded name obj */
		    return result;
		}
	    }
	    break;
	}
        case ARRAY_NEXTELEMENT: {
	    ArraySearch *searchPtr;
	    Tcl_HashEntry *hPtr;
	    
	    if (objc != 4) {
	        Tcl_WrongNumArgs(interp, 2, objv, 
                        "arrayName searchId");
		return TCL_ERROR;
	    }
	    if (notArray) {
  	        goto error;
	    }
	    searchPtr = ParseSearchId(interp, varPtr, varName, objv[3]);
	    if (searchPtr == NULL) {
	        return TCL_ERROR;
	    }
	    while (1) {
	        Var *varPtr2;

		hPtr = searchPtr->nextEntry;
		if (hPtr == NULL) {
		    hPtr = Tcl_NextHashEntry(&searchPtr->search);
		    if (hPtr == NULL) {
		        return TCL_OK;
		    }
		} else {
		    searchPtr->nextEntry = NULL;
		}
		varPtr2 = (Var *) Tcl_GetHashValue(hPtr);
		if (!TclIsVarUndefined(varPtr2)) {
		    break;
		}
	    }
	    Tcl_SetStringObj(resultPtr,
	            Tcl_GetHashKey(varPtr->value.tablePtr, hPtr), -1);
	    break;
	}
        case ARRAY_SET: {
	    if (objc != 4) {
	        Tcl_WrongNumArgs(interp, 2, objv, "arrayName list");
		return TCL_ERROR;
	    }
	    return(TclArraySet(interp, objv[2], objv[3]));
	}
        case ARRAY_SIZE: {
	    Tcl_HashSearch search;
	    Var *varPtr2;
	    int size;

	    if (objc != 3) {
	        Tcl_WrongNumArgs(interp, 2, objv, "arrayName");
		return TCL_ERROR;
	    }
	    size = 0;
	    if (!notArray) {
	        for (hPtr = Tcl_FirstHashEntry(varPtr->value.tablePtr, 
                        &search);
		     hPtr != NULL;  hPtr = Tcl_NextHashEntry(&search)) {
		    varPtr2 = (Var *) Tcl_GetHashValue(hPtr);
		    if (TclIsVarUndefined(varPtr2)) {
		        continue;
		    }
		    size++;
		}
	    }
	    Tcl_SetIntObj(resultPtr, size);
	    break;
	}
        case ARRAY_STARTSEARCH: {
	    ArraySearch *searchPtr;

	    if (objc != 3) {
	        Tcl_WrongNumArgs(interp, 2, objv, "arrayName");
		return TCL_ERROR;
	    }
	    if (notArray) {
	        goto error;
	    }
	    searchPtr = (ArraySearch *) ckalloc(sizeof(ArraySearch));
	    if (varPtr->searchPtr == NULL) {
	        searchPtr->id = 1;
		Tcl_AppendStringsToObj(resultPtr, "s-1-", varName,
		        (char *) NULL);
	    } else {
	        char string[TCL_INTEGER_SPACE];

		searchPtr->id = varPtr->searchPtr->id + 1;
		TclFormatInt(string, searchPtr->id);
		Tcl_AppendStringsToObj(resultPtr, "s-", string, "-", varName,
			(char *) NULL);
	    }
	    searchPtr->varPtr = varPtr;
	    searchPtr->nextEntry = Tcl_FirstHashEntry(varPtr->value.tablePtr,
		    &searchPtr->search);
	    searchPtr->nextPtr = varPtr->searchPtr;
	    varPtr->searchPtr = searchPtr;
	    break;
	}

	case ARRAY_STATISTICS: {
	    char *stats;

	    if (notArray) {
		goto error;
	    }

	    stats = Tcl_HashStats(varPtr->value.tablePtr);
	    if (stats != NULL) {
		Tcl_SetStringObj(Tcl_GetObjResult(interp), stats, -1);
		ckfree((void *)stats);
	    } else {
		Tcl_SetResult(interp, "error reading array statistics",
			TCL_STATIC);
		return TCL_ERROR;
	    }
	    break;
        }
	
	case ARRAY_UNSET: {
	    Tcl_HashSearch search;
	    Var *varPtr2;
	    char *pattern = NULL;
	    char *name;
          
	    if ((objc != 3) && (objc != 4)) {
		Tcl_WrongNumArgs(interp, 2, objv, "arrayName ?pattern?");
		return TCL_ERROR;
	    }
	    if (notArray) {
		return TCL_OK;
	    }
	    if (objc == 3) {
		/*
		 * When no pattern is given, just unset the whole array
		 */
		if (TclObjUnsetVar2(interp, varNamePtr, NULL, 0)
			!= TCL_OK) {
		    return TCL_ERROR;
		}
	    } else {
		pattern = Tcl_GetString(objv[3]);
		for (hPtr = Tcl_FirstHashEntry(varPtr->value.tablePtr,
			&search);
		     hPtr != NULL; hPtr = Tcl_NextHashEntry(&search)) {
		    varPtr2 = (Var *) Tcl_GetHashValue(hPtr);
		    if (TclIsVarUndefined(varPtr2)) {
			continue;
		    }
		    name = Tcl_GetHashKey(varPtr->value.tablePtr, hPtr);
		    if (Tcl_StringMatch(name, pattern) &&
			    (TclObjUnsetVar2(interp, varNamePtr, name, 0)
				    != TCL_OK)) {
			return TCL_ERROR;
		    }
		}
	    }
	    break;
	}
    }
    return TCL_OK;

    error:
    Tcl_AppendStringsToObj(resultPtr, "\"", varName, "\" isn't an array",
	    (char *) NULL);
    return TCL_ERROR;
}

/*
 *----------------------------------------------------------------------
 *
 * TclArraySet --
 *
 *	Set the elements of an array.  If there are no elements to
 *	set, create an empty array.  This routine is used by the
 *	Tcl_ArrayObjCmd and by the TclSetupEnv routine.
 *
 * Results:
 *	A standard Tcl result object.
 *
 * Side effects:
 *	A variable will be created if one does not already exist.
 *
 *----------------------------------------------------------------------
 */

int
TclArraySet(interp, arrayNameObj, arrayElemObj)
    Tcl_Interp *interp;		/* Current interpreter. */
    Tcl_Obj *arrayNameObj;	/* The array name. */
    Tcl_Obj *arrayElemObj;	/* The array elements list.  If this is
				 * NULL, create an empty array. */
{
    Var *varPtr, *arrayPtr;
    Tcl_Obj **elemPtrs;
    int result, elemLen, i, nameLen;
    char *varName, *p;
    
    varName = Tcl_GetStringFromObj(arrayNameObj, &nameLen);
    p = varName + nameLen - 1;
    if (*p == ')') {
	while (--p >= varName) {
	    if (*p == '(') {
		VarErrMsg(interp, varName, NULL, "set", needArray);
		return TCL_ERROR;
	    }
	}
    }

    varPtr = TclObjLookupVar(interp, arrayNameObj, NULL,
	    /*flags*/ TCL_LEAVE_ERR_MSG, /*msg*/ "set", /*createPart1*/ 1,
	    /*createPart2*/ 0, &arrayPtr);
    if (varPtr == NULL) {
	return TCL_ERROR;
    }

    if (arrayElemObj != NULL) {
	result = Tcl_ListObjGetElements(interp, arrayElemObj,
		&elemLen, &elemPtrs);
	if (result != TCL_OK) {
	    return result;
	}
	if (elemLen & 1) {
	    Tcl_ResetResult(interp);
	    Tcl_AppendToObj(Tcl_GetObjResult(interp),
		    "list must have an even number of elements", -1);
	    return TCL_ERROR;
	}
	if (elemLen > 0) {
	    /*
	     * We needn't worry about traces invalidating arrayPtr:
	     * should that be the case, TclPtrSetVar will return NULL
	     * so that we break out of the loop and return an error.
	     */

	    for (i = 0;  i < elemLen;  i += 2) {
		char *part2 = TclGetString(elemPtrs[i]);
		Var *elemVarPtr = TclLookupArrayElement(interp, varName, 
                        part2, TCL_LEAVE_ERR_MSG, "set", 1, 1, varPtr);
		if ((elemVarPtr == NULL) ||
		        (TclPtrSetVar(interp, elemVarPtr, varPtr, varName,
			 part2, elemPtrs[i+1], TCL_LEAVE_ERR_MSG) == NULL)) {
		    result = TCL_ERROR;
		    break;
		}

		/*
		 * The TclPtrSetVar call might have shimmered
		 * arrayElemObj to another type, so re-fetch
		 * the pointers for safety.
		 */
		Tcl_ListObjGetElements(NULL, arrayElemObj,
			&elemLen, &elemPtrs);
	    }
	    return result;
	}
    }
    
    /*
     * The list is empty make sure we have an array, or create
     * one if necessary.
     */
    
    if (varPtr != NULL) {
	if (!TclIsVarUndefined(varPtr) && TclIsVarArray(varPtr)) {
	    /*
	     * Already an array, done.
	     */
	    
	    return TCL_OK;
	}
	if (TclIsVarArrayElement(varPtr) || !TclIsVarUndefined(varPtr)) {
	    /*
	     * Either an array element, or a scalar: lose!
	     */
	    
	    VarErrMsg(interp, varName, (char *)NULL, "array set", needArray);
	    return TCL_ERROR;
	}
    }
    TclSetVarArray(varPtr);
    TclClearVarUndefined(varPtr);
    varPtr->value.tablePtr =
	(Tcl_HashTable *) ckalloc(sizeof(Tcl_HashTable));
    Tcl_InitHashTable(varPtr->value.tablePtr, TCL_STRING_KEYS);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * ObjMakeUpvar --
 *
 *	This procedure does all of the work of the "global" and "upvar"
 *	commands.
 *
 * Results:
 *	A standard Tcl completion code. If an error occurs then an
 *	error message is left in iPtr->result.
 *
 * Side effects:
 *	The variable given by myName is linked to the variable in framePtr
 *	given by otherP1 and otherP2, so that references to myName are
 *	redirected to the other variable like a symbolic link.
 *
 *----------------------------------------------------------------------
 */

static int
ObjMakeUpvar(interp, framePtr, otherP1Ptr, otherP2, otherFlags, myName, myFlags, index)
    Tcl_Interp *interp;		/* Interpreter containing variables. Used
			         * for error messages, too. */
    CallFrame *framePtr;	/* Call frame containing "other" variable.
				 * NULL means use global :: context. */
    Tcl_Obj *otherP1Ptr;
    CONST char *otherP2;	/* Two-part name of variable in framePtr. */
    CONST int otherFlags;	/* 0, TCL_GLOBAL_ONLY or TCL_NAMESPACE_ONLY:
				 * indicates scope of "other" variable. */
    CONST char *myName;		/* Name of variable which will refer to
				 * otherP1/otherP2. Must be a scalar. */
    int myFlags;		/* 0, TCL_GLOBAL_ONLY or TCL_NAMESPACE_ONLY:
				 * indicates scope of myName. */
    int index;                  /* If the variable to be linked is an indexed
				 * scalar, this is its index. Otherwise, -1. */
{
    Interp *iPtr = (Interp *) interp;
    Var *otherPtr, *varPtr, *arrayPtr;
    CallFrame *varFramePtr;
    CONST char *errMsg;

    /*
     * Find "other" in "framePtr". If not looking up other in just the
     * current namespace, temporarily replace the current var frame
     * pointer in the interpreter in order to use TclObjLookupVar.
     */

    varFramePtr = iPtr->varFramePtr;
    if (!(otherFlags & TCL_NAMESPACE_ONLY)) {
	iPtr->varFramePtr = framePtr;
    }
    otherPtr = TclObjLookupVar(interp, otherP1Ptr, otherP2,
	    (otherFlags | TCL_LEAVE_ERR_MSG), "access",
            /*createPart1*/ 1, /*createPart2*/ 1, &arrayPtr);
    if (!(otherFlags & TCL_NAMESPACE_ONLY)) {
	iPtr->varFramePtr = varFramePtr;
    }
    if (otherPtr == NULL) {
	return TCL_ERROR;
    }

    if (index >= 0) {
	if (!varFramePtr->isProcCallFrame) {
	    panic("ObjMakeUpvar called with an index outside from a proc.\n");
	}

```

tcl8.4 8.4.20, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclVar.c`, function `CleanupVar(`, lines 5018–5148. Retained reference `rust-tcl-registry-tests-data-native_error_code_metadata_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclVar.c`.

```text
static void
CleanupVar(varPtr, arrayPtr)
    Var *varPtr;		/* Pointer to variable that may be a
				 * candidate for being expunged. */
    Var *arrayPtr;		/* Array that contains the variable, or
				 * NULL if this variable isn't an array
				 * element. */
{
    if (TclIsVarUndefined(varPtr) && (varPtr->refCount == 0)
	    && (varPtr->tracePtr == NULL)
	    && (varPtr->flags & VAR_IN_HASHTABLE)) {
	if (varPtr->hPtr != NULL) {
	    Tcl_DeleteHashEntry(varPtr->hPtr);
	}
	ckfree((char *) varPtr);
    }
    if (arrayPtr != NULL) {
	if (TclIsVarUndefined(arrayPtr) && (arrayPtr->refCount == 0)
		&& (arrayPtr->tracePtr == NULL)
	        && (arrayPtr->flags & VAR_IN_HASHTABLE)) {
	    if (arrayPtr->hPtr != NULL) {
		Tcl_DeleteHashEntry(arrayPtr->hPtr);
	    }
	    ckfree((char *) arrayPtr);
	}
    }
}
/*
 *----------------------------------------------------------------------
 *
 * VarErrMsg --
 *
 *      Generate a reasonable error message describing why a variable
 *      operation failed.
 *
 * Results:
 *      None.
 *
 * Side effects:
 *      The interp's result is set to hold a message identifying the
 *      variable given by part1 and part2 and describing why the
 *      variable operation failed.
 *
 *----------------------------------------------------------------------
 */

static void
VarErrMsg(interp, part1, part2, operation, reason)
    Tcl_Interp *interp;         /* Interpreter in which to record message. */
    CONST char *part1;
    CONST char *part2;		/* Variable's two-part name. */
    CONST char *operation;      /* String describing operation that failed,
                                 * e.g. "read", "set", or "unset". */
    CONST char *reason;         /* String describing why operation failed. */
{
    Tcl_ResetResult(interp);
    Tcl_AppendResult(interp, "can't ", operation, " \"", part1,
	    (char *) NULL);
    if (part2 != NULL) {
        Tcl_AppendResult(interp, "(", part2, ")", (char *) NULL);
    }
    Tcl_AppendResult(interp, "\": ", reason, (char *) NULL);
}

/*
 *----------------------------------------------------------------------
 *
 * TclTraceVarExists --
 *
 *	This is called from info exists.  We need to trigger read
 *	and/or array traces because they may end up creating a
 *	variable that doesn't currently exist.
 *
 * Results:
 *	A pointer to the Var structure, or NULL.
 *
 * Side effects:
 *	May fill in error messages in the interp.
 *
 *----------------------------------------------------------------------
 */

Var *
TclVarTraceExists(interp, varName)
    Tcl_Interp *interp;		/* The interpreter */
    CONST char *varName;	/* The variable name */
{
    Var *varPtr;
    Var *arrayPtr;

    /*
     * The choice of "create" flag values is delicate here, and
     * matches the semantics of GetVar.  Things are still not perfect,
     * however, because if you do "info exists x" you get a varPtr
     * and therefore trigger traces.  However, if you do 
     * "info exists x(i)", then you only get a varPtr if x is already
     * known to be an array.  Otherwise you get NULL, and no trace
     * is triggered.  This matches Tcl 7.6 semantics.
     */

    varPtr = TclLookupVar(interp, varName, (char *) NULL,
            0, "access", /*createPart1*/ 0, /*createPart2*/ 1, &arrayPtr);

    if (varPtr == NULL) {
	return NULL;
    }

    if ((varPtr->tracePtr != NULL)
	    || ((arrayPtr != NULL) && (arrayPtr->tracePtr != NULL))) {
	CallVarTraces((Interp *)interp, arrayPtr, varPtr, varName, NULL,
		TCL_TRACE_READS, /* leaveErrMsg */ 0);
    }

    /*
     * If the variable doesn't exist anymore and no-one's using
     * it, then free up the relevant structures and hash table entries.
     */

    if (TclIsVarUndefined(varPtr)) {
	CleanupVar(varPtr, arrayPtr);
	return NULL;
    }

    return varPtr;
}

/*
 *----------------------------------------------------------------------
 *
 * Internal functions for variable name object types --
 *

```

tcl8.4 8.4.20, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclVar.c`, function `Tcl_UnsetVar2(`, lines 1959–2079. Retained reference `rust-tcl-registry-tests-data-native_error_code_metadata_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclVar.c`.

```text
int
Tcl_UnsetVar2(interp, part1, part2, flags)
    Tcl_Interp *interp;		/* Command interpreter in which varName is
				 * to be looked up. */
    CONST char *part1;		/* Name of variable or array. */
    CONST char *part2;		/* Name of element within array or NULL. */
    int flags;			/* OR-ed combination of any of
				 * TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_LEAVE_ERR_MSG. */
{
    int result;
    Tcl_Obj *part1Ptr;

    part1Ptr = Tcl_NewStringObj(part1, -1);
    Tcl_IncrRefCount(part1Ptr);
    /* Filter to pass through only the flags this interface supports. */
    flags &= (TCL_GLOBAL_ONLY|TCL_NAMESPACE_ONLY|TCL_LEAVE_ERR_MSG);
    result = TclObjUnsetVar2(interp, part1Ptr, part2, flags);
    TclDecrRefCount(part1Ptr);

    return result;
}


/*
 *----------------------------------------------------------------------
 *
 * TclObjUnsetVar2 --
 *
 *	Delete a variable, given a 2-object name.
 *
 * Results:
 *	Returns TCL_OK if the variable was successfully deleted, TCL_ERROR
 *	if the variable can't be unset.  In the event of an error,
 *	if the TCL_LEAVE_ERR_MSG flag is set then an error message
 *	is left in the interp's result.
 *
 * Side effects:
 *	If part1ptr and part2Ptr indicate a local or global variable in interp,
 *	it is deleted.  If part1Ptr is an array name and part2Ptr is NULL, then
 *	the whole array is deleted.
 *
 *----------------------------------------------------------------------
 */

int
TclObjUnsetVar2(interp, part1Ptr, part2, flags)
    Tcl_Interp *interp;		/* Command interpreter in which varName is
				 * to be looked up. */
    Tcl_Obj *part1Ptr;		/* Name of variable or array. */
    CONST char *part2;		/* Name of element within array or NULL. */
    int flags;			/* OR-ed combination of any of
				 * TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_LEAVE_ERR_MSG. */
{
    Var *varPtr;
    Interp *iPtr = (Interp *) interp;
    Var *arrayPtr;
    int result;
    char *part1;

    part1 = TclGetString(part1Ptr);
    varPtr = TclObjLookupVar(interp, part1Ptr, part2, flags, "unset",
	    /*createPart1*/ 0, /*createPart2*/ 0, &arrayPtr);
    if (varPtr == NULL) {
	return TCL_ERROR;
    }
 
    result = (TclIsVarUndefined(varPtr)? TCL_ERROR : TCL_OK);

    /*
     * Keep the variable alive until we're done with it. We used to
     * increase/decrease the refCount for each operation, making it
     * hard to find [Bug 735335] - caused by unsetting the variable
     * whose value was the variable's name.
     */
    
    varPtr->refCount++;

    UnsetVarStruct(varPtr, arrayPtr, iPtr, part1, part2, flags);

    /*
     * It's an error to unset an undefined variable.
     */
	
    if (result != TCL_OK) {
	if (flags & TCL_LEAVE_ERR_MSG) {
	    VarErrMsg(interp, part1, part2, "unset", 
		    ((arrayPtr == NULL) ? noSuchVar : noSuchElement));
	}
    }

    /*
     * Try to avoid keeping the Var struct allocated due to a tclNsVarNameType 
     * keeping a reference. This removes some additional exteriorisations of
     * [Bug 736729], but may be a good thing independently of the bug.
     */

    if (part1Ptr->typePtr == &tclNsVarNameType) {
	part1Ptr->typePtr->freeIntRepProc(part1Ptr);
	part1Ptr->typePtr = NULL;
    }

    /*
     * Finally, if the variable is truly not in use then free up its Var
     * structure and remove it from its hash table, if any. The ref count of
     * its value object, if any, was decremented above.
     */

    varPtr->refCount--;
    CleanupVar(varPtr, arrayPtr);
    return result;
}

/*
 *----------------------------------------------------------------------
 *
 * UnsetVarStruct --
 *
 *	Unset and delete a variable. This does the internal work for
 *	TclObjUnsetVar2 and TclDeleteNamespaceVars, which call here for each

```

tcl8.4 8.4.20, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclVar.c`, function `Tcl_ObjSetVar2(`, lines 1492–1597. Retained reference `rust-tcl-registry-tests-data-native_error_code_metadata_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclVar.c`.

```text
Tcl_Obj *
Tcl_ObjSetVar2(interp, part1Ptr, part2Ptr, newValuePtr, flags)
    Tcl_Interp *interp;		/* Command interpreter in which variable is
				 * to be found. */
    register Tcl_Obj *part1Ptr;	/* Points to an object holding the name of
				 * an array (if part2 is non-NULL) or the
				 * name of a variable. */
    register Tcl_Obj *part2Ptr;	/* If non-null, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *newValuePtr;	/* New value for variable. */
    int flags;			/* Various flags that tell how to set value:
				 * any of TCL_GLOBAL_ONLY,
				 * TCL_NAMESPACE_ONLY, TCL_APPEND_VALUE,
				 * TCL_LIST_ELEMENT, or TCL_LEAVE_ERR_MSG. */
{
    Var *varPtr, *arrayPtr;
    char *part1, *part2;

    part1 = TclGetString(part1Ptr);
    part2 = ((part2Ptr == NULL) ? NULL : Tcl_GetString(part2Ptr));    

    /* Filter to pass through only the flags this interface supports. */
    flags &= (TCL_GLOBAL_ONLY|TCL_NAMESPACE_ONLY|TCL_LEAVE_ERR_MSG
	    |TCL_APPEND_VALUE|TCL_LIST_ELEMENT);
    varPtr = TclObjLookupVar(interp, part1Ptr, part2, flags, "set",
	    /*createPart1*/ 1, /*createPart2*/ 1, &arrayPtr);
    if (varPtr == NULL) {
	return NULL;
    }

    return TclPtrSetVar(interp, varPtr, arrayPtr, part1, part2, 
            newValuePtr, flags);
}


/*
 *----------------------------------------------------------------------
 *
 * TclPtrSetVar --
 *
 *	This function is the same as Tcl_SetVar2Ex above, except that
 *      it requires pointers to the variable's Var structs in addition
 *	to the variable names.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the write operation was disallowed because an array was
 *	expected but not found (or vice versa), then NULL is returned; if
 *	the TCL_LEAVE_ERR_MSG flag is set, then an explanatory message will
 *	be left in the interpreter's result. Note that the returned object
 *	may not be the same one referenced by newValuePtr; this is because
 *	variable traces may modify the variable's value.
 *
 * Side effects:
 *	The value of the given variable is set. If either the array or the
 *	entry didn't exist then a new variable is created.

 *
 *----------------------------------------------------------------------
 */

Tcl_Obj *
TclPtrSetVar(interp, varPtr, arrayPtr, part1, part2, newValuePtr, flags)
    Tcl_Interp *interp;		/* Command interpreter in which variable is
				 * to be looked up. */
    register Var *varPtr;
    Var *arrayPtr;
    CONST char *part1;		/* Name of an array (if part2 is non-NULL)
				 * or the name of a variable. */
    CONST char *part2;		/* If non-NULL, gives the name of an element
				 * in the array part1. */
    Tcl_Obj *newValuePtr;	/* New value for variable. */
    CONST int flags;		/* OR-ed combination of TCL_GLOBAL_ONLY,
				 * and TCL_LEAVE_ERR_MSG bits. */
{
    Interp *iPtr = (Interp *) interp;
    Tcl_Obj *oldValuePtr;
    Tcl_Obj *resultPtr = NULL;
    int result;

    /*
     * If the variable is in a hashtable and its hPtr field is NULL, then we
     * may have an upvar to an array element where the array was deleted
     * or an upvar to a namespace variable whose namespace was deleted.
     * Generate an error (allowing the variable to be reset would screw up
     * our storage allocation and is meaningless anyway).
     */

    if ((varPtr->flags & VAR_IN_HASHTABLE) && (varPtr->hPtr == NULL)) {
	if (flags & TCL_LEAVE_ERR_MSG) {
	    if (TclIsVarArrayElement(varPtr)) {
		VarErrMsg(interp, part1, part2, "set", danglingElement);
	    } else {
		VarErrMsg(interp, part1, part2, "set", danglingVar);
	    }
	}
	return NULL;
    }

    /*
     * It's an error to try to set an array variable itself.
     */

    if (TclIsVarArray(varPtr) && !TclIsVarUndefined(varPtr)) {
	if (flags & TCL_LEAVE_ERR_MSG) {

```

tcl8.4 8.4.20, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclBasic.c`, function `Tcl_EvalObjv(`, lines 3350–3460. Retained reference `rust-tcl-registry-tests-data-native_info_commands_literal_original-original-inputs-8.4.20-generic-tclBasic.c`.

```text
int
Tcl_EvalObjv(interp, objc, objv, flags)
    Tcl_Interp *interp;		/* Interpreter in which to evaluate the
				 * command.  Also used for error
				 * reporting. */
    int objc;			/* Number of words in command. */
    Tcl_Obj *CONST objv[];	/* An array of pointers to objects that are
				 * the words that make up the command. */
    int flags;			/* Collection of OR-ed bits that control
				 * the evaluation of the script.  Only
				 * TCL_EVAL_GLOBAL and TCL_EVAL_INVOKE
				 * are  currently supported. */
{
    Interp *iPtr = (Interp *)interp;
    Trace *tracePtr;
    Tcl_DString cmdBuf;
    char *cmdString = "";	/* A command string is only necessary for
				 * command traces or error logs; it will be
				 * generated to replace this default value if
				 * necessary. */
    int cmdLen = 0;		/* a non-zero value indicates that a command
				 * string was generated. */
    int code = TCL_OK;
    int i;
    int allowExceptions = (iPtr->evalFlags & TCL_ALLOW_EXCEPTIONS);

    for (tracePtr = iPtr->tracePtr; tracePtr; tracePtr = tracePtr->nextPtr) {
	if ((tracePtr->level == 0) || (iPtr->numLevels <= tracePtr->level)) {
	    /*
	     * The command may be needed for an execution trace.  Generate a
	     * command string.
	     */
	    
	    Tcl_DStringInit(&cmdBuf);
	    for (i = 0; i < objc; i++) {
		Tcl_DStringAppendElement(&cmdBuf, Tcl_GetString(objv[i]));
	    }
	    cmdString = Tcl_DStringValue(&cmdBuf);
	    cmdLen = Tcl_DStringLength(&cmdBuf);
	    break;
	}
    }

    iPtr->numLevels++;
    code = TclEvalObjvInternal(interp, objc, objv, cmdString, cmdLen, flags);
    iPtr->numLevels--;

    /*
     * If we are again at the top level, process any unusual 
     * return code returned by the evaluated code. 
     */
	
    if (iPtr->numLevels == 0) {
	if (code == TCL_RETURN) {
	    code = TclUpdateReturnInfo(iPtr);
	}
	if ((code != TCL_OK) && (code != TCL_ERROR) 
	    && !allowExceptions) {
	    ProcessUnexpectedResult(interp, code);
	    code = TCL_ERROR;
	}
    }
	    
    if ((code == TCL_ERROR) && !(flags & TCL_EVAL_INVOKE)) {

	/* 
	 * If there was an error, a command string will be needed for the 
	 * error log: generate it now if it was not done previously.
	 */

	if (cmdLen == 0) {
	    Tcl_DStringInit(&cmdBuf);
	    for (i = 0; i < objc; i++) {
		Tcl_DStringAppendElement(&cmdBuf, Tcl_GetString(objv[i]));
	    }
	    cmdString = Tcl_DStringValue(&cmdBuf);
	    cmdLen = Tcl_DStringLength(&cmdBuf);
	}
	Tcl_LogCommandInfo(interp, cmdString, cmdString, cmdLen);
    }

    if (cmdLen != 0) {
	Tcl_DStringFree(&cmdBuf);
    }
    return code;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_LogCommandInfo --
 *
 *	This procedure is invoked after an error occurs in an interpreter.
 *	It adds information to the "errorInfo" variable to describe the
 *	command that was being executed when the error occurred.
 *
 * Results:
 *	None.
 *
 * Side effects:
 *	Information about the command is added to errorInfo and the
 *	line number stored internally in the interpreter is set.  If this
 *	is the first call to this procedure or Tcl_AddObjErrorInfo since
 *	an error occurred, then old information in errorInfo is
 *	deleted.
 *
 *----------------------------------------------------------------------
 */

void
Tcl_LogCommandInfo(interp, script, command, length)

```

tcl8.5 8.5.19, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdIL.c`, function `InfoDefaultCmd(`, lines 899–1011. Retained reference `rust-tcl-registry-tests-data-native_info_inventory_original-sources-8.5.19-tclCmdIL.c`.

```text
static int
InfoDefaultCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Interp *iPtr = (Interp *) interp;
    char *procName, *argName, *varName;
    Proc *procPtr;
    CompiledLocal *localPtr;
    Tcl_Obj *valueObjPtr;

    if (objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "procname arg varname");
	return TCL_ERROR;
    }

    procName = TclGetString(objv[1]);
    argName = TclGetString(objv[2]);

    procPtr = TclFindProc(iPtr, procName);
    if (procPtr == NULL) {
	Tcl_AppendResult(interp, "\"", procName, "\" isn't a procedure",NULL);
	return TCL_ERROR;
    }

    for (localPtr = procPtr->firstLocalPtr;  localPtr != NULL;
	    localPtr = localPtr->nextPtr) {
	if (TclIsVarArgument(localPtr)
		&& (strcmp(argName, localPtr->name) == 0)) {
	    if (localPtr->defValuePtr != NULL) {
		valueObjPtr = Tcl_ObjSetVar2(interp, objv[3], NULL,
			localPtr->defValuePtr, 0);
		if (valueObjPtr == NULL) {
		    goto defStoreError;
		}
		Tcl_SetObjResult(interp, Tcl_NewIntObj(1));
	    } else {
		Tcl_Obj *nullObjPtr = Tcl_NewObj();
		valueObjPtr = Tcl_ObjSetVar2(interp, objv[3], NULL,
			nullObjPtr, 0);
		if (valueObjPtr == NULL) {
		    goto defStoreError;
		}
		Tcl_SetObjResult(interp, Tcl_NewIntObj(0));
	    }
	    return TCL_OK;
	}
    }

    Tcl_AppendResult(interp, "procedure \"", procName,
	    "\" doesn't have an argument \"", argName, "\"", NULL);
    return TCL_ERROR;

  defStoreError:
    varName = TclGetString(objv[3]);
    Tcl_AppendResult(interp, "couldn't store default value in variable \"",
	    varName, "\"", NULL);
    return TCL_ERROR;
}

/*
 *----------------------------------------------------------------------
 *
 * TclInfoExistsCmd --
 *
 *	Called to implement the "info exists" command that determines whether
 *	a variable exists. Handles the following syntax:
 *
 *	    info exists varName
 *
 * Results:
 *	Returns TCL_OK if successful and TCL_ERROR if there is an error.
 *
 * Side effects:
 *	Returns a result in the interpreter's result object. If there is an
 *	error, the result is an error message.
 *
 *----------------------------------------------------------------------
 */

int
TclInfoExistsCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    char *varName;
    Var *varPtr;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "varName");
	return TCL_ERROR;
    }

    varName = TclGetString(objv[1]);
    varPtr = TclVarTraceExists(interp, varName);

    Tcl_SetObjResult(interp,
	    Tcl_NewBooleanObj(varPtr && varPtr->value.objPtr));
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * InfoFrameCmd --
 *	TIP #280
 *
 *	Called to implement the "info frame" command that returns the location
 *	of either the currently executing command, or its caller. Handles the

```

tcl8.5 8.5.19, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdIL.c`, function `Tcl_IncrObjCmd(`, lines 332–452. Retained reference `rust-tcl-registry-tests-data-native_info_inventory_original-sources-8.5.19-tclCmdIL.c`.

```text
int
Tcl_IncrObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *newValuePtr, *incrPtr;

    if ((objc != 2) && (objc != 3)) {
	Tcl_WrongNumArgs(interp, 1, objv, "varName ?increment?");
	return TCL_ERROR;
    }

    if (objc == 3) {
	incrPtr = objv[2];
    } else {
	incrPtr = Tcl_NewIntObj(1);
    }
    Tcl_IncrRefCount(incrPtr);
    newValuePtr = TclIncrObjVar2(interp, objv[1], NULL,
	    incrPtr, TCL_LEAVE_ERR_MSG);
    Tcl_DecrRefCount(incrPtr);

    if (newValuePtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * Set the interpreter's object result to refer to the variable's new
     * value object.
     */

    Tcl_SetObjResult(interp, newValuePtr);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * TclInitInfoCmd --
 *
 *	This function is called to create the "info" Tcl command. See the user
 *	documentation for details on what it does.
 *
 * Results:
 *	FIXME
 *
 * Side effects:
 *	none
 *
 *----------------------------------------------------------------------
 */

Tcl_Command
TclInitInfoCmd(
    Tcl_Interp *interp)		/* Current interpreter. */
{
    return TclMakeEnsemble(interp, "info", defaultInfoMap);
}

/*
 *----------------------------------------------------------------------
 *
 * InfoArgsCmd --
 *
 *	Called to implement the "info args" command that returns the argument
 *	list for a procedure. Handles the following syntax:
 *
 *	    info args procName
 *
 * Results:
 *	Returns TCL_OK if successful and TCL_ERROR if there is an error.
 *
 * Side effects:
 *	Returns a result in the interpreter's result object. If there is an
 *	error, the result is an error message.
 *
 *----------------------------------------------------------------------
 */

static int
InfoArgsCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    register Interp *iPtr = (Interp *) interp;
    char *name;
    Proc *procPtr;
    CompiledLocal *localPtr;
    Tcl_Obj *listObjPtr;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "procname");
	return TCL_ERROR;
    }

    name = TclGetString(objv[1]);
    procPtr = TclFindProc(iPtr, name);
    if (procPtr == NULL) {
	Tcl_AppendResult(interp, "\"", name, "\" isn't a procedure", NULL);
	return TCL_ERROR;
    }

    /*
     * Build a return list containing the arguments.
     */

    listObjPtr = Tcl_NewListObj(0, NULL);
    for (localPtr = procPtr->firstLocalPtr;  localPtr != NULL;
	    localPtr = localPtr->nextPtr) {
	if (TclIsVarArgument(localPtr)) {
	    Tcl_ListObjAppendElement(interp, listObjPtr,
		    Tcl_NewStringObj(localPtr->name, -1));
	}
    }
    Tcl_SetObjResult(interp, listObjPtr);
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclVar.c`, function `Tcl_UnsetObjCmd(`, lines 2515–2600. Retained reference `rust-tcl-registry-tests-data-native_dict_write_error_lifetime226-failed-request225-original-source-tclVar.c`.

```text
int
Tcl_UnsetObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    register int i, flags = TCL_LEAVE_ERR_MSG;
    register char *name;

    if (objc == 1) {
	/*
	 * Do nothing if no arguments supplied, so as to match command
	 * documentation.
	 */

	return TCL_OK;
    }

    /*
     * Simple, restrictive argument parsing. The only options are -- and
     * -nocomplain (which must come first and be given exactly to be an
     * option).
     */

    i = 1;
    name = TclGetString(objv[i]);
    if (name[0] == '-') {
	if (strcmp("-nocomplain", name) == 0) {
	    i++;
	    if (i == objc) {
		return TCL_OK;
	    }
	    flags = 0;
	    name = TclGetString(objv[i]);
	}
	if (strcmp("--", name) == 0) {
	    i++;
	}
    }

    for (; i < objc; i++) {
	if ((TclObjUnsetVar2(interp, objv[i], NULL, flags) != TCL_OK)
		&& (flags == TCL_LEAVE_ERR_MSG)) {
	    return TCL_ERROR;
	}
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_AppendObjCmd --
 *
 *	This object-based function is invoked to process the "append" Tcl
 *	command. See the user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl object result value.
 *
 * Side effects:
 *	A variable's value may be changed.
 *
 *----------------------------------------------------------------------
 */

	/* ARGSUSED */
int
Tcl_AppendObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    register Tcl_Obj *varValuePtr = NULL;
				/* Initialized to avoid compiler warning. */

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "varName ?value value ...?");
	return TCL_ERROR;
    }

    if (objc == 2) {
	varValuePtr = Tcl_ObjGetVar2(interp, objv[1], NULL,TCL_LEAVE_ERR_MSG);
	if (varValuePtr == NULL) {

```

tcl8.5 8.5.19, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclVar.c`, function `TclIncrObjVar2(`, lines 2018–2118. Retained reference `rust-tcl-registry-tests-data-native_dict_write_error_lifetime226-failed-request225-original-source-tclVar.c`.

```text
Tcl_Obj *
TclIncrObjVar2(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Tcl_Obj *part1Ptr,		/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,		/* If non-null, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *incrPtr,		/* Amount to be added to variable. */
    int flags)			/* Various flags that tell how to incr value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
{
    Var *varPtr, *arrayPtr;

    varPtr = TclObjLookupVarEx(interp, part1Ptr, part2Ptr, flags, "read",
	    1, 1, &arrayPtr);
    if (varPtr == NULL) {
	Tcl_AddObjErrorInfo(interp,
		"\n    (reading value of variable to increment)", -1);
	return NULL;
    }
    return TclPtrIncrObjVar(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    incrPtr, flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * TclPtrIncrObjVar --
 *
 *	Given the pointers to a variable and possible containing array,
 *	increment the Tcl object value of the variable by a Tcl_Obj increment.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the specified variable doesn't exist, or there is a clash
 *	in array usage, or an error occurs while executing variable traces,
 *	then NULL is returned and a message will be left in the interpreter's
 *	result.
 *
 * Side effects:
 *	The value of the given variable is incremented by the specified
 *	amount. If either the array or the entry didn't exist then a new
 *	variable is created. The ref count for the returned object is _not_
 *	incremented to reflect the returned reference; if you want to keep a
 *	reference to the object you must increment its ref count yourself.
 *
 *----------------------------------------------------------------------
 */

Tcl_Obj *
TclPtrIncrObjVar(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Var *varPtr,		/* Reference to the variable to set. */
    Var *arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,		/* If non-null, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *incrPtr,		/* Increment value. */
/* TODO: Which of these flag values really make sense? */
    const int flags,		/* Various flags that tell how to incr value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
    int index)			/* Index into the local variable table of the
				 * variable, or -1. Only used when part1Ptr is
				 * NULL. */
{
    register Tcl_Obj *varValuePtr;

    if (TclIsVarInHash(varPtr)) {
	VarHashRefCount(varPtr)++;
    }
    varValuePtr = TclPtrGetVar(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    flags, index);
    if (TclIsVarInHash(varPtr)) {
	VarHashRefCount(varPtr)--;
    }
    if (varValuePtr == NULL) {
	varValuePtr = Tcl_NewIntObj(0);
    }
    if (Tcl_IsShared(varValuePtr)) {
	/* Copy on write */
	varValuePtr = Tcl_DuplicateObj(varValuePtr);

	if (TCL_OK == TclIncrObj(interp, varValuePtr, incrPtr)) {
	    return TclPtrSetVar(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
		    varValuePtr, flags, index);
	} else {
	    Tcl_DecrRefCount(varValuePtr);
	    return NULL;

```

tcl8.5 8.5.19, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclVar.c`, function `TclPtrIncrObjVar(`, lines 2072–2232. Retained reference `rust-tcl-registry-tests-data-native_dict_write_error_lifetime226-failed-request225-original-source-tclVar.c`.

```text
Tcl_Obj *
TclPtrIncrObjVar(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Var *varPtr,		/* Reference to the variable to set. */
    Var *arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,		/* If non-null, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *incrPtr,		/* Increment value. */
/* TODO: Which of these flag values really make sense? */
    const int flags,		/* Various flags that tell how to incr value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
    int index)			/* Index into the local variable table of the
				 * variable, or -1. Only used when part1Ptr is
				 * NULL. */
{
    register Tcl_Obj *varValuePtr;

    if (TclIsVarInHash(varPtr)) {
	VarHashRefCount(varPtr)++;
    }
    varValuePtr = TclPtrGetVar(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    flags, index);
    if (TclIsVarInHash(varPtr)) {
	VarHashRefCount(varPtr)--;
    }
    if (varValuePtr == NULL) {
	varValuePtr = Tcl_NewIntObj(0);
    }
    if (Tcl_IsShared(varValuePtr)) {
	/* Copy on write */
	varValuePtr = Tcl_DuplicateObj(varValuePtr);

	if (TCL_OK == TclIncrObj(interp, varValuePtr, incrPtr)) {
	    return TclPtrSetVar(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
		    varValuePtr, flags, index);
	} else {
	    Tcl_DecrRefCount(varValuePtr);
	    return NULL;
	}
    } else {
	/* Unshared - can Incr in place */
	if (TCL_OK == TclIncrObj(interp, varValuePtr, incrPtr)) {

	    /*
	     * This seems dumb to write the incremeted value into the var
	     * after we just adjusted the value in place, but the spec for
	     * [incr] requires that write traces fire, and making this call
	     * is the way to make that happen.
	     */

	    return TclPtrSetVar(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
		    varValuePtr, flags, index);
	} else {
	    return NULL;
	}
    }
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_UnsetVar --
 *
 *	Delete a variable, so that it may not be accessed anymore.
 *
 * Results:
 *	Returns TCL_OK if the variable was successfully deleted, TCL_ERROR if
 *	the variable can't be unset. In the event of an error, if the
 *	TCL_LEAVE_ERR_MSG flag is set then an error message is left in the
 *	interp's result.
 *
 * Side effects:
 *	If varName is defined as a local or global variable in interp, it is
 *	deleted.
 *
 *----------------------------------------------------------------------
 */

#undef Tcl_UnsetVar
int
Tcl_UnsetVar(
    Tcl_Interp *interp,		/* Command interpreter in which varName is to
				 * be looked up. */
    const char *varName,	/* Name of a variable in interp. May be either
				 * a scalar name or an array name or an
				 * element in an array. */
    int flags)			/* OR-ed combination of any of
				 * TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY or
				 * TCL_LEAVE_ERR_MSG. */
{
    return Tcl_UnsetVar2(interp, varName, NULL, flags);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_UnsetVar2 --
 *
 *	Delete a variable, given a 2-part name.
 *
 * Results:
 *	Returns TCL_OK if the variable was successfully deleted, TCL_ERROR if
 *	the variable can't be unset. In the event of an error, if the
 *	TCL_LEAVE_ERR_MSG flag is set then an error message is left in the
 *	interp's result.
 *
 * Side effects:
 *	If part1 and part2 indicate a local or global variable in interp, it
 *	is deleted. If part1 is an array name and part2 is NULL, then the
 *	whole array is deleted.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_UnsetVar2(
    Tcl_Interp *interp,		/* Command interpreter in which varName is to
				 * be looked up. */
    const char *part1,		/* Name of variable or array. */
    const char *part2,		/* Name of element within array or NULL. */
    int flags)			/* OR-ed combination of any of
				 * TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_LEAVE_ERR_MSG. */
{
    int result;
    Tcl_Obj *part2Ptr = NULL, *part1Ptr = Tcl_NewStringObj(part1, -1);

    if (part2) {
	part2Ptr = Tcl_NewStringObj(part2, -1);
    }

    /*
     * Filter to pass through only the flags this interface supports.
     */

    flags &= (TCL_GLOBAL_ONLY|TCL_NAMESPACE_ONLY|TCL_LEAVE_ERR_MSG);
    result = TclObjUnsetVar2(interp, part1Ptr, part2Ptr, flags);

    Tcl_DecrRefCount(part1Ptr);
    if (part2Ptr) {
	Tcl_DecrRefCount(part2Ptr);
    }
    return result;
}

/*
 *----------------------------------------------------------------------
 *
 * TclObjUnsetVar2 --
 *
 *	Delete a variable, given a 2-object name.
 *

```

tcl8.5 8.5.19, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclVar.c`, function `Tcl_ArrayObjCmd(`, lines 2791–3471. Retained reference `rust-tcl-registry-tests-data-native_dict_write_error_lifetime226-failed-request225-original-source-tclVar.c`.

```text
int
Tcl_ArrayObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    /*
     * The list of constants below should match the arrayOptions string array
     * below.
     */

    enum {
	ARRAY_ANYMORE, ARRAY_DONESEARCH, ARRAY_EXISTS, ARRAY_GET,
	ARRAY_NAMES, ARRAY_NEXTELEMENT, ARRAY_SET, ARRAY_SIZE,
	ARRAY_STARTSEARCH, ARRAY_STATISTICS, ARRAY_UNSET
    };
    static const char *arrayOptions[] = {
	"anymore", "donesearch", "exists", "get", "names", "nextelement",
	"set", "size", "startsearch", "statistics", "unset", NULL
    };

    Interp *iPtr = (Interp *) interp;
    Var *varPtr, *arrayPtr;
    Tcl_HashEntry *hPtr;
    Tcl_Obj *varNamePtr;
    int notArray;
    int index, result;

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "option arrayName ?arg ...?");
	return TCL_ERROR;
    }

    if (Tcl_GetIndexFromObj(interp, objv[1], arrayOptions, "option",
	    0, &index) != TCL_OK) {
	return TCL_ERROR;
    }

    /*
     * Locate the array variable
     */

    varNamePtr = objv[2];
    varPtr = TclObjLookupVarEx(interp, varNamePtr, NULL, /*flags*/ 0,
	    /*msg*/ 0, /*createPart1*/ 0, /*createPart2*/ 0, &arrayPtr);

    /*
     * Special array trace used to keep the env array in sync for array names,
     * array get, etc.
     */

    if (varPtr && (varPtr->flags & VAR_TRACED_ARRAY)
	    && (TclIsVarArray(varPtr) || TclIsVarUndefined(varPtr))) {
	if (TclObjCallVarTraces(iPtr, arrayPtr, varPtr, varNamePtr, NULL,
		(TCL_LEAVE_ERR_MSG|TCL_NAMESPACE_ONLY|TCL_GLOBAL_ONLY|
		TCL_TRACE_ARRAY), /* leaveErrMsg */ 1, -1) == TCL_ERROR) {
	    return TCL_ERROR;
	}
    }

    /*
     * Verify that it is indeed an array variable. This test comes after the
     * traces - the variable may actually become an array as an effect of said
     * traces.
     */

    notArray = 0;
    if ((varPtr == NULL) || !TclIsVarArray(varPtr)
	    || TclIsVarUndefined(varPtr)) {
	notArray = 1;
    }

    switch (index) {
    case ARRAY_ANYMORE: {
	ArraySearch *searchPtr;

	if (objc != 4) {
	    Tcl_WrongNumArgs(interp, 2, objv, "arrayName searchId");
	    return TCL_ERROR;
	}
	if (notArray) {
	    goto error;
	}
	searchPtr = ParseSearchId(interp, varPtr, varNamePtr, objv[3]);
	if (searchPtr == NULL) {
	    return TCL_ERROR;
	}
	while (1) {
	    if (searchPtr->nextEntry != NULL) {
		Var *varPtr2 = VarHashGetValue(searchPtr->nextEntry);
		if (!TclIsVarUndefined(varPtr2)) {
		    break;
		}
	    }
	    searchPtr->nextEntry = Tcl_NextHashEntry(&searchPtr->search);
	    if (searchPtr->nextEntry == NULL) {
		Tcl_SetObjResult(interp, iPtr->execEnvPtr->constants[0]);
		return TCL_OK;
	    }
	}
	Tcl_SetObjResult(interp, iPtr->execEnvPtr->constants[1]);
	break;
    }
    case ARRAY_DONESEARCH: {
	ArraySearch *searchPtr, *prevPtr;

	if (objc != 4) {
	    Tcl_WrongNumArgs(interp, 2, objv, "arrayName searchId");
	    return TCL_ERROR;
	}
	if (notArray) {
	    goto error;
	}
	searchPtr = ParseSearchId(interp, varPtr, varNamePtr, objv[3]);
	if (searchPtr == NULL) {
	    return TCL_ERROR;
	}
	hPtr = Tcl_FindHashEntry(&iPtr->varSearches,(char *) varPtr);
	if (searchPtr == Tcl_GetHashValue(hPtr)) {
	    if (searchPtr->nextPtr) {
		Tcl_SetHashValue(hPtr, searchPtr->nextPtr);
	    } else {
		varPtr->flags &= ~VAR_SEARCH_ACTIVE;
		Tcl_DeleteHashEntry(hPtr);
	    }
	} else {
	    for (prevPtr=Tcl_GetHashValue(hPtr) ;; prevPtr=prevPtr->nextPtr) {
		if (prevPtr->nextPtr == searchPtr) {
		    prevPtr->nextPtr = searchPtr->nextPtr;
		    break;
		}
	    }
	}
	ckfree((char *) searchPtr);
	break;
    }
    case ARRAY_NEXTELEMENT: {
	ArraySearch *searchPtr;
	Tcl_HashEntry *hPtr;
	Var *varPtr2;

	if (objc != 4) {
	    Tcl_WrongNumArgs(interp, 2, objv, "arrayName searchId");
	    return TCL_ERROR;
	}
	if (notArray) {
	    goto error;
	}
	searchPtr = ParseSearchId(interp, varPtr, varNamePtr, objv[3]);
	if (searchPtr == NULL) {
	    return TCL_ERROR;
	}
	while (1) {
	    hPtr = searchPtr->nextEntry;
	    if (hPtr == NULL) {
		hPtr = Tcl_NextHashEntry(&searchPtr->search);
		if (hPtr == NULL) {
		    return TCL_OK;
		}
	    } else {
		searchPtr->nextEntry = NULL;
	    }
	    varPtr2 = VarHashGetValue(hPtr);
	    if (!TclIsVarUndefined(varPtr2)) {
		break;
	    }
	}
	Tcl_SetObjResult(interp, VarHashGetKey(varPtr2));
	break;
    }
    case ARRAY_STARTSEARCH: {
	ArraySearch *searchPtr;
	int isNew;
	char *varName = TclGetString(varNamePtr);

	if (objc != 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "arrayName");
	    return TCL_ERROR;
	}
	if (notArray) {
	    goto error;
	}
	searchPtr = (ArraySearch *) ckalloc(sizeof(ArraySearch));
	hPtr = Tcl_CreateHashEntry(&iPtr->varSearches,
		(char *) varPtr, &isNew);
	if (isNew) {
	    searchPtr->id = 1;
	    Tcl_AppendResult(interp, "s-1-", varName, NULL);
	    varPtr->flags |= VAR_SEARCH_ACTIVE;
	    searchPtr->nextPtr = NULL;
	} else {
	    char string[TCL_INTEGER_SPACE];

	    searchPtr->id = ((ArraySearch *) Tcl_GetHashValue(hPtr))->id + 1;
	    TclFormatInt(string, searchPtr->id);
	    Tcl_AppendResult(interp, "s-", string, "-", varName, NULL);
	    searchPtr->nextPtr = Tcl_GetHashValue(hPtr);
	}
	searchPtr->varPtr = varPtr;
	searchPtr->nextEntry = VarHashFirstEntry(varPtr->value.tablePtr,
		&searchPtr->search);
	Tcl_SetHashValue(hPtr, searchPtr);
	break;
    }

    case ARRAY_EXISTS:
	if (objc != 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "arrayName");
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, iPtr->execEnvPtr->constants[!notArray]);
	break;
    case ARRAY_GET: {
	Tcl_HashSearch search;
	Var *varPtr2;
	char *pattern = NULL;
	char *name;
	Tcl_Obj *namePtr, *valuePtr, *nameLstPtr, *tmpResPtr, **namePtrPtr;
	int i, count;

	if ((objc != 3) && (objc != 4)) {
	    Tcl_WrongNumArgs(interp, 2, objv, "arrayName ?pattern?");
	    return TCL_ERROR;
	}
	if (notArray) {
	    return TCL_OK;
	}
	if (objc == 4) {
	    pattern = TclGetString(objv[3]);
	}

	/*
	 * Store the array names in a new object.
	 */

	TclNewObj(nameLstPtr);
	Tcl_IncrRefCount(nameLstPtr);
	if ((pattern != NULL) && TclMatchIsTrivial(pattern)) {
	    varPtr2 = VarHashFindVar(varPtr->value.tablePtr, objv[3]);
	    if (varPtr2 == NULL) {
		goto searchDone;
	    }
	    if (TclIsVarUndefined(varPtr2)) {
		goto searchDone;
	    }
	    result = Tcl_ListObjAppendElement(interp, nameLstPtr,
		    VarHashGetKey(varPtr2));
	    if (result != TCL_OK) {
		TclDecrRefCount(nameLstPtr);
		return result;
	    }
	    goto searchDone;
	}
	for (varPtr2 = VarHashFirstVar(varPtr->value.tablePtr, &search);
		varPtr2; varPtr2 = VarHashNextVar(&search)) {
	    if (TclIsVarUndefined(varPtr2)) {
		continue;
	    }
	    namePtr = VarHashGetKey(varPtr2);
	    name = TclGetString(namePtr);
	    if ((objc == 4) && !Tcl_StringMatch(name, pattern)) {
		continue;	/* Element name doesn't match pattern. */
	    }

	    result = Tcl_ListObjAppendElement(interp, nameLstPtr, namePtr);
	    if (result != TCL_OK) {
		TclDecrRefCount(nameLstPtr);
		return result;
	    }
	}

    searchDone:
	/*
	 * Make sure the Var structure of the array is not removed by a trace
	 * while we're working.
	 */

	if (TclIsVarInHash(varPtr)) {
	    VarHashRefCount(varPtr)++;
	}

	/*
	 * Get the array values corresponding to each element name.
	 */

	TclNewObj(tmpResPtr);
	result = Tcl_ListObjGetElements(interp, nameLstPtr, &count,
		&namePtrPtr);
	if (result != TCL_OK) {
	    goto errorInArrayGet;
	}

	for (i=0 ; i<count ; i++) {
	    namePtr = *namePtrPtr++;
	    valuePtr = Tcl_ObjGetVar2(interp, objv[2], namePtr,
		    TCL_LEAVE_ERR_MSG);
	    if (valuePtr == NULL) {
		/*
		 * Some trace played a trick on us; we need to diagnose to
		 * adapt our behaviour: was the array element unset, or did
		 * the modification modify the complete array?
		 */

		if (TclIsVarArray(varPtr)) {
		    /*
		     * The array itself looks OK, the variable was undefined:
		     * forget it.
		     */

		    continue;
		} else {
		    result = TCL_ERROR;
		    goto errorInArrayGet;
		}
	    }
	    result = Tcl_DictObjPut(interp, tmpResPtr, namePtr, valuePtr);
	    if (result != TCL_OK) {
		goto errorInArrayGet;
	    }
	}
	if (TclIsVarInHash(varPtr)) {
	    VarHashRefCount(varPtr)--;
	}
	Tcl_SetObjResult(interp, tmpResPtr);
	TclDecrRefCount(nameLstPtr);
	break;

    errorInArrayGet:
	if (TclIsVarInHash(varPtr)) {
	    VarHashRefCount(varPtr)--;
	}
	TclDecrRefCount(nameLstPtr);
	TclDecrRefCount(tmpResPtr);	/* Free unneeded temp result. */
	return result;
    }
    case ARRAY_NAMES: {
	Tcl_HashSearch search;
	Var *varPtr2;
	char *pattern;
	char *name;
	Tcl_Obj *namePtr, *resultPtr, *patternPtr;
	int mode, matched = 0;
	static const char *options[] = {
	    "-exact", "-glob", "-regexp", NULL
	};
	enum options { OPT_EXACT, OPT_GLOB, OPT_REGEXP };

	mode = OPT_GLOB;

	if ((objc < 3) || (objc > 5)) {
	    Tcl_WrongNumArgs(interp, 2,objv, "arrayName ?mode? ?pattern?");
	    return TCL_ERROR;
	}
	if (notArray) {
	    return TCL_OK;
	}
	if (objc == 4) {
	    patternPtr = objv[3];
	    pattern = TclGetString(patternPtr);
	} else if (objc == 5) {
	    patternPtr = objv[4];
	    pattern = TclGetString(patternPtr);
	    if (Tcl_GetIndexFromObj(interp, objv[3], options, "option", 0,
		    &mode) != TCL_OK) {
		return TCL_ERROR;
	    }
	} else {
	    patternPtr = NULL;
	    pattern = NULL;
	}
	TclNewObj(resultPtr);
	if (((enum options) mode)==OPT_GLOB && pattern!=NULL &&
		TclMatchIsTrivial(pattern)) {
	    varPtr2 = VarHashFindVar(varPtr->value.tablePtr, patternPtr);
	    if ((varPtr2 != NULL) && !TclIsVarUndefined(varPtr2)) {
		result = Tcl_ListObjAppendElement(interp, resultPtr,
			VarHashGetKey(varPtr2));
		if (result != TCL_OK) {
		    TclDecrRefCount(resultPtr);
		    return result;
		}
	    }
	    Tcl_SetObjResult(interp, resultPtr);
	    return TCL_OK;
	}
	for (varPtr2=VarHashFirstVar(varPtr->value.tablePtr, &search);
		varPtr2!=NULL ; varPtr2=VarHashNextVar(&search)) {
	    if (TclIsVarUndefined(varPtr2)) {
		continue;
	    }
	    namePtr = VarHashGetKey(varPtr2);
	    name = TclGetString(namePtr);
	    if (objc > 3) {
		switch ((enum options) mode) {
		case OPT_EXACT:
		    matched = (strcmp(name, pattern) == 0);
		    break;
		case OPT_GLOB:
		    matched = Tcl_StringMatch(name, pattern);
		    break;
		case OPT_REGEXP:
		    matched = Tcl_RegExpMatch(interp, name, pattern);
		    if (matched < 0) {
			TclDecrRefCount(resultPtr);
			return TCL_ERROR;
		    }
		    break;
		}
		if (matched == 0) {
		    continue;
		}
	    }

	    result = Tcl_ListObjAppendElement(interp, resultPtr, namePtr);
	    if (result != TCL_OK) {
		TclDecrRefCount(namePtr);	/* Free unneeded name obj. */
		return result;
	    }
	}
	Tcl_SetObjResult(interp, resultPtr);
	break;
    }
    case ARRAY_SET:
	if (objc != 4) {
	    Tcl_WrongNumArgs(interp, 2, objv, "arrayName list");
	    return TCL_ERROR;
	}
	return TclArraySet(interp, objv[2], objv[3]);
    case ARRAY_UNSET:
	if ((objc != 3) && (objc != 4)) {
	    Tcl_WrongNumArgs(interp, 2, objv, "arrayName ?pattern?");
	    return TCL_ERROR;
	}
	if (notArray) {
	    return TCL_OK;
	}
	if (objc == 3) {
	    /*
	     * When no pattern is given, just unset the whole array.
	     */

	    return TclObjUnsetVar2(interp, varNamePtr, NULL, 0);
	} else {
	    Tcl_HashSearch search;
	    Var *varPtr2, *protectedVarPtr;
	    const char *pattern = TclGetString(objv[3]);

	    /*
	     * With a trivial pattern, we can just unset.
	     */

	    if (TclMatchIsTrivial(pattern)) {
		varPtr2 = VarHashFindVar(varPtr->value.tablePtr, objv[3]);
		if (varPtr2 != NULL && !TclIsVarUndefined(varPtr2)) {
		    return TclObjUnsetVar2(interp, varNamePtr, objv[3], 0);
		}
		return TCL_OK;
	    }

	    /*
	     * Non-trivial case (well, deeply tricky really). We peek inside
	     * the hash iterator in order to allow us to guarantee that the
	     * following element in the array will not be scrubbed until we
	     * have dealt with it. This stops the overall iterator from ending
	     * up pointing into deallocated memory. [Bug 2939073]
	     */

	    protectedVarPtr = NULL;
	    for (varPtr2=VarHashFirstVar(varPtr->value.tablePtr, &search);
		    varPtr2!=NULL ; varPtr2=VarHashNextVar(&search)) {
		/*
		 * Drop the extra ref immediately. We don't need to free it at
		 * this point though; we'll be unsetting it if necessary soon.
		 */

		if (varPtr2 == protectedVarPtr) {
		    VarHashRefCount(varPtr2)--;
		}

		/*
		 * Guard the next item in the search chain by incrementing its
		 * refcount. This guarantees that the hash table iterator
		 * won't be dangling on the next time through the loop.
		 */

		if (search.nextEntryPtr != NULL) {
		    protectedVarPtr = VarHashGetValue(search.nextEntryPtr);
		    VarHashRefCount(protectedVarPtr)++;
		} else {
		    protectedVarPtr = NULL;
		}

		if (!TclIsVarUndefined(varPtr2)) {
		    Tcl_Obj *namePtr = VarHashGetKey(varPtr2);

		    if (Tcl_StringMatch(TclGetString(namePtr), pattern)
			    && TclObjUnsetVar2(interp, varNamePtr, namePtr,
				    0) != TCL_OK) {
			/*
			 * If we incremented a refcount, we must decrement it
			 * here as we will not be coming back properly due to
			 * the error.
			 */

			if (protectedVarPtr) {
			    VarHashRefCount(protectedVarPtr)--;
			    CleanupVar(protectedVarPtr, varPtr);
			}
			return TCL_ERROR;
		    }
		} else {
		    CleanupVar(varPtr2, varPtr);
		}
	    }
	    break;
	}

    case ARRAY_SIZE: {
	Tcl_HashSearch search;
	int size;

	if (objc != 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "arrayName");
	    return TCL_ERROR;
	}
	size = 0;

	/*
	 * Must iterate in order to get chance to check for present but
	 * "undefined" entries.
	 */

	if (!notArray) {
	    Var *varPtr2;

	    for (varPtr2=VarHashFirstVar(varPtr->value.tablePtr, &search);
		    varPtr2!=NULL ; varPtr2=VarHashNextVar(&search)) {
		if (TclIsVarUndefined(varPtr2)) {
		    continue;
		}
		size++;
	    }
	}
	Tcl_SetObjResult(interp, Tcl_NewIntObj(size));
	break;
    }

    case ARRAY_STATISTICS: {
	const char *stats;

	if (notArray) {
	    goto error;
	}

	stats = Tcl_HashStats((Tcl_HashTable *) varPtr->value.tablePtr);
	if (stats != NULL) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(stats, -1));
	    ckfree((void *)stats);
	} else {
	    Tcl_SetResult(interp,"error reading array statistics",TCL_STATIC);
	    return TCL_ERROR;
	}
	break;
    }
    }
    return TCL_OK;

  error:
    Tcl_AppendResult(interp, "\"", TclGetString(varNamePtr),
	    "\" isn't an array", NULL);
    return TCL_ERROR;
}

/*
 *----------------------------------------------------------------------
 *
 * TclArraySet --
 *
 *	Set the elements of an array. If there are no elements to set, create
 *	an empty array. This routine is used by the Tcl_ArrayObjCmd and by the
 *	TclSetupEnv routine.
 *
 * Results:
 *	A standard Tcl result object.
 *
 * Side effects:
 *	A variable will be created if one does not already exist.
 *	Callers must Incr arrayNameObj if they pland to Decr it.
 *
 *----------------------------------------------------------------------
 */

int
TclArraySet(
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Obj *arrayNameObj,	/* The array name. */
    Tcl_Obj *arrayElemObj)	/* The array elements list or dict. If this is
				 * NULL, create an empty array. */
{
    Var *varPtr, *arrayPtr;
    int result, i;

    varPtr = TclObjLookupVarEx(interp, arrayNameObj, NULL,
	    /*flags*/ TCL_LEAVE_ERR_MSG, /*msg*/ "set", /*createPart1*/ 1,
	    /*createPart2*/ 1, &arrayPtr);
    if (varPtr == NULL) {
	return TCL_ERROR;
    }
    if (arrayPtr) {
	CleanupVar(varPtr, arrayPtr);
	TclObjVarErrMsg(interp, arrayNameObj, NULL, "set", needArray, -1);
	return TCL_ERROR;
    }

    if (arrayElemObj == NULL) {
	goto ensureArray;
    }

    /*
     * Install the contents of the dictionary or list into the array.
     */

    if (arrayElemObj->typePtr == &tclDictType) {
	Tcl_Obj *keyPtr, *valuePtr;
	Tcl_DictSearch search;
	int done;

	if (Tcl_DictObjSize(interp, arrayElemObj, &done) != TCL_OK) {
	    return TCL_ERROR;
	}
	if (done == 0) {
	    /*
	     * Empty, so we'll just force the array to be properly existing
	     * instead.
	     */

	    goto ensureArray;
	}

	/*
	 * Don't need to look at result of Tcl_DictObjFirst as we've just
	 * successfully used a dictionary operation on the same object.
	 */

	for (Tcl_DictObjFirst(interp, arrayElemObj, &search,
		&keyPtr, &valuePtr, &done) ; !done ;
		Tcl_DictObjNext(&search, &keyPtr, &valuePtr, &done)) {
	    /*
	     * At this point, it would be nice if the key was directly usable
	     * by the array. This isn't the case though.
	     */

	    Var *elemVarPtr = TclLookupArrayElement(interp, arrayNameObj,
		    keyPtr, TCL_LEAVE_ERR_MSG, "set", 1, 1, varPtr, -1);

	    if ((elemVarPtr == NULL) ||
		    (TclPtrSetVar(interp, elemVarPtr, varPtr, arrayNameObj,
		    keyPtr, valuePtr, TCL_LEAVE_ERR_MSG, -1) == NULL)) {
		Tcl_DictObjDone(&search);
		return TCL_ERROR;
	    }
	}
	return TCL_OK;
    } else {
	/*
	 * Not a dictionary, so assume (and convert to, for backward-
	 * -compatability reasons) a list.
	 */

	int elemLen;
	Tcl_Obj **elemPtrs, *copyListObj;

	result = TclListObjGetElements(interp, arrayElemObj,
		&elemLen, &elemPtrs);
	if (result != TCL_OK) {
	    return result;
	}
	if (elemLen & 1) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "list must have an even number of elements", -1));

```

tcl8.5 8.5.19, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclVar.c`, function `CleanupVar(`, lines 316–446. Retained reference `rust-tcl-registry-tests-data-native_dict_write_error_lifetime226-failed-request225-original-source-tclVar.c`.

```text
static inline void
CleanupVar(
    Var *varPtr,		/* Pointer to variable that may be a candidate
				 * for being expunged. */
    Var *arrayPtr)		/* Array that contains the variable, or NULL
				 * if this variable isn't an array element. */
{
    if (TclIsVarUndefined(varPtr) && TclIsVarInHash(varPtr)
	    && !TclIsVarTraced(varPtr)
	    && (VarHashRefCount(varPtr) == !TclIsVarDeadHash(varPtr))) {
	if (VarHashRefCount(varPtr) == 0) {
	    ckfree((char *) varPtr);
	} else {
	    VarHashDeleteEntry(varPtr);
	}
    }
    if (arrayPtr != NULL && TclIsVarUndefined(arrayPtr) &&
	    TclIsVarInHash(arrayPtr) && !TclIsVarTraced(arrayPtr) &&
	    (VarHashRefCount(arrayPtr) == !TclIsVarDeadHash(arrayPtr))) {
	if (VarHashRefCount(arrayPtr) == 0) {
	    ckfree((char *) arrayPtr);
	} else {
	    VarHashDeleteEntry(arrayPtr);
	}
    }
}

void
TclCleanupVar(
    Var *varPtr,		/* Pointer to variable that may be a candidate
				 * for being expunged. */
    Var *arrayPtr)		/* Array that contains the variable, or NULL
				 * if this variable isn't an array element. */
{
    CleanupVar(varPtr, arrayPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * TclLookupVar --
 *
 *	This function is used to locate a variable given its name(s). It has
 *	been mostly superseded by TclObjLookupVar, it is now only used by the
 *	trace code. It is kept in tcl8.5 mainly because it is in the internal
 *	stubs table, so that some extension may be calling it.
 *
 * Results:
 *	The return value is a pointer to the variable structure indicated by
 *	part1 and part2, or NULL if the variable couldn't be found. If the
 *	variable is found, *arrayPtrPtr is filled in with the address of the
 *	variable structure for the array that contains the variable (or NULL
 *	if the variable is a scalar). If the variable can't be found and
 *	either createPart1 or createPart2 are 1, a new as-yet-undefined
 *	(VAR_UNDEFINED) variable structure is created, entered into a hash
 *	table, and returned.
 *
 *	If the variable isn't found and creation wasn't specified, or some
 *	other error occurs, NULL is returned and an error message is left in
 *	the interp's result if TCL_LEAVE_ERR_MSG is set in flags.
 *
 *	Note: it's possible for the variable returned to be VAR_UNDEFINED even
 *	if createPart1 or createPart2 are 1 (these only cause the hash table
 *	entry or array to be created). For example, the variable might be a
 *	global that has been unset but is still referenced by a procedure, or
 *	a variable that has been unset but it only being kept in existence (if
 *	VAR_UNDEFINED) by a trace.
 *
 * Side effects:
 *	New hashtable entries may be created if createPart1 or createPart2
 *	are 1.
 *
 *----------------------------------------------------------------------
 */

Var *
TclLookupVar(
    Tcl_Interp *interp,		/* Interpreter to use for lookup. */
    const char *part1,		/* If part2 isn't NULL, this is the name of an
				 * array. Otherwise, this is a full variable
				 * name that could include a parenthesized
				 * array element. */
    const char *part2,		/* Name of element within array, or NULL. */
    int flags,			/* Only TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * and TCL_LEAVE_ERR_MSG bits matter. */
    const char *msg,		/* Verb to use in error messages, e.g. "read"
				 * or "set". Only needed if TCL_LEAVE_ERR_MSG
				 * is set in flags. */
    int createPart1,		/* If 1, create hash table entry for part 1 of
				 * name, if it doesn't already exist. If 0,
				 * return error if it doesn't exist. */
    int createPart2,		/* If 1, create hash table entry for part 2 of
				 * name, if it doesn't already exist. If 0,
				 * return error if it doesn't exist. */
    Var **arrayPtrPtr)		/* If the name refers to an element of an
				 * array, *arrayPtrPtr gets filled in with
				 * address of array variable. Otherwise this
				 * is set to NULL. */
{
    Var *varPtr;
    Tcl_Obj *part1Ptr = Tcl_NewStringObj(part1, -1);

    if (createPart1) {
	Tcl_IncrRefCount(part1Ptr);
    }

    varPtr = TclObjLookupVar(interp, part1Ptr, part2, flags, msg,
	    createPart1, createPart2, arrayPtrPtr);

    TclDecrRefCount(part1Ptr);
    return varPtr;
}

/*
 *----------------------------------------------------------------------
 *
 * TclObjLookupVar, TclObjLookupVarEx --
 *
 *	This function is used by virtually all of the variable code to locate
 *	a variable given its name(s). The parsing into array/element
 *	components and (if possible) the lookup results are cached in
 *	part1Ptr, which is converted to one of the varNameTypes.
 *
 * Results:
 *	The return value is a pointer to the variable structure indicated by
 *	part1Ptr and part2, or NULL if the variable couldn't be found. If *
 *	the variable is found, *arrayPtrPtr is filled with the address of the
 *	variable structure for the array that contains the variable (or NULL
 *	if the variable is a scalar). If the variable can't be found and
 *	either createPart1 or createPart2 are 1, a new as-yet-undefined
 *	(VAR_UNDEFINED) variable structure is created, entered into a hash

```

tcl8.5 8.5.19, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclVar.c`, function `Tcl_UnsetVar2(`, lines 2195–2315. Retained reference `rust-tcl-registry-tests-data-native_dict_write_error_lifetime226-failed-request225-original-source-tclVar.c`.

```text
int
Tcl_UnsetVar2(
    Tcl_Interp *interp,		/* Command interpreter in which varName is to
				 * be looked up. */
    const char *part1,		/* Name of variable or array. */
    const char *part2,		/* Name of element within array or NULL. */
    int flags)			/* OR-ed combination of any of
				 * TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_LEAVE_ERR_MSG. */
{
    int result;
    Tcl_Obj *part2Ptr = NULL, *part1Ptr = Tcl_NewStringObj(part1, -1);

    if (part2) {
	part2Ptr = Tcl_NewStringObj(part2, -1);
    }

    /*
     * Filter to pass through only the flags this interface supports.
     */

    flags &= (TCL_GLOBAL_ONLY|TCL_NAMESPACE_ONLY|TCL_LEAVE_ERR_MSG);
    result = TclObjUnsetVar2(interp, part1Ptr, part2Ptr, flags);

    Tcl_DecrRefCount(part1Ptr);
    if (part2Ptr) {
	Tcl_DecrRefCount(part2Ptr);
    }
    return result;
}

/*
 *----------------------------------------------------------------------
 *
 * TclObjUnsetVar2 --
 *
 *	Delete a variable, given a 2-object name.
 *
 * Results:
 *	Returns TCL_OK if the variable was successfully deleted, TCL_ERROR if
 *	the variable can't be unset. In the event of an error, if the
 *	TCL_LEAVE_ERR_MSG flag is set then an error message is left in the
 *	interp's result.
 *
 * Side effects:
 *	If part1ptr and part2Ptr indicate a local or global variable in
 *	interp, it is deleted. If part1Ptr is an array name and part2Ptr is
 *	NULL, then the whole array is deleted.
 *
 *----------------------------------------------------------------------
 */

int
TclObjUnsetVar2(
    Tcl_Interp *interp,		/* Command interpreter in which varName is to
				 * be looked up. */
    Tcl_Obj *part1Ptr,		/* Name of variable or array. */
    Tcl_Obj *part2Ptr,		/* Name of element within array or NULL. */
    int flags)			/* OR-ed combination of any of
				 * TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_LEAVE_ERR_MSG. */
{
    Var *varPtr;
    Interp *iPtr = (Interp *) interp;
    Var *arrayPtr;
    int result;

    varPtr = TclObjLookupVarEx(interp, part1Ptr, part2Ptr, flags, "unset",
	    /*createPart1*/ 0, /*createPart2*/ 0, &arrayPtr);
    if (varPtr == NULL) {
	return TCL_ERROR;
    }

    result = (TclIsVarUndefined(varPtr)? TCL_ERROR : TCL_OK);

    /*
     * Keep the variable alive until we're done with it. We used to
     * increase/decrease the refCount for each operation, making it hard to
     * find [Bug 735335] - caused by unsetting the variable whose value was
     * the variable's name.
     */

    if (TclIsVarInHash(varPtr)) {
	VarHashRefCount(varPtr)++;
    }

    UnsetVarStruct(varPtr, arrayPtr, iPtr, part1Ptr, part2Ptr, flags);

    /*
     * It's an error to unset an undefined variable.
     */

    if (result != TCL_OK) {
	if (flags & TCL_LEAVE_ERR_MSG) {
	    TclObjVarErrMsg(interp, part1Ptr, part2Ptr, "unset",
		    ((arrayPtr == NULL) ? noSuchVar : noSuchElement), -1);
	}
    }

#if ENABLE_NS_VARNAME_CACHING
    /*
     * Try to avoid keeping the Var struct allocated due to a tclNsVarNameType
     * keeping a reference. This removes some additional exteriorisations of
     * [Bug 736729], but may be a good thing independently of the bug.
     */

    if (part1Ptr->typePtr == &tclNsVarNameType) {
	TclFreeIntRep(part1Ptr);
	part1Ptr->typePtr = NULL;
    }
#endif

    /*
     * Finally, if the variable is truly not in use then free up its Var
     * structure and remove it from its hash table, if any. The ref count of
     * its value object, if any, was decremented above.
     */

    if (TclIsVarInHash(varPtr)) {
	VarHashRefCount(varPtr)--;
	CleanupVar(varPtr, arrayPtr);

```

tcl8.5 8.5.19, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclVar.c`, function `Tcl_ObjSetVar2(`, lines 1726–1831. Retained reference `rust-tcl-registry-tests-data-native_dict_write_error_lifetime226-failed-request225-original-source-tclVar.c`.

```text
Tcl_Obj *
Tcl_ObjSetVar2(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    register Tcl_Obj *part1Ptr,	/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    register Tcl_Obj *part2Ptr,	/* If non-NULL, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *newValuePtr,	/* New value for variable. */
    int flags)			/* Various flags that tell how to set value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT, or
				 * TCL_LEAVE_ERR_MSG. */
{
    Var *varPtr, *arrayPtr;

    /*
     * Filter to pass through only the flags this interface supports.
     */

    flags &= (TCL_GLOBAL_ONLY|TCL_NAMESPACE_ONLY|TCL_LEAVE_ERR_MSG
	    |TCL_APPEND_VALUE|TCL_LIST_ELEMENT);
    varPtr = TclObjLookupVarEx(interp, part1Ptr, part2Ptr, flags, "set",
	    /*createPart1*/ 1, /*createPart2*/ 1, &arrayPtr);
    if (varPtr == NULL) {
	if (newValuePtr->refCount == 0) {
	    Tcl_DecrRefCount(newValuePtr);
	}
	return NULL;
    }

    return TclPtrSetVar(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    newValuePtr, flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * TclPtrSetVar --
 *
 *	This function is the same as Tcl_SetVar2Ex above, except that it
 *	requires pointers to the variable's Var structs in addition to the
 *	variable names.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the write operation was disallowed because an array was
 *	expected but not found (or vice versa), then NULL is returned; if the
 *	TCL_LEAVE_ERR_MSG flag is set, then an explanatory message will be
 *	left in the interpreter's result. Note that the returned object may
 *	not be the same one referenced by newValuePtr; this is because
 *	variable traces may modify the variable's value.
 *
 * Side effects:
 *	The value of the given variable is set. If either the array or the
 *	entry didn't exist then a new variable is created.
 *
 *----------------------------------------------------------------------
 */

Tcl_Obj *
TclPtrSetVar(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be looked up. */
    register Var *varPtr,	/* Reference to the variable to set. */
    Var *arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Name of an array (if part2 is non-NULL) or
				 * the name of a variable. NULL if the 'index'
				 * parameter is >= 0 */
    Tcl_Obj *part2Ptr,		/* If non-NULL, gives the name of an element
				 * in the array part1. */
    Tcl_Obj *newValuePtr,	/* New value for variable. */
    const int flags,		/* OR-ed combination of TCL_GLOBAL_ONLY, and
				 * TCL_LEAVE_ERR_MSG bits. */
    int index)			/* Index of local var where part1 is to be
				 * found. */
{
    Interp *iPtr = (Interp *) interp;
    Tcl_Obj *oldValuePtr;
    Tcl_Obj *resultPtr = NULL;
    int result;

    /*
     * If the variable is in a hashtable and its hPtr field is NULL, then we
     * may have an upvar to an array element where the array was deleted or an
     * upvar to a namespace variable whose namespace was deleted. Generate an
     * error (allowing the variable to be reset would screw up our storage
     * allocation and is meaningless anyway).
     */

    if (TclIsVarDeadHash(varPtr)) {
	if (flags & TCL_LEAVE_ERR_MSG) {
	    if (TclIsVarArrayElement(varPtr)) {
		TclObjVarErrMsg(interp, part1Ptr, part2Ptr, "set",
			danglingElement, index);
	    } else {
		TclObjVarErrMsg(interp, part1Ptr, part2Ptr, "set",
			danglingVar, index);
	    }
	}
	goto earlyError;
    }

```

tcl8.5 8.5.19, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclBasic.c`, function `Tcl_EvalObjv(`, lines 3915–4025. Retained reference `rust-tcl-registry-tests-data-native_info_commands_literal_original-original-inputs-8.5.19-generic-tclBasic.c`.

```text
int
Tcl_EvalObjv(
    Tcl_Interp *interp,		/* Interpreter in which to evaluate the
				 * command. Also used for error reporting. */
    int objc,			/* Number of words in command. */
    Tcl_Obj *const objv[],	/* An array of pointers to objects that are
				 * the words that make up the command. */
    int flags)			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Only
				 * TCL_EVAL_GLOBAL and TCL_EVAL_INVOKE are
				 * currently supported. */
{
    Interp *iPtr = (Interp *) interp;
    int code = TCL_OK;
    int allowExceptions = (iPtr->evalFlags & TCL_ALLOW_EXCEPTIONS);

    iPtr->numLevels++;
    code = TclEvalObjvInternal(interp, objc, objv, NULL, 0, flags);
    iPtr->numLevels--;

    if (code == TCL_OK) {
	return code;
    } else {

	/*
	 * If we are again at the top level, process any unusual return code
	 * returned by the evaluated code.
	 */

	if (iPtr->numLevels == 0) {
	    if (code == TCL_RETURN) {
		code = TclUpdateReturnInfo(iPtr);
	    }
	    if ((code != TCL_ERROR) && !allowExceptions) {
		ProcessUnexpectedResult(interp, code);
		code = TCL_ERROR;
	    }
	}

	if ((code == TCL_ERROR) && !(flags & TCL_EVAL_INVOKE)) {
	    /*
	     * If there was an error, a command string will be needed for the
	     * error log: generate it now. Do not worry too much about doing
	     * it expensively.
	     */

	    Tcl_Obj *listPtr;
	    char *cmdString;
	    int cmdLen;

	    listPtr = Tcl_NewListObj(objc, objv);
	    cmdString = Tcl_GetStringFromObj(listPtr, &cmdLen);
	    Tcl_LogCommandInfo(interp, cmdString, cmdString, cmdLen);
	    Tcl_DecrRefCount(listPtr);
	}

	return code;
    }
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_EvalTokensStandard --
 *
 *	Given an array of tokens parsed from a Tcl command (e.g., the tokens
 *	that make up a word or the index for an array variable) this function
 *	evaluates the tokens and concatenates their values to form a single
 *	result value.
 *
 * Results:
 *	The return value is a standard Tcl completion code such as TCL_OK or
 *	TCL_ERROR. A result or error message is left in interp's result.
 *
 * Side effects:
 *	Depends on the array of tokens being evaled.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_EvalTokensStandard(
    Tcl_Interp *interp,		/* Interpreter in which to lookup variables,
				 * execute nested commands, and report
				 * errors. */
    Tcl_Token *tokenPtr,	/* Pointer to first in an array of tokens to
				 * evaluate and concatenate. */
    int count)			/* Number of tokens to consider at tokenPtr.
				 * Must be at least 1. */
{
    return TclSubstTokens(interp, tokenPtr, count, /* numLeftPtr */ NULL, 1,
			  NULL, NULL);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_EvalTokens --
 *
 *	Given an array of tokens parsed from a Tcl command (e.g., the tokens
 *	that make up a word or the index for an array variable) this function
 *	evaluates the tokens and concatenates their values to form a single
 *	result value.
 *
 * Results:
 *	The return value is a pointer to a newly allocated Tcl_Obj containing
 *	the value of the array of tokens. The reference count of the returned
 *	object has been incremented. If an error occurs in evaluating the
 *	tokens then a NULL value is returned and an error message is left in
 *	interp's result.
 *

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdIL.c`, function `InfoDefaultCmd(`, lines 958–1070. Retained reference `rust-tcl-registry-tests-data-native_info_inventory_original-sources-8.6.18-tclCmdIL.c`.

```text
static int
InfoDefaultCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Interp *iPtr = (Interp *) interp;
    const char *procName, *argName;
    Proc *procPtr;
    CompiledLocal *localPtr;
    Tcl_Obj *valueObjPtr;

    if (objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "procname arg varname");
	return TCL_ERROR;
    }

    procName = TclGetString(objv[1]);
    argName = TclGetString(objv[2]);

    procPtr = TclFindProc(iPtr, procName);
    if (procPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"\"%s\" isn't a procedure", procName));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "PROCEDURE", procName,
		(char *)NULL);
	return TCL_ERROR;
    }

    for (localPtr = procPtr->firstLocalPtr;  localPtr != NULL;
	    localPtr = localPtr->nextPtr) {
	if (TclIsVarArgument(localPtr)
		&& (strcmp(argName, localPtr->name) == 0)) {
	    if (localPtr->defValuePtr != NULL) {
		valueObjPtr = Tcl_ObjSetVar2(interp, objv[3], NULL,
			localPtr->defValuePtr, TCL_LEAVE_ERR_MSG);
		if (valueObjPtr == NULL) {
		    return TCL_ERROR;
		}
		Tcl_SetObjResult(interp, Tcl_NewBooleanObj(1));
	    } else {
		Tcl_Obj *nullObjPtr;

		TclNewObj(nullObjPtr);
		valueObjPtr = Tcl_ObjSetVar2(interp, objv[3], NULL,
			nullObjPtr, TCL_LEAVE_ERR_MSG);
		if (valueObjPtr == NULL) {
		    return TCL_ERROR;
		}
		Tcl_SetObjResult(interp, Tcl_NewBooleanObj(0));
	    }
	    return TCL_OK;
	}
    }

    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
	    "procedure \"%s\" doesn't have an argument \"%s\"",
	    procName, argName));
    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "ARGUMENT", argName, (char *)NULL);
    return TCL_ERROR;
}

/*
 *----------------------------------------------------------------------
 *
 * InfoErrorStackCmd --
 *
 *	Called to implement the "info errorstack" command that returns information
 *	about the last error's call stack. Handles the following syntax:
 *
 *	    info errorstack ?interp?
 *
 * Results:
 *	Returns TCL_OK if successful and TCL_ERROR if there is an error.
 *
 * Side effects:
 *	Returns a result in the interpreter's result object. If there is an
 *	error, the result is an error message.
 *
 *----------------------------------------------------------------------
 */

static int
InfoErrorStackCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Interp *target;
    Interp *iPtr;

    if ((objc != 1) && (objc != 2)) {
	Tcl_WrongNumArgs(interp, 1, objv, "?interp?");
	return TCL_ERROR;
    }

    target = interp;
    if (objc == 2) {
	target = Tcl_GetChild(interp, TclGetString(objv[1]));
	if (target == NULL) {
	    return TCL_ERROR;
	}
    }

    iPtr = (Interp *) target;
    Tcl_SetObjResult(interp, iPtr->errorStack);

    return TCL_OK;
}

/*

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdIL.c`, function `Tcl_IncrObjCmd(`, lines 387–507. Retained reference `rust-tcl-registry-tests-data-native_info_inventory_original-sources-8.6.18-tclCmdIL.c`.

```text
int
Tcl_IncrObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *newValuePtr, *incrPtr;

    if ((objc != 2) && (objc != 3)) {
	Tcl_WrongNumArgs(interp, 1, objv, "varName ?increment?");
	return TCL_ERROR;
    }

    if (objc == 3) {
	incrPtr = objv[2];
    } else {
	TclNewIntObj(incrPtr, 1);
    }
    Tcl_IncrRefCount(incrPtr);
    newValuePtr = TclIncrObjVar2(interp, objv[1], NULL,
	    incrPtr, TCL_LEAVE_ERR_MSG);
    Tcl_DecrRefCount(incrPtr);

    if (newValuePtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * Set the interpreter's object result to refer to the variable's new
     * value object.
     */

    Tcl_SetObjResult(interp, newValuePtr);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * TclInitInfoCmd --
 *
 *	This function is called to create the "info" Tcl command. See the user
 *	documentation for details on what it does.
 *
 * Results:
 *	Handle for the info command, or NULL on failure.
 *
 * Side effects:
 *	none
 *
 *----------------------------------------------------------------------
 */

Tcl_Command
TclInitInfoCmd(
    Tcl_Interp *interp)		/* Current interpreter. */
{
    return TclMakeEnsemble(interp, "info", defaultInfoMap);
}

/*
 *----------------------------------------------------------------------
 *
 * InfoArgsCmd --
 *
 *	Called to implement the "info args" command that returns the argument
 *	list for a procedure. Handles the following syntax:
 *
 *	    info args procName
 *
 * Results:
 *	Returns TCL_OK if successful and TCL_ERROR if there is an error.
 *
 * Side effects:
 *	Returns a result in the interpreter's result object. If there is an
 *	error, the result is an error message.
 *
 *----------------------------------------------------------------------
 */

static int
InfoArgsCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Interp *iPtr = (Interp *) interp;
    const char *name;
    Proc *procPtr;
    CompiledLocal *localPtr;
    Tcl_Obj *listObjPtr;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "procname");
	return TCL_ERROR;
    }

    name = TclGetString(objv[1]);
    procPtr = TclFindProc(iPtr, name);
    if (procPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"\"%s\" isn't a procedure", name));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "PROCEDURE", name, (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Build a return list containing the arguments.
     */

    listObjPtr = Tcl_NewListObj(0, NULL);
    for (localPtr = procPtr->firstLocalPtr;  localPtr != NULL;
	    localPtr = localPtr->nextPtr) {
	if (TclIsVarArgument(localPtr)) {
	    Tcl_ListObjAppendElement(interp, listObjPtr,
		    Tcl_NewStringObj(localPtr->name, -1));
	}
    }
    Tcl_SetObjResult(interp, listObjPtr);

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclVar.c`, function `Tcl_UnsetObjCmd(`, lines 2709–2794. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-8.6.18-original-tclVar.c`.

```text
int
Tcl_UnsetObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int i, flags = TCL_LEAVE_ERR_MSG;
    const char *name;

    if (objc == 1) {
	/*
	 * Do nothing if no arguments supplied, so as to match command
	 * documentation.
	 */

	return TCL_OK;
    }

    /*
     * Simple, restrictive argument parsing. The only options are -- and
     * -nocomplain (which must come first and be given exactly to be an
     * option).
     */

    i = 1;
    name = TclGetString(objv[i]);
    if (name[0] == '-') {
	if (strcmp("-nocomplain", name) == 0) {
	    i++;
	    if (i == objc) {
		return TCL_OK;
	    }
	    flags = 0;
	    name = TclGetString(objv[i]);
	}
	if (strcmp("--", name) == 0) {
	    i++;
	}
    }

    for (; i < objc; i++) {
	if ((TclObjUnsetVar2(interp, objv[i], NULL, flags) != TCL_OK)
		&& (flags == TCL_LEAVE_ERR_MSG)) {
	    return TCL_ERROR;
	}
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_AppendObjCmd --
 *
 *	This object-based function is invoked to process the "append" Tcl
 *	command. See the user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl object result value.
 *
 * Side effects:
 *	A variable's value may be changed.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_AppendObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Var *varPtr, *arrayPtr;
    Tcl_Obj *varValuePtr = NULL;
				/* Initialized to avoid compiler warning. */
    int i;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "varName ?value ...?");
	return TCL_ERROR;
    }

    if (objc == 2) {
	varValuePtr = Tcl_ObjGetVar2(interp, objv[1], NULL,TCL_LEAVE_ERR_MSG);

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclVar.c`, function `TclIncrObjVar2(`, lines 2070–2170. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-8.6.18-original-tclVar.c`.

```text
Tcl_Obj *
TclIncrObjVar2(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Tcl_Obj *part1Ptr,		/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,		/* If non-null, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *incrPtr,		/* Amount to be added to variable. */
    int flags)			/* Various flags that tell how to incr value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
{
    Var *varPtr, *arrayPtr;

    varPtr = TclObjLookupVarEx(interp, part1Ptr, part2Ptr, flags, "read",
	    1, 1, &arrayPtr);
    if (varPtr == NULL) {
	Tcl_AddErrorInfo(interp,
		"\n    (reading value of variable to increment)");
	return NULL;
    }
    return TclPtrIncrObjVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    incrPtr, flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * TclPtrIncrObjVar --
 *
 *	Given the pointers to a variable and possible containing array,
 *	increment the Tcl object value of the variable by a Tcl_Obj increment.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the specified variable doesn't exist, or there is a clash
 *	in array usage, or an error occurs while executing variable traces,
 *	then NULL is returned and a message will be left in the interpreter's
 *	result.
 *
 * Side effects:
 *	The value of the given variable is incremented by the specified
 *	amount. If either the array or the entry didn't exist then a new
 *	variable is created. The ref count for the returned object is _not_
 *	incremented to reflect the returned reference; if you want to keep a
 *	reference to the object you must increment its ref count yourself.
 *
 *----------------------------------------------------------------------
 */

Tcl_Obj *
TclPtrIncrObjVar(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Tcl_Var varPtr,		/* Reference to the variable to set. */
    Tcl_Var arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,		/* If non-null, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *incrPtr,		/* Increment value. */
/* TODO: Which of these flag values really make sense? */
    int flags)		/* Various flags that tell how to incr value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
{
    if (varPtr == NULL) {
	Tcl_Panic("varPtr must not be NULL");
    }
    if (part1Ptr == NULL) {
	Tcl_Panic("part1Ptr must not be NULL");
    }
    return TclPtrIncrObjVarIdx(interp, (Var *) varPtr, (Var *) arrayPtr,
	    part1Ptr, part2Ptr, incrPtr, flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * TclPtrIncrObjVarIdx --
 *
 *	Given the pointers to a variable and possible containing array,
 *	increment the Tcl object value of the variable by a Tcl_Obj increment.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the specified variable doesn't exist, or there is a clash
 *	in array usage, or an error occurs while executing variable traces,
 *	then NULL is returned and a message will be left in the interpreter's
 *	result.
 *
 * Side effects:

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclVar.c`, function `TclPtrIncrObjVar(`, lines 2124–2284. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-8.6.18-original-tclVar.c`.

```text
Tcl_Obj *
TclPtrIncrObjVar(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Tcl_Var varPtr,		/* Reference to the variable to set. */
    Tcl_Var arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,		/* If non-null, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *incrPtr,		/* Increment value. */
/* TODO: Which of these flag values really make sense? */
    int flags)		/* Various flags that tell how to incr value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
{
    if (varPtr == NULL) {
	Tcl_Panic("varPtr must not be NULL");
    }
    if (part1Ptr == NULL) {
	Tcl_Panic("part1Ptr must not be NULL");
    }
    return TclPtrIncrObjVarIdx(interp, (Var *) varPtr, (Var *) arrayPtr,
	    part1Ptr, part2Ptr, incrPtr, flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * TclPtrIncrObjVarIdx --
 *
 *	Given the pointers to a variable and possible containing array,
 *	increment the Tcl object value of the variable by a Tcl_Obj increment.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the specified variable doesn't exist, or there is a clash
 *	in array usage, or an error occurs while executing variable traces,
 *	then NULL is returned and a message will be left in the interpreter's
 *	result.
 *
 * Side effects:
 *	The value of the given variable is incremented by the specified
 *	amount. If either the array or the entry didn't exist then a new
 *	variable is created. The ref count for the returned object is _not_
 *	incremented to reflect the returned reference; if you want to keep a
 *	reference to the object you must increment its ref count yourself.
 *
 *----------------------------------------------------------------------
 */

Tcl_Obj *
TclPtrIncrObjVarIdx(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Var *varPtr,		/* Reference to the variable to set. */
    Var *arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,		/* If non-null, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *incrPtr,		/* Increment value. */
/* TODO: Which of these flag values really make sense? */
    int flags,		/* Various flags that tell how to incr value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
    int index)			/* Index into the local variable table of the
				 * variable, or -1. Only used when part1Ptr is
				 * NULL. */
{
    Tcl_Obj *varValuePtr;

    if (TclIsVarInHash(varPtr)) {
	VarHashRefCount(varPtr)++;
    }
    varValuePtr = TclPtrGetVarIdx(interp, varPtr, arrayPtr, part1Ptr,
	    part2Ptr, flags, index);
    if (TclIsVarInHash(varPtr)) {
	VarHashRefCount(varPtr)--;
    }
    if (varValuePtr == NULL) {
	TclNewIntObj(varValuePtr, 0);
    }
    if (Tcl_IsShared(varValuePtr)) {
	/* Copy on write */
	varValuePtr = Tcl_DuplicateObj(varValuePtr);

	if (TCL_OK == TclIncrObj(interp, varValuePtr, incrPtr)) {
	    return TclPtrSetVarIdx(interp, varPtr, arrayPtr, part1Ptr,
		    part2Ptr, varValuePtr, flags, index);
	} else {
	    Tcl_DecrRefCount(varValuePtr);
	    return NULL;
	}
    } else {
	/* Unshared - can Incr in place */
	if (TCL_OK == TclIncrObj(interp, varValuePtr, incrPtr)) {
	    /*
	     * This seems dumb to write the incremeted value into the var
	     * after we just adjusted the value in place, but the spec for
	     * [incr] requires that write traces fire, and making this call
	     * is the way to make that happen.
	     */

	    return TclPtrSetVarIdx(interp, varPtr, arrayPtr, part1Ptr,
		    part2Ptr, varValuePtr, flags, index);
	} else {
	    return NULL;
	}
    }
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_UnsetVar --
 *
 *	Delete a variable, so that it may not be accessed anymore.
 *
 * Results:
 *	Returns TCL_OK if the variable was successfully deleted, TCL_ERROR if
 *	the variable can't be unset. In the event of an error, if the
 *	TCL_LEAVE_ERR_MSG flag is set then an error message is left in the
 *	interp's result.
 *
 * Side effects:
 *	If varName is defined as a local or global variable in interp, it is
 *	deleted.
 *
 *----------------------------------------------------------------------
 */

#undef Tcl_UnsetVar
int
Tcl_UnsetVar(
    Tcl_Interp *interp,		/* Command interpreter in which varName is to
				 * be looked up. */
    const char *varName,	/* Name of a variable in interp. May be either
				 * a scalar name or an array name or an
				 * element in an array. */
    int flags)			/* OR-ed combination of any of
				 * TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY or
				 * TCL_LEAVE_ERR_MSG. */
{
    int result;
    Tcl_Obj *varNamePtr;

    varNamePtr = Tcl_NewStringObj(varName, -1);
    Tcl_IncrRefCount(varNamePtr);

    /*

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclVar.c`, function `ArraySetCmd(`, lines 3654–3874. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-8.6.18-original-tclVar.c`.

```text
static int
ArraySetCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const objv[])
{
    Tcl_Obj *arrayNameObj;
    Tcl_Obj *arrayElemObj;
    Var *varPtr, *arrayPtr;
    int result, i;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "arrayName list");
	return TCL_ERROR;
    }

    if (TCL_ERROR == LocateArray(interp, objv[1], NULL, NULL)) {
	return TCL_ERROR;
    }

    arrayNameObj = objv[1];
    varPtr = TclObjLookupVarEx(interp, arrayNameObj, NULL,
	    /*flags*/ TCL_LEAVE_ERR_MSG, /*msg*/ "set", /*createPart1*/ 1,
	    /*createPart2*/ 1, &arrayPtr);
    if (varPtr == NULL) {
	return TCL_ERROR;
    }
    if (arrayPtr) {
	CleanupVar(varPtr, arrayPtr);
	TclObjVarErrMsg(interp, arrayNameObj, NULL, "set", NEEDARRAY, -1);
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "VARNAME",
		TclGetString(arrayNameObj), (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Install the contents of the dictionary or list into the array.
     */

    arrayElemObj = objv[2];
    if (arrayElemObj->typePtr == &tclDictType && arrayElemObj->bytes == NULL) {
	Tcl_Obj *keyPtr, *valuePtr;
	Tcl_DictSearch search;
	int done;

	if (Tcl_DictObjSize(interp, arrayElemObj, &done) != TCL_OK) {
	    return TCL_ERROR;
	}
	if (done == 0) {
	    /*
	     * Empty, so we'll just force the array to be properly existing
	     * instead.
	     */

	    goto ensureArray;
	}

	/*
	 * Don't need to look at result of Tcl_DictObjFirst as we've just
	 * successfully used a dictionary operation on the same object.
	 */

	for (Tcl_DictObjFirst(interp, arrayElemObj, &search,
		&keyPtr, &valuePtr, &done) ; !done ;
		Tcl_DictObjNext(&search, &keyPtr, &valuePtr, &done)) {
	    /*
	     * At this point, it would be nice if the key was directly usable
	     * by the array. This isn't the case though.
	     */

	    Var *elemVarPtr = TclLookupArrayElement(interp, arrayNameObj,
		    keyPtr, TCL_LEAVE_ERR_MSG, "set", 1, 1, varPtr, -1);

	    if ((elemVarPtr == NULL) ||
		    (TclPtrSetVarIdx(interp, elemVarPtr, varPtr, arrayNameObj,
		    keyPtr, valuePtr, TCL_LEAVE_ERR_MSG, -1) == NULL)) {
		Tcl_DictObjDone(&search);
		return TCL_ERROR;
	    }
	}
	return TCL_OK;
    } else {
	/*
	 * Not a dictionary, so assume (and convert to, for backward-
	 * -compatibility reasons) a list.
	 */

	int elemLen;
	Tcl_Obj **elemPtrs, *copyListObj;

	result = TclListObjGetElements(interp, arrayElemObj,
		&elemLen, &elemPtrs);
	if (result != TCL_OK) {
	    return result;
	}
	if (elemLen & 1) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "list must have an even number of elements", -1));
	    Tcl_SetErrorCode(interp, "TCL", "ARGUMENT", "FORMAT", (char *)NULL);
	    return TCL_ERROR;
	}
	if (elemLen == 0) {
	    goto ensureArray;
	}

	/*
	 * We needn't worry about traces invalidating arrayPtr: should that be
	 * the case, TclPtrSetVarIdx will return NULL so that we break out of
	 * the loop and return an error.
	 */

	copyListObj = TclListObjCopy(NULL, arrayElemObj);
	for (i=0 ; i<elemLen ; i+=2) {
	    Var *elemVarPtr = TclLookupArrayElement(interp, arrayNameObj,
		    elemPtrs[i], TCL_LEAVE_ERR_MSG, "set", 1, 1, varPtr, -1);

	    if ((elemVarPtr == NULL) ||
		    (TclPtrSetVarIdx(interp, elemVarPtr, varPtr, arrayNameObj,
			    elemPtrs[i], elemPtrs[i+1], TCL_LEAVE_ERR_MSG,
			    -1) == NULL)) {
		result = TCL_ERROR;
		break;
	    }
	}
	Tcl_DecrRefCount(copyListObj);
	return result;
    }

    /*
     * The list is empty make sure we have an array, or create one if
     * necessary.
     */

  ensureArray:
    if (varPtr != NULL) {
	if (TclIsVarArray(varPtr)) {
	    /*
	     * Already an array, done.
	     */

	    return TCL_OK;
	}
	if (TclIsVarArrayElement(varPtr) || !TclIsVarUndefined(varPtr)) {
	    /*
	     * Either an array element, or a scalar: lose!
	     */

	    TclObjVarErrMsg(interp, arrayNameObj, NULL, "array set",
		    NEEDARRAY, -1);
	    Tcl_SetErrorCode(interp, "TCL", "WRITE", "ARRAY", (char *)NULL);
	    return TCL_ERROR;
	}
    }
    TclSetVarArray(varPtr);
    varPtr->value.tablePtr = ckalloc(sizeof(TclVarHashTable));
    TclInitVarHashTable(varPtr->value.tablePtr, TclGetVarNsPtr(varPtr));
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * ArraySizeCmd --
 *
 *	This object-based function is invoked to process the "array size" Tcl
 *	command. See the user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl result object.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

static int
ArraySizeCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const objv[])
{
    Var *varPtr;
    Tcl_HashSearch search;
    Var *varPtr2;
    int isArray, size = 0;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "arrayName");
	return TCL_ERROR;
    }

    if (TCL_ERROR == LocateArray(interp, objv[1], &varPtr, &isArray)) {
	return TCL_ERROR;
    }

    /* We can only iterate over the array if it exists... */

    if (isArray) {
	/*
	 * Must iterate in order to get chance to check for present but
	 * "undefined" entries.
	 */

	for (varPtr2=VarHashFirstVar(varPtr->value.tablePtr, &search);
		varPtr2!=NULL ; varPtr2=VarHashNextVar(&search)) {
	    if (!TclIsVarUndefined(varPtr2)) {
		size++;
	    }
	}
    }

    Tcl_SetObjResult(interp, Tcl_NewIntObj(size));
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclVar.c`, function `TclPtrSetVarIdx(`, lines 1849–2149. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-8.6.18-original-tclVar.c`.

```text
Tcl_Obj *
TclPtrSetVarIdx(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be looked up. */
    Var *varPtr,	/* Reference to the variable to set. */
    Var *arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Name of an array (if part2 is non-NULL) or
				 * the name of a variable. NULL if the 'index'
				 * parameter is >= 0 */
    Tcl_Obj *part2Ptr,		/* If non-NULL, gives the name of an element
				 * in the array part1. */
    Tcl_Obj *newValuePtr,	/* New value for variable. */
    int flags,		/* OR-ed combination of TCL_GLOBAL_ONLY, and
				 * TCL_LEAVE_ERR_MSG bits. */
    int index)			/* Index of local var where part1 is to be
				 * found. */
{
    Interp *iPtr = (Interp *) interp;
    Tcl_Obj *oldValuePtr;
    Tcl_Obj *resultPtr = NULL;
    int result;
    int cleanupOnEarlyError = (newValuePtr->refCount == 0);

    /*
     * If the variable is in a hashtable and its hPtr field is NULL, then we
     * may have an upvar to an array element where the array was deleted or an
     * upvar to a namespace variable whose namespace was deleted. Generate an
     * error (allowing the variable to be reset would screw up our storage
     * allocation and is meaningless anyway).
     */

    if (TclIsVarDeadHash(varPtr)) {
	if (flags & TCL_LEAVE_ERR_MSG) {
	    if (TclIsVarArrayElement(varPtr)) {
		TclObjVarErrMsg(interp, part1Ptr, part2Ptr, "set",
			DANGLINGELEMENT, index);
		Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "ELEMENT", (char *)NULL);
	    } else {
		TclObjVarErrMsg(interp, part1Ptr, part2Ptr, "set",
			DANGLINGVAR, index);
		Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "VARNAME", (char *)NULL);
	    }
	}
	goto earlyError;
    }

    /*
     * It's an error to try to set an array variable itself.
     */

    if (TclIsVarArray(varPtr)) {
	if (flags & TCL_LEAVE_ERR_MSG) {
	    TclObjVarErrMsg(interp, part1Ptr, part2Ptr, "set", ISARRAY,index);
	    Tcl_SetErrorCode(interp, "TCL", "WRITE", "ARRAY", (char *)NULL);
	}
	goto earlyError;
    }

    /*
     * Invoke any read traces that have been set for the variable if it is
     * requested. This was done for INST_LAPPEND_* but that was inconsistent
     * with the non-bc instruction, and would cause failures trying to
     * lappend to any non-existing ::env var, which is inconsistent with
     * documented behavior. [Bug #3057639].
     */

    if ((flags & TCL_TRACE_READS) && ((varPtr->flags & VAR_TRACED_READ)
	    || (arrayPtr && (arrayPtr->flags & VAR_TRACED_READ)))) {
	if (TCL_ERROR == TclObjCallVarTraces(iPtr, arrayPtr, varPtr,
		part1Ptr, part2Ptr,
		TCL_TRACE_READS, (flags & TCL_LEAVE_ERR_MSG), index)) {
	    goto earlyError;
	}
    }

    /*
     * Set the variable's new value. If appending, append the new value to the
     * variable, either as a list element or as a string. Also, if appending,
     * then if the variable's old value is unshared we can modify it directly,
     * otherwise we must create a new copy to modify: this is "copy on write".
     */

    oldValuePtr = varPtr->value.objPtr;
    if (flags & TCL_LIST_ELEMENT && !(flags & TCL_APPEND_VALUE)) {
	varPtr->value.objPtr = NULL;
    }
    if (flags & (TCL_APPEND_VALUE|TCL_LIST_ELEMENT)) {
	if (flags & TCL_LIST_ELEMENT) {		/* Append list element. */
	    if (oldValuePtr == NULL) {
		TclNewObj(oldValuePtr);
		varPtr->value.objPtr = oldValuePtr;
		Tcl_IncrRefCount(oldValuePtr);	/* Since var is referenced. */
	    } else if (Tcl_IsShared(oldValuePtr)) {
		varPtr->value.objPtr = Tcl_DuplicateObj(oldValuePtr);
		TclDecrRefCount(oldValuePtr);
		oldValuePtr = varPtr->value.objPtr;
		Tcl_IncrRefCount(oldValuePtr);	/* Since var is referenced. */
	    }
	    result = Tcl_ListObjAppendElement(interp, oldValuePtr,
		    newValuePtr);
	    if (result != TCL_OK) {
		goto earlyError;
	    }
	} else {				/* Append string. */
	    /*
	     * We append newValuePtr's bytes but don't change its ref count.
	     */

	    if (oldValuePtr == NULL) {
		varPtr->value.objPtr = newValuePtr;
		Tcl_IncrRefCount(newValuePtr);
	    } else {
		if (Tcl_IsShared(oldValuePtr)) {	/* Append to copy. */
		    varPtr->value.objPtr = Tcl_DuplicateObj(oldValuePtr);

		    TclContinuationsCopy(varPtr->value.objPtr, oldValuePtr);

		    TclDecrRefCount(oldValuePtr);
		    oldValuePtr = varPtr->value.objPtr;
		    Tcl_IncrRefCount(oldValuePtr);	/* Since var is ref */
		}
		Tcl_AppendObjToObj(oldValuePtr, newValuePtr);
		if (newValuePtr->refCount == 0) {
		    Tcl_DecrRefCount(newValuePtr);
		}
	    }
	}
    } else if (newValuePtr != oldValuePtr) {
	/*
	 * In this case we are replacing the value, so we don't need to do
	 * more than swap the objects.
	 */

	varPtr->value.objPtr = newValuePtr;
	Tcl_IncrRefCount(newValuePtr);		/* Var is another ref. */
	if (oldValuePtr != NULL) {
	    TclDecrRefCount(oldValuePtr);	/* Discard old value. */
	}
    }

    /*
     * Invoke any write traces for the variable.
     */

    if ((varPtr->flags & VAR_TRACED_WRITE)
	    || (arrayPtr && (arrayPtr->flags & VAR_TRACED_WRITE))) {
	if (TCL_ERROR == TclObjCallVarTraces(iPtr, arrayPtr, varPtr, part1Ptr,
		part2Ptr, (flags & (TCL_GLOBAL_ONLY|TCL_NAMESPACE_ONLY))
		| TCL_TRACE_WRITES, (flags & TCL_LEAVE_ERR_MSG), index)) {
	    goto cleanup;
	}
    }

    /*
     * Return the variable's value unless the variable was changed in some
     * gross way by a trace (e.g. it was unset and then recreated as an
     * array).
     */

    if (TclIsVarScalar(varPtr) && !TclIsVarUndefined(varPtr)) {
	return varPtr->value.objPtr;
    }

    /*
     * A trace changed the value in some gross way. Return an empty string
     * object.
     */

    resultPtr = iPtr->emptyObjPtr;

    /*
     * If the variable doesn't exist anymore and no-one's using it, then free
     * up the relevant structures and hash table entries.
     */

  cleanup:
    if (resultPtr == NULL) {
	Tcl_SetErrorCode(interp, "TCL", "WRITE", "VARNAME", (char *)NULL);
    }
    if (TclIsVarUndefined(varPtr)) {
	TclCleanupVar(varPtr, arrayPtr);
    }
    return resultPtr;

  earlyError:
    if (cleanupOnEarlyError) {
	Tcl_DecrRefCount(newValuePtr);
    }
    goto cleanup;
}

/*
 *----------------------------------------------------------------------
 *
 * TclIncrObjVar2 --
 *
 *	Given a two-part variable name, which may refer either to a scalar
 *	variable or an element of an array, increment the Tcl object value of
 *	the variable by a specified Tcl_Obj increment value.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the specified variable doesn't exist, or there is a clash
 *	in array usage, or an error occurs while executing variable traces,
 *	then NULL is returned and a message will be left in the interpreter's
 *	result.
 *
 * Side effects:
 *	The value of the given variable is incremented by the specified
 *	amount. If either the array or the entry didn't exist then a new
 *	variable is created. The ref count for the returned object is _not_
 *	incremented to reflect the returned reference; if you want to keep a
 *	reference to the object you must increment its ref count yourself.
 *	Callers must Incr part1Ptr if they plan to Decr it.
 *	Callers must Incr part2Ptr if they plan to Decr it.
 *
 *----------------------------------------------------------------------
 */

Tcl_Obj *
TclIncrObjVar2(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Tcl_Obj *part1Ptr,		/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,		/* If non-null, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *incrPtr,		/* Amount to be added to variable. */
    int flags)			/* Various flags that tell how to incr value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
{
    Var *varPtr, *arrayPtr;

    varPtr = TclObjLookupVarEx(interp, part1Ptr, part2Ptr, flags, "read",
	    1, 1, &arrayPtr);
    if (varPtr == NULL) {
	Tcl_AddErrorInfo(interp,
		"\n    (reading value of variable to increment)");
	return NULL;
    }
    return TclPtrIncrObjVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    incrPtr, flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * TclPtrIncrObjVar --
 *
 *	Given the pointers to a variable and possible containing array,
 *	increment the Tcl object value of the variable by a Tcl_Obj increment.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the specified variable doesn't exist, or there is a clash
 *	in array usage, or an error occurs while executing variable traces,
 *	then NULL is returned and a message will be left in the interpreter's
 *	result.
 *
 * Side effects:
 *	The value of the given variable is incremented by the specified
 *	amount. If either the array or the entry didn't exist then a new
 *	variable is created. The ref count for the returned object is _not_
 *	incremented to reflect the returned reference; if you want to keep a
 *	reference to the object you must increment its ref count yourself.
 *
 *----------------------------------------------------------------------
 */

Tcl_Obj *
TclPtrIncrObjVar(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Tcl_Var varPtr,		/* Reference to the variable to set. */
    Tcl_Var arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,		/* If non-null, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *incrPtr,		/* Increment value. */
/* TODO: Which of these flag values really make sense? */
    int flags)		/* Various flags that tell how to incr value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
{
    if (varPtr == NULL) {
	Tcl_Panic("varPtr must not be NULL");
    }
    if (part1Ptr == NULL) {
	Tcl_Panic("part1Ptr must not be NULL");

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclVar.c`, function `CleanupVar(`, lines 331–461. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-8.6.18-original-tclVar.c`.

```text
static inline void
CleanupVar(
    Var *varPtr,		/* Pointer to variable that may be a candidate
				 * for being expunged. */
    Var *arrayPtr)		/* Array that contains the variable, or NULL
				 * if this variable isn't an array element. */
{
    if (TclIsVarUndefined(varPtr) && TclIsVarInHash(varPtr)
	    && !TclIsVarTraced(varPtr)
	    && (VarHashRefCount(varPtr) == !TclIsVarDeadHash(varPtr))) {
	if (VarHashRefCount(varPtr) == 0) {
	    ckfree(varPtr);
	} else {
	    VarHashDeleteEntry(varPtr);
	}
    }
    if (arrayPtr != NULL && TclIsVarUndefined(arrayPtr) &&
	    TclIsVarInHash(arrayPtr) && !TclIsVarTraced(arrayPtr) &&
	    (VarHashRefCount(arrayPtr) == !TclIsVarDeadHash(arrayPtr))) {
	if (VarHashRefCount(arrayPtr) == 0) {
	    ckfree(arrayPtr);
	} else {
	    VarHashDeleteEntry(arrayPtr);
	}
    }
}

void
TclCleanupVar(
    Var *varPtr,		/* Pointer to variable that may be a candidate
				 * for being expunged. */
    Var *arrayPtr)		/* Array that contains the variable, or NULL
				 * if this variable isn't an array element. */
{
    CleanupVar(varPtr, arrayPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * TclLookupVar --
 *
 *	This function is used to locate a variable given its name(s). It has
 *	been mostly superseded by TclObjLookupVar, it is now only used by the
 *	trace code. It is kept in tcl8.5 mainly because it is in the internal
 *	stubs table, so that some extension may be calling it.
 *
 * Results:
 *	The return value is a pointer to the variable structure indicated by
 *	part1 and part2, or NULL if the variable couldn't be found. If the
 *	variable is found, *arrayPtrPtr is filled in with the address of the
 *	variable structure for the array that contains the variable (or NULL
 *	if the variable is a scalar). If the variable can't be found and
 *	either createPart1 or createPart2 are 1, a new as-yet-undefined
 *	(VAR_UNDEFINED) variable structure is created, entered into a hash
 *	table, and returned.
 *
 *	If the variable isn't found and creation wasn't specified, or some
 *	other error occurs, NULL is returned and an error message is left in
 *	the interp's result if TCL_LEAVE_ERR_MSG is set in flags.
 *
 *	Note: it's possible for the variable returned to be VAR_UNDEFINED even
 *	if createPart1 or createPart2 are 1 (these only cause the hash table
 *	entry or array to be created). For example, the variable might be a
 *	global that has been unset but is still referenced by a procedure, or
 *	a variable that has been unset but it only being kept in existence (if
 *	VAR_UNDEFINED) by a trace.
 *
 * Side effects:
 *	New hashtable entries may be created if createPart1 or createPart2
 *	are 1.
 *
 *----------------------------------------------------------------------
 */

Var *
TclLookupVar(
    Tcl_Interp *interp,		/* Interpreter to use for lookup. */
    const char *part1,		/* If part2 isn't NULL, this is the name of an
				 * array. Otherwise, this is a full variable
				 * name that could include a parenthesized
				 * array element. */
    const char *part2,		/* Name of element within array, or NULL. */
    int flags,			/* Only TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * and TCL_LEAVE_ERR_MSG bits matter. */
    const char *msg,		/* Verb to use in error messages, e.g. "read"
				 * or "set". Only needed if TCL_LEAVE_ERR_MSG
				 * is set in flags. */
    int createPart1,		/* If 1, create hash table entry for part 1 of
				 * name, if it doesn't already exist. If 0,
				 * return error if it doesn't exist. */
    int createPart2,		/* If 1, create hash table entry for part 2 of
				 * name, if it doesn't already exist. If 0,
				 * return error if it doesn't exist. */
    Var **arrayPtrPtr)		/* If the name refers to an element of an
				 * array, *arrayPtrPtr gets filled in with
				 * address of array variable. Otherwise this
				 * is set to NULL. */
{
    Var *varPtr;
    Tcl_Obj *part1Ptr = Tcl_NewStringObj(part1, -1);

    if (createPart1) {
	Tcl_IncrRefCount(part1Ptr);
    }

    varPtr = TclObjLookupVar(interp, part1Ptr, part2, flags, msg,
	    createPart1, createPart2, arrayPtrPtr);

    TclDecrRefCount(part1Ptr);
    return varPtr;
}

/*
 *----------------------------------------------------------------------
 *
 * TclObjLookupVar, TclObjLookupVarEx --
 *
 *	This function is used by virtually all of the variable code to locate
 *	a variable given its name(s). The parsing into array/element
 *	components and (if possible) the lookup results are cached in
 *	part1Ptr, which is converted to one of the varNameTypes.
 *
 * Results:
 *	The return value is a pointer to the variable structure indicated by
 *	part1Ptr and part2, or NULL if the variable couldn't be found. If *
 *	the variable is found, *arrayPtrPtr is filled with the address of the
 *	variable structure for the array that contains the variable (or NULL
 *	if the variable is a scalar). If the variable can't be found and
 *	either createPart1 or createPart2 are 1, a new as-yet-undefined
 *	(VAR_UNDEFINED) variable structure is created, entered into a hash

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclVar.c`, function `Tcl_UnsetVar2(`, lines 2316–2436. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-8.6.18-original-tclVar.c`.

```text
int
Tcl_UnsetVar2(
    Tcl_Interp *interp,		/* Command interpreter in which varName is to
				 * be looked up. */
    const char *part1,		/* Name of variable or array. */
    const char *part2,		/* Name of element within array or NULL. */
    int flags)			/* OR-ed combination of any of
				 * TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_LEAVE_ERR_MSG. */
{
    int result;
    Tcl_Obj *part2Ptr = NULL, *part1Ptr = Tcl_NewStringObj(part1, -1);

    if (part2) {
	part2Ptr = Tcl_NewStringObj(part2, -1);
    }

    /*
     * Filter to pass through only the flags this interface supports.
     */

    flags &= (TCL_GLOBAL_ONLY|TCL_NAMESPACE_ONLY|TCL_LEAVE_ERR_MSG);
    result = TclObjUnsetVar2(interp, part1Ptr, part2Ptr, flags);

    Tcl_DecrRefCount(part1Ptr);
    if (part2Ptr) {
	Tcl_DecrRefCount(part2Ptr);
    }
    return result;
}

/*
 *----------------------------------------------------------------------
 *
 * TclObjUnsetVar2 --
 *
 *	Delete a variable, given a 2-object name.
 *
 * Results:
 *	Returns TCL_OK if the variable was successfully deleted, TCL_ERROR if
 *	the variable can't be unset. In the event of an error, if the
 *	TCL_LEAVE_ERR_MSG flag is set then an error message is left in the
 *	interp's result.
 *
 * Side effects:
 *	If part1ptr and part2Ptr indicate a local or global variable in
 *	interp, it is deleted. If part1Ptr is an array name and part2Ptr is
 *	NULL, then the whole array is deleted.
 *
 *----------------------------------------------------------------------
 */

int
TclObjUnsetVar2(
    Tcl_Interp *interp,		/* Command interpreter in which varName is to
				 * be looked up. */
    Tcl_Obj *part1Ptr,		/* Name of variable or array. */
    Tcl_Obj *part2Ptr,		/* Name of element within array or NULL. */
    int flags)			/* OR-ed combination of any of
				 * TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_LEAVE_ERR_MSG. */
{
    Var *varPtr, *arrayPtr;

    varPtr = TclObjLookupVarEx(interp, part1Ptr, part2Ptr, flags, "unset",
	    /*createPart1*/ 0, /*createPart2*/ 0, &arrayPtr);
    if (varPtr == NULL) {
	return TCL_ERROR;
    }

    return TclPtrUnsetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * TclPtrUnsetVar --
 *
 *	Delete a variable, given the pointers to the variable's (and possibly
 *	containing array's) VAR structure.
 *
 * Results:
 *	Returns TCL_OK if the variable was successfully deleted, TCL_ERROR if
 *	the variable can't be unset. In the event of an error, if the
 *	TCL_LEAVE_ERR_MSG flag is set then an error message is left in the
 *	interp's result.
 *
 * Side effects:
 *	If varPtr and arrayPtr indicate a local or global variable in interp,
 *	it is deleted. If varPtr is an array reference and part2Ptr is NULL,
 *	then the whole array is deleted.
 *
 *----------------------------------------------------------------------
 */

int
TclPtrUnsetVar(
    Tcl_Interp *interp,		/* Command interpreter in which varName is to
				 * be looked up. */
    Tcl_Var varPtr,		/* The variable to be unset. */
    Tcl_Var arrayPtr,		/* NULL for scalar variables, pointer to the
				 * containing array otherwise. */
    Tcl_Obj *part1Ptr,		/* Name of an array (if part2 is non-NULL) or
				 * the name of a variable. */
    Tcl_Obj *part2Ptr,		/* If non-NULL, gives the name of an element
				 * in the array part1. */
    int flags)		/* OR-ed combination of any of
				 * TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_LEAVE_ERR_MSG. */
{
    if (varPtr == NULL) {
	Tcl_Panic("varPtr must not be NULL");
    }
    if (part1Ptr == NULL) {
	Tcl_Panic("part1Ptr must not be NULL");
    }
    return TclPtrUnsetVarIdx(interp, (Var *) varPtr, (Var *) arrayPtr,
	    part1Ptr, part2Ptr, flags, -1);
}


```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclVar.c`, function `Tcl_ObjSetVar2(`, lines 1733–1838. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-8.6.18-original-tclVar.c`.

```text
Tcl_Obj *
Tcl_ObjSetVar2(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Tcl_Obj *part1Ptr,	/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,	/* If non-NULL, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *newValuePtr,	/* New value for variable. */
    int flags)			/* Various flags that tell how to set value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT, or
				 * TCL_LEAVE_ERR_MSG. */
{
    Var *varPtr, *arrayPtr;

    /*
     * Filter to pass through only the flags this interface supports.
     */

    flags &= (TCL_GLOBAL_ONLY|TCL_NAMESPACE_ONLY|TCL_LEAVE_ERR_MSG
	    |TCL_APPEND_VALUE|TCL_LIST_ELEMENT);
    varPtr = TclObjLookupVarEx(interp, part1Ptr, part2Ptr, flags, "set",
	    /*createPart1*/ 1, /*createPart2*/ 1, &arrayPtr);
    if (varPtr == NULL) {
	if (newValuePtr->refCount == 0) {
	    Tcl_DecrRefCount(newValuePtr);
	}
	return NULL;
    }

    return TclPtrSetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    newValuePtr, flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * TclPtrSetVar --
 *
 *	This function is the same as Tcl_SetVar2Ex above, except that it
 *	requires pointers to the variable's Var structs in addition to the
 *	variable names.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the write operation was disallowed because an array was
 *	expected but not found (or vice versa), then NULL is returned; if the
 *	TCL_LEAVE_ERR_MSG flag is set, then an explanatory message will be
 *	left in the interpreter's result. Note that the returned object may
 *	not be the same one referenced by newValuePtr; this is because
 *	variable traces may modify the variable's value.
 *
 * Side effects:
 *	The value of the given variable is set. If either the array or the
 *	entry didn't exist then a new variable is created.
 *
 *----------------------------------------------------------------------
 */

Tcl_Obj *
TclPtrSetVar(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be looked up. */
    Tcl_Var varPtr,		/* Reference to the variable to set. */
    Tcl_Var arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Name of an array (if part2 is non-NULL) or
				 * the name of a variable. */
    Tcl_Obj *part2Ptr,		/* If non-NULL, gives the name of an element
				 * in the array part1. */
    Tcl_Obj *newValuePtr,	/* New value for variable. */
    int flags)		/* OR-ed combination of TCL_GLOBAL_ONLY, and
				 * TCL_LEAVE_ERR_MSG bits. */
{
    if (varPtr == NULL) {
	Tcl_Panic("varPtr must not be NULL");
    }
    if (part1Ptr == NULL) {
	Tcl_Panic("part1Ptr must not be NULL");
    }
    if (newValuePtr == NULL) {
	Tcl_Panic("newValuePtr must not be NULL");
    }
    return TclPtrSetVarIdx(interp, (Var *) varPtr, (Var *) arrayPtr,
	    part1Ptr, part2Ptr, newValuePtr, flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * TclPtrSetVarIdx --
 *
 *	This function is the same as Tcl_SetVar2Ex above, except that it
 *	requires pointers to the variable's Var structs in addition to the
 *	variable names.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the write operation was disallowed because an array was
 *	expected but not found (or vice versa), then NULL is returned; if the
 *	TCL_LEAVE_ERR_MSG flag is set, then an explanatory message will be
 *	left in the interpreter's result. Note that the returned object may

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `Tcl_EvalObjv(`, lines 4245–4355. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-8.6.18-tclBasic.c`.

```text
int
Tcl_EvalObjv(
    Tcl_Interp *interp,		/* Interpreter in which to evaluate the
				 * command. Also used for error reporting. */
    int objc,			/* Number of words in command. */
    Tcl_Obj *const objv[],	/* An array of pointers to objects that are
				 * the words that make up the command. */
    int flags)			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Only
				 * TCL_EVAL_GLOBAL, TCL_EVAL_INVOKE and
				 * TCL_EVAL_NOERR are currently supported. */
{
    int result;
    NRE_callback *rootPtr = TOP_CB(interp);

    result = TclNREvalObjv(interp, objc, objv, flags, NULL);
    return TclNRRunCallbacks(interp, result, rootPtr);
}

int
TclNREvalObjv(
    Tcl_Interp *interp,		/* Interpreter in which to evaluate the
				 * command. Also used for error reporting. */
    int objc,			/* Number of words in command. */
    Tcl_Obj *const objv[],	/* An array of pointers to objects that are
				 * the words that make up the command. */
    int flags,			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Only
				 * TCL_EVAL_GLOBAL, TCL_EVAL_INVOKE and
				 * TCL_EVAL_NOERR are currently supported. */
    Command *cmdPtr)		/* NULL if the Command is to be looked up
				 * here, otherwise the pointer to the
				 * requested Command struct to be invoked. */
{
    Interp *iPtr = (Interp *) interp;

    /*
     * data[1] stores a marker for use by tailcalls; it will be set to 1 by
     * command redirectors (imports, alias, ensembles) so that tailcall skips
     * this callback (that marks the end of the target command) and goes back
     * to the end of the source command.
     */

    if (iPtr->deferredCallbacks) {
	iPtr->deferredCallbacks = NULL;
    } else {
	TclNRAddCallback(interp, NRCommand, NULL, NULL, NULL, NULL);
    }

    iPtr->numLevels++;
    TclNRAddCallback(interp, EvalObjvCore, cmdPtr, INT2PTR(flags),
	    INT2PTR(objc), objv);
    return TCL_OK;
}

static int
EvalObjvCore(
    ClientData data[],
    Tcl_Interp *interp,
    int result)
{
    Command *cmdPtr = NULL, *preCmdPtr = (Command *)data[0];
    int flags = PTR2INT(data[1]);
    int objc = PTR2INT(data[2]);
    Tcl_Obj **objv = (Tcl_Obj **)data[3];
    Interp *iPtr = (Interp *) interp;
    Namespace *lookupNsPtr = NULL;
    int enterTracesDone = 0;

    /*
     * Push records for task to be done on return, in INVERSE order. First, if
     * needed, the exception handlers (as they should happen last).
     */

    if (!(flags & TCL_EVAL_NOERR)) {
	TEOV_PushExceptionHandlers(interp, objc, objv, flags);
    }

    if (TCL_OK != TclInterpReady(interp)) {
	return TCL_ERROR;
    }

    if (objc == 0) {
	return TCL_OK;
    }

    if (TclLimitExceeded(iPtr->limit)) {
	/* generate error message if not yet already logged at this stage */
	if (!(iPtr->flags & ERR_ALREADY_LOGGED)) {
	    Tcl_LimitCheck(interp);
	}
	return TCL_ERROR;
    }

    /*
     * Configure evaluation context to match the requested flags.
     */

    if (iPtr->lookupNsPtr) {

	/*
	 * Capture the namespace we should do command name resolution in, as
	 * instructed by our caller sneaking it in to us in a private interp
	 * field.  Clear that field right away so we cannot possibly have its
	 * use leak where it should not.  The sneaky message pass is done.
	 *
	 * Use of this mechanism overrides the TCL_EVAL_GLOBAL flag.
	 * TODO: Is that a bug?
	 */

	lookupNsPtr = iPtr->lookupNsPtr;

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdIL.c`, function `InfoDefaultCmd(`, lines 947–1059. Retained reference `rust-tcl-registry-tests-data-native_info_inventory_original-sources-9.0.4-tclCmdIL.c`.

```text
static int
InfoDefaultCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Interp *iPtr = (Interp *) interp;
    const char *procName, *argName;
    Proc *procPtr;
    CompiledLocal *localPtr;
    Tcl_Obj *valueObjPtr;

    if (objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "procname arg varname");
	return TCL_ERROR;
    }

    procName = TclGetString(objv[1]);
    argName = TclGetString(objv[2]);

    procPtr = TclFindProc(iPtr, procName);
    if (procPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"\"%s\" isn't a procedure", procName));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "PROCEDURE", procName,
		(char *)NULL);
	return TCL_ERROR;
    }

    for (localPtr = procPtr->firstLocalPtr;  localPtr != NULL;
	    localPtr = localPtr->nextPtr) {
	if (TclIsVarArgument(localPtr)
		&& (strcmp(argName, localPtr->name) == 0)) {
	    if (localPtr->defValuePtr != NULL) {
		valueObjPtr = Tcl_ObjSetVar2(interp, objv[3], NULL,
			localPtr->defValuePtr, TCL_LEAVE_ERR_MSG);
		if (valueObjPtr == NULL) {
		    return TCL_ERROR;
		}
		Tcl_SetObjResult(interp, Tcl_NewBooleanObj(1));
	    } else {
		Tcl_Obj *nullObjPtr;
		TclNewObj(nullObjPtr);

		valueObjPtr = Tcl_ObjSetVar2(interp, objv[3], NULL,
			nullObjPtr, TCL_LEAVE_ERR_MSG);
		if (valueObjPtr == NULL) {
		    return TCL_ERROR;
		}
		Tcl_SetObjResult(interp, Tcl_NewBooleanObj(0));
	    }
	    return TCL_OK;
	}
    }

    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
	    "procedure \"%s\" doesn't have an argument \"%s\"",
	    procName, argName));
    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "ARGUMENT", argName, (char *)NULL);
    return TCL_ERROR;
}

/*
 *----------------------------------------------------------------------
 *
 * InfoErrorStackCmd --
 *
 *	Called to implement the "info errorstack" command that returns information
 *	about the last error's call stack. Handles the following syntax:
 *
 *	    info errorstack ?interp?
 *
 * Results:
 *	Returns TCL_OK if successful and TCL_ERROR if there is an error.
 *
 * Side effects:
 *	Returns a result in the interpreter's result object. If there is an
 *	error, the result is an error message.
 *
 *----------------------------------------------------------------------
 */

static int
InfoErrorStackCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Interp *target;
    Interp *iPtr;

    if ((objc != 1) && (objc != 2)) {
	Tcl_WrongNumArgs(interp, 1, objv, "?interp?");
	return TCL_ERROR;
    }

    target = interp;
    if (objc == 2) {
	target = Tcl_GetChild(interp, TclGetString(objv[1]));
	if (target == NULL) {
	    return TCL_ERROR;
	}
    }

    iPtr = (Interp *) target;
    Tcl_SetObjResult(interp, iPtr->errorStack);

    return TCL_OK;
}

/*

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdIL.c`, function `Tcl_IncrObjCmd(`, lines 386–506. Retained reference `rust-tcl-registry-tests-data-native_info_inventory_original-sources-9.0.4-tclCmdIL.c`.

```text
int
Tcl_IncrObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *newValuePtr, *incrPtr;

    if ((objc != 2) && (objc != 3)) {
	Tcl_WrongNumArgs(interp, 1, objv, "varName ?increment?");
	return TCL_ERROR;
    }

    if (objc == 3) {
	incrPtr = objv[2];
    } else {
	TclNewIntObj(incrPtr, 1);
    }
    Tcl_IncrRefCount(incrPtr);
    newValuePtr = TclIncrObjVar2(interp, objv[1], NULL,
	    incrPtr, TCL_LEAVE_ERR_MSG);
    Tcl_DecrRefCount(incrPtr);

    if (newValuePtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * Set the interpreter's object result to refer to the variable's new
     * value object.
     */

    Tcl_SetObjResult(interp, newValuePtr);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * TclInitInfoCmd --
 *
 *	This function is called to create the "info" Tcl command. See the user
 *	documentation for details on what it does.
 *
 * Results:
 *	Handle for the info command, or NULL on failure.
 *
 * Side effects:
 *	none
 *
 *----------------------------------------------------------------------
 */

Tcl_Command
TclInitInfoCmd(
    Tcl_Interp *interp)		/* Current interpreter. */
{
    return TclMakeEnsemble(interp, "info", defaultInfoMap);
}

/*
 *----------------------------------------------------------------------
 *
 * InfoArgsCmd --
 *
 *	Called to implement the "info args" command that returns the argument
 *	list for a procedure. Handles the following syntax:
 *
 *	    info args procName
 *
 * Results:
 *	Returns TCL_OK if successful and TCL_ERROR if there is an error.
 *
 * Side effects:
 *	Returns a result in the interpreter's result object. If there is an
 *	error, the result is an error message.
 *
 *----------------------------------------------------------------------
 */

static int
InfoArgsCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Interp *iPtr = (Interp *) interp;
    const char *name;
    Proc *procPtr;
    CompiledLocal *localPtr;
    Tcl_Obj *listObjPtr;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "procname");
	return TCL_ERROR;
    }

    name = TclGetString(objv[1]);
    procPtr = TclFindProc(iPtr, name);
    if (procPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"\"%s\" isn't a procedure", name));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "PROCEDURE", name, (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Build a return list containing the arguments.
     */

    listObjPtr = Tcl_NewListObj(0, NULL);
    for (localPtr = procPtr->firstLocalPtr;  localPtr != NULL;
	    localPtr = localPtr->nextPtr) {
	if (TclIsVarArgument(localPtr)) {
	    Tcl_ListObjAppendElement(interp, listObjPtr,
		    Tcl_NewStringObj(localPtr->name, -1));
	}
    }
    Tcl_SetObjResult(interp, listObjPtr);

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclVar.c`, function `Tcl_UnsetObjCmd(`, lines 2745–2830. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-9.0.4-original-tclVar.c`.

```text
int
Tcl_UnsetObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int i;
    int flags = TCL_LEAVE_ERR_MSG;
    const char *name;

    if (objc == 1) {
	/*
	 * Do nothing if no arguments supplied, so as to match command
	 * documentation.
	 */

	return TCL_OK;
    }

    /*
     * Simple, restrictive argument parsing. The only options are -- and
     * -nocomplain (which must come first and be given exactly to be an
     * option).
     */

    i = 1;
    name = TclGetString(objv[i]);
    if (name[0] == '-') {
	if (strcmp("-nocomplain", name) == 0) {
	    i++;
	    if (i == objc) {
		return TCL_OK;
	    }
	    flags = 0;
	    name = TclGetString(objv[i]);
	}
	if (strcmp("--", name) == 0) {
	    i++;
	}
    }

    for (; i < objc; i++) {
	if ((TclObjUnsetVar2(interp, objv[i], NULL, flags) != TCL_OK)
		&& (flags == TCL_LEAVE_ERR_MSG)) {
	    return TCL_ERROR;
	}
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_AppendObjCmd --
 *
 *	This object-based function is invoked to process the "append" Tcl
 *	command. See the user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl object result value.
 *
 * Side effects:
 *	A variable's value may be changed.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_AppendObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Var *varPtr, *arrayPtr;
    Tcl_Obj *varValuePtr = NULL;
				/* Initialized to avoid compiler warning. */
    int i;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "varName ?value ...?");
	return TCL_ERROR;
    }

    if (objc == 2) {

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclVar.c`, function `TclIncrObjVar2(`, lines 2115–2215. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-9.0.4-original-tclVar.c`.

```text
Tcl_Obj *
TclIncrObjVar2(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Tcl_Obj *part1Ptr,		/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,		/* If non-null, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *incrPtr,		/* Amount to be added to variable. */
    int flags)			/* Various flags that tell how to incr value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
{
    Var *varPtr, *arrayPtr;

    varPtr = TclObjLookupVarEx(interp, part1Ptr, part2Ptr, flags, "read",
	    1, 1, &arrayPtr);
    if (varPtr == NULL) {
	Tcl_AddErrorInfo(interp,
		"\n    (reading value of variable to increment)");
	return NULL;
    }
    return TclPtrIncrObjVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    incrPtr, flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * TclPtrIncrObjVar --
 *
 *	Given the pointers to a variable and possible containing array,
 *	increment the Tcl object value of the variable by a Tcl_Obj increment.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the specified variable doesn't exist, or there is a clash
 *	in array usage, or an error occurs while executing variable traces,
 *	then NULL is returned and a message will be left in the interpreter's
 *	result.
 *
 * Side effects:
 *	The value of the given variable is incremented by the specified
 *	amount. If either the array or the entry didn't exist then a new
 *	variable is created. The ref count for the returned object is _not_
 *	incremented to reflect the returned reference; if you want to keep a
 *	reference to the object you must increment its ref count yourself.
 *
 *----------------------------------------------------------------------
 */

Tcl_Obj *
TclPtrIncrObjVar(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Tcl_Var varPtr,		/* Reference to the variable to set. */
    Tcl_Var arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,		/* If non-null, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *incrPtr,		/* Increment value. */
/* TODO: Which of these flag values really make sense? */
    int flags)			/* Various flags that tell how to incr value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
{
    if (varPtr == NULL) {
	Tcl_Panic("varPtr must not be NULL");
    }
    if (part1Ptr == NULL) {
	Tcl_Panic("part1Ptr must not be NULL");
    }
    return TclPtrIncrObjVarIdx(interp, (Var *) varPtr, (Var *) arrayPtr,
	    part1Ptr, part2Ptr, incrPtr, flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * TclPtrIncrObjVarIdx --
 *
 *	Given the pointers to a variable and possible containing array,
 *	increment the Tcl object value of the variable by a Tcl_Obj increment.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the specified variable doesn't exist, or there is a clash
 *	in array usage, or an error occurs while executing variable traces,
 *	then NULL is returned and a message will be left in the interpreter's
 *	result.
 *
 * Side effects:

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclVar.c`, function `TclPtrIncrObjVar(`, lines 2169–2329. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-9.0.4-original-tclVar.c`.

```text
Tcl_Obj *
TclPtrIncrObjVar(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Tcl_Var varPtr,		/* Reference to the variable to set. */
    Tcl_Var arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,		/* If non-null, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *incrPtr,		/* Increment value. */
/* TODO: Which of these flag values really make sense? */
    int flags)			/* Various flags that tell how to incr value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
{
    if (varPtr == NULL) {
	Tcl_Panic("varPtr must not be NULL");
    }
    if (part1Ptr == NULL) {
	Tcl_Panic("part1Ptr must not be NULL");
    }
    return TclPtrIncrObjVarIdx(interp, (Var *) varPtr, (Var *) arrayPtr,
	    part1Ptr, part2Ptr, incrPtr, flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * TclPtrIncrObjVarIdx --
 *
 *	Given the pointers to a variable and possible containing array,
 *	increment the Tcl object value of the variable by a Tcl_Obj increment.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the specified variable doesn't exist, or there is a clash
 *	in array usage, or an error occurs while executing variable traces,
 *	then NULL is returned and a message will be left in the interpreter's
 *	result.
 *
 * Side effects:
 *	The value of the given variable is incremented by the specified
 *	amount. If either the array or the entry didn't exist then a new
 *	variable is created. The ref count for the returned object is _not_
 *	incremented to reflect the returned reference; if you want to keep a
 *	reference to the object you must increment its ref count yourself.
 *
 *----------------------------------------------------------------------
 */

Tcl_Obj *
TclPtrIncrObjVarIdx(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Var *varPtr,		/* Reference to the variable to set. */
    Var *arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,		/* If non-null, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *incrPtr,		/* Increment value. */
/* TODO: Which of these flag values really make sense? */
    int flags,			/* Various flags that tell how to incr value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
    int index)			/* Index into the local variable table of the
				 * variable, or -1. Only used when part1Ptr is
				 * NULL. */
{
    Tcl_Obj *varValuePtr;

    /*
     * It's an error to try to increment a constant.
     */
    if (TclIsVarConstant(varPtr)) {
	if (flags & TCL_LEAVE_ERR_MSG) {
	    TclObjVarErrMsg(interp, part1Ptr, part2Ptr, "incr", ISCONST,index);
	    Tcl_SetErrorCode(interp, "TCL", "WRITE", "CONST", (char *)NULL);
	}
	return NULL;
    }

    if (TclIsVarInHash(varPtr)) {
	VarHashRefCount(varPtr)++;
    }
    varValuePtr = TclPtrGetVarIdx(interp, varPtr, arrayPtr, part1Ptr,
	    part2Ptr, flags, index);
    if (TclIsVarInHash(varPtr)) {
	VarHashRefCount(varPtr)--;
    }
    if (varValuePtr == NULL) {
	TclNewIntObj(varValuePtr, 0);
    }
    if (Tcl_IsShared(varValuePtr)) {
	/* Copy on write */
	varValuePtr = Tcl_DuplicateObj(varValuePtr);

	if (TCL_OK == TclIncrObj(interp, varValuePtr, incrPtr)) {
	    return TclPtrSetVarIdx(interp, varPtr, arrayPtr, part1Ptr,
		    part2Ptr, varValuePtr, flags, index);
	} else {
	    Tcl_DecrRefCount(varValuePtr);
	    return NULL;
	}
    } else {
	/* Unshared - can Incr in place */
	if (TCL_OK == TclIncrObj(interp, varValuePtr, incrPtr)) {
	    /*
	     * This seems dumb to write the incremeted value into the var
	     * after we just adjusted the value in place, but the spec for
	     * [incr] requires that write traces fire, and making this call
	     * is the way to make that happen.
	     */

	    return TclPtrSetVarIdx(interp, varPtr, arrayPtr, part1Ptr,
		    part2Ptr, varValuePtr, flags, index);
	} else {
	    return NULL;
	}
    }
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_UnsetVar2 --
 *
 *	Delete a variable, given a 2-part name.
 *
 * Results:
 *	Returns TCL_OK if the variable was successfully deleted, TCL_ERROR if
 *	the variable can't be unset. In the event of an error, if the
 *	TCL_LEAVE_ERR_MSG flag is set then an error message is left in the
 *	interp's result.
 *
 * Side effects:
 *	If part1 and part2 indicate a local or global variable in interp, it
 *	is deleted. If part1 is an array name and part2 is NULL, then the
 *	whole array is deleted.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_UnsetVar2(
    Tcl_Interp *interp,		/* Command interpreter in which varName is to
				 * be looked up. */
    const char *part1,		/* Name of variable or array. */
    const char *part2,		/* Name of element within array or NULL. */
    int flags)			/* OR-ed combination of any of

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclVar.c`, function `ArraySetCmd(`, lines 4013–4233. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-9.0.4-original-tclVar.c`.

```text
static int
ArraySetCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const objv[])
{
    Tcl_Obj *arrayNameObj;
    Tcl_Obj *arrayElemObj;
    Var *varPtr, *arrayPtr;
    int result;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "arrayName list");
	return TCL_ERROR;
    }

    if (TCL_ERROR == LocateArray(interp, objv[1], NULL, NULL)) {
	return TCL_ERROR;
    }

    arrayNameObj = objv[1];
    varPtr = TclObjLookupVarEx(interp, arrayNameObj, NULL,
	    /*flags*/ TCL_LEAVE_ERR_MSG, /*msg*/ "set", /*createPart1*/ 1,
	    /*createPart2*/ 1, &arrayPtr);
    if (varPtr == NULL) {
	return TCL_ERROR;
    }
    if (arrayPtr) {
	CleanupVar(varPtr, arrayPtr);
	TclObjVarErrMsg(interp, arrayNameObj, NULL, "set", NEEDARRAY, -1);
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "VARNAME",
		TclGetString(arrayNameObj), (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Install the contents of the dictionary or list into the array.
     */

    arrayElemObj = objv[2];
    if (TclHasInternalRep(arrayElemObj, &tclDictType) && arrayElemObj->bytes == NULL) {
	Tcl_Obj *keyPtr, *valuePtr;
	Tcl_DictSearch search;
	int done;
	Tcl_Size size;

	if (Tcl_DictObjSize(interp, arrayElemObj, &size) != TCL_OK) {
	    return TCL_ERROR;
	}
	if (size == 0) {
	    /*
	     * Empty, so we'll just force the array to be properly existing
	     * instead.
	     */

	    goto ensureArray;
	}

	/*
	 * Don't need to look at result of Tcl_DictObjFirst as we've just
	 * successfully used a dictionary operation on the same object.
	 */

	for (Tcl_DictObjFirst(interp, arrayElemObj, &search,
		&keyPtr, &valuePtr, &done) ; !done ;
		Tcl_DictObjNext(&search, &keyPtr, &valuePtr, &done)) {
	    /*
	     * At this point, it would be nice if the key was directly usable
	     * by the array. This isn't the case though.
	     */

	    Var *elemVarPtr = TclLookupArrayElement(interp, arrayNameObj,
		    keyPtr, TCL_LEAVE_ERR_MSG, "set", 1, 1, varPtr, -1);

	    if ((elemVarPtr == NULL) ||
		    (TclPtrSetVarIdx(interp, elemVarPtr, varPtr, arrayNameObj,
		    keyPtr, valuePtr, TCL_LEAVE_ERR_MSG, -1) == NULL)) {
		Tcl_DictObjDone(&search);
		return TCL_ERROR;
	    }
	}
	return TCL_OK;
    } else {
	/*
	 * Not a dictionary, so assume (and convert to, for backward-
	 * -compatibility reasons) a list.
	 */

	Tcl_Size elemLen;
	Tcl_Obj **elemPtrs, *copyListObj;
	Tcl_Size i;

	result = TclListObjLength(interp, arrayElemObj, &elemLen);
	if (result != TCL_OK) {
	    return result;
	}
	if (elemLen & 1) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "list must have an even number of elements", -1));
	    Tcl_SetErrorCode(interp, "TCL", "ARGUMENT", "FORMAT", (char *)NULL);
	    return TCL_ERROR;
	}
	if (elemLen == 0) {
	    goto ensureArray;
	}
	result = TclListObjGetElements(interp, arrayElemObj,
		&elemLen, &elemPtrs);
	if (result != TCL_OK) {
	    return result;
	}

	/*
	 * We needn't worry about traces invalidating arrayPtr: should that be
	 * the case, TclPtrSetVarIdx will return NULL so that we break out of
	 * the loop and return an error.
	 */

	copyListObj = TclListObjCopy(NULL, arrayElemObj);
	if (!copyListObj) {
	    return TCL_ERROR;
	}
	for (i=0 ; i<elemLen ; i+=2) {
	    Var *elemVarPtr = TclLookupArrayElement(interp, arrayNameObj,
		    elemPtrs[i], TCL_LEAVE_ERR_MSG, "set", 1, 1, varPtr, -1);

	    if ((elemVarPtr == NULL) ||
		    (TclPtrSetVarIdx(interp, elemVarPtr, varPtr, arrayNameObj,
			    elemPtrs[i], elemPtrs[i+1], TCL_LEAVE_ERR_MSG,
			    -1) == NULL)) {
		result = TCL_ERROR;
		break;
	    }
	}
	Tcl_DecrRefCount(copyListObj);
	return result;
    }

    /*
     * The list is empty make sure we have an array, or create one if
     * necessary.
     */

  ensureArray:
    if (varPtr != NULL) {
	if (TclIsVarArray(varPtr)) {
	    /*
	     * Already an array, done.
	     */

	    return TCL_OK;
	}
	if (TclIsVarArrayElement(varPtr) || !TclIsVarUndefined(varPtr)) {
	    /*
	     * Either an array element, or a scalar: lose!
	     */

	    TclObjVarErrMsg(interp, arrayNameObj, NULL, "array set",
		    NEEDARRAY, -1);
	    Tcl_SetErrorCode(interp, "TCL", "WRITE", "ARRAY", (char *)NULL);
	    return TCL_ERROR;
	}
    }
    TclInitArrayVar(varPtr);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * ArraySizeCmd --
 *
 *	This object-based function is invoked to process the "array size" Tcl
 *	command. See the user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl result object.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

static int
ArraySizeCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const objv[])
{
    Var *varPtr;
    Tcl_HashSearch search;
    Var *varPtr2;
    int isArray, size = 0;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "arrayName");
	return TCL_ERROR;
    }

    if (TCL_ERROR == LocateArray(interp, objv[1], &varPtr, &isArray)) {
	return TCL_ERROR;
    }

    /* We can only iterate over the array if it exists... */

    if (isArray) {
	/*
	 * Must iterate in order to get chance to check for present but
	 * "undefined" entries.
	 */

	for (varPtr2=VarHashFirstVar(varPtr->value.tablePtr, &search);
		varPtr2!=NULL ; varPtr2=VarHashNextVar(&search)) {
	    if (!TclIsVarUndefined(varPtr2)) {
		size++;
	    }
	}
    }

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclVar.c`, function `ArrayForNRCmd(`, lines 3094–3194. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-9.0.4-original-tclVar.c`.

```text
static int
ArrayForNRCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Tcl_Obj *varListObj, *arrayNameObj, *scriptObj;
    ArraySearch *searchPtr = NULL;
    Var *varPtr;
    int isArray;
    Tcl_Size numVars;

    /*
     * array for {k v} a body
     */

    if (objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "{key value} arrayName script");
	return TCL_ERROR;
    }

    /*
     * Parse arguments.
     */

    if (TclListObjLength(interp, objv[1], &numVars) != TCL_OK) {
	return TCL_ERROR;
    }

    if (numVars != 2) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"must have two variable names", -1));
	Tcl_SetErrorCode(interp, "TCL", "SYNTAX", "array", "for", (char *)NULL);
	return TCL_ERROR;
    }

    arrayNameObj = objv[2];

    if (TCL_ERROR == LocateArray(interp, arrayNameObj, &varPtr, &isArray)) {
	return TCL_ERROR;
    }

    if (!isArray) {
	return NotArrayError(interp, arrayNameObj);
    }

    /*
     * Make a new array search, put it on the stack.
     */

    searchPtr = (ArraySearch *)Tcl_Alloc(sizeof(ArraySearch));
    ArrayPopulateSearch(interp, arrayNameObj, varPtr, searchPtr);

    /*
     * Make sure that these objects (which we need throughout the body of the
     * loop) don't vanish.
     */

    varListObj = TclListObjCopy(NULL, objv[1]);
    if (!varListObj) {
	return TCL_ERROR;
    }
    scriptObj = objv[3];
    Tcl_IncrRefCount(scriptObj);

    /*
     * Run the script.
     */

    TclNRAddCallback(interp, ArrayForLoopCallback, searchPtr, varListObj,
	    arrayNameObj, scriptObj);
    return TCL_OK;
}

static int
ArrayForLoopCallback(
    void *data[],
    Tcl_Interp *interp,
    int result)
{
    Interp *iPtr = (Interp *) interp;
    ArraySearch *searchPtr = (ArraySearch *)data[0];
    Tcl_Obj *varListObj = (Tcl_Obj *)data[1];
    Tcl_Obj *arrayNameObj = (Tcl_Obj *)data[2];
    Tcl_Obj *scriptObj = (Tcl_Obj *)data[3];
    Tcl_Obj **varv;
    Tcl_Obj *keyObj, *valueObj;
    Var *varPtr;
    Var *arrayPtr;
    int done;
    Tcl_Size varc;

    /*
     * Process the result from the previous execution of the script body.
     */

    done = TCL_ERROR;

    if (result == TCL_CONTINUE) {
	result = TCL_OK;

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclVar.c`, function `ArrayForLoopCallback(`, lines 3169–3319. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-9.0.4-original-tclVar.c`.

```text
static int
ArrayForLoopCallback(
    void *data[],
    Tcl_Interp *interp,
    int result)
{
    Interp *iPtr = (Interp *) interp;
    ArraySearch *searchPtr = (ArraySearch *)data[0];
    Tcl_Obj *varListObj = (Tcl_Obj *)data[1];
    Tcl_Obj *arrayNameObj = (Tcl_Obj *)data[2];
    Tcl_Obj *scriptObj = (Tcl_Obj *)data[3];
    Tcl_Obj **varv;
    Tcl_Obj *keyObj, *valueObj;
    Var *varPtr;
    Var *arrayPtr;
    int done;
    Tcl_Size varc;

    /*
     * Process the result from the previous execution of the script body.
     */

    done = TCL_ERROR;

    if (result == TCL_CONTINUE) {
	result = TCL_OK;
    } else if (result != TCL_OK) {
	if (result == TCL_BREAK) {
	    Tcl_ResetResult(interp);
	    result = TCL_OK;
	} else if (result == TCL_ERROR) {
	    Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		    "\n    (\"array for\" body line %d)",
		    Tcl_GetErrorLine(interp)));
	}
	goto arrayfordone;
    }

    /*
     * Get the next mapping from the array.
     */

    keyObj = NULL;
    valueObj = NULL;
    varPtr = TclObjLookupVarEx(interp, arrayNameObj, NULL, /*flags*/ 0,
	    /*msg*/ 0, /*createPart1*/ 0, /*createPart2*/ 0, &arrayPtr);
    if (varPtr == NULL) {
	done = TCL_ERROR;
    } else {
	done = ArrayObjNext(interp, arrayNameObj, varPtr, searchPtr, &keyObj,
		&valueObj);
    }

    result = TCL_OK;
    if (done != TCL_CONTINUE) {
	Tcl_ResetResult(interp);
	if (done == TCL_ERROR) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "array changed during iteration", -1));
	    Tcl_SetErrorCode(interp, "TCL", "READ", "array", "for", (char *)NULL);
	    varPtr->flags |= TCL_LEAVE_ERR_MSG;
	    result = done;
	}
	goto arrayfordone;
    }

    result = TclListObjGetElements(NULL, varListObj, &varc, &varv);
    if (result != TCL_OK) {
	goto arrayfordone;
    }
    if (Tcl_ObjSetVar2(interp, varv[0], NULL, keyObj,
	    TCL_LEAVE_ERR_MSG) == NULL) {
	result = TCL_ERROR;
	goto arrayfordone;
    }
    if (valueObj != NULL) {
	if (Tcl_ObjSetVar2(interp, varv[1], NULL, valueObj,
		TCL_LEAVE_ERR_MSG) == NULL) {
	    result = TCL_ERROR;
	    goto arrayfordone;
	}
    }

    /*
     * Run the script.
     */

    TclNRAddCallback(interp, ArrayForLoopCallback, searchPtr, varListObj,
	    arrayNameObj, scriptObj);
    return TclNREvalObjEx(interp, scriptObj, 0, iPtr->cmdFramePtr, 3);

    /*
     * For unwinding everything once the iterating is done.
     */

  arrayfordone:
    if (done != TCL_ERROR) {
	/*
	 * If the search was terminated by an array change, the
	 * VAR_SEARCH_ACTIVE flag will no longer be set.
	 */

	ArrayDoneSearch(iPtr, varPtr, searchPtr);
	Tcl_DecrRefCount(searchPtr->name);
	Tcl_Free(searchPtr);
    }

    TclDecrRefCount(varListObj);
    TclDecrRefCount(scriptObj);
    return result;
}

/*
 * ArrayPopulateSearch
 */

static void
ArrayPopulateSearch(
    Tcl_Interp *interp,
    Tcl_Obj *arrayNameObj,
    Var *varPtr,
    ArraySearch *searchPtr)
{
    Interp *iPtr = (Interp *) interp;
    Tcl_HashEntry *hPtr;
    int isNew;

    hPtr = Tcl_CreateHashEntry(&iPtr->varSearches, varPtr, &isNew);
    if (isNew) {
	searchPtr->id = 1;
	varPtr->flags |= VAR_SEARCH_ACTIVE;
	searchPtr->nextPtr = NULL;
    } else {
	searchPtr->id = ((ArraySearch *) Tcl_GetHashValue(hPtr))->id + 1;
	searchPtr->nextPtr = (ArraySearch *)Tcl_GetHashValue(hPtr);
    }
    searchPtr->varPtr = varPtr;
    searchPtr->nextEntry = VarHashFirstEntry(varPtr->value.tablePtr,
	    &searchPtr->search);
    Tcl_SetHashValue(hPtr, searchPtr);
    searchPtr->name = Tcl_ObjPrintf("s-%d-%s", searchPtr->id,
	    TclGetString(arrayNameObj));
    Tcl_IncrRefCount(searchPtr->name);
}
/*
 *----------------------------------------------------------------------
 *
 * ArrayStartSearchCmd --
 *
 *	This object-based function is invoked to process the "array
 *	startsearch" Tcl command. See the user documentation for details on

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclVar.c`, function `Tcl_ConstObjCmd(`, lines 4878–4973. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-9.0.4-original-tclVar.c`.

```text
int
Tcl_ConstObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Var *varPtr, *arrayPtr;
    Tcl_Obj *part1Ptr;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "varName value");
	return TCL_ERROR;
    }

    part1Ptr = objv[1];
    varPtr = TclObjLookupVarEx(interp, part1Ptr, NULL, TCL_LEAVE_ERR_MSG,
	    "const", /*createPart1*/ 1, /*createPart2*/ 1, &arrayPtr);
    if (varPtr == NULL) {
	return TCL_ERROR;
    }
    if (TclIsVarArray(varPtr)) {
	TclObjVarErrMsg(interp, part1Ptr, NULL, "make constant", ISARRAY, -1);
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "CONST", (char *)NULL);
	return TCL_ERROR;
    }
    if (TclIsVarArrayElement(varPtr)) {
	if (TclIsVarUndefined(varPtr)) {
	    CleanupVar(varPtr, arrayPtr);
	}
	TclObjVarErrMsg(interp, part1Ptr, NULL, "make constant", ISARRAYELEMENT, -1);
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "CONST", (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * If already exists, either a constant (no problem) or an error.
     */
    if (!TclIsVarUndefined(varPtr)) {
	if (TclIsVarConstant(varPtr)) {
	    return TCL_OK;
	}
	TclObjVarErrMsg(interp, part1Ptr, NULL, "make constant", EXISTS, -1);
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "CONST", (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Make the variable and flag it as a constant.
     */
    if (TclPtrSetVar(interp, (Tcl_Var) varPtr, NULL, objv[1], NULL,
	    objv[2], TCL_LEAVE_ERR_MSG) == NULL) {
	if (TclIsVarUndefined(varPtr)) {
	    CleanupVar(varPtr, arrayPtr);
	}
	return TCL_ERROR;
    }
    TclSetVarConstant(varPtr);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GlobalObjCmd --
 *
 *	This object-based function is invoked to process the "global" Tcl
 *	command. See the user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl object result value.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_GlobalObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Interp *iPtr = (Interp *) interp;
    Tcl_Obj *objPtr, *tailPtr;
    const char *varName;
    const char *tail;
    int result;
	int i;

    /*
     * If we are not executing inside a Tcl procedure, just return.
     */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclVar.c`, function `TclPtrSetVarIdx(`, lines 1912–2212. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-9.0.4-original-tclVar.c`.

```text
Tcl_Obj *
TclPtrSetVarIdx(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be looked up. */
    Var *varPtr,		/* Reference to the variable to set. */
    Var *arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Name of an array (if part2 is non-NULL) or
				 * the name of a variable. NULL if the 'index'
				 * parameter is >= 0 */
    Tcl_Obj *part2Ptr,		/* If non-NULL, gives the name of an element
				 * in the array part1. */
    Tcl_Obj *newValuePtr,	/* New value for variable. */
    int flags,			/* OR-ed combination of TCL_GLOBAL_ONLY, and
				 * TCL_LEAVE_ERR_MSG bits. */
    int index)			/* Index of local var where part1 is to be
				 * found. */
{
    Interp *iPtr = (Interp *) interp;
    Tcl_Obj *oldValuePtr;
    Tcl_Obj *resultPtr = NULL;
    int result;
    int cleanupOnEarlyError = (newValuePtr->refCount == 0);

    /*
     * If the variable is in a hashtable and its hPtr field is NULL, then we
     * may have an upvar to an array element where the array was deleted or an
     * upvar to a namespace variable whose namespace was deleted. Generate an
     * error (allowing the variable to be reset would screw up our storage
     * allocation and is meaningless anyway).
     */

    if (TclIsVarDeadHash(varPtr)) {
	if (flags & TCL_LEAVE_ERR_MSG) {
	    if (TclIsVarArrayElement(varPtr)) {
		TclObjVarErrMsg(interp, part1Ptr, part2Ptr, "set",
			DANGLINGELEMENT, index);
		Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "ELEMENT", (char *)NULL);
	    } else {
		TclObjVarErrMsg(interp, part1Ptr, part2Ptr, "set",
			DANGLINGVAR, index);
		Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "VARNAME", (char *)NULL);
	    }
	}
	goto earlyError;
    }

    /*
     * It's an error to try to set a constant.
     */
    if (TclIsVarConstant(varPtr)) {
	if (flags & TCL_LEAVE_ERR_MSG) {
	    TclObjVarErrMsg(interp, part1Ptr, part2Ptr, "set", ISCONST,index);
	    Tcl_SetErrorCode(interp, "TCL", "WRITE", "CONST", (char *)NULL);
	}
	goto earlyError;
    }

    /*
     * It's an error to try to set an array variable itself.
     */

    if (TclIsVarArray(varPtr)) {
	if (flags & TCL_LEAVE_ERR_MSG) {
	    TclObjVarErrMsg(interp, part1Ptr, part2Ptr, "set", ISARRAY,index);
	    Tcl_SetErrorCode(interp, "TCL", "WRITE", "ARRAY", (char *)NULL);
	}
	goto earlyError;
    }

    TclVarFindHiddenArray(varPtr, arrayPtr);

    /*
     * Invoke any read traces that have been set for the variable if it is
     * requested. This was done for INST_LAPPEND_* but that was inconsistent
     * with the non-bc instruction, and would cause failures trying to
     * lappend to any non-existing ::env var, which is inconsistent with
     * documented behavior. [Bug #3057639].
     */

    if ((flags & TCL_TRACE_READS) && ((varPtr->flags & VAR_TRACED_READ)
	    || (arrayPtr && (arrayPtr->flags & VAR_TRACED_READ)))) {
	if (TCL_ERROR == TclObjCallVarTraces(iPtr, arrayPtr, varPtr,
		part1Ptr, part2Ptr,
		TCL_TRACE_READS, (flags & TCL_LEAVE_ERR_MSG), index)) {
	    goto earlyError;
	}
    }

    /*
     * Set the variable's new value. If appending, append the new value to the
     * variable, either as a list element or as a string. Also, if appending,
     * then if the variable's old value is unshared we can modify it directly,
     * otherwise we must create a new copy to modify: this is "copy on write".
     */

    oldValuePtr = varPtr->value.objPtr;
    if (flags & TCL_LIST_ELEMENT && !(flags & TCL_APPEND_VALUE)) {
	varPtr->value.objPtr = NULL;
    }
    if (flags & (TCL_APPEND_VALUE|TCL_LIST_ELEMENT)) {
	if (flags & TCL_LIST_ELEMENT) {		/* Append list element. */
	    result = ListAppendInVar(interp, varPtr, arrayPtr, oldValuePtr,
		    newValuePtr);
	    if (result != TCL_OK) {
		goto earlyError;
	    }
	} else {				/* Append string. */
	    StringAppendInVar(varPtr, arrayPtr, oldValuePtr, newValuePtr);
	}
    } else if (newValuePtr != oldValuePtr) {
	/*
	 * In this case we are replacing the value, so we don't need to do
	 * more than swap the objects.
	 */

	varPtr->value.objPtr = newValuePtr;
	Tcl_IncrRefCount(newValuePtr);		/* Var is another ref. */
	if (oldValuePtr != NULL) {
	    TclDecrRefCount(oldValuePtr);	/* Discard old value. */
	}
    }

    /*
     * Invoke any write traces for the variable.
     */

    if ((varPtr->flags & VAR_TRACED_WRITE)
	    || (arrayPtr && (arrayPtr->flags & VAR_TRACED_WRITE))) {
	if (TCL_ERROR == TclObjCallVarTraces(iPtr, arrayPtr, varPtr, part1Ptr,
		part2Ptr, (flags & (TCL_GLOBAL_ONLY|TCL_NAMESPACE_ONLY))
		| TCL_TRACE_WRITES, (flags & TCL_LEAVE_ERR_MSG), index)) {
	    goto cleanup;
	}
    }

    /*
     * Return the variable's value unless the variable was changed in some
     * gross way by a trace (e.g. it was unset and then recreated as an
     * array).
     */

    if (TclIsVarScalar(varPtr) && !TclIsVarUndefined(varPtr)) {
	return varPtr->value.objPtr;
    }

    /*
     * A trace changed the value in some gross way. Return an empty string
     * object.
     */

    resultPtr = iPtr->emptyObjPtr;

    /*
     * If the variable doesn't exist anymore and no-one's using it, then free
     * up the relevant structures and hash table entries.
     */

  cleanup:
    if (resultPtr == NULL) {
	Tcl_SetErrorCode(interp, "TCL", "WRITE", "VARNAME", (char *)NULL);
    }
    if (TclIsVarUndefined(varPtr)) {
	TclCleanupVar(varPtr, arrayPtr);
    }
    return resultPtr;

  earlyError:
    if (cleanupOnEarlyError) {
	Tcl_DecrRefCount(newValuePtr);
    }
    goto cleanup;
}

/*
 *----------------------------------------------------------------------
 *
 * TclIncrObjVar2 --
 *
 *	Given a two-part variable name, which may refer either to a scalar
 *	variable or an element of an array, increment the Tcl object value of
 *	the variable by a specified Tcl_Obj increment value.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the specified variable doesn't exist, or there is a clash
 *	in array usage, or an error occurs while executing variable traces,
 *	then NULL is returned and a message will be left in the interpreter's
 *	result.
 *
 * Side effects:
 *	The value of the given variable is incremented by the specified
 *	amount. If either the array or the entry didn't exist then a new
 *	variable is created. The ref count for the returned object is _not_
 *	incremented to reflect the returned reference; if you want to keep a
 *	reference to the object you must increment its ref count yourself.
 *	Callers must Incr part1Ptr if they plan to Decr it.
 *	Callers must Incr part2Ptr if they plan to Decr it.
 *
 *----------------------------------------------------------------------
 */

Tcl_Obj *
TclIncrObjVar2(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Tcl_Obj *part1Ptr,		/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,		/* If non-null, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *incrPtr,		/* Amount to be added to variable. */
    int flags)			/* Various flags that tell how to incr value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
{
    Var *varPtr, *arrayPtr;

    varPtr = TclObjLookupVarEx(interp, part1Ptr, part2Ptr, flags, "read",
	    1, 1, &arrayPtr);
    if (varPtr == NULL) {
	Tcl_AddErrorInfo(interp,
		"\n    (reading value of variable to increment)");
	return NULL;
    }
    return TclPtrIncrObjVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    incrPtr, flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * TclPtrIncrObjVar --
 *
 *	Given the pointers to a variable and possible containing array,
 *	increment the Tcl object value of the variable by a Tcl_Obj increment.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the specified variable doesn't exist, or there is a clash
 *	in array usage, or an error occurs while executing variable traces,
 *	then NULL is returned and a message will be left in the interpreter's
 *	result.
 *
 * Side effects:
 *	The value of the given variable is incremented by the specified
 *	amount. If either the array or the entry didn't exist then a new
 *	variable is created. The ref count for the returned object is _not_
 *	incremented to reflect the returned reference; if you want to keep a
 *	reference to the object you must increment its ref count yourself.
 *
 *----------------------------------------------------------------------
 */

Tcl_Obj *
TclPtrIncrObjVar(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Tcl_Var varPtr,		/* Reference to the variable to set. */
    Tcl_Var arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,		/* If non-null, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *incrPtr,		/* Increment value. */
/* TODO: Which of these flag values really make sense? */
    int flags)			/* Various flags that tell how to incr value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
{
    if (varPtr == NULL) {
	Tcl_Panic("varPtr must not be NULL");
    }
    if (part1Ptr == NULL) {
	Tcl_Panic("part1Ptr must not be NULL");
    }
    return TclPtrIncrObjVarIdx(interp, (Var *) varPtr, (Var *) arrayPtr,
	    part1Ptr, part2Ptr, incrPtr, flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * TclPtrIncrObjVarIdx --
 *
 *	Given the pointers to a variable and possible containing array,
 *	increment the Tcl object value of the variable by a Tcl_Obj increment.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the specified variable doesn't exist, or there is a clash
 *	in array usage, or an error occurs while executing variable traces,
 *	then NULL is returned and a message will be left in the interpreter's

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclVar.c`, function `CleanupVar(`, lines 387–517. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-9.0.4-original-tclVar.c`.

```text
static inline void
CleanupVar(
    Var *varPtr,		/* Pointer to variable that may be a candidate
				 * for being expunged. */
    Var *arrayPtr)		/* Array that contains the variable, or NULL
				 * if this variable isn't an array element. */
{
    if (TclIsVarUndefined(varPtr) && TclIsVarInHash(varPtr)
	    && !TclIsVarTraced(varPtr)
	    && (VarHashRefCount(varPtr) == (Tcl_Size)
		    !TclIsVarDeadHash(varPtr))) {
	if (VarHashRefCount(varPtr) == 0) {
	    Tcl_Free(varPtr);
	} else {
	    VarHashDeleteEntry(varPtr);
	}
    }
    if (arrayPtr != NULL && TclIsVarUndefined(arrayPtr) &&
	    TclIsVarInHash(arrayPtr) && !TclIsVarTraced(arrayPtr) &&
	    (VarHashRefCount(arrayPtr) == (Tcl_Size)
		    !TclIsVarDeadHash(arrayPtr))) {
	if (VarHashRefCount(arrayPtr) == 0) {
	    Tcl_Free(arrayPtr);
	} else {
	    VarHashDeleteEntry(arrayPtr);
	}
    }
}

void
TclCleanupVar(
    Var *varPtr,		/* Pointer to variable that may be a candidate
				 * for being expunged. */
    Var *arrayPtr)		/* Array that contains the variable, or NULL
				 * if this variable isn't an array element. */
{
    CleanupVar(varPtr, arrayPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * TclLookupVar --
 *
 *	This function is used to locate a variable given its name(s). It has
 *	been mostly superseded by TclObjLookupVar, it is now only used by the
 *	trace code. It is kept in tcl9.0 mainly because it is in the internal
 *	stubs table, so that some extension may be calling it.
 *
 * Results:
 *	The return value is a pointer to the variable structure indicated by
 *	part1 and part2, or NULL if the variable couldn't be found. If the
 *	variable is found, *arrayPtrPtr is filled in with the address of the
 *	variable structure for the array that contains the variable (or NULL
 *	if the variable is a scalar). If the variable can't be found and
 *	either createPart1 or createPart2 are 1, a new as-yet-undefined
 *	(VAR_UNDEFINED) variable structure is created, entered into a hash
 *	table, and returned.
 *
 *	If the variable isn't found and creation wasn't specified, or some
 *	other error occurs, NULL is returned and an error message is left in
 *	the interp's result if TCL_LEAVE_ERR_MSG is set in flags.
 *
 *	Note: it's possible for the variable returned to be VAR_UNDEFINED even
 *	if createPart1 or createPart2 are 1 (these only cause the hash table
 *	entry or array to be created). For example, the variable might be a
 *	global that has been unset but is still referenced by a procedure, or
 *	a variable that has been unset but it only being kept in existence (if
 *	VAR_UNDEFINED) by a trace.
 *
 * Side effects:
 *	New hashtable entries may be created if createPart1 or createPart2
 *	are 1.
 *
 *----------------------------------------------------------------------
 */

Var *
TclLookupVar(
    Tcl_Interp *interp,		/* Interpreter to use for lookup. */
    const char *part1,		/* If part2 isn't NULL, this is the name of an
				 * array. Otherwise, this is a full variable
				 * name that could include a parenthesized
				 * array element. */
    const char *part2,		/* Name of element within array, or NULL. */
    int flags,			/* Only TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * and TCL_LEAVE_ERR_MSG bits matter. */
    const char *msg,		/* Verb to use in error messages, e.g. "read"
				 * or "set". Only needed if TCL_LEAVE_ERR_MSG
				 * is set in flags. */
    int createPart1,		/* If 1, create hash table entry for part 1 of
				 * name, if it doesn't already exist. If 0,
				 * return error if it doesn't exist. */
    int createPart2,		/* If 1, create hash table entry for part 2 of
				 * name, if it doesn't already exist. If 0,
				 * return error if it doesn't exist. */
    Var **arrayPtrPtr)		/* If the name refers to an element of an
				 * array, *arrayPtrPtr gets filled in with
				 * address of array variable. Otherwise this
				 * is set to NULL. */
{
    Var *varPtr;
    Tcl_Obj *part1Ptr = Tcl_NewStringObj(part1, -1);

    if (createPart1) {
	Tcl_IncrRefCount(part1Ptr);
    }

    varPtr = TclObjLookupVar(interp, part1Ptr, part2, flags, msg,
	    createPart1, createPart2, arrayPtrPtr);

    TclDecrRefCount(part1Ptr);
    return varPtr;
}

/*
 *----------------------------------------------------------------------
 *
 * TclObjLookupVar, TclObjLookupVarEx --
 *
 *	This function is used by virtually all of the variable code to locate
 *	a variable given its name(s). The parsing into array/element
 *	components and (if possible) the lookup results are cached in
 *	part1Ptr, which is converted to one of the varNameTypes.
 *
 * Results:
 *	The return value is a pointer to the variable structure indicated by
 *	part1Ptr and part2, or NULL if the variable couldn't be found. If *
 *	the variable is found, *arrayPtrPtr is filled with the address of the
 *	variable structure for the array that contains the variable (or NULL
 *	if the variable is a scalar). If the variable can't be found and

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclVar.c`, function `Tcl_UnsetVar2(`, lines 2323–2443. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-9.0.4-original-tclVar.c`.

```text
int
Tcl_UnsetVar2(
    Tcl_Interp *interp,		/* Command interpreter in which varName is to
				 * be looked up. */
    const char *part1,		/* Name of variable or array. */
    const char *part2,		/* Name of element within array or NULL. */
    int flags)			/* OR-ed combination of any of
				 * TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_LEAVE_ERR_MSG. */
{
    int result;
    Tcl_Obj *part2Ptr = NULL, *part1Ptr = Tcl_NewStringObj(part1, -1);

    if (part2) {
	part2Ptr = Tcl_NewStringObj(part2, -1);
    }

    /*
     * Filter to pass through only the flags this interface supports.
     */

    flags &= (TCL_GLOBAL_ONLY|TCL_NAMESPACE_ONLY|TCL_LEAVE_ERR_MSG);
    result = TclObjUnsetVar2(interp, part1Ptr, part2Ptr, flags);

    Tcl_DecrRefCount(part1Ptr);
    if (part2Ptr) {
	Tcl_DecrRefCount(part2Ptr);
    }
    return result;
}

/*
 *----------------------------------------------------------------------
 *
 * TclObjUnsetVar2 --
 *
 *	Delete a variable, given a 2-object name.
 *
 * Results:
 *	Returns TCL_OK if the variable was successfully deleted, TCL_ERROR if
 *	the variable can't be unset. In the event of an error, if the
 *	TCL_LEAVE_ERR_MSG flag is set then an error message is left in the
 *	interp's result.
 *
 * Side effects:
 *	If part1ptr and part2Ptr indicate a local or global variable in
 *	interp, it is deleted. If part1Ptr is an array name and part2Ptr is
 *	NULL, then the whole array is deleted.
 *
 *----------------------------------------------------------------------
 */

int
TclObjUnsetVar2(
    Tcl_Interp *interp,		/* Command interpreter in which varName is to
				 * be looked up. */
    Tcl_Obj *part1Ptr,		/* Name of variable or array. */
    Tcl_Obj *part2Ptr,		/* Name of element within array or NULL. */
    int flags)			/* OR-ed combination of any of
				 * TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_LEAVE_ERR_MSG. */
{
    Var *varPtr, *arrayPtr;

    varPtr = TclObjLookupVarEx(interp, part1Ptr, part2Ptr, flags, "unset",
	    /*createPart1*/ 0, /*createPart2*/ 0, &arrayPtr);
    if (varPtr == NULL) {
	return TCL_ERROR;
    }

    return TclPtrUnsetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * TclPtrUnsetVar --
 *
 *	Delete a variable, given the pointers to the variable's (and possibly
 *	containing array's) VAR structure.
 *
 * Results:
 *	Returns TCL_OK if the variable was successfully deleted, TCL_ERROR if
 *	the variable can't be unset. In the event of an error, if the
 *	TCL_LEAVE_ERR_MSG flag is set then an error message is left in the
 *	interp's result.
 *
 * Side effects:
 *	If varPtr and arrayPtr indicate a local or global variable in interp,
 *	it is deleted. If varPtr is an array reference and part2Ptr is NULL,
 *	then the whole array is deleted.
 *
 *----------------------------------------------------------------------
 */

int
TclPtrUnsetVar(
    Tcl_Interp *interp,		/* Command interpreter in which varName is to
				 * be looked up. */
    Tcl_Var varPtr,		/* The variable to be unset. */
    Tcl_Var arrayPtr,		/* NULL for scalar variables, pointer to the
				 * containing array otherwise. */
    Tcl_Obj *part1Ptr,		/* Name of an array (if part2 is non-NULL) or
				 * the name of a variable. */
    Tcl_Obj *part2Ptr,		/* If non-NULL, gives the name of an element
				 * in the array part1. */
    int flags)			/* OR-ed combination of any of
				 * TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_LEAVE_ERR_MSG. */
{
    if (varPtr == NULL) {
	Tcl_Panic("varPtr must not be NULL");
    }
    if (part1Ptr == NULL) {
	Tcl_Panic("part1Ptr must not be NULL");
    }
    return TclPtrUnsetVarIdx(interp, (Var *) varPtr, (Var *) arrayPtr,
	    part1Ptr, part2Ptr, flags, -1);
}


```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInt.h`, function `#define TclSetVarConstant`, lines 816–826. Retained reference `rust-tcl-registry-tests-data-native_ensemble_holder_routing205-source-inspection208-9.0.4-tclInt.h`.

```text

#define TclSetVarConstant(varPtr) \
    (varPtr)->flags = ((varPtr)->flags & ~(VAR_ARRAY|VAR_LINK)) | VAR_CONSTANT

#define TclSetVarArrayElement(varPtr) \
    (varPtr)->flags = ((varPtr)->flags & ~VAR_ARRAY) | VAR_ARRAY_ELEMENT

#define TclSetVarUndefined(varPtr) \
    (varPtr)->flags &= ~(VAR_ARRAY|VAR_LINK|VAR_CONSTANT);\
    (varPtr)->value.objPtr = NULL

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclVar.c`, function `Tcl_ObjSetVar2(`, lines 1672–1777. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-9.0.4-original-tclVar.c`.

```text
Tcl_Obj *
Tcl_ObjSetVar2(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Tcl_Obj *part1Ptr,		/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,		/* If non-NULL, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *newValuePtr,	/* New value for variable. */
    int flags)			/* Various flags that tell how to set value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT, or
				 * TCL_LEAVE_ERR_MSG. */
{
    Var *varPtr, *arrayPtr;

    /*
     * Filter to pass through only the flags this interface supports.
     */

    flags &= (TCL_GLOBAL_ONLY|TCL_NAMESPACE_ONLY|TCL_LEAVE_ERR_MSG
	    |TCL_APPEND_VALUE|TCL_LIST_ELEMENT);
    varPtr = TclObjLookupVarEx(interp, part1Ptr, part2Ptr, flags, "set",
	    /*createPart1*/ 1, /*createPart2*/ 1, &arrayPtr);
    if (varPtr == NULL) {
	if (newValuePtr->refCount == 0) {
	    Tcl_DecrRefCount(newValuePtr);
	}
	return NULL;
    }

    return TclPtrSetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    newValuePtr, flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * TclPtrSetVar --
 *
 *	This function is the same as Tcl_SetVar2Ex above, except that it
 *	requires pointers to the variable's Var structs in addition to the
 *	variable names.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the write operation was disallowed because an array was
 *	expected but not found (or vice versa), then NULL is returned; if the
 *	TCL_LEAVE_ERR_MSG flag is set, then an explanatory message will be
 *	left in the interpreter's result. Note that the returned object may
 *	not be the same one referenced by newValuePtr; this is because
 *	variable traces may modify the variable's value.
 *
 * Side effects:
 *	The value of the given variable is set. If either the array or the
 *	entry didn't exist then a new variable is created.
 *
 *----------------------------------------------------------------------
 */

Tcl_Obj *
TclPtrSetVar(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be looked up. */
    Tcl_Var varPtr,		/* Reference to the variable to set. */
    Tcl_Var arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Name of an array (if part2 is non-NULL) or
				 * the name of a variable. */
    Tcl_Obj *part2Ptr,		/* If non-NULL, gives the name of an element
				 * in the array part1. */
    Tcl_Obj *newValuePtr,	/* New value for variable. */
    int flags)			/* OR-ed combination of TCL_GLOBAL_ONLY, and
				 * TCL_LEAVE_ERR_MSG bits. */
{
    if (varPtr == NULL) {
	Tcl_Panic("varPtr must not be NULL");
    }
    if (part1Ptr == NULL) {
	Tcl_Panic("part1Ptr must not be NULL");
    }
    if (newValuePtr == NULL) {
	Tcl_Panic("newValuePtr must not be NULL");
    }
    return TclPtrSetVarIdx(interp, (Var *) varPtr, (Var *) arrayPtr,
	    part1Ptr, part2Ptr, newValuePtr, flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * ListAppendInVar, StringAppendInVar --
 *
 *	Support functions for TclPtrSetVarIdx that implement various types of
 *	appending operations.
 *
 * Results:
 *	ListAppendInVar returns a Tcl result code (from the core list append
 *	operation). StringAppendInVar has no return value.
 *
 * Side effects:
 *	The variable or element of the array is updated. This may make the
 *	variable/element exist. Reference counts of values may be updated.

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `Tcl_EvalObjv(`, lines 4471–4581. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.0.4-tclBasic.c`.

```text
int
Tcl_EvalObjv(
    Tcl_Interp *interp,		/* Interpreter in which to evaluate the
				 * command. Also used for error reporting. */
    Tcl_Size objc,		/* Number of words in command. */
    Tcl_Obj *const objv[],	/* An array of pointers to objects that are
				 * the words that make up the command. */
    int flags)			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Only
				 * TCL_EVAL_GLOBAL, TCL_EVAL_INVOKE and
				 * TCL_EVAL_NOERR are currently supported. */
{
    int result;
    NRE_callback *rootPtr = TOP_CB(interp);

    result = TclNREvalObjv(interp, objc, objv, flags, NULL);
    return TclNRRunCallbacks(interp, result, rootPtr);
}

int
TclNREvalObjv(
    Tcl_Interp *interp,		/* Interpreter in which to evaluate the
				 * command. Also used for error reporting. */
    Tcl_Size objc,		/* Number of words in command. */
    Tcl_Obj *const objv[],	/* An array of pointers to objects that are
				 * the words that make up the command. */
    int flags,			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Only
				 * TCL_EVAL_GLOBAL, TCL_EVAL_INVOKE and
				 * TCL_EVAL_NOERR are currently supported. */
    Command *cmdPtr)		/* NULL if the Command is to be looked up
				 * here, otherwise the pointer to the
				 * requested Command struct to be invoked. */
{
    Interp *iPtr = (Interp *) interp;

    /*
     * data[1] stores a marker for use by tailcalls; it will be set to 1 by
     * command redirectors (imports, alias, ensembles) so that tailcall skips
     * this callback (that marks the end of the target command) and goes back
     * to the end of the source command.
     */

    if (iPtr->deferredCallbacks) {
	iPtr->deferredCallbacks = NULL;
    } else {
	TclNRAddCallback(interp, NRCommand, NULL, NULL, NULL, NULL);
    }

    iPtr->numLevels++;
    TclNRAddCallback(interp, EvalObjvCore, cmdPtr, INT2PTR(flags),
	    INT2PTR(objc), objv);
    return TCL_OK;
}

static int
EvalObjvCore(
    void *data[],
    Tcl_Interp *interp,
    TCL_UNUSED(int) /*result*/)
{
    Command *cmdPtr = NULL, *preCmdPtr = (Command *)data[0];
    int flags = (int)PTR2INT(data[1]);
    Tcl_Size objc = PTR2INT(data[2]);
    Tcl_Obj **objv = (Tcl_Obj **)data[3];
    Interp *iPtr = (Interp *) interp;
    Namespace *lookupNsPtr = NULL;
    int enterTracesDone = 0;

    /*
     * Push records for task to be done on return, in INVERSE order. First, if
     * needed, the exception handlers (as they should happen last).
     */

    if (!(flags & TCL_EVAL_NOERR)) {
	TEOV_PushExceptionHandlers(interp, objc, objv, flags);
    }

    if (TCL_OK != TclInterpReady(interp)) {
	return TCL_ERROR;
    }

    if (objc == 0) {
	return TCL_OK;
    }

    if (TclLimitExceeded(iPtr->limit)) {
	/* generate error message if not yet already logged at this stage */
	if (!(iPtr->flags & ERR_ALREADY_LOGGED)) {
	    Tcl_LimitCheck(interp);
	}
	return TCL_ERROR;
    }

    /*
     * Configure evaluation context to match the requested flags.
     */

    if (iPtr->lookupNsPtr) {

	/*
	 * Capture the namespace we should do command name resolution in, as
	 * instructed by our caller sneaking it in to us in a private interp
	 * field.  Clear that field right away so we cannot possibly have its
	 * use leak where it should not.  The sneaky message pass is done.
	 *
	 * Use of this mechanism overrides the TCL_EVAL_GLOBAL flag.
	 * TODO: Is that a bug?
	 */

	lookupNsPtr = iPtr->lookupNsPtr;

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdIL.c`, function `InfoDefaultCmd(`, lines 925–1037. Retained reference `rust-tcl-registry-tests-data-native_info_inventory_original-sources-9.1.0-tclCmdIL.c`.

```text
static int
InfoDefaultCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Interp *iPtr = (Interp *) interp;
    const char *procName, *argName;
    Proc *procPtr;
    CompiledLocal *localPtr;
    Tcl_Obj *valueObjPtr;

    if (objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "procname arg varname");
	return TCL_ERROR;
    }

    procName = TclGetString(objv[1]);
    argName = TclGetString(objv[2]);

    procPtr = TclFindProc(iPtr, procName);
    if (procPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"\"%s\" isn't a procedure", procName));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "PROCEDURE", procName,
		(char *)NULL);
	return TCL_ERROR;
    }

    for (localPtr = procPtr->firstLocalPtr;  localPtr != NULL;
	    localPtr = localPtr->nextPtr) {
	if (TclIsVarArgument(localPtr)
		&& (strcmp(argName, localPtr->name) == 0)) {
	    if (localPtr->defValuePtr != NULL) {
		valueObjPtr = Tcl_ObjSetVar2(interp, objv[3], NULL,
			localPtr->defValuePtr, TCL_LEAVE_ERR_MSG);
		if (valueObjPtr == NULL) {
		    return TCL_ERROR;
		}
		Tcl_SetObjResult(interp, Tcl_NewBooleanObj(1));
	    } else {
		Tcl_Obj *nullObjPtr;
		TclNewObj(nullObjPtr);

		valueObjPtr = Tcl_ObjSetVar2(interp, objv[3], NULL,
			nullObjPtr, TCL_LEAVE_ERR_MSG);
		if (valueObjPtr == NULL) {
		    return TCL_ERROR;
		}
		Tcl_SetObjResult(interp, Tcl_NewBooleanObj(0));
	    }
	    return TCL_OK;
	}
    }

    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
	    "procedure \"%s\" doesn't have an argument \"%s\"",
	    procName, argName));
    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "ARGUMENT", argName, (char *)NULL);
    return TCL_ERROR;
}

/*
 *----------------------------------------------------------------------
 *
 * InfoErrorStackCmd --
 *
 *	Called to implement the "info errorstack" command that returns information
 *	about the last error's call stack. Handles the following syntax:
 *
 *	    info errorstack ?interp?
 *
 * Results:
 *	Returns TCL_OK if successful and TCL_ERROR if there is an error.
 *
 * Side effects:
 *	Returns a result in the interpreter's result object. If there is an
 *	error, the result is an error message.
 *
 *----------------------------------------------------------------------
 */

static int
InfoErrorStackCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Interp *target;
    Interp *iPtr;

    if ((objc != 1) && (objc != 2)) {
	Tcl_WrongNumArgs(interp, 1, objv, "?interp?");
	return TCL_ERROR;
    }

    target = interp;
    if (objc == 2) {
	target = Tcl_GetChild(interp, TclGetString(objv[1]));
	if (target == NULL) {
	    return TCL_ERROR;
	}
    }

    iPtr = (Interp *) target;
    Tcl_SetObjResult(interp, iPtr->errorStack);

    return TCL_OK;
}

/*

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdIL.c`, function `Tcl_IncrObjCmd(`, lines 388–508. Retained reference `rust-tcl-registry-tests-data-native_info_inventory_original-sources-9.1.0-tclCmdIL.c`.

```text
int
Tcl_IncrObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Obj *newValuePtr, *incrPtr;

    if ((objc != 2) && (objc != 3)) {
	Tcl_WrongNumArgs(interp, 1, objv, "varName ?increment?");
	return TCL_ERROR;
    }

    if (objc == 3) {
	incrPtr = objv[2];
    } else {
	TclNewIntObj(incrPtr, 1);
    }
    Tcl_IncrRefCount(incrPtr);
    newValuePtr = TclIncrObjVar2(interp, objv[1], NULL,
	    incrPtr, TCL_LEAVE_ERR_MSG);
    Tcl_DecrRefCount(incrPtr);

    if (newValuePtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * Set the interpreter's object result to refer to the variable's new
     * value object.
     */

    Tcl_SetObjResult(interp, newValuePtr);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * InfoArgsCmd --
 *
 *	Called to implement the "info args" command that returns the argument
 *	list for a procedure. Handles the following syntax:
 *
 *	    info args procName
 *
 * Results:
 *	Returns TCL_OK if successful and TCL_ERROR if there is an error.
 *
 * Side effects:
 *	Returns a result in the interpreter's result object. If there is an
 *	error, the result is an error message.
 *
 *----------------------------------------------------------------------
 */

static int
InfoArgsCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Interp *iPtr = (Interp *) interp;
    const char *name;
    Proc *procPtr;
    CompiledLocal *localPtr;
    Tcl_Obj *listObjPtr;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "procname");
	return TCL_ERROR;
    }

    name = TclGetString(objv[1]);
    procPtr = TclFindProc(iPtr, name);
    if (procPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"\"%s\" isn't a procedure", name));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "PROCEDURE", name, (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Build a return list containing the arguments.
     */

    listObjPtr = Tcl_NewListObj(0, NULL);
    for (localPtr = procPtr->firstLocalPtr;  localPtr != NULL;
	    localPtr = localPtr->nextPtr) {
	if (TclIsVarArgument(localPtr)) {
	    Tcl_ListObjAppendElement(interp, listObjPtr,
		    Tcl_NewStringObj(localPtr->name, -1));
	}
    }
    Tcl_SetObjResult(interp, listObjPtr);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * InfoBodyCmd --
 *
 *	Called to implement the "info body" command that returns the body for
 *	a procedure. Handles the following syntax:
 *
 *	    info body procName
 *
 * Results:
 *	Returns TCL_OK if successful and TCL_ERROR if there is an error.
 *
 * Side effects:
 *	Returns a result in the interpreter's result object. If there is an
 *	error, the result is an error message.
 *
 *----------------------------------------------------------------------
 */

static int

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclVar.c`, function `Tcl_UnsetObjCmd(`, lines 2796–2881. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-9.1.0-original-tclVar.c`.

```text
int
Tcl_UnsetObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Size i;
    int flags = TCL_LEAVE_ERR_MSG;
    const char *name;

    if (objc == 1) {
	/*
	 * Do nothing if no arguments supplied, so as to match command
	 * documentation.
	 */

	return TCL_OK;
    }

    /*
     * Simple, restrictive argument parsing. The only options are -- and
     * -nocomplain (which must come first and be given exactly to be an
     * option).
     */

    i = 1;
    name = TclGetString(objv[i]);
    if (name[0] == '-') {
	if (strcmp("-nocomplain", name) == 0) {
	    i++;
	    if (i == objc) {
		return TCL_OK;
	    }
	    flags = 0;
	    name = TclGetString(objv[i]);
	}
	if (strcmp("--", name) == 0) {
	    i++;
	}
    }

    for (; i < objc; i++) {
	if ((TclObjUnsetVar2(interp, objv[i], NULL, flags) != TCL_OK)
		&& (flags == TCL_LEAVE_ERR_MSG)) {
	    return TCL_ERROR;
	}
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_AppendObjCmd --
 *
 *	This object-based function is invoked to process the "append" Tcl
 *	command. See the user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl object result value.
 *
 * Side effects:
 *	A variable's value may be changed.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_AppendObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Var *varPtr, *arrayPtr;
    Tcl_Obj *varValuePtr = NULL;
				/* Initialized to avoid compiler warning. */
    Tcl_Size i;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "varName ?value ...?");
	return TCL_ERROR;
    }

    if (objc == 2) {

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclVar.c`, function `TclIncrObjVar2(`, lines 2169–2269. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-9.1.0-original-tclVar.c`.

```text
Tcl_Obj *
TclIncrObjVar2(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Tcl_Obj *part1Ptr,		/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,		/* If non-null, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *incrPtr,		/* Amount to be added to variable. */
    int flags)			/* Various flags that tell how to incr value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
{
    Var *varPtr, *arrayPtr;

    varPtr = TclObjLookupVarEx(interp, part1Ptr, part2Ptr, flags, "read",
	    1, 1, &arrayPtr);
    if (varPtr == NULL) {
	Tcl_AddErrorInfo(interp,
		"\n    (reading value of variable to increment)");
	return NULL;
    }
    return TclPtrIncrObjVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    incrPtr, flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * TclPtrIncrObjVar --
 *
 *	Given the pointers to a variable and possible containing array,
 *	increment the Tcl object value of the variable by a Tcl_Obj increment.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the specified variable doesn't exist, or there is a clash
 *	in array usage, or an error occurs while executing variable traces,
 *	then NULL is returned and a message will be left in the interpreter's
 *	result.
 *
 * Side effects:
 *	The value of the given variable is incremented by the specified
 *	amount. If either the array or the entry didn't exist then a new
 *	variable is created. The ref count for the returned object is _not_
 *	incremented to reflect the returned reference; if you want to keep a
 *	reference to the object you must increment its ref count yourself.
 *
 *----------------------------------------------------------------------
 */

Tcl_Obj *
TclPtrIncrObjVar(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Tcl_Var varPtr,		/* Reference to the variable to set. */
    Tcl_Var arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,		/* If non-null, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *incrPtr,		/* Increment value. */
/* TODO: Which of these flag values really make sense? */
    int flags)			/* Various flags that tell how to incr value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
{
    if (varPtr == NULL) {
	Tcl_Panic("varPtr must not be NULL");
    }
    if (part1Ptr == NULL) {
	Tcl_Panic("part1Ptr must not be NULL");
    }
    return TclPtrIncrObjVarIdx(interp, (Var *) varPtr, (Var *) arrayPtr,
	    part1Ptr, part2Ptr, incrPtr, flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * TclPtrIncrObjVarIdx --
 *
 *	Given the pointers to a variable and possible containing array,
 *	increment the Tcl object value of the variable by a Tcl_Obj increment.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the specified variable doesn't exist, or there is a clash
 *	in array usage, or an error occurs while executing variable traces,
 *	then NULL is returned and a message will be left in the interpreter's
 *	result.
 *
 * Side effects:

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclVar.c`, function `TclPtrIncrObjVar(`, lines 2223–2383. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-9.1.0-original-tclVar.c`.

```text
Tcl_Obj *
TclPtrIncrObjVar(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Tcl_Var varPtr,		/* Reference to the variable to set. */
    Tcl_Var arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,		/* If non-null, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *incrPtr,		/* Increment value. */
/* TODO: Which of these flag values really make sense? */
    int flags)			/* Various flags that tell how to incr value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
{
    if (varPtr == NULL) {
	Tcl_Panic("varPtr must not be NULL");
    }
    if (part1Ptr == NULL) {
	Tcl_Panic("part1Ptr must not be NULL");
    }
    return TclPtrIncrObjVarIdx(interp, (Var *) varPtr, (Var *) arrayPtr,
	    part1Ptr, part2Ptr, incrPtr, flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * TclPtrIncrObjVarIdx --
 *
 *	Given the pointers to a variable and possible containing array,
 *	increment the Tcl object value of the variable by a Tcl_Obj increment.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the specified variable doesn't exist, or there is a clash
 *	in array usage, or an error occurs while executing variable traces,
 *	then NULL is returned and a message will be left in the interpreter's
 *	result.
 *
 * Side effects:
 *	The value of the given variable is incremented by the specified
 *	amount. If either the array or the entry didn't exist then a new
 *	variable is created. The ref count for the returned object is _not_
 *	incremented to reflect the returned reference; if you want to keep a
 *	reference to the object you must increment its ref count yourself.
 *
 *----------------------------------------------------------------------
 */

Tcl_Obj *
TclPtrIncrObjVarIdx(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Var *varPtr,		/* Reference to the variable to set. */
    Var *arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,		/* If non-null, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *incrPtr,		/* Increment value. */
/* TODO: Which of these flag values really make sense? */
    int flags,			/* Various flags that tell how to incr value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
    Tcl_Size index)		/* Index into the local variable table of the
				 * variable, or -1. Only used when part1Ptr is
				 * NULL. */
{
    Tcl_Obj *varValuePtr;

    /*
     * It's an error to try to increment a constant.
     */
    if (TclIsVarConstant(varPtr)) {
	if (flags & TCL_LEAVE_ERR_MSG) {
	    TclObjVarErrMsg(interp, part1Ptr, part2Ptr, "incr", ISCONST,index);
	    Tcl_SetErrorCode(interp, "TCL", "WRITE", "CONST", (char *)NULL);
	}
	return NULL;
    }

    if (TclIsVarInHash(varPtr)) {
	VarHashRefCount(varPtr)++;
    }
    varValuePtr = TclPtrGetVarIdx(interp, varPtr, arrayPtr, part1Ptr,
	    part2Ptr, flags, index);
    if (TclIsVarInHash(varPtr)) {
	VarHashRefCount(varPtr)--;
    }
    if (varValuePtr == NULL) {
	TclNewIntObj(varValuePtr, 0);
    }
    if (Tcl_IsShared(varValuePtr)) {
	/* Copy on write */
	varValuePtr = Tcl_DuplicateObj(varValuePtr);

	if (TCL_OK == TclIncrObj(interp, varValuePtr, incrPtr)) {
	    return TclPtrSetVarIdx(interp, varPtr, arrayPtr, part1Ptr,
		    part2Ptr, varValuePtr, flags, index);
	} else {
	    Tcl_DecrRefCount(varValuePtr);
	    return NULL;
	}
    } else {
	/* Unshared - can Incr in place */
	if (TCL_OK == TclIncrObj(interp, varValuePtr, incrPtr)) {
	    /*
	     * This seems dumb to write the incremeted value into the var
	     * after we just adjusted the value in place, but the spec for
	     * [incr] requires that write traces fire, and making this call
	     * is the way to make that happen.
	     */

	    return TclPtrSetVarIdx(interp, varPtr, arrayPtr, part1Ptr,
		    part2Ptr, varValuePtr, flags, index);
	} else {
	    return NULL;
	}
    }
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_UnsetVar2 --
 *
 *	Delete a variable, given a 2-part name.
 *
 * Results:
 *	Returns TCL_OK if the variable was successfully deleted, TCL_ERROR if
 *	the variable can't be unset. In the event of an error, if the
 *	TCL_LEAVE_ERR_MSG flag is set then an error message is left in the
 *	interp's result.
 *
 * Side effects:
 *	If part1 and part2 indicate a local or global variable in interp, it
 *	is deleted. If part1 is an array name and part2 is NULL, then the
 *	whole array is deleted.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_UnsetVar2(
    Tcl_Interp *interp,		/* Command interpreter in which varName is to
				 * be looked up. */
    const char *part1,		/* Name of variable or array. */
    const char *part2,		/* Name of element within array or NULL. */
    int flags)			/* OR-ed combination of any of

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclVar.c`, function `ArraySetCmd(`, lines 4068–4288. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-9.1.0-original-tclVar.c`.

```text
static int
ArraySetCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Tcl_Obj *arrayNameObj;
    Tcl_Obj *arrayElemObj;
    Var *varPtr, *arrayPtr;
    int result;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "arrayName list");
	return TCL_ERROR;
    }

    if (TCL_ERROR == LocateArray(interp, objv[1], NULL, NULL)) {
	return TCL_ERROR;
    }

    arrayNameObj = objv[1];
    varPtr = TclObjLookupVarEx(interp, arrayNameObj, NULL,
	    /*flags*/ TCL_LEAVE_ERR_MSG, /*msg*/ "set", /*createPart1*/ 1,
	    /*createPart2*/ 1, &arrayPtr);
    if (varPtr == NULL) {
	return TCL_ERROR;
    }
    if (arrayPtr) {
	CleanupVar(varPtr, arrayPtr);
	TclObjVarErrMsg(interp, arrayNameObj, NULL, "set", NEEDARRAY, -1);
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "VARNAME",
		TclGetString(arrayNameObj), (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Install the contents of the dictionary or list into the array.
     */

    arrayElemObj = objv[2];
    if (TclHasInternalRep(arrayElemObj, &tclDictType) && arrayElemObj->bytes == NULL) {
	Tcl_Obj *keyPtr, *valuePtr;
	Tcl_DictSearch search;
	int done;
	Tcl_Size size;

	if (Tcl_DictObjSize(interp, arrayElemObj, &size) != TCL_OK) {
	    return TCL_ERROR;
	}
	if (size == 0) {
	    /*
	     * Empty, so we'll just force the array to be properly existing
	     * instead.
	     */

	    goto ensureArray;
	}

	/*
	 * Don't need to look at result of Tcl_DictObjFirst as we've just
	 * successfully used a dictionary operation on the same object.
	 */

	for (Tcl_DictObjFirst(interp, arrayElemObj, &search,
		&keyPtr, &valuePtr, &done) ; !done ;
		Tcl_DictObjNext(&search, &keyPtr, &valuePtr, &done)) {
	    /*
	     * At this point, it would be nice if the key was directly usable
	     * by the array. This isn't the case though.
	     */

	    Var *elemVarPtr = TclLookupArrayElement(interp, arrayNameObj,
		    keyPtr, TCL_LEAVE_ERR_MSG, "set", 1, 1, varPtr, -1);

	    if ((elemVarPtr == NULL) ||
		    (TclPtrSetVarIdx(interp, elemVarPtr, varPtr, arrayNameObj,
		    keyPtr, valuePtr, TCL_LEAVE_ERR_MSG, -1) == NULL)) {
		Tcl_DictObjDone(&search);
		return TCL_ERROR;
	    }
	}
	return TCL_OK;
    } else {
	/*
	 * Not a dictionary, so assume (and convert to, for backward-
	 * -compatibility reasons) a list.
	 */

	Tcl_Size elemLen;
	Tcl_Obj **elemPtrs, *copyListObj;
	Tcl_Size i;

	result = TclListObjLength(interp, arrayElemObj, &elemLen);
	if (result != TCL_OK) {
	    return result;
	}
	if (elemLen & 1) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "list must have an even number of elements", -1));
	    Tcl_SetErrorCode(interp, "TCL", "ARGUMENT", "FORMAT", (char *)NULL);
	    return TCL_ERROR;
	}
	if (elemLen == 0) {
	    goto ensureArray;
	}
	result = TclListObjGetElements(interp, arrayElemObj,
		&elemLen, &elemPtrs);
	if (result != TCL_OK) {
	    return result;
	}

	/*
	 * We needn't worry about traces invalidating arrayPtr: should that be
	 * the case, TclPtrSetVarIdx will return NULL so that we break out of
	 * the loop and return an error.
	 */

	copyListObj = TclListObjCopy(NULL, arrayElemObj);
	if (!copyListObj) {
	    return TCL_ERROR;
	}
	for (i=0 ; i<elemLen ; i+=2) {
	    Var *elemVarPtr = TclLookupArrayElement(interp, arrayNameObj,
		    elemPtrs[i], TCL_LEAVE_ERR_MSG, "set", 1, 1, varPtr, -1);

	    if ((elemVarPtr == NULL) ||
		    (TclPtrSetVarIdx(interp, elemVarPtr, varPtr, arrayNameObj,
			    elemPtrs[i], elemPtrs[i+1], TCL_LEAVE_ERR_MSG,
			    -1) == NULL)) {
		result = TCL_ERROR;
		break;
	    }
	}
	Tcl_DecrRefCount(copyListObj);
	return result;
    }

    /*
     * The list is empty make sure we have an array, or create one if
     * necessary.
     */

  ensureArray:
    if (varPtr != NULL) {
	if (TclIsVarArray(varPtr)) {
	    /*
	     * Already an array, done.
	     */

	    return TCL_OK;
	}
	if (TclIsVarArrayElement(varPtr) || !TclIsVarUndefined(varPtr)) {
	    /*
	     * Either an array element, or a scalar: lose!
	     */

	    TclObjVarErrMsg(interp, arrayNameObj, NULL, "array set",
		    NEEDARRAY, -1);
	    Tcl_SetErrorCode(interp, "TCL", "WRITE", "ARRAY", (char *)NULL);
	    return TCL_ERROR;
	}
    }
    TclInitArrayVar(varPtr);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * ArraySizeCmd --
 *
 *	This object-based function is invoked to process the "array size" Tcl
 *	command. See the user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl result object.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

static int
ArraySizeCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Var *varPtr;
    Tcl_HashSearch search;
    Var *varPtr2;
    int isArray, size = 0;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "arrayName");
	return TCL_ERROR;
    }

    if (TCL_ERROR == LocateArray(interp, objv[1], &varPtr, &isArray)) {
	return TCL_ERROR;
    }

    /* We can only iterate over the array if it exists... */

    if (isArray) {
	/*
	 * Must iterate in order to get chance to check for present but
	 * "undefined" entries.
	 */

	for (varPtr2=VarHashFirstVar(varPtr->value.tablePtr, &search);
		varPtr2!=NULL ; varPtr2=VarHashNextVar(&search)) {
	    if (!TclIsVarUndefined(varPtr2)) {
		size++;
	    }
	}
    }

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclVar.c`, function `ArrayForNRCmd(`, lines 3145–3245. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-9.1.0-original-tclVar.c`.

```text
static int
ArrayForNRCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Tcl_Obj *varListObj, *arrayNameObj, *scriptObj;
    ArraySearch *searchPtr = NULL;
    Var *varPtr;
    int isArray;
    Tcl_Size numVars;

    /*
     * array for {k v} a body
     */

    if (objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "{key value} arrayName script");
	return TCL_ERROR;
    }

    /*
     * Parse arguments.
     */

    if (TclListObjLength(interp, objv[1], &numVars) != TCL_OK) {
	return TCL_ERROR;
    }

    if (numVars != 2) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"must have two variable names", -1));
	Tcl_SetErrorCode(interp, "TCL", "SYNTAX", "array", "for", (char *)NULL);
	return TCL_ERROR;
    }

    arrayNameObj = objv[2];

    if (TCL_ERROR == LocateArray(interp, arrayNameObj, &varPtr, &isArray)) {
	return TCL_ERROR;
    }

    if (!isArray) {
	return NotArrayError(interp, arrayNameObj);
    }

    /*
     * Make a new array search, put it on the stack.
     */

    searchPtr = (ArraySearch *)Tcl_Alloc(sizeof(ArraySearch));
    ArrayPopulateSearch(interp, arrayNameObj, varPtr, searchPtr);

    /*
     * Make sure that these objects (which we need throughout the body of the
     * loop) don't vanish.
     */

    varListObj = TclListObjCopy(NULL, objv[1]);
    if (!varListObj) {
	return TCL_ERROR;
    }
    scriptObj = objv[3];
    Tcl_IncrRefCount(scriptObj);

    /*
     * Run the script.
     */

    TclNRAddCallback(interp, ArrayForLoopCallback, searchPtr, varListObj,
	    arrayNameObj, scriptObj);
    return TCL_OK;
}

static int
ArrayForLoopCallback(
    void *data[],
    Tcl_Interp *interp,
    int result)
{
    Interp *iPtr = (Interp *) interp;
    ArraySearch *searchPtr = (ArraySearch *)data[0];
    Tcl_Obj *varListObj = (Tcl_Obj *)data[1];
    Tcl_Obj *arrayNameObj = (Tcl_Obj *)data[2];
    Tcl_Obj *scriptObj = (Tcl_Obj *)data[3];
    Tcl_Obj **varv;
    Tcl_Obj *keyObj, *valueObj;
    Var *varPtr;
    Var *arrayPtr;
    int done;
    Tcl_Size varc;

    /*
     * Process the result from the previous execution of the script body.
     */

    done = TCL_ERROR;

    if (result == TCL_CONTINUE) {
	result = TCL_OK;

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclVar.c`, function `ArrayForLoopCallback(`, lines 3220–3370. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-9.1.0-original-tclVar.c`.

```text
static int
ArrayForLoopCallback(
    void *data[],
    Tcl_Interp *interp,
    int result)
{
    Interp *iPtr = (Interp *) interp;
    ArraySearch *searchPtr = (ArraySearch *)data[0];
    Tcl_Obj *varListObj = (Tcl_Obj *)data[1];
    Tcl_Obj *arrayNameObj = (Tcl_Obj *)data[2];
    Tcl_Obj *scriptObj = (Tcl_Obj *)data[3];
    Tcl_Obj **varv;
    Tcl_Obj *keyObj, *valueObj;
    Var *varPtr;
    Var *arrayPtr;
    int done;
    Tcl_Size varc;

    /*
     * Process the result from the previous execution of the script body.
     */

    done = TCL_ERROR;

    if (result == TCL_CONTINUE) {
	result = TCL_OK;
    } else if (result != TCL_OK) {
	if (result == TCL_BREAK) {
	    Tcl_ResetResult(interp);
	    result = TCL_OK;
	} else if (result == TCL_ERROR) {
	    Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		    "\n    (\"array for\" body line %d)",
		    Tcl_GetErrorLine(interp)));
	}
	goto arrayfordone;
    }

    /*
     * Get the next mapping from the array.
     */

    keyObj = NULL;
    valueObj = NULL;
    varPtr = TclObjLookupVarEx(interp, arrayNameObj, NULL, /*flags*/ 0,
	    /*msg*/ 0, /*createPart1*/ 0, /*createPart2*/ 0, &arrayPtr);
    if (varPtr == NULL) {
	done = TCL_ERROR;
    } else {
	done = ArrayObjNext(interp, arrayNameObj, varPtr, searchPtr, &keyObj,
		&valueObj);
    }

    result = TCL_OK;
    if (done != TCL_CONTINUE) {
	Tcl_ResetResult(interp);
	if (done == TCL_ERROR) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "array changed during iteration", -1));
	    Tcl_SetErrorCode(interp, "TCL", "READ", "array", "for", (char *)NULL);
	    varPtr->flags |= TCL_LEAVE_ERR_MSG;
	    result = done;
	}
	goto arrayfordone;
    }

    result = TclListObjGetElements(NULL, varListObj, &varc, &varv);
    if (result != TCL_OK) {
	goto arrayfordone;
    }
    if (Tcl_ObjSetVar2(interp, varv[0], NULL, keyObj,
	    TCL_LEAVE_ERR_MSG) == NULL) {
	result = TCL_ERROR;
	goto arrayfordone;
    }
    if (valueObj != NULL) {
	if (Tcl_ObjSetVar2(interp, varv[1], NULL, valueObj,
		TCL_LEAVE_ERR_MSG) == NULL) {
	    result = TCL_ERROR;
	    goto arrayfordone;
	}
    }

    /*
     * Run the script.
     */

    TclNRAddCallback(interp, ArrayForLoopCallback, searchPtr, varListObj,
	    arrayNameObj, scriptObj);
    return TclNREvalObjEx(interp, scriptObj, 0, iPtr->cmdFramePtr, 3);

    /*
     * For unwinding everything once the iterating is done.
     */

  arrayfordone:
    if (done != TCL_ERROR) {
	/*
	 * If the search was terminated by an array change, the
	 * VAR_SEARCH_ACTIVE flag will no longer be set.
	 */

	ArrayDoneSearch(iPtr, varPtr, searchPtr);
	Tcl_DecrRefCount(searchPtr->name);
	Tcl_Free(searchPtr);
    }

    TclDecrRefCount(varListObj);
    TclDecrRefCount(scriptObj);
    return result;
}

/*
 *----------------------------------------------------------------------
 *
 * ArrayPopulateSearch --
 *
 *	Set up an already-allocated array search iterator structure.
 *
 *----------------------------------------------------------------------
 */
static void
ArrayPopulateSearch(
    Tcl_Interp *interp,
    Tcl_Obj *arrayNameObj,
    Var *varPtr,
    ArraySearch *searchPtr)
{
    Interp *iPtr = (Interp *) interp;
    Tcl_HashEntry *hPtr;
    int isNew;

    hPtr = Tcl_CreateHashEntry(&iPtr->varSearches, varPtr, &isNew);
    if (isNew) {
	searchPtr->id = 1;
	varPtr->flags |= VAR_SEARCH_ACTIVE;
	searchPtr->nextPtr = NULL;
    } else {
	searchPtr->id = ((ArraySearch *) Tcl_GetHashValue(hPtr))->id + 1;
	searchPtr->nextPtr = (ArraySearch *)Tcl_GetHashValue(hPtr);
    }
    searchPtr->varPtr = varPtr;
    searchPtr->nextEntry = VarHashFirstEntry(varPtr->value.tablePtr,
	    &searchPtr->search);
    Tcl_SetHashValue(hPtr, searchPtr);
    searchPtr->name = Tcl_ObjPrintf("s-%d-%s", searchPtr->id,
	    TclGetString(arrayNameObj));
    Tcl_IncrRefCount(searchPtr->name);
}
/*
 *----------------------------------------------------------------------

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclVar.c`, function `Tcl_ConstObjCmd(`, lines 4893–4988. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-9.1.0-original-tclVar.c`.

```text
int
Tcl_ConstObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Var *varPtr, *arrayPtr;
    Tcl_Obj *part1Ptr;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "varName value");
	return TCL_ERROR;
    }

    part1Ptr = objv[1];
    varPtr = TclObjLookupVarEx(interp, part1Ptr, NULL, TCL_LEAVE_ERR_MSG,
	    "const", /*createPart1*/ 1, /*createPart2*/ 1, &arrayPtr);
    if (varPtr == NULL) {
	return TCL_ERROR;
    }
    if (TclIsVarArray(varPtr)) {
	TclObjVarErrMsg(interp, part1Ptr, NULL, "make constant", ISARRAY, -1);
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "CONST", (char *)NULL);
	return TCL_ERROR;
    }
    if (TclIsVarArrayElement(varPtr)) {
	if (TclIsVarUndefined(varPtr)) {
	    CleanupVar(varPtr, arrayPtr);
	}
	TclObjVarErrMsg(interp, part1Ptr, NULL, "make constant", ISARRAYELEMENT, -1);
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "CONST", (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * If already exists, either a constant (no problem) or an error.
     */
    if (!TclIsVarUndefined(varPtr)) {
	if (TclIsVarConstant(varPtr)) {
	    return TCL_OK;
	}
	TclObjVarErrMsg(interp, part1Ptr, NULL, "make constant", EXISTS, -1);
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "CONST", (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Make the variable and flag it as a constant.
     */
    if (TclPtrSetVar(interp, (Tcl_Var) varPtr, NULL, objv[1], NULL,
	    objv[2], TCL_LEAVE_ERR_MSG) == NULL) {
	if (TclIsVarUndefined(varPtr)) {
	    CleanupVar(varPtr, arrayPtr);
	}
	return TCL_ERROR;
    }
    TclSetVarConstant(varPtr);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GlobalObjCmd --
 *
 *	This object-based function is invoked to process the "global" Tcl
 *	command. See the user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl object result value.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_GlobalObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Interp *iPtr = (Interp *) interp;
    Tcl_Obj *objPtr, *tailPtr;
    const char *varName;
    const char *tail;
    int result;
    Tcl_Size i;

    /*
     * If we are not executing inside a Tcl procedure, just return.
     */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclVar.c`, function `TclPtrSetVarIdx(`, lines 1966–2266. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-9.1.0-original-tclVar.c`.

```text
Tcl_Obj *
TclPtrSetVarIdx(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be looked up. */
    Var *varPtr,		/* Reference to the variable to set. */
    Var *arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Name of an array (if part2 is non-NULL) or
				 * the name of a variable. NULL if the 'index'
				 * parameter is >= 0 */
    Tcl_Obj *part2Ptr,		/* If non-NULL, gives the name of an element
				 * in the array part1. */
    Tcl_Obj *newValuePtr,	/* New value for variable. */
    int flags,			/* OR-ed combination of TCL_GLOBAL_ONLY, and
				 * TCL_LEAVE_ERR_MSG bits. */
    Tcl_Size index)		/* Index of local var where part1 is to be
				 * found. */
{
    Interp *iPtr = (Interp *) interp;
    Tcl_Obj *oldValuePtr;
    Tcl_Obj *resultPtr = NULL;
    int result;
    int cleanupOnEarlyError = (newValuePtr->refCount == 0);

    /*
     * If the variable is in a hashtable and its hPtr field is NULL, then we
     * may have an upvar to an array element where the array was deleted or an
     * upvar to a namespace variable whose namespace was deleted. Generate an
     * error (allowing the variable to be reset would screw up our storage
     * allocation and is meaningless anyway).
     */

    if (TclIsVarDeadHash(varPtr)) {
	if (flags & TCL_LEAVE_ERR_MSG) {
	    if (TclIsVarArrayElement(varPtr)) {
		TclObjVarErrMsg(interp, part1Ptr, part2Ptr, "set",
			DANGLINGELEMENT, index);
		Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "ELEMENT", (char *)NULL);
	    } else {
		TclObjVarErrMsg(interp, part1Ptr, part2Ptr, "set",
			DANGLINGVAR, index);
		Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "VARNAME", (char *)NULL);
	    }
	}
	goto earlyError;
    }

    /*
     * It's an error to try to set a constant.
     */
    if (TclIsVarConstant(varPtr)) {
	if (flags & TCL_LEAVE_ERR_MSG) {
	    TclObjVarErrMsg(interp, part1Ptr, part2Ptr, "set", ISCONST,index);
	    Tcl_SetErrorCode(interp, "TCL", "WRITE", "CONST", (char *)NULL);
	}
	goto earlyError;
    }

    /*
     * It's an error to try to set an array variable itself.
     */

    if (TclIsVarArray(varPtr)) {
	if (flags & TCL_LEAVE_ERR_MSG) {
	    TclObjVarErrMsg(interp, part1Ptr, part2Ptr, "set", ISARRAY,index);
	    Tcl_SetErrorCode(interp, "TCL", "WRITE", "ARRAY", (char *)NULL);
	}
	goto earlyError;
    }

    TclVarFindHiddenArray(varPtr, arrayPtr);

    /*
     * Invoke any read traces that have been set for the variable if it is
     * requested. This was done for INST_LAPPEND_* but that was inconsistent
     * with the non-bc instruction, and would cause failures trying to
     * lappend to any non-existing ::env var, which is inconsistent with
     * documented behavior. [Bug #3057639].
     */

    if ((flags & TCL_TRACE_READS) && ((varPtr->flags & VAR_TRACED_READ)
	    || (arrayPtr && (arrayPtr->flags & VAR_TRACED_READ)))) {
	if (TCL_ERROR == TclObjCallVarTraces(iPtr, arrayPtr, varPtr,
		part1Ptr, part2Ptr,
		TCL_TRACE_READS, (flags & TCL_LEAVE_ERR_MSG), index)) {
	    goto earlyError;
	}
    }

    /*
     * Set the variable's new value. If appending, append the new value to the
     * variable, either as a list element or as a string. Also, if appending,
     * then if the variable's old value is unshared we can modify it directly,
     * otherwise we must create a new copy to modify: this is "copy on write".
     */

    oldValuePtr = varPtr->value.objPtr;
    if (flags & TCL_LIST_ELEMENT && !(flags & TCL_APPEND_VALUE)) {
	varPtr->value.objPtr = NULL;
    }
    if (flags & (TCL_APPEND_VALUE|TCL_LIST_ELEMENT)) {
	if (flags & TCL_LIST_ELEMENT) {		/* Append list element. */
	    result = ListAppendInVar(interp, varPtr, arrayPtr, oldValuePtr,
		    newValuePtr);
	    if (result != TCL_OK) {
		goto earlyError;
	    }
	} else {				/* Append string. */
	    StringAppendInVar(varPtr, arrayPtr, oldValuePtr, newValuePtr);
	}
    } else if (newValuePtr != oldValuePtr) {
	/*
	 * In this case we are replacing the value, so we don't need to do
	 * more than swap the objects.
	 */

	varPtr->value.objPtr = newValuePtr;
	Tcl_IncrRefCount(newValuePtr);		/* Var is another ref. */
	if (oldValuePtr != NULL) {
	    TclDecrRefCount(oldValuePtr);	/* Discard old value. */
	}
    }

    /*
     * Invoke any write traces for the variable.
     */

    if ((varPtr->flags & VAR_TRACED_WRITE)
	    || (arrayPtr && (arrayPtr->flags & VAR_TRACED_WRITE))) {
	if (TCL_ERROR == TclObjCallVarTraces(iPtr, arrayPtr, varPtr, part1Ptr,
		part2Ptr, (flags & (TCL_GLOBAL_ONLY|TCL_NAMESPACE_ONLY))
		| TCL_TRACE_WRITES, (flags & TCL_LEAVE_ERR_MSG), index)) {
	    goto cleanup;
	}
    }

    /*
     * Return the variable's value unless the variable was changed in some
     * gross way by a trace (e.g. it was unset and then recreated as an
     * array).
     */

    if (TclIsVarScalar(varPtr) && !TclIsVarUndefined(varPtr)) {
	return varPtr->value.objPtr;
    }

    /*
     * A trace changed the value in some gross way. Return an empty string
     * object.
     */

    resultPtr = iPtr->emptyObjPtr;

    /*
     * If the variable doesn't exist anymore and no-one's using it, then free
     * up the relevant structures and hash table entries.
     */

  cleanup:
    if (resultPtr == NULL) {
	Tcl_SetErrorCode(interp, "TCL", "WRITE", "VARNAME", (char *)NULL);
    }
    if (TclIsVarUndefined(varPtr)) {
	TclCleanupVar(varPtr, arrayPtr);
    }
    return resultPtr;

  earlyError:
    if (cleanupOnEarlyError) {
	Tcl_DecrRefCount(newValuePtr);
    }
    goto cleanup;
}

/*
 *----------------------------------------------------------------------
 *
 * TclIncrObjVar2 --
 *
 *	Given a two-part variable name, which may refer either to a scalar
 *	variable or an element of an array, increment the Tcl object value of
 *	the variable by a specified Tcl_Obj increment value.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the specified variable doesn't exist, or there is a clash
 *	in array usage, or an error occurs while executing variable traces,
 *	then NULL is returned and a message will be left in the interpreter's
 *	result.
 *
 * Side effects:
 *	The value of the given variable is incremented by the specified
 *	amount. If either the array or the entry didn't exist then a new
 *	variable is created. The ref count for the returned object is _not_
 *	incremented to reflect the returned reference; if you want to keep a
 *	reference to the object you must increment its ref count yourself.
 *	Callers must Incr part1Ptr if they plan to Decr it.
 *	Callers must Incr part2Ptr if they plan to Decr it.
 *
 *----------------------------------------------------------------------
 */

Tcl_Obj *
TclIncrObjVar2(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Tcl_Obj *part1Ptr,		/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,		/* If non-null, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *incrPtr,		/* Amount to be added to variable. */
    int flags)			/* Various flags that tell how to incr value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
{
    Var *varPtr, *arrayPtr;

    varPtr = TclObjLookupVarEx(interp, part1Ptr, part2Ptr, flags, "read",
	    1, 1, &arrayPtr);
    if (varPtr == NULL) {
	Tcl_AddErrorInfo(interp,
		"\n    (reading value of variable to increment)");
	return NULL;
    }
    return TclPtrIncrObjVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    incrPtr, flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * TclPtrIncrObjVar --
 *
 *	Given the pointers to a variable and possible containing array,
 *	increment the Tcl object value of the variable by a Tcl_Obj increment.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the specified variable doesn't exist, or there is a clash
 *	in array usage, or an error occurs while executing variable traces,
 *	then NULL is returned and a message will be left in the interpreter's
 *	result.
 *
 * Side effects:
 *	The value of the given variable is incremented by the specified
 *	amount. If either the array or the entry didn't exist then a new
 *	variable is created. The ref count for the returned object is _not_
 *	incremented to reflect the returned reference; if you want to keep a
 *	reference to the object you must increment its ref count yourself.
 *
 *----------------------------------------------------------------------
 */

Tcl_Obj *
TclPtrIncrObjVar(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Tcl_Var varPtr,		/* Reference to the variable to set. */
    Tcl_Var arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,		/* If non-null, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *incrPtr,		/* Increment value. */
/* TODO: Which of these flag values really make sense? */
    int flags)			/* Various flags that tell how to incr value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
{
    if (varPtr == NULL) {
	Tcl_Panic("varPtr must not be NULL");
    }
    if (part1Ptr == NULL) {
	Tcl_Panic("part1Ptr must not be NULL");
    }
    return TclPtrIncrObjVarIdx(interp, (Var *) varPtr, (Var *) arrayPtr,
	    part1Ptr, part2Ptr, incrPtr, flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * TclPtrIncrObjVarIdx --
 *
 *	Given the pointers to a variable and possible containing array,
 *	increment the Tcl object value of the variable by a Tcl_Obj increment.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the specified variable doesn't exist, or there is a clash
 *	in array usage, or an error occurs while executing variable traces,
 *	then NULL is returned and a message will be left in the interpreter's

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclVar.c`, function `CleanupVar(`, lines 427–557. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-9.1.0-original-tclVar.c`.

```text
static inline void
CleanupVar(
    Var *varPtr,		/* Pointer to variable that may be a candidate
				 * for being expunged. */
    Var *arrayPtr)		/* Array that contains the variable, or NULL
				 * if this variable isn't an array element. */
{
    if (TclIsVarUndefined(varPtr) && TclIsVarInHash(varPtr)
	    && !TclIsVarTraced(varPtr)
	    && (VarHashRefCount(varPtr) == (Tcl_Size)
		    !TclIsVarDeadHash(varPtr))) {
	if (VarHashRefCount(varPtr) == 0) {
	    Tcl_Free(varPtr);
	} else {
	    VarHashDeleteEntry(varPtr);
	}
    }
    if (arrayPtr != NULL && TclIsVarUndefined(arrayPtr) &&
	    TclIsVarInHash(arrayPtr) && !TclIsVarTraced(arrayPtr) &&
	    (VarHashRefCount(arrayPtr) == (Tcl_Size)
		    !TclIsVarDeadHash(arrayPtr))) {
	if (VarHashRefCount(arrayPtr) == 0) {
	    Tcl_Free(arrayPtr);
	} else {
	    VarHashDeleteEntry(arrayPtr);
	}
    }
}

void
TclCleanupVar(
    Var *varPtr,		/* Pointer to variable that may be a candidate
				 * for being expunged. */
    Var *arrayPtr)		/* Array that contains the variable, or NULL
				 * if this variable isn't an array element. */
{
    CleanupVar(varPtr, arrayPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * TclLookupVar --
 *
 *	This function is used to locate a variable given its name(s). It has
 *	been mostly superseded by TclObjLookupVar, it is now only used by the
 *	trace code. It is kept in tcl9.0 mainly because it is in the internal
 *	stubs table, so that some extension may be calling it.
 *
 * Results:
 *	The return value is a pointer to the variable structure indicated by
 *	part1 and part2, or NULL if the variable couldn't be found. If the
 *	variable is found, *arrayPtrPtr is filled in with the address of the
 *	variable structure for the array that contains the variable (or NULL
 *	if the variable is a scalar). If the variable can't be found and
 *	either createPart1 or createPart2 are 1, a new as-yet-undefined
 *	(VAR_UNDEFINED) variable structure is created, entered into a hash
 *	table, and returned.
 *
 *	If the variable isn't found and creation wasn't specified, or some
 *	other error occurs, NULL is returned and an error message is left in
 *	the interp's result if TCL_LEAVE_ERR_MSG is set in flags.
 *
 *	Note: it's possible for the variable returned to be VAR_UNDEFINED even
 *	if createPart1 or createPart2 are 1 (these only cause the hash table
 *	entry or array to be created). For example, the variable might be a
 *	global that has been unset but is still referenced by a procedure, or
 *	a variable that has been unset but it only being kept in existence (if
 *	VAR_UNDEFINED) by a trace.
 *
 * Side effects:
 *	New hashtable entries may be created if createPart1 or createPart2
 *	are 1.
 *
 *----------------------------------------------------------------------
 */

Var *
TclLookupVar(
    Tcl_Interp *interp,		/* Interpreter to use for lookup. */
    const char *part1,		/* If part2 isn't NULL, this is the name of an
				 * array. Otherwise, this is a full variable
				 * name that could include a parenthesized
				 * array element. */
    const char *part2,		/* Name of element within array, or NULL. */
    int flags,			/* Only TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * and TCL_LEAVE_ERR_MSG bits matter. */
    const char *msg,		/* Verb to use in error messages, e.g. "read"
				 * or "set". Only needed if TCL_LEAVE_ERR_MSG
				 * is set in flags. */
    int createPart1,		/* If 1, create hash table entry for part 1 of
				 * name, if it doesn't already exist. If 0,
				 * return error if it doesn't exist. */
    int createPart2,		/* If 1, create hash table entry for part 2 of
				 * name, if it doesn't already exist. If 0,
				 * return error if it doesn't exist. */
    Var **arrayPtrPtr)		/* If the name refers to an element of an
				 * array, *arrayPtrPtr gets filled in with
				 * address of array variable. Otherwise this
				 * is set to NULL. */
{
    Var *varPtr;
    Tcl_Obj *part1Ptr = Tcl_NewStringObj(part1, -1);

    if (createPart1) {
	Tcl_IncrRefCount(part1Ptr);
    }

    varPtr = TclObjLookupVar(interp, part1Ptr, part2, flags, msg,
	    createPart1, createPart2, arrayPtrPtr);

    TclDecrRefCount(part1Ptr);
    return varPtr;
}

/*
 *----------------------------------------------------------------------
 *
 * TclObjLookupVar, TclObjLookupVarEx --
 *
 *	This function is used by virtually all of the variable code to locate
 *	a variable given its name(s). The parsing into array/element
 *	components and (if possible) the lookup results are cached in
 *	part1Ptr, which is converted to one of the varNameTypes.
 *
 * Results:
 *	The return value is a pointer to the variable structure indicated by
 *	part1Ptr and part2, or NULL if the variable couldn't be found. If *
 *	the variable is found, *arrayPtrPtr is filled with the address of the
 *	variable structure for the array that contains the variable (or NULL
 *	if the variable is a scalar). If the variable can't be found and

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclVar.c`, function `Tcl_UnsetVar2(`, lines 2377–2497. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-9.1.0-original-tclVar.c`.

```text
int
Tcl_UnsetVar2(
    Tcl_Interp *interp,		/* Command interpreter in which varName is to
				 * be looked up. */
    const char *part1,		/* Name of variable or array. */
    const char *part2,		/* Name of element within array or NULL. */
    int flags)			/* OR-ed combination of any of
				 * TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_LEAVE_ERR_MSG. */
{
    int result;
    Tcl_Obj *part2Ptr = NULL, *part1Ptr = Tcl_NewStringObj(part1, -1);

    if (part2) {
	part2Ptr = Tcl_NewStringObj(part2, -1);
    }

    /*
     * Filter to pass through only the flags this interface supports.
     */

    flags &= (TCL_GLOBAL_ONLY|TCL_NAMESPACE_ONLY|TCL_LEAVE_ERR_MSG);
    result = TclObjUnsetVar2(interp, part1Ptr, part2Ptr, flags);

    Tcl_DecrRefCount(part1Ptr);
    if (part2Ptr) {
	Tcl_DecrRefCount(part2Ptr);
    }
    return result;
}

/*
 *----------------------------------------------------------------------
 *
 * TclObjUnsetVar2 --
 *
 *	Delete a variable, given a 2-object name.
 *
 * Results:
 *	Returns TCL_OK if the variable was successfully deleted, TCL_ERROR if
 *	the variable can't be unset. In the event of an error, if the
 *	TCL_LEAVE_ERR_MSG flag is set then an error message is left in the
 *	interp's result.
 *
 * Side effects:
 *	If part1ptr and part2Ptr indicate a local or global variable in
 *	interp, it is deleted. If part1Ptr is an array name and part2Ptr is
 *	NULL, then the whole array is deleted.
 *
 *----------------------------------------------------------------------
 */

int
TclObjUnsetVar2(
    Tcl_Interp *interp,		/* Command interpreter in which varName is to
				 * be looked up. */
    Tcl_Obj *part1Ptr,		/* Name of variable or array. */
    Tcl_Obj *part2Ptr,		/* Name of element within array or NULL. */
    int flags)			/* OR-ed combination of any of
				 * TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_LEAVE_ERR_MSG. */
{
    Var *varPtr, *arrayPtr;

    varPtr = TclObjLookupVarEx(interp, part1Ptr, part2Ptr, flags, "unset",
	    /*createPart1*/ 0, /*createPart2*/ 0, &arrayPtr);
    if (varPtr == NULL) {
	return TCL_ERROR;
    }

    return TclPtrUnsetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * TclPtrUnsetVar --
 *
 *	Delete a variable, given the pointers to the variable's (and possibly
 *	containing array's) VAR structure.
 *
 * Results:
 *	Returns TCL_OK if the variable was successfully deleted, TCL_ERROR if
 *	the variable can't be unset. In the event of an error, if the
 *	TCL_LEAVE_ERR_MSG flag is set then an error message is left in the
 *	interp's result.
 *
 * Side effects:
 *	If varPtr and arrayPtr indicate a local or global variable in interp,
 *	it is deleted. If varPtr is an array reference and part2Ptr is NULL,
 *	then the whole array is deleted.
 *
 *----------------------------------------------------------------------
 */

int
TclPtrUnsetVar(
    Tcl_Interp *interp,		/* Command interpreter in which varName is to
				 * be looked up. */
    Tcl_Var varPtr,		/* The variable to be unset. */
    Tcl_Var arrayPtr,		/* NULL for scalar variables, pointer to the
				 * containing array otherwise. */
    Tcl_Obj *part1Ptr,		/* Name of an array (if part2 is non-NULL) or
				 * the name of a variable. */
    Tcl_Obj *part2Ptr,		/* If non-NULL, gives the name of an element
				 * in the array part1. */
    int flags)			/* OR-ed combination of any of
				 * TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_LEAVE_ERR_MSG. */
{
    if (varPtr == NULL) {
	Tcl_Panic("varPtr must not be NULL");
    }
    if (part1Ptr == NULL) {
	Tcl_Panic("part1Ptr must not be NULL");
    }
    return TclPtrUnsetVarIdx(interp, (Var *) varPtr, (Var *) arrayPtr,
	    part1Ptr, part2Ptr, flags, -1);
}


```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInt.h`, function `#define TclSetVarConstant`, lines 803–813. Retained reference `rust-tcl-registry-tests-data-native_ensemble_holder_routing205-source-inspection208-9.1.0-tclInt.h`.

```text

#define TclSetVarConstant(varPtr) \
    (varPtr)->flags = ((varPtr)->flags & ~(VAR_ARRAY|VAR_LINK)) | VAR_CONSTANT

#define TclSetVarArrayElement(varPtr) \
    (varPtr)->flags = ((varPtr)->flags & ~VAR_ARRAY) | VAR_ARRAY_ELEMENT

#define TclSetVarUndefined(varPtr) \
    (varPtr)->flags &= ~(VAR_ARRAY|VAR_LINK|VAR_CONSTANT);	\
    (varPtr)->value.objPtr = NULL

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclVar.c`, function `Tcl_ObjSetVar2(`, lines 1726–1831. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-9.1.0-original-tclVar.c`.

```text
Tcl_Obj *
Tcl_ObjSetVar2(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be found. */
    Tcl_Obj *part1Ptr,		/* Points to an object holding the name of an
				 * array (if part2 is non-NULL) or the name of
				 * a variable. */
    Tcl_Obj *part2Ptr,		/* If non-NULL, points to an object holding
				 * the name of an element in the array
				 * part1Ptr. */
    Tcl_Obj *newValuePtr,	/* New value for variable. */
    int flags)			/* Various flags that tell how to set value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT, or
				 * TCL_LEAVE_ERR_MSG. */
{
    Var *varPtr, *arrayPtr;

    /*
     * Filter to pass through only the flags this interface supports.
     */

    flags &= (TCL_GLOBAL_ONLY|TCL_NAMESPACE_ONLY|TCL_LEAVE_ERR_MSG
	    |TCL_APPEND_VALUE|TCL_LIST_ELEMENT);
    varPtr = TclObjLookupVarEx(interp, part1Ptr, part2Ptr, flags, "set",
	    /*createPart1*/ 1, /*createPart2*/ 1, &arrayPtr);
    if (varPtr == NULL) {
	if (newValuePtr->refCount == 0) {
	    Tcl_DecrRefCount(newValuePtr);
	}
	return NULL;
    }

    return TclPtrSetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    newValuePtr, flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * TclPtrSetVar --
 *
 *	This function is the same as Tcl_SetVar2Ex above, except that it
 *	requires pointers to the variable's Var structs in addition to the
 *	variable names.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the write operation was disallowed because an array was
 *	expected but not found (or vice versa), then NULL is returned; if the
 *	TCL_LEAVE_ERR_MSG flag is set, then an explanatory message will be
 *	left in the interpreter's result. Note that the returned object may
 *	not be the same one referenced by newValuePtr; this is because
 *	variable traces may modify the variable's value.
 *
 * Side effects:
 *	The value of the given variable is set. If either the array or the
 *	entry didn't exist then a new variable is created.
 *
 *----------------------------------------------------------------------
 */

Tcl_Obj *
TclPtrSetVar(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be looked up. */
    Tcl_Var varPtr,		/* Reference to the variable to set. */
    Tcl_Var arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Name of an array (if part2 is non-NULL) or
				 * the name of a variable. */
    Tcl_Obj *part2Ptr,		/* If non-NULL, gives the name of an element
				 * in the array part1. */
    Tcl_Obj *newValuePtr,	/* New value for variable. */
    int flags)			/* OR-ed combination of TCL_GLOBAL_ONLY, and
				 * TCL_LEAVE_ERR_MSG bits. */
{
    if (varPtr == NULL) {
	Tcl_Panic("varPtr must not be NULL");
    }
    if (part1Ptr == NULL) {
	Tcl_Panic("part1Ptr must not be NULL");
    }
    if (newValuePtr == NULL) {
	Tcl_Panic("newValuePtr must not be NULL");
    }
    return TclPtrSetVarIdx(interp, (Var *) varPtr, (Var *) arrayPtr,
	    part1Ptr, part2Ptr, newValuePtr, flags, -1);
}

/*
 *----------------------------------------------------------------------
 *
 * ListAppendInVar, StringAppendInVar --
 *
 *	Support functions for TclPtrSetVarIdx that implement various types of
 *	appending operations.
 *
 * Results:
 *	ListAppendInVar returns a Tcl result code (from the core list append
 *	operation). StringAppendInVar has no return value.
 *
 * Side effects:
 *	The variable or element of the array is updated. This may make the
 *	variable/element exist. Reference counts of values may be updated.

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `Tcl_EvalObjv(`, lines 4441–4551. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.1.0-tclBasic.c`.

```text
int
Tcl_EvalObjv(
    Tcl_Interp *interp,		/* Interpreter in which to evaluate the
				 * command. Also used for error reporting. */
    Tcl_Size objc,		/* Number of words in command. */
    Tcl_Obj *const *objv,	/* An array of pointers to objects that are
				 * the words that make up the command. */
    int flags)			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Only
				 * TCL_EVAL_GLOBAL, TCL_EVAL_INVOKE and
				 * TCL_EVAL_NOERR are currently supported. */
{
    int result;
    NRE_callback *rootPtr = TOP_CB(interp);

    result = TclNREvalObjv(interp, objc, objv, flags, NULL);
    return TclNRRunCallbacks(interp, result, rootPtr);
}

int
TclNREvalObjv(
    Tcl_Interp *interp,		/* Interpreter in which to evaluate the
				 * command. Also used for error reporting. */
    Tcl_Size objc,		/* Number of words in command. */
    Tcl_Obj *const *objv,	/* An array of pointers to objects that are
				 * the words that make up the command. */
    int flags,			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Only
				 * TCL_EVAL_GLOBAL, TCL_EVAL_INVOKE and
				 * TCL_EVAL_NOERR are currently supported. */
    Command *cmdPtr)		/* NULL if the Command is to be looked up
				 * here, otherwise the pointer to the
				 * requested Command struct to be invoked. */
{
    Interp *iPtr = (Interp *) interp;

    /*
     * data[1] stores a marker for use by tailcalls; it will be set to 1 by
     * command redirectors (imports, alias, ensembles) so that tailcall skips
     * this callback (that marks the end of the target command) and goes back
     * to the end of the source command.
     */

    if (iPtr->deferredCallbacks) {
	iPtr->deferredCallbacks = NULL;
    } else {
	TclNRAddCallback(interp, NRCommand, NULL, NULL, NULL, NULL);
    }

    iPtr->numLevels++;
    TclNRAddCallback(interp, EvalObjvCore, cmdPtr, INT2PTR(flags),
	    INT2PTR(objc), objv);
    return TCL_OK;
}

static int
EvalObjvCore(
    void *data[],
    Tcl_Interp *interp,
    TCL_UNUSED(int) /*result*/)
{
    Command *cmdPtr = NULL, *preCmdPtr = (Command *)data[0];
    int flags = (int)PTR2INT(data[1]);
    Tcl_Size objc = PTR2INT(data[2]);
    Tcl_Obj **objv = (Tcl_Obj **)data[3];
    Interp *iPtr = (Interp *) interp;
    Namespace *lookupNsPtr = NULL;
    int enterTracesDone = 0;

    /*
     * Push records for task to be done on return, in INVERSE order. First, if
     * needed, the exception handlers (as they should happen last).
     */

    if (!(flags & TCL_EVAL_NOERR)) {
	TEOV_PushExceptionHandlers(interp, objc, objv, flags);
    }

    if (TCL_OK != TclInterpReady(interp)) {
	return TCL_ERROR;
    }

    if (objc == 0) {
	return TCL_OK;
    }

    if (TclLimitExceeded(iPtr->limit)) {
	/* generate error message if not yet already logged at this stage */
	if (!(iPtr->flags & ERR_ALREADY_LOGGED)) {
	    Tcl_LimitCheck(interp);
	}
	return TCL_ERROR;
    }

    /*
     * Configure evaluation context to match the requested flags.
     */

    if (iPtr->lookupNsPtr) {
	/*
	 * Capture the namespace we should do command name resolution in, as
	 * instructed by our caller sneaking it in to us in a private interp
	 * field.  Clear that field right away so we cannot possibly have its
	 * use leak where it should not.  The sneaky message pass is done.
	 *
	 * Use of this mechanism overrides the TCL_EVAL_GLOBAL flag.
	 * TODO: Is that a bug?
	 */

	lookupNsPtr = iPtr->lookupNsPtr;
	iPtr->lookupNsPtr = NULL;

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole source retained`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `static int Jim_InfoCoreCommand(`, lines 15985–16205. Retained reference `rust-tcl-registry-tests-data-native_jim_alias_error_extent279-request-jim.c`.

```text
/* [info] */
static int Jim_InfoCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_Obj *objPtr;
    int mode = 1;

    /* Must be kept in order with the array below */
    enum {
        INFO_ALIAS,
        INFO_ALIASES,
        INFO_ARGS,
        INFO_BODY,
        INFO_CHANNELS,
        INFO_COMMANDS,
        INFO_COMPLETE,
        INFO_EXISTS,
        INFO_FRAME,
        INFO_GLOBALS,
        INFO_HELP,
        INFO_HOSTNAME,
        INFO_LEVEL,
        INFO_LOCALS,
        INFO_NAMEOFEXECUTABLE,
        INFO_PATCHLEVEL,
        INFO_PROCS,
        INFO_REFERENCES,
        INFO_RETURNCODES,
        INFO_SCRIPT,
        INFO_SOURCE,
        INFO_STACKTRACE,
        INFO_STATICS,
        INFO_TAINTED,
        INFO_USAGE,
        INFO_VARS,
        INFO_VERSION,
        INFO_COUNT
    };
    static const jim_subcmd_type cmds[INFO_COUNT + 1] = {
        JIM_DEF_SUBCMD("alias", "command", 1, 1),
        JIM_DEF_SUBCMD("aliases", "?-all? ?pattern?", 0, 2),
        JIM_DEF_SUBCMD("args", "procname", 1, 1),
        JIM_DEF_SUBCMD("body", "procname", 1, 1),
        JIM_DEF_SUBCMD("channels", "?-all? ?pattern?", 0, 2),
        JIM_DEF_SUBCMD("commands", "?-all? ?pattern?", 0, 2),
        JIM_DEF_SUBCMD("complete", "script ?missing?", 1, 2),
        JIM_DEF_SUBCMD("exists", "varName", 1, 1),
        JIM_DEF_SUBCMD("frame", "?levelNum?", 0, 1),
        JIM_DEF_SUBCMD("globals", "?pattern?", 0, 1),
        JIM_DEF_SUBCMD("help", "command", 1, 1),
        JIM_DEF_SUBCMD("hostname", NULL, 0, 0),
        JIM_DEF_SUBCMD("level", "?levelNum?", 0, 1),
        JIM_DEF_SUBCMD("locals", "?pattern?", 0, 1),
        JIM_DEF_SUBCMD("nameofexecutable", NULL, 0, 0),
        JIM_DEF_SUBCMD("patchlevel", NULL, 0, 0),
        JIM_DEF_SUBCMD("procs", "?-all? ?pattern?", 0, 2),
        JIM_DEF_SUBCMD("references", NULL, 0, 0),
        JIM_DEF_SUBCMD("returncodes", "?code?", 0, 1),
        JIM_DEF_SUBCMD("script", "?filename?", 0, 1),
        JIM_DEF_SUBCMD("source", "source ?filename line?", 1, 3),
        JIM_DEF_SUBCMD("stacktrace", NULL, 0, 0),
        JIM_DEF_SUBCMD("statics", "procname", 1, 1),
        JIM_DEF_SUBCMD("tainted", "value", 1, 1),
        JIM_DEF_SUBCMD("usage", "command", 1, 1),
        JIM_DEF_SUBCMD("vars", "?pattern?", 0, 1),
        JIM_DEF_SUBCMD("version", NULL, 0, 0),
        { NULL }
    };
    const jim_subcmd_type *ct;
#ifdef jim_ext_namespace
    int nons = 0;

    if (argc > 2 && Jim_CompareStringImmediate(interp, argv[1], "-nons")) {
        /* This is for internal use only */
        argc--;
        argv++;
        nons = 1;
    }
#endif
    ct = Jim_ParseSubCmd(interp, cmds, argc, argv);
    if (!ct) {
        return JIM_ERR;
    }
    if (ct->function) {
        /* This is -help or -commands */
        return ct->function(interp, argc, argv);
    }
    /* (ct - cmds) is the index into the table */
    int option = ct - cmds;

    switch (option) {
        case INFO_EXISTS:
            Jim_SetResultBool(interp, Jim_GetVariable(interp, argv[2], 0) != NULL);
            return JIM_OK;

        case INFO_ALIAS:{
            Jim_Cmd *cmdPtr;

            if ((cmdPtr = Jim_GetCommand(interp, argv[2], JIM_ERRMSG)) == NULL) {
                return JIM_ERR;
            }
            if ((cmdPtr->flags & JIM_CMD_ISALIAS) == 0) {
                Jim_SetResultFormatted(interp, "command \"%#s\" is not an alias", argv[2]);
                return JIM_ERR;
            }
            Jim_SetResult(interp, (Jim_Obj *)cmdPtr->u.native.privData);
            return JIM_OK;
        }

        case INFO_TAINTED:
            if (argc != 3) {
                Jim_WrongNumArgs(interp, 2, argv, "value");
                return JIM_ERR;
            }
            Jim_SetResultBool(interp, argv[2]->taint != 0);
            return JIM_OK;
        case INFO_CHANNELS:
            mode <<= 1;             /* JIM_CMDLIST_CHANNELS */
#ifndef jim_ext_aio
            Jim_SetResultString(interp, "aio not enabled", -1);
            return JIM_ERR;
#endif
            /* fall through */
        case INFO_ALIASES:
            mode <<= 1;             /* JIM_CMDLIST_ALIASES */
            /* fall through */
        case INFO_PROCS:
            mode <<= 1;             /* JIM_CMDLIST_PROCS */
            /* fall through */
        case INFO_COMMANDS:{
            int n = 0;
            /* mode = 1 => JIM_CMDLIST_COMMANDS */
            if (argc > 2 && Jim_CompareStringImmediate(interp, argv[2], "-all")) {
                mode |= JIM_CMDLIST_ALL;
                n++;
            }
            if (argc < 2 + n || argc > 3 + n) {
                Jim_SetResultFormatted(interp, "wrong # args: should be \"info %#s %s\"", argv[1], cmds[option].args);
                return JIM_ERR;
            }
#ifdef jim_ext_namespace
            if (!nons) {
                /* Called as 'info -nons commands|procs' so respect the current namespace */
                if (Jim_Length(interp->framePtr->nsObj) || (argc == 3 && JimIsGlobalNamespace(argv[2]))) {
                    return Jim_EvalPrefix(interp, "namespace info", argc - 1, argv + 1);
                }
            }
#endif
            Jim_SetResult(interp, JimCommandsList(interp, (argc == 3 + n) ? argv[2 + n] : NULL, mode));
            return JIM_OK;
        }

        case INFO_VARS:
            mode <<= 1;             /* JIM_VARLIST_VARS */
            /* fall through */
        case INFO_LOCALS:
            mode <<= 1;             /* JIM_VARLIST_LOCALS */
            /* fall through */
        case INFO_GLOBALS:
            /* mode = 1 => JIM_VARLIST_GLOBALS */
#ifdef jim_ext_namespace
            if (!nons) {
                if (Jim_Length(interp->framePtr->nsObj) || (argc == 3 && JimIsGlobalNamespace(argv[2]))) {
                    return Jim_EvalPrefix(interp, "namespace info", argc - 1, argv + 1);
                }
            }
#endif
            Jim_SetResult(interp, JimVariablesList(interp, argc == 3 ? argv[2] : NULL, mode));
            return JIM_OK;

        case INFO_SCRIPT:
            if (argc == 3) {
                Jim_IncrRefCount(argv[2]);
                Jim_DecrRefCount(interp, interp->currentFilenameObj);
                interp->currentFilenameObj = argv[2];
            }
            Jim_SetResult(interp, interp->currentFilenameObj);
            return JIM_OK;

        case INFO_SOURCE:{
                Jim_Obj *resObjPtr;
                Jim_Obj *fileNameObj;

                if (argc == 4) {
                    Jim_SubCmdArgError(interp, ct, argv[0]);
                    return JIM_ERR;
                }
                if (argc == 5) {
                    jim_wide line;
                    if (Jim_GetWide(interp, argv[4], &line) != JIM_OK) {
                        return JIM_ERR;
                    }
                    resObjPtr = Jim_NewStringObj(interp, Jim_String(argv[2]), Jim_Length(argv[2]));
                    Jim_SetSourceInfo(interp, resObjPtr, argv[3], line);
                }
                else {
                    int line;
                    fileNameObj = Jim_GetSourceInfo(interp, argv[2], &line);
                    resObjPtr = Jim_NewListObj(interp, NULL, 0);
                    Jim_ListAppendElement(interp, resObjPtr, fileNameObj);
                    Jim_ListAppendElement(interp, resObjPtr, Jim_NewIntObj(interp, line));
                }
                Jim_SetResult(interp, resObjPtr);
                return JIM_OK;
            }

        case INFO_STACKTRACE:
            Jim_SetResult(interp, interp->stackTrace);
            return JIM_OK;

        case INFO_LEVEL:
            if (argc == 2) {
                Jim_SetResultInt(interp, interp->framePtr->level);
            }
            else {
                if (JimInfoLevel(interp, argv[2], &objPtr) != JIM_OK) {
                    return JIM_ERR;
                }
                Jim_SetResult(interp, objPtr);
            }
            return JIM_OK;

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole source retained`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `static int Jim_IncrCoreCommand(`, lines 10952–11007. Retained reference `rust-tcl-registry-tests-data-native_jim_alias_error_extent279-request-jim.c`.

```text
/* [incr] */
static int Jim_IncrCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    jim_wide wideValue, increment = 1;
    Jim_Obj *intObjPtr;

    if (argc == 3) {
        if (Jim_GetWideExpr(interp, argv[2], &increment) != JIM_OK)
            return JIM_ERR;
    }
    intObjPtr = Jim_GetVariable(interp, argv[1], JIM_UNSHARED);
    if (!intObjPtr) {
        /* Set missing variable to 0 */
        wideValue = 0;
    }
    else if (Jim_GetWide(interp, intObjPtr, &wideValue) != JIM_OK) {
        return JIM_ERR;
    }
    if (!intObjPtr || Jim_IsShared(intObjPtr)) {
        intObjPtr = Jim_NewIntObj(interp, wideValue + increment);
        if (Jim_SetVariable(interp, argv[1], intObjPtr) != JIM_OK) {
            return JIM_ERR;
        }
    }
    else {
        /* Can do it the quick way */
        Jim_InvalidateStringRep(intObjPtr);
        JimWideValue(intObjPtr) = wideValue + increment;

        /* The following step is required in order to invalidate the
         * string repr of "FOO" if the var name is on the form of "FOO(IDX)" */
        if (argv[1]->typePtr != &variableObjType) {
            /* Note that this can't fail since GetVariable already succeeded */
            Jim_SetVariable(interp, argv[1], intObjPtr);
        }
    }
    Jim_SetResult(interp, intObjPtr);
    return JIM_OK;
}

/* -----------------------------------------------------------------------------
 * Eval
 * ---------------------------------------------------------------------------*/
#define JIM_EVAL_SARGV_LEN 8    /* static arguments vector length */
#define JIM_EVAL_SINTV_LEN 8    /* static interpolation vector length */

static int JimTraceCallback(Jim_Interp *interp, const char *type, int argc, Jim_Obj *const *argv)
{
    JimPanic((interp->traceCmdObj == NULL, "xtrace invoked with no object"));

    int ret;
    Jim_Obj *nargv[7];
    Jim_Obj *traceCmdObj = interp->traceCmdObj;
    Jim_Obj *resultObj = Jim_GetResult(interp);
    ScriptObj *script = NULL;

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole source retained`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `static int Jim_UnsetCoreCommand(`, lines 12752–12794. Retained reference `rust-tcl-registry-tests-data-native_jim_alias_error_extent279-request-jim.c`.

```text
 */
static int Jim_UnsetCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int i = 1;
    int complain = 1;

    while (i < argc) {
        if (Jim_CompareStringImmediate(interp, argv[i], "--")) {
            i++;
            break;
        }
        if (Jim_CompareStringImmediate(interp, argv[i], "-nocomplain")) {
            complain = 0;
            i++;
            continue;
        }
        break;
    }

    while (i < argc) {
        if (Jim_UnsetVariable(interp, argv[i], complain ? JIM_ERRMSG : JIM_NONE) != JIM_OK
            && complain) {
            return JIM_ERR;
        }
        i++;
    }

    Jim_SetEmptyResult(interp);
    return JIM_OK;
}

/**
 * All commands that support break, continue from a loop (while, loop, foreach, for)
 * use this to check for break_level.
 *
 * If break_level is > 0, decrements the break_level and returns 1.
 * Otherwise returns 0
 */
static int JimCheckLoopRetcode(Jim_Interp *interp, int retval)
{
    if (retval == JIM_BREAK || retval == JIM_CONTINUE) {
        if (--interp->break_level > 0) {
            return 1;

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole source retained`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `int Jim_EvalObjVector(`, lines 11216–11296. Retained reference `rust-tcl-registry-tests-data-native_jim_alias_error_extent279-request-jim.c`.

```text
 * command argument. */
int Jim_EvalObjVector(Jim_Interp *interp, int objc, Jim_Obj *const *objv)
{
    int i, retcode;
    Jim_EvalFrame frame;

    /* Incr refcount of arguments. */
    for (i = 0; i < objc; i++)
        Jim_IncrRefCount(objv[i]);

    /* Note that we have no source for this command so push NULL as the scriptObj */
    JimPushEvalFrame(interp, &frame, NULL);

    retcode = JimInvokeCommand(interp, objc, objv);

    JimPopEvalFrame(interp);

    /* Decr refcount of arguments and return the retcode */
    for (i = 0; i < objc; i++)
        Jim_DecrRefCount(interp, objv[i]);

    return retcode;
}

/**
 * Invokes 'prefix' as a command with the objv array as arguments.
 */
int Jim_EvalObjPrefix(Jim_Interp *interp, Jim_Obj *prefix, int objc, Jim_Obj *const *objv)
{
    int ret;
    Jim_Obj **nargv = Jim_Alloc((objc + 1) * sizeof(*nargv));

    nargv[0] = prefix;
    memcpy(&nargv[1], &objv[0], sizeof(nargv[0]) * objc);
    ret = Jim_EvalObjVector(interp, objc + 1, nargv);
    Jim_Free(nargv);
    return ret;
}

/* Perform one substitution step for a parsed script token. */
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
            objPtr = JimExpandDictSugar(interp, token->objPtr);
            break;
        case JIM_TT_EXPRSUGAR:
            ret = Jim_EvalExpression(interp, token->objPtr);
            if (ret == JIM_OK) {
                objPtr = Jim_GetResult(interp);
            }
            else {
                objPtr = NULL;
            }
            break;
        case JIM_TT_CMD:
            ret = Jim_EvalObj(interp, token->objPtr);
            if (ret == JIM_OK || ret == JIM_RETURN) {
                objPtr = interp->result;
            } else {
                /* includes JIM_BREAK, JIM_CONTINUE */
                objPtr = NULL;
            }
            break;
        default:
            JimPanic((1,
                "default token type (%d) reached " "in Jim_SubstObj().", token->type));
            objPtr = NULL;
            break;
    }
    if (objPtr) {

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole source retained`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_Obj *Jim_GetGlobalVariableStr(`, lines 4999–5029. Retained reference `rust-tcl-registry-tests-data-native_jim_alias_error_extent279-request-jim.c`.

```text

Jim_Obj *Jim_GetGlobalVariableStr(Jim_Interp *interp, const char *name, int flags)
{
    Jim_CallFrame *savedFramePtr;
    Jim_Obj *objPtr;

    savedFramePtr = interp->framePtr;
    interp->framePtr = interp->topFramePtr;
    objPtr = Jim_GetVariableStr(interp, name, flags);
    interp->framePtr = savedFramePtr;

    return objPtr;
}

/* Unset a variable.
 * Note: On success unset invalidates all the (cached) variable objects
 * by incrementing callFrameEpoch
 */
int Jim_UnsetVariable(Jim_Interp *interp, Jim_Obj *nameObjPtr, int flags)
{
    Jim_VarVal *vv;
    int retval;
    Jim_CallFrame *framePtr;

    retval = SetVariableFromAny(interp, nameObjPtr);
    if (retval == JIM_DICT_SUGAR) {
        /* [dict] syntax sugar. */
        return JimDictSugarSet(interp, nameObjPtr, NULL);
    }
    else if (retval == JIM_OK) {
        vv = nameObjPtr->internalRep.varValue.vv;

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole source retained`, `/workspace/.proofs/native-providers/jimtcl/jim-array.c`, function `static int array_cmd_set(`, lines 167–232. Retained reference `runtime-rust-tests-data-native_variable_introspection_getter_order_original-original-inputs-workspace--.proofs--native-providers--jimtcl--jim-array.c`.

```text

static int array_cmd_set(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int i;
    int len;
    Jim_Obj *listObj = argv[1];
    Jim_Obj *dictObj;

    len = Jim_ListLength(interp, listObj);
    if (len % 2) {
        Jim_SetResultString(interp, "list must have an even number of elements", -1);
        return JIM_ERR;
    }

    dictObj = Jim_GetVariable(interp, argv[0], JIM_UNSHARED);
    if (!dictObj) {
        /* Doesn't exist, so just set the list directly */
        return Jim_SetVariable(interp, argv[0], listObj);
    }
    else if (Jim_DictSize(interp, dictObj) < 0) {
        return JIM_ERR;
    }

    if (Jim_IsShared(dictObj)) {
        dictObj = Jim_DuplicateObj(interp, dictObj);
    }

    for (i = 0; i < len; i += 2) {
        Jim_Obj *nameObj;
        Jim_Obj *valueObj;

        Jim_ListIndex(interp, listObj, i, &nameObj, JIM_NONE);
        Jim_ListIndex(interp, listObj, i + 1, &valueObj, JIM_NONE);

        Jim_DictAddElement(interp, dictObj, nameObj, valueObj);
    }
    return Jim_SetVariable(interp, argv[0], dictObj);
}

static const jim_subcmd_type array_command_table[] = {
        {       "exists",
                "arrayName",
                array_cmd_exists,
                1,
                1,
                /* Description: Does array exist? */
        },
        {       "get",
                "arrayName ?pattern?",
                array_cmd_get,
                1,
                2,
                /* Description: Array contents as name value list */
        },
        {       "names",
                "arrayName ?pattern?",
                array_cmd_names,
                1,
                2,
                /* Description: Array keys as a list */
        },
        {       "set",
                "arrayName list",
                array_cmd_set,
                2,
                2,

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole source retained`, `/workspace/.proofs/native-providers/jimtcl/jim.h`, function `JIM_EXPORT int Jim_EvalObjVector`, lines 760–765. Retained reference `rust-tcl-registry-tests-data-native_jim_core_alias_prefix273-jim.h`.

```text
/** Evaluate a command vector as if it were a parsed command. */
JIM_EXPORT int Jim_EvalObjVector (Jim_Interp *interp, int objc,
        Jim_Obj *const *objv);
/** Evaluate a list object as a command. */
JIM_EXPORT int Jim_EvalObjList(Jim_Interp *interp, Jim_Obj *listObj);
/** Evaluate a command by prepending a command prefix object to an argument vector. */

```

## Consumer bindings

No implementation binding is claimed by this observation record.

## Finite reached boundaries

| Capture or purpose | Original observation |
| --- | --- |
| Buffered511 C9 | Execute -11; cases0–2 complete, case3 partial, cases4–7 absent; no fatal-site inference from buffered tail |
| Isolated511b | 61 closed receipts; 48 case processes; 46 exit0 with markers and C90/C91 case4 -11 without markers |
| C9 case4 worker | VECTOR_RETURN0, EVAL_RETURN0 and original worker ROW are retained |
| C9 case4 post-query | Public exists/array result1 0; post-scalar EVAL_RETURN0/header retained; scalar-result ROW and final markers absent |
| Same-spelling scalar query | Public value/constant flag only; no original captured Var identity or liveness |
| Inspected Tcl_ConstObjCmd | Captured Var → TclPtrSetVar → captured-pointer constant flag after non-null return; source sequence supplies no trace-safe lifetime |
| C84/C85/C86/Jim | Initialized const availability empty and actual invalid-command Guest observations; no C9 worker semantics |

The original request's declared registry fixture path remains unchanged.
The canonical immutable mapping is the Runtime fixture directory named in
the evidence paths, with separate511 and511b captures. Reused SDK payloads
are joined by whole hash; every original request, stream and process receipt
retains its own authority. Complete API returns do not invent missing public
results, and no state after abnormal termination is asserted.

## Evidence

- [runtime/rust/tests/data](../../../../runtime/rust/tests/data)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original)
- [rust/tcl-registry/tests/data/native_capi_scalar_publication_original](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original)
- [rust/tcl-registry/tests/data/native_command_recreation_original](../../../../rust/tcl-registry/tests/data/native_command_recreation_original)
- [runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/8.4.20](../../../../runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/8.4.20)
- [runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/8.5.19](../../../../runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/8.5.19)
- [runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/8.6.18](../../../../runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/8.6.18)
- [runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/9.0.4](../../../../runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/9.0.4)
- [runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/9.1.0](../../../../runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/9.1.0)
- [runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/jim](../../../../runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/jim)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.4.20](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.4.20)
- [rust/tcl-registry/tests/data/native_naming_argument_getter_effect_order_original](../../../../rust/tcl-registry/tests/data/native_naming_argument_getter_effect_order_original)
- [rust/tcl-registry/tests/data/native_deep_import_chain_original](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original)
- [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original)
- [rust/tcl-registry/tests/data/native_error_code_metadata_original/original-inputs](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/original-inputs)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.5.19](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.5.19)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim)
- [rust/tcl-registry/tests/data/native_child_alias_inventory298](../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298)
- [rust/tcl-registry/tests/data/native_child_bootstrap_context303/source](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303/source)
- [rust/tcl-registry/tests/data/native_jim_alias_introspection274](../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274)
- [rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/jim](../../../../rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/jim)
- [rust/tcl-test-support/tests/data/native_resolution_reference_builds](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds)
- [runtime/rust/tests/data/native_variable_introspection_getter_order_original/original-inputs](../../../../runtime/rust/tests/data/native_variable_introspection_getter_order_original/original-inputs)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.4.20/cases/0](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.4.20/cases/0)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.4.20/cases/1](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.4.20/cases/1)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.4.20/cases/2](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.4.20/cases/2)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.4.20/cases/3](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.4.20/cases/3)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.4.20/cases/4](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.4.20/cases/4)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.4.20/cases/5](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.4.20/cases/5)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.4.20/cases/6](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.4.20/cases/6)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.4.20/cases/7](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.4.20/cases/7)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.5.19/cases/0](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.5.19/cases/0)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.5.19/cases/1](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.5.19/cases/1)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.5.19/cases/2](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.5.19/cases/2)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.5.19/cases/3](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.5.19/cases/3)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.5.19/cases/4](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.5.19/cases/4)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.5.19/cases/5](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.5.19/cases/5)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.5.19/cases/6](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.5.19/cases/6)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.5.19/cases/7](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.5.19/cases/7)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.6.18/cases/0](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.6.18/cases/0)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.6.18/cases/1](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.6.18/cases/1)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.6.18/cases/2](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.6.18/cases/2)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.6.18/cases/3](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.6.18/cases/3)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.6.18/cases/4](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.6.18/cases/4)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.6.18/cases/5](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.6.18/cases/5)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.6.18/cases/6](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.6.18/cases/6)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.6.18/cases/7](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/8.6.18/cases/7)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.0.4/cases/0](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.0.4/cases/0)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.0.4/cases/1](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.0.4/cases/1)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.0.4/cases/2](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.0.4/cases/2)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.0.4/cases/3](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.0.4/cases/3)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.0.4/cases/4](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.0.4/cases/4)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.0.4/cases/5](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.0.4/cases/5)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.0.4/cases/6](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.0.4/cases/6)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.0.4/cases/7](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.0.4/cases/7)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.1.0/cases/0](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.1.0/cases/0)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.1.0/cases/1](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.1.0/cases/1)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.1.0/cases/2](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.1.0/cases/2)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.1.0/cases/3](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.1.0/cases/3)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.1.0/cases/4](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.1.0/cases/4)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.1.0/cases/5](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.1.0/cases/5)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.1.0/cases/6](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.1.0/cases/6)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.1.0/cases/7](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/9.1.0/cases/7)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/jim/cases/0](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/jim/cases/0)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/jim/cases/1](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/jim/cases/1)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/jim/cases/2](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/jim/cases/2)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/jim/cases/3](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/jim/cases/3)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/jim/cases/4](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/jim/cases/4)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/jim/cases/5](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/jim/cases/5)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/jim/cases/6](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/jim/cases/6)
- [runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/jim/cases/7](../../../../runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original/511b/providers/jim/cases/7)
