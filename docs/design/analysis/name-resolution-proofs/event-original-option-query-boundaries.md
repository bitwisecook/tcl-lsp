# naming.event.original-option-query-boundaries

Kind: `native-observation`

## Problem statement

Reparsing rendered after/update selectors loses the provider-specific abbreviation, identifier, query and result boundaries.

## Question

What are the exact captured after cancellation/info, abbreviation and update selector outcomes on the six selected providers?

## Conclusion

The retained cancel controls leave done at 0; script-info returns set done 1. Zero-timer kind is timer on C and idle on Jim. C accepts after in, rejects ambiguous after i with its composed diagnostic, and accepts update i; Jim requires exact after selector matches and accepts abbreviated update. Missing-event code/result/options remain exact captured rows, with C84/Jim options API unavailable/unqueried explicit.

## Scope

Eight exact source controls per provider in v106. Current selected table/identifier/query boundaries, not generic numeric conversion, index-cache authority, all raw binary selector cases or event-lifetime proof. C84 Tcl_GetReturnOptions is absent; Jim C return-options API is not queried; corresponding rows remain explicitly not-tested. Raw receipts and prior attempts remain immutable.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest. Version: tcl8.4 8.4.20.

v106/CANCEL_COUNTED: CANCEL_COUNTED: code 0, bytes b'0' v106/CANCEL_CONCAT: CANCEL_CONCAT: code 0, bytes b'0' v106/INFO_SCRIPT: INFO_SCRIPT: code 0, bytes b'set done 1' v106/INFO_KIND: INFO_KIND: code 0, bytes b'timer' v106/INFO_MISSING: INFO_MISSING: code 1, bytes b'event "after#99999999" doesn\'t exist' v106/AFTER_ABBREVIATION: AFTER_ABBREVIATION: code 0, bytes b'' v106/AFTER_AMBIGUOUS: AFTER_AMBIGUOUS: code 1, bytes b'bad argument "i": must be cancel, idle, info, or a number' v106/UPDATE_ABBREVIATION: UPDATE_ABBREVIATION: code 0, bytes b''

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

v106/CANCEL_COUNTED: CANCEL_COUNTED: code 0, bytes b'0' v106/CANCEL_CONCAT: CANCEL_CONCAT: code 0, bytes b'0' v106/INFO_SCRIPT: INFO_SCRIPT: code 0, bytes b'set done 1' v106/INFO_KIND: INFO_KIND: code 0, bytes b'timer' v106/INFO_MISSING: INFO_MISSING: code 1, bytes b'event "after#99999999" doesn\'t exist' v106/AFTER_ABBREVIATION: AFTER_ABBREVIATION: code 0, bytes b'' v106/AFTER_AMBIGUOUS: AFTER_AMBIGUOUS: code 1, bytes b'bad argument "i": must be cancel, idle, info, or an integer' v106/UPDATE_ABBREVIATION: UPDATE_ABBREVIATION: code 0, bytes b''

### jim

Status: `observed`. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest. Version: jim 0.84-9-g5bac7c9.

v106/CANCEL_COUNTED: CANCEL_COUNTED: code 0, bytes b'0' v106/CANCEL_CONCAT: CANCEL_CONCAT: code 0, bytes b'0' v106/INFO_SCRIPT: INFO_SCRIPT: code 0, bytes b'set done 1' v106/INFO_KIND: INFO_KIND: code 0, bytes b'idle' v106/INFO_MISSING: INFO_MISSING: code 1, bytes b'event "after#99999999" doesn\'t exist' v106/AFTER_ABBREVIATION: AFTER_ABBREVIATION: code 1, bytes b'bad argument "in": must be cancel, idle, or info' v106/AFTER_AMBIGUOUS: AFTER_AMBIGUOUS: code 1, bytes b'bad argument "i": must be cancel, idle, or info' v106/UPDATE_ABBREVIATION: UPDATE_ABBREVIATION: code 0, bytes b''

### bigip

Status: `not-tested`. Channel: not exercised.

No BIG-IP provider was executed or inspected for this Event question.

