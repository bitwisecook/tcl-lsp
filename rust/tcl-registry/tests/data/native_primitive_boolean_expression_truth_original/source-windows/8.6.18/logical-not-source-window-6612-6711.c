    case INST_LNOT: {
	int b;

	valuePtr = OBJ_AT_TOS;

	/* TODO - check claim that taking address of b harms performance */
	/* TODO - consider optimization search for constants */
	if (TclGetBooleanFromObj(NULL, valuePtr, &b) != TCL_OK) {
	    TRACE(("\"%.20s\" => ERROR: illegal type %s\n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	/* TODO: Consider peephole opt. */
	objResultPtr = TCONST(!b);
	TRACE_WITH_OBJ(("%s => ", O2S(valuePtr)), objResultPtr);
	NEXT_INST_F(1, 1, 1);
    }

    case INST_BITNOT:
	valuePtr = OBJ_AT_TOS;
	TRACE(("\"%.20s\" => ", O2S(valuePtr)));
	if ((GetNumberFromObj(NULL, valuePtr, &ptr1, &type1) != TCL_OK)
		|| (type1==TCL_NUMBER_NAN) || (type1==TCL_NUMBER_DOUBLE)) {
	    /*
	     * ... ~$NonInteger => raise an error.
	     */

	    TRACE_APPEND(("ERROR: illegal type %s\n",
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	if (type1 == TCL_NUMBER_LONG) {
	    l1 = *((const long *) ptr1);
	    if (Tcl_IsShared(valuePtr)) {
		TclNewLongObj(objResultPtr, ~l1);
		TRACE_APPEND(("%s\n", O2S(objResultPtr)));
		NEXT_INST_F(1, 1, 1);
	    }
	    TclSetLongObj(valuePtr, ~l1);
	    TRACE_APPEND(("%s\n", O2S(valuePtr)));
	    NEXT_INST_F(1, 0, 0);
	}
	objResultPtr = ExecuteExtendedUnaryMathOp(*pc, valuePtr);
	if (objResultPtr != NULL) {
	    TRACE_APPEND(("%s\n", O2S(objResultPtr)));
	    NEXT_INST_F(1, 1, 1);
	} else {
	    TRACE_APPEND(("%s\n", O2S(valuePtr)));
	    NEXT_INST_F(1, 0, 0);
	}

    case INST_UMINUS:
	valuePtr = OBJ_AT_TOS;
	TRACE(("\"%.20s\" => ", O2S(valuePtr)));
	if ((GetNumberFromObj(NULL, valuePtr, &ptr1, &type1) != TCL_OK)
		|| IsErroringNaNType(type1)) {
	    TRACE_APPEND(("ERROR: illegal type %s \n",
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	switch (type1) {
	case TCL_NUMBER_NAN:
	    /* -NaN => NaN */
	    TRACE_APPEND(("%s\n", O2S(valuePtr)));
	    NEXT_INST_F(1, 0, 0);
	break;
	case TCL_NUMBER_LONG:
	    l1 = *((const long *) ptr1);
	    if (l1 != LONG_MIN) {
		if (Tcl_IsShared(valuePtr)) {
		    TclNewLongObj(objResultPtr, -l1);
		    TRACE_APPEND(("%s\n", O2S(objResultPtr)));
		    NEXT_INST_F(1, 1, 1);
		}
		TclSetLongObj(valuePtr, -l1);
		TRACE_APPEND(("%s\n", O2S(valuePtr)));
		NEXT_INST_F(1, 0, 0);
	    }
	    /* FALLTHROUGH */
	}
	objResultPtr = ExecuteExtendedUnaryMathOp(*pc, valuePtr);
	if (objResultPtr != NULL) {
	    TRACE_APPEND(("%s\n", O2S(objResultPtr)));
	    NEXT_INST_F(1, 1, 1);
	} else {
	    TRACE_APPEND(("%s\n", O2S(valuePtr)));
	    NEXT_INST_F(1, 0, 0);
	}

    case INST_UPLUS:
    case INST_TRY_CVT_TO_NUMERIC:
