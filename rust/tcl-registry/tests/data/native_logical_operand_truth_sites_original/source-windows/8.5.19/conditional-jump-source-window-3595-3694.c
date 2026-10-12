    case INST_JUMP_FALSE4:
	jmpOffset[0] = TclGetInt4AtPtr(pc+1);	/* FALSE offset */
	jmpOffset[1] = 5;			/* TRUE offset*/
	goto doCondJump;

    case INST_JUMP_TRUE4:
	jmpOffset[0] = 5;
	jmpOffset[1] = TclGetInt4AtPtr(pc+1);
	goto doCondJump;

    case INST_JUMP_FALSE1:
	jmpOffset[0] = TclGetInt1AtPtr(pc+1);
	jmpOffset[1] = 2;
	goto doCondJump;

    case INST_JUMP_TRUE1:
	jmpOffset[0] = 2;
	jmpOffset[1] = TclGetInt1AtPtr(pc+1);

    doCondJump:
	valuePtr = OBJ_AT_TOS;

	/* TODO - check claim that taking address of b harms performance */
	/* TODO - consider optimization search for constants */
	result = TclGetBooleanFromObj(interp, valuePtr, &b);
	if (result != TCL_OK) {
	    TRACE_WITH_OBJ(("%d => ERROR: ", jmpOffset[
		    ((*pc == INST_JUMP_FALSE1) || (*pc == INST_JUMP_FALSE4))
		    ? 0 : 1]), Tcl_GetObjResult(interp));
	    goto checkForCatch;
	}

#ifdef TCL_COMPILE_DEBUG
	if (b) {
	    if ((*pc == INST_JUMP_TRUE1) || (*pc == INST_JUMP_TRUE4)) {
		TRACE(("%d => %.20s true, new pc %u\n", jmpOffset[1],
			O2S(valuePtr),
			(unsigned)(pc + jmpOffset[1] - codePtr->codeStart)));
	    } else {
		TRACE(("%d => %.20s true\n", jmpOffset[0], O2S(valuePtr)));
	    }
	} else {
	    if ((*pc == INST_JUMP_TRUE1) || (*pc == INST_JUMP_TRUE4)) {
		TRACE(("%d => %.20s false\n", jmpOffset[0], O2S(valuePtr)));
	    } else {
		TRACE(("%d => %.20s false, new pc %u\n", jmpOffset[0],
			O2S(valuePtr),
			(unsigned)(pc + jmpOffset[1] - codePtr->codeStart)));
	    }
	}
#endif
	NEXT_INST_F(jmpOffset[b], 1, 0);
    }

    case INST_JUMP_TABLE: {
	Tcl_HashEntry *hPtr;
	JumptableInfo *jtPtr;
	int opnd;

	/*
	 * Jump to location looked up in a hashtable; fall through to next
	 * instr if lookup fails.
	 */

	opnd = TclGetInt4AtPtr(pc+1);
	jtPtr = (JumptableInfo *) codePtr->auxDataArrayPtr[opnd].clientData;
	TRACE(("%d => %.20s ", opnd, O2S(OBJ_AT_TOS)));
	hPtr = Tcl_FindHashEntry(&jtPtr->hashTable, TclGetString(OBJ_AT_TOS));
	if (hPtr != NULL) {
	    int jumpOffset = PTR2INT(Tcl_GetHashValue(hPtr));

	    TRACE_APPEND(("found in table, new pc %u\n",
		    (unsigned)(pc - codePtr->codeStart + jumpOffset)));
	    NEXT_INST_F(jumpOffset, 1, 0);
	} else {
	    TRACE_APPEND(("not found in table\n"));
	    NEXT_INST_F(5, 1, 0);
	}
    }

    /*
     * These two instructions are now redundant: the complete logic of the LOR
     * and LAND is now handled by the expression compiler.
     */

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