## Source inspection

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclTimer.c`, function `Tcl_AfterObjCmd`, lines 735–933. Retained reference `e7ca34f2d6fd9344d4610`.

```text
Tcl_AfterObjCmd(clientData, interp, objc, objv)
    ClientData clientData;	/* Unused */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    int ms;
    AfterInfo *afterPtr;
    AfterAssocData *assocPtr;
    int length;
    char *argString;
    int index;
    char buf[16 + TCL_INTEGER_SPACE];
    static CONST char *afterSubCmds[] = {
	"cancel", "idle", "info", (char *) NULL
    };
    enum afterSubCmds {AFTER_CANCEL, AFTER_IDLE, AFTER_INFO};
    ThreadSpecificData *tsdPtr = InitTimer();

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "option ?arg arg ...?");
	return TCL_ERROR;
    }

    /*
     * Create the "after" information associated for this interpreter,
     * if it doesn't already exist.  
     */

    assocPtr = Tcl_GetAssocData( interp, "tclAfter", NULL );
    if (assocPtr == NULL) {
	assocPtr = (AfterAssocData *) ckalloc(sizeof(AfterAssocData));
	assocPtr->interp = interp;
	assocPtr->firstAfterPtr = NULL;
	Tcl_SetAssocData(interp, "tclAfter", AfterCleanupProc,
		(ClientData) assocPtr);
    }

    /*
     * First lets see if the command was passed a number as the first argument.
     */

    if (objv[1]->typePtr == &tclIntType) {
	ms = (int) objv[1]->internalRep.longValue;
	goto processInteger;
    }
    argString = Tcl_GetStringFromObj(objv[1], &length);
    if (argString[0] == '+' || argString[0] == '-'
	|| isdigit(UCHAR(argString[0]))) {	/* INTL: digit */
	if (Tcl_GetIntFromObj(interp, objv[1], &ms) != TCL_OK) {
	    return TCL_ERROR;
	}
processInteger:
	if (ms < 0) {
	    ms = 0;
	}
	if (objc == 2) {
	    Tcl_Sleep(ms);
	    return TCL_OK;
	}
	afterPtr = (AfterInfo *) ckalloc((unsigned) (sizeof(AfterInfo)));
	afterPtr->assocPtr = assocPtr;
	if (objc == 3) {
	    afterPtr->commandPtr = objv[2];
	} else {
 	    afterPtr->commandPtr = Tcl_ConcatObj(objc-2, objv+2);
	}
	Tcl_IncrRefCount(afterPtr->commandPtr);
	/*
	 * The variable below is used to generate unique identifiers for
	 * after commands.  This id can wrap around, which can potentially
	 * cause problems.  However, there are not likely to be problems
	 * in practice, because after commands can only be requested to
	 * about a month in the future, and wrap-around is unlikely to
	 * occur in less than about 1-10 years.  Thus it's unlikely that
	 * any old ids will still be around when wrap-around occurs.
	 */
	afterPtr->id = tsdPtr->afterId;
	tsdPtr->afterId += 1;
	afterPtr->token = Tcl_CreateTimerHandler(ms, AfterProc,
		(ClientData) afterPtr);
	afterPtr->nextPtr = assocPtr->firstAfterPtr;
	assocPtr->firstAfterPtr = afterPtr;
	sprintf(buf, "after#%d", afterPtr->id);
	Tcl_AppendResult(interp, buf, (char *) NULL);
	return TCL_OK;
    }

    /*
     * If it's not a number it must be a subcommand.
     */

    if (Tcl_GetIndexFromObj(NULL, objv[1], afterSubCmds, "argument",
            0, &index) != TCL_OK) {
	Tcl_AppendResult(interp, "bad argument \"", argString,
		"\": must be cancel, idle, info, or a number",
		(char *) NULL);
	return TCL_ERROR;
    }
    switch ((enum afterSubCmds) index) {
        case AFTER_CANCEL: {
	    Tcl_Obj *commandPtr;
	    char *command, *tempCommand;
	    int tempLength;

	    if (objc < 3) {
		Tcl_WrongNumArgs(interp, 2, objv, "id|command");
		return TCL_ERROR;
	    }
	    if (objc == 3) {
		commandPtr = objv[2];
	    } else {
		commandPtr = Tcl_ConcatObj(objc-2, objv+2);;
	    }
	    command = Tcl_GetStringFromObj(commandPtr, &length);
	    for (afterPtr = assocPtr->firstAfterPtr;  afterPtr != NULL;
		    afterPtr = afterPtr->nextPtr) {
		tempCommand = Tcl_GetStringFromObj(afterPtr->commandPtr,
			&tempLength);
		if ((length == tempLength)
		        && (memcmp((void*) command, (void*) tempCommand,
			        (unsigned) length) == 0)) {
		    break;
		}
	    }
	    if (afterPtr == NULL) {
		afterPtr = GetAfterEvent(assocPtr, commandPtr);
	    }
	    if (objc != 3) {
		Tcl_DecrRefCount(commandPtr);
	    }
	    if (afterPtr != NULL) {
		if (afterPtr->token != NULL) {
		    Tcl_DeleteTimerHandler(afterPtr->token);
		} else {
		    Tcl_CancelIdleCall(AfterProc, (ClientData) afterPtr);
		}
		FreeAfterPtr(afterPtr);
	    }
	    break;
	}
	case AFTER_IDLE:
	    if (objc < 3) {
		Tcl_WrongNumArgs(interp, 2, objv, "script script ...");
		return TCL_ERROR;
	    }
	    afterPtr = (AfterInfo *) ckalloc((unsigned) (sizeof(AfterInfo)));
	    afterPtr->assocPtr = assocPtr;
	    if (objc == 3) {
 		afterPtr->commandPtr = objv[2];
	    } else {
		afterPtr->commandPtr = Tcl_ConcatObj(objc-2, objv+2);
	    }
	    Tcl_IncrRefCount(afterPtr->commandPtr);
	    afterPtr->id = tsdPtr->afterId;
	    tsdPtr->afterId += 1;
	    afterPtr->token = NULL;
	    afterPtr->nextPtr = assocPtr->firstAfterPtr;
	    assocPtr->firstAfterPtr = afterPtr;
	    Tcl_DoWhenIdle(AfterProc, (ClientData) afterPtr);
	    sprintf(buf, "after#%d", afterPtr->id);
	    Tcl_AppendResult(interp, buf, (char *) NULL);
	    break;
	case AFTER_INFO: {
	    Tcl_Obj *resultListPtr;

	    if (objc == 2) {
		for (afterPtr = assocPtr->firstAfterPtr; afterPtr != NULL;
		     afterPtr = afterPtr->nextPtr) {
		    if (assocPtr->interp == interp) {
			sprintf(buf, "after#%d", afterPtr->id);
			Tcl_AppendElement(interp, buf);
		    }
		}
		return TCL_OK;
	    }
	    if (objc != 3) {
		Tcl_WrongNumArgs(interp, 2, objv, "?id?");
		return TCL_ERROR;
	    }
	    afterPtr = GetAfterEvent(assocPtr, objv[2]);
	    if (afterPtr == NULL) {
		Tcl_AppendResult(interp, "event \"", Tcl_GetString(objv[2]),
			"\" doesn't exist", (char *) NULL);
		return TCL_ERROR;
	    }
	    resultListPtr = Tcl_GetObjResult(interp);
 	    Tcl_ListObjAppendElement(interp, resultListPtr, afterPtr->commandPtr);
 	    Tcl_ListObjAppendElement(interp, resultListPtr, Tcl_NewStringObj(
 		(afterPtr->token == NULL) ? "idle" : "timer", -1));
	    Tcl_SetObjResult(interp, resultListPtr);
	    break;
	}
	default: {
	    panic("Tcl_AfterObjCmd: bad subcommand index to afterSubCmds");
	}
    }
    return TCL_OK;
}
```

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclTimer.c`, function `GetAfterEvent`, lines 955–982. Retained reference `e7ca34f2d6fd9344d4610`.

```text
GetAfterEvent(assocPtr, commandPtr)
    AfterAssocData *assocPtr;	/* Points to "after"-related information for
				 * this interpreter. */
    Tcl_Obj *commandPtr;
{
    char *cmdString;		/* Textual identifier for after event, such
				 * as "after#6". */
    AfterInfo *afterPtr;
    int id;
    char *end;

    cmdString = Tcl_GetString(commandPtr);
    if (strncmp(cmdString, "after#", 6) != 0) {
	return NULL;
    }
    cmdString += 6;
    id = strtoul(cmdString, &end, 10);
    if ((end == cmdString) || (*end != 0)) {
	return NULL;
    }
    for (afterPtr = assocPtr->firstAfterPtr; afterPtr != NULL;
	    afterPtr = afterPtr->nextPtr) {
	if (afterPtr->id == id) {
	    return afterPtr;
	}
    }
    return NULL;
}
```

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEvent.c`, function `Tcl_UpdateObjCmd`, lines 1252–1295. Retained reference `e98d4bc726037374525cd`.

```text
Tcl_UpdateObjCmd(clientData, interp, objc, objv)
    ClientData clientData;	/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    int optionIndex;
    int flags = 0;		/* Initialized to avoid compiler warning. */
    static CONST char *updateOptions[] = {"idletasks", (char *) NULL};
    enum updateOptions {REGEXP_IDLETASKS};

    if (objc == 1) {
	flags = TCL_ALL_EVENTS|TCL_DONT_WAIT;
    } else if (objc == 2) {
	if (Tcl_GetIndexFromObj(interp, objv[1], updateOptions,
		"option", 0, &optionIndex) != TCL_OK) {
	    return TCL_ERROR;
	}
	switch ((enum updateOptions) optionIndex) {
	    case REGEXP_IDLETASKS: {
		flags = TCL_WINDOW_EVENTS|TCL_IDLE_EVENTS|TCL_DONT_WAIT;
		break;
	    }
	    default: {
		panic("Tcl_UpdateObjCmd: bad option index to UpdateOptions");
	    }
	}
    } else {
        Tcl_WrongNumArgs(interp, 1, objv, "?idletasks?");
	return TCL_ERROR;
    }
    
    while (Tcl_DoOneEvent(flags) != 0) {
	/* Empty loop body */
    }

    /*
     * Must clear the interpreter's result because event handlers could
     * have executed commands.
     */

    Tcl_ResetResult(interp);
    return TCL_OK;
}
```

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclTimer.c`, function `Tcl_AfterObjCmd`, lines 766–969. Retained reference `eb83569215d92fa37da10`.

```text
Tcl_AfterObjCmd(
    ClientData clientData,	/* Unused */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *CONST objv[])	/* Argument objects. */
{
    Tcl_WideInt ms;		/* Number of milliseconds to wait */
    Tcl_Time wakeup;
    AfterInfo *afterPtr;
    AfterAssocData *assocPtr;
    int length;
    int index;
    char buf[16 + TCL_INTEGER_SPACE];
    static CONST char *afterSubCmds[] = {
	"cancel", "idle", "info", NULL
    };
    enum afterSubCmds {AFTER_CANCEL, AFTER_IDLE, AFTER_INFO};
    ThreadSpecificData *tsdPtr = InitTimer();

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "option ?arg arg ...?");
	return TCL_ERROR;
    }

    /*
     * Create the "after" information associated for this interpreter, if it
     * doesn't already exist.
     */

    assocPtr = Tcl_GetAssocData(interp, "tclAfter", NULL);
    if (assocPtr == NULL) {
	assocPtr = (AfterAssocData *) ckalloc(sizeof(AfterAssocData));
	assocPtr->interp = interp;
	assocPtr->firstAfterPtr = NULL;
	Tcl_SetAssocData(interp, "tclAfter", AfterCleanupProc,
		(ClientData) assocPtr);
    }

    /*
     * First lets see if the command was passed a number as the first argument.
     */

    if (objv[1]->typePtr == &tclIntType
#ifndef NO_WIDE_TYPE
	|| objv[1]->typePtr == &tclWideIntType
#endif
	|| objv[1]->typePtr == &tclBignumType
	|| ( Tcl_GetIndexFromObj(NULL, objv[1], afterSubCmds, "", 0, 
				 &index) != TCL_OK )) {
	index = -1;
	if (Tcl_GetWideIntFromObj(NULL, objv[1], &ms) != TCL_OK) {
	    Tcl_AppendResult(interp, "bad argument \"",
			     Tcl_GetString(objv[1]),
			     "\": must be cancel, idle, info, or an integer",
			     NULL);
	    return TCL_ERROR;
	}
    }

    /* 
     * At this point, either index = -1 and ms contains the number of ms
     * to wait, or else index is the index of a subcommand.
     */

    switch (index) {
    case -1: {
	if (ms < 0) {
	    ms = 0;
	}
	if (objc == 2) {
	    return AfterDelay(interp, ms);
	}
	afterPtr = (AfterInfo *) ckalloc((unsigned) (sizeof(AfterInfo)));
	afterPtr->assocPtr = assocPtr;
	if (objc == 3) {
	    afterPtr->commandPtr = objv[2];
	} else {
 	    afterPtr->commandPtr = Tcl_ConcatObj(objc-2, objv+2);
	}
	Tcl_IncrRefCount(afterPtr->commandPtr);

	/*
	 * The variable below is used to generate unique identifiers for after
	 * commands. This id can wrap around, which can potentially cause
	 * problems. However, there are not likely to be problems in practice,
	 * because after commands can only be requested to about a month in
	 * the future, and wrap-around is unlikely to occur in less than about
	 * 1-10 years. Thus it's unlikely that any old ids will still be
	 * around when wrap-around occurs.
	 */

	afterPtr->id = tsdPtr->afterId;
	tsdPtr->afterId += 1;
	Tcl_GetTime(&wakeup);
	wakeup.sec += (long)(ms / 1000);
	wakeup.usec += ((long)(ms % 1000)) * 1000;
	if (wakeup.usec > 1000000) {
	    wakeup.sec++;
	    wakeup.usec -= 1000000;
	}
	afterPtr->token = TclCreateAbsoluteTimerHandler(&wakeup, AfterProc,
							(ClientData) afterPtr);
	afterPtr->nextPtr = assocPtr->firstAfterPtr;
	assocPtr->firstAfterPtr = afterPtr;
	Tcl_SetObjResult(interp, Tcl_ObjPrintf("after#%d", afterPtr->id));
	return TCL_OK;
    }
    case AFTER_CANCEL: {
	Tcl_Obj *commandPtr;
	char *command, *tempCommand;
	int tempLength;

	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "id|command");
	    return TCL_ERROR;
	}
	if (objc == 3) {
	    commandPtr = objv[2];
	} else {
	    commandPtr = Tcl_ConcatObj(objc-2, objv+2);;
	}
	command = Tcl_GetStringFromObj(commandPtr, &length);
	for (afterPtr = assocPtr->firstAfterPtr;  afterPtr != NULL;
		afterPtr = afterPtr->nextPtr) {
	    tempCommand = Tcl_GetStringFromObj(afterPtr->commandPtr,
		    &tempLength);
	    if ((length == tempLength)
		    && (memcmp((void*) command, (void*) tempCommand,
			    (unsigned) length) == 0)) {
		break;
	    }
	}
	if (afterPtr == NULL) {
	    afterPtr = GetAfterEvent(assocPtr, commandPtr);
	}
	if (objc != 3) {
	    Tcl_DecrRefCount(commandPtr);
	}
	if (afterPtr != NULL) {
	    if (afterPtr->token != NULL) {
		Tcl_DeleteTimerHandler(afterPtr->token);
	    } else {
		Tcl_CancelIdleCall(AfterProc, (ClientData) afterPtr);
	    }
	    FreeAfterPtr(afterPtr);
	}
	break;
    }
    case AFTER_IDLE:
	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "script script ...");
	    return TCL_ERROR;
	}
	afterPtr = (AfterInfo *) ckalloc((unsigned) (sizeof(AfterInfo)));
	afterPtr->assocPtr = assocPtr;
	if (objc == 3) {
	    afterPtr->commandPtr = objv[2];
	} else {
	    afterPtr->commandPtr = Tcl_ConcatObj(objc-2, objv+2);
	}
	Tcl_IncrRefCount(afterPtr->commandPtr);
	afterPtr->id = tsdPtr->afterId;
	tsdPtr->afterId += 1;
	afterPtr->token = NULL;
	afterPtr->nextPtr = assocPtr->firstAfterPtr;
	assocPtr->firstAfterPtr = afterPtr;
	Tcl_DoWhenIdle(AfterProc, (ClientData) afterPtr);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf("after#%d", afterPtr->id));
	break;
    case AFTER_INFO: {
	Tcl_Obj *resultListPtr;

	if (objc == 2) {
	    for (afterPtr = assocPtr->firstAfterPtr; afterPtr != NULL;
		    afterPtr = afterPtr->nextPtr) {
		if (assocPtr->interp == interp) {
		    sprintf(buf, "after#%d", afterPtr->id);
		    Tcl_AppendElement(interp, buf);
		}
	    }
	    return TCL_OK;
	}
	if (objc != 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "?id?");
	    return TCL_ERROR;
	}
	afterPtr = GetAfterEvent(assocPtr, objv[2]);
	if (afterPtr == NULL) {
	    Tcl_AppendResult(interp, "event \"", TclGetString(objv[2]),
		    "\" doesn't exist", NULL);
	    return TCL_ERROR;
	}
	resultListPtr = Tcl_NewObj();
	Tcl_ListObjAppendElement(interp, resultListPtr, afterPtr->commandPtr);
	Tcl_ListObjAppendElement(interp, resultListPtr, Tcl_NewStringObj(
 		(afterPtr->token == NULL) ? "idle" : "timer", -1));
	Tcl_SetObjResult(interp, resultListPtr);
	break;
    }
    default:
	Tcl_Panic("Tcl_AfterObjCmd: bad subcommand index to afterSubCmds");
    }
    return TCL_OK;
}
```

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclTimer.c`, function `GetAfterEvent`, lines 1065–1092. Retained reference `eb83569215d92fa37da10`.

```text
GetAfterEvent(
    AfterAssocData *assocPtr,	/* Points to "after"-related information for
				 * this interpreter. */
    Tcl_Obj *commandPtr)
{
    char *cmdString;		/* Textual identifier for after event, such as
				 * "after#6". */
    AfterInfo *afterPtr;
    int id;
    char *end;

    cmdString = TclGetString(commandPtr);
    if (strncmp(cmdString, "after#", 6) != 0) {
	return NULL;
    }
    cmdString += 6;
    id = strtoul(cmdString, &end, 10);
    if ((end == cmdString) || (*end != 0)) {
	return NULL;
    }
    for (afterPtr = assocPtr->firstAfterPtr; afterPtr != NULL;
	    afterPtr = afterPtr->nextPtr) {
	if (afterPtr->id == id) {
	    return afterPtr;
	}
    }
    return NULL;
}
```

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEvent.c`, function `Tcl_UpdateObjCmd`, lines 1404–1449. Retained reference `ebcd14ee056c41f642dc1`.

```text
Tcl_UpdateObjCmd(
    ClientData clientData,	/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *CONST objv[])	/* Argument objects. */
{
    int optionIndex;
    int flags = 0;		/* Initialized to avoid compiler warning. */
    static CONST char *updateOptions[] = {"idletasks", NULL};
    enum updateOptions {REGEXP_IDLETASKS};

    if (objc == 1) {
	flags = TCL_ALL_EVENTS|TCL_DONT_WAIT;
    } else if (objc == 2) {
	if (Tcl_GetIndexFromObj(interp, objv[1], updateOptions,
		"option", 0, &optionIndex) != TCL_OK) {
	    return TCL_ERROR;
	}
	switch ((enum updateOptions) optionIndex) {
	case REGEXP_IDLETASKS:
	    flags = TCL_WINDOW_EVENTS|TCL_IDLE_EVENTS|TCL_DONT_WAIT;
	    break;
	default:
	    Tcl_Panic("Tcl_UpdateObjCmd: bad option index to UpdateOptions");
	}
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "?idletasks?");
	return TCL_ERROR;
    }

    while (Tcl_DoOneEvent(flags) != 0) {
	if (Tcl_LimitExceeded(interp)) {
	    Tcl_ResetResult(interp);
	    Tcl_AppendResult(interp, "limit exceeded", NULL);
	    return TCL_ERROR;
	}
    }

    /*
     * Must clear the interpreter's result because event handlers could have
     * executed commands.
     */

    Tcl_ResetResult(interp);
    return TCL_OK;
}
```

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclTimer.c`, function `Tcl_AfterObjCmd`, lines 781–992. Retained reference `e5cad7b4101332ff96f7c`.

```text
Tcl_AfterObjCmd(
    ClientData clientData,	/* Unused */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_WideInt ms = 0;		/* Number of milliseconds to wait */
    Tcl_Time wakeup;
    AfterInfo *afterPtr;
    AfterAssocData *assocPtr;
    int length;
    int index;
    static const char *const afterSubCmds[] = {
	"cancel", "idle", "info", NULL
    };
    enum afterSubCmds {AFTER_CANCEL, AFTER_IDLE, AFTER_INFO};
    ThreadSpecificData *tsdPtr = InitTimer();

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "option ?arg ...?");
	return TCL_ERROR;
    }

    /*
     * Create the "after" information associated for this interpreter, if it
     * doesn't already exist.
     */

    assocPtr = Tcl_GetAssocData(interp, "tclAfter", NULL);
    if (assocPtr == NULL) {
	assocPtr = ckalloc(sizeof(AfterAssocData));
	assocPtr->interp = interp;
	assocPtr->firstAfterPtr = NULL;
	Tcl_SetAssocData(interp, "tclAfter", AfterCleanupProc, assocPtr);
    }

    /*
     * First lets see if the command was passed a number as the first argument.
     */

    if (objv[1]->typePtr == &tclIntType
#ifndef TCL_WIDE_INT_IS_LONG
	    || objv[1]->typePtr == &tclWideIntType
#endif
	    || objv[1]->typePtr == &tclBignumType
	    || (Tcl_GetIndexFromObj(NULL, objv[1], afterSubCmds, "", 0,
		    &index) != TCL_OK)) {
	index = -1;
	if (Tcl_GetWideIntFromObj(NULL, objv[1], &ms) != TCL_OK) {
            const char *arg = TclGetString(objv[1]);

	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
                    "bad argument \"%s\": must be"
                    " cancel, idle, info, or an integer", arg));
            Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "INDEX", "argument",
                    arg, (char *)NULL);
	    return TCL_ERROR;
	}
    }

    /*
     * At this point, either index = -1 and ms contains the number of ms
     * to wait, or else index is the index of a subcommand.
     */

    switch (index) {
    case -1: {
	if (ms < 0) {
	    ms = 0;
	}
	if (objc == 2) {
	    return AfterDelay(interp, ms);
	}
	afterPtr = ckalloc(sizeof(AfterInfo));
	afterPtr->assocPtr = assocPtr;
	if (objc == 3) {
	    afterPtr->commandPtr = objv[2];
	} else {
	    afterPtr->commandPtr = Tcl_ConcatObj(objc-2, objv+2);
	}
	Tcl_IncrRefCount(afterPtr->commandPtr);

	/*
	 * The variable below is used to generate unique identifiers for after
	 * commands. This id can wrap around, which can potentially cause
	 * problems. However, there are not likely to be problems in practice,
	 * because after commands can only be requested to about a month in
	 * the future, and wrap-around is unlikely to occur in less than about
	 * 1-10 years. Thus it's unlikely that any old ids will still be
	 * around when wrap-around occurs.
	 */

	afterPtr->id = tsdPtr->afterId;
	tsdPtr->afterId += 1;
	Tcl_GetTime(&wakeup);
	wakeup.sec += (long)(ms / 1000);
	wakeup.usec += ((long)(ms % 1000)) * 1000;
	if (wakeup.usec > 1000000) {
	    wakeup.sec++;
	    wakeup.usec -= 1000000;
	}
	afterPtr->token = TclCreateAbsoluteTimerHandler(&wakeup,
		AfterProc, afterPtr);
	afterPtr->nextPtr = assocPtr->firstAfterPtr;
	assocPtr->firstAfterPtr = afterPtr;
	Tcl_SetObjResult(interp, Tcl_ObjPrintf("after#%d", afterPtr->id));
	return TCL_OK;
    }
    case AFTER_CANCEL: {
	Tcl_Obj *commandPtr;
	const char *command, *tempCommand;
	int tempLength;

	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "id|command");
	    return TCL_ERROR;
	}
	if (objc == 3) {
	    commandPtr = objv[2];
	} else {
	    commandPtr = Tcl_ConcatObj(objc-2, objv+2);
	}
	command = TclGetStringFromObj(commandPtr, &length);
	for (afterPtr = assocPtr->firstAfterPtr;  afterPtr != NULL;
		afterPtr = afterPtr->nextPtr) {
	    tempCommand = TclGetStringFromObj(afterPtr->commandPtr,
		    &tempLength);
	    if ((length == tempLength)
		    && !memcmp(command, tempCommand, length)) {
		break;
	    }
	}
	if (afterPtr == NULL) {
	    afterPtr = GetAfterEvent(assocPtr, commandPtr);
	}
	if (objc != 3) {
	    Tcl_DecrRefCount(commandPtr);
	}
	if (afterPtr != NULL) {
	    if (afterPtr->token != NULL) {
		Tcl_DeleteTimerHandler(afterPtr->token);
	    } else {
		Tcl_CancelIdleCall(AfterProc, afterPtr);
	    }
	    FreeAfterPtr(afterPtr);
	}
	break;
    }
    case AFTER_IDLE:
	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "script ?script ...?");
	    return TCL_ERROR;
	}
	afterPtr = ckalloc(sizeof(AfterInfo));
	afterPtr->assocPtr = assocPtr;
	if (objc == 3) {
	    afterPtr->commandPtr = objv[2];
	} else {
	    afterPtr->commandPtr = Tcl_ConcatObj(objc-2, objv+2);
	}
	Tcl_IncrRefCount(afterPtr->commandPtr);
	afterPtr->id = tsdPtr->afterId;
	tsdPtr->afterId += 1;
	afterPtr->token = NULL;
	afterPtr->nextPtr = assocPtr->firstAfterPtr;
	assocPtr->firstAfterPtr = afterPtr;
	Tcl_DoWhenIdle(AfterProc, afterPtr);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf("after#%d", afterPtr->id));
	break;
    case AFTER_INFO:
	if (objc == 2) {
	    Tcl_Obj *resultObj;

	    TclNewObj(resultObj);
	    for (afterPtr = assocPtr->firstAfterPtr; afterPtr != NULL;
		    afterPtr = afterPtr->nextPtr) {
		if (assocPtr->interp == interp) {
		    Tcl_ListObjAppendElement(NULL, resultObj, Tcl_ObjPrintf(
			    "after#%d", afterPtr->id));
		}
	    }
            Tcl_SetObjResult(interp, resultObj);
	    return TCL_OK;
	}
	if (objc != 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "?id?");
	    return TCL_ERROR;
	}
	afterPtr = GetAfterEvent(assocPtr, objv[2]);
	if (afterPtr == NULL) {
            const char *eventStr = TclGetString(objv[2]);

	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
                    "event \"%s\" doesn't exist", eventStr));
            Tcl_SetErrorCode(interp, "TCL","LOOKUP","EVENT", eventStr, (char *)NULL);
	    return TCL_ERROR;
	} else {
	    Tcl_Obj *resultListPtr;

	    TclNewObj(resultListPtr);
	    Tcl_ListObjAppendElement(interp, resultListPtr,
		    afterPtr->commandPtr);
	    Tcl_ListObjAppendElement(interp, resultListPtr, Tcl_NewStringObj(
		    (afterPtr->token == NULL) ? "idle" : "timer", -1));
            Tcl_SetObjResult(interp, resultListPtr);
	}
	break;
    default:
	Tcl_Panic("Tcl_AfterObjCmd: bad subcommand index to afterSubCmds");
    }
    return TCL_OK;
}
```

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclTimer.c`, function `GetAfterEvent`, lines 1119–1146. Retained reference `e5cad7b4101332ff96f7c`.

```text
GetAfterEvent(
    AfterAssocData *assocPtr,	/* Points to "after"-related information for
				 * this interpreter. */
    Tcl_Obj *commandPtr)
{
    const char *cmdString;	/* Textual identifier for after event, such as
				 * "after#6". */
    AfterInfo *afterPtr;
    int id;
    char *end;

    cmdString = TclGetString(commandPtr);
    if (strncmp(cmdString, "after#", 6) != 0) {
	return NULL;
    }
    cmdString += 6;
    id = strtoul(cmdString, &end, 10);
    if ((end == cmdString) || (*end != 0)) {
	return NULL;
    }
    for (afterPtr = assocPtr->firstAfterPtr; afterPtr != NULL;
	    afterPtr = afterPtr->nextPtr) {
	if (afterPtr->id == id) {
	    return afterPtr;
	}
    }
    return NULL;
}
```

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEvent.c`, function `Tcl_UpdateObjCmd`, lines 1483–1531. Retained reference `e1799f8f23cd143629768`.

```text
Tcl_UpdateObjCmd(
    ClientData clientData,	/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int optionIndex;
    int flags = 0;		/* Initialized to avoid compiler warning. */
    static const char *const updateOptions[] = {"idletasks", NULL};
    enum updateOptionsEnum {OPT_IDLETASKS};

    if (objc == 1) {
	flags = TCL_ALL_EVENTS|TCL_DONT_WAIT;
    } else if (objc == 2) {
	if (Tcl_GetIndexFromObj(interp, objv[1], updateOptions,
		"option", 0, &optionIndex) != TCL_OK) {
	    return TCL_ERROR;
	}
	switch ((enum updateOptionsEnum) optionIndex) {
	case OPT_IDLETASKS:
	    flags = TCL_WINDOW_EVENTS|TCL_IDLE_EVENTS|TCL_DONT_WAIT;
	    break;
	default:
	    Tcl_Panic("Tcl_UpdateObjCmd: bad option index to UpdateOptions");
	}
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "?idletasks?");
	return TCL_ERROR;
    }

    while (Tcl_DoOneEvent(flags) != 0) {
	if (Tcl_Canceled(interp, TCL_LEAVE_ERR_MSG) == TCL_ERROR) {
	    return TCL_ERROR;
	}
	if (Tcl_LimitExceeded(interp)) {
	    Tcl_ResetResult(interp);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj("limit exceeded", -1));
	    return TCL_ERROR;
	}
    }

    /*
     * Must clear the interpreter's result because event handlers could have
     * executed commands.
     */

    Tcl_ResetResult(interp);
    return TCL_OK;
}
```

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclTimer.c`, function `Tcl_AfterObjCmd`, lines 776–981. Retained reference `e4d5b46a62af04d177edb`.

```text
Tcl_AfterObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_WideInt ms = 0;		/* Number of milliseconds to wait */
    Tcl_Time wakeup;
    AfterInfo *afterPtr;
    AfterAssocData *assocPtr;
    Tcl_Size length;
    int index = -1;
    static const char *const afterSubCmds[] = {
	"cancel", "idle", "info", NULL
    };
    enum afterSubCmdsEnum {AFTER_CANCEL, AFTER_IDLE, AFTER_INFO};
    ThreadSpecificData *tsdPtr = InitTimer();

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "option ?arg ...?");
	return TCL_ERROR;
    }

    /*
     * Create the "after" information associated for this interpreter, if it
     * doesn't already exist.
     */

    assocPtr = (AfterAssocData *)Tcl_GetAssocData(interp, "tclAfter", NULL);
    if (assocPtr == NULL) {
	assocPtr = (AfterAssocData *)Tcl_Alloc(sizeof(AfterAssocData));
	assocPtr->interp = interp;
	assocPtr->firstAfterPtr = NULL;
	Tcl_SetAssocData(interp, "tclAfter", AfterCleanupProc, assocPtr);
    }

    /*
     * First lets see if the command was passed a number as the first argument.
     */

    if (TclGetWideIntFromObj(NULL, objv[1], &ms) != TCL_OK) {
	if (Tcl_GetIndexFromObj(NULL, objv[1], afterSubCmds, "", 0, &index)
		!= TCL_OK) {
	    const char *arg = TclGetString(objv[1]);

	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "bad argument \"%s\": must be"
		    " cancel, idle, info, or an integer", arg));
	    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "INDEX", "argument",
		    arg, (char *)NULL);
	    return TCL_ERROR;
	}
    }

    /*
     * At this point, either index = -1 and ms contains the number of ms
     * to wait, or else index is the index of a subcommand.
     */

    switch (index) {
    case -1: {
	if (ms < 0) {
	    ms = 0;
	}
	if (objc == 2) {
	    return AfterDelay(interp, ms);
	}
	afterPtr = (AfterInfo *)Tcl_Alloc(sizeof(AfterInfo));
	afterPtr->assocPtr = assocPtr;
	if (objc == 3) {
	    afterPtr->commandPtr = objv[2];
	} else {
	    afterPtr->commandPtr = Tcl_ConcatObj(objc-2, objv+2);
	}
	Tcl_IncrRefCount(afterPtr->commandPtr);

	/*
	 * The variable below is used to generate unique identifiers for after
	 * commands. This id can wrap around, which can potentially cause
	 * problems. However, there are not likely to be problems in practice,
	 * because after commands can only be requested to about a month in
	 * the future, and wrap-around is unlikely to occur in less than about
	 * 1-10 years. Thus it's unlikely that any old ids will still be
	 * around when wrap-around occurs.
	 */

	afterPtr->id = tsdPtr->afterId;
	tsdPtr->afterId += 1;
	Tcl_GetTime(&wakeup);
	wakeup.sec += ms / 1000;
	wakeup.usec += ms % 1000 * 1000;
	if (wakeup.usec > 1000000) {
	    wakeup.sec++;
	    wakeup.usec -= 1000000;
	}
	afterPtr->token = TclCreateAbsoluteTimerHandler(&wakeup,
		AfterProc, afterPtr);
	afterPtr->nextPtr = assocPtr->firstAfterPtr;
	assocPtr->firstAfterPtr = afterPtr;
	Tcl_SetObjResult(interp, Tcl_ObjPrintf("after#%d", afterPtr->id));
	return TCL_OK;
    }
    case AFTER_CANCEL: {
	Tcl_Obj *commandPtr;
	const char *command, *tempCommand;
	Tcl_Size tempLength;

	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "id|command");
	    return TCL_ERROR;
	}
	if (objc == 3) {
	    commandPtr = objv[2];
	} else {
	    commandPtr = Tcl_ConcatObj(objc-2, objv+2);
	}
	command = TclGetStringFromObj(commandPtr, &length);
	for (afterPtr = assocPtr->firstAfterPtr;  afterPtr != NULL;
		afterPtr = afterPtr->nextPtr) {
	    tempCommand = TclGetStringFromObj(afterPtr->commandPtr,
		    &tempLength);
	    if ((length == tempLength)
		    && !memcmp(command, tempCommand, length)) {
		break;
	    }
	}
	if (afterPtr == NULL) {
	    afterPtr = GetAfterEvent(assocPtr, commandPtr);
	}
	if (objc != 3) {
	    Tcl_DecrRefCount(commandPtr);
	}
	if (afterPtr != NULL) {
	    if (afterPtr->token != NULL) {
		Tcl_DeleteTimerHandler(afterPtr->token);
	    } else {
		Tcl_CancelIdleCall(AfterProc, afterPtr);
	    }
	    FreeAfterPtr(afterPtr);
	}
	break;
    }
    case AFTER_IDLE:
	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "script ?script ...?");
	    return TCL_ERROR;
	}
	afterPtr = (AfterInfo *)Tcl_Alloc(sizeof(AfterInfo));
	afterPtr->assocPtr = assocPtr;
	if (objc == 3) {
	    afterPtr->commandPtr = objv[2];
	} else {
	    afterPtr->commandPtr = Tcl_ConcatObj(objc-2, objv+2);
	}
	Tcl_IncrRefCount(afterPtr->commandPtr);
	afterPtr->id = tsdPtr->afterId;
	tsdPtr->afterId += 1;
	afterPtr->token = NULL;
	afterPtr->nextPtr = assocPtr->firstAfterPtr;
	assocPtr->firstAfterPtr = afterPtr;
	Tcl_DoWhenIdle(AfterProc, afterPtr);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf("after#%d", afterPtr->id));
	break;
    case AFTER_INFO:
	if (objc == 2) {
	    Tcl_Obj *resultObj;

	    TclNewObj(resultObj);
	    for (afterPtr = assocPtr->firstAfterPtr; afterPtr != NULL;
		    afterPtr = afterPtr->nextPtr) {
		if (assocPtr->interp == interp) {
		    Tcl_ListObjAppendElement(NULL, resultObj, Tcl_ObjPrintf(
			    "after#%d", afterPtr->id));
		}
	    }
	    Tcl_SetObjResult(interp, resultObj);
	    return TCL_OK;
	}
	if (objc != 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "?id?");
	    return TCL_ERROR;
	}
	afterPtr = GetAfterEvent(assocPtr, objv[2]);
	if (afterPtr == NULL) {
	    const char *eventStr = TclGetString(objv[2]);

	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "event \"%s\" doesn't exist", eventStr));
	    Tcl_SetErrorCode(interp, "TCL","LOOKUP","EVENT", eventStr, (char *)NULL);
	    return TCL_ERROR;
	} else {
	    Tcl_Obj *resultListPtr;

	    TclNewObj(resultListPtr);
	    Tcl_ListObjAppendElement(interp, resultListPtr,
		    afterPtr->commandPtr);
	    Tcl_ListObjAppendElement(interp, resultListPtr, Tcl_NewStringObj(
		    (afterPtr->token == NULL) ? "idle" : "timer", -1));
	    Tcl_SetObjResult(interp, resultListPtr);
	}
	break;
    default:
	TCL_UNREACHABLE();
    }
    return TCL_OK;
}
```

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclTimer.c`, function `GetAfterEvent`, lines 1098–1125. Retained reference `e4d5b46a62af04d177edb`.

```text
GetAfterEvent(
    AfterAssocData *assocPtr,	/* Points to "after"-related information for
				 * this interpreter. */
    Tcl_Obj *commandPtr)
{
    const char *cmdString;	/* Textual identifier for after event, such as
				 * "after#6". */
    AfterInfo *afterPtr;
    int id;
    char *end;

    cmdString = TclGetString(commandPtr);
    if (strncmp(cmdString, "after#", 6) != 0) {
	return NULL;
    }
    cmdString += 6;
    id = (int)strtoul(cmdString, &end, 10);
    if ((end == cmdString) || (*end != 0)) {
	return NULL;
    }
    for (afterPtr = assocPtr->firstAfterPtr; afterPtr != NULL;
	    afterPtr = afterPtr->nextPtr) {
	if (afterPtr->id == id) {
	    return afterPtr;
	}
    }
    return NULL;
}
```

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEvent.c`, function `Tcl_UpdateObjCmd`, lines 1953–2000. Retained reference `e30bb9be41c1824d28e5d`.

