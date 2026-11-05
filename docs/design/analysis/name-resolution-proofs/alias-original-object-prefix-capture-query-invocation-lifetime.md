# naming.alias.original-object-prefix-capture-query-invocation-lifetime

Kind: `native-observation`

## Problem statement

Which original Tcl_Obj headers are retained during alias capture, queried without presentation, invoked through selected actual getters, and released at deletion? C fresh query List headers and original members are measured independently from the current Jim core alias List owner.

## Question

For original C interp-alias and Jim core-alias APIs, which supplied lazy target/prefix objects are captured without String getters, whether two raw queries return a fresh or shared List header containing the original members, which selected invocation getters run, and which retained owners keep unread members live through alias deletion?

## Conclusion

All five C releases preserve unread original prefix members at alias capture and return two distinct unstringified raw query List headers containing those same members. C84/C85 pretest the target String before alias name; C86/C90/C91 reach alias name then target. Jim captures target/prefix without String and returns its same stored List header, so query List String cache sharing differs from C. The supplied callback receives original members, selects prefix2 then prefix1 and retains counted opaque NUL bytes; the second invocation reuses each reached cache. Retained raw query owners keep members live through caller release and alias deletion; their final release retires selected unread internal representations without a getter. Jim C-style child/query axes are unavailable and separate from measured Jim core alias. The exact header/member identities, cache/refcount/release stages and finite public results supply no binding generation, runtime issuer, whole-alias parity or BIG-IP result.

## Scope

Six original compile/dependency/execute captures close with own successful original producers; C five streams contain93 ROW each and Jim73. Six requested fixed cases include query/presentation/invocation/lifetime and a separately unavailable Jim C-style child-query case. Original counted prefix bytes are produced data, not source NUL or generic numeric/name acceptance. Headers/pointers only compare supplied live objects within their process; internalRep free callbacks do not prove header deallocation. Original requests/probe/launcher/whole source windows/link maps/header/config/library/control/runtime inputs and unchanged executable receipts are retained. Source/library inputs are retained; configure/build processes lack replay receipts. Exact system compiler/header/link/runtime payload hashes carry explicit independently-supplied replay limits. Native source interpretation, public captured rows and derived joins remain separate; no Rust assertion/Host refusal/frame/Native handler or generation authority is supplied.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Original stringless extension object headers from fixed ASCII C source and counted produced prefix bytes; separate capture, raw List query, explicit presentation, callback invocation and release stages. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

The fixed original C interp-alias capture preserves both unread prefix objects. C84/C85 first request target role1 String in the public pretest, then alias-name role0. Each of two concurrent raw alias queries creates a distinct unstringified List header containing the same exact original target and prefix members. Presenting one query materialises those shared members but leaves the other List String cache absent. The original invocation callback receives the original prefix objects and reads prefix2 then prefix1; its counted NUL payload 616c70686120002078 remains intact and the second invocation reuses each reached cache. Held query Lists keep original members live after caller holds and alias deletion; final query release triggers each selected internalRep retirement once. With no query owner, alias deletion retires unread prefix internal representations without asking for String. The separate C child-to-parent case6 passes the original members to the actual target callback. These are finite supplied-header/member/cache observations, not command generation or application Native issuer evidence.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original stringless extension object headers from fixed ASCII C source and counted produced prefix bytes; separate capture, raw List query, explicit presentation, callback invocation and release stages. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

The fixed original C interp-alias capture preserves both unread prefix objects. C86/C90/C91 first request alias-name role0 String during capture/publication, then target role1 in original alias-loop lookup. Each of two concurrent raw alias queries creates a distinct unstringified List header containing the same exact original target and prefix members. Presenting one query materialises those shared members but leaves the other List String cache absent. The original invocation callback receives the original prefix objects and reads prefix2 then prefix1; its counted NUL payload 616c70686120002078 remains intact and the second invocation reuses each reached cache. Held query Lists keep original members live after caller holds and alias deletion; final query release triggers each selected internalRep retirement once. With no query owner, alias deletion retires unread prefix internal representations without asking for String. The separate C child-to-parent case6 passes the original members to the actual target callback. These are finite supplied-header/member/cache observations, not command generation or application Native issuer evidence.

### jim

Status: `observed`. Channel: Original stringless extension object headers from fixed ASCII C source and counted produced prefix bytes; separate capture, raw List query, explicit presentation, callback invocation and release stages. Version: jim 0.84-9-g5bac7c9.

The actual Jim core alias capture reaches only alias-name role0 String; target and both prefix objects remain unread at capture. Two raw info-alias queries return the same stored unstringified List header and exact original target/prefix members. Presenting one query String also presents the shared List header visible through the other query. Original invocation receives the retained target and prefix objects; the capture callback reads prefix2 then prefix1, with counted 616c70686120002078 bytes retained, and a second invocation reuses their one reached String cache. Held raw query owners preserve members after caller holds and alias deletion; their final release triggers each selected internalRep retirement once without requesting previously unread prefix Strings. C interp-alias query/child APIs are explicitly unavailable and are not supplied by the measured Jim core-alias API.

### bigip

Status: `not-tested`.

No original BIG-IP alias object/header/query/invocation process was requested or executed.

## Source inspection

tcl8.4 8.4.20, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInterp.c`, function `AliasCreate`, lines 1148–1302. Retained reference `rust-tcl-registry-tests-data-native_info_commands_literal_original-original-inputs-8.4.20-generic-tclInterp.c`.

```text
 *	for the slave interpreter.
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

```

tcl8.4 8.4.20, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInterp.c`, function `AliasDescribe`, lines 1344–1496. Retained reference `rust-tcl-registry-tests-data-native_info_commands_literal_original-original-inputs-8.4.20-generic-tclInterp.c`.

```text
 *	None.
 *
 *----------------------------------------------------------------------
 */

static int
AliasDescribe(interp, slaveInterp, namePtr)
    Tcl_Interp *interp;		/* Interpreter for result & errors. */
    Tcl_Interp *slaveInterp;	/* Interpreter containing alias. */
    Tcl_Obj *namePtr;		/* Name of alias to describe. */
{
    Slave *slavePtr;
    Tcl_HashEntry *hPtr;
    Alias *aliasPtr;	
    Tcl_Obj *prefixPtr;

    /*
     * If the alias has been renamed in the slave, the master can still use
     * the original name (with which it was created) to find the alias to
     * describe it.
     */

    slavePtr = &((InterpInfo *) ((Interp *) slaveInterp)->interpInfo)->slave;
    hPtr = Tcl_FindHashEntry(&slavePtr->aliasTable, Tcl_GetString(namePtr));
    if (hPtr == NULL) {
        return TCL_OK;
    }
    aliasPtr = (Alias *) Tcl_GetHashValue(hPtr);
    prefixPtr = Tcl_NewListObj(aliasPtr->objc, &aliasPtr->objPtr);
    Tcl_SetObjResult(interp, prefixPtr);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * AliasList --
 *
 *	Computes a list of aliases defined in a slave interpreter.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

static int
AliasList(interp, slaveInterp)
    Tcl_Interp *interp;		/* Interp for data return. */
    Tcl_Interp *slaveInterp;	/* Interp whose aliases to compute. */
{
    Tcl_HashEntry *entryPtr;
    Tcl_HashSearch hashSearch;
    Tcl_Obj *resultPtr;	
    Alias *aliasPtr;
    Slave *slavePtr;

    slavePtr = &((InterpInfo *) ((Interp *) slaveInterp)->interpInfo)->slave;
    resultPtr = Tcl_GetObjResult(interp);

    entryPtr = Tcl_FirstHashEntry(&slavePtr->aliasTable, &hashSearch);
    for ( ; entryPtr != NULL; entryPtr = Tcl_NextHashEntry(&hashSearch)) {
        aliasPtr = (Alias *) Tcl_GetHashValue(entryPtr);
        Tcl_ListObjAppendElement(NULL, resultPtr, aliasPtr->namePtr);
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * AliasObjCmd --
 *
 *	This is the procedure that services invocations of aliases in a
 *	slave interpreter. One such command exists for each alias. When
 *	invoked, this procedure redirects the invocation to the target
 *	command in the master interpreter as designated by the Alias
 *	record associated with this command.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	Causes forwarding of the invocation; all possible side effects
 *	may occur as a result of invoking the command to which the
 *	invocation is forwarded.
 *
 *----------------------------------------------------------------------
 */

static int
AliasObjCmd(clientData, interp, objc, objv)
    ClientData clientData;	/* Alias record. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument vector. */	
{
#define ALIAS_CMDV_PREALLOC 10
    Tcl_Interp *targetInterp;	
    Alias *aliasPtr;		
    int result, prefc, cmdc, i;
    Tcl_Obj **prefv, **cmdv;
    Tcl_Obj *cmdArr[ALIAS_CMDV_PREALLOC];
    aliasPtr = (Alias *) clientData;
    targetInterp = aliasPtr->targetInterp;

    /*
     * Append the arguments to the command prefix and invoke the command
     * in the target interp's global namespace.
     */
     
    prefc = aliasPtr->objc;
    prefv = &aliasPtr->objPtr;
    cmdc = prefc + objc - 1;
    if (cmdc <= ALIAS_CMDV_PREALLOC) {
	cmdv = cmdArr;
    } else {
	cmdv = (Tcl_Obj **) ckalloc((unsigned) (cmdc * sizeof(Tcl_Obj *)));
    }

    prefv = &aliasPtr->objPtr;
    memcpy((VOID *) cmdv, (VOID *) prefv, 
            (size_t) (prefc * sizeof(Tcl_Obj *)));
    memcpy((VOID *) (cmdv+prefc), (VOID *) (objv+1), 
	    (size_t) ((objc-1) * sizeof(Tcl_Obj *)));

    Tcl_ResetResult(targetInterp);

    for (i=0; i<cmdc; i++) {
	Tcl_IncrRefCount(cmdv[i]);
    }
    if (targetInterp != interp) {
	Tcl_Preserve((ClientData) targetInterp);
	result = Tcl_EvalObjv(targetInterp, cmdc, cmdv, TCL_EVAL_INVOKE);
	TclTransferResult(targetInterp, result, interp);	
	Tcl_Release((ClientData) targetInterp);
    } else {
	result = Tcl_EvalObjv(targetInterp, cmdc, cmdv, TCL_EVAL_INVOKE);
    }
    for (i=0; i<cmdc; i++) {
	Tcl_DecrRefCount(cmdv[i]);
    }

    if (cmdv != cmdArr) {
	ckfree((char *) cmdv);
    }
    return result;        
#undef ALIAS_CMDV_PREALLOC
}


```

tcl8.4 8.4.20, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInterp.c`, function `AliasObjCmd`, lines 1432–1584. Retained reference `rust-tcl-registry-tests-data-native_info_commands_literal_original-original-inputs-8.4.20-generic-tclInterp.c`.

```text
 *	invocation is forwarded.
 *
 *----------------------------------------------------------------------
 */

static int
AliasObjCmd(clientData, interp, objc, objv)
    ClientData clientData;	/* Alias record. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument vector. */	
{
#define ALIAS_CMDV_PREALLOC 10
    Tcl_Interp *targetInterp;	
    Alias *aliasPtr;		
    int result, prefc, cmdc, i;
    Tcl_Obj **prefv, **cmdv;
    Tcl_Obj *cmdArr[ALIAS_CMDV_PREALLOC];
    aliasPtr = (Alias *) clientData;
    targetInterp = aliasPtr->targetInterp;

    /*
     * Append the arguments to the command prefix and invoke the command
     * in the target interp's global namespace.
     */
     
    prefc = aliasPtr->objc;
    prefv = &aliasPtr->objPtr;
    cmdc = prefc + objc - 1;
    if (cmdc <= ALIAS_CMDV_PREALLOC) {
	cmdv = cmdArr;
    } else {
	cmdv = (Tcl_Obj **) ckalloc((unsigned) (cmdc * sizeof(Tcl_Obj *)));
    }

    prefv = &aliasPtr->objPtr;
    memcpy((VOID *) cmdv, (VOID *) prefv, 
            (size_t) (prefc * sizeof(Tcl_Obj *)));
    memcpy((VOID *) (cmdv+prefc), (VOID *) (objv+1), 
	    (size_t) ((objc-1) * sizeof(Tcl_Obj *)));

    Tcl_ResetResult(targetInterp);

    for (i=0; i<cmdc; i++) {
	Tcl_IncrRefCount(cmdv[i]);
    }
    if (targetInterp != interp) {
	Tcl_Preserve((ClientData) targetInterp);
	result = Tcl_EvalObjv(targetInterp, cmdc, cmdv, TCL_EVAL_INVOKE);
	TclTransferResult(targetInterp, result, interp);	
	Tcl_Release((ClientData) targetInterp);
    } else {
	result = Tcl_EvalObjv(targetInterp, cmdc, cmdv, TCL_EVAL_INVOKE);
    }
    for (i=0; i<cmdc; i++) {
	Tcl_DecrRefCount(cmdv[i]);
    }

    if (cmdv != cmdArr) {
	ckfree((char *) cmdv);
    }
    return result;        
#undef ALIAS_CMDV_PREALLOC
}

/*
 *----------------------------------------------------------------------
 *
 * AliasObjCmdDeleteProc --
 *
 *	Is invoked when an alias command is deleted in a slave. Cleans up
 *	all storage associated with this alias.
 *
 * Results:
 *	None.
 *
 * Side effects:
 *	Deletes the alias record and its entry in the alias table for
 *	the interpreter.
 *
 *----------------------------------------------------------------------
 */

