    case INST_LNOT:
    {
	/*
	 * The operand must be numeric or a boolean string as
	 * accepted by Tcl_GetBooleanFromObj(). If the operand
	 * object is unshared modify it directly, otherwise
	 * create a copy to modify: this is "copy on write".
	 * Free any old string representation since it is now
	 * invalid.
	 */

	double d;
	int boolvar;
	Tcl_ObjType *tPtr;

	valuePtr = stackPtr[stackTop];
	tPtr = valuePtr->typePtr;
	if (!IS_INTEGER_TYPE(tPtr) && ((tPtr != &tclDoubleType)
	        || (valuePtr->bytes != NULL))) {
	    if ((tPtr == &tclBooleanType) && (valuePtr->bytes == NULL)) {
		valuePtr->typePtr = &tclIntType;
	    } else {
		char *s = Tcl_GetStringFromObj(valuePtr, &length);
		if (TclLooksLikeInt(s, length)) {
		    GET_WIDE_OR_INT(result, valuePtr, i, w);
		} else {
		    result = Tcl_GetDoubleFromObj((Tcl_Interp *) NULL,
		            valuePtr, &d);
		}
		if (result == TCL_ERROR && *pc == INST_LNOT) {
		    result = Tcl_GetBooleanFromObj((Tcl_Interp *)NULL,
		            valuePtr, &boolvar);
		    i = (long)boolvar; /* i is long, not int! */
		}
		if (result != TCL_OK) {
		    TRACE(("\"%.20s\" => ILLEGAL TYPE %s\n",
		            s, (tPtr? tPtr->name : "null")));
		    DECACHE_STACK_INFO();
		    IllegalExprOperandType(interp, pc, valuePtr);
		    CACHE_STACK_INFO();
		    goto checkForCatch;
		}
	    }
	    tPtr = valuePtr->typePtr;
	}

	if (Tcl_IsShared(valuePtr)) {
	    /*
	     * Create a new object.
	     */
	    if ((tPtr == &tclIntType) || (tPtr == &tclBooleanType)) {
		i = valuePtr->internalRep.longValue;
		objResultPtr = Tcl_NewLongObj(
		    (*pc == INST_UMINUS)? -i : !i);
		TRACE_WITH_OBJ(("%ld => ", i), objResultPtr);
	    } else if (tPtr == &tclWideIntType) {
		TclGetWide(w,valuePtr);
		if (*pc == INST_UMINUS) {
		    objResultPtr = Tcl_NewWideIntObj(-w);
		} else {
		    objResultPtr = Tcl_NewLongObj(w == W0);
		}
		TRACE_WITH_OBJ((LLD" => ", w), objResultPtr);
	    } else {
		d = valuePtr->internalRep.doubleValue;
		if (*pc == INST_UMINUS) {
		    objResultPtr = Tcl_NewDoubleObj(-d);
		} else {
		    /*
		     * Should be able to use "!d", but apparently
		     * some compilers can't handle it.
		     */
		    objResultPtr = Tcl_NewLongObj((d==0.0)? 1 : 0);
		}
		TRACE_WITH_OBJ(("%.6g => ", d), objResultPtr);
	    }
	    NEXT_INST_F(1, 1, 1);
	} else {
	    /*
	     * valuePtr is unshared. Modify it directly.
	     */
	    if ((tPtr == &tclIntType) || (tPtr == &tclBooleanType)) {
		i = valuePtr->internalRep.longValue;
		Tcl_SetLongObj(valuePtr,
	                (*pc == INST_UMINUS)? -i : !i);
		TRACE_WITH_OBJ(("%ld => ", i), valuePtr);
	    } else if (tPtr == &tclWideIntType) {
		TclGetWide(w,valuePtr);
		if (*pc == INST_UMINUS) {
		    Tcl_SetWideIntObj(valuePtr, -w);
		} else {
		    Tcl_SetLongObj(valuePtr, w == W0);
		}
		TRACE_WITH_OBJ((LLD" => ", w), valuePtr);
	    } else {
		d = valuePtr->internalRep.doubleValue;
		if (*pc == INST_UMINUS) {
		    Tcl_SetDoubleObj(valuePtr, -d);
		} else {
		    /*
