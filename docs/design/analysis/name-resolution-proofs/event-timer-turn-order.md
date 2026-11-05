# naming.event.timer-turn-order

Kind: `native-observation`

## Problem statement

Removing a whole copied batch before callbacks hides later cancellation and incorrectly admits new registrations into the same turn.

## Question

In these event controls, how do callback cancellation, callback registration, idle-only updates and a bare delay affect serviced scripts?

## Conclusion

All six captured BATCH_CANCEL controls return FIRST and BATCH_NEW returns FIRST SECOND NEW. UPDATE_IDLE ends at IDLE, with pinned source selecting C idle-only service and Jim timer service. DELAY_NO_EVENTS remains OLD. These are current live-entry and generation controls, not a microsecond timing guarantee.

## Scope

Four exact ASCII source controls per provider in v106/v107, including event-turn live cancellation and later registration. Does not establish external notifier inventory, asynchronous sources, concurrency, scheduler fairness or general event completion purity. C84 Tcl_GetReturnOptions is absent; Jim C return-options API is not queried; corresponding rows remain explicitly not-tested. Raw receipts and prior attempts remain immutable.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim 0.84-9-g5bac7c9.

v106/UPDATE_IDLE: UPDATE_IDLE: code 0, bytes b'IDLE' v106/DELAY_NO_EVENTS: DELAY_NO_EVENTS: code 0, bytes b'OLD' v107/BATCH_CANCEL: BATCH_CANCEL: code 0, bytes b'FIRST' v107/BATCH_NEW: BATCH_NEW: code 0, bytes b'FIRST SECOND NEW'

### bigip

Status: `not-tested`. Channel: not exercised.

No BIG-IP provider was executed or inspected for this Event question.