static void
AliasObjCmdDeleteProc(clientData)
    ClientData clientData;	/* The alias record for this alias. */
{
    Alias *aliasPtr;		
    Target *targetPtr;		
    int i;
    Tcl_Obj **objv;

    aliasPtr = (Alias *) clientData;
    
    Tcl_DecrRefCount(aliasPtr->namePtr);
    objv = &aliasPtr->objPtr;
    for (i = 0; i < aliasPtr->objc; i++) {
	Tcl_DecrRefCount(objv[i]);
    }
    Tcl_DeleteHashEntry(aliasPtr->aliasEntryPtr);

    targetPtr = (Target *) Tcl_GetHashValue(aliasPtr->targetEntryPtr);
    ckfree((char *) targetPtr);
    Tcl_DeleteHashEntry(aliasPtr->targetEntryPtr);

    ckfree((char *) aliasPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_CreateSlave --
 *
 *	Creates a slave interpreter. The slavePath argument denotes the
 *	name of the new slave relative to the current interpreter; the
 *	slave is a direct descendant of the one-before-last component of
 *	the path, e.g. it is a descendant of the current interpreter if
 *	the slavePath argument contains only one component. Optionally makes
 *	the slave interpreter safe.
 *
 * Results:
 *	Returns the interpreter structure created, or NULL if an error
 *	occurred.
 *
 * Side effects:
 *	Creates a new interpreter and a new interpreter object command in
 *	the interpreter indicated by the slavePath argument.
 *
 *----------------------------------------------------------------------
 */

Tcl_Interp *
Tcl_CreateSlave(interp, slavePath, isSafe)
    Tcl_Interp *interp;		/* Interpreter to start search at. */
    CONST char *slavePath;	/* Name of slave to create. */
    int isSafe;			/* Should new slave be "safe" ? */
{
    Tcl_Obj *pathPtr;
    Tcl_Interp *slaveInterp;

    pathPtr = Tcl_NewStringObj(slavePath, -1);
    slaveInterp = SlaveCreate(interp, pathPtr, isSafe);
    Tcl_DecrRefCount(pathPtr);

    return slaveInterp;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetSlave --
 *
 *	Finds a slave interpreter by its path name.

```

tcl8.4 8.4.20, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInterp.c`, function `AliasObjCmdDeleteProc`, lines 1510–1661. Retained reference `rust-tcl-registry-tests-data-native_info_commands_literal_original-original-inputs-8.4.20-generic-tclInterp.c`.

```text
 *	the interpreter.
 *
 *----------------------------------------------------------------------
 */

static void
AliasObjCmdDeleteProc(clientData)
    ClientData clientData;	/* The alias record for this alias. */
{
    Alias *aliasPtr;		
    Target *targetPtr;		
    int i;
    Tcl_Obj **objv;

    aliasPtr = (Alias *) clientData;
    
    Tcl_DecrRefCount(aliasPtr->namePtr);
    objv = &aliasPtr->objPtr;
    for (i = 0; i < aliasPtr->objc; i++) {
	Tcl_DecrRefCount(objv[i]);
    }
    Tcl_DeleteHashEntry(aliasPtr->aliasEntryPtr);

    targetPtr = (Target *) Tcl_GetHashValue(aliasPtr->targetEntryPtr);
    ckfree((char *) targetPtr);
    Tcl_DeleteHashEntry(aliasPtr->targetEntryPtr);

    ckfree((char *) aliasPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_CreateSlave --
 *
 *	Creates a slave interpreter. The slavePath argument denotes the
 *	name of the new slave relative to the current interpreter; the
 *	slave is a direct descendant of the one-before-last component of
 *	the path, e.g. it is a descendant of the current interpreter if
 *	the slavePath argument contains only one component. Optionally makes
 *	the slave interpreter safe.
 *
 * Results:
 *	Returns the interpreter structure created, or NULL if an error
 *	occurred.
 *
 * Side effects:
 *	Creates a new interpreter and a new interpreter object command in
 *	the interpreter indicated by the slavePath argument.
 *
 *----------------------------------------------------------------------
 */

Tcl_Interp *
Tcl_CreateSlave(interp, slavePath, isSafe)
    Tcl_Interp *interp;		/* Interpreter to start search at. */
    CONST char *slavePath;	/* Name of slave to create. */
    int isSafe;			/* Should new slave be "safe" ? */
{
    Tcl_Obj *pathPtr;
    Tcl_Interp *slaveInterp;

    pathPtr = Tcl_NewStringObj(slavePath, -1);
    slaveInterp = SlaveCreate(interp, pathPtr, isSafe);
    Tcl_DecrRefCount(pathPtr);

    return slaveInterp;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetSlave --
 *
 *	Finds a slave interpreter by its path name.
 *
 * Results:
 *	Returns a Tcl_Interp * for the named interpreter or NULL if not
 *	found.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

Tcl_Interp *
Tcl_GetSlave(interp, slavePath)
    Tcl_Interp *interp;		/* Interpreter to start search from. */
    CONST char *slavePath;	/* Path of slave to find. */
{
    Tcl_Obj *pathPtr;
    Tcl_Interp *slaveInterp;

    pathPtr = Tcl_NewStringObj(slavePath, -1);
    slaveInterp = GetInterp(interp, pathPtr);
    Tcl_DecrRefCount(pathPtr);

    return slaveInterp;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetMaster --
 *
 *	Finds the master interpreter of a slave interpreter.
 *
 * Results:
 *	Returns a Tcl_Interp * for the master interpreter or NULL if none.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

Tcl_Interp *
Tcl_GetMaster(interp)
    Tcl_Interp *interp;		/* Get the master of this interpreter. */
{
    Slave *slavePtr;		/* Slave record of this interpreter. */

    if (interp == (Tcl_Interp *) NULL) {
        return NULL;
    }
    slavePtr = &((InterpInfo *) ((Interp *) interp)->interpInfo)->slave;
    return slavePtr->masterInterp;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetInterpPath --
 *
 *	Sets the result of the asking interpreter to a proper Tcl list
 *	containing the names of interpreters between the asking and
 *	target interpreters. The target interpreter must be either the
 *	same as the asking interpreter or one of its slaves (including
 *	recursively).
 *
 * Results:
 *	TCL_OK if the target interpreter is the same as, or a descendant
 *	of, the asking interpreter; TCL_ERROR else. This way one can
 *	distinguish between the case where the asking and target interps
 *	are the same (an empty list is the result, and TCL_OK is returned)
 *	and when the target is not a descendant of the asking interpreter
 *	(in which case the Tcl result is an error message and the function
 *	returns TCL_ERROR).
 *
 * Side effects:
 *	None.

```

tcl8.4 8.4.20, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInterp.c`, function `TclPreventAliasLoop`, lines 1049–1203. Retained reference `rust-tcl-registry-tests-data-native_info_commands_literal_original-original-inputs-8.4.20-generic-tclInterp.c`.

```text
 *	this file) because it is also used from TclRenameCommand.
 *
 *----------------------------------------------------------------------
 */

int
TclPreventAliasLoop(interp, cmdInterp, cmd)
    Tcl_Interp *interp;			/* Interp in which to report errors. */
    Tcl_Interp *cmdInterp;		/* Interp in which the command is
                                         * being defined. */
    Tcl_Command cmd;                    /* Tcl command we are attempting
                                         * to define. */
{
    Command *cmdPtr = (Command *) cmd;
    Alias *aliasPtr, *nextAliasPtr;
    Tcl_Command aliasCmd;
    Command *aliasCmdPtr;

    /*
     * If we are not creating or renaming an alias, then it is
     * always OK to create or rename the command.
     */
    
    if (cmdPtr->objProc != AliasObjCmd) {
        return TCL_OK;
    }

    /*
     * OK, we are dealing with an alias, so traverse the chain of aliases.
     * If we encounter the alias we are defining (or renaming to) any in
     * the chain then we have a loop.
     */

    aliasPtr = (Alias *) cmdPtr->objClientData;
    nextAliasPtr = aliasPtr;
    while (1) {
	Tcl_Obj *cmdNamePtr;

        /*
         * If the target of the next alias in the chain is the same as
         * the source alias, we have a loop.
	 */

	if (Tcl_InterpDeleted(nextAliasPtr->targetInterp)) {
	    /*
	     * The slave interpreter can be deleted while creating the alias.
	     * [Bug #641195]
	     */

	    Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
		    "cannot define or rename alias \"",
		    Tcl_GetString(aliasPtr->namePtr),
		    "\": interpreter deleted", (char *) NULL);
	    return TCL_ERROR;
	}
	cmdNamePtr = nextAliasPtr->objPtr;
	aliasCmd = Tcl_FindCommand(nextAliasPtr->targetInterp,
                Tcl_GetString(cmdNamePtr),
		Tcl_GetGlobalNamespace(nextAliasPtr->targetInterp),
		/*flags*/ 0);
        if (aliasCmd == (Tcl_Command) NULL) {
            return TCL_OK;
        }
	aliasCmdPtr = (Command *) aliasCmd;
        if (aliasCmdPtr == cmdPtr) {
            Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
		    "cannot define or rename alias \"",
		    Tcl_GetString(aliasPtr->namePtr),
		    "\": would create a loop", (char *) NULL);
            return TCL_ERROR;
        }

        /*
	 * Otherwise, follow the chain one step further. See if the target
         * command is an alias - if so, follow the loop to its target
         * command. Otherwise we do not have a loop.
	 */

        if (aliasCmdPtr->objProc != AliasObjCmd) {
            return TCL_OK;
        }
        nextAliasPtr = (Alias *) aliasCmdPtr->objClientData;
    }

    /* NOTREACHED */
}

/*
 *----------------------------------------------------------------------
 *
 * AliasCreate --
 *
 *	Helper function to do the work to actually create an alias.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	An alias command is created and entered into the alias table
 *	for the slave interpreter.
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

```

tcl8.5 8.5.19, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInterp.c`, function `AliasCreate`, lines 1421–1575. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-8.5.19-tclInterp.c`.

```text
 *	slave interpreter.
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

```

tcl8.5 8.5.19, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInterp.c`, function `AliasDescribe`, lines 1628–1781. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-8.5.19-tclInterp.c`.

```text
 *	None.
 *
 *----------------------------------------------------------------------
 */

