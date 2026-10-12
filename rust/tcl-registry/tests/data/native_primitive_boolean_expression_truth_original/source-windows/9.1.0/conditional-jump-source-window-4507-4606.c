    case INST_JUMP_FALSE1:
	DEPRECATED_OPCODE_MARK(INST_JUMP_FALSE1);
	jmpOffset[0] = TclGetInt1AtPtr(pc + 1);
	jmpOffset[1] = 2;
	TRACE("%d => ", jmpOffset[0]);
	goto doCondJump;

    case INST_JUMP_TRUE1:
	DEPRECATED_OPCODE_MARK(INST_JUMP_TRUE1);
	jmpOffset[0] = 2;
	jmpOffset[1] = TclGetInt1AtPtr(pc + 1);
	TRACE("%d => ", jmpOffset[1]);
	goto doCondJump;
#endif

    case INST_JUMP_FALSE:
	jmpOffset[0] = TclGetInt4AtPtr(pc + 1);	/* FALSE offset */
	jmpOffset[1] = 5;			/* TRUE offset */
	TRACE("%d => ", jmpOffset[0]);
	goto doCondJump;

    case INST_JUMP_TRUE:
	jmpOffset[0] = 5;
	jmpOffset[1] = TclGetInt4AtPtr(pc + 1);
	TRACE("%d => ", jmpOffset[1]);

    doCondJump:
	valuePtr = OBJ_AT_TOS;

	/* TODO - check claim that taking address of b harms performance */
	/* TODO - consider optimization search for constants */
	if (TclGetBooleanFromObj(interp, valuePtr, &b) != TCL_OK) {
	    TRACE_ERROR(interp);
	    goto gotError;
	}

#ifdef TCL_COMPILE_DEBUG
	if (b) {
	    if ((*pc == INST_JUMP_TRUE)
#ifndef REMOVE_DEPRECATED_OPCODES
		    ||  (*pc == INST_JUMP_TRUE1)
#endif
		    ) {
		TRACE_APPEND("%.20s true, new pc %" SIZEd "\n", O2S(valuePtr),
			PC_REL + jmpOffset[1]);
	    } else {
		TRACE_APPEND("%.20s true\n", O2S(valuePtr));
	    }
	} else {
	    if ((*pc == INST_JUMP_TRUE)
#ifndef REMOVE_DEPRECATED_OPCODES
		    || (*pc == INST_JUMP_TRUE1)
#endif
		    ) {
		TRACE_APPEND("%.20s false\n", O2S(valuePtr));
	    } else {
		TRACE_APPEND("%.20s false, new pc %" SIZEd "\n", O2S(valuePtr),
			PC_REL + jmpOffset[0]);
	    }
	}
#endif
	NEXT_INST_F0(jmpOffset[b], 1);
    }

    {
	Tcl_HashEntry *hPtr;
	JumptableInfo *jtPtr;
	JumptableNumInfo *jtnPtr;

	/*
	 * Jump to location looked up in a hashtable; fall through to next
	 * instr if lookup fails. Lookup by string.
	 */

    case INST_JUMP_TABLE:
	tblIdx = TclGetInt4AtPtr(pc + 1);
	jtPtr = (JumptableInfo *)
		codePtr->auxDataArrayPtr[tblIdx].clientData;
	TRACE("%u \"%.20s\" => ", tblIdx, O2S(OBJ_AT_TOS));
	hPtr = Tcl_FindHashEntry(&jtPtr->hashTable, TclGetString(OBJ_AT_TOS));
	goto processJumpTableEntry;

	/*
	 * Jump to location looked up in a hashtable; fall through to next
	 * instr if lookup fails or key is non-integer. Lookup by integer.
	 */

    case INST_JUMP_TABLE_NUM:
	tblIdx = TclGetInt4AtPtr(pc + 1);
	jtnPtr = (JumptableNumInfo *)
		codePtr->auxDataArrayPtr[tblIdx].clientData;
	TRACE("%u \"%.20s\" => ", tblIdx, O2S(OBJ_AT_TOS));
	DECACHE_STACK_INFO();
	Tcl_WideInt key;
	if (Tcl_GetWideIntFromObj(interp, OBJ_AT_TOS, &key) != TCL_OK) {
	    TRACE_ERROR(interp);
	    goto gotError;
	}
	CACHE_STACK_INFO();
	hPtr = Tcl_FindHashEntry(&jtnPtr->hashTable, INT2PTR(key));
