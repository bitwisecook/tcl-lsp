    case INST_LOR:
    case INST_LAND: {
	/*
	 * Operands must be boolean or numeric. No int->double conversions are
	 * performed.
	 */

	int i1, i2, iResult;
	Tcl_Obj *value2Ptr = OBJ_AT_TOS;
	Tcl_Obj *valuePtr = OBJ_UNDER_TOS;

	result = TclGetBooleanFromObj(NULL, valuePtr, &i1);
	if (result != TCL_OK) {
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto checkForCatch;
	}

	result = TclGetBooleanFromObj(NULL, value2Ptr, &i2);
	if (result != TCL_OK) {
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(value2Ptr),
		    (value2Ptr->typePtr? value2Ptr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, value2Ptr);
	    CACHE_STACK_INFO();
	    goto checkForCatch;
	}

	if (*pc == INST_LOR) {
	    iResult = (i1 || i2);
	} else {
	    iResult = (i1 && i2);
	}
	objResultPtr = constants[iResult];
	TRACE(("%.20s %.20s => %d\n", O2S(valuePtr),O2S(value2Ptr),iResult));
	NEXT_INST_F(1, 2, 1);
    }

    /*
     * ---------------------------------------------------------
     *	   Start of INST_LIST and related instructions.
     */

    case INST_LIST: {
	/*
	 * Pop the opnd (objc) top stack elements into a new list obj and then
	 * decrement their ref counts.
	 */

	int opnd;

	opnd = TclGetUInt4AtPtr(pc+1);
	objResultPtr = Tcl_NewListObj(opnd, &OBJ_AT_DEPTH(opnd-1));
	TRACE_WITH_OBJ(("%u => ", opnd), objResultPtr);
	NEXT_INST_V(5, opnd, 1);
    }

    case INST_LIST_LENGTH: {
	Tcl_Obj *valuePtr;
	int length;

	valuePtr = OBJ_AT_TOS;

	result = TclListObjLength(interp, valuePtr, &length);
	if (result == TCL_OK) {
	    TclNewIntObj(objResultPtr, length);
	    TRACE(("%.20s => %d\n", O2S(valuePtr), length));
	    NEXT_INST_F(1, 1, 1);
	} else {
	    TRACE_WITH_OBJ(("%.30s => ERROR: ", O2S(valuePtr)),
		    Tcl_GetObjResult(interp));
	    goto checkForCatch;
	}
    }

    case INST_LIST_INDEX: {
	/*** lindex with objc == 3 ***/

	/* Variables also for INST_LIST_INDEX_IMM */

	int listc, idx, opnd, pcAdjustment;
	Tcl_Obj **listv;
	Tcl_Obj *valuePtr, *value2Ptr;

	/*
	 * Pop the two operands.
	 */

	value2Ptr = OBJ_AT_TOS;
	valuePtr = OBJ_UNDER_TOS;

	/*
	 * Extract the desired list element.
	 */

	result = TclListObjGetElements(interp, valuePtr, &listc, &listv);
	if ((result == TCL_OK) && (value2Ptr->typePtr != &tclListType)
		&& (TclGetIntForIndexM(NULL , value2Ptr, listc-1,
			&idx) == TCL_OK)) {
	    TclDecrRefCount(value2Ptr);
	    tosPtr--;
	    pcAdjustment = 1;
	    goto lindexFastPath;
	}

	objResultPtr = TclLindexList(interp, valuePtr, value2Ptr);
	if (objResultPtr) {
	    /*
	     * Stash the list element on the stack.
	     */

	    TRACE(("%.20s %.20s => %s\n",
		    O2S(valuePtr), O2S(value2Ptr), O2S(objResultPtr)));
	    NEXT_INST_F(1, 2, -1);	/* Already has the correct refCount */
	} else {
	    TRACE_WITH_OBJ(("%.30s %.30s => ERROR: ", O2S(valuePtr),
		    O2S(value2Ptr)), Tcl_GetObjResult(interp));
	    result = TCL_ERROR;
	    goto checkForCatch;
	}

    case INST_LIST_INDEX_IMM:
	/*** lindex with objc==3 and index in bytecode stream ***/

	pcAdjustment = 5;

	/*
	 * Pop the list and get the index.
	 */

	valuePtr = OBJ_AT_TOS;
	opnd = TclGetInt4AtPtr(pc+1);

	/*
	 * Get the contents of the list, making sure that it really is a list
	 * in the process.
	 */

	result = TclListObjGetElements(interp, valuePtr, &listc, &listv);

	if (result == TCL_OK) {
	    /*
	     * Select the list item based on the index. Negative operand means
	     * end-based indexing.
	     */

	    if (opnd < -1) {
