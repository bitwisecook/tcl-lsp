# naming.diagnostics.original-error-code-metadata-not-message

Kind: `native-observation`

## Problem statement

Does guest error text that looks like a wrong-arguments diagnostic select its error-code classification, or does classification belong to the original error producer and explicit metadata? Distinguish an arbitrary error message, genuine set wrongargs and explicit supplied error code independently in every original provider.

## Question

Does wrong-arguments-looking error text establish public errorCode metadata, and how do actual set arity errors and explicit error-code arguments differ across the pinned original C releases and Jim?

## Conclusion

All six original providers replace the explicit seed with NONE for an arbitrary error whose message resembles a wrong-arguments diagnostic. Genuine catch {set} reports NONE in C8.4.20, C8.5.19 and current Jim 0.84-9-g5bac7c9, but TCL WRONGARGS in C8.6.18, C9.0.4 and C9.1.0. The five C releases accept the explicit error third argument and retain USER CODE while reporting the same arbitrary message. Jim rejects that three-argument command with its own error-message arity diagnostic and reports NONE; no Jim explicit-code positive is observed. Every public errorCode query succeeds with availability1, including Jim, and all case catch codes are1. Thus matching message text does not establish emitted diagnostic metadata, and the actual wrong-arguments producer has a measured release boundary. Six fresh original CLI processes exit0 with empty stderr and complete three-case markers. No generic error/completion classification, private object/cache/frame/worker identity, source edit, handler admission, BIG-IP transfer or Rust assertion result is established.

A linked software embedding control keeps original caller-owned error-code/error-info metadata independently of arbitrary message text. This is a transport contract distinct from the six original Native CLI processes and their measured release boundary.

The selected software wrong-arguments protocol compares its error-code bytes with the six original published genuine catch-set rows. Those rows independently establish NONE for C8.4/C8.5/Jim and TCL WRONGARGS for C8.6/C9.0/C9.1; arbitrary message text remains separate.

## Scope

The immutable original ASCII LF source performs three caught public script operations in one fresh process per provider, setting a distinct seed before each. Count-preserving binary-scan hex retains exact result and public errorCode bytes; both availability and returned value are measured. Seeds are overwritten in every captured case. The fixed source does not contain raw NUL, non-ASCII, binary-produced keys, private headers or direct object-vector entry. Fresh info patchlevel queries are retained independently of prior version receipts reused under exact original CLI/source/header/configuration/static archive pins. No new compile is run; selected original CLI executable identities, launch argv, explicit TCL_LIBRARY overlay and prelaunch 162 required pins are retained. Inherited environment, compiler version and unrelated system library identities are unrecorded. Nineteen exact original Tcl_ErrorObjCmd/Tcl_SetObjCmd/Tcl_WrongNumArgs and Jim error/set/wrongargs/registration windows explain producers separately from observed output. The unchanged preparation request includes a conditional unsupported/untouched Jim errorCode caution; the actual captured Jim query instead succeeds and reports NONE. Only Jim third-argument explicit-code acceptance is purpose-unavailable; it is never used to label the supported public query N/A. These three cases do not certify other commands, release points, arbitrary catch options or message parsing. Software API controls and typed Guest/Host settlement require independent receipts; all prior native records remain unchanged. BIG-IP is not tested.

The embedding control has no executed outcome attached. Original Native430 provider answers, public query availability, explicit-code acceptance/refusal, exact source/streams and evidence pins remain unchanged. Software transport does not confer a generic diagnostic class, Native object/header/frame or successful handler.

The linked assertion definition has no executed result attached by this binding. Its NativeOrigin and String-producer assertions are software selection contracts, not original public header observations. Original Native430 provider answers, explicit-code acceptance/refusal, evidence/source windows and replay remain unchanged.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Original fixed ASCII LF CLI source; caught script completions, exact result/errorCode hex and public variable availability, with explicit seed before each case. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

Arbitrary-message NONE; genuine set wrongargs NONE; explicit USER CODE accepted with the unchanged arbitrary message.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original fixed ASCII LF CLI source; caught script completions, exact result/errorCode hex and public variable availability, with explicit seed before each case. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Arbitrary-message NONE; genuine set wrongargs TCL WRONGARGS; explicit USER CODE accepted with the unchanged arbitrary message.

### jim

Status: `observed`. Channel: Original fixed ASCII LF CLI source; caught script completions, exact result/errorCode hex and public variable availability, with explicit seed before each case. Version: jim 0.84-9-g5bac7c9.

Arbitrary-message NONE; genuine set wrongargs NONE; explicit third argument rejected by error arity, with NONE. The public errorCode query succeeds; only explicit-code acceptance is unavailable.

### bigip

Status: `not-tested`.

No BIG-IP process, diagnostic code, completion or transfer observation is executed by this request.

## Source inspection

tcl8.4 8.4.20, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdAH.c`, function `Tcl_ErrorObjCmd`, lines 536–652. Retained reference `native_encoding_utf8_offset307-8.4.20-tclCmdAH.c`.

```text
 *
 * Results:
 *	A standard Tcl object result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

	/* ARGSUSED */
int
Tcl_ErrorObjCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    Interp *iPtr = (Interp *) interp;
    char *info;
    int infoLen;

    if ((objc < 2) || (objc > 4)) {
	Tcl_WrongNumArgs(interp, 1, objv, "message ?errorInfo? ?errorCode?");
	return TCL_ERROR;
    }
    
    if (objc >= 3) {		/* process the optional info argument */
	info = Tcl_GetStringFromObj(objv[2], &infoLen);
	if (infoLen > 0) {
	    Tcl_AddObjErrorInfo(interp, info, infoLen);
	    iPtr->flags |= ERR_ALREADY_LOGGED;
	}
    }
    
    if (objc == 4) {
	Tcl_SetVar2Ex(interp, "errorCode", NULL, objv[3], TCL_GLOBAL_ONLY);
	iPtr->flags |= ERROR_CODE_SET;
    }
    
    Tcl_SetObjResult(interp, objv[1]);
    return TCL_ERROR;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_EvalObjCmd --
 *
 *	This object-based procedure is invoked to process the "eval" Tcl 
 *	command. See the user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl object result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

	/* ARGSUSED */
int
Tcl_EvalObjCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    int result;
    register Tcl_Obj *objPtr;
#ifdef TCL_TIP280
    Interp* iPtr = (Interp*) interp;
#endif

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "arg ?arg ...?");
	return TCL_ERROR;
    }
    
    if (objc == 2) {
#ifndef TCL_TIP280
	result = Tcl_EvalObjEx(interp, objv[1], TCL_EVAL_DIRECT);
#else
	/* TIP #280. Make argument location available to eval'd script */
	CmdFrame* invoker = iPtr->cmdFramePtr;
	int word          = 1;
	TclArgumentGet (interp, objv[1], &invoker, &word);
	result = TclEvalObjEx(interp, objv[1], TCL_EVAL_DIRECT,
			      invoker, word);
#endif
    } else {
	/*
	 * More than one argument: concatenate them together with spaces
	 * between, then evaluate the result.  Tcl_EvalObjEx will delete
	 * the object when it decrements its refcount after eval'ing it.
	 */
    	objPtr = Tcl_ConcatObj(objc-1, objv+1);
#ifndef TCL_TIP280
	result = Tcl_EvalObjEx(interp, objPtr, TCL_EVAL_DIRECT);
#else
	/* TIP #280. Make invoking context available to eval'd script */
	result = TclEvalObjEx(interp, objPtr, TCL_EVAL_DIRECT, NULL, 0);
#endif
    }
    if (result == TCL_ERROR) {
	char msg[32 + TCL_INTEGER_SPACE];

	sprintf(msg, "\n    (\"eval\" body line %d)", interp->errorLine);
	Tcl_AddObjErrorInfo(interp, msg, -1);
    }
    return result;
}

/*
 *----------------------------------------------------------------------
 *

```

tcl8.4 8.4.20, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclVar.c`, function `Tcl_SetObjCmd`, lines 1258–1374. Retained reference `native_error_code_metadata_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclVar.c`.

```text
 *
 * Results:
 *	A standard Tcl result value.
 *
 * Side effects:
 *	A variable's value may be changed.
 *
 *----------------------------------------------------------------------
 */

	/* ARGSUSED */
