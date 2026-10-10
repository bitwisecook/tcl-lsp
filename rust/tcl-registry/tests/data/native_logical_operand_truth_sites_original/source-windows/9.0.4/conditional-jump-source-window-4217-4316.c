    case INST_JUMP_FALSE4:
	jmpOffset[0] = TclGetInt4AtPtr(pc + 1);	/* FALSE offset */
	jmpOffset[1] = 5;			/* TRUE offset */
	goto doCondJump;

    case INST_JUMP_TRUE4:
	jmpOffset[0] = 5;
	jmpOffset[1] = TclGetInt4AtPtr(pc + 1);
	goto doCondJump;

    case INST_JUMP_FALSE1:
	jmpOffset[0] = TclGetInt1AtPtr(pc + 1);
	jmpOffset[1] = 2;
	goto doCondJump;

    case INST_JUMP_TRUE1:
	jmpOffset[0] = 2;
	jmpOffset[1] = TclGetInt1AtPtr(pc + 1);

    doCondJump:
	valuePtr = OBJ_AT_TOS;
	TRACE(("%d => ", jmpOffset[
		(*pc==INST_JUMP_FALSE1 || *pc==INST_JUMP_FALSE4) ? 0 : 1]));

	/* TODO - check claim that taking address of b harms performance */
	/* TODO - consider optimization search for constants */
	if (TclGetBooleanFromObj(interp, valuePtr, &b) != TCL_OK) {
	    TRACE_ERROR(interp);
	    goto gotError;
	}

#ifdef TCL_COMPILE_DEBUG
	if (b) {
	    if ((*pc == INST_JUMP_TRUE1) || (*pc == INST_JUMP_TRUE4)) {
		TRACE_APPEND(("%.20s true, new pc %" TCL_Z_MODIFIER "u\n", O2S(valuePtr),
			(size_t)(pc + jmpOffset[1] - codePtr->codeStart)));
	    } else {
		TRACE_APPEND(("%.20s true\n", O2S(valuePtr)));
	    }
	} else {
	    if ((*pc == INST_JUMP_TRUE1) || (*pc == INST_JUMP_TRUE4)) {
		TRACE_APPEND(("%.20s false\n", O2S(valuePtr)));
	    } else {
		TRACE_APPEND(("%.20s false, new pc %" TCL_Z_MODIFIER "u\n", O2S(valuePtr),
			(size_t)(pc + jmpOffset[0] - codePtr->codeStart)));
	    }
	}
#endif
	NEXT_INST_F(jmpOffset[b], 1, 0);
    }
    break;

    case INST_JUMP_TABLE: {
	Tcl_HashEntry *hPtr;
	JumptableInfo *jtPtr;

	/*
	 * Jump to location looked up in a hashtable; fall through to next
	 * instr if lookup fails.
	 */

	opnd = TclGetInt4AtPtr(pc + 1);
	jtPtr = (JumptableInfo *) codePtr->auxDataArrayPtr[opnd].clientData;
	TRACE(("%d \"%.20s\" => ", opnd, O2S(OBJ_AT_TOS)));
	hPtr = Tcl_FindHashEntry(&jtPtr->hashTable, TclGetString(OBJ_AT_TOS));
	if (hPtr != NULL) {
	    Tcl_Size jumpOffset = PTR2INT(Tcl_GetHashValue(hPtr));

	    TRACE_APPEND(("found in table, new pc %" TCL_Z_MODIFIER "u\n",
		    (size_t)(pc - codePtr->codeStart + jumpOffset)));
	    NEXT_INST_F(jumpOffset, 1, 0);
	} else {
	    TRACE_APPEND(("not found in table\n"));
	    NEXT_INST_F(5, 1, 0);
	}
    }
    break;

    /*
     * -----------------------------------------------------------------
     *	   Start of general introspector instructions.
     */

    case INST_NS_CURRENT:
	objResultPtr = TclNewNamespaceObj(TclGetCurrentNamespace(interp));
	TRACE_WITH_OBJ(("=> "), objResultPtr);
	NEXT_INST_F(1, 0, 1);
    break;
    case INST_COROUTINE_NAME: {
	CoroutineData *corPtr = iPtr->execEnvPtr->corPtr;

	TclNewObj(objResultPtr);
	if (corPtr && !(corPtr->cmdPtr->flags & CMD_DYING)) {
	    Tcl_GetCommandFullName(interp, (Tcl_Command) corPtr->cmdPtr,
		    objResultPtr);
	}
	TRACE_WITH_OBJ(("=> "), objResultPtr);
	NEXT_INST_F(1, 0, 1);
    }
    break;
