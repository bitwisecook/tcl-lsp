    case INST_LNOT: {
	valuePtr = OBJ_AT_TOS;
	TRACE("\"%.20s\" => ", O2S(valuePtr));

	/* TODO - check claim that taking address of b harms performance */
	/* TODO - consider optimization search for constants */
	int b;
	if (TclGetBooleanFromObj(NULL, valuePtr, &b) != TCL_OK) {
	    TRACE_APPEND("ERROR: illegal type %s\n", TY2S(valuePtr->typePtr));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, "", pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	/* TODO: Consider peephole opt. */
	objResultPtr = TCONST(!b);
	TRACE_APPEND_OBJ(objResultPtr);
	NEXT_INST_F(1, 1, 1);
    }

    case INST_BITNOT:
	valuePtr = OBJ_AT_TOS;
	TRACE("\"%.20s\" => ", O2S(valuePtr));
	if ((GetNumberFromObj(NULL, valuePtr, &ptr1, &type1) != TCL_OK)
		|| (type1==TCL_NUMBER_NAN) || (type1==TCL_NUMBER_DOUBLE)) {
	    /*
	     * ... ~$NonInteger => raise an error.
	     */

	    TRACE_APPEND("ERROR: illegal type %s\n", TY2S(valuePtr->typePtr));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, "", pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	if (type1 == TCL_NUMBER_INT) {
	    w1 = *((const Tcl_WideInt *) ptr1);
	    if (Tcl_IsShared(valuePtr)) {
		TclNewIntObj(objResultPtr, ~w1);
		TRACE_APPEND_NUM_OBJ(objResultPtr);
		NEXT_INST_F(1, 1, 1);
	    }
	    TclSetIntObj(valuePtr, ~w1);
	    TRACE_APPEND("%s\n", O2S(valuePtr));
	    NEXT_INST_F0(1, 0);
	}
	objResultPtr = ExecuteExtendedUnaryMathOp(*pc, valuePtr);
	if (objResultPtr != NULL) {
	    TRACE_APPEND_NUM_OBJ(objResultPtr);
	    NEXT_INST_F(1, 1, 1);
	} else {
	    TRACE_APPEND_NUM_OBJ(valuePtr);
	    NEXT_INST_F0(1, 0);
	}

    case INST_UMINUS:
	valuePtr = OBJ_AT_TOS;
	TRACE("\"%.20s\" => ", O2S(valuePtr));
	if ((GetNumberFromObj(NULL, valuePtr, &ptr1, &type1) != TCL_OK)
		|| IsErroringNaNType(type1)) {
	    TRACE_APPEND("ERROR: illegal type %s\n", TY2S(valuePtr->typePtr));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, "", pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	switch (type1) {
	case TCL_NUMBER_NAN:
	    /* -NaN => NaN */
	    TRACE_APPEND_NUM_OBJ(valuePtr);
	    NEXT_INST_F0(1, 0);
	case TCL_NUMBER_INT:
	    w1 = *((const Tcl_WideInt *) ptr1);
	    if (w1 != WIDE_MIN) {
		if (Tcl_IsShared(valuePtr)) {
		    TclNewIntObj(objResultPtr, -w1);
		    TRACE_APPEND_NUM_OBJ(objResultPtr);
		    NEXT_INST_F(1, 1, 1);
		}
		TclSetIntObj(valuePtr, -w1);
		TRACE_APPEND_NUM_OBJ(valuePtr);
		NEXT_INST_F0(1, 0);
	    }
	    TCL_FALLTHROUGH();
	default:
	    break;
	}
	objResultPtr = ExecuteExtendedUnaryMathOp(*pc, valuePtr);
	if (objResultPtr != NULL) {
	    TRACE_APPEND_NUM_OBJ(objResultPtr);
	    NEXT_INST_F(1, 1, 1);
	} else {
	    TRACE_APPEND_NUM_OBJ(valuePtr);
	    NEXT_INST_F0(1, 0);
	}

    case INST_UPLUS:
    case INST_TRY_CVT_TO_NUMERIC:
	/*
	 * Try to convert the topmost stack object to numeric object. This is
