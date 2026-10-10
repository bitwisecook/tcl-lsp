    case INST_LOR:
    case INST_LAND: {
	/*
	 * Operands must be boolean or numeric. No int->double conversions are
	 * performed.
	 */

	int i1, i2, iResult;

	value2Ptr = OBJ_AT_TOS;
	valuePtr = OBJ_UNDER_TOS;
	if (TclGetBooleanFromObj(NULL, valuePtr, &i1) != TCL_OK) {
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}

	if (TclGetBooleanFromObj(NULL, value2Ptr, &i2) != TCL_OK) {
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(value2Ptr),
		    (value2Ptr->typePtr? value2Ptr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, value2Ptr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}

	if (*pc == INST_LOR) {
	    iResult = (i1 || i2);
	} else {
	    iResult = (i1 && i2);
	}
	objResultPtr = TCONST(iResult);
	TRACE(("%.20s %.20s => %d\n", O2S(valuePtr),O2S(value2Ptr),iResult));
	NEXT_INST_F(1, 2, 1);
    }
    break;

    /*
     * -----------------------------------------------------------------
     *	   Start of general introspector instructions.
     */

    case INST_NS_CURRENT: {
	Namespace *currNsPtr = (Namespace *) TclGetCurrentNamespace(interp);

	if (currNsPtr == (Namespace *) TclGetGlobalNamespace(interp)) {
	    TclNewLiteralStringObj(objResultPtr, "::");
	} else {
	    TclNewStringObj(objResultPtr, currNsPtr->fullName,
		    strlen(currNsPtr->fullName));
	}
	TRACE_WITH_OBJ(("=> "), objResultPtr);
	NEXT_INST_F(1, 0, 1);
    }
    break;
    case INST_COROUTINE_NAME: {
	CoroutineData *corPtr = iPtr->execEnvPtr->corPtr;

	TclNewObj(objResultPtr);
	if (corPtr && !(corPtr->cmdPtr->flags & CMD_IS_DELETED)) {
	    Tcl_GetCommandFullName(interp, (Tcl_Command) corPtr->cmdPtr,
		    objResultPtr);
	}
	TRACE_WITH_OBJ(("=> "), objResultPtr);
	NEXT_INST_F(1, 0, 1);
    }
    break;
    case INST_INFO_LEVEL_NUM:
	TclNewIntObj(objResultPtr, iPtr->varFramePtr->level);
	TRACE_WITH_OBJ(("=> "), objResultPtr);
	NEXT_INST_F(1, 0, 1);
    break;
    case INST_INFO_LEVEL_ARGS: {
	int level;
	CallFrame *framePtr = iPtr->varFramePtr;
	CallFrame *rootFramePtr = iPtr->rootFramePtr;

	TRACE(("\"%.30s\" => ", O2S(OBJ_AT_TOS)));
	if (TclGetIntFromObj(interp, OBJ_AT_TOS, &level) != TCL_OK) {
	    TRACE_ERROR(interp);
	    goto gotError;
	}
	if (level <= 0) {
	    level += framePtr->level;
	}
	for (; (framePtr->level!=level) && (framePtr!=rootFramePtr) ;
		framePtr = framePtr->callerVarPtr) {
	    /* Empty loop body */
	}
	if (framePtr == rootFramePtr) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "bad level \"%s\"", TclGetString(OBJ_AT_TOS)));
	    TRACE_ERROR(interp);
	    DECACHE_STACK_INFO();
	    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "STACK_LEVEL",
		    TclGetString(OBJ_AT_TOS), (char *)NULL);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	objResultPtr = Tcl_NewListObj(framePtr->objc, framePtr->objv);
	TRACE_APPEND(("%.30s\n", O2S(objResultPtr)));
	NEXT_INST_F(1, 1, 1);
    }
    {
	Tcl_Command cmd, origCmd;

    case INST_RESOLVE_COMMAND:
	cmd = Tcl_GetCommandFromObj(interp, OBJ_AT_TOS);
	TclNewObj(objResultPtr);
	if (cmd != NULL) {
	    Tcl_GetCommandFullName(interp, cmd, objResultPtr);
	}
	TRACE_WITH_OBJ(("\"%.20s\" => ", O2S(OBJ_AT_TOS)), objResultPtr);
	NEXT_INST_F(1, 1, 1);

    case INST_ORIGIN_COMMAND:
	TRACE(("\"%.30s\" => ", O2S(OBJ_AT_TOS)));
	cmd = Tcl_GetCommandFromObj(interp, OBJ_AT_TOS);
	if (cmd == NULL) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "invalid command name \"%s\"", TclGetString(OBJ_AT_TOS)));
	    DECACHE_STACK_INFO();
	    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "COMMAND",
		    TclGetString(OBJ_AT_TOS), (char *)NULL);
	    CACHE_STACK_INFO();
	    TRACE_APPEND(("ERROR: not command\n"));
	    goto gotError;
	}
	origCmd = TclGetOriginalCommand(cmd);
	if (origCmd == NULL) {
	    origCmd = cmd;
	}
	TclNewObj(objResultPtr);
	Tcl_GetCommandFullName(interp, origCmd, objResultPtr);
	TRACE_APPEND(("\"%.30s\"", O2S(OBJ_AT_TOS)));
	NEXT_INST_F(1, 1, 1);
    }

    /*
     * -----------------------------------------------------------------
     *	   Start of TclOO support instructions.
     */

    {
	Object *oPtr;
	CallFrame *framePtr;
	CallContext *contextPtr;