## Source inspection

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclTimer.c`, function `TimerHandlerEventProc`, lines 467–549. Retained reference `e7ca34f2d6fd9344d4610`.

```text
TimerHandlerEventProc(evPtr, flags)
    Tcl_Event *evPtr;		/* Event to service. */
    int flags;			/* Flags that indicate what events to
				 * handle, such as TCL_FILE_EVENTS. */
{
    TimerHandler *timerHandlerPtr, **nextPtrPtr;
    Tcl_Time time;
    int currentTimerId;
    ThreadSpecificData *tsdPtr = InitTimer();

    /*
     * Do nothing if timers aren't enabled.  This leaves the event on the
     * queue, so we will get to it as soon as ServiceEvents() is called
     * with timers enabled.
     */

    if (!(flags & TCL_TIMER_EVENTS)) {
	return 0;
    }

    /*
     * The code below is trickier than it may look, for the following
     * reasons:
     *
     * 1. New handlers can get added to the list while the current
     *    one is being processed.  If new ones get added, we don't
     *    want to process them during this pass through the list to avoid
     *	  starving other event sources.  This is implemented using the
     *	  token number in the handler:  new handlers will have a
     *    newer token than any of the ones currently on the list.
     * 2. The handler can call Tcl_DoOneEvent, so we have to remove
     *    the handler from the list before calling it. Otherwise an
     *    infinite loop could result.
     * 3. Tcl_DeleteTimerHandler can be called to remove an element from
     *    the list while a handler is executing, so the list could
     *    change structure during the call.
     * 4. Because we only fetch the current time before entering the loop,
     *    the only way a new timer will even be considered runnable is if
     *	  its expiration time is within the same millisecond as the
     *	  current time.  This is fairly likely on Windows, since it has
     *	  a course granularity clock.  Since timers are placed
     *	  on the queue in time order with the most recently created
     *    handler appearing after earlier ones with the same expiration
     *	  time, we don't have to worry about newer generation timers
     *	  appearing before later ones.
     */

    tsdPtr->timerPending = 0;
    currentTimerId = tsdPtr->lastTimerId;
    Tcl_GetTime(&time);
    while (1) {
	nextPtrPtr = &tsdPtr->firstTimerHandlerPtr;
	timerHandlerPtr = tsdPtr->firstTimerHandlerPtr;
	if (timerHandlerPtr == NULL) {
	    break;
	}
	    
	if ((timerHandlerPtr->time.sec > time.sec)
		|| ((timerHandlerPtr->time.sec == time.sec)
			&& (timerHandlerPtr->time.usec > time.usec))) {
	    break;
	}

	/*
	 * Bail out if the next timer is of a newer generation.
	 */

	if ((currentTimerId - (int)timerHandlerPtr->token) < 0) {
	    break;
	}

	/*
	 * Remove the handler from the queue before invoking it,
	 * to avoid potential reentrancy problems.
	 */

	(*nextPtrPtr) = timerHandlerPtr->nextPtr;
	(*timerHandlerPtr->proc)(timerHandlerPtr->clientData);
	ckfree((char *) timerHandlerPtr);
    }
    TimerSetupProc(NULL, TCL_TIMER_EVENTS);
    return 1;
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

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclTimer.c`, function `TimerHandlerEventProc`, lines 505–583. Retained reference `eb83569215d92fa37da10`.

```text
TimerHandlerEventProc(
    Tcl_Event *evPtr,		/* Event to service. */
    int flags)			/* Flags that indicate what events to handle,
				 * such as TCL_FILE_EVENTS. */
{
    TimerHandler *timerHandlerPtr, **nextPtrPtr;
    Tcl_Time time;
    int currentTimerId;
    ThreadSpecificData *tsdPtr = InitTimer();

    /*
     * Do nothing if timers aren't enabled. This leaves the event on the
     * queue, so we will get to it as soon as ServiceEvents() is called with
     * timers enabled.
     */

    if (!(flags & TCL_TIMER_EVENTS)) {
	return 0;
    }

    /*
     * The code below is trickier than it may look, for the following reasons:
     *
     * 1. New handlers can get added to the list while the current one is
     *	  being processed. If new ones get added, we don't want to process
     *	  them during this pass through the list to avoid starving other event
     *	  sources. This is implemented using the token number in the handler:
     *	  new handlers will have a newer token than any of the ones currently
     *	  on the list.
     * 2. The handler can call Tcl_DoOneEvent, so we have to remove the
     *	  handler from the list before calling it. Otherwise an infinite loop
     *	  could result.
     * 3. Tcl_DeleteTimerHandler can be called to remove an element from the
     *	  list while a handler is executing, so the list could change
     *	  structure during the call.
     * 4. Because we only fetch the current time before entering the loop, the
     *	  only way a new timer will even be considered runnable is if its
     *	  expiration time is within the same millisecond as the current time.
     *	  This is fairly likely on Windows, since it has a course granularity
     *	  clock. Since timers are placed on the queue in time order with the
     *	  most recently created handler appearing after earlier ones with the
     *	  same expiration time, we don't have to worry about newer generation
     *	  timers appearing before later ones.
     */

    tsdPtr->timerPending = 0;
    currentTimerId = tsdPtr->lastTimerId;
    Tcl_GetTime(&time);
    while (1) {
	nextPtrPtr = &tsdPtr->firstTimerHandlerPtr;
	timerHandlerPtr = tsdPtr->firstTimerHandlerPtr;
	if (timerHandlerPtr == NULL) {
	    break;
	}

	if (TCL_TIME_BEFORE(time, timerHandlerPtr->time)) {
	    break;
	}

	/*
	 * Bail out if the next timer is of a newer generation.
	 */

	if ((currentTimerId - PTR2INT(timerHandlerPtr->token)) < 0) {
	    break;
	}

	/*
	 * Remove the handler from the queue before invoking it, to avoid
	 * potential reentrancy problems.
	 */

	(*nextPtrPtr) = timerHandlerPtr->nextPtr;
	(*timerHandlerPtr->proc)(timerHandlerPtr->clientData);
	ckfree((char *) timerHandlerPtr);
    }
    TimerSetupProc(NULL, TCL_TIMER_EVENTS);
    return 1;
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

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclTimer.c`, function `TimerHandlerEventProc`, lines 520–598. Retained reference `e5cad7b4101332ff96f7c`.

```text
TimerHandlerEventProc(
    Tcl_Event *evPtr,		/* Event to service. */
    int flags)			/* Flags that indicate what events to handle,
				 * such as TCL_FILE_EVENTS. */
{
    TimerHandler *timerHandlerPtr, **nextPtrPtr;
    Tcl_Time time;
    int currentTimerId;
    ThreadSpecificData *tsdPtr = InitTimer();

    /*
     * Do nothing if timers aren't enabled. This leaves the event on the
     * queue, so we will get to it as soon as ServiceEvents() is called with
     * timers enabled.
     */

    if (!(flags & TCL_TIMER_EVENTS)) {
	return 0;
    }

    /*
     * The code below is trickier than it may look, for the following reasons:
     *
     * 1. New handlers can get added to the list while the current one is
     *	  being processed. If new ones get added, we don't want to process
     *	  them during this pass through the list to avoid starving other event
     *	  sources. This is implemented using the token number in the handler:
     *	  new handlers will have a newer token than any of the ones currently
     *	  on the list.
     * 2. The handler can call Tcl_DoOneEvent, so we have to remove the
     *	  handler from the list before calling it. Otherwise an infinite loop
     *	  could result.
     * 3. Tcl_DeleteTimerHandler can be called to remove an element from the
     *	  list while a handler is executing, so the list could change
     *	  structure during the call.
     * 4. Because we only fetch the current time before entering the loop, the
     *	  only way a new timer will even be considered runnable is if its
     *	  expiration time is within the same millisecond as the current time.
     *	  This is fairly likely on Windows, since it has a course granularity
     *	  clock. Since timers are placed on the queue in time order with the
     *	  most recently created handler appearing after earlier ones with the
     *	  same expiration time, we don't have to worry about newer generation
     *	  timers appearing before later ones.
     */

    tsdPtr->timerPending = 0;
    currentTimerId = tsdPtr->lastTimerId;
    Tcl_GetTime(&time);
    while (1) {
	nextPtrPtr = &tsdPtr->firstTimerHandlerPtr;
	timerHandlerPtr = tsdPtr->firstTimerHandlerPtr;
	if (timerHandlerPtr == NULL) {
	    break;
	}

	if (TCL_TIME_BEFORE(time, timerHandlerPtr->time)) {
	    break;
	}

	/*
	 * Bail out if the next timer is of a newer generation.
	 */

	if ((currentTimerId - PTR2INT(timerHandlerPtr->token)) < 0) {
	    break;
	}

	/*
	 * Remove the handler from the queue before invoking it, to avoid
	 * potential reentrancy problems.
	 */

	*nextPtrPtr = timerHandlerPtr->nextPtr;
	timerHandlerPtr->proc(timerHandlerPtr->clientData);
	ckfree(timerHandlerPtr);
    }
    TimerSetupProc(NULL, TCL_TIMER_EVENTS);
    return 1;
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

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclTimer.c`, function `TimerHandlerEventProc`, lines 517–595. Retained reference `e4d5b46a62af04d177edb`.

```text
TimerHandlerEventProc(
    TCL_UNUSED(Tcl_Event *),
    int flags)			/* Flags that indicate what events to handle,
				 * such as TCL_FILE_EVENTS. */
{
    TimerHandler *timerHandlerPtr, **nextPtrPtr;
    Tcl_Time time;
    int currentTimerId;
    ThreadSpecificData *tsdPtr = InitTimer();

    /*
     * Do nothing if timers aren't enabled. This leaves the event on the
     * queue, so we will get to it as soon as ServiceEvents() is called with
     * timers enabled.
     */

    if (!(flags & TCL_TIMER_EVENTS)) {
	return 0;
    }

    /*
     * The code below is trickier than it may look, for the following reasons:
     *
     * 1. New handlers can get added to the list while the current one is
     *	  being processed. If new ones get added, we don't want to process
     *	  them during this pass through the list to avoid starving other event
     *	  sources. This is implemented using the token number in the handler:
     *	  new handlers will have a newer token than any of the ones currently
     *	  on the list.
     * 2. The handler can call Tcl_DoOneEvent, so we have to remove the
     *	  handler from the list before calling it. Otherwise an infinite loop
     *	  could result.
     * 3. Tcl_DeleteTimerHandler can be called to remove an element from the
     *	  list while a handler is executing, so the list could change
     *	  structure during the call.
     * 4. Because we only fetch the current time before entering the loop, the
     *	  only way a new timer will even be considered runnable is if its
     *	  expiration time is within the same millisecond as the current time.
     *	  This is fairly likely on Windows, since it has a course granularity
     *	  clock. Since timers are placed on the queue in time order with the
     *	  most recently created handler appearing after earlier ones with the
     *	  same expiration time, we don't have to worry about newer generation
     *	  timers appearing before later ones.
     */

    tsdPtr->timerPending = 0;
    currentTimerId = tsdPtr->lastTimerId;
    Tcl_GetTime(&time);
    while (1) {
	nextPtrPtr = &tsdPtr->firstTimerHandlerPtr;
	timerHandlerPtr = tsdPtr->firstTimerHandlerPtr;
	if (timerHandlerPtr == NULL) {
	    break;
	}

	if (TCL_TIME_BEFORE(time, timerHandlerPtr->time)) {
	    break;
	}

	/*
	 * Bail out if the next timer is of a newer generation.
	 */

	if ((currentTimerId - PTR2INT(timerHandlerPtr->token)) < 0) {
	    break;
	}

	/*
	 * Remove the handler from the queue before invoking it, to avoid
	 * potential reentrancy problems.
	 */

	*nextPtrPtr = timerHandlerPtr->nextPtr;
	timerHandlerPtr->proc(timerHandlerPtr->clientData);
	Tcl_Free(timerHandlerPtr);
    }
    TimerSetupProc(NULL, TCL_TIMER_EVENTS);
    return 1;
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

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclTimer.c`, function `TimerHandlerEventProc`, lines 744–841. Retained reference `eaf331dbfda81fff6c4db`.

```text
TimerHandlerEventProc(
    TCL_UNUSED(Tcl_Event *),
    int flags)			/* Flags that indicate what events to handle,
				 * such as TCL_FILE_EVENTS. */
{
    ThreadSpecificData *tsdPtr = InitTimer();

    /*
     * Do nothing if timers aren't enabled. This leaves the event on the
     * queue, so we will get to it as soon as ServiceEvents() is called with
     * timers enabled.
     */

    if (!(flags & TCL_TIMER_EVENTS)) {
	return 0;
    }

    /*
     * The code below is trickier than it may look, for the following reasons:
     *
     * 1. New handlers can get added to the list while the current one is
     *	  being processed. If new ones get added, we don't want to process
     *	  them during this pass through the list to avoid starving other event
     *	  sources. This is implemented using the token number in the handler:
     *	  new handlers will have a newer token than any of the ones currently
     *	  on the list.
     * 2. The handler can call Tcl_DoOneEvent, so we have to remove the
     *	  handler from the list before calling it. Otherwise an infinite loop
     *	  could result.
     * 3. Tcl_DeleteTimerHandler can be called to remove an element from the
     *	  list while a handler is executing, so the list could change
     *	  structure during the call.
     * 4. Because we only fetch the current time before entering the loop, the
     *	  only way a new timer will even be considered runnable is if its
     *	  expiration time is within the same millisecond as the current time.
     *	  This is fairly likely on Windows, since it has a course granularity
     *	  clock. Since timers are placed on the queue in time order with the
     *	  most recently created handler appearing after earlier ones with the
     *	  same expiration time, we don't have to worry about newer generation
     *	  timers appearing before later ones.
     */

    for (int timerHandlerIndex = timerHandlerMonotonic;
	    timerHandlerIndex <= timerHandlerWallclock; timerHandlerIndex++) {
	long long timeUS;
	TimerHandler *timerHandlerPtr, **nextPtrPtr;
	int currentTimerId;

	if (!tsdPtr->timerPendingQueue[timerHandlerIndex]) {
	    continue;
	}
	tsdPtr->timerPendingQueue[timerHandlerIndex] = 0;

	/*
	 * As the queues are independent, but the timer ids are used for both,
	 * its creation time should be compared per queue.
	 * Thus, use the queues latest id.
	 */

	currentTimerId = tsdPtr->lastTimerIdQueue[timerHandlerIndex];

	if (timerHandlerIndex == timerHandlerMonotonic) {
	    timeUS = Tcl_GetMonotonicTime();
	} else {
	    timeUS = Tcl_GetDayTime();
	}
	while (1) {
	    nextPtrPtr = &tsdPtr->firstTimerHandlerPtr[timerHandlerIndex];
	    timerHandlerPtr = tsdPtr->firstTimerHandlerPtr[timerHandlerIndex];
	    if (timerHandlerPtr == NULL) {
		break;
	    }

	    if (timeUS < timerHandlerPtr->time) {
		break;
	    }

	    /*
	     * Bail out if the next timer is of a newer generation.
	     */

	    if ((currentTimerId - PTR2INT(timerHandlerPtr->token)) < 0) {
		break;
	    }

	    /*
	     * Remove the handler from the queue before invoking it, to avoid
	     * potential reentrancy problems.
	     */

	    *nextPtrPtr = timerHandlerPtr->nextPtr;
	    timerHandlerPtr->proc(timerHandlerPtr->clientData);
	    Tcl_Free(timerHandlerPtr);
	}
    }
    TimerSetupProc(NULL, TCL_TIMER_EVENTS);
    return 1;
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

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `Jim_ProcessEvents`, lines 377–534. Retained reference `e4521e47b782c5165c471`.

```text
int Jim_ProcessEvents(Jim_Interp *interp, int flags)
{
    jim_wide sleep_us = -1;
    int processed = 0;
    Jim_EventLoop *eventLoop = Jim_GetAssocData(interp, "eventloop");
    Jim_FileEvent *fe = eventLoop->fileEventHead;
    Jim_TimeEvent *te;
    jim_wide maxId;

    if ((flags & JIM_FILE_EVENTS) == 0 || fe == NULL) {
        /* No file events */
        if ((flags & JIM_TIME_EVENTS) == 0 || eventLoop->timeEventHead == NULL) {
            /* No time events */
            return -1;
        }
    }

    /* Note that we want call select() even if there are no
     * file events to process as long as we want to process time
     * events, in order to sleep until the next time event is ready
     * to fire. */

    if (flags & JIM_DONT_WAIT) {
        /* Wait no time */
        sleep_us = 0;
    }
    else if (flags & JIM_TIME_EVENTS) {
        /* The nearest timer is always at the head of the list */
        if (eventLoop->timeEventHead) {
            Jim_TimeEvent *shortest = eventLoop->timeEventHead;

            /* Calculate the time missing for the nearest
             * timer to fire. */
            sleep_us = shortest->when - Jim_GetTimeUsec(CLOCK_MONOTONIC_RAW);
            if (sleep_us < 0) {
                sleep_us = 0;
            }
        }
        else {
            /* Wait forever */
            sleep_us = -1;
        }
    }

#ifdef HAVE_SELECT
    if (flags & JIM_FILE_EVENTS) {
        int retval;
        struct timeval tv, *tvp = NULL;
        fd_set rfds, wfds, efds;
        int maxfd = -1;

        FD_ZERO(&rfds);
        FD_ZERO(&wfds);
        FD_ZERO(&efds);

        /* Check file events */
        while (fe != NULL) {
            if (fe->mask & JIM_EVENT_READABLE)
                FD_SET(fe->fd, &rfds);
            if (fe->mask & JIM_EVENT_WRITABLE)
                FD_SET(fe->fd, &wfds);
            if (fe->mask & JIM_EVENT_EXCEPTION)
                FD_SET(fe->fd, &efds);
            if (maxfd < fe->fd)
                maxfd = fe->fd;
            fe = fe->next;
        }

        if (sleep_us >= 0) {
            tvp = &tv;
            tvp->tv_sec = sleep_us / 1000000;
            tvp->tv_usec = sleep_us % 1000000;
        }

        retval = select(maxfd + 1, &rfds, &wfds, &efds, tvp);

        if (retval < 0) {
            if (errno == EINVAL) {
                /* This can happen on mingw32 if a non-socket filehandle is passed */
                Jim_SetResultString(interp, "non-waitable filehandle", -1);
                return -2;
            }
        }
        else if (retval > 0) {
            fe = eventLoop->fileEventHead;
            while (fe != NULL) {
                int mask = 0;
                int fd = fe->fd;

                if ((fe->mask & JIM_EVENT_READABLE) && FD_ISSET(fd, &rfds))
                    mask |= JIM_EVENT_READABLE;
                if (fe->mask & JIM_EVENT_WRITABLE && FD_ISSET(fd, &wfds))
                    mask |= JIM_EVENT_WRITABLE;
                if (fe->mask & JIM_EVENT_EXCEPTION && FD_ISSET(fd, &efds))
                    mask |= JIM_EVENT_EXCEPTION;

                if (mask) {
                    int ret = fe->fileProc(interp, fe->clientData, mask);
                    if (ret != JIM_OK && ret != JIM_RETURN) {
                        /* Remove the element on handler error */
                        Jim_DeleteFileHandler(interp, fd, mask);
                        /* At this point fe is no longer valid - it will be assigned below */
                    }
                    processed++;
                    /* After an event is processed our file event list
                     * may no longer be the same, so what we do
                     * is to clear the bit for this file descriptor and
                     * restart again from the head. */
                    FD_CLR(fd, &rfds);
                    FD_CLR(fd, &wfds);
                    FD_CLR(fd, &efds);
                    fe = eventLoop->fileEventHead;
                }
                else {
                    fe = fe->next;
                }
            }
        }
    }
#else
    if (sleep_us > 0) {
        usleep(sleep_us);
    }
#endif

    /* Check time events */
    te = eventLoop->timeEventHead;
    maxId = eventLoop->timeEventNextId;
    while (te) {
        jim_wide id;

        if (te->id > maxId) {
            te = te->next;
            continue;
        }
        if (Jim_GetTimeUsec(CLOCK_MONOTONIC_RAW) >= te->when) {
            id = te->id;
            /* Remove from the list before executing */
            Jim_RemoveTimeHandler(eventLoop, id);
            te->timeProc(interp, te->clientData);
            /* After an event is processed our time event list may
             * no longer be the same, so we restart from head.
             * Still we make sure to don't process events registered
             * by event handlers itself in order to don't loop forever
             * even in case an [after 0] that continuously register
             * itself. To do so we saved the max ID we want to handle. */
            Jim_FreeTimeHandler(interp, te);

            te = eventLoop->timeEventHead;
            processed++;
        }
        else {
            te = te->next;
        }
    }

    return processed;
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

## Consumer bindings

- [rust/tcl-registry/src/native_event.rs](../../../../rust/tcl-registry/src/native_event.rs), `NativeEventProtocol`: Pure independently selected event purpose; no original-object or callback-free grant.
- [rust/tcl-cmd-core/src/event.rs](../../../../rust/tcl-cmd-core/src/event.rs), `EventQueue`: Live original script-handle queue and current service-turn topology.
- [rust/tcl-vm/src/cmd_event.rs](../../../../rust/tcl-vm/src/cmd_event.rs), `cmd_after`: Port-owned genuine original object/current frame consumer; does not infer independent Normal or Native preparation.
- [runtime/rust/src/cmd_event.rs](../../../../runtime/rust/src/cmd_event.rs), `after_cmd`: Port-owned genuine original object/current frame consumer; does not infer independent Normal or Native preparation.
- [rust/tcl-vm/src/cmd_event/native_original_tests.rs](../../../../rust/tcl-vm/src/cmd_event/native_original_tests.rs), `cmd_event::native_original_tests::original_event_scripts_global_waits_and_timer_turns_match_native_controls` (linked): Finite code/result comparison for 21 retained source controls across six independently selected profiles (126 comparisons per port). No execution result is asserted by this binding.
- [runtime/rust/src/cmd_event/native_original_tests.rs](../../../../runtime/rust/src/cmd_event/native_original_tests.rs), `cmd_event::native_original_tests::original_event_scripts_global_waits_and_timer_turns_match_native_controls` (linked): Finite code/result comparison for 21 retained source controls across six independently selected profiles (126 comparisons per port). No execution result is asserted by this binding.

## Evidence

- [rust/tcl-registry/tests/data/native_event_original/v106/capture](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_event_original/v107/capture](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture)