int
Tcl_SetObjCmd(dummy, interp, objc, objv)
    ClientData dummy;			/* Not used. */
    register Tcl_Interp *interp;	/* Current interpreter. */
    int objc;				/* Number of arguments. */
    Tcl_Obj *CONST objv[];		/* Argument objects. */
{
    Tcl_Obj *varValueObj;

    if (objc == 2) {
	varValueObj = Tcl_ObjGetVar2(interp, objv[1], NULL, TCL_LEAVE_ERR_MSG);
	if (varValueObj == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, varValueObj);
	return TCL_OK;
    } else if (objc == 3) {

	varValueObj = Tcl_ObjSetVar2(interp, objv[1], NULL, objv[2],
		TCL_LEAVE_ERR_MSG);
	if (varValueObj == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, varValueObj);
	return TCL_OK;
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "varName ?newValue?");
	return TCL_ERROR;
    }
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_SetVar --
 *
 *	Change the value of a variable.
 *
 * Results:
 *	Returns a pointer to the malloc'ed string which is the character
 *	representation of the variable's new value. The caller must not
 *	modify this string. If the write operation was disallowed then NULL
 *	is returned; if the TCL_LEAVE_ERR_MSG flag is set, then an
 *	explanatory message will be left in the interp's result. Note that the
 *	returned string may not be the same as newValue; this is because
 *	variable traces may modify the variable's value.
 *
 * Side effects:
 *	If varName is defined as a local or global variable in interp,
 *	its value is changed to newValue. If varName isn't currently
 *	defined, then a new global variable by that name is created.
 *
 *----------------------------------------------------------------------
 */

#undef Tcl_SetVar
CONST char *
Tcl_SetVar(interp, varName, newValue, flags)
    Tcl_Interp *interp;		/* Command interpreter in which varName is
				 * to be looked up. */
    CONST char *varName;	/* Name of a variable in interp. */
    CONST char *newValue;	/* New value for varName. */
    int flags;			/* Various flags that tell how to set value:
				 * any of TCL_GLOBAL_ONLY,
				 * TCL_NAMESPACE_ONLY, TCL_APPEND_VALUE,
				 * TCL_LIST_ELEMENT, TCL_LEAVE_ERR_MSG. */
{
    return Tcl_SetVar2(interp, varName, (char *) NULL, newValue, flags);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_SetVar2 --
 *
 *      Given a two-part variable name, which may refer either to a
 *      scalar variable or an element of an array, change the value
 *      of the variable.  If the named scalar or array or element
 *      doesn't exist then create one.
 *
 * Results:
 *	Returns a pointer to the malloc'ed string which is the character
 *	representation of the variable's new value. The caller must not
 *	modify this string. If the write operation was disallowed because an
 *	array was expected but not found (or vice versa), then NULL is
 *	returned; if the TCL_LEAVE_ERR_MSG flag is set, then an explanatory
 *	message will be left in the interp's result. Note that the returned
 *	string may not be the same as newValue; this is because variable
 *	traces may modify the variable's value.
 *
 * Side effects:
 *      The value of the given variable is set. If either the array
 *      or the entry didn't exist then a new one is created.
 *
 *----------------------------------------------------------------------
 */

CONST char *
Tcl_SetVar2(interp, part1, part2, newValue, flags)
    Tcl_Interp *interp;         /* Command interpreter in which variable is
                                 * to be looked up. */
    CONST char *part1;          /* If part2 is NULL, this is name of scalar
                                 * variable. Otherwise it is the name of
                                 * an array. */
    CONST char *part2;		/* Name of an element within an array, or
				 * NULL. */

```

tcl8.4 8.4.20, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclIndexObj.c`, function `Tcl_WrongNumArgs`, lines 430–488. Retained reference `native_error_code_metadata_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclIndexObj.c`.

```text
 *	An error message is generated in interp's result object to
 *	indicate that a command was invoked with the wrong number of
 *	arguments.  The message has the form
 *		wrong # args: should be "foo bar additional stuff"
 *	where "foo" and "bar" are the initial objects in objv (objc
 *	determines how many of these are printed) and "additional stuff"
 *	is the contents of the message argument.
 *
 *----------------------------------------------------------------------
 */

void
Tcl_WrongNumArgs(interp, objc, objv, message)
    Tcl_Interp *interp;			/* Current interpreter. */
    int objc;				/* Number of arguments to print
					 * from objv. */
    Tcl_Obj *CONST objv[];		/* Initial argument objects, which
					 * should be included in the error
					 * message. */
    CONST char *message;		/* Error message to print after the
					 * leading objects in objv. The
					 * message may be NULL. */
{
    Tcl_Obj *objPtr;
    int i;
    register IndexRep *indexRep;

    TclNewObj(objPtr);
    Tcl_SetObjResult(interp, objPtr);
    Tcl_AppendToObj(objPtr, "wrong # args: should be \"", -1);
    for (i = 0; i < objc; i++) {
	/*
	 * If the object is an index type use the index table which allows
	 * for the correct error message even if the subcommand was
	 * abbreviated.  Otherwise, just use the string rep.
	 */
	
	if (objv[i]->typePtr == &tclIndexType) {
	    indexRep = (IndexRep *) objv[i]->internalRep.otherValuePtr;
	    Tcl_AppendStringsToObj(objPtr, EXPAND_OF(indexRep), (char *) NULL);
	} else {
	    Tcl_AppendStringsToObj(objPtr, Tcl_GetString(objv[i]),
		    (char *) NULL);
	}

	/*
	 * Append a space character (" ") if there is more text to follow
	 * (either another element from objv, or the message string).
	 */
	if ((i < (objc - 1)) || message) {
	    Tcl_AppendStringsToObj(objPtr, " ", (char *) NULL);
	}
    }

    if (message) {
	Tcl_AppendStringsToObj(objPtr, message, (char *) NULL);
    }
    Tcl_AppendStringsToObj(objPtr, "\"", (char *) NULL);
}

```

tcl8.5 8.5.19, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdAH.c`, function `Tcl_ErrorObjCmd`, lines 581–697. Retained reference `native_encoding_utf8_offset307-8.5.19-tclCmdAH.c`.

```text
 *
 * Results:
 *	A standard Tcl object result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

	/* ARGSUSED */
int
Tcl_ErrorObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *options, *optName;

    if ((objc < 2) || (objc > 4)) {
	Tcl_WrongNumArgs(interp, 1, objv, "message ?errorInfo? ?errorCode?");
	return TCL_ERROR;
    }

    TclNewLiteralStringObj(options, "-code error -level 0");

    if (objc >= 3) {		/* Process the optional info argument */
	TclNewLiteralStringObj(optName, "-errorinfo");
	Tcl_ListObjAppendElement(NULL, options, optName);
	Tcl_ListObjAppendElement(NULL, options, objv[2]);
    }

    if (objc >= 4) {		/* Process the optional code argument */
	TclNewLiteralStringObj(optName, "-errorcode");
	Tcl_ListObjAppendElement(NULL, options, optName);
	Tcl_ListObjAppendElement(NULL, options, objv[3]);
    }

    Tcl_SetObjResult(interp, objv[1]);
    return Tcl_SetReturnOptions(interp, options);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_EvalObjCmd --
 *
 *	This object-based procedure is invoked to process the "eval" Tcl
 *	command. See the user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl object result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

	/* ARGSUSED */
int
Tcl_EvalObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int result;
    register Tcl_Obj *objPtr;
    Interp *iPtr = (Interp *) interp;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "arg ?arg ...?");
	return TCL_ERROR;
    }

    if (objc == 2) {
	/*
	 * TIP #280. Make argument location available to eval'd script.
	 */

	CmdFrame* invoker = iPtr->cmdFramePtr;
	int word          = 1;
	TclArgumentGet (interp, objv[1], &invoker, &word);

	result = TclEvalObjEx(interp, objv[1], TCL_EVAL_DIRECT,
		invoker, word);
    } else {
	/*
	 * More than one argument: concatenate them together with spaces
	 * between, then evaluate the result. Tcl_EvalObjEx will delete the
	 * object when it decrements its refcount after eval'ing it.
	 */

	objPtr = Tcl_ConcatObj(objc-1, objv+1);

	/*
	 * TIP #280. Make invoking context available to eval'd script.
	 */

	result = TclEvalObjEx(interp, objPtr, TCL_EVAL_DIRECT, NULL, 0);
    }
    if (result == TCL_ERROR) {
	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (\"eval\" body line %d)", interp->errorLine));
    }
    return result;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_ExitObjCmd --
 *
 *	This procedure is invoked to process the "exit" Tcl command. See the
 *	user documentation for details on what it does.

```

tcl8.5 8.5.19, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclVar.c`, function `Tcl_SetObjCmd`, lines 1500–1616. Retained reference `native_dict_write_error_lifetime226-failed-request225-original-source-tclVar.c`.

```text
 *
 * Results:
 *	A standard Tcl result value.
 *
 * Side effects:
 *	A variable's value may be changed.
 *
 *----------------------------------------------------------------------
 */

	/* ARGSUSED */
int
Tcl_SetObjCmd(
    ClientData dummy,		/* Not used. */
    register Tcl_Interp *interp,/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *varValueObj;

    if (objc == 2) {
	varValueObj = Tcl_ObjGetVar2(interp, objv[1], NULL,TCL_LEAVE_ERR_MSG);
	if (varValueObj == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, varValueObj);
	return TCL_OK;
    } else if (objc == 3) {
	varValueObj = Tcl_ObjSetVar2(interp, objv[1], NULL, objv[2],
		TCL_LEAVE_ERR_MSG);
	if (varValueObj == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, varValueObj);
	return TCL_OK;
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "varName ?newValue?");
	return TCL_ERROR;
    }
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_SetVar --
 *
 *	Change the value of a variable.
 *
 * Results:
 *	Returns a pointer to the malloc'ed string which is the character
 *	representation of the variable's new value. The caller must not modify
 *	this string. If the write operation was disallowed then NULL is
 *	returned; if the TCL_LEAVE_ERR_MSG flag is set, then an explanatory
 *	message will be left in the interp's result. Note that the returned
 *	string may not be the same as newValue; this is because variable
 *	traces may modify the variable's value.
 *
 * Side effects:
 *	If varName is defined as a local or global variable in interp, its
 *	value is changed to newValue. If varName isn't currently defined, then
 *	a new global variable by that name is created.
 *
 *----------------------------------------------------------------------
 */

#undef Tcl_SetVar
const char *
Tcl_SetVar(
    Tcl_Interp *interp,		/* Command interpreter in which varName is to
				 * be looked up. */
    const char *varName,	/* Name of a variable in interp. */
    const char *newValue,	/* New value for varName. */
    int flags)			/* Various flags that tell how to set value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
{
    return Tcl_SetVar2(interp, varName, NULL, newValue, flags);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_SetVar2 --
 *
 *	Given a two-part variable name, which may refer either to a scalar
 *	variable or an element of an array, change the value of the variable.
 *	If the named scalar or array or element doesn't exist then create one.
 *
 * Results:
 *	Returns a pointer to the malloc'ed string which is the character
 *	representation of the variable's new value. The caller must not modify
 *	this string. If the write operation was disallowed because an array
 *	was expected but not found (or vice versa), then NULL is returned; if
 *	the TCL_LEAVE_ERR_MSG flag is set, then an explanatory message will be
 *	left in the interp's result. Note that the returned string may not be
 *	the same as newValue; this is because variable traces may modify the
 *	variable's value.
 *
 * Side effects:
 *	The value of the given variable is set. If either the array or the
 *	entry didn't exist then a new one is created.
 *
 *----------------------------------------------------------------------
 */

const char *
Tcl_SetVar2(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be looked up. */
    const char *part1,		/* If part2 is NULL, this is name of scalar
				 * variable. Otherwise it is the name of an
				 * array. */
    const char *part2,		/* Name of an element within an array, or
				 * NULL. */
    const char *newValue,	/* New value for variable. */
    int flags)			/* Various flags that tell how to set value:

```

tcl8.5 8.5.19, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclIndexObj.c`, function `Tcl_WrongNumArgs`, lines 441–557. Retained reference `native_error_code_metadata_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclIndexObj.c`.

```text
 *	in the interpreter to generate complex multi-part messages by calling
 *	this function repeatedly. This allows the code that knows how to
 *	handle ensemble-related error messages to be kept here while still
 *	generating suitable error messages for commands like [read] and
 *	[socket]. Ideally, this would be done through an extra flags argument,
 *	but that wouldn't be source-compatible with the existing API and it's
 *	a fairly rare requirement anyway.
 *
 *----------------------------------------------------------------------
 */

void
Tcl_WrongNumArgs(
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments to print from objv. */
    Tcl_Obj *const objv[],	/* Initial argument objects, which should be
				 * included in the error message. */
    const char *message)	/* Error message to print after the leading
				 * objects in objv. The message may be
				 * NULL. */
{
    Tcl_Obj *objPtr;
    int i, len, elemLen, flags;
    Interp *iPtr = (Interp *) interp;
    const char *elementStr;

    /*
     * [incr Tcl] does something fairly horrific when generating error
     * messages for its ensembles; it passes the whole set of ensemble
     * arguments as a list in the first argument. This means that this code
     * causes a problem in iTcl if it attempts to correctly quote all
     * arguments, which would be the correct thing to do. We work around this
     * nasty behaviour for now, and hope that we can remove it all in the
     * future...
     */

#ifndef AVOID_HACKS_FOR_ITCL
    int isFirst = 1;		/* Special flag used to inhibit the treating
				 * of the first word as a list element so the
				 * hacky way Itcl generates error messages for
				 * its ensembles will still work. [Bug
				 * 1066837] */
#   define MAY_QUOTE_WORD	(!isFirst)
#   define AFTER_FIRST_WORD	(isFirst = 0)
#else /* !AVOID_HACKS_FOR_ITCL */
#   define MAY_QUOTE_WORD	1
#   define AFTER_FIRST_WORD	(void) 0
#endif /* AVOID_HACKS_FOR_ITCL */

    TclNewObj(objPtr);
    if (iPtr->flags & INTERP_ALTERNATE_WRONG_ARGS) {
	Tcl_AppendObjToObj(objPtr, Tcl_GetObjResult(interp));
	Tcl_AppendToObj(objPtr, " or \"", -1);
    } else {
	Tcl_AppendToObj(objPtr, "wrong # args: should be \"", -1);
    }

    /*
     * Check to see if we are processing an ensemble implementation, and if so
     * rewrite the results in terms of how the ensemble was invoked.
     */

    if (iPtr->ensembleRewrite.sourceObjs != NULL) {
	int toSkip = iPtr->ensembleRewrite.numInsertedObjs;
	int toPrint = iPtr->ensembleRewrite.numRemovedObjs;
	Tcl_Obj *const *origObjv = iPtr->ensembleRewrite.sourceObjs;

	/*
	 * We only know how to do rewriting if all the replaced objects are
	 * actually arguments (in objv) to this function. Otherwise it just
	 * gets too complicated and we'd be better off just giving a slightly
	 * confusing error message...
	 */

	if (objc < toSkip) {
	    goto addNormalArgumentsToMessage;
	}

	/*
	 * Strip out the actual arguments that the ensemble inserted.
	 */

	objv += toSkip;
	objc -= toSkip;

	/*
	 * We assume no object is of index type.
	 */

	for (i=0 ; i<toPrint ; i++) {
	    /*
	     * Add the element, quoting it if necessary.
	     */

	    if (origObjv[i]->typePtr == &indexType) {
		register IndexRep *indexRep =
			origObjv[i]->internalRep.twoPtrValue.ptr1;

		elementStr = EXPAND_OF(indexRep);
		elemLen = strlen(elementStr);
	    } else if (origObjv[i]->typePtr == &tclEnsembleCmdType) {
		register EnsembleCmdRep *ecrPtr =
			origObjv[i]->internalRep.twoPtrValue.ptr1;

		elementStr = ecrPtr->fullSubcmdName;
		elemLen = strlen(elementStr);
	    } else {
		elementStr = TclGetStringFromObj(origObjv[i], &elemLen);
	    }
	    flags = 0;
	    len = TclScanElement(elementStr, elemLen, &flags);

	    if (MAY_QUOTE_WORD && len != elemLen) {
		char *quotedElementStr = TclStackAlloc(interp,
			(unsigned)len + 1);

		len = TclConvertElement(elementStr, elemLen,

```

tcl8.6 8.6.18, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdAH.c`, function `Tcl_ErrorObjCmd`, lines 874–990. Retained reference `native_encoding_utf8_offset307-8.6.18-tclCmdAH.c`.

```text
 *	user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl object result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_ErrorObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *options, *optName;

    if ((objc < 2) || (objc > 4)) {
	Tcl_WrongNumArgs(interp, 1, objv, "message ?errorInfo? ?errorCode?");
	return TCL_ERROR;
    }

    TclNewLiteralStringObj(options, "-code error -level 0");

    if (objc >= 3) {		/* Process the optional info argument */
	TclNewLiteralStringObj(optName, "-errorinfo");
	Tcl_ListObjAppendElement(NULL, options, optName);
	Tcl_ListObjAppendElement(NULL, options, objv[2]);
    }

    if (objc >= 4) {		/* Process the optional code argument */
	TclNewLiteralStringObj(optName, "-errorcode");
	Tcl_ListObjAppendElement(NULL, options, optName);
	Tcl_ListObjAppendElement(NULL, options, objv[3]);
    }

    Tcl_SetObjResult(interp, objv[1]);
    return Tcl_SetReturnOptions(interp, options);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_EvalObjCmd --
 *
 *	This object-based procedure is invoked to process the "eval" Tcl
 *	command. See the user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl object result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

static int
EvalCmdErrMsg(
    ClientData data[],
    Tcl_Interp *interp,
    int result)
{
    if (result == TCL_ERROR) {
	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (\"eval\" body line %d)", Tcl_GetErrorLine(interp)));
    }
    return result;
}

int
Tcl_EvalObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    return Tcl_NRCallObjProc(interp, TclNREvalObjCmd, dummy, objc, objv);
}

int
TclNREvalObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *objPtr;
    Interp *iPtr = (Interp *) interp;
    CmdFrame *invoker = NULL;
    int word = 0;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "arg ?arg ...?");
	return TCL_ERROR;
    }

    if (objc == 2) {
	/*
	 * TIP #280. Make argument location available to eval'd script.
	 */

	invoker = iPtr->cmdFramePtr;
	word = 1;
	objPtr = objv[1];
	TclArgumentGet(interp, objPtr, &invoker, &word);
    } else {
	/*
	 * More than one argument: concatenate them together with spaces
	 * between, then evaluate the result. Tcl_EvalObjEx will delete the
	 * object when it decrements its refcount after eval'ing it.
	 *
	 * TIP #280. Make invoking context available to eval'd script, done
	 * with the default values.

```

tcl8.6 8.6.18, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclVar.c`, function `Tcl_SetObjCmd`, lines 1497–1613. Retained reference `native_dict_update_write_error_lifetime398-request-8.6.18-original-tclVar.c`.

```text
 *	user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl result value.
 *
 * Side effects:
 *	A variable's value may be changed.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_SetObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *varValueObj;

    if (objc == 2) {
	varValueObj = Tcl_ObjGetVar2(interp, objv[1], NULL,TCL_LEAVE_ERR_MSG);
	if (varValueObj == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, varValueObj);
	return TCL_OK;
    } else if (objc == 3) {
	varValueObj = Tcl_ObjSetVar2(interp, objv[1], NULL, objv[2],
		TCL_LEAVE_ERR_MSG);
	if (varValueObj == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, varValueObj);
	return TCL_OK;
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "varName ?newValue?");
	return TCL_ERROR;
    }
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_SetVar --
 *
 *	Change the value of a variable.
 *
 * Results:
 *	Returns a pointer to the malloc'ed string which is the character
 *	representation of the variable's new value. The caller must not modify
 *	this string. If the write operation was disallowed then NULL is
 *	returned; if the TCL_LEAVE_ERR_MSG flag is set, then an explanatory
 *	message will be left in the interp's result. Note that the returned
 *	string may not be the same as newValue; this is because variable
 *	traces may modify the variable's value.
 *
 * Side effects:
 *	If varName is defined as a local or global variable in interp, its
 *	value is changed to newValue. If varName isn't currently defined, then
 *	a new global variable by that name is created.
 *
 *----------------------------------------------------------------------
 */

#undef Tcl_SetVar
const char *
Tcl_SetVar(
    Tcl_Interp *interp,		/* Command interpreter in which varName is to
				 * be looked up. */
    const char *varName,	/* Name of a variable in interp. */
    const char *newValue,	/* New value for varName. */
    int flags)			/* Various flags that tell how to set value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
{
    Tcl_Obj *varValuePtr, *varNamePtr = Tcl_NewStringObj(varName, -1);

    Tcl_IncrRefCount(varNamePtr);
    varValuePtr = Tcl_ObjSetVar2(interp, varNamePtr, NULL,
	    Tcl_NewStringObj(newValue, -1), flags);
    Tcl_DecrRefCount(varNamePtr);

    if (varValuePtr == NULL) {
	return NULL;
    }
    return TclGetString(varValuePtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_SetVar2 --
 *
 *	Given a two-part variable name, which may refer either to a scalar
 *	variable or an element of an array, change the value of the variable.
 *	If the named scalar or array or element doesn't exist then create one.
 *
 * Results:
 *	Returns a pointer to the malloc'ed string which is the character
 *	representation of the variable's new value. The caller must not modify
 *	this string. If the write operation was disallowed because an array
 *	was expected but not found (or vice versa), then NULL is returned; if
 *	the TCL_LEAVE_ERR_MSG flag is set, then an explanatory message will be
 *	left in the interp's result. Note that the returned string may not be
 *	the same as newValue; this is because variable traces may modify the
 *	variable's value.
 *
 * Side effects:
 *	The value of the given variable is set. If either the array or the
 *	entry didn't exist then a new one is created.
 *
 *----------------------------------------------------------------------
 */

const char *

```

tcl8.6 8.6.18, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclIndexObj.c`, function `Tcl_WrongNumArgs`, lines 826–942. Retained reference `native_error_code_metadata_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclIndexObj.c`.

```text
 *	in the interpreter to generate complex multi-part messages by calling
 *	this function repeatedly. This allows the code that knows how to
 *	handle ensemble-related error messages to be kept here while still
 *	generating suitable error messages for commands like [read] and
 *	[socket]. Ideally, this would be done through an extra flags argument,
 *	but that wouldn't be source-compatible with the existing API and it's
 *	a fairly rare requirement anyway.
 *
 *----------------------------------------------------------------------
 */

void
Tcl_WrongNumArgs(
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments to print from objv. */
    Tcl_Obj *const objv[],	/* Initial argument objects, which should be
				 * included in the error message. */
    const char *message)	/* Error message to print after the leading
				 * objects in objv. The message may be
				 * NULL. */
{
    Tcl_Obj *objPtr;
    int i, len, elemLen;
    char flags;
    Interp *iPtr = (Interp *)interp;
    const char *elementStr;

    /*
     * [incr Tcl] does something fairly horrific when generating error
     * messages for its ensembles; it passes the whole set of ensemble
     * arguments as a list in the first argument. This means that this code
     * causes a problem in iTcl if it attempts to correctly quote all
     * arguments, which would be the correct thing to do. We work around this
     * nasty behaviour for now, and hope that we can remove it all in the
     * future...
     */

#ifndef AVOID_HACKS_FOR_ITCL
    int isFirst = 1;		/* Special flag used to inhibit the treating
				 * of the first word as a list element so the
				 * hacky way Itcl generates error messages for
				 * its ensembles will still work. [Bug
				 * 1066837] */
#   define MAY_QUOTE_WORD	(!isFirst)
#   define AFTER_FIRST_WORD	(isFirst = 0)
#else /* !AVOID_HACKS_FOR_ITCL */
#   define MAY_QUOTE_WORD	1
#   define AFTER_FIRST_WORD	(void) 0
#endif /* AVOID_HACKS_FOR_ITCL */

    TclNewObj(objPtr);
    if (iPtr->flags & INTERP_ALTERNATE_WRONG_ARGS) {
	iPtr->flags &= ~INTERP_ALTERNATE_WRONG_ARGS;
	Tcl_AppendObjToObj(objPtr, Tcl_GetObjResult(interp));
	Tcl_AppendToObj(objPtr, " or \"", -1);
    } else {
	Tcl_AppendToObj(objPtr, "wrong # args: should be \"", -1);
    }

    /*
     * If processing an an ensemble implementation, rewrite the results in
     * terms of how the ensemble was invoked.
     */

    if (iPtr->ensembleRewrite.sourceObjs != NULL) {
	int toSkip = iPtr->ensembleRewrite.numInsertedObjs;
	int toPrint = iPtr->ensembleRewrite.numRemovedObjs;
	Tcl_Obj *const *origObjv = TclEnsembleGetRewriteValues(interp);

	/*
	 * Only do rewrite the command if all the replaced objects are
	 * actually arguments (in objv) to this function. Otherwise it just
	 * gets too complicated and it's to just give a slightly
	 * confusing error message...
	 */

	if (objc < toSkip) {
	    goto addNormalArgumentsToMessage;
	}

	/*
	 * Strip out the actual arguments that the ensemble inserted.
	 */

	objv += toSkip;
	objc -= toSkip;

	/*
	 * Assume no object is of index type.
	 */

	for (i=0 ; i<toPrint ; i++) {
	    /*
	     * Add the element, quoting it if necessary.
	     */

	    if (origObjv[i]->typePtr == &indexType) {
		IndexRep *indexRep =
			(IndexRep *)origObjv[i]->internalRep.twoPtrValue.ptr1;

		elementStr = EXPAND_OF(indexRep);
		elemLen = strlen(elementStr);
	    } else {
		elementStr = TclGetStringFromObj(origObjv[i], &elemLen);
	    }
	    flags = 0;
	    len = TclScanElement(elementStr, elemLen, &flags);

	    if (MAY_QUOTE_WORD && len != elemLen) {
		char *quotedElementStr = (char *)TclStackAlloc(interp, len + 1);

		len = TclConvertElement(elementStr, elemLen,
			quotedElementStr, flags);
		Tcl_AppendToObj(objPtr, quotedElementStr, len);
		TclStackFree(interp, quotedElementStr);
	    } else {
		Tcl_AppendToObj(objPtr, elementStr, elemLen);

```

tcl9.0 9.0.4, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdAH.c`, function `Tcl_ErrorObjCmd`, lines 875–991. Retained reference `native_encoding_utf8_offset307-9.0.4-tclCmdAH.c`.

```text
 *	user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl object result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_ErrorObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *options, *optName;

    if ((objc < 2) || (objc > 4)) {
	Tcl_WrongNumArgs(interp, 1, objv, "message ?errorInfo? ?errorCode?");
	return TCL_ERROR;
    }

    TclNewLiteralStringObj(options, "-code error -level 0");

    if (objc >= 3) {		/* Process the optional info argument */
	TclNewLiteralStringObj(optName, "-errorinfo");
	Tcl_ListObjAppendElement(NULL, options, optName);
	Tcl_ListObjAppendElement(NULL, options, objv[2]);
    }

    if (objc >= 4) {		/* Process the optional code argument */
	TclNewLiteralStringObj(optName, "-errorcode");
	Tcl_ListObjAppendElement(NULL, options, optName);
	Tcl_ListObjAppendElement(NULL, options, objv[3]);
    }

    Tcl_SetObjResult(interp, objv[1]);
    return Tcl_SetReturnOptions(interp, options);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_EvalObjCmd --
 *
 *	This object-based procedure is invoked to process the "eval" Tcl
 *	command. See the user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl object result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

static int
EvalCmdErrMsg(
    TCL_UNUSED(void **),
    Tcl_Interp *interp,
    int result)
{
    if (result == TCL_ERROR) {
	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (\"eval\" body line %d)", Tcl_GetErrorLine(interp)));
    }
    return result;
}

int
Tcl_EvalObjCmd(
    void *clientData,
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    return Tcl_NRCallObjProc(interp, TclNREvalObjCmd, clientData, objc, objv);
}

int
TclNREvalObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *objPtr;
    Interp *iPtr = (Interp *) interp;
    CmdFrame *invoker = NULL;
    int word = 0;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "arg ?arg ...?");
	return TCL_ERROR;
    }

    if (objc == 2) {
	/*
	 * TIP #280. Make argument location available to eval'd script.
	 */

	invoker = iPtr->cmdFramePtr;
	word = 1;
	objPtr = objv[1];
	TclArgumentGet(interp, objPtr, &invoker, &word);
    } else {
	/*
	 * More than one argument: concatenate them together with spaces
	 * between, then evaluate the result. Tcl_EvalObjEx will delete the
	 * object when it decrements its refcount after eval'ing it.
	 *
	 * TIP #280. Make invoking context available to eval'd script, done
	 * with the default values.

```

tcl9.0 9.0.4, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclVar.c`, function `Tcl_SetObjCmd`, lines 1485–1601. Retained reference `native_dict_update_write_error_lifetime398-request-9.0.4-original-tclVar.c`.

```text
 *	user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl result value.
 *
 * Side effects:
 *	A variable's value may be changed.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_SetObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *varValueObj;

    if (objc == 2) {
	varValueObj = Tcl_ObjGetVar2(interp, objv[1], NULL,TCL_LEAVE_ERR_MSG);
	if (varValueObj == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, varValueObj);
	return TCL_OK;
    } else if (objc == 3) {
	varValueObj = Tcl_ObjSetVar2(interp, objv[1], NULL, objv[2],
		TCL_LEAVE_ERR_MSG);
	if (varValueObj == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, varValueObj);
	return TCL_OK;
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "varName ?newValue?");
	return TCL_ERROR;
    }
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_SetVar2 --
 *
 *	Given a two-part variable name, which may refer either to a scalar
 *	variable or an element of an array, change the value of the variable.
 *	If the named scalar or array or element doesn't exist then create one.
 *
 * Results:
 *	Returns a pointer to the malloc'ed string which is the character
 *	representation of the variable's new value. The caller must not modify
 *	this string. If the write operation was disallowed because an array
 *	was expected but not found (or vice versa), then NULL is returned; if
 *	the TCL_LEAVE_ERR_MSG flag is set, then an explanatory message will be
 *	left in the interp's result. Note that the returned string may not be
 *	the same as newValue; this is because variable traces may modify the
 *	variable's value.
 *
 * Side effects:
 *	The value of the given variable is set. If either the array or the
 *	entry didn't exist then a new one is created.
 *
 *----------------------------------------------------------------------
 */

const char *
Tcl_SetVar2(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be looked up. */
    const char *part1,		/* If part2 is NULL, this is name of scalar
				 * variable. Otherwise it is the name of an
				 * array. */
    const char *part2,		/* Name of an element within an array, or
				 * NULL. */
    const char *newValue,	/* New value for variable. */
    int flags)			/* Various flags that tell how to set value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT, or
				 * TCL_LEAVE_ERR_MSG. */
{
    Tcl_Obj *varValuePtr = Tcl_SetVar2Ex(interp, part1, part2,
	    Tcl_NewStringObj(newValue, -1), flags);

    if (varValuePtr == NULL) {
	return NULL;
    }
    return TclGetString(varValuePtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_SetVar2Ex --
 *
 *	Given a two-part variable name, which may refer either to a scalar
 *	variable or an element of an array, change the value of the variable
 *	to a new Tcl object value. If the named scalar or array or element
 *	doesn't exist then create one.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the write operation was disallowed because an array was
 *	expected but not found (or vice versa), then NULL is returned; if the
 *	TCL_LEAVE_ERR_MSG flag is set, then an explanatory message will be
 *	left in the interpreter's result. Note that the returned object may
 *	not be the same one referenced by newValuePtr; this is because
 *	variable traces may modify the variable's value.
 *
 * Side effects:
 *	The value of the given variable is set. If either the array or the
 *	entry didn't exist then a new variable is created.
 *
 *	The reference count is decremented for any old value of the variable
 *	and incremented for its new value. If the new value for the variable
 *	is not the same one referenced by newValuePtr (perhaps as a result of

```

tcl9.0 9.0.4, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclIndexObj.c`, function `Tcl_WrongNumArgs`, lines 796–912. Retained reference `native_error_code_metadata_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclIndexObj.c`.

```text
 *	in the interpreter to generate complex multi-part messages by calling
 *	this function repeatedly. This allows the code that knows how to
 *	handle ensemble-related error messages to be kept here while still
 *	generating suitable error messages for commands like [read] and
 *	[socket]. Ideally, this would be done through an extra flags argument,
 *	but that wouldn't be source-compatible with the existing API and it's
 *	a fairly rare requirement anyway.
 *
 *----------------------------------------------------------------------
 */

void
Tcl_WrongNumArgs(
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,			/* Number of arguments to print from objv. */
    Tcl_Obj *const objv[],	/* Initial argument objects, which should be
				 * included in the error message. */
    const char *message)	/* Error message to print after the leading
				 * objects in objv. The message may be
				 * NULL. */
{
    Tcl_Obj *objPtr;
    Tcl_Size i, len, elemLen;
    char flags;
    Interp *iPtr = (Interp *)interp;
    const char *elementStr;

    TclNewObj(objPtr);
    if (iPtr->flags & INTERP_ALTERNATE_WRONG_ARGS) {
	iPtr->flags &= ~INTERP_ALTERNATE_WRONG_ARGS;
	Tcl_AppendObjToObj(objPtr, Tcl_GetObjResult(interp));
	Tcl_AppendToObj(objPtr, " or \"", TCL_INDEX_NONE);
    } else {
	Tcl_AppendToObj(objPtr, "wrong # args: should be \"", TCL_INDEX_NONE);
    }

    /*
     * If processing an ensemble implementation, rewrite the results in
     * terms of how the ensemble was invoked.
     */

    if (iPtr->ensembleRewrite.sourceObjs != NULL) {
	Tcl_Size toSkip = iPtr->ensembleRewrite.numInsertedObjs;
	Tcl_Size toPrint = iPtr->ensembleRewrite.numRemovedObjs;
	Tcl_Obj *const *origObjv = TclEnsembleGetRewriteValues(interp);

	/*
	 * Only do rewrite the command if all the replaced objects are
	 * actually arguments (in objv) to this function. Otherwise it just
	 * gets too complicated and it's to just give a slightly
	 * confusing error message...
	 */

	if (objc < toSkip) {
	    goto addNormalArgumentsToMessage;
	}

	/*
	 * Strip out the actual arguments that the ensemble inserted.
	 */

	objv += toSkip;
	objc -= toSkip;

	/*
	 * Assume no object is of index type.
	 */

	for (i=0 ; i<toPrint ; i++) {
	    /*
	     * Add the element, quoting it if necessary.
	     */
	    const Tcl_ObjInternalRep *irPtr;

	    if ((irPtr = TclFetchInternalRep(origObjv[i], &tclIndexType))) {
		IndexRep *indexRep = (IndexRep *)irPtr->twoPtrValue.ptr1;

		elementStr = EXPAND_OF(indexRep);
		elemLen = strlen(elementStr);
	    } else {
		elementStr = TclGetStringFromObj(origObjv[i], &elemLen);
	    }
	    flags = 0;
	    len = TclScanElement(elementStr, elemLen, &flags);

	    if (len != elemLen) {
		char *quotedElementStr = (char *)TclStackAlloc(interp, len + 1);

		len = TclConvertElement(elementStr, elemLen,
			quotedElementStr, flags);
		Tcl_AppendToObj(objPtr, quotedElementStr, len);
		TclStackFree(interp, quotedElementStr);
	    } else {
		Tcl_AppendToObj(objPtr, elementStr, elemLen);
	    }

	    /*
	     * Add a space if the word is not the last one (which has a
	     * moderately complex condition here).
	     */

	    if (i + 1 < toPrint || objc!=0 || message!=NULL) {
		Tcl_AppendStringsToObj(objPtr, " ", (char *)NULL);
	    }
	}
    }

    /*
     * Now add the arguments (other than those rewritten) that the caller took
     * from its calling context.
     */

  addNormalArgumentsToMessage:
    for (i = 0; i < objc; i++) {
	/*
	 * If the object is an index type, use the index table which allows for
	 * the correct error message even if the subcommand was abbreviated.

```

tcl9.1 9.1.0, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdAH.c`, function `Tcl_ErrorObjCmd`, lines 896–1012. Retained reference `native_encoding_utf8_offset307-9.1.0-tclCmdAH.c`.

```text
 *	user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl object result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_ErrorObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Obj *options, *optName;

    if ((objc < 2) || (objc > 4)) {
	Tcl_WrongNumArgs(interp, 1, objv, "message ?errorInfo? ?errorCode?");
	return TCL_ERROR;
    }

    TclNewLiteralStringObj(options, "-code error -level 0");

    if (objc >= 3) {		/* Process the optional info argument */
	TclNewLiteralStringObj(optName, "-errorinfo");
	Tcl_ListObjAppendElement(NULL, options, optName);
	Tcl_ListObjAppendElement(NULL, options, objv[2]);
    }

    if (objc >= 4) {		/* Process the optional code argument */
	TclNewLiteralStringObj(optName, "-errorcode");
	Tcl_ListObjAppendElement(NULL, options, optName);
	Tcl_ListObjAppendElement(NULL, options, objv[3]);
    }

    Tcl_SetObjResult(interp, objv[1]);
    return Tcl_SetReturnOptions(interp, options);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_EvalObjCmd --
 *
 *	This object-based procedure is invoked to process the "eval" Tcl
 *	command. See the user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl object result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

static int
EvalCmdErrMsg(
    TCL_UNUSED(void **),
    Tcl_Interp *interp,
    int result)
{
    if (result == TCL_ERROR) {
	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (\"eval\" body line %d)", Tcl_GetErrorLine(interp)));
    }
    return result;
}

int
Tcl_EvalObjCmd(
    void *clientData,
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    return Tcl_NRCallObjProc2(interp, TclNREvalObjCmd, clientData, objc, objv);
}

int
TclNREvalObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Obj *objPtr;
    Interp *iPtr = (Interp *) interp;
    CmdFrame *invoker = NULL;
    Tcl_Size word = 0;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "arg ?arg ...?");
	return TCL_ERROR;
    }

    if (objc == 2) {
	/*
	 * TIP #280. Make argument location available to eval'd script.
	 */

	invoker = iPtr->cmdFramePtr;
	word = 1;
	objPtr = objv[1];
	TclArgumentGet(interp, objPtr, &invoker, &word);
    } else {
	/*
	 * More than one argument: concatenate them together with spaces
	 * between, then evaluate the result. Tcl_EvalObjEx will delete the
	 * object when it decrements its refcount after eval'ing it.
	 *
	 * TIP #280. Make invoking context available to eval'd script, done
	 * with the default values.

```

tcl9.1 9.1.0, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclVar.c`, function `Tcl_SetObjCmd`, lines 1539–1655. Retained reference `native_dict_update_write_error_lifetime398-request-9.1.0-original-tclVar.c`.

```text
 *	user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl result value.
 *
 * Side effects:
 *	A variable's value may be changed.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_SetObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Obj *varValueObj;

    if (objc == 2) {
	varValueObj = Tcl_ObjGetVar2(interp, objv[1], NULL,TCL_LEAVE_ERR_MSG);
	if (varValueObj == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, varValueObj);
	return TCL_OK;
    } else if (objc == 3) {
	varValueObj = Tcl_ObjSetVar2(interp, objv[1], NULL, objv[2],
		TCL_LEAVE_ERR_MSG);
	if (varValueObj == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, varValueObj);
	return TCL_OK;
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "varName ?newValue?");
	return TCL_ERROR;
    }
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_SetVar2 --
 *
 *	Given a two-part variable name, which may refer either to a scalar
 *	variable or an element of an array, change the value of the variable.
 *	If the named scalar or array or element doesn't exist then create one.
 *
 * Results:
 *	Returns a pointer to the malloc'ed string which is the character
 *	representation of the variable's new value. The caller must not modify
 *	this string. If the write operation was disallowed because an array
 *	was expected but not found (or vice versa), then NULL is returned; if
 *	the TCL_LEAVE_ERR_MSG flag is set, then an explanatory message will be
 *	left in the interp's result. Note that the returned string may not be
 *	the same as newValue; this is because variable traces may modify the
 *	variable's value.
 *
 * Side effects:
 *	The value of the given variable is set. If either the array or the
 *	entry didn't exist then a new one is created.
 *
 *----------------------------------------------------------------------
 */

const char *
Tcl_SetVar2(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be looked up. */
    const char *part1,		/* If part2 is NULL, this is name of scalar
				 * variable. Otherwise it is the name of an
				 * array. */
    const char *part2,		/* Name of an element within an array, or
				 * NULL. */
    const char *newValue,	/* New value for variable. */
    int flags)			/* Various flags that tell how to set value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT, or
				 * TCL_LEAVE_ERR_MSG. */
{
    Tcl_Obj *varValuePtr = Tcl_SetVar2Ex(interp, part1, part2,
	    Tcl_NewStringObj(newValue, -1), flags);

    if (varValuePtr == NULL) {
	return NULL;
    }
    return TclGetString(varValuePtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_SetVar2Ex --
 *
 *	Given a two-part variable name, which may refer either to a scalar
 *	variable or an element of an array, change the value of the variable
 *	to a new Tcl object value. If the named scalar or array or element
 *	doesn't exist then create one.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the write operation was disallowed because an array was
 *	expected but not found (or vice versa), then NULL is returned; if the
 *	TCL_LEAVE_ERR_MSG flag is set, then an explanatory message will be
 *	left in the interpreter's result. Note that the returned object may
 *	not be the same one referenced by newValuePtr; this is because
 *	variable traces may modify the variable's value.
 *
 * Side effects:
 *	The value of the given variable is set. If either the array or the
 *	entry didn't exist then a new variable is created.
 *
 *	The reference count is decremented for any old value of the variable
 *	and incremented for its new value. If the new value for the variable
 *	is not the same one referenced by newValuePtr (perhaps as a result of

```

tcl9.1 9.1.0, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclIndexObj.c`, function `Tcl_WrongNumArgs`, lines 798–914. Retained reference `native_error_code_metadata_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclIndexObj.c`.

```text
 *	in the interpreter to generate complex multi-part messages by calling
 *	this function repeatedly. This allows the code that knows how to
 *	handle ensemble-related error messages to be kept here while still
 *	generating suitable error messages for commands like [read] and
 *	[socket]. Ideally, this would be done through an extra flags argument,
 *	but that wouldn't be source-compatible with the existing API and it's
 *	a fairly rare requirement anyway.
 *
 *----------------------------------------------------------------------
 */

void
Tcl_WrongNumArgs(
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments to print from objv. */
    Tcl_Obj *const *objv,	/* Initial argument objects, which should be
				 * included in the error message. */
    const char *message)	/* Error message to print after the leading
				 * objects in objv. The message may be
				 * NULL. */
{
    Tcl_Obj *objPtr;
    Tcl_Size i, len, elemLen;
    char flags;
    Interp *iPtr = (Interp *)interp;
    const char *elementStr;

    TclNewObj(objPtr);
    if (iPtr->flags & INTERP_ALTERNATE_WRONG_ARGS) {
	iPtr->flags &= ~INTERP_ALTERNATE_WRONG_ARGS;
	Tcl_AppendObjToObj(objPtr, Tcl_GetObjResult(interp));
	Tcl_AppendToObj(objPtr, " or \"", TCL_INDEX_NONE);
    } else {
	Tcl_AppendToObj(objPtr, "wrong # args: should be \"", TCL_INDEX_NONE);
    }

    /*
     * If processing an ensemble implementation, rewrite the results in
     * terms of how the ensemble was invoked.
     */

    if (iPtr->ensembleRewrite.sourceObjs != NULL) {
	Tcl_Size toSkip = iPtr->ensembleRewrite.numInsertedObjs;
	Tcl_Size toPrint = iPtr->ensembleRewrite.numRemovedObjs;
	Tcl_Obj *const *origObjv = TclEnsembleGetRewriteValues(interp);

	/*
	 * Only do rewrite the command if all the replaced objects are
	 * actually arguments (in objv) to this function. Otherwise it just
	 * gets too complicated and it's to just give a slightly
	 * confusing error message...
	 */

	if (objc < toSkip) {
	    goto addNormalArgumentsToMessage;
	}

	/*
	 * Strip out the actual arguments that the ensemble inserted.
	 */

	objv += toSkip;
	objc -= toSkip;

	/*
	 * Assume no object is of index type.
	 */

	for (i=0 ; i<toPrint ; i++) {
	    /*
	     * Add the element, quoting it if necessary.
	     */
	    const Tcl_ObjInternalRep *irPtr;

	    if ((irPtr = TclFetchInternalRep(origObjv[i], &tclIndexType))) {
		IndexRep *indexRep = (IndexRep *)irPtr->twoPtrValue.ptr1;

		elementStr = EXPAND_OF(indexRep);
		elemLen = strlen(elementStr);
	    } else {
		elementStr = TclGetStringFromObj(origObjv[i], &elemLen);
	    }
	    flags = 0;
	    len = TclScanElement(elementStr, elemLen, &flags);

	    if (len != elemLen) {
		char *quotedElementStr = (char *)TclStackAlloc(interp, len + 1);

		len = TclConvertElement(elementStr, elemLen,
			quotedElementStr, flags);
		Tcl_AppendToObj(objPtr, quotedElementStr, len);
		TclStackFree(interp, quotedElementStr);
	    } else {
		Tcl_AppendToObj(objPtr, elementStr, elemLen);
	    }

	    /*
	     * Add a space if the word is not the last one (which has a
	     * moderately complex condition here).
	     */

	    if (i + 1 < toPrint || objc!=0 || message!=NULL) {
		Tcl_AppendStringsToObj(objPtr, " ", (char *)NULL);
	    }
	}
    }

    /*
     * Now add the arguments (other than those rewritten) that the caller took
     * from its calling context.
     */

  addNormalArgumentsToMessage:
    for (i = 0; i < objc; i++) {
	/*
	 * If the object is an index type, use the index table which allows for
	 * the correct error message even if the subcommand was abbreviated.

```

jim 0.84-9-g5bac7c9, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_ErrorCoreCommand`, lines 16582–16698. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
    }
    else {
        if (listPtr == (Jim_Obj *)EOF) {
            Jim_SetResult(interp, Jim_NewListObj(interp, 0, 0));
            return JIM_OK;
        }
        Jim_SetResult(interp, listPtr);
    }
    return JIM_OK;
}

/* [error] */
static int Jim_ErrorCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_SetResult(interp, argv[1]);
    if (argc == 3) {
        JimSetStackTrace(interp, argv[2]);
        return JIM_ERR;
    }
    return JIM_ERR;
}

/* [lrange] */
static int Jim_LrangeCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_Obj *objPtr;

    if ((objPtr = Jim_ListRange(interp, argv[1], argv[2], argv[3])) == NULL)
        return JIM_ERR;
    Jim_SetResult(interp, objPtr);
    return JIM_OK;
}

/* [lrepeat] */
static int Jim_LrepeatCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_Obj *objPtr;
    jim_wide count;

    if (argc < 2 || Jim_GetWideExpr(interp, argv[1], &count) != JIM_OK || count < 0) {
        return JIM_USAGE;
    }
    if (count == 0 || argc == 2) {
        Jim_SetEmptyResult(interp);
        return JIM_OK;
    }

    argc -= 2;
    argv += 2;

    objPtr = Jim_NewListObj(interp, NULL, 0);
    ListEnsureLength(objPtr, argc * count);
    while (count--) {
        ListInsertElements(objPtr, -1, argc, argv);
    }

    Jim_SetResult(interp, objPtr);
    return JIM_OK;
}

char **Jim_GetEnviron(void)
{
#if defined(HAVE__NSGETENVIRON)
    return *_NSGetEnviron();
#elif defined(_environ)
    return _environ;
#else
    #if !defined(NO_ENVIRON_EXTERN)
    extern char **environ;
    #endif
    return environ;
#endif
}

void Jim_SetEnviron(char **env)
{
#if defined(HAVE__NSGETENVIRON)
    *_NSGetEnviron() = env;
#elif defined(_environ)
    _environ = env;
#else
    #if !defined(NO_ENVIRON_EXTERN)
    extern char **environ;
    #endif

    environ = env;
#endif
}

/* [env] */
static int Jim_EnvCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    const char *key;
    const char *val;

    if (argc == 1) {
        char **e = Jim_GetEnviron();

        int i;
        Jim_Obj *listObjPtr = Jim_NewListObj(interp, NULL, 0);

        for (i = 0; e[i]; i++) {
            const char *equals = strchr(e[i], '=');

            if (equals) {
                Jim_ListAppendElement(interp, listObjPtr, Jim_NewStringObj(interp, e[i],
                        equals - e[i]));
                Jim_ListAppendElement(interp, listObjPtr, Jim_NewStringObj(interp, equals + 1, -1));
            }
        }

        Jim_SetResult(interp, listObjPtr);
        return JIM_OK;
    }

    key = Jim_String(argv[1]);
    val = getenv(key);

```

jim 0.84-9-g5bac7c9, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_SetCoreCommand`, lines 12719–12835. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
static int Jim_SubCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    return JimSubDivHelper(interp, argc, argv, JIM_EXPROP_SUB);
}

/* [/] */
static int Jim_DivCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    return JimSubDivHelper(interp, argc, argv, JIM_EXPROP_DIV);
}

/* [set] */
static int Jim_SetCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    if (argc == 2) {
        Jim_Obj *objPtr;

        objPtr = Jim_GetVariable(interp, argv[1], JIM_ERRMSG);
        if (!objPtr)
            return JIM_ERR;
        Jim_SetResult(interp, objPtr);
        return JIM_OK;
    }
    /* argc == 3 case. */
    if (Jim_SetVariable(interp, argv[1], argv[2]) != JIM_OK)
        return JIM_ERR;
    Jim_SetResult(interp, argv[2]);
    return JIM_OK;
}

/* [unset]
 *
 * unset ?-nocomplain? ?--? ?varName ...?
 */
static int Jim_UnsetCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int i = 1;
    int complain = 1;

    while (i < argc) {
        if (Jim_CompareStringImmediate(interp, argv[i], "--")) {
            i++;
            break;
        }
        if (Jim_CompareStringImmediate(interp, argv[i], "-nocomplain")) {
            complain = 0;
            i++;
            continue;
        }
        break;
    }

    while (i < argc) {
        if (Jim_UnsetVariable(interp, argv[i], complain ? JIM_ERRMSG : JIM_NONE) != JIM_OK
            && complain) {
            return JIM_ERR;
        }
        i++;
    }

    Jim_SetEmptyResult(interp);
    return JIM_OK;
}

/**
 * All commands that support break, continue from a loop (while, loop, foreach, for)
 * use this to check for break_level.
 *
 * If break_level is > 0, decrements the break_level and returns 1.
 * Otherwise returns 0
 */
static int JimCheckLoopRetcode(Jim_Interp *interp, int retval)
{
    if (retval == JIM_BREAK || retval == JIM_CONTINUE) {
        if (--interp->break_level > 0) {
            return 1;
        }
    }
    return 0;
}

/* [while] */
static int Jim_WhileCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    /* The general purpose implementation of while starts here */
    while (1) {
        int boolean = 0, retval;

        if ((retval = Jim_GetBoolFromExpr(interp, argv[1], &boolean)) != JIM_OK)
            return retval;
        if (!boolean)
            break;

        if ((retval = Jim_EvalObj(interp, argv[2])) != JIM_OK) {
            if (JimCheckLoopRetcode(interp, retval)) {
                return retval;
            }
            switch (retval) {
                case JIM_BREAK:
                    goto out;
                case JIM_CONTINUE:
                    continue;
                default:
                    return retval;
            }
        }
    }
  out:
    Jim_SetEmptyResult(interp);
    return JIM_OK;
}

/* [for] */
static int Jim_ForCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int retval;
    int boolean = 1;

```

jim 0.84-9-g5bac7c9, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_WrongNumArgs`, lines 12324–12440. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
    Jim_IncrRefCount(listObjPtr);
    objPtr = Jim_ListJoin(interp, listObjPtr, " ", 1);
    Jim_DecrRefCount(interp, listObjPtr);
    Jim_IncrRefCount(objPtr);

    return objPtr;
}

 /* This function should not be necessary for simple commands.
  * Instead set minargs/maxargs and a usage string when registering the command and
  * optionally return JIM_USAGE from the command proc to generate a usage message.
  */
void Jim_WrongNumArgs(Jim_Interp *interp, int argc, Jim_Obj *const *argv, const char *msg)
{
    Jim_Obj *objPtr = JimJoinCmdArgs(interp, argc, argv);
    if (*msg) {
        Jim_SetResultFormatted(interp, "wrong # args: should be \"%#s %s\"", objPtr, msg);
    }
    else {
        Jim_SetResultFormatted(interp, "wrong # args: should be \"%#s\"", objPtr);
    }
    Jim_DecrRefCount(interp, objPtr);
}

/**
 * Calculates the taint of the given objects (skipping NULL entries).
 */
int Jim_CalcTaint(int argc, Jim_Obj *const *argv)
{
    int taint = 0;
#ifdef JIM_TAINT
    int i;
    for (i = 0; i < argc; i++) {
        if (argv[i]) {
            taint |= argv[i]->taint;
        }
    }
#endif
    return taint;
}

void Jim_SetTaintError(Jim_Interp *interp, int cmdargs, Jim_Obj *const *argv)
{
#ifdef JIM_TAINT
    Jim_Obj *objPtr = JimJoinCmdArgs(interp, cmdargs, argv);
    Jim_SetResultFormatted(interp, "%#s: tainted data", objPtr);
    Jim_DecrRefCount(interp, objPtr);
    Jim_SetGlobalVariableStr(interp, "errorCode", Jim_NewStringObj(interp, "TAINTED", -1));
#endif
}

/**
 * May add the key and/or value to the list.
 */
typedef void JimHashtableIteratorCallbackType(Jim_Interp *interp, Jim_Obj *listObjPtr,
    Jim_Obj *keyObjPtr, void *value, Jim_Obj *patternObjPtr, int type);

#define JimTrivialMatch(pattern)    (strpbrk((pattern), "*[?\\") == NULL)

/**
 * For each key of the hash table 'ht' with object keys that
 * matches the glob pattern (all if NULL), invoke the callback to add entries to a list.
 * Returns the list.
 */
static Jim_Obj *JimHashtablePatternMatch(Jim_Interp *interp, Jim_HashTable *ht, Jim_Obj *patternObjPtr,
    JimHashtableIteratorCallbackType *callback, int type)
{
    Jim_HashEntry *he;
    Jim_Obj *listObjPtr = Jim_NewListObj(interp, NULL, 0);

    /* Check for the non-pattern case. We can do this much more efficiently. */
    if (patternObjPtr && JimTrivialMatch(Jim_String(patternObjPtr))) {
        he = Jim_FindHashEntry(ht, patternObjPtr);
        if (he) {
            callback(interp, listObjPtr, Jim_GetHashEntryKey(he), Jim_GetHashEntryVal(he),
                patternObjPtr, type);
        }
    }
    else {
        Jim_HashTableIterator htiter;
        JimInitHashTableIterator(ht, &htiter);
        while ((he = Jim_NextHashEntry(&htiter)) != NULL) {
            callback(interp, listObjPtr, Jim_GetHashEntryKey(he), Jim_GetHashEntryVal(he),
                patternObjPtr, type);
        }
    }
    return listObjPtr;
}

/* Keep these in order */
#define JIM_CMDLIST_COMMANDS 1
#define JIM_CMDLIST_PROCS 2
#define JIM_CMDLIST_ALIASES 4
#define JIM_CMDLIST_CHANNELS 8

#define JIM_CMDLIST_ALL 0x1000

/**
 * Adds matching command names (procs, channels) to the list.
 */
static void JimCommandMatch(Jim_Interp *interp, Jim_Obj *listObjPtr,
    Jim_Obj *keyObj, void *value, Jim_Obj *patternObj, int type)
{
    Jim_Cmd *cmdPtr = (Jim_Cmd *)value;
    int match = 1;

    if ((type & JIM_CMDLIST_PROCS) && !(cmdPtr->flags & JIM_CMD_ISPROC)) {
        /* not a proc */
        return;
    }
    if ((type & JIM_CMDLIST_CHANNELS) && !(cmdPtr->flags & JIM_CMD_ISCHANNEL)) {
        /* not a channel */
        return;
    }
    if ((type & JIM_CMDLIST_ALIASES) && !(cmdPtr->flags & JIM_CMD_ISALIAS)) {
        /* not an alias */
        return;

```

jim 0.84-9-g5bac7c9, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `error command argc 1..2 registration`, lines 16837–16856. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
    {"apply", Jim_ApplyCoreCommand, 1, -1, "lambdaExpr ?arg ...?" },
    {"break", Jim_BreakCoreCommand, 0, 1, "?level?" },
    {"catch", Jim_CatchCoreCommand, 1, -1, "?-?no?code ... --? script ?resultVarName? ?optionVarName?" },
    {"concat", Jim_ConcatCoreCommand, 0, -1, "?arg ...?" },
    {"continue", Jim_ContinueCoreCommand, 0, 1, "?level?" },
#if defined(JIM_DEBUG_COMMAND) && !defined(JIM_BOOTSTRAP)
    {"debug", Jim_DebugCoreCommand, 1, -1, "subcommand ?arg ...?" },
#endif /* JIM_DEBUG_COMMAND && !JIM_BOOTSTRAP */
    {"dict", Jim_DictCoreCommand, 1, -1, "subcommand ?arg ...?"},
    {"env", Jim_EnvCoreCommand, 0, 2, "?varName? ?default?" },
    {"error", Jim_ErrorCoreCommand, 1, 2, "message ?stacktrace?" },
    {"eval", Jim_EvalCoreCommand, 1, -1, "arg ?arg ...?" },
    {"exists", Jim_ExistsCoreCommand, 1, 2, "?-command|-proc|-alias|-channel|-var? name" },
    {"exit", Jim_ExitCoreCommand, 0, 1, "?exitCode?" },
#ifdef JIM_COMPAT
    {"expr", Jim_ExprCoreCommand, 1, -1, "expression ?...?" },
#else
    {"expr", Jim_ExprCoreCommand, 1, 1, "expression" },
#endif
    {"for", Jim_ForCoreCommand, 4, 4, "start test next body" },

```

## Consumer bindings

No implementation binding is claimed by this observation record.

- [rust/tcl-vm/tests/embed_api_e2e.rs](../../../../rust/tcl-vm/tests/it_3/embed_api_e2e.rs), `an_error_a_host_took_as_its_own_leaves_error_code_and_error_info`: Keep caller-owned caught completion metadata and arbitrary message text separate in the software embedding consumer; actual Native430 release-specific public producers remain independent.
- [rust/tcl-vm/tests/embed_api_e2e.rs](../../../../rust/tcl-vm/tests/it_3/embed_api_e2e.rs), `an_error_a_host_took_as_its_own_leaves_error_code_and_error_info` (linked): Software caller-owned completion metadata and arbitrary message text, independently from Native430 original script cases.

These source bindings establish no executed assertion result; exact software outcomes belong to the independently pinned Rust validation receipts.

- [rust/tcl-registry/src/native_wrong_arguments.rs](../../../../rust/tcl-registry/src/native_wrong_arguments.rs), `InvocationDialect::native_wrong_arguments_protocol`: Select exact measured error-code policy by independent actual release: NONE for C8.4/C8.5/Jim and TCL WRONGARGS for C8.6/C9; software origin/String assertions do not attest public original headers.
- [rust/tcl-registry/src/native_wrong_arguments.rs](../../../../rust/tcl-registry/src/native_wrong_arguments.rs), `native_wrong_arguments::tests::measured_native_codes_have_independent_origin` (linked): Selected software error-code protocol compared with exact six-provider public original catch {set} metadata. Existing origin/String producer assertions remain software controls; original public scripts do not observe physical headers. This links the software assertion definition only. Independent actual execution receipts retain their own failures and scope; no whole-provider or specialised compilation pass is inferred.

These source bindings establish no executed assertion result; exact software outcomes belong to the independently pinned Rust validation receipts.

## Evidence

- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_error_code_metadata_original/providers](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/providers)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs)
- [rust/tcl-registry/tests/data/native_child_bootstrap_context303/source](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303/source)
- [rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/jim](../../../../rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/jim)
- [rust/tcl-registry/tests/data/native_jim_class_source269](../../../../rust/tcl-registry/tests/data/native_jim_class_source269)
- [rust/tcl-registry/tests/data/native_c_alias_holder197/8.4.20](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.4.20)