```text
Tcl_UpdateObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int flags = 0;		/* Initialized to avoid compiler warning. */
    static const char *const updateOptions[] = {"idletasks", NULL};
    enum updateOptionsEnum {OPT_IDLETASKS} optionIndex;

    if (objc == 1) {
	flags = TCL_ALL_EVENTS|TCL_DONT_WAIT;
    } else if (objc == 2) {
	if (Tcl_GetIndexFromObj(interp, objv[1], updateOptions,
		"option", 0, &optionIndex) != TCL_OK) {
	    return TCL_ERROR;
	}
	switch (optionIndex) {
	case OPT_IDLETASKS:
	    flags = TCL_IDLE_EVENTS|TCL_DONT_WAIT;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "?idletasks?");
	return TCL_ERROR;
    }

    while (Tcl_DoOneEvent(flags) != 0) {
	if (Tcl_Canceled(interp, TCL_LEAVE_ERR_MSG) == TCL_ERROR) {
	    return TCL_ERROR;
	}
	if (Tcl_LimitExceeded(interp)) {
	    Tcl_ResetResult(interp);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj("limit exceeded", -1));
	    return TCL_ERROR;
	}
    }

    /*
     * Must clear the interpreter's result because event handlers could have
     * executed commands.
     */

    Tcl_ResetResult(interp);
    return TCL_OK;
}
```

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclTimer.c`, function `Tcl_AfterObjCmd`, lines 1018–1171. Retained reference `eaf331dbfda81fff6c4db`.

```text
Tcl_AfterObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_WideInt ms = 0;		/* Number of milliseconds to wait */
    int index = -1;
    static const char *const afterSubCmds[] = {
	"cancel", "idle", "info", NULL
    };
    enum afterSubCmdsEnum {AFTER_CANCEL, AFTER_IDLE, AFTER_INFO};
    Tcl_Obj *cmdObj;
    int res;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "option ?arg ...?");
	return TCL_ERROR;
    }

    /*
     * First lets see if the command was passed a number as the first argument.
     */

    if (TclGetWideIntFromObj(NULL, objv[1], &ms) != TCL_OK) {
	if (Tcl_GetIndexFromObj(NULL, objv[1], afterSubCmds, "", 0, &index)
		!= TCL_OK) {
	    const char *arg = TclGetString(objv[1]);

	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "bad argument \"%s\": must be"
		    " cancel, idle, info, or an integer", arg));
	    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "INDEX", "argument",
		    arg, (char *)NULL);
	    return TCL_ERROR;
	}
    }

    /*
     * At this point, either index = -1 and ms contains the number of ms
     * to wait, or else index is the index of a subcommand.
     */

    switch (index) {
    case -1: {
	AfterInfo *afterPtr;
	AfterAssocData *assocPtr;
	ThreadSpecificData *tsdPtr;
	long long microSeconds;

	if (ms < 0) {
	    ms = 0;
	}

	long long wakeupUS;
	wakeupUS = Tcl_GetMonotonicTime();

	if (ms >= LLONG_MAX / US_PER_MS) {
	    TimeTooFarError(interp);
	    return TCL_ERROR;
	}
	microSeconds = ms * US_PER_MS;

	if (LLONG_MAX - microSeconds < wakeupUS) {
	    TimeTooFarError(interp);
	    return TCL_ERROR;
	}

	wakeupUS += microSeconds;

	if (objc == 2) {
	    /*
	     * No command given: wait the given monotonic time.
	     */

	    return TimerDelayMonotonic(interp, wakeupUS);
	}

	/*
	 * Invoke command after given monotonic time distance.
	 */

	assocPtr = TimerAssocDataGet(interp);
	tsdPtr = InitTimer();
	afterPtr = (AfterInfo *)Tcl_Alloc(sizeof(AfterInfo));
	afterPtr->assocPtr = assocPtr;
	if (objc == 3) {
	    afterPtr->commandPtr = objv[2];
	} else {
	    afterPtr->commandPtr = Tcl_ConcatObj(objc-2, objv+2);
	}
	Tcl_IncrRefCount(afterPtr->commandPtr);

	/*
	 * The variable below is used to generate unique identifiers for after
	 * commands. This id can wrap around, which can potentially cause
	 * problems. However, there are not likely to be problems in practice,
	 * because after commands can only be requested to about a month in
	 * the future, and wrap-around is unlikely to occur in less than about
	 * 1-10 years. Thus it's unlikely that any old ids will still be
	 * around when wrap-around occurs.
	 */

	afterPtr->id = tsdPtr->afterId;
	tsdPtr->afterId += 1;
	afterPtr->token = CreateTimerHandler(wakeupUS,
		AfterProc, afterPtr, true);
	afterPtr->nextPtr = assocPtr->firstAfterPtr;
	assocPtr->firstAfterPtr = afterPtr;
	Tcl_SetObjResult(interp, Tcl_ObjPrintf("after#%d", afterPtr->id));
	return TCL_OK;
    }
    case AFTER_CANCEL:
	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "id|script ?script?");
	    return TCL_ERROR;
	}
	if (objc == 3) {
	    res = TimerCancelDo(interp, objv[2], false);
	} else {
	    cmdObj = Tcl_ConcatObj(objc-2, objv+2);
	    res = TimerCancelDo(interp, cmdObj, false);

	    /*
	     * When Tcl_ConcatObj was used, the created object is only
	     * decremented in this case, not in the other 3 cases in this
	     * function. I don't know why.
	     */

	    Tcl_DecrRefCount(cmdObj);
	}
	return res;
    case AFTER_IDLE:
	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "script ?script ...?");
	    return TCL_ERROR;
	}
	if (objc == 3) {
	    cmdObj = objv[2];
	} else {
	    cmdObj = Tcl_ConcatObj(objc-2, objv+2);
	}
	return TimerIdleDo(interp, cmdObj);
    case AFTER_INFO:
	if (objc < 2 || objc > 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "?id?");
	    return TCL_ERROR;
	}
	return TimerInfoDo(interp, objc-1, objv+1, true);
    default:
	TCL_UNREACHABLE();
    }
    return TCL_OK;
}
```

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclTimer.c`, function `GetAfterEvent`, lines 1484–1511. Retained reference `eaf331dbfda81fff6c4db`.

