# naming.alias.original-command-key-replacement-child-capture-and-rename

Kind: `native-observation`

## Problem statement

An alias record name, the occupied command-table key, the caller original name and a child registration key can have equal bytes while retaining distinct object roles. Public query Lists and invocation/result getters independently retain or read original members. Replacement and rename must preserve those finite role and channel differences.

## Question

For original core and child-handle alias creation, equal-byte replacement and rename, which original name/key/target/prefix headers are read or retained, and which public query and invocation stages preserve their separate List and lifetime roles?

## Conclusion

Five original cases per provider preserve a C alias-record name separately from a Jim occupied command-table key: equal-byte replacement and rename retain opposite original-name roles. C child capture pretests the target before reading and retaining the caller name; Jim child registration reads the name and creates an independent String key. C queries produce fresh original-member Lists; Jim core queries share the stored List while child info-alias is actual Guest1 with no public query List. Each case reaches two original target/prefix callbacks and explicit arg2-before-arg1 getters with counted NUL bytes. Held queries and original free-representation events retain their own phases. No application generation, current Native issuer, general alias parity or Rust assertion follows.

## Scope

One independently fixed421b request, all19 actual closed compiler/compile/dependency/execute records, complete outer channels, all six own original ELF/source/header/library/dependency before/after joins and20 source windows. Five fresh cases1,7,8,9,10 per provider separately cover core control, occupied equal-byte replacement, actual child capture, child replacement and distinct destination rename. Capture, result clear, raw query/header/member reads, deliberate invocation/result/presentation getters and alias/query/caller releases keep their own original ordering. Jim public EvalObjVector name getters include independent invocation holds; they cannot be compared to a handler-only count. C fresh query List purpose is independent of TclOO stored-header behavior. Free-internalRep is representation retirement rather than proof of header destruction; released unheld roles report-1 without dereferencing the old pointer, not a zero-reference claim. SDK source/library inputs are retained; configure/build processes lack replay receipts. No Var/frame/body/Normal/compiler/edit, arbitrary external List, application Native generation/issuer or broad provider-equality grant is made.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Original caller-held lazy counted alias name/target/prefix headers; actual core versus child public API channels, raw List member inspection, invocation getters, presentation and retirement have separate phases. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

For the five original cases, C replacement retains the new original alias-record name: old role0 caller-only1/new role4 caller+alias2 after result clear. Rename keeps the original record name role0 at2 and destination role4 caller-only1. Actual child capture reads target at refs1 before name at refs2, then retains original name/target/prefix members; equal-byte child replacement leaves old/new original name refs1/2. Public queries create distinct fresh List headers containing the original target/prefix members. Held queries preserve those members after alias deletion. Each case reaches two original callbacks and the deliberate arg2-before-arg1 getters; counted NUL-bearing arg1 remains unchanged. Core capture first reads target then name.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original caller-held lazy counted alias name/target/prefix headers; actual core versus child public API channels, raw List member inspection, invocation getters, presentation and retirement have separate phases. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

For the five original cases, C replacement retains the new original alias-record name: old role0 caller-only1/new role4 caller+alias2 after result clear. Rename keeps the original record name role0 at2 and destination role4 caller-only1. Actual child capture reads target at refs1 before name at refs2, then retains original name/target/prefix members; equal-byte child replacement leaves old/new original name refs1/2. Public queries create distinct fresh List headers containing the original target/prefix members. Held queries preserve those members after alias deletion. Each case reaches two original callbacks and the deliberate arg2-before-arg1 getters; counted NUL-bearing arg1 remains unchanged. Core capture first reads name then target.

### jim

Status: `observed`. Channel: Original caller-held lazy counted alias name/target/prefix headers; actual core versus child public API channels, raw List member inspection, invocation getters, presentation and retirement have separate phases. Version: jim 0.84-9-g5bac7c9.

Five original Jim cases distinguish occupied command-table key and independent child registration key. Core equal-byte replacement keeps old role0 at2 and new role4 caller-only1; rename releases the old key to caller-only1 and retains the new destination role4 at2. Core queries share the stored List header; replacement creates a new stored List while a held old query retains its original members. Actual child registration reads the caller name, creates an independent String key, and retains the parent prefix List with both caller name headers at1. Its public info-alias query reaches Guest1 because this command is not an alias; no original public query List is granted. All five cases reach two original target/prefix callbacks, with explicit arg2-before-arg1 getters and counted NUL-bearing arg1. Public EvalObjVector body name-getter refcounts retain their additional invocation roles.

### bigip

Status: `not-tested`.

No original BIG-IP process for this finite command-key replacement, child capture and rename question.

## Source inspection

tcl8.4 8.4.20, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInterp.c`, function `SlaveObjCmd`, lines 1862–2026. Retained reference `rust-tcl-registry-tests-data-native_info_commands_literal_original-original-inputs-8.4.20-generic-tclInterp.c`.

```text
 *
 *----------------------------------------------------------------------
 */

static int
SlaveObjCmd(clientData, interp, objc, objv)
    ClientData clientData;	/* Slave interpreter. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    Tcl_Interp *slaveInterp;
    int index;
    static CONST char *options[] = {
        "alias",	"aliases",	"eval",		"expose",
        "hide",		"hidden",	"issafe",	"invokehidden",
        "marktrusted",	"recursionlimit", NULL
    };
    enum options {
	OPT_ALIAS,	OPT_ALIASES,	OPT_EVAL,	OPT_EXPOSE,
	OPT_HIDE,	OPT_HIDDEN,	OPT_ISSAFE,	OPT_INVOKEHIDDEN,
	OPT_MARKTRUSTED, OPT_RECLIMIT
    };
    
    slaveInterp = (Tcl_Interp *) clientData;
    if (slaveInterp == NULL) {
	panic("SlaveObjCmd: interpreter has been deleted");
    }

    if (objc < 2) {
        Tcl_WrongNumArgs(interp, 1, objv, "cmd ?arg ...?");
        return TCL_ERROR;
    }
    if (Tcl_GetIndexFromObj(interp, objv[1], options, "option", 0,
	    &index) != TCL_OK) {
	return TCL_ERROR;
    }

    switch ((enum options) index) {
	case OPT_ALIAS: {
	    if (objc > 2) {
		if (objc == 3) {
		    return AliasDescribe(interp, slaveInterp, objv[2]);
		}
		if (Tcl_GetString(objv[3])[0] == '\0') {
		    if (objc == 4) {
			return AliasDelete(interp, slaveInterp, objv[2]);
		    }
		} else {
		    return AliasCreate(interp, slaveInterp, interp, objv[2],
			    objv[3], objc - 4, objv + 4);
		}
	    }
	    Tcl_WrongNumArgs(interp, 2, objv,
		    "aliasName ?targetName? ?args..?");
            return TCL_ERROR;
	}
	case OPT_ALIASES: {
	    if (objc != 2) {
		Tcl_WrongNumArgs(interp, 2, objv, (char *) NULL);
		return TCL_ERROR;
	    }
	    return AliasList(interp, slaveInterp);
	}
	case OPT_EVAL: {
	    if (objc < 3) {
		Tcl_WrongNumArgs(interp, 2, objv, "arg ?arg ...?");
		return TCL_ERROR;
	    }
	    return SlaveEval(interp, slaveInterp, objc - 2, objv + 2);
	}
        case OPT_EXPOSE: {
	    if ((objc < 3) || (objc > 4)) {
		Tcl_WrongNumArgs(interp, 2, objv, "hiddenCmdName ?cmdName?");
		return TCL_ERROR;
	    }
            return SlaveExpose(interp, slaveInterp, objc - 2, objv + 2);
	}
	case OPT_HIDE: {
	    if ((objc < 3) || (objc > 4)) {
		Tcl_WrongNumArgs(interp, 2, objv, "cmdName ?hiddenCmdName?");
		return TCL_ERROR;
	    }
            return SlaveHide(interp, slaveInterp, objc - 2, objv + 2);
	}
        case OPT_HIDDEN: {
	    if (objc != 2) {
		Tcl_WrongNumArgs(interp, 2, objv, NULL);
		return TCL_ERROR;
	    }
            return SlaveHidden(interp, slaveInterp);
	}
        case OPT_ISSAFE: {
	    if (objc != 2) {
		Tcl_WrongNumArgs(interp, 2, objv, (char *) NULL);
		return TCL_ERROR;
	    }
	    Tcl_SetIntObj(Tcl_GetObjResult(interp), Tcl_IsSafe(slaveInterp));
	    return TCL_OK;
	}
        case OPT_INVOKEHIDDEN: {
	    int global, i, index;
	    static CONST char *hiddenOptions[] = {
		"-global",	"--",		NULL
	    };
	    enum hiddenOption {
		OPT_GLOBAL,	OPT_LAST
	    };
	    global = 0;
	    for (i = 2; i < objc; i++) {
		if (Tcl_GetString(objv[i])[0] != '-') {
		    break;
		}
		if (Tcl_GetIndexFromObj(interp, objv[i], hiddenOptions,
			"option", 0, &index) != TCL_OK) {
		    return TCL_ERROR;
		}
		if (index == OPT_GLOBAL) {
		    global = 1;
		} else {
		    i++;
		    break;
		}
	    }
	    if (objc - i < 1) {
		Tcl_WrongNumArgs(interp, 2, objv,
			"?-global? ?--? cmd ?arg ..?");
		return TCL_ERROR;
	    }
	    return SlaveInvokeHidden(interp, slaveInterp, global, objc - i,
		    objv + i);
	}
	case OPT_MARKTRUSTED: {
	    if (objc != 2) {
		Tcl_WrongNumArgs(interp, 2, objv, NULL);
		return TCL_ERROR;
	    }
            return SlaveMarkTrusted(interp, slaveInterp);
	}
	case OPT_RECLIMIT: {
	    if (objc != 2 && objc != 3) {
		Tcl_WrongNumArgs(interp, 2, objv, "?newlimit?");
		return TCL_ERROR;
	    }
	    return SlaveRecursionLimit(interp, slaveInterp, objc - 2, objv + 2);
	}
    }

    return TCL_ERROR;
}

/*
 *----------------------------------------------------------------------
 *
 * SlaveObjCmdDeleteProc --
 *
 *	Invoked when an object command for a slave interpreter is deleted;
 *	cleans up all state associated with the slave interpreter and destroys
 *	the slave interpreter.
 *
 * Results:
 *	None.
 *
 * Side effects:
 *	Cleans up all state associated with the slave interpreter and

```

tcl8.4 8.4.20, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInterp.c`, function `AliasCreate`, lines 1149–1313. Retained reference `rust-tcl-registry-tests-data-native_info_commands_literal_original-original-inputs-8.4.20-generic-tclInterp.c`.

```text
 *
 *----------------------------------------------------------------------
 */

