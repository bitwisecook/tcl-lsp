    case INST_JUMP_FALSE4:
	opnd = 5;                             /* TRUE */
	pcAdjustment = TclGetInt4AtPtr(pc+1); /* FALSE */
	goto doJumpTrue;

    case INST_JUMP_TRUE4:
	opnd = TclGetInt4AtPtr(pc+1);         /* TRUE */
	pcAdjustment = 5;                     /* FALSE */
	goto doJumpTrue;

    case INST_JUMP_FALSE1:
	opnd = 2;                             /* TRUE */
	pcAdjustment = TclGetInt1AtPtr(pc+1); /* FALSE */
	goto doJumpTrue;

    case INST_JUMP_TRUE1:
	opnd = TclGetInt1AtPtr(pc+1);          /* TRUE */
	pcAdjustment = 2;                      /* FALSE */
	    
    doJumpTrue:
	{
	    int b;
		
	    valuePtr = stackPtr[stackTop];
	    if (valuePtr->typePtr == &tclIntType) {
		b = (valuePtr->internalRep.longValue != 0);
	    } else if (valuePtr->typePtr == &tclDoubleType) {
		b = (valuePtr->internalRep.doubleValue != 0.0);
	    } else if (valuePtr->typePtr == &tclWideIntType) {
		TclGetWide(w,valuePtr);
		b = (w != W0);
	    } else {
		result = Tcl_GetBooleanFromObj(interp, valuePtr, &b);
		if (result != TCL_OK) {
		    TRACE_WITH_OBJ(("%d => ERROR: ", opnd), Tcl_GetObjResult(interp));
		    goto checkForCatch;
		}
	    }
#ifndef TCL_COMPILE_DEBUG
	    NEXT_INST_F((b? opnd : pcAdjustment), 1, 0);
#else
	    if (b) {
		if ((*pc == INST_JUMP_TRUE1) || (*pc == INST_JUMP_TRUE1)) {
		    TRACE(("%d => %.20s true, new pc %u\n", opnd, O2S(valuePtr),
		            (unsigned int)(pc+opnd - codePtr->codeStart)));
		} else {
		    TRACE(("%d => %.20s true\n", pcAdjustment, O2S(valuePtr)));
		}
		NEXT_INST_F(opnd, 1, 0);
	    } else {
		if ((*pc == INST_JUMP_TRUE1) || (*pc == INST_JUMP_TRUE1)) {
		    TRACE(("%d => %.20s false\n", opnd, O2S(valuePtr)));
		} else {
		    opnd = pcAdjustment;
		    TRACE(("%d => %.20s false, new pc %u\n", opnd, O2S(valuePtr),
		            (unsigned int)(pc + opnd - codePtr->codeStart)));
		}
		NEXT_INST_F(pcAdjustment, 1, 0);
	    }
#endif
	}
	    	    
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
