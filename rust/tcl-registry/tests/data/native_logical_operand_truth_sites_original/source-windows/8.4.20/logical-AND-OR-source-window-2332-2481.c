    case INST_LOR:
    case INST_LAND:
    {
	/*
	 * Operands must be boolean or numeric. No int->double
	 * conversions are performed.
	 */
		
	int i1, i2;
	int iResult;
	char *s;
	Tcl_ObjType *t1Ptr, *t2Ptr;

	value2Ptr = stackPtr[stackTop];
	valuePtr  = stackPtr[stackTop - 1];;
	t1Ptr = valuePtr->typePtr;
	t2Ptr = value2Ptr->typePtr;

	if ((t1Ptr == &tclIntType) || (t1Ptr == &tclBooleanType)) {
	    i1 = (valuePtr->internalRep.longValue != 0);
	} else if (t1Ptr == &tclWideIntType) {
	    TclGetWide(w,valuePtr);
	    i1 = (w != W0);
	} else if (t1Ptr == &tclDoubleType) {
	    i1 = (valuePtr->internalRep.doubleValue != 0.0);
	} else {
	    s = Tcl_GetStringFromObj(valuePtr, &length);
	    if (TclLooksLikeInt(s, length)) {
		GET_WIDE_OR_INT(result, valuePtr, i, w);
		if (valuePtr->typePtr == &tclIntType) {
		    i1 = (i != 0);
		} else {
		    i1 = (w != W0);
		}
	    } else {
		result = Tcl_GetBooleanFromObj((Tcl_Interp *) NULL,
					       valuePtr, &i1);
		i1 = (i1 != 0);
	    }
	    if (result != TCL_OK) {
		TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(valuePtr),
		        (t1Ptr? t1Ptr->name : "null")));
		DECACHE_STACK_INFO();
		IllegalExprOperandType(interp, pc, valuePtr);
		CACHE_STACK_INFO();
		goto checkForCatch;
	    }
	}
		
	if ((t2Ptr == &tclIntType) || (t2Ptr == &tclBooleanType)) {
	    i2 = (value2Ptr->internalRep.longValue != 0);
	} else if (t2Ptr == &tclWideIntType) {
	    TclGetWide(w,value2Ptr);
	    i2 = (w != W0);
	} else if (t2Ptr == &tclDoubleType) {
	    i2 = (value2Ptr->internalRep.doubleValue != 0.0);
	} else {
	    s = Tcl_GetStringFromObj(value2Ptr, &length);
	    if (TclLooksLikeInt(s, length)) {
		GET_WIDE_OR_INT(result, value2Ptr, i, w);
		if (value2Ptr->typePtr == &tclIntType) {
		    i2 = (i != 0);
		} else {
		    i2 = (w != W0);
		}
	    } else {
		result = Tcl_GetBooleanFromObj((Tcl_Interp *) NULL, value2Ptr, &i2);
	    }
	    if (result != TCL_OK) {
		TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(value2Ptr),
		        (t2Ptr? t2Ptr->name : "null")));
		DECACHE_STACK_INFO();
		IllegalExprOperandType(interp, pc, value2Ptr);
		CACHE_STACK_INFO();
		goto checkForCatch;
	    }
	}

	/*
	 * Reuse the valuePtr object already on stack if possible.
	 */
	
	if (*pc == INST_LOR) {
	    iResult = (i1 || i2);
	} else {
	    iResult = (i1 && i2);
	}
	if (Tcl_IsShared(valuePtr)) {
	    objResultPtr = Tcl_NewLongObj(iResult);
	    TRACE(("%.20s %.20s => %d\n", O2S(valuePtr), O2S(value2Ptr), iResult));
	    NEXT_INST_F(1, 2, 1);
	} else {	/* reuse the valuePtr object */
	    TRACE(("%.20s %.20s => %d\n", O2S(valuePtr), O2S(value2Ptr), iResult));
	    Tcl_SetLongObj(valuePtr, iResult);
	    NEXT_INST_F(1, 1, 0);
	}
    }

    /*
     * ---------------------------------------------------------
     *     Start of INST_LIST and related instructions.
     */

    case INST_LIST:
	/*
	 * Pop the opnd (objc) top stack elements into a new list obj
	 * and then decrement their ref counts. 
	 */

	opnd = TclGetUInt4AtPtr(pc+1);
	objResultPtr = Tcl_NewListObj(opnd, &(stackPtr[stackTop - (opnd-1)]));
	TRACE_WITH_OBJ(("%u => ", opnd), objResultPtr);
	NEXT_INST_V(5, opnd, 1);

    case INST_LIST_LENGTH:
	valuePtr = stackPtr[stackTop];

	result = Tcl_ListObjLength(interp, valuePtr, &length);
	if (result != TCL_OK) {
	    TRACE_WITH_OBJ(("%.30s => ERROR: ", O2S(valuePtr)),
	            Tcl_GetObjResult(interp));
	    goto checkForCatch;
	}
	objResultPtr = Tcl_NewIntObj(length);
	TRACE(("%.20s => %d\n", O2S(valuePtr), length));
	NEXT_INST_F(1, 1, 1);
	    
    case INST_LIST_INDEX:
	/*** lindex with objc == 3 ***/
		
	/*
	 * Pop the two operands
	 */
	value2Ptr = stackPtr[stackTop];
	valuePtr  = stackPtr[stackTop- 1];

	/*
	 * Extract the desired list element
	 */
	objResultPtr = TclLindexList(interp, valuePtr, value2Ptr);
	if (objResultPtr == NULL) {
	    TRACE_WITH_OBJ(("%.30s %.30s => ERROR: ", O2S(valuePtr), O2S(value2Ptr)),
	            Tcl_GetObjResult(interp));
	    result = TCL_ERROR;
	    goto checkForCatch;
	}

	/*
	 * Stash the list element on the stack
	 */