static int
AliasCreate(interp, slaveInterp, masterInterp, namePtr, targetNamePtr,
	objc, objv)
    Tcl_Interp *interp;		/* Interp for error reporting. */
    Tcl_Interp *slaveInterp;	/* Interp where alias cmd will live or from
				 * which alias will be deleted. */
    Tcl_Interp *masterInterp;	/* Interp in which target command will be
				 * invoked. */
    Tcl_Obj *namePtr;		/* Name of alias cmd. */
    Tcl_Obj *targetNamePtr;	/* Name of target cmd. */
    int objc;			/* Additional arguments to store */
    Tcl_Obj *CONST objv[];	/* with alias. */
{
    Alias *aliasPtr;
    Tcl_HashEntry *hPtr;
    Target *targetPtr;
    Slave *slavePtr;
    Master *masterPtr;
    Tcl_Obj **prefv;
    int new, i;

    aliasPtr = (Alias *) ckalloc((unsigned) (sizeof(Alias) 
            + objc * sizeof(Tcl_Obj *)));
    aliasPtr->namePtr		= namePtr;
    Tcl_IncrRefCount(aliasPtr->namePtr);
    aliasPtr->targetInterp	= masterInterp;

    aliasPtr->objc = objc + 1;
    prefv = &aliasPtr->objPtr;

    *prefv = targetNamePtr;
    Tcl_IncrRefCount(targetNamePtr);
    for (i = 0; i < objc; i++) {
	*(++prefv) = objv[i];
	Tcl_IncrRefCount(objv[i]);
    }

    Tcl_Preserve(slaveInterp);
    Tcl_Preserve(masterInterp);

    aliasPtr->slaveCmd = Tcl_CreateObjCommand(slaveInterp,
	    Tcl_GetString(namePtr), AliasObjCmd, (ClientData) aliasPtr,
	    AliasObjCmdDeleteProc);

    if (TclPreventAliasLoop(interp, slaveInterp,
	    aliasPtr->slaveCmd) != TCL_OK) {
	/*
	 * Found an alias loop!	 The last call to Tcl_CreateObjCommand made
	 * the alias point to itself.  Delete the command and its alias
	 * record.  Be careful to wipe out its client data first, so the
	 * command doesn't try to delete itself.
	 */

	Command *cmdPtr;
	
	Tcl_DecrRefCount(aliasPtr->namePtr);
	Tcl_DecrRefCount(targetNamePtr);
	for (i = 0; i < objc; i++) {
	    Tcl_DecrRefCount(objv[i]);
	}
	
	cmdPtr = (Command *) aliasPtr->slaveCmd;
	cmdPtr->clientData = NULL;
	cmdPtr->deleteProc = NULL;
	cmdPtr->deleteData = NULL;
	Tcl_DeleteCommandFromToken(slaveInterp, aliasPtr->slaveCmd);

	ckfree((char *) aliasPtr);

	/*
	 * The result was already set by TclPreventAliasLoop.
	 */

	Tcl_Release(slaveInterp);
	Tcl_Release(masterInterp);
	return TCL_ERROR;
    }

    /*
     * Make an entry in the alias table. If it already exists delete
     * the alias command. Then retry.
     */

    slavePtr = &((InterpInfo *) ((Interp *) slaveInterp)->interpInfo)->slave;
    while (1) {
	Alias *oldAliasPtr;
	char *string;
	
	string = Tcl_GetString(namePtr);
	hPtr = Tcl_CreateHashEntry(&slavePtr->aliasTable, string, &new);
	if (new != 0) {
	    break;
	}

	oldAliasPtr = (Alias *) Tcl_GetHashValue(hPtr);
	Tcl_DeleteCommandFromToken(slaveInterp, oldAliasPtr->slaveCmd);
    }

    aliasPtr->aliasEntryPtr = hPtr;
    Tcl_SetHashValue(hPtr, (ClientData) aliasPtr);
    
    /*
     * Create the new command. We must do it after deleting any old command,
     * because the alias may be pointing at a renamed alias, as in:
     *
     * interp alias {} foo {} bar		# Create an alias "foo"
     * rename foo zop				# Now rename the alias
     * interp alias {} foo {} zop		# Now recreate "foo"...
     */

    targetPtr = (Target *) ckalloc((unsigned) sizeof(Target));
    targetPtr->slaveCmd = aliasPtr->slaveCmd;
    targetPtr->slaveInterp = slaveInterp;

    Tcl_MutexLock(&cntMutex);
    masterPtr = &((InterpInfo *) ((Interp *) masterInterp)->interpInfo)->master;
    do {
        hPtr = Tcl_CreateHashEntry(&masterPtr->targetTable,
                (char *) aliasCounter, &new);
	aliasCounter++;
    } while (new == 0);
    Tcl_MutexUnlock(&cntMutex);

    Tcl_SetHashValue(hPtr, (ClientData) targetPtr);
    aliasPtr->targetEntryPtr = hPtr;

    Tcl_SetObjResult(interp, namePtr);

    Tcl_Release(slaveInterp);
    Tcl_Release(masterInterp);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * AliasDelete --
 *
 *	Deletes the given alias from the slave interpreter given.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	Deletes the alias from the slave interpreter.
 *
 *----------------------------------------------------------------------
 */

static int
AliasDelete(interp, slaveInterp, namePtr)
    Tcl_Interp *interp;		/* Interpreter for result & errors. */
    Tcl_Interp *slaveInterp;	/* Interpreter containing alias. */
    Tcl_Obj *namePtr;		/* Name of alias to delete. */
{
    Slave *slavePtr;
    Alias *aliasPtr;
    Tcl_HashEntry *hPtr;

    /*
     * If the alias has been renamed in the slave, the master can still use

```

tcl8.5 8.5.19, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInterp.c`, function `SlaveObjCmd`, lines 2265–2429. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-8.5.19-tclInterp.c`.

```text
 *
 *----------------------------------------------------------------------
 */

static int
SlaveObjCmd(
    ClientData clientData,	/* Slave interpreter. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Interp *slaveInterp = clientData;
    int index;
    static const char *options[] = {
	"alias",	"aliases",	"bgerror",	"debug",	"eval",
	"expose",	"hide",		"hidden",	"issafe",
	"invokehidden",	"limit",	"marktrusted",	"recursionlimit", NULL
    };
    enum options {
	OPT_ALIAS,	OPT_ALIASES,	OPT_BGERROR,	OPT_DEBUG,	OPT_EVAL,
	OPT_EXPOSE,	OPT_HIDE,	OPT_HIDDEN,	OPT_ISSAFE,
	OPT_INVOKEHIDDEN, OPT_LIMIT,	OPT_MARKTRUSTED, OPT_RECLIMIT
    };

    if (slaveInterp == NULL) {
	Tcl_Panic("SlaveObjCmd: interpreter has been deleted");
    }

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "cmd ?arg ...?");
	return TCL_ERROR;
    }
    if (Tcl_GetIndexFromObj(interp, objv[1], options, "option", 0,
	    &index) != TCL_OK) {
	return TCL_ERROR;
    }

    switch ((enum options) index) {
    case OPT_ALIAS:
	if (objc > 2) {
	    if (objc == 3) {
		return AliasDescribe(interp, slaveInterp, objv[2]);
	    }
	    if (TclGetString(objv[3])[0] == '\0') {
		if (objc == 4) {
		    return AliasDelete(interp, slaveInterp, objv[2]);
		}
	    } else {
		return AliasCreate(interp, slaveInterp, interp, objv[2],
			objv[3], objc - 4, objv + 4);
	    }
	}
	Tcl_WrongNumArgs(interp, 2, objv, "aliasName ?targetName? ?args..?");
	return TCL_ERROR;
    case OPT_ALIASES:
	if (objc != 2) {
	    Tcl_WrongNumArgs(interp, 2, objv, NULL);
	    return TCL_ERROR;
	}
	return AliasList(interp, slaveInterp);
    case OPT_BGERROR:
	if (objc != 2 && objc != 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "?cmdPrefix?");
	    return TCL_ERROR;
	}
	return SlaveBgerror(interp, slaveInterp, objc - 2, objv + 2);
    case OPT_DEBUG:
	/*
	 * TIP #378 *
	 * Currently only -frame supported, otherwise ?-option ?value? ...?
	 */
	if (objc > 4) {
	    Tcl_WrongNumArgs(interp, 2, objv, "?-frame ?bool??");
	    return TCL_ERROR;
	}
	return SlaveDebugCmd(interp, slaveInterp, objc - 2, objv + 2);
    case OPT_EVAL:
	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "arg ?arg ...?");
	    return TCL_ERROR;
	}
	return SlaveEval(interp, slaveInterp, objc - 2, objv + 2);
    case OPT_EXPOSE:
	if ((objc < 3) || (objc > 4)) {
	    Tcl_WrongNumArgs(interp, 2, objv, "hiddenCmdName ?cmdName?");
	    return TCL_ERROR;
	}
	return SlaveExpose(interp, slaveInterp, objc - 2, objv + 2);
    case OPT_HIDE:
	if ((objc < 3) || (objc > 4)) {
	    Tcl_WrongNumArgs(interp, 2, objv, "cmdName ?hiddenCmdName?");
	    return TCL_ERROR;
	}
	return SlaveHide(interp, slaveInterp, objc - 2, objv + 2);
    case OPT_HIDDEN:
	if (objc != 2) {
	    Tcl_WrongNumArgs(interp, 2, objv, NULL);
	    return TCL_ERROR;
	}
	return SlaveHidden(interp, slaveInterp);
    case OPT_ISSAFE:
	if (objc != 2) {
	    Tcl_WrongNumArgs(interp, 2, objv, NULL);
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, Tcl_NewBooleanObj(Tcl_IsSafe(slaveInterp)));
	return TCL_OK;
    case OPT_INVOKEHIDDEN: {
	int i, index;
	const char *namespaceName;
	static const char *hiddenOptions[] = {
	    "-global",	"-namespace",	"--", NULL
	};
	enum hiddenOption {
	    OPT_GLOBAL,	OPT_NAMESPACE,	OPT_LAST
	};

	namespaceName = NULL;
	for (i = 2; i < objc; i++) {
	    if (TclGetString(objv[i])[0] != '-') {
		break;
	    }
	    if (Tcl_GetIndexFromObj(interp, objv[i], hiddenOptions, "option",
		    0, &index) != TCL_OK) {
		return TCL_ERROR;
	    }
	    if (index == OPT_GLOBAL) {
		namespaceName = "::";
	    } else if (index == OPT_NAMESPACE) {
		if (++i == objc) { /* There must be more arguments. */
		    break;
		} else {
		    namespaceName = TclGetString(objv[i]);
		}
	    } else {
		i++;
		break;
	    }
	}
	if (objc - i < 1) {
	    Tcl_WrongNumArgs(interp, 2, objv,
		    "?-namespace ns? ?-global? ?--? cmd ?arg ..?");
	    return TCL_ERROR;
	}
	return SlaveInvokeHidden(interp, slaveInterp, namespaceName,
		objc - i, objv + i);
    }
    case OPT_LIMIT: {
	static const char *limitTypes[] = {
	    "commands", "time", NULL
	};
	enum LimitTypes {
	    LIMIT_TYPE_COMMANDS, LIMIT_TYPE_TIME
	};
	int limitType;

	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "limitType ?options?");
	    return TCL_ERROR;
	}
	if (Tcl_GetIndexFromObj(interp, objv[2], limitTypes, "limit type", 0,
		&limitType) != TCL_OK) {
	    return TCL_ERROR;
	}
	switch ((enum LimitTypes) limitType) {

```

tcl8.5 8.5.19, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInterp.c`, function `AliasCreate`, lines 1422–1586. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-8.5.19-tclInterp.c`.

```text
 *
 *----------------------------------------------------------------------
 */

static int
AliasCreate(
    Tcl_Interp *interp,		/* Interp for error reporting. */
    Tcl_Interp *slaveInterp,	/* Interp where alias cmd will live or from
				 * which alias will be deleted. */
    Tcl_Interp *masterInterp,	/* Interp in which target command will be
				 * invoked. */
    Tcl_Obj *namePtr,		/* Name of alias cmd. */
    Tcl_Obj *targetNamePtr,	/* Name of target cmd. */
    int objc,			/* Additional arguments to store */
    Tcl_Obj *const objv[])	/* with alias. */
{
    Alias *aliasPtr;
    Tcl_HashEntry *hPtr;
    Target *targetPtr;
    Slave *slavePtr;
    Master *masterPtr;
    Tcl_Obj **prefv;
    int isNew, i;

    aliasPtr = (Alias *) ckalloc((unsigned) (sizeof(Alias)
	    + objc * sizeof(Tcl_Obj *)));
    aliasPtr->token = namePtr;
    Tcl_IncrRefCount(aliasPtr->token);
    aliasPtr->targetInterp = masterInterp;

    aliasPtr->objc = objc + 1;
    prefv = &aliasPtr->objPtr;

    *prefv = targetNamePtr;
    Tcl_IncrRefCount(targetNamePtr);
    for (i = 0; i < objc; i++) {
	*(++prefv) = objv[i];
	Tcl_IncrRefCount(objv[i]);
    }

    Tcl_Preserve(slaveInterp);
    Tcl_Preserve(masterInterp);

    aliasPtr->slaveCmd = Tcl_CreateObjCommand(slaveInterp,
	    TclGetString(namePtr), AliasObjCmd, aliasPtr,
	    AliasObjCmdDeleteProc);

    if (TclPreventAliasLoop(interp, slaveInterp,
	    aliasPtr->slaveCmd) != TCL_OK) {
	/*
	 * Found an alias loop! The last call to Tcl_CreateObjCommand made the
	 * alias point to itself. Delete the command and its alias record. Be
	 * careful to wipe out its client data first, so the command doesn't
	 * try to delete itself.
	 */

	Command *cmdPtr;

	Tcl_DecrRefCount(aliasPtr->token);
	Tcl_DecrRefCount(targetNamePtr);
	for (i = 0; i < objc; i++) {
	    Tcl_DecrRefCount(objv[i]);
	}

	cmdPtr = (Command *) aliasPtr->slaveCmd;
	cmdPtr->clientData = NULL;
	cmdPtr->deleteProc = NULL;
	cmdPtr->deleteData = NULL;
	Tcl_DeleteCommandFromToken(slaveInterp, aliasPtr->slaveCmd);

	ckfree((char *) aliasPtr);

	/*
	 * The result was already set by TclPreventAliasLoop.
	 */

	Tcl_Release(slaveInterp);
	Tcl_Release(masterInterp);
	return TCL_ERROR;
    }

    /*
     * Make an entry in the alias table. If it already exists, retry.
     */

    slavePtr = &((InterpInfo *) ((Interp *) slaveInterp)->interpInfo)->slave;
    while (1) {
	Tcl_Obj *newToken;
	char *string;

	string = TclGetString(aliasPtr->token);
	hPtr = Tcl_CreateHashEntry(&slavePtr->aliasTable, string, &isNew);
	if (isNew != 0) {
	    break;
	}

	/*
	 * The alias name cannot be used as unique token, it is already taken.
	 * We can produce a unique token by prepending "::" repeatedly. This
	 * algorithm is a stop-gap to try to maintain the command name as
	 * token for most use cases, fearful of possible backwards compat
	 * problems. A better algorithm would produce unique tokens that need
	 * not be related to the command name.
	 *
	 * ATTENTION: the tests in interp.test and possibly safe.test depend
	 * on the precise definition of these tokens.
	 */

	TclNewLiteralStringObj(newToken, "::");
	Tcl_AppendObjToObj(newToken, aliasPtr->token);
	Tcl_DecrRefCount(aliasPtr->token);
	aliasPtr->token = newToken;
	Tcl_IncrRefCount(aliasPtr->token);
    }

    aliasPtr->aliasEntryPtr = hPtr;
    Tcl_SetHashValue(hPtr, aliasPtr);

    /*
     * Create the new command. We must do it after deleting any old command,
     * because the alias may be pointing at a renamed alias, as in:
     *
     * interp alias {} foo {} bar		# Create an alias "foo"
     * rename foo zop				# Now rename the alias
     * interp alias {} foo {} zop		# Now recreate "foo"...
     */

    targetPtr = (Target *) ckalloc((unsigned) sizeof(Target));
    targetPtr->slaveCmd = aliasPtr->slaveCmd;
    targetPtr->slaveInterp = slaveInterp;

    masterPtr = &((InterpInfo *) ((Interp*) masterInterp)->interpInfo)->master;
    targetPtr->nextPtr = masterPtr->targetsPtr;
    targetPtr->prevPtr = NULL;
    if (masterPtr->targetsPtr != NULL) {
	masterPtr->targetsPtr->prevPtr = targetPtr;
    }
    masterPtr->targetsPtr = targetPtr;
    aliasPtr->targetPtr = targetPtr;

    Tcl_SetObjResult(interp, aliasPtr->token);

    Tcl_Release(slaveInterp);
    Tcl_Release(masterInterp);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * AliasDelete --
 *
 *	Deletes the given alias from the slave interpreter given.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	Deletes the alias from the slave interpreter.
 *
 *----------------------------------------------------------------------
 */

static int
AliasDelete(

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclInterp.c`, function `NRChildCmd`, lines 2449–2613. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-8.6.18-tclInterp.c`.

```text
{
    return Tcl_NRCallObjProc(interp, NRChildCmd, clientData, objc, objv);
}

static int
NRChildCmd(
    ClientData clientData,	/* Child interpreter. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Interp *childInterp = (Tcl_Interp *)clientData;
    int index;
    static const char *const options[] = {
	"alias",	"aliases",	"bgerror",	"debug",
	"eval",		"expose",	"hide",		"hidden",
	"issafe",	"invokehidden",	"limit",	"marktrusted",
	"recursionlimit", NULL
    };
    enum childCmdOptionsEnum {
	OPT_ALIAS,	OPT_ALIASES,	OPT_BGERROR,	OPT_DEBUG,
	OPT_EVAL,	OPT_EXPOSE,	OPT_HIDE,	OPT_HIDDEN,
	OPT_ISSAFE,	OPT_INVOKEHIDDEN, OPT_LIMIT,	OPT_MARKTRUSTED,
	OPT_RECLIMIT
    };

    if (childInterp == NULL) {
	Tcl_Panic("ChildObjCmd: interpreter has been deleted");
    }

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "cmd ?arg ...?");
	return TCL_ERROR;
    }
    if (Tcl_GetIndexFromObj(interp, objv[1], options, "option", 0,
	    &index) != TCL_OK) {
	return TCL_ERROR;
    }

    switch ((enum childCmdOptionsEnum) index) {
    case OPT_ALIAS:
	if (objc > 2) {
	    if (objc == 3) {
		return AliasDescribe(interp, childInterp, objv[2]);
	    }
	    if (TclGetString(objv[3])[0] == '\0') {
		if (objc == 4) {
		    return AliasDelete(interp, childInterp, objv[2]);
		}
	    } else {
		return AliasCreate(interp, childInterp, interp, objv[2],
			objv[3], objc - 4, objv + 4);
	    }
	}
	Tcl_WrongNumArgs(interp, 2, objv, "aliasName ?targetName? ?arg ...?");
	return TCL_ERROR;
    case OPT_ALIASES:
	if (objc != 2) {
	    Tcl_WrongNumArgs(interp, 2, objv, NULL);
	    return TCL_ERROR;
	}
	return AliasList(interp, childInterp);
    case OPT_BGERROR:
	if (objc != 2 && objc != 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "?cmdPrefix?");
	    return TCL_ERROR;
	}
	return ChildBgerror(interp, childInterp, objc - 2, objv + 2);
    case OPT_DEBUG:
	/*
	 * TIP #378
	 * Currently only -frame supported, otherwise ?-option ?value? ...?
	 */
	if (objc > 4) {
	    Tcl_WrongNumArgs(interp, 2, objv, "?-frame ?bool??");
	    return TCL_ERROR;
	}
	return ChildDebugCmd(interp, childInterp, objc - 2, objv + 2);
    case OPT_EVAL:
	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "arg ?arg ...?");
	    return TCL_ERROR;
	}
	return ChildEval(interp, childInterp, objc - 2, objv + 2);
    case OPT_EXPOSE:
	if ((objc < 3) || (objc > 4)) {
	    Tcl_WrongNumArgs(interp, 2, objv, "hiddenCmdName ?cmdName?");
	    return TCL_ERROR;
	}
	return ChildExpose(interp, childInterp, objc - 2, objv + 2);
    case OPT_HIDE:
	if ((objc < 3) || (objc > 4)) {
	    Tcl_WrongNumArgs(interp, 2, objv, "cmdName ?hiddenCmdName?");
	    return TCL_ERROR;
	}
	return ChildHide(interp, childInterp, objc - 2, objv + 2);
    case OPT_HIDDEN:
	if (objc != 2) {
	    Tcl_WrongNumArgs(interp, 2, objv, NULL);
	    return TCL_ERROR;
	}
	return ChildHidden(interp, childInterp);
    case OPT_ISSAFE:
	if (objc != 2) {
	    Tcl_WrongNumArgs(interp, 2, objv, NULL);
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, Tcl_NewBooleanObj(Tcl_IsSafe(childInterp)));
	return TCL_OK;
    case OPT_INVOKEHIDDEN: {
	int i;
	const char *namespaceName;
	static const char *const hiddenOptions[] = {
	    "-global",	"-namespace",	"--", NULL
	};
	enum hiddenOption {
	    OPT_GLOBAL,	OPT_NAMESPACE,	OPT_LAST
	};

	namespaceName = NULL;
	for (i = 2; i < objc; i++) {
	    if (TclGetString(objv[i])[0] != '-') {
		break;
	    }
	    if (Tcl_GetIndexFromObj(interp, objv[i], hiddenOptions, "option",
		    0, &index) != TCL_OK) {
		return TCL_ERROR;
	    }
	    if (index == OPT_GLOBAL) {
		namespaceName = "::";
	    } else if (index == OPT_NAMESPACE) {
		if (++i == objc) { /* There must be more arguments. */
		    break;
		} else {
		    namespaceName = TclGetString(objv[i]);
		}
	    } else {
		i++;
		break;
	    }
	}
	if (objc - i < 1) {
	    Tcl_WrongNumArgs(interp, 2, objv,
		    "?-namespace ns? ?-global? ?--? cmd ?arg ..?");
	    return TCL_ERROR;
	}
	return ChildInvokeHidden(interp, childInterp, namespaceName,
		objc - i, objv + i);
    }
    case OPT_LIMIT: {
	static const char *const limitTypes[] = {
	    "commands", "time", NULL
	};
	enum LimitTypes {
	    LIMIT_TYPE_COMMANDS, LIMIT_TYPE_TIME
	};
	int limitType;

	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "limitType ?-option value ...?");
	    return TCL_ERROR;
	}
	if (Tcl_GetIndexFromObj(interp, objv[2], limitTypes, "limit type", 0,
		&limitType) != TCL_OK) {
	    return TCL_ERROR;

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclInterp.c`, function `AliasCreate`, lines 1479–1643. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-8.6.18-tclInterp.c`.

```text
 *
 *----------------------------------------------------------------------
 */

static int
AliasCreate(
    Tcl_Interp *interp,		/* Interp for error reporting. */
    Tcl_Interp *childInterp,	/* Interp where alias cmd will live or from
				 * which alias will be deleted. */
    Tcl_Interp *parentInterp,	/* Interp in which target command will be
				 * invoked. */
    Tcl_Obj *namePtr,		/* Name of alias cmd. */
    Tcl_Obj *targetCmdPtr,	/* Name of target cmd. */
    int objc,			/* Additional arguments to store */
    Tcl_Obj *const objv[])	/* with alias. */
{
    Alias *aliasPtr;
    Tcl_HashEntry *hPtr;
    Target *targetPtr;
    Child *childPtr;
    Parent *parentPtr;
    Tcl_Obj **prefv;
    int isNew, i;

    aliasPtr = (Alias *)ckalloc(sizeof(Alias) + objc * sizeof(Tcl_Obj *));
    aliasPtr->token = namePtr;
    Tcl_IncrRefCount(aliasPtr->token);
    aliasPtr->targetInterp = parentInterp;

    aliasPtr->objc = objc + 1;
    prefv = &aliasPtr->objPtr;

    *prefv = targetCmdPtr;
    Tcl_IncrRefCount(targetCmdPtr);
    for (i = 0; i < objc; i++) {
	*(++prefv) = objv[i];
	Tcl_IncrRefCount(objv[i]);
    }

    Tcl_Preserve(childInterp);
    Tcl_Preserve(parentInterp);

    if (childInterp == parentInterp) {
	aliasPtr->childCmd = Tcl_NRCreateCommand(childInterp,
		TclGetString(namePtr), AliasObjCmd, AliasNRCmd, aliasPtr,
		AliasObjCmdDeleteProc);
    } else {
    aliasPtr->childCmd = Tcl_CreateObjCommand(childInterp,
	    TclGetString(namePtr), AliasObjCmd, aliasPtr,
	    AliasObjCmdDeleteProc);
    }

    if (TclPreventAliasLoop(interp, childInterp,
	    aliasPtr->childCmd) != TCL_OK) {
	/*
	 * Found an alias loop! The last call to Tcl_CreateObjCommand made the
	 * alias point to itself. Delete the command and its alias record. Be
	 * careful to wipe out its client data first, so the command doesn't
	 * try to delete itself.
	 */

	Command *cmdPtr;

	Tcl_DecrRefCount(aliasPtr->token);
	Tcl_DecrRefCount(targetCmdPtr);
	for (i = 0; i < objc; i++) {
	    Tcl_DecrRefCount(objv[i]);
	}

	cmdPtr = (Command *) aliasPtr->childCmd;
	cmdPtr->clientData = NULL;
	cmdPtr->deleteProc = NULL;
	cmdPtr->deleteData = NULL;
	Tcl_DeleteCommandFromToken(childInterp, aliasPtr->childCmd);

	ckfree(aliasPtr);

	/*
	 * The result was already set by TclPreventAliasLoop.
	 */

	Tcl_Release(childInterp);
	Tcl_Release(parentInterp);
	return TCL_ERROR;
    }

    /*
     * Make an entry in the alias table. If it already exists, retry.
     */

    childPtr = &((InterpInfo *) ((Interp *) childInterp)->interpInfo)->child;
    while (1) {
	Tcl_Obj *newToken;
	const char *string;

	string = TclGetString(aliasPtr->token);
	hPtr = Tcl_CreateHashEntry(&childPtr->aliasTable, string, &isNew);
	if (isNew != 0) {
	    break;
	}

	/*
	 * The alias name cannot be used as unique token, it is already taken.
	 * We can produce a unique token by prepending "::" repeatedly. This
	 * algorithm is a stop-gap to try to maintain the command name as
	 * token for most use cases, fearful of possible backwards compat
	 * problems. A better algorithm would produce unique tokens that need
	 * not be related to the command name.
	 *
	 * ATTENTION: the tests in interp.test and possibly safe.test depend
	 * on the precise definition of these tokens.
	 */

	TclNewLiteralStringObj(newToken, "::");
	Tcl_AppendObjToObj(newToken, aliasPtr->token);
	Tcl_DecrRefCount(aliasPtr->token);
	aliasPtr->token = newToken;
	Tcl_IncrRefCount(aliasPtr->token);
    }

    aliasPtr->aliasEntryPtr = hPtr;
    Tcl_SetHashValue(hPtr, aliasPtr);

    /*
     * Create the new command. We must do it after deleting any old command,
     * because the alias may be pointing at a renamed alias, as in:
     *
     * interp alias {} foo {} bar		# Create an alias "foo"
     * rename foo zop				# Now rename the alias
     * interp alias {} foo {} zop		# Now recreate "foo"...
     */

    targetPtr = (Target *)ckalloc(sizeof(Target));
    targetPtr->childCmd = aliasPtr->childCmd;
    targetPtr->childInterp = childInterp;

    parentPtr = &((InterpInfo*) ((Interp*) parentInterp)->interpInfo)->parent;
    targetPtr->nextPtr = parentPtr->targetsPtr;
    targetPtr->prevPtr = NULL;
    if (parentPtr->targetsPtr != NULL) {
	parentPtr->targetsPtr->prevPtr = targetPtr;
    }
    parentPtr->targetsPtr = targetPtr;
    aliasPtr->targetPtr = targetPtr;

    Tcl_SetObjResult(interp, aliasPtr->token);

    Tcl_Release(childInterp);
    Tcl_Release(parentInterp);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * AliasDelete --
 *
 *	Deletes the given alias from the child interpreter given.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	Deletes the alias from the child interpreter.
 *

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInterp.c`, function `NRChildCmd`, lines 2522–2686. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-9.0.4-tclInterp.c`.

```text
{
    return Tcl_NRCallObjProc(interp, NRChildCmd, clientData, objc, objv);
}

static int
NRChildCmd(
    void *clientData,		/* Child interpreter. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Interp *childInterp = (Tcl_Interp *) clientData;
    static const char *const options[] = {
	"alias",	"aliases",	"bgerror",	"debug",
	"eval",		"expose",	"hide",		"hidden",
	"issafe",	"invokehidden",	"limit",	"marktrusted",
	"recursionlimit", NULL
    };
    enum childCmdOptionsEnum {
	OPT_ALIAS,	OPT_ALIASES,	OPT_BGERROR,	OPT_DEBUG,
	OPT_EVAL,	OPT_EXPOSE,	OPT_HIDE,	OPT_HIDDEN,
	OPT_ISSAFE,	OPT_INVOKEHIDDEN, OPT_LIMIT,	OPT_MARKTRUSTED,
	OPT_RECLIMIT
    } index;

    if (childInterp == NULL) {
	Tcl_Panic("TclChildObjCmd: interpreter has been deleted");
    }

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "cmd ?arg ...?");
	return TCL_ERROR;
    }
    if (Tcl_GetIndexFromObj(interp, objv[1], options, "option", 0,
	    &index) != TCL_OK) {
	return TCL_ERROR;
    }

    switch (index) {
    case OPT_ALIAS:
	if (objc > 2) {
	    if (objc == 3) {
		return AliasDescribe(interp, childInterp, objv[2]);
	    }
	    if (TclGetString(objv[3])[0] == '\0') {
		if (objc == 4) {
		    return AliasDelete(interp, childInterp, objv[2]);
		}
	    } else {
		return TclAliasCreate(interp, childInterp, interp, objv[2],
			objv[3], objc - 4, objv + 4);
	    }
	}
	Tcl_WrongNumArgs(interp, 2, objv, "aliasName ?targetName? ?arg ...?");
	return TCL_ERROR;
    case OPT_ALIASES:
	if (objc != 2) {
	    Tcl_WrongNumArgs(interp, 2, objv, NULL);
	    return TCL_ERROR;
	}
	return AliasList(interp, childInterp);
    case OPT_BGERROR:
	if (objc != 2 && objc != 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "?cmdPrefix?");
	    return TCL_ERROR;
	}
	return ChildBgerror(interp, childInterp, objc - 2, objv + 2);
    case OPT_DEBUG:
	/*
	 * TIP #378
	 * Currently only -frame supported, otherwise ?-option ?value? ...?
	 */
	if (objc > 4) {
	    Tcl_WrongNumArgs(interp, 2, objv, "?-frame ?bool??");
	    return TCL_ERROR;
	}
	return ChildDebugCmd(interp, childInterp, objc - 2, objv + 2);
    case OPT_EVAL:
	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "arg ?arg ...?");
	    return TCL_ERROR;
	}
	return ChildEval(interp, childInterp, objc - 2, objv + 2);
    case OPT_EXPOSE:
	if ((objc < 3) || (objc > 4)) {
	    Tcl_WrongNumArgs(interp, 2, objv, "hiddenCmdName ?cmdName?");
	    return TCL_ERROR;
	}
	return ChildExpose(interp, childInterp, objc - 2, objv + 2);
    case OPT_HIDE:
	if ((objc < 3) || (objc > 4)) {
	    Tcl_WrongNumArgs(interp, 2, objv, "cmdName ?hiddenCmdName?");
	    return TCL_ERROR;
	}
	return ChildHide(interp, childInterp, objc - 2, objv + 2);
    case OPT_HIDDEN:
	if (objc != 2) {
	    Tcl_WrongNumArgs(interp, 2, objv, NULL);
	    return TCL_ERROR;
	}
	return ChildHidden(interp, childInterp);
    case OPT_ISSAFE:
	if (objc != 2) {
	    Tcl_WrongNumArgs(interp, 2, objv, NULL);
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, Tcl_NewBooleanObj(Tcl_IsSafe(childInterp)));
	return TCL_OK;
    case OPT_INVOKEHIDDEN: {
	Tcl_Size i;
	const char *namespaceName;
	static const char *const hiddenOptions[] = {
	    "-global",	"-namespace",	"--", NULL
	};
	enum hiddenOption {
	    OPT_GLOBAL,	OPT_NAMESPACE,	OPT_LAST
	} idx;

	namespaceName = NULL;
	for (i = 2; i < objc; i++) {
	    if (TclGetString(objv[i])[0] != '-') {
		break;
	    }
	    if (Tcl_GetIndexFromObj(interp, objv[i], hiddenOptions, "option",
		    0, &idx) != TCL_OK) {
		return TCL_ERROR;
	    }
	    if (idx == OPT_GLOBAL) {
		namespaceName = "::";
	    } else if (idx == OPT_NAMESPACE) {
		if (++i == objc) { /* There must be more arguments. */
		    break;
		}
		namespaceName = TclGetString(objv[i]);
	    } else {
		i++;
		break;
	    }
	}
	if (objc - i < 1) {
	    Tcl_WrongNumArgs(interp, 2, objv,
		    "?-namespace ns? ?-global? ?--? cmd ?arg ..?");
	    return TCL_ERROR;
	}
	return ChildInvokeHidden(interp, childInterp, namespaceName,
		objc - i, objv + i);
    }
    case OPT_LIMIT: {
	static const char *const limitTypes[] = {
	    "commands", "time", NULL
	};
	enum LimitTypes {
	    LIMIT_TYPE_COMMANDS, LIMIT_TYPE_TIME
	} limitType;

	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "limitType ?-option value ...?");
	    return TCL_ERROR;
	}
	if (Tcl_GetIndexFromObj(interp, objv[2], limitTypes, "limit type", 0,
		&limitType) != TCL_OK) {
	    return TCL_ERROR;
	}
	switch (limitType) {
	case LIMIT_TYPE_COMMANDS:

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInterp.c`, function `TclAliasCreate`, lines 1475–1639. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-9.0.4-tclInterp.c`.

```text
 *
 *----------------------------------------------------------------------
 */

int
TclAliasCreate(
    Tcl_Interp *interp,		/* Interp for error reporting. */
    Tcl_Interp *childInterp,	/* Interp where alias cmd will live or from
				 * which alias will be deleted. */
    Tcl_Interp *parentInterp,	/* Interp in which target command will be
				 * invoked. */
    Tcl_Obj *namePtr,		/* Name of alias cmd. */
    Tcl_Obj *targetCmdPtr,	/* Name of target cmd. */
    Tcl_Size objc,		/* Additional arguments to store */
    Tcl_Obj *const objv[])	/* with alias. */
{
    Alias *aliasPtr;
    Tcl_HashEntry *hPtr;
    Target *targetPtr;
    Child *childPtr;
    Parent *parentPtr;
    Tcl_Obj **prefv;
    int isNew;
    Tcl_Size i;

    aliasPtr = (Alias *) Tcl_Alloc(sizeof(Alias) + objc * sizeof(Tcl_Obj *));
    aliasPtr->token = namePtr;
    Tcl_IncrRefCount(aliasPtr->token);
    aliasPtr->targetInterp = parentInterp;

    aliasPtr->objc = objc + 1;
    prefv = &aliasPtr->objPtr;

    *prefv = targetCmdPtr;
    Tcl_IncrRefCount(targetCmdPtr);
    for (i = 0; i < objc; i++) {
	*(++prefv) = objv[i];
	Tcl_IncrRefCount(objv[i]);
    }

    Tcl_Preserve(childInterp);
    Tcl_Preserve(parentInterp);

    if (childInterp == parentInterp) {
	aliasPtr->childCmd = Tcl_NRCreateCommand(childInterp,
		TclGetString(namePtr), TclLocalAliasObjCmd, AliasNRCmd,
		aliasPtr, AliasObjCmdDeleteProc);
    } else {
	aliasPtr->childCmd = Tcl_CreateObjCommand(childInterp,
		TclGetString(namePtr), TclAliasObjCmd, aliasPtr,
		AliasObjCmdDeleteProc);
    }

    if (TclPreventAliasLoop(interp, childInterp,
	    aliasPtr->childCmd) != TCL_OK) {
	/*
	 * Found an alias loop! The last call to Tcl_CreateObjCommand made the
	 * alias point to itself. Delete the command and its alias record. Be
	 * careful to wipe out its client data first, so the command doesn't
	 * try to delete itself.
	 */

	Command *cmdPtr;

	Tcl_DecrRefCount(aliasPtr->token);
	Tcl_DecrRefCount(targetCmdPtr);
	for (i = 0; i < objc; i++) {
	    Tcl_DecrRefCount(objv[i]);
	}

	cmdPtr = (Command *) aliasPtr->childCmd;
	cmdPtr->clientData = NULL;
	cmdPtr->deleteProc = NULL;
	cmdPtr->deleteData = NULL;
	Tcl_DeleteCommandFromToken(childInterp, aliasPtr->childCmd);

	Tcl_Free(aliasPtr);

	/*
	 * The result was already set by TclPreventAliasLoop.
	 */

	Tcl_Release(childInterp);
	Tcl_Release(parentInterp);
	return TCL_ERROR;
    }

    /*
     * Make an entry in the alias table. If it already exists, retry.
     */

    childPtr = &INTERP_INFO(childInterp)->child;
    while (1) {
	Tcl_Obj *newToken;
	const char *string;

	string = TclGetString(aliasPtr->token);
	hPtr = Tcl_CreateHashEntry(&childPtr->aliasTable, string, &isNew);
	if (isNew != 0) {
	    break;
	}

	/*
	 * The alias name cannot be used as unique token, it is already taken.
	 * We can produce a unique token by prepending "::" repeatedly. This
	 * algorithm is a stop-gap to try to maintain the command name as
	 * token for most use cases, fearful of possible backwards compat
	 * problems. A better algorithm would produce unique tokens that need
	 * not be related to the command name.
	 *
	 * ATTENTION: the tests in interp.test and possibly safe.test depend
	 * on the precise definition of these tokens.
	 */

	TclNewLiteralStringObj(newToken, "::");
	Tcl_AppendObjToObj(newToken, aliasPtr->token);
	Tcl_DecrRefCount(aliasPtr->token);
	aliasPtr->token = newToken;
	Tcl_IncrRefCount(aliasPtr->token);
    }

    aliasPtr->aliasEntryPtr = hPtr;
    Tcl_SetHashValue(hPtr, aliasPtr);

    /*
     * Create the new command. We must do it after deleting any old command,
     * because the alias may be pointing at a renamed alias, as in:
     *
     * interp alias {} foo {} bar		# Create an alias "foo"
     * rename foo zop				# Now rename the alias
     * interp alias {} foo {} zop		# Now recreate "foo"...
     */

    targetPtr = (Target *) Tcl_Alloc(sizeof(Target));
    targetPtr->childCmd = aliasPtr->childCmd;
    targetPtr->childInterp = childInterp;

    parentPtr = &INTERP_INFO(parentInterp)->parent;
    targetPtr->nextPtr = parentPtr->targetsPtr;
    targetPtr->prevPtr = NULL;
    if (parentPtr->targetsPtr != NULL) {
	parentPtr->targetsPtr->prevPtr = targetPtr;
    }
    parentPtr->targetsPtr = targetPtr;
    aliasPtr->targetPtr = targetPtr;

    Tcl_SetObjResult(interp, aliasPtr->token);

    Tcl_Release(childInterp);
    Tcl_Release(parentInterp);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * AliasDelete --
 *
 *	Deletes the given alias from the child interpreter given.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	Deletes the alias from the child interpreter.

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInterp.c`, function `NRChildCmd`, lines 2907–3071. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-9.1.0-tclInterp.c`.

```text
{
    return Tcl_NRCallObjProc2(interp, NRChildCmd, clientData, objc, objv);
}

static int
NRChildCmd(
    void *clientData,		/* Child interpreter. */
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Interp *childInterp = (Tcl_Interp *) clientData;
    static const char *const options[] = {
	"alias",	"aliases",	"bgerror",	"debug",
	"eval",		"expose",	"hide",		"hidden",
	"issafe",	"invokehidden",	"limit",	"marktrusted",
	"recursionlimit", "set",	NULL
    };
    enum childCmdOptionsEnum {
	OPT_ALIAS,	OPT_ALIASES,	OPT_BGERROR,	OPT_DEBUG,
	OPT_EVAL,	OPT_EXPOSE,	OPT_HIDE,	OPT_HIDDEN,
	OPT_ISSAFE,	OPT_INVOKEHIDDEN, OPT_LIMIT,	OPT_MARKTRUSTED,
	OPT_RECLIMIT,	OPT_SET
    } index;

    if (childInterp == NULL) {
	Tcl_Panic("TclChildObjCmd: interpreter has been deleted");
    }

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "cmd ?arg ...?");
	return TCL_ERROR;
    }
    if (Tcl_GetIndexFromObj(interp, objv[1], options, "option", 0,
	    &index) != TCL_OK) {
	return TCL_ERROR;
    }

    switch (index) {
    case OPT_ALIAS:
	if (objc > 2) {
	    if (objc == 3) {
		return AliasDescribe(interp, childInterp, objv[2]);
	    }
	    if (Tcl_IsEmpty(objv[3])) {
		if (objc == 4) {
		    return AliasDelete(interp, childInterp, objv[2]);
		}
	    } else {
		return TclAliasCreate(interp, childInterp, interp, objv[2],
			objv[3], objc - 4, objv + 4);
	    }
	}
	Tcl_WrongNumArgs(interp, 2, objv, "aliasName ?targetName? ?arg ...?");
	return TCL_ERROR;
    case OPT_ALIASES:
	if (objc != 2) {
	    Tcl_WrongNumArgs(interp, 2, objv, NULL);
	    return TCL_ERROR;
	}
	return AliasList(interp, childInterp);
    case OPT_BGERROR:
	if (objc != 2 && objc != 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "?cmdPrefix?");
	    return TCL_ERROR;
	}
	return ChildBgerror(interp, childInterp, objc - 2, objv + 2);
    case OPT_DEBUG:
	/*
	 * TIP #378
	 * Currently only -frame supported, otherwise ?-option ?value? ...?
	 */
	if (objc > 4) {
	    Tcl_WrongNumArgs(interp, 2, objv, "?-frame ?bool??");
	    return TCL_ERROR;
	}
	return ChildDebugCmd(interp, childInterp, objc - 2, objv + 2);
    case OPT_EVAL:
	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "arg ?arg ...?");
	    return TCL_ERROR;
	}
	return ChildEval(interp, childInterp, objc - 2, objv + 2);
    case OPT_EXPOSE:
	if ((objc < 3) || (objc > 4)) {
	    Tcl_WrongNumArgs(interp, 2, objv, "hiddenCmdName ?cmdName?");
	    return TCL_ERROR;
	}
	return ChildExpose(interp, childInterp, objc - 2, objv + 2);
    case OPT_HIDE:
	if ((objc < 3) || (objc > 4)) {
	    Tcl_WrongNumArgs(interp, 2, objv, "cmdName ?hiddenCmdName?");
	    return TCL_ERROR;
	}
	return ChildHide(interp, childInterp, objc - 2, objv + 2);
    case OPT_HIDDEN:
	if (objc != 2) {
	    Tcl_WrongNumArgs(interp, 2, objv, NULL);
	    return TCL_ERROR;
	}
	return ChildHidden(interp, childInterp);
    case OPT_ISSAFE:
	if (objc != 2) {
	    Tcl_WrongNumArgs(interp, 2, objv, NULL);
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, Tcl_NewBooleanObj(Tcl_IsSafe(childInterp)));
	return TCL_OK;
    case OPT_INVOKEHIDDEN: {
	Tcl_Size i;
	const char *namespaceName;
	static const char *const hiddenOptions[] = {
	    "-global",	"-namespace",	"--", NULL
	};
	enum hiddenOption {
	    OPT_GLOBAL,	OPT_NAMESPACE,	OPT_LAST
	} idx;

	namespaceName = NULL;
	for (i = 2; i < objc; i++) {
	    if (TclGetString(objv[i])[0] != '-') {
		break;
	    }
	    if (Tcl_GetIndexFromObj(interp, objv[i], hiddenOptions, "option",
		    0, &idx) != TCL_OK) {
		return TCL_ERROR;
	    }
	    if (idx == OPT_GLOBAL) {
		namespaceName = "::";
	    } else if (idx == OPT_NAMESPACE) {
		if (++i == objc) { /* There must be more arguments. */
		    break;
		}
		namespaceName = TclGetString(objv[i]);
	    } else {
		i++;
		break;
	    }
	}
	if (objc - i < 1) {
	    Tcl_WrongNumArgs(interp, 2, objv,
		    "?-namespace ns? ?-global? ?--? cmd ?arg ..?");
	    return TCL_ERROR;
	}
	return ChildInvokeHidden(interp, childInterp, namespaceName,
		objc - i, objv + i);
    }
    case OPT_LIMIT: {
	static const char *const limitTypes[] = {
	    "commands", "time", NULL
	};
	enum LimitTypes {
	    LIMIT_TYPE_COMMANDS, LIMIT_TYPE_TIME
	} limitType;

	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "limitType ?-option value ...?");
	    return TCL_ERROR;
	}
	if (Tcl_GetIndexFromObj(interp, objv[2], limitTypes, "limit type", 0,
		&limitType) != TCL_OK) {
	    return TCL_ERROR;
	}
	switch (limitType) {
	case LIMIT_TYPE_COMMANDS:

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInterp.c`, function `TclAliasCreate`, lines 1860–2024. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-9.1.0-tclInterp.c`.

```text
 *
 *----------------------------------------------------------------------
 */

int
TclAliasCreate(
    Tcl_Interp *interp,		/* Interp for error reporting. */
    Tcl_Interp *childInterp,	/* Interp where alias cmd will live or from
				 * which alias will be deleted. */
    Tcl_Interp *parentInterp,	/* Interp in which target command will be
				 * invoked. */
    Tcl_Obj *namePtr,		/* Name of alias cmd. */
    Tcl_Obj *targetCmdPtr,	/* Name of target cmd. */
    Tcl_Size objc,		/* Additional arguments to store */
    Tcl_Obj *const *objv)	/* with alias. */
{
    Alias *aliasPtr;
    Tcl_HashEntry *hPtr;
    Target *targetPtr;
    Child *childPtr;
    Parent *parentPtr;
    Tcl_Obj **prefv;
    int isNew;
    Tcl_Size i;

    aliasPtr = (Alias *) Tcl_Alloc(sizeof(Alias) + objc * sizeof(Tcl_Obj *));
    aliasPtr->token = namePtr;
    Tcl_IncrRefCount(aliasPtr->token);
    aliasPtr->targetInterp = parentInterp;

    aliasPtr->objc = objc + 1;
    prefv = &aliasPtr->objPtr;

    *prefv = targetCmdPtr;
    Tcl_IncrRefCount(targetCmdPtr);
    for (i = 0; i < objc; i++) {
	*(++prefv) = objv[i];
	Tcl_IncrRefCount(objv[i]);
    }

    Tcl_Preserve(childInterp);
    Tcl_Preserve(parentInterp);

    if (childInterp == parentInterp) {
	aliasPtr->childCmd = Tcl_NRCreateCommand2(childInterp,
		TclGetString(namePtr), TclLocalAliasObjCmd, AliasNRCmd,
		aliasPtr, AliasObjCmdDeleteProc);
    } else {
	aliasPtr->childCmd = Tcl_CreateObjCommand2(childInterp,
		TclGetString(namePtr), TclAliasObjCmd, aliasPtr,
		AliasObjCmdDeleteProc);
    }

    if (TclPreventAliasLoop(interp, childInterp,
	    aliasPtr->childCmd) != TCL_OK) {
	/*
	 * Found an alias loop! The last call to Tcl_CreateObjCommand made the
	 * alias point to itself. Delete the command and its alias record. Be
	 * careful to wipe out its client data first, so the command doesn't
	 * try to delete itself.
	 */

	Command *cmdPtr;

	Tcl_DecrRefCount(aliasPtr->token);
	Tcl_DecrRefCount(targetCmdPtr);
	for (i = 0; i < objc; i++) {
	    Tcl_DecrRefCount(objv[i]);
	}

	cmdPtr = (Command *) aliasPtr->childCmd;
	cmdPtr->clientData = NULL;
	cmdPtr->deleteProc = NULL;
	cmdPtr->deleteData = NULL;
	Tcl_DeleteCommandFromToken(childInterp, aliasPtr->childCmd);

	Tcl_Free(aliasPtr);

	/*
	 * The result was already set by TclPreventAliasLoop.
	 */

	Tcl_Release(childInterp);
	Tcl_Release(parentInterp);
	return TCL_ERROR;
    }

    /*
     * Make an entry in the alias table. If it already exists, retry.
     */

    childPtr = &INTERP_INFO(childInterp)->child;
    while (1) {
	Tcl_Obj *newToken;
	const char *string;

	string = TclGetString(aliasPtr->token);
	hPtr = Tcl_CreateHashEntry(&childPtr->aliasTable, string, &isNew);
	if (isNew != 0) {
	    break;
	}

	/*
	 * The alias name cannot be used as unique token, it is already taken.
	 * We can produce a unique token by prepending "::" repeatedly. This
	 * algorithm is a stop-gap to try to maintain the command name as
	 * token for most use cases, fearful of possible backwards compat
	 * problems. A better algorithm would produce unique tokens that need
	 * not be related to the command name.
	 *
	 * ATTENTION: the tests in interp.test and possibly safe.test depend
	 * on the precise definition of these tokens.
	 */

	TclNewLiteralStringObj(newToken, "::");
	Tcl_AppendObjToObj(newToken, aliasPtr->token);
	Tcl_DecrRefCount(aliasPtr->token);
	aliasPtr->token = newToken;
	Tcl_IncrRefCount(aliasPtr->token);
    }

    aliasPtr->aliasEntryPtr = hPtr;
    Tcl_SetHashValue(hPtr, aliasPtr);

    /*
     * Create the new command. We must do it after deleting any old command,
     * because the alias may be pointing at a renamed alias, as in:
     *
     * interp alias {} foo {} bar		# Create an alias "foo"
     * rename foo zop				# Now rename the alias
     * interp alias {} foo {} zop		# Now recreate "foo"...
     */

    targetPtr = (Target *) Tcl_Alloc(sizeof(Target));
    targetPtr->childCmd = aliasPtr->childCmd;
    targetPtr->childInterp = childInterp;

    parentPtr = &INTERP_INFO(parentInterp)->parent;
    targetPtr->nextPtr = parentPtr->targetsPtr;
    targetPtr->prevPtr = NULL;
    if (parentPtr->targetsPtr != NULL) {
	parentPtr->targetsPtr->prevPtr = targetPtr;
    }
    parentPtr->targetsPtr = targetPtr;
    aliasPtr->targetPtr = targetPtr;

    Tcl_SetObjResult(interp, aliasPtr->token);

    Tcl_Release(childInterp);
    Tcl_Release(parentInterp);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * AliasDelete --
 *
 *	Deletes the given alias from the child interpreter given.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	Deletes the alias from the child interpreter.

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole source retained`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_ReplaceHashEntry`, lines 897–1061. Retained reference `rust-tcl-registry-tests-data-native_jim_alias_error_extent279-request-jim.c`.

```text
    Jim_SetHashVal(ht, entry, val);
    return JIM_OK;
}

/* Add an element, discarding the old if the key already exists */
int Jim_ReplaceHashEntry(Jim_HashTable *ht, const void *key, void *val)
{
    int existed;
    Jim_HashEntry *entry;

    /* Get the index of the new element, or -1 if
     * the element already exists. */
    entry = JimInsertHashEntry(ht, key, 1);
    if (entry->key) {
        /* It already exists, so only replace the value.
         * Note if both a destructor and a duplicate function exist,
         * need to dup before destroy. perhaps they are the same
         * reference counted object
         */
        if (ht->type->valDestructor && ht->type->valDup) {
            void *newval = ht->type->valDup(ht->privdata, val);
            ht->type->valDestructor(ht->privdata, entry->u.val);
            entry->u.val = newval;
        }
        else {
            Jim_FreeEntryVal(ht, entry);
            Jim_SetHashVal(ht, entry, val);
        }
        existed = 1;
    }
    else {
        /* Doesn't exist, so set the key */
        Jim_SetHashKey(ht, entry, key);
        Jim_SetHashVal(ht, entry, val);
        existed = 0;
    }

    return existed;
}

/**
 * Search the hash table for the given key.
 * If found, removes the hash entry and returns JIM_OK.
 * Otherwise returns JIM_ERR.
 */
int Jim_DeleteHashEntry(Jim_HashTable *ht, const void *key)
{
    if (ht->used) {
        unsigned int h = Jim_HashKey(ht, key) & ht->sizemask;
        Jim_HashEntry *prevHe = NULL;
        Jim_HashEntry *he = ht->table[h];

        while (he) {
            if (Jim_CompareHashKeys(ht, key, he->key)) {
                /* Unlink the element from the list */
                if (prevHe)
                    prevHe->next = he->next;
                else
                    ht->table[h] = he->next;
                ht->used--;
                Jim_FreeEntryKey(ht, he);
                Jim_FreeEntryVal(ht, he);
                Jim_Free(he);
                return JIM_OK;
            }
            prevHe = he;
            he = he->next;
        }
    }
    /* not found */
    return JIM_ERR;
}

/**
 * Clear all hash entries from the table, but don't free
 * the table.
 */
void Jim_ClearHashTable(Jim_HashTable *ht)
{
    unsigned int i;

    /* Free all the elements */
    for (i = 0; ht->used > 0; i++) {
        Jim_HashEntry *he, *nextHe;

        he = ht->table[i];
        while (he) {
            nextHe = he->next;
            Jim_FreeEntryKey(ht, he);
            Jim_FreeEntryVal(ht, he);
            Jim_Free(he);
            ht->used--;
            he = nextHe;
        }
        ht->table[i] = NULL;
    }
}

/* Remove all entries from the hash table
 * and leave it empty for reuse
 */
int Jim_FreeHashTable(Jim_HashTable *ht)
{
    Jim_ClearHashTable(ht);
    /* Free the table and the allocated cache structure */
    Jim_Free(ht->table);
    /* Re-initialize the table */
    JimResetHashTable(ht);
    return JIM_OK;              /* never fails */
}

Jim_HashEntry *Jim_FindHashEntry(Jim_HashTable *ht, const void *key)
{
    Jim_HashEntry *he;
    unsigned int h;

    if (ht->used == 0)
        return NULL;
    h = Jim_HashKey(ht, key) & ht->sizemask;
    he = ht->table[h];
    while (he) {
        if (Jim_CompareHashKeys(ht, key, he->key))
            return he;
        he = he->next;
    }
    return NULL;
}

Jim_HashTableIterator *Jim_GetHashTableIterator(Jim_HashTable *ht)
{
    Jim_HashTableIterator *iter = Jim_Alloc(sizeof(*iter));
    JimInitHashTableIterator(ht, iter);
    return iter;
}

Jim_HashEntry *Jim_NextHashEntry(Jim_HashTableIterator *iter)
{
    while (1) {
        if (iter->entry == NULL) {
            iter->index++;
            if (iter->index >= (signed)iter->ht->size)
                break;
            iter->entry = iter->ht->table[iter->index];
        }
        else {
            iter->entry = iter->nextEntry;
        }
        if (iter->entry) {
            /* We need to save the 'next' here, the iterator user
             * may delete the entry we are returning. */
            iter->nextEntry = iter->entry->next;
            return iter->entry;
        }
    }
    return NULL;
}

/* ------------------------- private functions ------------------------------ */

/* Expand the hash table if needed */
static void JimExpandHashTableIfNeeded(Jim_HashTable *ht)
{
    /* If the hash table is empty expand it to the intial size,
     * if the table is "full" double its size. */
    if (ht->size == 0)

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole source retained`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimObjectHTKeyValDup`, lines 3988–4152. Retained reference `rust-tcl-registry-tests-data-native_jim_alias_error_extent279-request-jim.c`.

```text
static int JimObjectHTKeyCompare(void *privdata, const void *key1, const void *key2)
{
    return Jim_StringEqObj((Jim_Obj *)key1, (Jim_Obj *)key2);
}

static void *JimObjectHTKeyValDup(void *privdata, const void *val)
{
    Jim_IncrRefCount((Jim_Obj *)val);
    return (void *)val;
}

/* Destroy an object-table value and release its referenced object. */
static void JimObjectHTKeyValDestructor(void *interp, void *val)
{
    Jim_DecrRefCount(interp, (Jim_Obj *)val);
}

static void *JimVariablesHTValDup(void *privdata, const void *val)
{
    JimIncrVarRef((Jim_VarVal *)val);
    return (void *)val;
}

static const Jim_HashTableType JimVariablesHashTableType = {
    JimObjectHTHashFunction, /* hash function */
    JimObjectHTKeyValDup,       /* key dup */
    JimVariablesHTValDup,       /* val dup */
    JimObjectHTKeyCompare,      /* key compare */
    JimObjectHTKeyValDestructor,    /* key destructor */
    JimVariablesHTValDestructor /* val destructor */
};

/* Commands HashTable Type.
 *
 * Keys are Jim Objects where any leading namespace qualifier
 * is ignored. Values are Jim_Cmd structures.
 */

/**
 * Like Jim_GetString() but strips any leading namespace qualifier.
 */
static const char *Jim_GetStringNoQualifier(Jim_Obj *objPtr, int *length)
{
    int len;
    const char *str = Jim_GetString(objPtr, &len);
    if (len >= 2 && str[0] == ':' && str[1] == ':') {
        while (len && *str == ':') {
            len--;
            str++;
        }
    }
    *length = len;
    return str;
}

/* Hash a command-table key, ignoring any namespace prefix. */
static unsigned int JimCommandsHT_HashFunction(const void *key)
{
    int len;
    const char *str = Jim_GetStringNoQualifier((Jim_Obj *)key, &len);
    return Jim_GenHashFunction((const unsigned char *)str, len);
}

/* Compare command names, allowing namespace-qualified lookups to match local entries. */
static int JimCommandsHT_KeyCompare(void *privdata, const void *key1, const void *key2)
{
    int len1, len2;
    const char *str1 = Jim_GetStringNoQualifier((Jim_Obj *)key1, &len1);
    const char *str2 = Jim_GetStringNoQualifier((Jim_Obj *)key2, &len2);
    return len1 == len2 && *str1 == *str2 && memcmp(str1, str2, len1) == 0;
}

/* Destroy a command-table value. */
static void JimCommandsHT_ValDestructor(void *interp, void *val)
{
    JimDecrCmdRefCount(interp, val);
}

static const Jim_HashTableType JimCommandsHashTableType = {
    JimCommandsHT_HashFunction,    /* hash function */
    JimObjectHTKeyValDup,       /* key dup */
    NULL,                           /* val dup */
    JimCommandsHT_KeyCompare,      /* key compare */
    JimObjectHTKeyValDestructor,    /* key destructor */
    JimCommandsHT_ValDestructor     /* val destructor */
};

/* ------------------------- Commands related functions --------------------- */

/**
 * If nameObjPtr starts with "::", returns it.
 * Otherwise returns a new object with nameObjPtr prefixed with "::".
 * In this case, decrements the ref count of nameObjPtr.
 */
Jim_Obj *Jim_MakeGlobalNamespaceName(Jim_Interp *interp, Jim_Obj *nameObjPtr)
{
#ifdef jim_ext_namespace
    Jim_Obj *resultObj;

    const char *name = Jim_String(nameObjPtr);
    if (name[0] == ':' && name[1] == ':') {
        return nameObjPtr;
    }
    Jim_IncrRefCount(nameObjPtr);
    resultObj = Jim_NewStringObj(interp, "::", -1);
    Jim_AppendObj(interp, resultObj, nameObjPtr);
    Jim_DecrRefCount(interp, nameObjPtr);

    return resultObj;
#else
    return nameObjPtr;
#endif
}

/**
 * If the name in objPtr is not fully qualified, and a non-global namespace
 * is in effect, qualifies the name with the current namespace and returns the new name.
 * Otherwise returns objPtr.
 *
 * In either case the ref count is incremented and should be decremented by the caller.
 * with Jim_DecrRefCount()
 */
static Jim_Obj *JimQualifyName(Jim_Interp *interp, Jim_Obj *objPtr)
{
#ifdef jim_ext_namespace
    if (Jim_Length(interp->framePtr->nsObj)) {
        int len;
        const char *name = Jim_GetString(objPtr, &len);
        if (len < 2 || name[0] != ':' || name[1] != ':') {
            /* OK. Need to qualify this name */
            objPtr = Jim_DuplicateObj(interp, interp->framePtr->nsObj);
            Jim_AppendStrings(interp, objPtr, "::", name, NULL);
        }
    }
#endif
    Jim_IncrRefCount(objPtr);
    return objPtr;
}

/**
 * Add the command to the commands hash table
 */
static void JimCreateCommand(Jim_Interp *interp, Jim_Obj *nameObjPtr, Jim_Cmd *cmd)
{
    /* If the entry already exists, nameObjPtr will not be used,
     * so the refCount of nameObjPtr can't be zero, relying on this function to
     * release it in that case.
     */
    JimPanic((nameObjPtr->refCount == 0, "JimCreateCommand called with zero ref count name"));

    /* It may already exist, so we try to delete the old one.
     * Note that reference count means that it won't be deleted yet if
     * it exists in the call stack.
     *
     * BUT, if 'local' is in force, instead of deleting the existing
     * proc, we stash a reference to the old proc here.
     */
    if (interp->local) {
        Jim_HashEntry *he = Jim_FindHashEntry(&interp->commands, nameObjPtr);
        if (he) {
            /* Push this command over the top of the previous one */
            cmd->prevCmd = Jim_GetHashEntryVal(he);
            Jim_SetHashVal(&interp->commands, he, cmd);
            /* Need to increment the proc epoch here so that the new command will be used */

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole source retained`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimCreateCommand`, lines 4126–4290. Retained reference `rust-tcl-registry-tests-data-native_jim_alias_error_extent279-request-jim.c`.

```text
}

/**
 * Add the command to the commands hash table
 */
static void JimCreateCommand(Jim_Interp *interp, Jim_Obj *nameObjPtr, Jim_Cmd *cmd)
{
    /* If the entry already exists, nameObjPtr will not be used,
     * so the refCount of nameObjPtr can't be zero, relying on this function to
     * release it in that case.
     */
    JimPanic((nameObjPtr->refCount == 0, "JimCreateCommand called with zero ref count name"));

    /* It may already exist, so we try to delete the old one.
     * Note that reference count means that it won't be deleted yet if
     * it exists in the call stack.
     *
     * BUT, if 'local' is in force, instead of deleting the existing
     * proc, we stash a reference to the old proc here.
     */
    if (interp->local) {
        Jim_HashEntry *he = Jim_FindHashEntry(&interp->commands, nameObjPtr);
        if (he) {
            /* Push this command over the top of the previous one */
            cmd->prevCmd = Jim_GetHashEntryVal(he);
            Jim_SetHashVal(&interp->commands, he, cmd);
            /* Need to increment the proc epoch here so that the new command will be used */
            Jim_InterpIncrProcEpoch(interp);
            return;
        }
    }

    /* Otherwise simply replace any existing command */

    /* Note that it is not necessary to increment the 'proc epoch' because any
     * existing command that is replaced will be held as a negative cache entry
     * until the next time the proc epoch is incremented.
     */
    Jim_ReplaceHashEntry(&interp->commands, nameObjPtr, cmd);
}

/* commands */
Jim_Cmd *Jim_RegisterCommand(Jim_Interp *interp, Jim_Obj *cmdNameObj,
    Jim_CmdProc *cmdProc,
    Jim_DelCmdProc *delProc,
    const char *usage,
    const char *help,
    short minargs,
    short maxargs,
    int flags,
    void *privData)
{
    JimPanic(((flags & JIM_CMD_ISPROC), "Jim_RegisterCommand called with JIM_CMD_ISPROC flag"));
    Jim_Cmd *cmdPtr = Jim_Alloc(sizeof(*cmdPtr));

    /* Store the new details for this command */
    memset(cmdPtr, 0, sizeof(*cmdPtr));
    cmdPtr->inUse = 1;
    cmdPtr->flags = flags;
    cmdPtr->u.native.delProc = delProc;
    cmdPtr->u.native.cmdProc = cmdProc;
    cmdPtr->u.native.usage = usage;
    cmdPtr->u.native.help = help;
    cmdPtr->u.native.minargs = minargs;
    cmdPtr->u.native.maxargs = maxargs;
    cmdPtr->u.native.privData = privData;

    Jim_IncrRefCount(cmdNameObj);
    JimCreateCommand(interp, cmdNameObj, cmdPtr);
    Jim_DecrRefCount(interp, cmdNameObj);

    return cmdPtr;
}

/* This remains for backward compatiblity to create a command
 * with no arg limitations or usage/help.
 * Prefer one of the Jim_Register* variants instead.
 */
int Jim_CreateCommand(Jim_Interp *interp, const char *cmdNameStr,
    Jim_CmdProc *cmdProc, void *privData, Jim_DelCmdProc *delProc)
{
    Jim_RegisterCmd(interp, cmdNameStr, NULL, 0, -1, cmdProc, delProc, privData, 0);
    return JIM_OK;
}

/* Create the named static variables declared for a proc body. */
static int JimCreateProcedureStatics(Jim_Interp *interp, Jim_Cmd *cmdPtr, Jim_Obj *staticsListObjPtr)
{
    int len, i;

    len = Jim_ListLength(interp, staticsListObjPtr);
    if (len == 0) {
        return JIM_OK;
    }

    cmdPtr->u.proc.staticVars = Jim_Alloc(sizeof(Jim_HashTable));
    Jim_InitHashTable(cmdPtr->u.proc.staticVars, &JimVariablesHashTableType, interp);
    for (i = 0; i < len; i++) {
        Jim_Obj *initObjPtr = NULL;
        Jim_Obj *nameObjPtr;
        Jim_VarVal *vv = NULL;
        Jim_Obj *objPtr = Jim_ListGetIndex(interp, staticsListObjPtr, i);
        int subLen = Jim_ListLength(interp, objPtr);
        int byref = 0;

        /* Check if it's composed of two elements. */
        if (subLen != 1 && subLen != 2) {
            Jim_SetResultFormatted(interp, "too many fields in static specifier \"%#s\"",
                objPtr);
            return JIM_ERR;
        }

        nameObjPtr = Jim_ListGetIndex(interp, objPtr, 0);

        /* How to intialise or link? */
        if (subLen == 1) {
            int len;
            const char *pt = Jim_GetString(nameObjPtr, &len);
            if (*pt == '&') {
                /* Create as a reference */
                nameObjPtr = Jim_NewStringObj(interp, pt + 1, len - 1);
                byref = 1;
            }
        }
        Jim_IncrRefCount(nameObjPtr);

        if (subLen == 1) {
            switch (SetVariableFromAny(interp, nameObjPtr)) {
                case JIM_DICT_SUGAR:
                    /* XXX: This message seem unnecessarily verbose, but it matches Tcl */
                    if (byref) {
                        Jim_SetResultFormatted(interp, "Can't link to array element \"%#s\"", nameObjPtr);
                    }
                    else {
                        Jim_SetResultFormatted(interp, "Can't initialise array element \"%#s\"", nameObjPtr);
                    }
                    Jim_DecrRefCount(interp, nameObjPtr);
                    return JIM_ERR;

                case JIM_OK:
                    if (byref) {
                        vv = nameObjPtr->internalRep.varValue.vv;
                    }
                    else {
                        initObjPtr = Jim_GetVariable(interp, nameObjPtr, JIM_NONE);
                        if (!initObjPtr) {
                            Jim_SetResultFormatted(interp, "Could not resolve upvar \"%#s\"'s value", nameObjPtr);
                            Jim_DecrRefCount(interp, nameObjPtr);
                            return JIM_ERR;
                        }
                    }
                    break;

                case JIM_ERR:
                    /* Doesn't exist */
                    Jim_SetResultFormatted(interp,
                        "variable for initialization of static \"%#s\" not found in the local context",
                        nameObjPtr);
                    Jim_DecrRefCount(interp, nameObjPtr);
                    return JIM_ERR;
            }
        }
        else {
            initObjPtr = Jim_ListGetIndex(interp, objPtr, 1);
        }

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole source retained`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_RegisterCommand`, lines 4163–4327. Retained reference `rust-tcl-registry-tests-data-native_jim_alias_error_extent279-request-jim.c`.

```text
     */
    Jim_ReplaceHashEntry(&interp->commands, nameObjPtr, cmd);
}

/* commands */
Jim_Cmd *Jim_RegisterCommand(Jim_Interp *interp, Jim_Obj *cmdNameObj,
    Jim_CmdProc *cmdProc,
    Jim_DelCmdProc *delProc,
    const char *usage,
    const char *help,
    short minargs,
    short maxargs,
    int flags,
    void *privData)
{
    JimPanic(((flags & JIM_CMD_ISPROC), "Jim_RegisterCommand called with JIM_CMD_ISPROC flag"));
    Jim_Cmd *cmdPtr = Jim_Alloc(sizeof(*cmdPtr));

    /* Store the new details for this command */
    memset(cmdPtr, 0, sizeof(*cmdPtr));
    cmdPtr->inUse = 1;
    cmdPtr->flags = flags;
    cmdPtr->u.native.delProc = delProc;
    cmdPtr->u.native.cmdProc = cmdProc;
    cmdPtr->u.native.usage = usage;
    cmdPtr->u.native.help = help;
    cmdPtr->u.native.minargs = minargs;
    cmdPtr->u.native.maxargs = maxargs;
    cmdPtr->u.native.privData = privData;

    Jim_IncrRefCount(cmdNameObj);
    JimCreateCommand(interp, cmdNameObj, cmdPtr);
    Jim_DecrRefCount(interp, cmdNameObj);

    return cmdPtr;
}

/* This remains for backward compatiblity to create a command
 * with no arg limitations or usage/help.
 * Prefer one of the Jim_Register* variants instead.
 */
int Jim_CreateCommand(Jim_Interp *interp, const char *cmdNameStr,
    Jim_CmdProc *cmdProc, void *privData, Jim_DelCmdProc *delProc)
{
    Jim_RegisterCmd(interp, cmdNameStr, NULL, 0, -1, cmdProc, delProc, privData, 0);
    return JIM_OK;
}

/* Create the named static variables declared for a proc body. */
static int JimCreateProcedureStatics(Jim_Interp *interp, Jim_Cmd *cmdPtr, Jim_Obj *staticsListObjPtr)
{
    int len, i;

    len = Jim_ListLength(interp, staticsListObjPtr);
    if (len == 0) {
        return JIM_OK;
    }

    cmdPtr->u.proc.staticVars = Jim_Alloc(sizeof(Jim_HashTable));
    Jim_InitHashTable(cmdPtr->u.proc.staticVars, &JimVariablesHashTableType, interp);
    for (i = 0; i < len; i++) {
        Jim_Obj *initObjPtr = NULL;
        Jim_Obj *nameObjPtr;
        Jim_VarVal *vv = NULL;
        Jim_Obj *objPtr = Jim_ListGetIndex(interp, staticsListObjPtr, i);
        int subLen = Jim_ListLength(interp, objPtr);
        int byref = 0;

        /* Check if it's composed of two elements. */
        if (subLen != 1 && subLen != 2) {
            Jim_SetResultFormatted(interp, "too many fields in static specifier \"%#s\"",
                objPtr);
            return JIM_ERR;
        }

        nameObjPtr = Jim_ListGetIndex(interp, objPtr, 0);

        /* How to intialise or link? */
        if (subLen == 1) {
            int len;
            const char *pt = Jim_GetString(nameObjPtr, &len);
            if (*pt == '&') {
                /* Create as a reference */
                nameObjPtr = Jim_NewStringObj(interp, pt + 1, len - 1);
                byref = 1;
            }
        }
        Jim_IncrRefCount(nameObjPtr);

        if (subLen == 1) {
            switch (SetVariableFromAny(interp, nameObjPtr)) {
                case JIM_DICT_SUGAR:
                    /* XXX: This message seem unnecessarily verbose, but it matches Tcl */
                    if (byref) {
                        Jim_SetResultFormatted(interp, "Can't link to array element \"%#s\"", nameObjPtr);
                    }
                    else {
                        Jim_SetResultFormatted(interp, "Can't initialise array element \"%#s\"", nameObjPtr);
                    }
                    Jim_DecrRefCount(interp, nameObjPtr);
                    return JIM_ERR;

                case JIM_OK:
                    if (byref) {
                        vv = nameObjPtr->internalRep.varValue.vv;
                    }
                    else {
                        initObjPtr = Jim_GetVariable(interp, nameObjPtr, JIM_NONE);
                        if (!initObjPtr) {
                            Jim_SetResultFormatted(interp, "Could not resolve upvar \"%#s\"'s value", nameObjPtr);
                            Jim_DecrRefCount(interp, nameObjPtr);
                            return JIM_ERR;
                        }
                    }
                    break;

                case JIM_ERR:
                    /* Doesn't exist */
                    Jim_SetResultFormatted(interp,
                        "variable for initialization of static \"%#s\" not found in the local context",
                        nameObjPtr);
                    Jim_DecrRefCount(interp, nameObjPtr);
                    return JIM_ERR;
            }
        }
        else {
            initObjPtr = Jim_ListGetIndex(interp, objPtr, 1);
        }

        if (vv == NULL) {
            vv = Jim_Alloc(sizeof(*vv));
            vv->objPtr = initObjPtr;
            Jim_IncrRefCount(vv->objPtr);
            vv->linkFramePtr = NULL;
            vv->refCount = 0;
        }

        if (JimSetNewVariable(cmdPtr->u.proc.staticVars, nameObjPtr, vv) != JIM_OK) {
            Jim_SetResultFormatted(interp,
                "static variable name \"%#s\" duplicated in statics list", nameObjPtr);
            JimIncrVarRef(vv);
            JimDecrVarRef(interp, vv);
            Jim_DecrRefCount(interp, nameObjPtr);
            return JIM_ERR;
        }

        Jim_DecrRefCount(interp, nameObjPtr);
    }
    return JIM_OK;
}

/* memrchr() is not standard */
#ifdef jim_ext_namespace
static const char *Jim_memrchr(const char *p, int c, int len)
{
    int i;
    for (i = len; i > 0; i--) {
        if (p[i] == c) {
            return p + i;
        }
    }
    return NULL;
}
#endif

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole source retained`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_DeleteCommand`, lines 4442–4606. Retained reference `rust-tcl-registry-tests-data-native_jim_alias_error_extent279-request-jim.c`.

```text
    }

    return cmdPtr;
}

int Jim_DeleteCommand(Jim_Interp *interp, Jim_Obj *nameObj)
{
    int ret = JIM_OK;

    nameObj = JimQualifyName(interp, nameObj);

    if (Jim_DeleteHashEntry(&interp->commands, nameObj) == JIM_ERR) {
        Jim_SetResultFormatted(interp, "can't delete \"%#s\": command doesn't exist", nameObj);
        ret = JIM_ERR;
    }
    Jim_DecrRefCount(interp, nameObj);

    return ret;
}

int Jim_RenameCommand(Jim_Interp *interp, Jim_Obj *oldNameObj, Jim_Obj *newNameObj)
{
    int ret = JIM_ERR;
    Jim_HashEntry *he;
    Jim_Cmd *cmdPtr;

    if (Jim_Length(newNameObj) == 0) {
        return Jim_DeleteCommand(interp, oldNameObj);
    }

    /* each name may need to have the current namespace added to it */

    oldNameObj = JimQualifyName(interp, oldNameObj);
    newNameObj = JimQualifyName(interp, newNameObj);

    /* Does it exist? */
    he = Jim_FindHashEntry(&interp->commands, oldNameObj);
    if (he == NULL) {
        Jim_SetResultFormatted(interp, "can't rename \"%#s\": command doesn't exist", oldNameObj);
    }
    else if (Jim_FindHashEntry(&interp->commands, newNameObj)) {
        Jim_SetResultFormatted(interp, "can't rename to \"%#s\": command already exists", newNameObj);
    }
    else {
        cmdPtr = Jim_GetHashEntryVal(he);
        if (cmdPtr->prevCmd) {
            /* If the command replaced another command with 'local', renaming it
             * would break the usage of upcall, so don't allow it.
             */
            Jim_SetResultFormatted(interp, "can't rename local command \"%#s\"", oldNameObj);
        }
        else {
            /* Add the new name first */
            JimIncrCmdRefCount(cmdPtr);
            JimUpdateProcNamespace(interp, cmdPtr, newNameObj);
            Jim_AddHashEntry(&interp->commands, newNameObj, cmdPtr);

            /* Now remove the old name */
            Jim_DeleteHashEntry(&interp->commands, oldNameObj);

            /* Increment the epoch */
            Jim_InterpIncrProcEpoch(interp);

            ret = JIM_OK;
        }
    }

    Jim_DecrRefCount(interp, oldNameObj);
    Jim_DecrRefCount(interp, newNameObj);

    return ret;
}

/* -----------------------------------------------------------------------------
 * Command object
 * ---------------------------------------------------------------------------*/

static void FreeCommandInternalRep(Jim_Interp *interp, Jim_Obj *objPtr)
{
    Jim_DecrRefCount(interp, objPtr->internalRep.cmdValue.nsObj);
}

/* Duplicate the cached namespace information for a resolved command object. */
static void DupCommandInternalRep(Jim_Interp *interp, Jim_Obj *srcPtr, Jim_Obj *dupPtr)
{
    dupPtr->internalRep.cmdValue = srcPtr->internalRep.cmdValue;
    dupPtr->typePtr = srcPtr->typePtr;
    Jim_IncrRefCount(dupPtr->internalRep.cmdValue.nsObj);
}

static const Jim_ObjType commandObjType = {
    "command",
    FreeCommandInternalRep,
    DupCommandInternalRep,
    NULL,
    JIM_TYPE_REFERENCES,
};

/* This function returns the command structure for the command name
 * stored in objPtr. It specializes the objPtr to contain
 * cached info instead of performing the lookup into the hash table
 * every time. The information cached may not be up-to-date, in this
 * case the lookup is performed and the cache updated.
 *
 * Respects the 'upcall' setting.
 */
Jim_Cmd *Jim_GetCommand(Jim_Interp *interp, Jim_Obj *objPtr, int flags)
{
    Jim_Cmd *cmd;

    /* In order to be valid, the proc epoch must match and
     * the lookup must have occurred in the same namespace
     */
    if (objPtr->typePtr == &commandObjType
        && objPtr->internalRep.cmdValue.procEpoch == interp->procEpoch
#ifdef jim_ext_namespace
        && Jim_StringEqObj(objPtr->internalRep.cmdValue.nsObj, interp->framePtr->nsObj)
#endif
        && objPtr->internalRep.cmdValue.cmdPtr->inUse) {
        /* Cached value is valid */
        cmd = objPtr->internalRep.cmdValue.cmdPtr;
    }
    else {
        Jim_Obj *qualifiedNameObj = JimQualifyName(interp, objPtr);
        Jim_HashEntry *he = Jim_FindHashEntry(&interp->commands, qualifiedNameObj);
#ifdef jim_ext_namespace
        if (he == NULL && Jim_Length(interp->framePtr->nsObj)) {
            he = Jim_FindHashEntry(&interp->commands, objPtr);
        }
#endif
        if (he == NULL) {
            if (flags & JIM_ERRMSG) {
                Jim_SetResultFormatted(interp, "invalid command name \"%#s\"", objPtr);
            }
            Jim_DecrRefCount(interp, qualifiedNameObj);
            return NULL;
        }
        cmd = Jim_GetHashEntryVal(he);

        /* Stash the resolved command name. Note that we don't incr the ref count here
         * since we don't want to increase the ref count. The lifetime is the same as the key
         * in the commands hash table
         */
        cmd->cmdNameObj = Jim_GetHashEntryKey(he);

        /* Free the old internal rep and set the new one. */
        Jim_FreeIntRep(interp, objPtr);
        objPtr->typePtr = &commandObjType;
        objPtr->internalRep.cmdValue.procEpoch = interp->procEpoch;
        objPtr->internalRep.cmdValue.cmdPtr = cmd;
        objPtr->internalRep.cmdValue.nsObj = interp->framePtr->nsObj;
        Jim_IncrRefCount(interp->framePtr->nsObj);
        Jim_DecrRefCount(interp, qualifiedNameObj);
    }
    while ((cmd->flags & JIM_CMD_ISPROC) && cmd->u.proc.upcall) {
        cmd = cmd->prevCmd;
    }
    return cmd;
}

/* -----------------------------------------------------------------------------
 * Variables
 * ---------------------------------------------------------------------------*/

/* -----------------------------------------------------------------------------

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole source retained`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_RenameCommand`, lines 4457–4621. Retained reference `rust-tcl-registry-tests-data-native_jim_alias_error_extent279-request-jim.c`.

```text
    Jim_DecrRefCount(interp, nameObj);

    return ret;
}

int Jim_RenameCommand(Jim_Interp *interp, Jim_Obj *oldNameObj, Jim_Obj *newNameObj)
{
    int ret = JIM_ERR;
    Jim_HashEntry *he;
    Jim_Cmd *cmdPtr;

    if (Jim_Length(newNameObj) == 0) {
        return Jim_DeleteCommand(interp, oldNameObj);
    }

    /* each name may need to have the current namespace added to it */

    oldNameObj = JimQualifyName(interp, oldNameObj);
    newNameObj = JimQualifyName(interp, newNameObj);

    /* Does it exist? */
    he = Jim_FindHashEntry(&interp->commands, oldNameObj);
    if (he == NULL) {
        Jim_SetResultFormatted(interp, "can't rename \"%#s\": command doesn't exist", oldNameObj);
    }
    else if (Jim_FindHashEntry(&interp->commands, newNameObj)) {
        Jim_SetResultFormatted(interp, "can't rename to \"%#s\": command already exists", newNameObj);
    }
    else {
        cmdPtr = Jim_GetHashEntryVal(he);
        if (cmdPtr->prevCmd) {
            /* If the command replaced another command with 'local', renaming it
             * would break the usage of upcall, so don't allow it.
             */
            Jim_SetResultFormatted(interp, "can't rename local command \"%#s\"", oldNameObj);
        }
        else {
            /* Add the new name first */
            JimIncrCmdRefCount(cmdPtr);
            JimUpdateProcNamespace(interp, cmdPtr, newNameObj);
            Jim_AddHashEntry(&interp->commands, newNameObj, cmdPtr);

            /* Now remove the old name */
            Jim_DeleteHashEntry(&interp->commands, oldNameObj);

            /* Increment the epoch */
            Jim_InterpIncrProcEpoch(interp);

            ret = JIM_OK;
        }
    }

    Jim_DecrRefCount(interp, oldNameObj);
    Jim_DecrRefCount(interp, newNameObj);

    return ret;
}

/* -----------------------------------------------------------------------------
 * Command object
 * ---------------------------------------------------------------------------*/

static void FreeCommandInternalRep(Jim_Interp *interp, Jim_Obj *objPtr)
{
    Jim_DecrRefCount(interp, objPtr->internalRep.cmdValue.nsObj);
}

/* Duplicate the cached namespace information for a resolved command object. */
static void DupCommandInternalRep(Jim_Interp *interp, Jim_Obj *srcPtr, Jim_Obj *dupPtr)
{
    dupPtr->internalRep.cmdValue = srcPtr->internalRep.cmdValue;
    dupPtr->typePtr = srcPtr->typePtr;
    Jim_IncrRefCount(dupPtr->internalRep.cmdValue.nsObj);
}

static const Jim_ObjType commandObjType = {
    "command",
    FreeCommandInternalRep,
    DupCommandInternalRep,
    NULL,
    JIM_TYPE_REFERENCES,
};

/* This function returns the command structure for the command name
 * stored in objPtr. It specializes the objPtr to contain
 * cached info instead of performing the lookup into the hash table
 * every time. The information cached may not be up-to-date, in this
 * case the lookup is performed and the cache updated.
 *
 * Respects the 'upcall' setting.
 */
Jim_Cmd *Jim_GetCommand(Jim_Interp *interp, Jim_Obj *objPtr, int flags)
{
    Jim_Cmd *cmd;

    /* In order to be valid, the proc epoch must match and
     * the lookup must have occurred in the same namespace
     */
    if (objPtr->typePtr == &commandObjType
        && objPtr->internalRep.cmdValue.procEpoch == interp->procEpoch
#ifdef jim_ext_namespace
        && Jim_StringEqObj(objPtr->internalRep.cmdValue.nsObj, interp->framePtr->nsObj)
#endif
        && objPtr->internalRep.cmdValue.cmdPtr->inUse) {
        /* Cached value is valid */
        cmd = objPtr->internalRep.cmdValue.cmdPtr;
    }
    else {
        Jim_Obj *qualifiedNameObj = JimQualifyName(interp, objPtr);
        Jim_HashEntry *he = Jim_FindHashEntry(&interp->commands, qualifiedNameObj);
#ifdef jim_ext_namespace
        if (he == NULL && Jim_Length(interp->framePtr->nsObj)) {
            he = Jim_FindHashEntry(&interp->commands, objPtr);
        }
#endif
        if (he == NULL) {
            if (flags & JIM_ERRMSG) {
                Jim_SetResultFormatted(interp, "invalid command name \"%#s\"", objPtr);
            }
            Jim_DecrRefCount(interp, qualifiedNameObj);
            return NULL;
        }
        cmd = Jim_GetHashEntryVal(he);

        /* Stash the resolved command name. Note that we don't incr the ref count here
         * since we don't want to increase the ref count. The lifetime is the same as the key
         * in the commands hash table
         */
        cmd->cmdNameObj = Jim_GetHashEntryKey(he);

        /* Free the old internal rep and set the new one. */
        Jim_FreeIntRep(interp, objPtr);
        objPtr->typePtr = &commandObjType;
        objPtr->internalRep.cmdValue.procEpoch = interp->procEpoch;
        objPtr->internalRep.cmdValue.cmdPtr = cmd;
        objPtr->internalRep.cmdValue.nsObj = interp->framePtr->nsObj;
        Jim_IncrRefCount(interp->framePtr->nsObj);
        Jim_DecrRefCount(interp, qualifiedNameObj);
    }
    while ((cmd->flags & JIM_CMD_ISPROC) && cmd->u.proc.upcall) {
        cmd = cmd->prevCmd;
    }
    return cmd;
}

/* -----------------------------------------------------------------------------
 * Variables
 * ---------------------------------------------------------------------------*/

/* -----------------------------------------------------------------------------
 * Variable object
 * ---------------------------------------------------------------------------*/

static const Jim_ObjType variableObjType = {
    "variable",
    NULL,
    NULL,
    NULL,
    JIM_TYPE_REFERENCES,
};

/* This method should be called only by the variable API.
 * It returns JIM_OK on success (variable already exists),
 * JIM_ERR if it does not exist, JIM_DICT_SUGAR if it's not
 * a variable name, but syntax glue for [dict] i.e. the last

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole source retained`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_AliasCoreCommand`, lines 14433–14597. Retained reference `rust-tcl-registry-tests-data-native_jim_alias_error_extent279-request-jim.c`.

```text
    Jim_Obj *prefixListObj = privData;
    Jim_DecrRefCount(interp, prefixListObj);
}

/* Implement the alias command that creates command-prefix aliases. */
static int Jim_AliasCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_Obj *prefixListObj = Jim_NewListObj(interp, argv + 2, argc - 2);
    Jim_IncrRefCount(prefixListObj);
    Jim_SetResult(interp, argv[1]);

    Jim_RegisterCommand(interp, argv[1], JimAliasCmd, JimAliasCmdDelete, NULL, NULL, 0, -1, JIM_CMD_ISALIAS, prefixListObj);
    return JIM_OK;
}

/* [proc] */
static int Jim_ProcCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_Cmd *cmd;

    if (argc == 4) {
        cmd = JimCreateProcedureCmd(interp, argv[2], NULL, argv[3], NULL);
    }
    else {
        cmd = JimCreateProcedureCmd(interp, argv[2], argv[3], argv[4], NULL);
    }

    if (cmd) {
        /* Add the new command */
        Jim_Obj *nameObjPtr = JimQualifyName(interp, argv[1]);
        JimCreateCommand(interp, nameObjPtr, cmd);

        /* Calculate and set the namespace for this proc */
        JimUpdateProcNamespace(interp, cmd, nameObjPtr);
        Jim_DecrRefCount(interp, nameObjPtr);

        /* Unlike Tcl, set the name of the proc as the result */
        Jim_SetResult(interp, argv[1]);
        return JIM_OK;
    }
    return JIM_ERR;
}

/* [xtrace] */
static int Jim_XtraceCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    if (interp->traceCmdObj) {
        Jim_DecrRefCount(interp, interp->traceCmdObj);
        interp->traceCmdObj = NULL;
    }

    if (Jim_Length(argv[1])) {
        /* Install the new execution trace callback */
        interp->traceCmdObj = argv[1];
        Jim_IncrRefCount(interp->traceCmdObj);
    }
    return JIM_OK;
}

/* [local] */
static int Jim_LocalCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int retcode;

    /* Evaluate the arguments with 'local' in force */
    interp->local++;
    retcode = Jim_EvalObjVector(interp, argc - 1, argv + 1);
    interp->local--;

    /* If OK, and the result is a proc, add it to the list of local procs */
    if (retcode == 0) {
        Jim_Obj *cmdNameObj = Jim_GetResult(interp);

        if (Jim_GetCommand(interp, cmdNameObj, JIM_ERRMSG) == NULL) {
            return JIM_ERR;
        }
        if (interp->framePtr->localCommands == NULL) {
            interp->framePtr->localCommands = Jim_Alloc(sizeof(*interp->framePtr->localCommands));
            Jim_StackInit(interp->framePtr->localCommands, NULL);
        }
        Jim_IncrRefCount(cmdNameObj);
        Jim_StackPush(interp->framePtr->localCommands, cmdNameObj);
    }

    return retcode;
}

/* [upcall] */
static int Jim_UpcallCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int retcode;

    Jim_Cmd *cmdPtr = Jim_GetCommand(interp, argv[1], JIM_ERRMSG);
    if (cmdPtr == NULL || !(cmdPtr->flags & JIM_CMD_ISPROC) || !cmdPtr->prevCmd) {
        Jim_SetResultFormatted(interp, "no previous command: \"%#s\"", argv[1]);
        return JIM_ERR;
    }
    /* OK. Mark this command as being in an upcall */
    cmdPtr->u.proc.upcall++;
    JimIncrCmdRefCount(cmdPtr);

    /* Invoke the command as normal */
    retcode = Jim_EvalObjVector(interp, argc - 1, argv + 1);

    /* No longer in an upcall */
    cmdPtr->u.proc.upcall--;
    JimDecrCmdRefCount(interp, cmdPtr);

    return retcode;
}

/* [apply] */
static int Jim_ApplyCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int ret;
    Jim_Cmd *cmd;
    Jim_Obj *argListObjPtr;
    Jim_Obj *bodyObjPtr;
    Jim_Obj *nsObj = NULL;
    Jim_Obj **nargv;

    int len = Jim_ListLength(interp, argv[1]);
    if (len != 2 && len != 3) {
        Jim_SetResultFormatted(interp, "can't interpret \"%#s\" as a lambda expression", argv[1]);
        return JIM_ERR;
    }

    if (len == 3) {
#ifdef jim_ext_namespace
        /* Note that the namespace is always treated as global */
        nsObj = Jim_ListGetIndex(interp, argv[1], 2);
#else
        Jim_SetResultString(interp, "namespaces not enabled", -1);
        return JIM_ERR;
#endif
    }
    argListObjPtr = Jim_ListGetIndex(interp, argv[1], 0);
    bodyObjPtr = Jim_ListGetIndex(interp, argv[1], 1);

    cmd = JimCreateProcedureCmd(interp, argListObjPtr, NULL, bodyObjPtr, nsObj);

    if (cmd) {
        /* Create a new argv array with a dummy argv[0], for error messages */
        nargv = Jim_Alloc((argc - 2 + 1) * sizeof(*nargv));
        nargv[0] = Jim_NewStringObj(interp, "apply lambdaExpr", -1);
        Jim_IncrRefCount(nargv[0]);
        memcpy(&nargv[1], argv + 2, (argc - 2) * sizeof(*nargv));
        ret = JimCallProcedure(interp, cmd, argc - 2 + 1, nargv);
        Jim_DecrRefCount(interp, nargv[0]);
        Jim_Free(nargv);

        JimDecrCmdRefCount(interp, cmd);
        return ret;
    }
    return JIM_ERR;
}

#ifdef JIM_TAINT
static int JimTaintVariable(Jim_Interp *interp, Jim_Obj *nameObjPtr, int taint)
{
    Jim_Obj *valueObjPtr = Jim_GetVariable(interp, nameObjPtr, JIM_ERRMSG | JIM_UNSHARED);

    if (valueObjPtr == NULL) {
        return JIM_ERR;

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole source retained`, `/workspace/.proofs/native-providers/jimtcl/jim-interp.c`, function `interp_cmd_alias`, lines 74–174. Retained reference `rust-tcl-registry-tests-data-native_child_alias_inventory298-jim-interp.c`.

```text

    JimInterpCopyResult(interp, parent);
    return ret;
}

static int interp_cmd_alias(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_Interp *child = Jim_CmdPrivData(interp);
    Jim_Obj *aliasPrefixList;

    /* The prefix list will be held inside the child, but it still belongs
     * to the parent!
     */

    aliasPrefixList = Jim_NewListObj(interp, argv + 1, argc - 1);
    Jim_IncrRefCount(aliasPrefixList);

    Jim_RegisterCmd(child, Jim_String(argv[0]), NULL, 0, -1, JimInterpAliasProc, JimInterpDelAlias, aliasPrefixList, 0);
    return JIM_OK;
}

static const jim_subcmd_type interp_command_table[] = {
    {   "eval",
        "script ...",
        interp_cmd_eval,
        1,
        -1,
        /* Description: Concat the args and evaluate the script in the interpreter */
    },
    {   "delete",
        NULL,
        interp_cmd_delete,
        0,
        0,
        JIM_MODFLAG_FULLARGV,
        /* Description: Delete this interpreter */
    },
    {   "alias",
        "childcmd parentcmd ?arg ...?",
        interp_cmd_alias,
        2,
        -1,
        /* Description: Create an alias which refers to a script in the parent interpreter */
    },
    { NULL }
};

static int JimInterpSubCmdProc(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    return Jim_CallSubCmd(interp, Jim_ParseSubCmd(interp, interp_command_table, argc, argv), argc, argv);
}

static void JimInterpCopyVariable(Jim_Interp *target, Jim_Interp *source, const char *var, const char *default_value)
{
    Jim_Obj *value = Jim_GetGlobalVariableStr(source, var, JIM_NONE);
    const char *str;

    str = value ? Jim_String(value) : default_value;
    if (str) {
        Jim_SetGlobalVariableStr(target, var, Jim_NewStringObj(target, str, -1));
    }
}

/**
 * [interp] creates a new interpreter.
 */
static int JimInterpCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_Interp *child;
    char buf[34];
    int i;
    static const char * const copyvars[] = {
        "argv", "argc", "argv0", "jim::argv0", "jim::exe", "jim::lineedit", NULL
    };

    /* Create the interpreter command */
    child = Jim_CreateInterp();
    Jim_RegisterCoreCommands(child);
    Jim_InitStaticExtensions(child);

    /* Copy some core variables to the new interpreter */
    for (i = 0; copyvars[i]; i++) {
        JimInterpCopyVariable(child, interp, copyvars[i], NULL);
    }

    /* Allow the child interpreter to find the parent */
    Jim_SetAssocData(child, "interp.parent", NULL, interp);

    snprintf(buf, sizeof(buf), "interp.handle%ld", Jim_GetId(interp));
    Jim_RegisterCmd(interp, buf, "subcommand ?arg ...?", 1, -1, JimInterpSubCmdProc, JimInterpDelProc, child, 0);
    Jim_SetResult(interp, Jim_MakeGlobalNamespaceName(interp, Jim_NewStringObj(interp, buf, -1)));
    return JIM_OK;
}

int Jim_interpInit(Jim_Interp *interp)
{
    Jim_PackageProvideCheck(interp, "interp");
    Jim_RegisterSimpleCmd(interp, "interp", "", 0, 0, JimInterpCommand);

    return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole source retained`, `/workspace/.proofs/native-providers/jimtcl/jim-interp.c`, function `JimInterpAliasProc`, lines 50–174. Retained reference `rust-tcl-registry-tests-data-native_child_alias_inventory298-jim-interp.c`.

```text
{
    Jim_Interp *parent = Jim_GetAssocData(interp, "interp.parent");
    Jim_DecrRefCount(parent, (Jim_Obj *)privData);
}

static int JimInterpAliasProc(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int i, ret;
    Jim_Interp *parent = Jim_GetAssocData(interp, "interp.parent");
    Jim_Obj *targetPrefixObj = Jim_CmdPrivData(interp);
    Jim_Obj *targetScriptObj;

    assert(parent);

    /* Build the complete command */
    targetScriptObj = Jim_DuplicateObj(parent, targetPrefixObj);
    for (i = 1; i < argc; i++) {
        Jim_ListAppendElement(parent, targetScriptObj,
            JimInterpCopyObj(parent, argv[i]));
    }

    Jim_IncrRefCount(targetScriptObj);
    ret = Jim_EvalObj(parent, targetScriptObj);
    Jim_DecrRefCount(parent, targetScriptObj);

    JimInterpCopyResult(interp, parent);
    return ret;
}

static int interp_cmd_alias(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_Interp *child = Jim_CmdPrivData(interp);
    Jim_Obj *aliasPrefixList;

    /* The prefix list will be held inside the child, but it still belongs
     * to the parent!
     */

    aliasPrefixList = Jim_NewListObj(interp, argv + 1, argc - 1);
    Jim_IncrRefCount(aliasPrefixList);

    Jim_RegisterCmd(child, Jim_String(argv[0]), NULL, 0, -1, JimInterpAliasProc, JimInterpDelAlias, aliasPrefixList, 0);
    return JIM_OK;
}

static const jim_subcmd_type interp_command_table[] = {
    {   "eval",
        "script ...",
        interp_cmd_eval,
        1,
        -1,
        /* Description: Concat the args and evaluate the script in the interpreter */
    },
    {   "delete",
        NULL,
        interp_cmd_delete,
        0,
        0,
        JIM_MODFLAG_FULLARGV,
        /* Description: Delete this interpreter */
    },
    {   "alias",
        "childcmd parentcmd ?arg ...?",
        interp_cmd_alias,
        2,
        -1,
        /* Description: Create an alias which refers to a script in the parent interpreter */
    },
    { NULL }
};

static int JimInterpSubCmdProc(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    return Jim_CallSubCmd(interp, Jim_ParseSubCmd(interp, interp_command_table, argc, argv), argc, argv);
}

static void JimInterpCopyVariable(Jim_Interp *target, Jim_Interp *source, const char *var, const char *default_value)
{
    Jim_Obj *value = Jim_GetGlobalVariableStr(source, var, JIM_NONE);
    const char *str;

    str = value ? Jim_String(value) : default_value;
    if (str) {
        Jim_SetGlobalVariableStr(target, var, Jim_NewStringObj(target, str, -1));
    }
}

/**
 * [interp] creates a new interpreter.
 */
static int JimInterpCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_Interp *child;
    char buf[34];
    int i;
    static const char * const copyvars[] = {
        "argv", "argc", "argv0", "jim::argv0", "jim::exe", "jim::lineedit", NULL
    };

    /* Create the interpreter command */
    child = Jim_CreateInterp();
    Jim_RegisterCoreCommands(child);
    Jim_InitStaticExtensions(child);

    /* Copy some core variables to the new interpreter */
    for (i = 0; copyvars[i]; i++) {
        JimInterpCopyVariable(child, interp, copyvars[i], NULL);
    }

    /* Allow the child interpreter to find the parent */
    Jim_SetAssocData(child, "interp.parent", NULL, interp);

    snprintf(buf, sizeof(buf), "interp.handle%ld", Jim_GetId(interp));
    Jim_RegisterCmd(interp, buf, "subcommand ?arg ...?", 1, -1, JimInterpSubCmdProc, JimInterpDelProc, child, 0);
    Jim_SetResult(interp, Jim_MakeGlobalNamespaceName(interp, Jim_NewStringObj(interp, buf, -1)));
    return JIM_OK;
}

int Jim_interpInit(Jim_Interp *interp)
{
    Jim_PackageProvideCheck(interp, "interp");
    Jim_RegisterSimpleCmd(interp, "interp", "", 0, 0, JimInterpCommand);

    return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole source retained`, `/workspace/.proofs/native-providers/jimtcl/jim.h`, function `#define Jim_RegisterCmd`, lines 931–1095. Retained reference `rust-tcl-registry-tests-data-native_jim_core_alias_prefix273-jim.h`.

```text
 * don't need delProc or privData
 */
#define Jim_RegisterSimpleCmd(interp, name, usage, minargs, maxargs, cmdproc) \
        Jim_RegisterCommand(interp, Jim_NewStringObj(interp, name, -1), cmdproc, NULL, usage, NULL, minargs, maxargs, 0, NULL)
/* And also slightly more complex where delProc, privData and flags may be needed */
#define Jim_RegisterCmd(interp, name, usage, minargs, maxargs, cmdproc, delproc, privdata, flags) \
        Jim_RegisterCommand(interp, Jim_NewStringObj(interp, name, -1), cmdproc, delproc, usage, NULL, minargs, maxargs, flags, privdata)

/** Register the built-in core commands in an interpreter. */
JIM_EXPORT void Jim_RegisterCoreCommands (Jim_Interp *interp);
/** Delete a command by name object. */
JIM_EXPORT int Jim_DeleteCommand (Jim_Interp *interp,
        Jim_Obj *cmdNameObj);
/** Rename an existing command. */
JIM_EXPORT int Jim_RenameCommand (Jim_Interp *interp,
        Jim_Obj *oldNameObj, Jim_Obj *newNameObj);
/** Look up a command by name object. Accepts JIM_ERRMSG. */
JIM_EXPORT Jim_Cmd * Jim_GetCommand (Jim_Interp *interp,
        Jim_Obj *objPtr, int flags);
/** Set a variable using object names and values. */
/* Note that if Jim_SetVariable() fails, and valObjPtr has a zero reference count, it will be freed */
JIM_EXPORT int Jim_SetVariable (Jim_Interp *interp,
        Jim_Obj *nameObjPtr, Jim_Obj *valObjPtr);
/** Set a variable from a C string name. */
JIM_EXPORT int Jim_SetVariableStr (Jim_Interp *interp,
        const char *name, Jim_Obj *objPtr);
/** Set a global variable from a C string name. */
JIM_EXPORT int Jim_SetGlobalVariableStr (Jim_Interp *interp,
        const char *name, Jim_Obj *objPtr);
/** Set a variable from C string name and value inputs. */
JIM_EXPORT int Jim_SetVariableStrWithStr (Jim_Interp *interp,
        const char *name, const char *val);
/** Link a variable name to another variable in a target call frame. */
JIM_EXPORT int Jim_SetVariableLink (Jim_Interp *interp,
        Jim_Obj *nameObjPtr, Jim_Obj *targetNameObjPtr,
        Jim_CallFrame *targetCallFrame);
/** Build the fully qualified global namespace name for an object name. */
JIM_EXPORT Jim_Obj * Jim_MakeGlobalNamespaceName(Jim_Interp *interp,
        Jim_Obj *nameObjPtr);
/**
 * Get a variable by object name.
 *
 * Accepts JIM_ERRMSG and JIM_UNSHARED. JIM_UNSHARED only affects dict-sugar
 * access such as foo(bar): if the backing dictionary is shared, it is
 * duplicated before the element is returned so callers can safely update it.
 */
JIM_EXPORT Jim_Obj * Jim_GetVariable (Jim_Interp *interp,
        Jim_Obj *nameObjPtr, int flags);
/** Get a global variable by object name. Accepts the same flags as Jim_GetVariable(). */
JIM_EXPORT Jim_Obj * Jim_GetGlobalVariable (Jim_Interp *interp,
        Jim_Obj *nameObjPtr, int flags);
/** Get a variable by C string name. Accepts the same flags as Jim_GetVariable(). */
JIM_EXPORT Jim_Obj * Jim_GetVariableStr (Jim_Interp *interp,
        const char *name, int flags);
/** Get a global variable by C string name. Accepts the same flags as Jim_GetVariable(). */
JIM_EXPORT Jim_Obj * Jim_GetGlobalVariableStr (Jim_Interp *interp,
        const char *name, int flags);
/** Unset a variable by object name. Accepts JIM_ERRMSG. */
JIM_EXPORT int Jim_UnsetVariable (Jim_Interp *interp,
        Jim_Obj *nameObjPtr, int flags);

/* call frame */
/** Resolve a Tcl-style stack level to the corresponding call frame. */
JIM_EXPORT Jim_CallFrame *Jim_GetCallFrameByLevel(Jim_Interp *interp,
        Jim_Obj *levelObjPtr);

/* garbage collection */
/** Run a garbage collection pass. */
JIM_EXPORT int Jim_Collect (Jim_Interp *interp);
/** Run garbage collection if the interpreter decides it is needed. */
JIM_EXPORT void Jim_CollectIfNeeded (Jim_Interp *interp);

/* index object */
/** Extract an integer index from an object. */
JIM_EXPORT int Jim_GetIndex (Jim_Interp *interp, Jim_Obj *objPtr,
        int *indexPtr);

/* list object */
/** Create a list object from an element vector. */
JIM_EXPORT Jim_Obj * Jim_NewListObj (Jim_Interp *interp,
        Jim_Obj *const *elements, int len);
/** Insert elements into a list object at the specified index. */
JIM_EXPORT void Jim_ListInsertElements (Jim_Interp *interp,
        Jim_Obj *listPtr, int listindex, int objc, Jim_Obj *const *objVec);
/** Append one element to a list object. */
JIM_EXPORT void Jim_ListAppendElement (Jim_Interp *interp,
        Jim_Obj *listPtr, Jim_Obj *objPtr);
/** Append all elements of one list object to another. */
JIM_EXPORT void Jim_ListAppendList (Jim_Interp *interp,
        Jim_Obj *listPtr, Jim_Obj *appendListPtr);
/** Return the element count of a list object. */
JIM_EXPORT int Jim_ListLength (Jim_Interp *interp, Jim_Obj *objPtr);
/** Return a list element by index, optionally reporting conversion errors. */
JIM_EXPORT int Jim_ListIndex (Jim_Interp *interp, Jim_Obj *listPrt,
        int listindex, Jim_Obj **objPtrPtr, int seterr);
/** Return a list element by index as an object pointer. */
JIM_EXPORT Jim_Obj *Jim_ListGetIndex(Jim_Interp *interp, Jim_Obj *listPtr, int idx);
/** Set a list element reachable through a variable and index vector. */
JIM_EXPORT int Jim_SetListIndex (Jim_Interp *interp,
        Jim_Obj *varNamePtr, Jim_Obj *const *indexv, int indexc,
        Jim_Obj *newObjPtr);
/** Concatenate multiple list-like objects using Tcl list concatenation rules. */
JIM_EXPORT Jim_Obj * Jim_ConcatObj (Jim_Interp *interp, int objc,
        Jim_Obj *const *objv);
/** Join the elements of a list into a string with the supplied separator. */
JIM_EXPORT Jim_Obj *Jim_ListJoin(Jim_Interp *interp,
        Jim_Obj *listObjPtr, const char *joinStr, int joinStrLen);

/* dict object */
/** Create a dictionary object from alternating key/value elements. */
JIM_EXPORT Jim_Obj * Jim_NewDictObj (Jim_Interp *interp,
        Jim_Obj *const *elements, int len);
/** Look up one key in a dictionary object. Accepts JIM_ERRMSG. */
JIM_EXPORT int Jim_DictKey (Jim_Interp *interp, Jim_Obj *dictPtr,
        Jim_Obj *keyPtr, Jim_Obj **objPtrPtr, int flags);
/** Follow a vector of keys through nested dictionaries. Accepts JIM_ERRMSG. */
JIM_EXPORT int Jim_DictKeysVector (Jim_Interp *interp,
        Jim_Obj *dictPtr, Jim_Obj *const *keyv, int keyc,
        Jim_Obj **objPtrPtr, int flags);
/**
 * Set a nested dictionary value addressed by a key vector inside a variable.
 *
 * Accepts JIM_ERRMSG, JIM_UNSHARED, JIM_MUSTEXIST, and JIM_NORESULT.
 * JIM_UNSHARED duplicates a shared container before modifying it.
 * JIM_MUSTEXIST makes missing variables or keys an error during unset-style updates.
 * JIM_NORESULT suppresses storing the updated dictionary in the interpreter result.
 */
JIM_EXPORT int Jim_SetDictKeysVector (Jim_Interp *interp,
        Jim_Obj *varNamePtr, Jim_Obj *const *keyv, int keyc,
        Jim_Obj *newObjPtr, int flags);
/** Return the alternating key/value object array used by a dictionary. */
JIM_EXPORT Jim_Obj **Jim_DictPairs(Jim_Interp *interp,
        Jim_Obj *dictPtr, int *len);
/** Add or replace one key/value pair in a dictionary object. */
JIM_EXPORT int Jim_DictAddElement(Jim_Interp *interp, Jim_Obj *objPtr,
        Jim_Obj *keyObjPtr, Jim_Obj *valueObjPtr);

#define JIM_DICTMATCH_KEYS 0x0001
#define JIM_DICTMATCH_VALUES 0x002

/** Match dictionary keys and/or values against a pattern and return the requested components. */
JIM_EXPORT int Jim_DictMatchTypes(Jim_Interp *interp, Jim_Obj *objPtr, Jim_Obj *patternObj, int match_type, int return_types);
/** Return the number of key/value pairs in a dictionary object. */
JIM_EXPORT int Jim_DictSize(Jim_Interp *interp, Jim_Obj *objPtr);
/** Return implementation-specific diagnostic information about a dictionary object. */
JIM_EXPORT int Jim_DictInfo(Jim_Interp *interp, Jim_Obj *objPtr);
/** Merge multiple dictionaries and return the merged dictionary object. */
JIM_EXPORT Jim_Obj *Jim_DictMerge(Jim_Interp *interp, int objc, Jim_Obj *const *objv);

/* return code object */
/** Extract a return code integer from an object. */
JIM_EXPORT int Jim_GetReturnCode (Jim_Interp *interp, Jim_Obj *objPtr,
        int *intPtr);

/* expression object */
/** Evaluate an expression object and leave the result in the interpreter. */
JIM_EXPORT int Jim_EvalExpression (Jim_Interp *interp,
        Jim_Obj *exprObjPtr);
/** Evaluate an expression object and convert the result to boolean. */
JIM_EXPORT int Jim_GetBoolFromExpr (Jim_Interp *interp,
        Jim_Obj *exprObjPtr, int *boolPtr);

/* boolean object */
/** Convert an object to boolean. */
JIM_EXPORT int Jim_GetBoolean(Jim_Interp *interp, Jim_Obj *objPtr,

```

## Consumer bindings

- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `naming::native::original_command_key_tests::original_jim_command_key_constructors_keep_independent_capture_purposes` (linked): Finite source-defined control: original jim command key constructors keep independent capture purposes. Constructor selection is a pure recipe, not object ownership or a grant to install a command, evaluate a prefix or recover a context. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `cmd_alias::original_alias_key_tests::original_alias_replacement_keeps_measured_name_key_and_query_roles` (linked): Finite source-defined control: original alias replacement keeps measured name key and query roles. Only observed name refs and query header/member roles are compared; no Jim body-only updater refcount or general frame claim is inferred. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `cmd_alias::original_alias_key_tests::original_alias_rename_keeps_measured_record_and_destination_key_roles` (linked): Finite source-defined control: original alias rename keeps measured record and destination key roles. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `cmd_alias::original_alias_key_tests::original_jim_key_retirement_runs_before_value_cleanup_without_adopting_replacement` (linked): Finite source-defined control: original jim key retirement runs before value cleanup without adopting replacement. Real destructor callbacks are software controls of cleanup/reentry. Native421b did not execute these Host/context callback modes. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `cmd_alias::original_alias_key_tests::original_jim_replacement_refusal_keeps_occupied_key_and_reached_name_result` (linked): Finite source-defined control: original jim replacement refusal keeps occupied key and reached name result. These real value-destructor Host/context modes are software controls; the original421b probe did not execute these refusal callbacks. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `cmd_alias::original_alias_key_tests::original_jim_rename_qualification_and_table_key_keep_distinct_native_references` (linked): Finite source-defined control: original jim rename qualification and table key keep distinct native references. The held temporary stage checks the pinned QualifyName/keyDup source roles. Native421b's measured comparison is after both temporaries have actually retired, rather than a claim of an observed callback. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/interp/native_command_names.rs](../../../../runtime/rust/src/interp/native_command_names.rs), `interp::native_command_names::original_key_construction_tests::original_jim_qualification_keeps_namespace_owner_and_stops_after_refused_getter` (linked): Finite source-defined control: original jim qualification keeps namespace owner and stops after refused getter. Qualified source/first-cause controls; Native421b observes root keys. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/interp/native_command_names.rs](../../../../runtime/rust/src/interp/native_command_names.rs), `interp::native_command_names::original_key_construction_tests::original_jim_key_append_keeps_the_actual_zero_reference_receiver` (linked): Finite source-defined control: original jim key append keeps the actual zero reference receiver. Retire the real constructor receiver directly; a lease is not a protective native reference or a manufactured key/lookup receipt. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/interp/native_command_names.rs](../../../../runtime/rust/src/interp/native_command_names.rs), `interp::native_command_names::original_key_construction_tests::original_jim_missing_rename_source_stops_before_destination_lookup` (linked): Finite source-defined control: original jim missing rename source stops before destination lookup. The destination length getter was reached once before qualification. A missing exact source cannot publish or borrow a fallback binding. The source hash lookup is exact in the current Jim namespace. A. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [rust/tcl-vm/src/interp/original_name_residual_tests.rs](../../../../rust/tcl-vm/src/interp/original_name_residual_tests.rs), `interp::original_name_residual_tests::original_c_alias_query_keeps_target_global_late_binding_and_loop_cleanup` (linked): C alias queries use the real target GLOBAL context, late binding and loop cleanup without issuing command execution or prefix authority. This is an inspected Rust software control; it supplies no passed assertion, external Native callback chronology, role parity or new getter/worker authority.
- [rust/tcl-vm/src/interp/original_name_residual_tests.rs](../../../../rust/tcl-vm/src/interp/original_name_residual_tests.rs), `interp::original_name_residual_tests::original_jim_alias_creation_reaches_no_c_loop_string_worker` (linked): Jim alias creation reaches no C loop String worker; this finite control exercises no actual cyclic invocation. This is an inspected Rust software control; it supplies no passed assertion, external Native callback chronology, role parity or new getter/worker authority.
- [rust/tcl-vm/src/interp/original_name_residual_tests.rs](../../../../rust/tcl-vm/src/interp/original_name_residual_tests.rs), `interp::original_name_residual_tests::original_alias_retained_payload_cannot_revive_a_deleted_target_interpreter` (linked): A retained real alias payload cannot revive a deleted target interpreter; the same original host boundary retains the dead-target refusal. This is an inspected Rust software control; it supplies no passed assertion, external Native callback chronology, role parity or new getter/worker authority.

## Evidence

- [runtime/rust/tests/data/native_alias_command_key_replacement_child_capture_rename_original](../../../../runtime/rust/tests/data/native_alias_command_key_replacement_child_capture_rename_original)
- [runtime/rust/tests/data/native_alias_command_key_replacement_child_capture_rename_original/421b](../../../../runtime/rust/tests/data/native_alias_command_key_replacement_child_capture_rename_original/421b)
- [runtime/rust/tests/data/native_alias_command_key_replacement_child_capture_rename_original/421b/source-windows](../../../../runtime/rust/tests/data/native_alias_command_key_replacement_child_capture_rename_original/421b/source-windows)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/499c/providers](../../../../rust/tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/499c/providers)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs)
- [rust/tcl-registry/tests/data/native_child_bootstrap_context303/source](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303/source)
- [rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/jim](../../../../rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/jim)
- [rust/tcl-test-support/tests/data/native_resolution_reference_builds](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds)
- [runtime/rust/tests/data/native_alias_object_prefix_capture_original/415](../../../../runtime/rust/tests/data/native_alias_object_prefix_capture_original/415)
- [runtime/rust/tests/data/native_alias_command_key_replacement_child_capture_rename_original/421b/physical-source-windows](../../../../runtime/rust/tests/data/native_alias_command_key_replacement_child_capture_rename_original/421b/physical-source-windows)
- [rust/tcl-vm/src/interp](../../../../rust/tcl-vm/src/interp)