static int
AliasDescribe(
    Tcl_Interp *interp,		/* Interpreter for result & errors. */
    Tcl_Interp *slaveInterp,	/* Interpreter containing alias. */
    Tcl_Obj *namePtr)		/* Name of alias to describe. */
{
    Slave *slavePtr;
    Tcl_HashEntry *hPtr;
    Alias *aliasPtr;
    Tcl_Obj *prefixPtr;

    /*
     * If the alias has been renamed in the slave, the master can still use
     * the original name (with which it was created) to find the alias to
     * describe it.
     */

    slavePtr = &((InterpInfo *) ((Interp *) slaveInterp)->interpInfo)->slave;
    hPtr = Tcl_FindHashEntry(&slavePtr->aliasTable, Tcl_GetString(namePtr));
    if (hPtr == NULL) {
	return TCL_OK;
    }
    aliasPtr = Tcl_GetHashValue(hPtr);
    prefixPtr = Tcl_NewListObj(aliasPtr->objc, &aliasPtr->objPtr);
    Tcl_SetObjResult(interp, prefixPtr);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * AliasList --
 *
 *	Computes a list of aliases defined in a slave interpreter.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

static int
AliasList(
    Tcl_Interp *interp,		/* Interp for data return. */
    Tcl_Interp *slaveInterp)	/* Interp whose aliases to compute. */
{
    Tcl_HashEntry *entryPtr;
    Tcl_HashSearch hashSearch;
    Tcl_Obj *resultPtr = Tcl_NewObj();
    Alias *aliasPtr;
    Slave *slavePtr;

    slavePtr = &((InterpInfo *) ((Interp *) slaveInterp)->interpInfo)->slave;

    entryPtr = Tcl_FirstHashEntry(&slavePtr->aliasTable, &hashSearch);
    for ( ; entryPtr != NULL; entryPtr = Tcl_NextHashEntry(&hashSearch)) {
	aliasPtr = Tcl_GetHashValue(entryPtr);
	Tcl_ListObjAppendElement(NULL, resultPtr, aliasPtr->token);
    }
    Tcl_SetObjResult(interp, resultPtr);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * AliasObjCmd --
 *
 *	This is the function that services invocations of aliases in a slave
 *	interpreter. One such command exists for each alias. When invoked,
 *	this function redirects the invocation to the target command in the
 *	master interpreter as designated by the Alias record associated with
 *	this command.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	Causes forwarding of the invocation; all possible side effects may
 *	occur as a result of invoking the command to which the invocation is
 *	forwarded.
 *
 *----------------------------------------------------------------------
 */

static int
AliasObjCmd(
    ClientData clientData,	/* Alias record. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument vector. */
{
#define ALIAS_CMDV_PREALLOC 10
    Alias *aliasPtr = clientData;
    Tcl_Interp *targetInterp = aliasPtr->targetInterp;
    int result, prefc, cmdc, i;
    Tcl_Obj **prefv, **cmdv;
    Tcl_Obj *cmdArr[ALIAS_CMDV_PREALLOC];
    Interp *tPtr = (Interp *) targetInterp;
    int isRootEnsemble = (tPtr->ensembleRewrite.sourceObjs == NULL);

    /*
     * Append the arguments to the command prefix and invoke the command in
     * the target interp's global namespace.
     */

    prefc = aliasPtr->objc;
    prefv = &aliasPtr->objPtr;
    cmdc = prefc + objc - 1;
    if (cmdc <= ALIAS_CMDV_PREALLOC) {
	cmdv = cmdArr;
    } else {
	cmdv = (Tcl_Obj **) TclStackAlloc(interp, cmdc*(int)sizeof(Tcl_Obj*));
    }

    memcpy(cmdv, prefv, (size_t) (prefc * sizeof(Tcl_Obj *)));
    memcpy(cmdv+prefc, objv+1, (size_t) ((objc-1) * sizeof(Tcl_Obj *)));

    Tcl_ResetResult(targetInterp);

    for (i=0; i<cmdc; i++) {
	Tcl_IncrRefCount(cmdv[i]);
    }

    /*
     * Use the ensemble rewriting machinery to ensure correct error messages:
     * only the source command should show, not the full target prefix.
     */

    if (isRootEnsemble) {
	tPtr->ensembleRewrite.sourceObjs = objv;
	tPtr->ensembleRewrite.numRemovedObjs = 1;
	tPtr->ensembleRewrite.numInsertedObjs = prefc;
    } else {
	tPtr->ensembleRewrite.numInsertedObjs += prefc - 1;
    }

    /*
     * Protect the target interpreter if it isn't the same as the source
     * interpreter so that we can continue to work with it after the target
     * command completes.
     */

    if (targetInterp != interp) {
	Tcl_Preserve(targetInterp);
    }

```

tcl8.5 8.5.19, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInterp.c`, function `AliasObjCmd`, lines 1716–1870. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-8.5.19-tclInterp.c`.

```text
 *	forwarded.
 *
 *----------------------------------------------------------------------
 */

static int
AliasObjCmd(
    ClientData clientData,	/* Alias record. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument vector. */
{
#define ALIAS_CMDV_PREALLOC 10
    Alias *aliasPtr = clientData;
    Tcl_Interp *targetInterp = aliasPtr->targetInterp;
    int result, prefc, cmdc, i;
    Tcl_Obj **prefv, **cmdv;
    Tcl_Obj *cmdArr[ALIAS_CMDV_PREALLOC];
    Interp *tPtr = (Interp *) targetInterp;
    int isRootEnsemble = (tPtr->ensembleRewrite.sourceObjs == NULL);

    /*
     * Append the arguments to the command prefix and invoke the command in
     * the target interp's global namespace.
     */

    prefc = aliasPtr->objc;
    prefv = &aliasPtr->objPtr;
    cmdc = prefc + objc - 1;
    if (cmdc <= ALIAS_CMDV_PREALLOC) {
	cmdv = cmdArr;
    } else {
	cmdv = (Tcl_Obj **) TclStackAlloc(interp, cmdc*(int)sizeof(Tcl_Obj*));
    }

    memcpy(cmdv, prefv, (size_t) (prefc * sizeof(Tcl_Obj *)));
    memcpy(cmdv+prefc, objv+1, (size_t) ((objc-1) * sizeof(Tcl_Obj *)));

    Tcl_ResetResult(targetInterp);

    for (i=0; i<cmdc; i++) {
	Tcl_IncrRefCount(cmdv[i]);
    }

    /*
     * Use the ensemble rewriting machinery to ensure correct error messages:
     * only the source command should show, not the full target prefix.
     */

    if (isRootEnsemble) {
	tPtr->ensembleRewrite.sourceObjs = objv;
	tPtr->ensembleRewrite.numRemovedObjs = 1;
	tPtr->ensembleRewrite.numInsertedObjs = prefc;
    } else {
	tPtr->ensembleRewrite.numInsertedObjs += prefc - 1;
    }

    /*
     * Protect the target interpreter if it isn't the same as the source
     * interpreter so that we can continue to work with it after the target
     * command completes.
     */

    if (targetInterp != interp) {
	Tcl_Preserve(targetInterp);
    }

    /*
     * Execute the target command in the target interpreter.
     */

    result = Tcl_EvalObjv(targetInterp, cmdc, cmdv, TCL_EVAL_INVOKE);

    /*
     * Clean up the ensemble rewrite info if we set it in the first place.
     */

    if (isRootEnsemble) {
	tPtr->ensembleRewrite.sourceObjs = NULL;
	tPtr->ensembleRewrite.numRemovedObjs = 0;
	tPtr->ensembleRewrite.numInsertedObjs = 0;
    }

    /*
     * If it was a cross-interpreter alias, we need to transfer the result
     * back to the source interpreter and release the lock we previously set
     * on the target interpreter.
     */

    if (targetInterp != interp) {
	TclTransferResult(targetInterp, result, interp);
	Tcl_Release(targetInterp);
    }

    for (i=0; i<cmdc; i++) {
	Tcl_DecrRefCount(cmdv[i]);
    }
    if (cmdv != cmdArr) {
	TclStackFree(interp, cmdv);
    }
    return result;
#undef ALIAS_CMDV_PREALLOC
}

/*
 *----------------------------------------------------------------------
 *
 * AliasObjCmdDeleteProc --
 *
 *	Is invoked when an alias command is deleted in a slave. Cleans up all
 *	storage associated with this alias.
 *
 * Results:
 *	None.
 *
 * Side effects:
 *	Deletes the alias record and its entry in the alias table for the
 *	interpreter.
 *
 *----------------------------------------------------------------------
 */

static void
AliasObjCmdDeleteProc(
    ClientData clientData)	/* The alias record for this alias. */
{
    Alias *aliasPtr = clientData;
    Target *targetPtr;
    int i;
    Tcl_Obj **objv;

    Tcl_DecrRefCount(aliasPtr->token);
    objv = &aliasPtr->objPtr;
    for (i = 0; i < aliasPtr->objc; i++) {
	Tcl_DecrRefCount(objv[i]);
    }
    Tcl_DeleteHashEntry(aliasPtr->aliasEntryPtr);

    /*
     * Splice the target record out of the target interpreter's master list.
     */

    targetPtr = aliasPtr->targetPtr;
    if (targetPtr->prevPtr != NULL) {
	targetPtr->prevPtr->nextPtr = targetPtr->nextPtr;
    } else {
	Master *masterPtr = &((InterpInfo *) ((Interp *)
		aliasPtr->targetInterp)->interpInfo)->master;

	masterPtr->targetsPtr = targetPtr->nextPtr;
    }
    if (targetPtr->nextPtr != NULL) {
	targetPtr->nextPtr->prevPtr = targetPtr->prevPtr;
    }

```

tcl8.5 8.5.19, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInterp.c`, function `AliasObjCmdDeleteProc`, lines 1833–1984. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-8.5.19-tclInterp.c`.

```text
 *	interpreter.
 *
 *----------------------------------------------------------------------
 */

static void
AliasObjCmdDeleteProc(
    ClientData clientData)	/* The alias record for this alias. */
{
    Alias *aliasPtr = clientData;
    Target *targetPtr;
    int i;
    Tcl_Obj **objv;

    Tcl_DecrRefCount(aliasPtr->token);
    objv = &aliasPtr->objPtr;
    for (i = 0; i < aliasPtr->objc; i++) {
	Tcl_DecrRefCount(objv[i]);
    }
    Tcl_DeleteHashEntry(aliasPtr->aliasEntryPtr);

    /*
     * Splice the target record out of the target interpreter's master list.
     */

    targetPtr = aliasPtr->targetPtr;
    if (targetPtr->prevPtr != NULL) {
	targetPtr->prevPtr->nextPtr = targetPtr->nextPtr;
    } else {
	Master *masterPtr = &((InterpInfo *) ((Interp *)
		aliasPtr->targetInterp)->interpInfo)->master;

	masterPtr->targetsPtr = targetPtr->nextPtr;
    }
    if (targetPtr->nextPtr != NULL) {
	targetPtr->nextPtr->prevPtr = targetPtr->prevPtr;
    }

    ckfree((char *) targetPtr);
    ckfree((char *) aliasPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_CreateSlave --
 *
 *	Creates a slave interpreter. The slavePath argument denotes the name
 *	of the new slave relative to the current interpreter; the slave is a
 *	direct descendant of the one-before-last component of the path,
 *	e.g. it is a descendant of the current interpreter if the slavePath
 *	argument contains only one component. Optionally makes the slave
 *	interpreter safe.
 *
 * Results:
 *	Returns the interpreter structure created, or NULL if an error
 *	occurred.
 *
 * Side effects:
 *	Creates a new interpreter and a new interpreter object command in the
 *	interpreter indicated by the slavePath argument.
 *
 *----------------------------------------------------------------------
 */

Tcl_Interp *
Tcl_CreateSlave(
    Tcl_Interp *interp,		/* Interpreter to start search at. */
    const char *slavePath,	/* Name of slave to create. */
    int isSafe)			/* Should new slave be "safe" ? */
{
    Tcl_Obj *pathPtr;
    Tcl_Interp *slaveInterp;

    pathPtr = Tcl_NewStringObj(slavePath, -1);
    slaveInterp = SlaveCreate(interp, pathPtr, isSafe);
    Tcl_DecrRefCount(pathPtr);

    return slaveInterp;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetSlave --
 *
 *	Finds a slave interpreter by its path name.
 *
 * Results:
 *	Returns a Tcl_Interp * for the named interpreter or NULL if not found.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

Tcl_Interp *
Tcl_GetSlave(
    Tcl_Interp *interp,		/* Interpreter to start search from. */
    const char *slavePath)	/* Path of slave to find. */
{
    Tcl_Obj *pathPtr;
    Tcl_Interp *slaveInterp;

    pathPtr = Tcl_NewStringObj(slavePath, -1);
    slaveInterp = GetInterp(interp, pathPtr);
    Tcl_DecrRefCount(pathPtr);

    return slaveInterp;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetMaster --
 *
 *	Finds the master interpreter of a slave interpreter.
 *
 * Results:
 *	Returns a Tcl_Interp * for the master interpreter or NULL if none.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

Tcl_Interp *
Tcl_GetMaster(
    Tcl_Interp *interp)		/* Get the master of this interpreter. */
{
    Slave *slavePtr;		/* Slave record of this interpreter. */

    if (interp == NULL) {
	return NULL;
    }
    slavePtr = &((InterpInfo *) ((Interp *) interp)->interpInfo)->slave;
    return slavePtr->masterInterp;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetInterpPath --
 *
 *	Sets the result of the asking interpreter to a proper Tcl list
 *	containing the names of interpreters between the asking and target
 *	interpreters. The target interpreter must be either the same as the
 *	asking interpreter or one of its slaves (including recursively).
 *
 * Results:

```

tcl8.5 8.5.19, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInterp.c`, function `TclPreventAliasLoop`, lines 1325–1479. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-8.5.19-tclInterp.c`.

```text
 *	file) because it is also used from TclRenameCommand.
 *
 *----------------------------------------------------------------------
 */

int
TclPreventAliasLoop(
    Tcl_Interp *interp,		/* Interp in which to report errors. */
    Tcl_Interp *cmdInterp,	/* Interp in which the command is being
				 * defined. */
    Tcl_Command cmd)		/* Tcl command we are attempting to define. */
{
    Command *cmdPtr = (Command *) cmd;
    Alias *aliasPtr, *nextAliasPtr;
    Tcl_Command aliasCmd;
    Command *aliasCmdPtr;

    /*
     * If we are not creating or renaming an alias, then it is always OK to
     * create or rename the command.
     */

    if (cmdPtr->objProc != AliasObjCmd) {
	return TCL_OK;
    }

    /*
     * OK, we are dealing with an alias, so traverse the chain of aliases. If
     * we encounter the alias we are defining (or renaming to) any in the
     * chain then we have a loop.
     */

    aliasPtr = (Alias *) cmdPtr->objClientData;
    nextAliasPtr = aliasPtr;
    while (1) {
	Tcl_Obj *cmdNamePtr;

	/*
	 * If the target of the next alias in the chain is the same as the
	 * source alias, we have a loop.
	 */

	if (Tcl_InterpDeleted(nextAliasPtr->targetInterp)) {
	    /*
	     * The slave interpreter can be deleted while creating the alias.
	     * [Bug #641195]
	     */

	    Tcl_AppendResult(interp, "cannot define or rename alias \"",
		    Tcl_GetCommandName(cmdInterp, cmd),
		    "\": interpreter deleted", NULL);
	    return TCL_ERROR;
	}
	cmdNamePtr = nextAliasPtr->objPtr;
	aliasCmd = Tcl_FindCommand(nextAliasPtr->targetInterp,
		TclGetString(cmdNamePtr),
		Tcl_GetGlobalNamespace(nextAliasPtr->targetInterp),
		/*flags*/ 0);
	if (aliasCmd == NULL) {
	    return TCL_OK;
	}
	aliasCmdPtr = (Command *) aliasCmd;
	if (aliasCmdPtr == cmdPtr) {
	    Tcl_AppendResult(interp, "cannot define or rename alias \"",
		    Tcl_GetCommandName(cmdInterp, cmd),
		    "\": would create a loop", NULL);
	    return TCL_ERROR;
	}

	/*
	 * Otherwise, follow the chain one step further. See if the target
	 * command is an alias - if so, follow the loop to its target command.
	 * Otherwise we do not have a loop.
	 */

	if (aliasCmdPtr->objProc != AliasObjCmd) {
	    return TCL_OK;
	}
	nextAliasPtr = (Alias *) aliasCmdPtr->objClientData;
    }

    /* NOTREACHED */
}

/*
 *----------------------------------------------------------------------
 *
 * AliasCreate --
 *
 *	Helper function to do the work to actually create an alias.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	An alias command is created and entered into the alias table for the
 *	slave interpreter.
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

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclInterp.c`, function `AliasCreate`, lines 1478–1632. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-8.6.18-tclInterp.c`.

```text
 *	child interpreter.
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

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclInterp.c`, function `AliasDescribe`, lines 1690–1843. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-8.6.18-tclInterp.c`.

```text
 *	None.
 *
 *----------------------------------------------------------------------
 */

static int
AliasDescribe(
    Tcl_Interp *interp,		/* Interpreter for result & errors. */
    Tcl_Interp *childInterp,	/* Interpreter containing alias. */
    Tcl_Obj *namePtr)		/* Name of alias to describe. */
{
    Child *childPtr;
    Tcl_HashEntry *hPtr;
    Alias *aliasPtr;
    Tcl_Obj *prefixPtr;

    /*
     * If the alias has been renamed in the child, the parent can still use
     * the original name (with which it was created) to find the alias to
     * describe it.
     */

    childPtr = &((InterpInfo *) ((Interp *) childInterp)->interpInfo)->child;
    hPtr = Tcl_FindHashEntry(&childPtr->aliasTable, TclGetString(namePtr));
    if (hPtr == NULL) {
	return TCL_OK;
    }
    aliasPtr = (Alias *)Tcl_GetHashValue(hPtr);
    prefixPtr = Tcl_NewListObj(aliasPtr->objc, &aliasPtr->objPtr);
    Tcl_SetObjResult(interp, prefixPtr);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * AliasList --
 *
 *	Computes a list of aliases defined in a child interpreter.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

static int
AliasList(
    Tcl_Interp *interp,		/* Interp for data return. */
    Tcl_Interp *childInterp)	/* Interp whose aliases to compute. */
{
    Tcl_HashEntry *entryPtr;
    Tcl_HashSearch hashSearch;
    Tcl_Obj *resultPtr;
    Alias *aliasPtr;
    Child *childPtr;

    TclNewObj(resultPtr);
    childPtr = &((InterpInfo *) ((Interp *) childInterp)->interpInfo)->child;

    entryPtr = Tcl_FirstHashEntry(&childPtr->aliasTable, &hashSearch);
    for ( ; entryPtr != NULL; entryPtr = Tcl_NextHashEntry(&hashSearch)) {
	aliasPtr = (Alias *)Tcl_GetHashValue(entryPtr);
	Tcl_ListObjAppendElement(NULL, resultPtr, aliasPtr->token);
    }
    Tcl_SetObjResult(interp, resultPtr);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * AliasObjCmd --
 *
 *	This is the function that services invocations of aliases in a child
 *	interpreter. One such command exists for each alias. When invoked,
 *	this function redirects the invocation to the target command in the
 *	parent interpreter as designated by the Alias record associated with
 *	this command.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	Causes forwarding of the invocation; all possible side effects may
 *	occur as a result of invoking the command to which the invocation is
 *	forwarded.
 *
 *----------------------------------------------------------------------
 */

static int
AliasNRCmd(
    ClientData clientData,	/* Alias record. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument vector. */
{
    Alias *aliasPtr = (Alias *)clientData;
    int prefc, cmdc, i;
    Tcl_Obj **prefv, **cmdv;
    Tcl_Obj *listPtr;
    List *listRep;
    int flags = TCL_EVAL_INVOKE;

    /*
     * Append the arguments to the command prefix and invoke the command in
     * the target interp's global namespace.
     */

    prefc = aliasPtr->objc;
    prefv = &aliasPtr->objPtr;
    cmdc = prefc + objc - 1;

    listPtr = Tcl_NewListObj(cmdc, NULL);
    listRep = (List *)listPtr->internalRep.twoPtrValue.ptr1;
    listRep->elemCount = cmdc;
    cmdv = &listRep->elements;

    prefv = &aliasPtr->objPtr;
    memcpy(cmdv, prefv, prefc * sizeof(Tcl_Obj *));
    memcpy(cmdv+prefc, objv+1, (objc-1) * sizeof(Tcl_Obj *));

    for (i=0; i<cmdc; i++) {
	Tcl_IncrRefCount(cmdv[i]);
    }

    /*
     * Use the ensemble rewriting machinery to ensure correct error messages:
     * only the source command should show, not the full target prefix.
     */

    if (TclInitRewriteEnsemble(interp, 1, prefc, objv)) {
	TclNRAddCallback(interp, TclClearRootEnsemble, NULL, NULL, NULL, NULL);
    }
    TclSkipTailcall(interp);
    return Tcl_NREvalObj(interp, listPtr, flags);
}

static int
AliasObjCmd(
    ClientData clientData,	/* Alias record. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument vector. */
{
#define ALIAS_CMDV_PREALLOC 10
    Alias *aliasPtr = (Alias *)clientData;
    Tcl_Interp *targetInterp = aliasPtr->targetInterp;
    int result, prefc, cmdc, i;
    Tcl_Obj **prefv, **cmdv;

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclInterp.c`, function `AliasObjCmd`, lines 1827–1980. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-8.6.18-tclInterp.c`.

```text
    }
    TclSkipTailcall(interp);
    return Tcl_NREvalObj(interp, listPtr, flags);
}

static int
AliasObjCmd(
    ClientData clientData,	/* Alias record. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument vector. */
{
#define ALIAS_CMDV_PREALLOC 10
    Alias *aliasPtr = (Alias *)clientData;
    Tcl_Interp *targetInterp = aliasPtr->targetInterp;
    int result, prefc, cmdc, i;
    Tcl_Obj **prefv, **cmdv;
    Tcl_Obj *cmdArr[ALIAS_CMDV_PREALLOC];
    Interp *tPtr = (Interp *) targetInterp;
    int isRootEnsemble;

    /*
     * Append the arguments to the command prefix and invoke the command in
     * the target interp's global namespace.
     */

    prefc = aliasPtr->objc;
    prefv = &aliasPtr->objPtr;
    cmdc = prefc + objc - 1;
    if (cmdc <= ALIAS_CMDV_PREALLOC) {
	cmdv = cmdArr;
    } else {
	cmdv = (Tcl_Obj **)TclStackAlloc(interp, cmdc * sizeof(Tcl_Obj *));
    }

    memcpy(cmdv, prefv, prefc * sizeof(Tcl_Obj *));
    memcpy(cmdv+prefc, objv+1, (objc-1) * sizeof(Tcl_Obj *));

    Tcl_ResetResult(targetInterp);

    for (i=0; i<cmdc; i++) {
	Tcl_IncrRefCount(cmdv[i]);
    }

    /*
     * Use the ensemble rewriting machinery to ensure correct error messages:
     * only the source command should show, not the full target prefix.
     */

    isRootEnsemble = TclInitRewriteEnsemble((Tcl_Interp *)tPtr, 1, prefc, objv);

    /*
     * Protect the target interpreter if it isn't the same as the source
     * interpreter so that we can continue to work with it after the target
     * command completes.
     */

    if (targetInterp != interp) {
	Tcl_Preserve(targetInterp);
    }

    /*
     * Execute the target command in the target interpreter.
     */

    result = Tcl_EvalObjv(targetInterp, cmdc, cmdv, TCL_EVAL_INVOKE);

    /*
     * Clean up the ensemble rewrite info if we set it in the first place.
     */

    if (isRootEnsemble) {
	TclResetRewriteEnsemble((Tcl_Interp *)tPtr, 1);
    }

    /*
     * If it was a cross-interpreter alias, we need to transfer the result
     * back to the source interpreter and release the lock we previously set
     * on the target interpreter.
     */

    if (targetInterp != interp) {
	Tcl_TransferResult(targetInterp, result, interp);
	Tcl_Release(targetInterp);
    }

    for (i=0; i<cmdc; i++) {
	Tcl_DecrRefCount(cmdv[i]);
    }
    if (cmdv != cmdArr) {
	TclStackFree(interp, cmdv);
    }
    return result;
#undef ALIAS_CMDV_PREALLOC
}

/*
 *----------------------------------------------------------------------
 *
 * AliasObjCmdDeleteProc --
 *
 *	Is invoked when an alias command is deleted in a child. Cleans up all
 *	storage associated with this alias.
 *
 * Results:
 *	None.
 *
 * Side effects:
 *	Deletes the alias record and its entry in the alias table for the
 *	interpreter.
 *
 *----------------------------------------------------------------------
 */

static void
AliasObjCmdDeleteProc(
    ClientData clientData)	/* The alias record for this alias. */
{
    Alias *aliasPtr = (Alias *)clientData;
    Target *targetPtr;
    int i;
    Tcl_Obj **objv;

    Tcl_DecrRefCount(aliasPtr->token);
    objv = &aliasPtr->objPtr;
    for (i = 0; i < aliasPtr->objc; i++) {
	Tcl_DecrRefCount(objv[i]);
    }
    Tcl_DeleteHashEntry(aliasPtr->aliasEntryPtr);

    /*
     * Splice the target record out of the target interpreter's parent list.
     */

    targetPtr = aliasPtr->targetPtr;
    if (targetPtr->prevPtr != NULL) {
	targetPtr->prevPtr->nextPtr = targetPtr->nextPtr;
    } else {
	Parent *parentPtr = &((InterpInfo *) ((Interp *)
		aliasPtr->targetInterp)->interpInfo)->parent;

	parentPtr->targetsPtr = targetPtr->nextPtr;
    }
    if (targetPtr->nextPtr != NULL) {
	targetPtr->nextPtr->prevPtr = targetPtr->prevPtr;
    }

    ckfree(targetPtr);
    ckfree(aliasPtr);
}

/*
 *----------------------------------------------------------------------
 *

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclInterp.c`, function `AliasObjCmdDeleteProc`, lines 1936–2087. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-8.6.18-tclInterp.c`.

```text
 *	interpreter.
 *
 *----------------------------------------------------------------------
 */

static void
AliasObjCmdDeleteProc(
    ClientData clientData)	/* The alias record for this alias. */
{
    Alias *aliasPtr = (Alias *)clientData;
    Target *targetPtr;
    int i;
    Tcl_Obj **objv;

    Tcl_DecrRefCount(aliasPtr->token);
    objv = &aliasPtr->objPtr;
    for (i = 0; i < aliasPtr->objc; i++) {
	Tcl_DecrRefCount(objv[i]);
    }
    Tcl_DeleteHashEntry(aliasPtr->aliasEntryPtr);

    /*
     * Splice the target record out of the target interpreter's parent list.
     */

    targetPtr = aliasPtr->targetPtr;
    if (targetPtr->prevPtr != NULL) {
	targetPtr->prevPtr->nextPtr = targetPtr->nextPtr;
    } else {
	Parent *parentPtr = &((InterpInfo *) ((Interp *)
		aliasPtr->targetInterp)->interpInfo)->parent;

	parentPtr->targetsPtr = targetPtr->nextPtr;
    }
    if (targetPtr->nextPtr != NULL) {
	targetPtr->nextPtr->prevPtr = targetPtr->prevPtr;
    }

    ckfree(targetPtr);
    ckfree(aliasPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_CreateChild --
 *
 *	Creates a child interpreter. The childPath argument denotes the name
 *	of the new child relative to the current interpreter; the child is a
 *	direct descendant of the one-before-last component of the path,
 *	e.g. it is a descendant of the current interpreter if the childPath
 *	argument contains only one component. Optionally makes the child
 *	interpreter safe.
 *
 * Results:
 *	Returns the interpreter structure created, or NULL if an error
 *	occurred.
 *
 * Side effects:
 *	Creates a new interpreter and a new interpreter object command in the
 *	interpreter indicated by the childPath argument.
 *
 *----------------------------------------------------------------------
 */

Tcl_Interp *
Tcl_CreateChild(
    Tcl_Interp *interp,		/* Interpreter to start search at. */
    const char *childPath,	/* Name of child to create. */
    int isSafe)			/* Should new child be "safe" ? */
{
    Tcl_Obj *pathPtr;
    Tcl_Interp *childInterp;

    pathPtr = Tcl_NewStringObj(childPath, -1);
    childInterp = ChildCreate(interp, pathPtr, isSafe);
    Tcl_DecrRefCount(pathPtr);

    return childInterp;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetChild --
 *
 *	Finds a child interpreter by its path name.
 *
 * Results:
 *	Returns a Tcl_Interp * for the named interpreter or NULL if not found.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

Tcl_Interp *
Tcl_GetChild(
    Tcl_Interp *interp,		/* Interpreter to start search from. */
    const char *childPath)	/* Path of child to find. */
{
    Tcl_Obj *pathPtr;
    Tcl_Interp *childInterp;

    pathPtr = Tcl_NewStringObj(childPath, -1);
    childInterp = GetInterp(interp, pathPtr);
    Tcl_DecrRefCount(pathPtr);

    return childInterp;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetParent --
 *
 *	Finds the parent interpreter of a child interpreter.
 *
 * Results:
 *	Returns a Tcl_Interp * for the parent interpreter or NULL if none.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

Tcl_Interp *
Tcl_GetParent(
    Tcl_Interp *interp)		/* Get the parent of this interpreter. */
{
    Child *childPtr;		/* Child record of this interpreter. */

    if (interp == NULL) {
	return NULL;
    }
    childPtr = &((InterpInfo *) ((Interp *) interp)->interpInfo)->child;
    return childPtr->parentInterp;
}

/*
 *----------------------------------------------------------------------
 *
 * TclSetChildCancelFlags --
 *
 *	This function marks all child interpreters belonging to a given
 *	interpreter as being canceled or not canceled, depending on the
 *	provided flags.
 *
 * Results:
 *	None.

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclInterp.c`, function `TclPreventAliasLoop`, lines 1382–1536. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-8.6.18-tclInterp.c`.

```text
 *	file) because it is also used from TclRenameCommand.
 *
 *----------------------------------------------------------------------
 */

int
TclPreventAliasLoop(
    Tcl_Interp *interp,		/* Interp in which to report errors. */
    Tcl_Interp *cmdInterp,	/* Interp in which the command is being
				 * defined. */
    Tcl_Command cmd)		/* Tcl command we are attempting to define. */
{
    Command *cmdPtr = (Command *) cmd;
    Alias *aliasPtr, *nextAliasPtr;
    Tcl_Command aliasCmd;
    Command *aliasCmdPtr;

    /*
     * If we are not creating or renaming an alias, then it is always OK to
     * create or rename the command.
     */

    if (cmdPtr->objProc != AliasObjCmd) {
	return TCL_OK;
    }

    /*
     * OK, we are dealing with an alias, so traverse the chain of aliases. If
     * we encounter the alias we are defining (or renaming to) any in the
     * chain then we have a loop.
     */

    aliasPtr = (Alias *)cmdPtr->objClientData;
    nextAliasPtr = aliasPtr;
    while (1) {
	Tcl_Obj *cmdNamePtr;

	/*
	 * If the target of the next alias in the chain is the same as the
	 * source alias, we have a loop.
	 */

	if (Tcl_InterpDeleted(nextAliasPtr->targetInterp)) {
	    /*
	     * The child interpreter can be deleted while creating the alias.
	     * [Bug #641195]
	     */

	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "cannot define or rename alias \"%s\": interpreter deleted",
		    Tcl_GetCommandName(cmdInterp, cmd)));
	    return TCL_ERROR;
	}
	cmdNamePtr = nextAliasPtr->objPtr;
	aliasCmd = Tcl_FindCommand(nextAliasPtr->targetInterp,
		TclGetString(cmdNamePtr),
		Tcl_GetGlobalNamespace(nextAliasPtr->targetInterp),
		/*flags*/ 0);
	if (aliasCmd == NULL) {
	    return TCL_OK;
	}
	aliasCmdPtr = (Command *) aliasCmd;
	if (aliasCmdPtr == cmdPtr) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "cannot define or rename alias \"%s\": would create a loop",
		    Tcl_GetCommandName(cmdInterp, cmd)));
	    Tcl_SetErrorCode(interp, "TCL", "OPERATION", "INTERP",
		    "ALIASLOOP", (char *)NULL);
	    return TCL_ERROR;
	}

	/*
	 * Otherwise, follow the chain one step further. See if the target
	 * command is an alias - if so, follow the loop to its target command.
	 * Otherwise we do not have a loop.
	 */

	if (aliasCmdPtr->objProc != AliasObjCmd) {
	    return TCL_OK;
	}
	nextAliasPtr = (Alias *)aliasCmdPtr->objClientData;
    }
}

/*
 *----------------------------------------------------------------------
 *
 * AliasCreate --
 *
 *	Helper function to do the work to actually create an alias.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	An alias command is created and entered into the alias table for the
 *	child interpreter.
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

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInterp.c`, function `TclAliasCreate`, lines 1474–1628. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-9.0.4-tclInterp.c`.

```text
 *	child interpreter.
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

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInterp.c`, function `AliasDescribe`, lines 1687–1840. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-9.0.4-tclInterp.c`.

```text
 *	None.
 *
 *----------------------------------------------------------------------
 */

static int
AliasDescribe(
    Tcl_Interp *interp,		/* Interpreter for result & errors. */
    Tcl_Interp *childInterp,	/* Interpreter containing alias. */
    Tcl_Obj *namePtr)		/* Name of alias to describe. */
{
    Child *childPtr;
    Tcl_HashEntry *hPtr;
    Alias *aliasPtr;
    Tcl_Obj *prefixPtr;

    /*
     * If the alias has been renamed in the child, the parent can still use
     * the original name (with which it was created) to find the alias to
     * describe it.
     */

    childPtr = &INTERP_INFO(childInterp)->child;
    hPtr = Tcl_FindHashEntry(&childPtr->aliasTable, TclGetString(namePtr));
    if (hPtr == NULL) {
	return TCL_OK;
    }
    aliasPtr = (Alias *) Tcl_GetHashValue(hPtr);
    prefixPtr = Tcl_NewListObj(aliasPtr->objc, &aliasPtr->objPtr);
    Tcl_SetObjResult(interp, prefixPtr);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * AliasList --
 *
 *	Computes a list of aliases defined in a child interpreter.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

static int
AliasList(
    Tcl_Interp *interp,		/* Interp for data return. */
    Tcl_Interp *childInterp)	/* Interp whose aliases to compute. */
{
    Tcl_HashEntry *entryPtr;
    Tcl_HashSearch hashSearch;
    Tcl_Obj *resultPtr;
    Alias *aliasPtr;
    Child *childPtr;

    TclNewObj(resultPtr);
    childPtr = &INTERP_INFO(childInterp)->child;

    entryPtr = Tcl_FirstHashEntry(&childPtr->aliasTable, &hashSearch);
    for ( ; entryPtr != NULL; entryPtr = Tcl_NextHashEntry(&hashSearch)) {
	aliasPtr = (Alias *) Tcl_GetHashValue(entryPtr);
	Tcl_ListObjAppendElement(NULL, resultPtr, aliasPtr->token);
    }
    Tcl_SetObjResult(interp, resultPtr);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * TclAliasObjCmd, TclLocalAliasObjCmd --
 *
 *	This is the function that services invocations of aliases in a child
 *	interpreter. One such command exists for each alias. When invoked,
 *	this function redirects the invocation to the target command in the
 *	parent interpreter as designated by the Alias record associated with
 *	this command.
 *
 *	TclLocalAliasObjCmd is a stripped down version used when the source
 *	and target interpreters of the alias are the same. That lets a number
 *	of safety precautions be avoided: the state is much more precisely
 *	known.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	Causes forwarding of the invocation; all possible side effects may
 *	occur as a result of invoking the command to which the invocation is
 *	forwarded.
 *
 *----------------------------------------------------------------------
 */

static int
AliasNRCmd(
    void *clientData,		/* Alias record. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument vector. */
{
    Alias *aliasPtr = (Alias *) clientData;
    Tcl_Size prefc, cmdc, i;
    Tcl_Obj **prefv, **cmdv;
    Tcl_Obj *listPtr;
    ListRep listRep;
    int flags = TCL_EVAL_INVOKE;

    /*
     * Append the arguments to the command prefix and invoke the command in
     * the target interp's global namespace.
     */

    prefc = aliasPtr->objc;
    prefv = &aliasPtr->objPtr;
    cmdc = prefc + objc - 1;

    /* TODO - encapsulate this into tclListObj.c */
    listPtr = Tcl_NewListObj(cmdc, NULL);
    ListObjGetRep(listPtr, &listRep);
    cmdv = ListRepElementsBase(&listRep);
    listRep.storePtr->numUsed = cmdc;
    if (listRep.spanPtr) {
	listRep.spanPtr->spanStart = listRep.storePtr->firstUsed;
	listRep.spanPtr->spanLength = listRep.storePtr->numUsed;
    }

    prefv = &aliasPtr->objPtr;
    memcpy(cmdv, prefv, prefc * sizeof(Tcl_Obj *));
    memcpy(cmdv+prefc, objv+1, (objc-1) * sizeof(Tcl_Obj *));

    for (i=0; i<cmdc; i++) {
	Tcl_IncrRefCount(cmdv[i]);
    }

    /*
     * Use the ensemble rewriting machinery to ensure correct error messages:
     * only the source command should show, not the full target prefix.
     */

    if (TclInitRewriteEnsemble(interp, 1, prefc, objv)) {
	TclNRAddCallback(interp, TclClearRootEnsemble, NULL, NULL, NULL, NULL);
    }
    TclSkipTailcall(interp);
    return Tcl_NREvalObj(interp, listPtr, flags);
}

int
TclAliasObjCmd(

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInterp.c`, function `TclAliasObjCmd`, lines 1834–1989. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-9.0.4-tclInterp.c`.

```text
    }
    TclSkipTailcall(interp);
    return Tcl_NREvalObj(interp, listPtr, flags);
}

int
TclAliasObjCmd(
    void *clientData,		/* Alias record. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument vector. */
{
#define ALIAS_CMDV_PREALLOC 10
    Alias *aliasPtr = (Alias *) clientData;
    Tcl_Interp *targetInterp = aliasPtr->targetInterp;
    int result;
    Tcl_Size prefc, cmdc, i;
    Tcl_Obj **prefv, **cmdv;
    Tcl_Obj *cmdArr[ALIAS_CMDV_PREALLOC];
    int isRootEnsemble;

    /*
     * Append the arguments to the command prefix and invoke the command in
     * the target interp's global namespace.
     */

    prefc = aliasPtr->objc;
    prefv = &aliasPtr->objPtr;
    cmdc = prefc + objc - 1;
    if (cmdc <= ALIAS_CMDV_PREALLOC) {
	cmdv = cmdArr;
    } else {
	cmdv = (Tcl_Obj **) TclStackAlloc(interp, cmdc * sizeof(Tcl_Obj *));
    }

    memcpy(cmdv, prefv, prefc * sizeof(Tcl_Obj *));
    memcpy(cmdv+prefc, objv+1, (objc-1) * sizeof(Tcl_Obj *));

    Tcl_ResetResult(targetInterp);

    for (i=0; i<cmdc; i++) {
	Tcl_IncrRefCount(cmdv[i]);
    }

    /*
     * Use the ensemble rewriting machinery to ensure correct error messages:
     * only the source command should show, not the full target prefix.
     */

    isRootEnsemble = TclInitRewriteEnsemble(targetInterp, 1, prefc, objv);

    /*
     * Protect the target interpreter if it isn't the same as the source
     * interpreter so that we can continue to work with it after the target
     * command completes.
     */

    if (targetInterp != interp) {
	Tcl_Preserve(targetInterp);
    }

    /*
     * Execute the target command in the target interpreter.
     */

    result = Tcl_EvalObjv(targetInterp, cmdc, cmdv, TCL_EVAL_INVOKE);

    /*
     * Clean up the ensemble rewrite info if we set it in the first place.
     */

    if (isRootEnsemble) {
	TclResetRewriteEnsemble(targetInterp, 1);
    }

    /*
     * If it was a cross-interpreter alias, we need to transfer the result
     * back to the source interpreter and release the lock we previously set
     * on the target interpreter.
     */

    if (targetInterp != interp) {
	Tcl_TransferResult(targetInterp, result, interp);
	Tcl_Release(targetInterp);
    }

    for (i=0; i<cmdc; i++) {
	Tcl_DecrRefCount(cmdv[i]);
    }
    if (cmdv != cmdArr) {
	TclStackFree(interp, cmdv);
    }
    return result;
#undef ALIAS_CMDV_PREALLOC
}

int
TclLocalAliasObjCmd(
    void *clientData,		/* Alias record. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument vector. */
{
#define ALIAS_CMDV_PREALLOC 10
    Alias *aliasPtr = (Alias *) clientData;
    int result;
    Tcl_Size prefc, cmdc, i;
    Tcl_Obj **prefv, **cmdv;
    Tcl_Obj *cmdArr[ALIAS_CMDV_PREALLOC];
    int isRootEnsemble;

    /*
     * Append the arguments to the command prefix and invoke the command in
     * the global namespace.
     */

    prefc = aliasPtr->objc;
    prefv = &aliasPtr->objPtr;
    cmdc = prefc + objc - 1;
    if (cmdc <= ALIAS_CMDV_PREALLOC) {
	cmdv = cmdArr;
    } else {
	cmdv = (Tcl_Obj **) TclStackAlloc(interp, cmdc * sizeof(Tcl_Obj *));
    }

    memcpy(cmdv, prefv, prefc * sizeof(Tcl_Obj *));
    memcpy(cmdv+prefc, objv+1, (objc-1) * sizeof(Tcl_Obj *));

    for (i=0; i<cmdc; i++) {
	Tcl_IncrRefCount(cmdv[i]);
    }

    /*
     * Use the ensemble rewriting machinery to ensure correct error messages:
     * only the source command should show, not the full target prefix.
     */

    isRootEnsemble = TclInitRewriteEnsemble(interp, 1, prefc, objv);

    /*
     * Execute the target command in the target interpreter.
     */

    result = Tcl_EvalObjv(interp, cmdc, cmdv, TCL_EVAL_INVOKE);

    /*
     * Clean up the ensemble rewrite info if we set it in the first place.
     */

    if (isRootEnsemble) {
	TclResetRewriteEnsemble(interp, 1);
    }

    for (i=0; i<cmdc; i++) {
	Tcl_DecrRefCount(cmdv[i]);
    }

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInterp.c`, function `TclLocalAliasObjCmd`, lines 1925–2078. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-9.0.4-tclInterp.c`.

```text
    }
    return result;
#undef ALIAS_CMDV_PREALLOC
}

int
TclLocalAliasObjCmd(
    void *clientData,		/* Alias record. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument vector. */
{
#define ALIAS_CMDV_PREALLOC 10
    Alias *aliasPtr = (Alias *) clientData;
    int result;
    Tcl_Size prefc, cmdc, i;
    Tcl_Obj **prefv, **cmdv;
    Tcl_Obj *cmdArr[ALIAS_CMDV_PREALLOC];
    int isRootEnsemble;

    /*
     * Append the arguments to the command prefix and invoke the command in
     * the global namespace.
     */

    prefc = aliasPtr->objc;
    prefv = &aliasPtr->objPtr;
    cmdc = prefc + objc - 1;
    if (cmdc <= ALIAS_CMDV_PREALLOC) {
	cmdv = cmdArr;
    } else {
	cmdv = (Tcl_Obj **) TclStackAlloc(interp, cmdc * sizeof(Tcl_Obj *));
    }

    memcpy(cmdv, prefv, prefc * sizeof(Tcl_Obj *));
    memcpy(cmdv+prefc, objv+1, (objc-1) * sizeof(Tcl_Obj *));

    for (i=0; i<cmdc; i++) {
	Tcl_IncrRefCount(cmdv[i]);
    }

    /*
     * Use the ensemble rewriting machinery to ensure correct error messages:
     * only the source command should show, not the full target prefix.
     */

    isRootEnsemble = TclInitRewriteEnsemble(interp, 1, prefc, objv);

    /*
     * Execute the target command in the target interpreter.
     */

    result = Tcl_EvalObjv(interp, cmdc, cmdv, TCL_EVAL_INVOKE);

    /*
     * Clean up the ensemble rewrite info if we set it in the first place.
     */

    if (isRootEnsemble) {
	TclResetRewriteEnsemble(interp, 1);
    }

    for (i=0; i<cmdc; i++) {
	Tcl_DecrRefCount(cmdv[i]);
    }
    if (cmdv != cmdArr) {
	TclStackFree(interp, cmdv);
    }
    return result;
#undef ALIAS_CMDV_PREALLOC
}

/*
 *----------------------------------------------------------------------
 *
 * AliasObjCmdDeleteProc --
 *
 *	Is invoked when an alias command is deleted in a child. Cleans up all
 *	storage associated with this alias.
 *
 * Results:
 *	None.
 *
 * Side effects:
 *	Deletes the alias record and its entry in the alias table for the
 *	interpreter.
 *
 *----------------------------------------------------------------------
 */

static void
AliasObjCmdDeleteProc(
    void *clientData)		/* The alias record for this alias. */
{
    Alias *aliasPtr = (Alias *) clientData;
    Target *targetPtr;
    Tcl_Size i;
    Tcl_Obj **objv;

    Tcl_DecrRefCount(aliasPtr->token);
    objv = &aliasPtr->objPtr;
    for (i = 0; i < aliasPtr->objc; i++) {
	Tcl_DecrRefCount(objv[i]);
    }
    Tcl_DeleteHashEntry(aliasPtr->aliasEntryPtr);

    /*
     * Splice the target record out of the target interpreter's parent list.
     */

    targetPtr = aliasPtr->targetPtr;
    if (targetPtr->prevPtr != NULL) {
	targetPtr->prevPtr->nextPtr = targetPtr->nextPtr;
    } else {
	Parent *parentPtr = &INTERP_INFO(aliasPtr->targetInterp)->parent;

	parentPtr->targetsPtr = targetPtr->nextPtr;
    }
    if (targetPtr->nextPtr != NULL) {
	targetPtr->nextPtr->prevPtr = targetPtr->prevPtr;
    }

    Tcl_Free(targetPtr);
    Tcl_Free(aliasPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_CreateChild --
 *
 *	Creates a child interpreter. The childPath argument denotes the name
 *	of the new child relative to the current interpreter; the child is a
 *	direct descendant of the one-before-last component of the path,
 *	e.g. it is a descendant of the current interpreter if the childPath
 *	argument contains only one component. Optionally makes the child
 *	interpreter safe.
 *
 * Results:
 *	Returns the interpreter structure created, or NULL if an error
 *	occurred.
 *
 * Side effects:
 *	Creates a new interpreter and a new interpreter object command in the
 *	interpreter indicated by the childPath argument.
 *
 *----------------------------------------------------------------------
 */

Tcl_Interp *
Tcl_CreateChild(
    Tcl_Interp *interp,		/* Interpreter to start search at. */
    const char *childPath,	/* Name of child to create. */
    int isSafe)			/* Should new child be "safe" ? */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInterp.c`, function `AliasObjCmdDeleteProc`, lines 2010–2161. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-9.0.4-tclInterp.c`.

```text
 *	interpreter.
 *
 *----------------------------------------------------------------------
 */

static void
AliasObjCmdDeleteProc(
    void *clientData)		/* The alias record for this alias. */
{
    Alias *aliasPtr = (Alias *) clientData;
    Target *targetPtr;
    Tcl_Size i;
    Tcl_Obj **objv;

    Tcl_DecrRefCount(aliasPtr->token);
    objv = &aliasPtr->objPtr;
    for (i = 0; i < aliasPtr->objc; i++) {
	Tcl_DecrRefCount(objv[i]);
    }
    Tcl_DeleteHashEntry(aliasPtr->aliasEntryPtr);

    /*
     * Splice the target record out of the target interpreter's parent list.
     */

    targetPtr = aliasPtr->targetPtr;
    if (targetPtr->prevPtr != NULL) {
	targetPtr->prevPtr->nextPtr = targetPtr->nextPtr;
    } else {
	Parent *parentPtr = &INTERP_INFO(aliasPtr->targetInterp)->parent;

	parentPtr->targetsPtr = targetPtr->nextPtr;
    }
    if (targetPtr->nextPtr != NULL) {
	targetPtr->nextPtr->prevPtr = targetPtr->prevPtr;
    }

    Tcl_Free(targetPtr);
    Tcl_Free(aliasPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_CreateChild --
 *
 *	Creates a child interpreter. The childPath argument denotes the name
 *	of the new child relative to the current interpreter; the child is a
 *	direct descendant of the one-before-last component of the path,
 *	e.g. it is a descendant of the current interpreter if the childPath
 *	argument contains only one component. Optionally makes the child
 *	interpreter safe.
 *
 * Results:
 *	Returns the interpreter structure created, or NULL if an error
 *	occurred.
 *
 * Side effects:
 *	Creates a new interpreter and a new interpreter object command in the
 *	interpreter indicated by the childPath argument.
 *
 *----------------------------------------------------------------------
 */

Tcl_Interp *
Tcl_CreateChild(
    Tcl_Interp *interp,		/* Interpreter to start search at. */
    const char *childPath,	/* Name of child to create. */
    int isSafe)			/* Should new child be "safe" ? */
{
    Tcl_Obj *pathPtr;
    Tcl_Interp *childInterp;

    pathPtr = Tcl_NewStringObj(childPath, -1);
    childInterp = ChildCreate(interp, pathPtr, isSafe);
    Tcl_DecrRefCount(pathPtr);

    return childInterp;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetChild --
 *
 *	Finds a child interpreter by its path name.
 *
 * Results:
 *	Returns a Tcl_Interp * for the named interpreter or NULL if not found.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

Tcl_Interp *
Tcl_GetChild(
    Tcl_Interp *interp,		/* Interpreter to start search from. */
    const char *childPath)	/* Path of child to find. */
{
    Tcl_Obj *pathPtr;
    Tcl_Interp *childInterp;

    pathPtr = Tcl_NewStringObj(childPath, -1);
    childInterp = GetInterp(interp, pathPtr);
    Tcl_DecrRefCount(pathPtr);

    return childInterp;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetParent --
 *
 *	Finds the parent interpreter of a child interpreter.
 *
 * Results:
 *	Returns a Tcl_Interp * for the parent interpreter or NULL if none.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

Tcl_Interp *
Tcl_GetParent(
    Tcl_Interp *interp)		/* Get the parent of this interpreter. */
{
    Child *childPtr;		/* Child record of this interpreter. */

    if (interp == NULL) {
	return NULL;
    }
    childPtr = &INTERP_INFO(interp)->child;
    return childPtr->parentInterp;
}

/*
 *----------------------------------------------------------------------
 *
 * TclSetChildCancelFlags --
 *
 *	This function marks all child interpreters belonging to a given
 *	interpreter as being canceled or not canceled, depending on the
 *	provided flags.
 *
 * Results:
 *	None.
 *

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInterp.c`, function `TclPreventAliasLoop`, lines 1376–1530. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-9.0.4-tclInterp.c`.

```text
 *	file) because it is also used from TclRenameCommand.
 *
 *----------------------------------------------------------------------
 */

int
TclPreventAliasLoop(
    Tcl_Interp *interp,		/* Interp in which to report errors. */
    Tcl_Interp *cmdInterp,	/* Interp in which the command is being
				 * defined. */
    Tcl_Command cmd)		/* Tcl command we are attempting to define. */
{
    Command *cmdPtr = (Command *) cmd;
    Alias *aliasPtr, *nextAliasPtr;
    Tcl_Command aliasCmd;
    Command *aliasCmdPtr;

    /*
     * If we are not creating or renaming an alias, then it is always OK to
     * create or rename the command.
     */

    if (cmdPtr->objProc != TclAliasObjCmd
	    && cmdPtr->objProc != TclLocalAliasObjCmd) {
	return TCL_OK;
    }

    /*
     * OK, we are dealing with an alias, so traverse the chain of aliases. If
     * we encounter the alias we are defining (or renaming to) any in the
     * chain then we have a loop.
     */

    aliasPtr = (Alias *) cmdPtr->objClientData;
    nextAliasPtr = aliasPtr;
    while (1) {
	Tcl_Obj *cmdNamePtr;

	/*
	 * If the target of the next alias in the chain is the same as the
	 * source alias, we have a loop.
	 */

	if (Tcl_InterpDeleted(nextAliasPtr->targetInterp)) {
	    /*
	     * The child interpreter can be deleted while creating the alias.
	     * [Bug #641195]
	     */

	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "cannot define or rename alias \"%s\": interpreter deleted",
		    Tcl_GetCommandName(cmdInterp, cmd)));
	    return TCL_ERROR;
	}
	cmdNamePtr = nextAliasPtr->objPtr;
	aliasCmd = Tcl_FindCommand(nextAliasPtr->targetInterp,
		TclGetString(cmdNamePtr),
		Tcl_GetGlobalNamespace(nextAliasPtr->targetInterp),
		/*flags*/ 0);
	if (aliasCmd == NULL) {
	    return TCL_OK;
	}
	aliasCmdPtr = (Command *) aliasCmd;
	if (aliasCmdPtr == cmdPtr) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "cannot define or rename alias \"%s\": would create a loop",
		    Tcl_GetCommandName(cmdInterp, cmd)));
	    Tcl_SetErrorCode(interp, "TCL", "OPERATION", "INTERP",
		    "ALIASLOOP", (char *)NULL);
	    return TCL_ERROR;
	}

	/*
	 * Otherwise, follow the chain one step further. See if the target
	 * command is an alias - if so, follow the loop to its target command.
	 * Otherwise we do not have a loop.
	 */

	if (aliasCmdPtr->objProc != TclAliasObjCmd
		&& aliasCmdPtr->objProc != TclLocalAliasObjCmd) {
	    return TCL_OK;
	}
	nextAliasPtr = (Alias *) aliasCmdPtr->objClientData;
    }
}

/*
 *----------------------------------------------------------------------
 *
 * TclAliasCreate --
 *
 *	Helper function to do the work to actually create an alias.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	An alias command is created and entered into the alias table for the
 *	child interpreter.
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

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInterp.c`, function `TclAliasCreate`, lines 1859–2013. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-9.1.0-tclInterp.c`.

```text
 *	child interpreter.
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

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInterp.c`, function `AliasDescribe`, lines 2072–2225. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-9.1.0-tclInterp.c`.

```text
 *	None.
 *
 *----------------------------------------------------------------------
 */

static int
AliasDescribe(
    Tcl_Interp *interp,		/* Interpreter for result & errors. */
    Tcl_Interp *childInterp,	/* Interpreter containing alias. */
    Tcl_Obj *namePtr)		/* Name of alias to describe. */
{
    Child *childPtr;
    Tcl_HashEntry *hPtr;
    Alias *aliasPtr;
    Tcl_Obj *prefixPtr;

    /*
     * If the alias has been renamed in the child, the parent can still use
     * the original name (with which it was created) to find the alias to
     * describe it.
     */

    childPtr = &INTERP_INFO(childInterp)->child;
    hPtr = Tcl_FindHashEntry(&childPtr->aliasTable, TclGetString(namePtr));
    if (hPtr == NULL) {
	return TCL_OK;
    }
    aliasPtr = (Alias *) Tcl_GetHashValue(hPtr);
    prefixPtr = Tcl_NewListObj(aliasPtr->objc, &aliasPtr->objPtr);
    Tcl_SetObjResult(interp, prefixPtr);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * AliasList --
 *
 *	Computes a list of aliases defined in a child interpreter.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

static int
AliasList(
    Tcl_Interp *interp,		/* Interp for data return. */
    Tcl_Interp *childInterp)	/* Interp whose aliases to compute. */
{
    Tcl_HashEntry *entryPtr;
    Tcl_HashSearch hashSearch;
    Tcl_Obj *resultPtr;
    Alias *aliasPtr;
    Child *childPtr;

    TclNewObj(resultPtr);
    childPtr = &INTERP_INFO(childInterp)->child;

    entryPtr = Tcl_FirstHashEntry(&childPtr->aliasTable, &hashSearch);
    for ( ; entryPtr != NULL; entryPtr = Tcl_NextHashEntry(&hashSearch)) {
	aliasPtr = (Alias *) Tcl_GetHashValue(entryPtr);
	Tcl_ListObjAppendElement(NULL, resultPtr, aliasPtr->token);
    }
    Tcl_SetObjResult(interp, resultPtr);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * TclAliasObjCmd, TclLocalAliasObjCmd --
 *
 *	This is the function that services invocations of aliases in a child
 *	interpreter. One such command exists for each alias. When invoked,
 *	this function redirects the invocation to the target command in the
 *	parent interpreter as designated by the Alias record associated with
 *	this command.
 *
 *	TclLocalAliasObjCmd is a stripped down version used when the source
 *	and target interpreters of the alias are the same. That lets a number
 *	of safety precautions be avoided: the state is much more precisely
 *	known.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	Causes forwarding of the invocation; all possible side effects may
 *	occur as a result of invoking the command to which the invocation is
 *	forwarded.
 *
 *----------------------------------------------------------------------
 */

static int
AliasNRCmd(
    void *clientData,		/* Alias record. */
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument vector. */
{
    Alias *aliasPtr = (Alias *) clientData;
    Tcl_Size prefc, cmdc, i;
    Tcl_Obj **prefv, **cmdv;
    Tcl_Obj *listPtr;
    ListRep listRep;
    int flags = TCL_EVAL_INVOKE;

    /*
     * Append the arguments to the command prefix and invoke the command in
     * the target interp's global namespace.
     */

    prefc = aliasPtr->objc;
    prefv = &aliasPtr->objPtr;
    cmdc = prefc + objc - 1;

    /* TODO - encapsulate this into tclListObj.c */
    listPtr = Tcl_NewListObj(cmdc, NULL);
    ListObjGetRep(listPtr, &listRep);
    cmdv = ListRepElementsBase(&listRep);
    listRep.storePtr->numUsed = cmdc;
    if (listRep.spanPtr) {
	listRep.spanPtr->spanStart = listRep.storePtr->firstUsed;
	listRep.spanPtr->spanLength = listRep.storePtr->numUsed;
    }

    prefv = &aliasPtr->objPtr;
    memcpy(cmdv, prefv, prefc * sizeof(Tcl_Obj *));
    memcpy(cmdv+prefc, objv+1, (objc-1) * sizeof(Tcl_Obj *));

    for (i=0; i<cmdc; i++) {
	Tcl_IncrRefCount(cmdv[i]);
    }

    /*
     * Use the ensemble rewriting machinery to ensure correct error messages:
     * only the source command should show, not the full target prefix.
     */

    if (TclInitRewriteEnsemble(interp, 1, prefc, objv)) {
	TclNRAddCallback(interp, TclClearRootEnsemble, NULL, NULL, NULL, NULL);
    }
    TclSkipTailcall(interp);
    return Tcl_NREvalObj(interp, listPtr, flags);
}

int
TclAliasObjCmd(

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInterp.c`, function `TclAliasObjCmd`, lines 2219–2374. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-9.1.0-tclInterp.c`.

```text
    }
    TclSkipTailcall(interp);
    return Tcl_NREvalObj(interp, listPtr, flags);
}

int
TclAliasObjCmd(
    void *clientData,		/* Alias record. */
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument vector. */
{
#define ALIAS_CMDV_PREALLOC 10
    Alias *aliasPtr = (Alias *) clientData;
    Tcl_Interp *targetInterp = aliasPtr->targetInterp;
    int result;
    Tcl_Size prefc, cmdc, i;
    Tcl_Obj **prefv, **cmdv;
    Tcl_Obj *cmdArr[ALIAS_CMDV_PREALLOC];
    int isRootEnsemble;

    /*
     * Append the arguments to the command prefix and invoke the command in
     * the target interp's global namespace.
     */

    prefc = aliasPtr->objc;
    prefv = &aliasPtr->objPtr;
    cmdc = prefc + objc - 1;
    if (cmdc <= ALIAS_CMDV_PREALLOC) {
	cmdv = cmdArr;
    } else {
	cmdv = (Tcl_Obj **) TclStackAlloc(interp, cmdc * sizeof(Tcl_Obj *));
    }

    memcpy(cmdv, prefv, prefc * sizeof(Tcl_Obj *));
    memcpy(cmdv+prefc, objv+1, (objc-1) * sizeof(Tcl_Obj *));

    Tcl_ResetResult(targetInterp);

    for (i=0; i<cmdc; i++) {
	Tcl_IncrRefCount(cmdv[i]);
    }

    /*
     * Use the ensemble rewriting machinery to ensure correct error messages:
     * only the source command should show, not the full target prefix.
     */

    isRootEnsemble = TclInitRewriteEnsemble(targetInterp, 1, prefc, objv);

    /*
     * Protect the target interpreter if it isn't the same as the source
     * interpreter so that we can continue to work with it after the target
     * command completes.
     */

    if (targetInterp != interp) {
	Tcl_Preserve(targetInterp);
    }

    /*
     * Execute the target command in the target interpreter.
     */

    result = Tcl_EvalObjv(targetInterp, cmdc, cmdv, TCL_EVAL_INVOKE);

    /*
     * Clean up the ensemble rewrite info if we set it in the first place.
     */

    if (isRootEnsemble) {
	TclResetRewriteEnsemble(targetInterp, 1);
    }

    /*
     * If it was a cross-interpreter alias, we need to transfer the result
     * back to the source interpreter and release the lock we previously set
     * on the target interpreter.
     */

    if (targetInterp != interp) {
	Tcl_TransferResult(targetInterp, result, interp);
	Tcl_Release(targetInterp);
    }

    for (i=0; i<cmdc; i++) {
	Tcl_DecrRefCount(cmdv[i]);
    }
    if (cmdv != cmdArr) {
	TclStackFree(interp, cmdv);
    }
    return result;
#undef ALIAS_CMDV_PREALLOC
}

int
TclLocalAliasObjCmd(
    void *clientData,		/* Alias record. */
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument vector. */
{
#define ALIAS_CMDV_PREALLOC 10
    Alias *aliasPtr = (Alias *) clientData;
    int result;
    Tcl_Size prefc, cmdc, i;
    Tcl_Obj **prefv, **cmdv;
    Tcl_Obj *cmdArr[ALIAS_CMDV_PREALLOC];
    int isRootEnsemble;

    /*
     * Append the arguments to the command prefix and invoke the command in
     * the global namespace.
     */

    prefc = aliasPtr->objc;
    prefv = &aliasPtr->objPtr;
    cmdc = prefc + objc - 1;
    if (cmdc <= ALIAS_CMDV_PREALLOC) {
	cmdv = cmdArr;
    } else {
	cmdv = (Tcl_Obj **) TclStackAlloc(interp, cmdc * sizeof(Tcl_Obj *));
    }

    memcpy(cmdv, prefv, prefc * sizeof(Tcl_Obj *));
    memcpy(cmdv+prefc, objv+1, (objc-1) * sizeof(Tcl_Obj *));

    for (i=0; i<cmdc; i++) {
	Tcl_IncrRefCount(cmdv[i]);
    }

    /*
     * Use the ensemble rewriting machinery to ensure correct error messages:
     * only the source command should show, not the full target prefix.
     */

    isRootEnsemble = TclInitRewriteEnsemble(interp, 1, prefc, objv);

    /*
     * Execute the target command in the target interpreter.
     */

    result = Tcl_EvalObjv(interp, cmdc, cmdv, TCL_EVAL_INVOKE);

    /*
     * Clean up the ensemble rewrite info if we set it in the first place.
     */

    if (isRootEnsemble) {
	TclResetRewriteEnsemble(interp, 1);
    }

    for (i=0; i<cmdc; i++) {
	Tcl_DecrRefCount(cmdv[i]);
    }

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInterp.c`, function `TclLocalAliasObjCmd`, lines 2310–2463. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-9.1.0-tclInterp.c`.

```text
    }
    return result;
#undef ALIAS_CMDV_PREALLOC
}

int
TclLocalAliasObjCmd(
    void *clientData,		/* Alias record. */
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument vector. */
{
#define ALIAS_CMDV_PREALLOC 10
    Alias *aliasPtr = (Alias *) clientData;
    int result;
    Tcl_Size prefc, cmdc, i;
    Tcl_Obj **prefv, **cmdv;
    Tcl_Obj *cmdArr[ALIAS_CMDV_PREALLOC];
    int isRootEnsemble;

    /*
     * Append the arguments to the command prefix and invoke the command in
     * the global namespace.
     */

    prefc = aliasPtr->objc;
    prefv = &aliasPtr->objPtr;
    cmdc = prefc + objc - 1;
    if (cmdc <= ALIAS_CMDV_PREALLOC) {
	cmdv = cmdArr;
    } else {
	cmdv = (Tcl_Obj **) TclStackAlloc(interp, cmdc * sizeof(Tcl_Obj *));
    }

    memcpy(cmdv, prefv, prefc * sizeof(Tcl_Obj *));
    memcpy(cmdv+prefc, objv+1, (objc-1) * sizeof(Tcl_Obj *));

    for (i=0; i<cmdc; i++) {
	Tcl_IncrRefCount(cmdv[i]);
    }

    /*
     * Use the ensemble rewriting machinery to ensure correct error messages:
     * only the source command should show, not the full target prefix.
     */

    isRootEnsemble = TclInitRewriteEnsemble(interp, 1, prefc, objv);

    /*
     * Execute the target command in the target interpreter.
     */

    result = Tcl_EvalObjv(interp, cmdc, cmdv, TCL_EVAL_INVOKE);

    /*
     * Clean up the ensemble rewrite info if we set it in the first place.
     */

    if (isRootEnsemble) {
	TclResetRewriteEnsemble(interp, 1);
    }

    for (i=0; i<cmdc; i++) {
	Tcl_DecrRefCount(cmdv[i]);
    }
    if (cmdv != cmdArr) {
	TclStackFree(interp, cmdv);
    }
    return result;
#undef ALIAS_CMDV_PREALLOC
}

/*
 *----------------------------------------------------------------------
 *
 * AliasObjCmdDeleteProc --
 *
 *	Is invoked when an alias command is deleted in a child. Cleans up all
 *	storage associated with this alias.
 *
 * Results:
 *	None.
 *
 * Side effects:
 *	Deletes the alias record and its entry in the alias table for the
 *	interpreter.
 *
 *----------------------------------------------------------------------
 */

static void
AliasObjCmdDeleteProc(
    void *clientData)		/* The alias record for this alias. */
{
    Alias *aliasPtr = (Alias *) clientData;
    Target *targetPtr;
    Tcl_Size i;
    Tcl_Obj **objv;

    Tcl_DecrRefCount(aliasPtr->token);
    objv = &aliasPtr->objPtr;
    for (i = 0; i < aliasPtr->objc; i++) {
	Tcl_DecrRefCount(objv[i]);
    }
    Tcl_DeleteHashEntry(aliasPtr->aliasEntryPtr);

    /*
     * Splice the target record out of the target interpreter's parent list.
     */

    targetPtr = aliasPtr->targetPtr;
    if (targetPtr->prevPtr != NULL) {
	targetPtr->prevPtr->nextPtr = targetPtr->nextPtr;
    } else {
	Parent *parentPtr = &INTERP_INFO(aliasPtr->targetInterp)->parent;

	parentPtr->targetsPtr = targetPtr->nextPtr;
    }
    if (targetPtr->nextPtr != NULL) {
	targetPtr->nextPtr->prevPtr = targetPtr->prevPtr;
    }

    Tcl_Free(targetPtr);
    Tcl_Free(aliasPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_CreateChild --
 *
 *	Creates a child interpreter. The childPath argument denotes the name
 *	of the new child relative to the current interpreter; the child is a
 *	direct descendant of the one-before-last component of the path,
 *	e.g. it is a descendant of the current interpreter if the childPath
 *	argument contains only one component. Optionally makes the child
 *	interpreter safe.
 *
 * Results:
 *	Returns the interpreter structure created, or NULL if an error
 *	occurred.
 *
 * Side effects:
 *	Creates a new interpreter and a new interpreter object command in the
 *	interpreter indicated by the childPath argument.
 *
 *----------------------------------------------------------------------
 */

Tcl_Interp *
Tcl_CreateChild(
    Tcl_Interp *interp,		/* Interpreter to start search at. */
    const char *childPath,	/* Name of child to create. */
    int isSafe)			/* Should new child be "safe" ? */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInterp.c`, function `AliasObjCmdDeleteProc`, lines 2395–2546. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-9.1.0-tclInterp.c`.

```text
 *	interpreter.
 *
 *----------------------------------------------------------------------
 */

static void
AliasObjCmdDeleteProc(
    void *clientData)		/* The alias record for this alias. */
{
    Alias *aliasPtr = (Alias *) clientData;
    Target *targetPtr;
    Tcl_Size i;
    Tcl_Obj **objv;

    Tcl_DecrRefCount(aliasPtr->token);
    objv = &aliasPtr->objPtr;
    for (i = 0; i < aliasPtr->objc; i++) {
	Tcl_DecrRefCount(objv[i]);
    }
    Tcl_DeleteHashEntry(aliasPtr->aliasEntryPtr);

    /*
     * Splice the target record out of the target interpreter's parent list.
     */

    targetPtr = aliasPtr->targetPtr;
    if (targetPtr->prevPtr != NULL) {
	targetPtr->prevPtr->nextPtr = targetPtr->nextPtr;
    } else {
	Parent *parentPtr = &INTERP_INFO(aliasPtr->targetInterp)->parent;

	parentPtr->targetsPtr = targetPtr->nextPtr;
    }
    if (targetPtr->nextPtr != NULL) {
	targetPtr->nextPtr->prevPtr = targetPtr->prevPtr;
    }

    Tcl_Free(targetPtr);
    Tcl_Free(aliasPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_CreateChild --
 *
 *	Creates a child interpreter. The childPath argument denotes the name
 *	of the new child relative to the current interpreter; the child is a
 *	direct descendant of the one-before-last component of the path,
 *	e.g. it is a descendant of the current interpreter if the childPath
 *	argument contains only one component. Optionally makes the child
 *	interpreter safe.
 *
 * Results:
 *	Returns the interpreter structure created, or NULL if an error
 *	occurred.
 *
 * Side effects:
 *	Creates a new interpreter and a new interpreter object command in the
 *	interpreter indicated by the childPath argument.
 *
 *----------------------------------------------------------------------
 */

Tcl_Interp *
Tcl_CreateChild(
    Tcl_Interp *interp,		/* Interpreter to start search at. */
    const char *childPath,	/* Name of child to create. */
    int isSafe)			/* Should new child be "safe" ? */
{
    Tcl_Obj *pathPtr;
    Tcl_Interp *childInterp;

    pathPtr = Tcl_NewStringObj(childPath, -1);
    childInterp = ChildCreate(interp, pathPtr, isSafe);
    Tcl_DecrRefCount(pathPtr);

    return childInterp;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetChild --
 *
 *	Finds a child interpreter by its path name.
 *
 * Results:
 *	Returns a Tcl_Interp * for the named interpreter or NULL if not found.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

Tcl_Interp *
Tcl_GetChild(
    Tcl_Interp *interp,		/* Interpreter to start search from. */
    const char *childPath)	/* Path of child to find. */
{
    Tcl_Obj *pathPtr;
    Tcl_Interp *childInterp;

    pathPtr = Tcl_NewStringObj(childPath, -1);
    childInterp = GetInterp(interp, pathPtr);
    Tcl_DecrRefCount(pathPtr);

    return childInterp;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetParent --
 *
 *	Finds the parent interpreter of a child interpreter.
 *
 * Results:
 *	Returns a Tcl_Interp * for the parent interpreter or NULL if none.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

Tcl_Interp *
Tcl_GetParent(
    Tcl_Interp *interp)		/* Get the parent of this interpreter. */
{
    Child *childPtr;		/* Child record of this interpreter. */

    if (interp == NULL) {
	return NULL;
    }
    childPtr = &INTERP_INFO(interp)->child;
    return childPtr->parentInterp;
}

/*
 *----------------------------------------------------------------------
 *
 * TclSetChildCancelFlags --
 *
 *	This function marks all child interpreters belonging to a given
 *	interpreter as being canceled or not canceled, depending on the
 *	provided flags.
 *
 * Results:
 *	None.
 *

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInterp.c`, function `TclPreventAliasLoop`, lines 1761–1915. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-9.1.0-tclInterp.c`.

```text
 *	file) because it is also used from TclRenameCommand.
 *
 *----------------------------------------------------------------------
 */

int
TclPreventAliasLoop(
    Tcl_Interp *interp,		/* Interp in which to report errors. */
    Tcl_Interp *cmdInterp,	/* Interp in which the command is being
				 * defined. */
    Tcl_Command cmd)		/* Tcl command we are attempting to define. */
{
    Command *cmdPtr = (Command *) cmd;
    Alias *aliasPtr, *nextAliasPtr;
    Tcl_Command aliasCmd;
    Command *aliasCmdPtr;

    /*
     * If we are not creating or renaming an alias, then it is always OK to
     * create or rename the command.
     */

    if (cmdPtr->objProc2 != TclAliasObjCmd
	    && cmdPtr->objProc2 != TclLocalAliasObjCmd) {
	return TCL_OK;
    }

    /*
     * OK, we are dealing with an alias, so traverse the chain of aliases. If
     * we encounter the alias we are defining (or renaming to) any in the
     * chain then we have a loop.
     */

    aliasPtr = (Alias *)cmdPtr->objClientData2;
    nextAliasPtr = aliasPtr;
    while (1) {
	Tcl_Obj *cmdNamePtr;

	/*
	 * If the target of the next alias in the chain is the same as the
	 * source alias, we have a loop.
	 */

	if (Tcl_InterpDeleted(nextAliasPtr->targetInterp)) {
	    /*
	     * The child interpreter can be deleted while creating the alias.
	     * [Bug #641195]
	     */

	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "cannot define or rename alias \"%s\": interpreter deleted",
		    Tcl_GetCommandName(cmdInterp, cmd)));
	    return TCL_ERROR;
	}
	cmdNamePtr = nextAliasPtr->objPtr;
	aliasCmd = Tcl_FindCommand(nextAliasPtr->targetInterp,
		TclGetString(cmdNamePtr),
		Tcl_GetGlobalNamespace(nextAliasPtr->targetInterp),
		/*flags*/ 0);
	if (aliasCmd == NULL) {
	    return TCL_OK;
	}
	aliasCmdPtr = (Command *) aliasCmd;
	if (aliasCmdPtr == cmdPtr) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "cannot define or rename alias \"%s\": would create a loop",
		    Tcl_GetCommandName(cmdInterp, cmd)));
	    Tcl_SetErrorCode(interp, "TCL", "OPERATION", "INTERP",
		    "ALIASLOOP", (char *)NULL);
	    return TCL_ERROR;
	}

	/*
	 * Otherwise, follow the chain one step further. See if the target
	 * command is an alias - if so, follow the loop to its target command.
	 * Otherwise we do not have a loop.
	 */

	if (aliasCmdPtr->objProc2 != TclAliasObjCmd
		&& aliasCmdPtr->objProc2 != TclLocalAliasObjCmd) {
	    return TCL_OK;
	}
	nextAliasPtr = (Alias *)aliasCmdPtr->objClientData2;
    }
}

/*
 *----------------------------------------------------------------------
 *
 * TclAliasCreate --
 *
 *	Helper function to do the work to actually create an alias.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	An alias command is created and entered into the alias table for the
 *	child interpreter.
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

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole source retained`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimAliasCmd`, lines 14412–14567. Retained reference `rust-tcl-registry-tests-data-native_jim_alias_error_extent279-request-jim.c`.

```text
        return JIM_EVAL;
    }
    return JIM_OK;
}

/* Invoke an alias by prepending its stored command prefix. */
static int JimAliasCmd(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_Obj *cmdList;
    Jim_Obj *prefixListObj = Jim_CmdPrivData(interp);

    /* prefixListObj is a list to which the args need to be appended */
    cmdList = Jim_DuplicateObj(interp, prefixListObj);
    Jim_ListInsertElements(interp, cmdList, Jim_ListLength(interp, cmdList), argc - 1, argv + 1);

    return JimEvalObjList(interp, cmdList);
}

/* Release the command prefix stored for an alias. */
static void JimAliasCmdDelete(Jim_Interp *interp, void *privData)
{
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

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole source retained`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimAliasCmdDelete`, lines 14425–14580. Retained reference `rust-tcl-registry-tests-data-native_jim_alias_error_extent279-request-jim.c`.

```text
    Jim_ListInsertElements(interp, cmdList, Jim_ListLength(interp, cmdList), argc - 1, argv + 1);

    return JimEvalObjList(interp, cmdList);
}

/* Release the command prefix stored for an alias. */
static void JimAliasCmdDelete(Jim_Interp *interp, void *privData)
{
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

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole source retained`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_AliasCoreCommand`, lines 14432–14587. Retained reference `rust-tcl-registry-tests-data-native_jim_alias_error_extent279-request-jim.c`.

```text
{
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

```

tcl8.4 8.4.20, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInterp.c`, function `OPT_ALIAS`, lines 382–416. Retained reference `rust-tcl-registry-tests-data-native_info_commands_literal_original-original-inputs-8.4.20-generic-tclInterp.c`.

```text
	case OPT_ALIAS: {
	    Tcl_Interp *slaveInterp, *masterInterp;

	    if (objc < 4) {
		aliasArgs:
		Tcl_WrongNumArgs(interp, 2, objv,
			"slavePath slaveCmd ?masterPath masterCmd? ?args ..?");
		return TCL_ERROR;
	    }
	    slaveInterp = GetInterp(interp, objv[2]);
	    if (slaveInterp == (Tcl_Interp *) NULL) {
		return TCL_ERROR;
	    }
	    if (objc == 4) {
		return AliasDescribe(interp, slaveInterp, objv[3]);
	    }
	    if ((objc == 5) && (Tcl_GetString(objv[4])[0] == '\0')) {
		return AliasDelete(interp, slaveInterp, objv[3]);
	    }
	    if (objc > 5) {
		masterInterp = GetInterp(interp, objv[4]);
		if (masterInterp == (Tcl_Interp *) NULL) {
		    return TCL_ERROR;
		}
		if (Tcl_GetString(objv[5])[0] == '\0') {
		    if (objc == 6) {
			return AliasDelete(interp, slaveInterp, objv[3]);
		    }
		} else {
		    return AliasCreate(interp, slaveInterp, masterInterp,
			    objv[3], objv[5], objc - 6, objv + 6);
		}
	    }
	    goto aliasArgs;
	}

```

tcl8.5 8.5.19, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInterp.c`, function `OPT_ALIAS`, lines 616–650. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-8.5.19-tclInterp.c`.

```text
    case OPT_ALIAS: {
	Tcl_Interp *slaveInterp, *masterInterp;

	if (objc < 4) {
	aliasArgs:
	    Tcl_WrongNumArgs(interp, 2, objv,
		    "slavePath slaveCmd ?masterPath masterCmd? ?args ..?");
	    return TCL_ERROR;
	}
	slaveInterp = GetInterp(interp, objv[2]);
	if (slaveInterp == NULL) {
	    return TCL_ERROR;
	}
	if (objc == 4) {
	    return AliasDescribe(interp, slaveInterp, objv[3]);
	}
	if ((objc == 5) && (TclGetString(objv[4])[0] == '\0')) {
	    return AliasDelete(interp, slaveInterp, objv[3]);
	}
	if (objc > 5) {
	    masterInterp = GetInterp(interp, objv[4]);
	    if (masterInterp == NULL) {
		return TCL_ERROR;
	    }
	    if (TclGetString(objv[5])[0] == '\0') {
		if (objc == 6) {
		    return AliasDelete(interp, slaveInterp, objv[3]);
		}
	    } else {
		return AliasCreate(interp, slaveInterp, masterInterp, objv[3],
			objv[5], objc - 6, objv + 6);
	    }
	}
	goto aliasArgs;
    }

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclInterp.c`, function `OPT_ALIAS`, lines 636–665. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-8.6.18-tclInterp.c`.

```text
    case OPT_ALIAS: {
	Tcl_Interp *parentInterp;

	if (objc < 4) {
	aliasArgs:
	    Tcl_WrongNumArgs(interp, 2, objv,
		    "slavePath slaveCmd ?masterPath masterCmd? ?arg ...?");
	    return TCL_ERROR;
	}
	childInterp = GetInterp(interp, objv[2]);
	if (childInterp == NULL) {
	    return TCL_ERROR;
	}
	if (objc == 4) {
	    return AliasDescribe(interp, childInterp, objv[3]);
	}
	if ((objc == 5) && (TclGetString(objv[4])[0] == '\0')) {
	    return AliasDelete(interp, childInterp, objv[3]);
	}
	if (objc > 5) {
	    parentInterp = GetInterp(interp, objv[4]);
	    if (parentInterp == NULL) {
		return TCL_ERROR;
	    }

	    return AliasCreate(interp, childInterp, parentInterp, objv[3],
		    objv[5], objc - 6, objv + 6);
	}
	goto aliasArgs;
    }

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInterp.c`, function `OPT_ALIAS`, lines 692–722. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-9.0.4-tclInterp.c`.

```text
    case OPT_ALIAS: {
	Tcl_Interp *parentInterp;

	if (objc < 4) {
	    goto aliasArgs;
	}
	childInterp = GetInterp(interp, objv[2]);
	if (childInterp == NULL) {
	    return TCL_ERROR;
	}
	if (objc == 4) {
	    return AliasDescribe(interp, childInterp, objv[3]);
	}
	if ((objc == 5) && (TclGetString(objv[4])[0] == '\0')) {
	    return AliasDelete(interp, childInterp, objv[3]);
	}
	if (objc > 5) {
	    parentInterp = GetInterp(interp, objv[4]);
	    if (parentInterp == NULL) {
		return TCL_ERROR;
	    }

	    return TclAliasCreate(interp, childInterp, parentInterp, objv[3],
		    objv[5], objc - 6, objv + 6);
	}

    aliasArgs:
	Tcl_WrongNumArgs(interp, 2, objv,
		"childPath childCmd ?parentPath parentCmd? ?arg ...?");
	return TCL_ERROR;
    }

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInterp.c`, function `OPT_ALIAS`, lines 1066–1096. Retained reference `rust-tcl-registry-tests-data-native_event_original-source-9.1.0-tclInterp.c`.

```text
    case OPT_ALIAS: {
	Tcl_Interp *parentInterp;

	if (objc < 4) {
	    goto aliasArgs;
	}
	childInterp = GetInterp(interp, objv[2]);
	if (childInterp == NULL) {
	    return TCL_ERROR;
	}
	if (objc == 4) {
	    return AliasDescribe(interp, childInterp, objv[3]);
	}
	if ((objc == 5) && (Tcl_IsEmpty(objv[4]))) {
	    return AliasDelete(interp, childInterp, objv[3]);
	}
	if (objc > 5) {
	    parentInterp = GetInterp(interp, objv[4]);
	    if (parentInterp == NULL) {
		return TCL_ERROR;
	    }

	    return TclAliasCreate(interp, childInterp, parentInterp, objv[3],
		    objv[5], objc - 6, objv + 6);
	}

    aliasArgs:
	Tcl_WrongNumArgs(interp, 2, objv,
		"childPath childCmd ?parentPath parentCmd? ?arg ...?");
	return TCL_ERROR;
    }

```

## Consumer bindings

- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `cmd_alias::original_operand_tests::original_alias_prefix_refusal_is_reached_only_at_actual_invocation` (linked): Finite source-defined control: original alias prefix refusal is reached only at actual invocation. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `cmd_alias::original_operand_tests::original_child_handle_alias_reads_the_selected_operand_before_publication` (linked): Finite source-defined control: original child handle alias reads the selected operand before publication. This actual worker control distinguishes the C target pretest from Jim's List-first capture. It does not equate child and core name roles. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `cmd_alias::original_alias_storage_tests::original_alias_capture_and_query_match_the_measured_member_and_header_recipe` (linked): Finite source-defined control: original alias capture and query match the measured member and header recipe. Bounded exact physical fields; Jim body-only name updater refs are not compared with its public EvalObjVector registration invocation. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `cmd_alias::original_alias_storage_tests::original_alias_query_retains_unread_members_until_the_last_query_is_released` (linked): Finite source-defined control: original alias query retains unread members until the last query is released. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `cmd_alias::original_alias_storage_tests::original_child_handle_alias_retains_lazy_parent_members_and_selected_name_role` (linked): Finite source-defined control: original child handle alias retains lazy parent members and selected name role. C aliases own the supplied name; Jim_RegisterCmd owns a fresh child key. These are software role controls, not unmeasured native counts. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.

- [runtime/rust/src/cmd_oo/original_object_resolution_tests.rs](../../../../runtime/rust/src/cmd_oo/original_object_resolution_tests.rs), `cmd_oo::original_object_resolution_tests::original_jim_alias_lookup_uses_the_stored_list_and_keeps_absent_context_terminal` (linked): Jim alias lookup uses the stored list and keeps absent context terminal. This is a source-defined software control without an attached matching current owned assertion outcome. It grants no external-provider observation, Native chronology, physical reference-count/frame identity or execution permission.

## Evidence

- [runtime/rust/tests/data](../../../../runtime/rust/tests/data)
- [runtime/rust/tests/data/native_alias_object_prefix_capture_original/415](../../../../runtime/rust/tests/data/native_alias_object_prefix_capture_original/415)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/499c/providers](../../../../rust/tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/499c/providers)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs)
- [rust/tcl-registry/tests/data/native_child_bootstrap_context303/source](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303/source)
- [rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/jim](../../../../rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/jim)
- [rust/tcl-test-support/tests/data/native_resolution_reference_builds](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix)
- [runtime/rust/tests/data/native_alias_object_prefix_capture_original](../../../../runtime/rust/tests/data/native_alias_object_prefix_capture_original)