```text
GetAfterEvent(
    AfterAssocData *assocPtr,	/* Points to "after"-related information for
				 * this interpreter. */
    Tcl_Obj *commandPtr)
{
    const char *cmdString;	/* Textual identifier for after event, such as
				 * "after#6". */
    AfterInfo *afterPtr;
    int id;
    char *end;

    cmdString = TclGetString(commandPtr);
    if (strncmp(cmdString, "after#", 6) != 0) {
	return NULL;
    }
    cmdString += 6;
    id = (int)strtoul(cmdString, &end, 10);
    if ((end == cmdString) || (*end != 0)) {
	return NULL;
    }
    for (afterPtr = assocPtr->firstAfterPtr; afterPtr != NULL;
	    afterPtr = afterPtr->nextPtr) {
	if (afterPtr->id == id) {
	    return afterPtr;
	}
    }
    return NULL;
}
```

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEvent.c`, function `Tcl_UpdateObjCmd`, lines 1992–2039. Retained reference `eaa5f919dc246958bc5d1`.

```text
Tcl_UpdateObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    int flags = 0;		/* Initialized to avoid compiler warning. */
    static const char *const updateOptions[] = {"idletasks", NULL};
    enum updateOptionsEnum {OPT_IDLETASKS} optionIndex;

    if (objc == 1) {
	flags = TCL_ALL_EVENTS|TCL_DONT_WAIT;
    } else if (objc == 2) {
	if (Tcl_GetIndexFromObj(interp, objv[1], updateOptions,
		"option", 0, &optionIndex) != TCL_OK) {
	    return TCL_ERROR;
	}
	switch (optionIndex) {
	case OPT_IDLETASKS:
	    flags = TCL_IDLE_EVENTS|TCL_DONT_WAIT;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "?idletasks?");
	return TCL_ERROR;
    }

    while (Tcl_DoOneEvent(flags) != 0) {
	if (Tcl_Canceled(interp, TCL_LEAVE_ERR_MSG) == TCL_ERROR) {
	    return TCL_ERROR;
	}
	if (Tcl_LimitExceeded(interp)) {
	    Tcl_ResetResult(interp);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj("limit exceeded", -1));
	    return TCL_ERROR;
	}
    }

    /*
     * Must clear the interpreter's result because event handlers could have
     * executed commands.
     */

    Tcl_ResetResult(interp);
    return TCL_OK;
}
```

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `JimELUpdateCommand`, lines 647–670. Retained reference `e4521e47b782c5165c471`.

```text
static int JimELUpdateCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_EventLoop *eventLoop = Jim_CmdPrivData(interp);
    static const char * const options[] = {
        "idletasks", NULL
    };
    enum { UPDATE_IDLE, UPDATE_NONE };
    int option = UPDATE_NONE;
    int flags = JIM_TIME_EVENTS;

    if (argc == 1) {
        flags = JIM_ALL_EVENTS;
    }
    else if (argc > 2 || Jim_GetEnum(interp, argv[1], options, &option, NULL, JIM_ENUM_ABBREV) != JIM_OK) {
        return JIM_USAGE;
    }

    eventLoop->suppress_bgerror = 0;

    while (Jim_ProcessEvents(interp, flags | JIM_DONT_WAIT) > 0) {
    }

    return JIM_OK;
}
```

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `JimELAfterCommand`, lines 686–793. Retained reference `e4521e47b782c5165c471`.

```text
static int JimELAfterCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_EventLoop *eventLoop = Jim_CmdPrivData(interp);
    double ms = 0;
    jim_wide id;
    Jim_Obj *objPtr, *idObjPtr;
    static const char * const options[] = {
        "cancel", "info", "idle", NULL
    };
    enum
    { AFTER_CANCEL, AFTER_INFO, AFTER_IDLE, AFTER_RESTART, AFTER_EXPIRE, AFTER_CREATE };
    int option = AFTER_CREATE;
    if (Jim_GetDouble(interp, argv[1], &ms) != JIM_OK) {
        if (Jim_GetEnum(interp, argv[1], options, &option, "argument", JIM_ERRMSG) != JIM_OK) {
            return JIM_ERR;
        }
        Jim_SetEmptyResult(interp);
    }
    else if (argc == 2) {
        /* Simply a sleep */
        usleep(ms * 1000);
        return JIM_OK;
    }

    switch (option) {
        case AFTER_IDLE:
            if (argc < 3) {
                Jim_WrongNumArgs(interp, 2, argv, "script ?script ...?");
                return JIM_ERR;
            }
            /* fall through */
        case AFTER_CREATE: {
            Jim_Obj *scriptObj = Jim_ConcatObj(interp, argc - 2, argv + 2);
            Jim_IncrRefCount(scriptObj);
            id = Jim_CreateTimeHandler(interp, (jim_wide)(ms * 1000), JimAfterTimeHandler, scriptObj,
                JimAfterTimeEventFinalizer);
            objPtr = Jim_NewStringObj(interp, NULL, 0);
            Jim_AppendString(interp, objPtr, "after#", -1);
            idObjPtr = Jim_NewIntObj(interp, id);
            Jim_IncrRefCount(idObjPtr);
            Jim_AppendObj(interp, objPtr, idObjPtr);
            Jim_DecrRefCount(interp, idObjPtr);
            Jim_SetResult(interp, objPtr);
            return JIM_OK;
        }
        case AFTER_CANCEL:
            if (argc < 3) {
                Jim_WrongNumArgs(interp, 2, argv, "id|command");
                return JIM_ERR;
            }
            else {
                jim_wide remain = 0;

                id = JimParseAfterId(argv[2]);
                if (id <= 0) {
                    /* Not an event id, so search by script */
                    Jim_Obj *scriptObj = Jim_ConcatObj(interp, argc - 2, argv + 2);
                    id = JimFindAfterByScript(eventLoop, scriptObj);
                    Jim_FreeNewObj(interp, scriptObj);
                    if (id <= 0) {
                        /* Not found */
                        break;
                    }
                }
                remain = Jim_DeleteTimeHandler(interp, id);
                if (remain >= 0) {
                    Jim_SetResultInt(interp, remain);
                }
            }
            break;

        case AFTER_INFO:
            if (argc == 2) {
                Jim_TimeEvent *te = eventLoop->timeEventHead;
                Jim_Obj *listObj = Jim_NewListObj(interp, NULL, 0);
                char buf[30];
                const char *fmt = "after#%" JIM_WIDE_MODIFIER;

                while (te) {
                    snprintf(buf, sizeof(buf), fmt, te->id);
                    Jim_ListAppendElement(interp, listObj, Jim_NewStringObj(interp, buf, -1));
                    te = te->next;
                }
                Jim_SetResult(interp, listObj);
            }
            else if (argc == 3) {
                id = JimParseAfterId(argv[2]);
                if (id >= 0) {
                    Jim_TimeEvent *e = JimFindTimeHandlerById(eventLoop, id);
                    if (e && e->timeProc == JimAfterTimeHandler) {
                        Jim_Obj *listObj = Jim_NewListObj(interp, NULL, 0);
                        Jim_ListAppendElement(interp, listObj, e->clientData);
                        Jim_ListAppendElement(interp, listObj, Jim_NewStringObj(interp, e->initialus ? "timer" : "idle", -1));
                        Jim_SetResult(interp, listObj);
                        return JIM_OK;
                    }
                }
                Jim_SetResultFormatted(interp, "event \"%#s\" doesn't exist", argv[2]);
                return JIM_ERR;
            }
            else {
                Jim_WrongNumArgs(interp, 2, argv, "?id?");
                return JIM_ERR;
            }
            break;
    }
    return JIM_OK;
}
```

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `JimParseAfterId`, lines 276–286. Retained reference `e4521e47b782c5165c471`.

```text
static jim_wide JimParseAfterId(Jim_Obj *idObj)
{
    const char *tok = Jim_String(idObj);
    jim_wide id;

    if (strncmp(tok, "after#", 6) == 0 && Jim_StringToWide(tok + 6, &id, 10) == JIM_OK) {
        /* Got an event by id */
        return id;
    }
    return -1;
}
```

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `Jim_DeleteTimeHandler`, lines 339–359. Retained reference `e4521e47b782c5165c471`.

```text
jim_wide Jim_DeleteTimeHandler(Jim_Interp *interp, jim_wide id)
{
    Jim_TimeEvent *te;
    Jim_EventLoop *eventLoop = Jim_GetAssocData(interp, "eventloop");

    if (id > eventLoop->timeEventNextId) {
        return -2;              /* wrong event ID */
    }

    te = Jim_RemoveTimeHandler(eventLoop, id);
    if (te) {
        jim_wide remain;

        remain = te->when - Jim_GetTimeUsec(CLOCK_MONOTONIC_RAW);
        remain = (remain < 0) ? 0 : remain;

        Jim_FreeTimeHandler(interp, te);
        return remain;
    }
    return -1;                  /* NO event with the specified ID found */
}
```

## Consumer bindings

- [rust/tcl-registry/src/native_event.rs](../../../../rust/tcl-registry/src/native_event.rs), `NativeEventProtocol`: Pure independently selected event purpose; no original-object or callback-free grant.
- [rust/tcl-cmd-core/src/event.rs](../../../../rust/tcl-cmd-core/src/event.rs), `EventQueue`: Live original script-handle queue and current service-turn topology.
- [rust/tcl-vm/src/cmd_event.rs](../../../../rust/tcl-vm/src/cmd_event.rs), `cmd_after`: Port-owned genuine original object/current frame consumer; does not infer independent Normal or Native preparation.
- [runtime/rust/src/cmd_event.rs](../../../../runtime/rust/src/cmd_event.rs), `after_cmd`: Port-owned genuine original object/current frame consumer; does not infer independent Normal or Native preparation.
- [rust/tcl-vm/src/cmd_event/native_original_tests.rs](../../../../rust/tcl-vm/src/cmd_event/native_original_tests.rs), `cmd_event::native_original_tests::original_event_scripts_global_waits_and_timer_turns_match_native_controls` (linked): Finite code/result comparison for 21 retained source controls across six independently selected profiles (126 comparisons per port). No execution result is asserted by this binding.
- [runtime/rust/src/cmd_event/native_original_tests.rs](../../../../runtime/rust/src/cmd_event/native_original_tests.rs), `cmd_event::native_original_tests::original_event_scripts_global_waits_and_timer_turns_match_native_controls` (linked): Finite code/result comparison for 21 retained source controls across six independently selected profiles (126 comparisons per port). No execution result is asserted by this binding.

## Evidence

- [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20)
- [rust/tcl-registry/tests/data/native_event_original](../../../../rust/tcl-registry/tests/data/native_event_original)
- [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19)
- [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18)
- [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4)
- [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0)
- [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim)
