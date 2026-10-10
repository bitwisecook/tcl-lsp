    case INST_LNOT: {
	int b;
	Tcl_Obj *valuePtr = OBJ_AT_TOS;

	/* TODO - check claim that taking address of b harms performance */
	/* TODO - consider optimization search for constants */
	result = TclGetBooleanFromObj(NULL, valuePtr, &b);
	if (result != TCL_OK) {
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s\n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto checkForCatch;
	}
	/* TODO: Consider peephole opt. */
	objResultPtr = constants[!b];
	NEXT_INST_F(1, 1, 1);
    }

    case INST_BITNOT: {
	mp_int big;
	ClientData ptr;
	int type;
	Tcl_Obj *valuePtr = OBJ_AT_TOS;

	result = GetNumberFromObj(NULL, valuePtr, &ptr, &type);
	if ((result != TCL_OK)
		|| (type == TCL_NUMBER_NAN) || (type == TCL_NUMBER_DOUBLE)) {
	    /*
	     * ... ~$NonInteger => raise an error.
	     */

	    result = TCL_ERROR;
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto checkForCatch;
	}
	if (type == TCL_NUMBER_LONG) {
	    long l = *((const long *)ptr);

	    if (Tcl_IsShared(valuePtr)) {
		TclNewLongObj(objResultPtr, ~l);
		NEXT_INST_F(1, 1, 1);
	    }
	    TclSetLongObj(valuePtr, ~l);
	    NEXT_INST_F(1, 0, 0);
	}
#ifndef NO_WIDE_TYPE
	if (type == TCL_NUMBER_WIDE) {
	    Tcl_WideInt w = *((const Tcl_WideInt *)ptr);

	    if (Tcl_IsShared(valuePtr)) {
		objResultPtr = Tcl_NewWideIntObj(~w);
		NEXT_INST_F(1, 1, 1);
	    }
	    Tcl_SetWideIntObj(valuePtr, ~w);
	    NEXT_INST_F(1, 0, 0);
	}
#endif
	Tcl_TakeBignumFromObj(NULL, valuePtr, &big);
	/* ~a = - a - 1 */
	mp_neg(&big, &big);
	mp_sub_d(&big, 1, &big);
	if (Tcl_IsShared(valuePtr)) {
	    objResultPtr = Tcl_NewBignumObj(&big);
	    NEXT_INST_F(1, 1, 1);
	}
	Tcl_SetBignumObj(valuePtr, &big);
	NEXT_INST_F(1, 0, 0);
    }

    case INST_UMINUS: {
	ClientData ptr;
	int type;
	Tcl_Obj *valuePtr = OBJ_AT_TOS;

	result = GetNumberFromObj(NULL, valuePtr, &ptr, &type);
	if ((result != TCL_OK)
#ifndef ACCEPT_NAN
		|| (type == TCL_NUMBER_NAN)
#endif
		) {
	    result = TCL_ERROR;
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto checkForCatch;
	}
	switch (type) {
	case TCL_NUMBER_DOUBLE: {
	    double d;

	    if (Tcl_IsShared(valuePtr)) {
		TclNewDoubleObj(objResultPtr, -(*((const double *)ptr)));
