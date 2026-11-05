# naming.invocation.original-naming-argument-getter-and-effect-order

Kind: `native-observation`

## Problem statement

Reading a whole batch of namespace or procedure arguments before the selected original worker can move a later String getter ahead of an earlier mutation or Guest failure. Retaining only final successful bytes also loses the reached prefix effects and release-specific procedure/body stages. This question keeps the original lazy getters, exact intermediate public state and finite worker outcomes together without assuming C and Jim share the same namespace or procedure recipe.

## Question

On the fixed original namespace export/import/forget and procedure declarations, which original lazy argument String getters are reached, what export list or command presence is visible at each reached getter, and which earlier mutations survive a later Guest failure? How do the independently measured C and Jim worker purposes differ?

## Conclusion

All five original C releases agree on the fifteen fresh-interpreter public cases. Export clear is visible before the first reached invalid-pattern getter on C85 and later; a valid first export/import/forget effect remains when the next pattern fails, and the later third getter is not reached. C84 callback export-list observation is explicitly unavailable, while its genuine public follow-up export rows agree. Later -clear is an ordinary export pattern. Procedure unknown-namespace failure reaches only its name; valid-name malformed-formal and successful declarations reach name, body, then formal-list Strings (A,C,B); the successful invocation returns OK. Fixed binary-produced NUL suffixes retain the exact observed C truncated-name/pattern purpose, and the UTF8 payload export keeps its accepted prefix before the later error. The fresh global-operand Jim capture has an export helper with no lazy String reads, import alias processing that reaches later patterns despite missing source matches, an actual unsupported forget subcommand error, and procedure declaration B,A order with body C reached only by successful invocation. Exact public Jim bytes and query spellings remain independent of C namespace and formal validation. Runs499b and499c are separate retained captures: 499b is partial before Jim execution, and499c Jim variables fail before the intended getters. The closed host processes and finite callback/public rows do not attest Runtime parity, original binding generations, Native handler/frame/source issuers or BIG-IP.

## Scope

Exact ASCII/LF original C99 probe and fixed source scripts, fifteen fresh initialized interpreters/provider. Lazy extension objects start stringless with original public header/count fields and explicit hex payloads; genuine updateString callbacks snapshot selected public export-list and direct command-existence state before installing their own String. ROW records public evaluation code/result bytes and cumulative getter count, with actual setup/version/worker/export/command/invocation phases separate. C84 lacks the selected public Tcl_AppendExportList callback observer and records NA; Jim has no C export-list axis and its distribution namespace helper purpose stays separate. Hex-produced NUL and UTF8 payloads are original produced bytes, not source NUL/nonASCII characters or generic counted name/numeric permission. Whole requests/source/windows/compile/link/dependency streams and before/after executable identities are retained; required source/header/library/control/system inputs are independently whole-hash inspected. Probe ABI reports the actual int/long/refcount sizes. C84/C85 successful compilations retain their original const-qualifier assignment warnings on stderr; other captured process stderr is empty. SDK source/library inputs are retained, configure/build processes lack replay receipts, and exact external system compiler/header/link/runtime payloads are hash-recorded with explicit replay requirements. Source interpretation of already launch-pinned full original files remains independent of public reached fields. No original internal stock-worker stop, compile/body duplication/bytecode, callback Host/Runtime Scope, command generation/token/namespace incarnation, SSA/edit authority, Rust assertion or BIG-IP observation is supplied.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original stringless lazy extension objects from explicit ASCII hex; fixed namespace/procedure scripts, actual callback public export/name query and code/result follow-up channels. Version: tcl8.4 8.4.20.

Fifteen original fresh cases retain exact public code/result and reached A/B/C callback state. Export clear/append and import/forget commit per reached operand; a later Guest failure retains the first effect and blocks a later third getter. Unknown procedure namespace reaches only name A; valid-name malformed-list and successful declarations reach A,C,B, with successful invocation OK. Binary NUL suffix and UTF8 prefix cases retain exact observed fields. Callback-time export list is NA because the selected public C84 headers lack the API; genuine public follow-up export rows remain measured. The independent reached C runs499b/c agree on the finite fields but are not spliced into499d.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original stringless lazy extension objects from explicit ASCII hex; fixed namespace/procedure scripts, actual callback public export/name query and code/result follow-up channels. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Fifteen original fresh cases retain exact public code/result and reached A/B/C callback state. Export clear/append and import/forget commit per reached operand; a later Guest failure retains the first effect and blocks a later third getter. Unknown procedure namespace reaches only name A; valid-name malformed-list and successful declarations reach A,C,B, with successful invocation OK. Binary NUL suffix and UTF8 prefix cases retain exact observed fields. Callback-time direct export snapshots observe clear before first invalid pattern and accepted first prefix before the second failure. The independent reached C runs499b/c agree on the finite fields but are not spliced into499d.

### jim

Status: `observed`. Channel: Original stringless lazy extension objects from explicit ASCII hex; fixed namespace/procedure scripts, actual callback public export/name query and code/result follow-up channels. Version: jim 0.84-9-g5bac7c9.

The independent global-operand run499d completes fifteen original fresh cases. Export cases0-4,12,14 return0/empty with no lazy getter. Imports5/6 reach A,B,C, case7 A,B, case13 A,B, return0 and retain the exact observed alias-command state. Forget8 returns actual unknown-subcommand Guest error before lazy getters. Procedure9/10 declarations reach formal B then name A and return0; case11 declaration reaches B,A, then its successful public invocation reaches body C and returns OK. Exact query spelling (including ::::d::p) and callback counts are preserved, with no C namespace or formal-validator recipe. Run499c independently returns cannot-read-a for workers1-14 before any getter; run499b has no Jim execute/dependency receipt. These do not supply Jim chronology.

### bigip

Status: `not-tested`.

No original BIG-IP namespace/procedure lazy argument or getter/effect process was requested or executed.

## Source inspection

tcl8.4 8.4.20, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclNamesp.c`, function `NamespaceExportCmd`, lines 3123–3185. Retained reference `native_ensemble_map_prefix312-map-source-8.4.20-tclNamesp.c`.

```text
static int
NamespaceExportCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    int firstArg, i;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 2, objv,
	        "?-clear? ?pattern pattern...?");
        return TCL_ERROR;
    }

    /*
     * If no pattern arguments are given, and "-clear" isn't specified,
     * return the namespace's current export pattern list.
     */

    if (objc == 2) {
	Tcl_Obj *listPtr = Tcl_NewObj();

	(void) Tcl_AppendExportList(interp, NULL, listPtr);
	Tcl_SetObjResult(interp, listPtr);
	return TCL_OK;
    }

    /*
     * Process the optional "-clear" argument.
     */

    firstArg = 2;
    if (strcmp("-clear", Tcl_GetString(objv[firstArg])) == 0) {
	Tcl_Export(interp, NULL, "::", 1);
	Tcl_ResetResult(interp);
	firstArg++;
    }

    /*
     * Add each pattern to the namespace's export pattern list.
     */
    
    for (i = firstArg;  i < objc;  i++) {
	int result = Tcl_Export(interp, NULL, Tcl_GetString(objv[i]), 0);
        if (result != TCL_OK) {
            return result;
        }
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespaceForgetCmd --
 *
 *	Invoked to implement the "namespace forget" command to remove
 *	imported commands from a namespace. Handles the following syntax:
 *
 *	    namespace forget ?pattern pattern...?
 *
 *	Each pattern is a name like "foo::*" or "a::b::x*". That is, the

```

tcl8.4 8.4.20, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclNamesp.c`, function `NamespaceImportCmd`, lines 3267–3377. Retained reference `native_ensemble_map_prefix312-map-source-8.4.20-tclNamesp.c`.

```text
static int
NamespaceImportCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    int allowOverwrite = 0;
    char *string, *pattern;
    register int i, result;
    int firstArg;

    if (objc < 2) {
        Tcl_WrongNumArgs(interp, 2, objv,
	        "?-force? ?pattern pattern...?");
        return TCL_ERROR;
    }

    /*
     * Skip over the optional "-force" as the first argument.
     */

    firstArg = 2;
    if (firstArg < objc) {
	string = Tcl_GetString(objv[firstArg]);
	if ((*string == '-') && (strcmp(string, "-force") == 0)) {
	    allowOverwrite = 1;
	    firstArg++;
	}
    }

    /*
     * Handle the imports for each of the patterns.
     */

    for (i = firstArg;  i < objc;  i++) {
        pattern = Tcl_GetString(objv[i]);
	result = Tcl_Import(interp, (Tcl_Namespace *) NULL, pattern,
	        allowOverwrite);
        if (result != TCL_OK) {
            return result;
        }
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespaceInscopeCmd --
 *
 *	Invoked to implement the "namespace inscope" command that executes a
 *	script in the context of a particular namespace. This command is not
 *	expected to be used directly by programmers; calls to it are
 *	generated implicitly when programs use "namespace code" commands
 *	to register callback scripts. Handles the following syntax:
 *
 *	    namespace inscope name arg ?arg...?
 *
 *	The "namespace inscope" command is much like the "namespace eval"
 *	command except that it has lappend semantics and the namespace must
 *	already exist. It treats the first argument as a list, and appends
 *	any arguments after the first onto the end as proper list elements.
 *	For example,
 *
 *	    namespace inscope ::foo a b c d
 *
 *	is equivalent to
 *
 *	    namespace eval ::foo [concat a [list b c d]]
 *
 *	This lappend semantics is important because many callback scripts
 *	are actually prefixes.
 *
 * Results:
 *	Returns TCL_OK to indicate success, or TCL_ERROR to indicate
 *	failure.
 *
 * Side effects:
 *	Returns a result in the Tcl interpreter's result object.
 *
 *----------------------------------------------------------------------
 */

static int
NamespaceInscopeCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    Tcl_Namespace *namespacePtr;
    Tcl_CallFrame frame;
    int i, result;

    if (objc < 4) {
	Tcl_WrongNumArgs(interp, 2, objv, "name arg ?arg...?");
        return TCL_ERROR;
    }

    /*
     * Resolve the namespace reference.
     */

    result = GetNamespaceFromObj(interp, objv[2], &namespacePtr);
    if (result != TCL_OK) {
        return result;
    }
    if (namespacePtr == NULL) {
	Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
	        "unknown namespace \"", Tcl_GetString(objv[2]),

```

tcl8.4 8.4.20, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclNamesp.c`, function `NamespaceForgetCmd`, lines 3205–3255. Retained reference `native_ensemble_map_prefix312-map-source-8.4.20-tclNamesp.c`.

```text
static int
NamespaceForgetCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    char *pattern;
    register int i, result;

    if (objc < 2) {
        Tcl_WrongNumArgs(interp, 2, objv, "?pattern pattern...?");
        return TCL_ERROR;
    }

    for (i = 2;  i < objc;  i++) {
        pattern = Tcl_GetString(objv[i]);
	result = Tcl_ForgetImport(interp, (Tcl_Namespace *) NULL, pattern);
        if (result != TCL_OK) {
            return result;
        }
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespaceImportCmd --
 *
 *	Invoked to implement the "namespace import" command that imports
 *	commands into a namespace. Handles the following syntax:
 *
 *	    namespace import ?-force? ?pattern pattern...?
 *
 *	Each pattern is a namespace-qualified name like "foo::*",
 *	"a::b::x*", or "bar::p". That is, the pattern may include the
 *	special pattern matching characters recognized by the "string match"
 *	command, but only in the command name at the end of the qualified
 *	name; the special pattern characters may not appear in a namespace
 *	name. All of the commands that match the pattern and which are
 *	exported from their namespace are made accessible from the current
 *	namespace context. This is done by creating a new "imported command"
 *	in the current namespace that points to the real command in its
 *	original namespace; when the imported command is called, it invokes
 *	the real command.
 *
 *	If an imported command conflicts with an existing command, it is
 *	treated as an error. But if the "-force" option is included, then
 *	existing commands are overwritten by the imported commands.
 *	

```

tcl8.4 8.4.20, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclProc.c`, function `Tcl_ProcObjCmd`, lines 66–160. Retained reference `native_child_colon_publication-full-source-8.4.20-tclProc.c`.

```text
int
Tcl_ProcObjCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    register Interp *iPtr = (Interp *) interp;
    Proc *procPtr;
    char *fullName;
    CONST char *procName, *procArgs, *procBody;
    Namespace *nsPtr, *altNsPtr, *cxtNsPtr;
    Tcl_Command cmd;
    Tcl_DString ds;

    if (objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "name args body");
	return TCL_ERROR;
    }

    /*
     * Determine the namespace where the procedure should reside. Unless
     * the command name includes namespace qualifiers, this will be the
     * current namespace.
     */

    fullName = TclGetString(objv[1]);
    TclGetNamespaceForQualName(interp, fullName, (Namespace *) NULL,
	    0, &nsPtr, &altNsPtr, &cxtNsPtr, &procName);

    if (nsPtr == NULL) {
        Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
		"can't create procedure \"", fullName,
		"\": unknown namespace", (char *) NULL);
        return TCL_ERROR;
    }
    if (procName == NULL) {
	Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
		"can't create procedure \"", fullName,
		"\": bad procedure name", (char *) NULL);
        return TCL_ERROR;
    }
    if ((nsPtr != iPtr->globalNsPtr)
	    && (procName != NULL) && (procName[0] == ':')) {
	Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
		"can't create procedure \"", procName,
		"\" in non-global namespace with name starting with \":\"",
	        (char *) NULL);
        return TCL_ERROR;
    }

    /*
     *  Create the data structure to represent the procedure.
     */
    if (TclCreateProc(interp, nsPtr, procName, objv[2], objv[3],
        &procPtr) != TCL_OK) {
        return TCL_ERROR;
    }

    /*
     * Now create a command for the procedure. This will initially be in
     * the current namespace unless the procedure's name included namespace
     * qualifiers. To create the new command in the right namespace, we
     * generate a fully qualified name for it.
     */

    Tcl_DStringInit(&ds);
    if (nsPtr != iPtr->globalNsPtr) {
	Tcl_DStringAppend(&ds, nsPtr->fullName, -1);
	Tcl_DStringAppend(&ds, "::", 2);
    }
    Tcl_DStringAppend(&ds, procName, -1);

    Tcl_CreateCommand(interp, Tcl_DStringValue(&ds), TclProcInterpProc,
	    (ClientData) procPtr, TclProcDeleteProc);
    cmd = Tcl_CreateObjCommand(interp, Tcl_DStringValue(&ds),
	    TclObjInterpProc, (ClientData) procPtr, TclProcDeleteProc);

    Tcl_DStringFree(&ds);
    /*
     * Now initialize the new procedure's cmdPtr field. This will be used
     * later when the procedure is called to determine what namespace the
     * procedure will run in. This will be different than the current
     * namespace if the proc was renamed into a different namespace.
     */

    procPtr->cmdPtr = (Command *) cmd;

#ifdef TCL_TIP280
    /* TIP #280 Remember the line the procedure body is starting on. In a
     * Byte code context we ask the engine to provide us with the necessary
     * information. This is for the initialization of the byte code compiler
     * when the body is used for the first time.
     */

```

tcl8.5 8.5.19, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclNamesp.c`, function `NamespaceExportCmd`, lines 3428–3490. Retained reference `native_ensemble_holder_routing205-source-inspection208-8.5.19-tclNamesp.c`.

```text
static int
NamespaceExportCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int firstArg, i;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 2, objv, "?-clear? ?pattern pattern...?");
	return TCL_ERROR;
    }

    /*
     * If no pattern arguments are given, and "-clear" isn't specified, return
     * the namespace's current export pattern list.
     */

    if (objc == 2) {
	Tcl_Obj *listPtr = Tcl_NewObj();

	(void) Tcl_AppendExportList(interp, NULL, listPtr);
	Tcl_SetObjResult(interp, listPtr);
	return TCL_OK;
    }

    /*
     * Process the optional "-clear" argument.
     */

    firstArg = 2;
    if (strcmp("-clear", Tcl_GetString(objv[firstArg])) == 0) {
	Tcl_Export(interp, NULL, "::", 1);
	Tcl_ResetResult(interp);
	firstArg++;
    }

    /*
     * Add each pattern to the namespace's export pattern list.
     */

    for (i = firstArg;  i < objc;  i++) {
	int result = Tcl_Export(interp, NULL, Tcl_GetString(objv[i]), 0);
	if (result != TCL_OK) {
	    return result;
	}
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespaceForgetCmd --
 *
 *	Invoked to implement the "namespace forget" command to remove imported
 *	commands from a namespace. Handles the following syntax:
 *
 *	    namespace forget ?pattern pattern...?
 *
 *	Each pattern is a name like "foo::*" or "a::b::x*". That is, the
 *	pattern may include the special pattern matching characters recognized

```

tcl8.5 8.5.19, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclNamesp.c`, function `NamespaceImportCmd`, lines 3574–3684. Retained reference `native_ensemble_holder_routing205-source-inspection208-8.5.19-tclNamesp.c`.

```text
static int
NamespaceImportCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int allowOverwrite = 0;
    char *string, *pattern;
    register int i, result;
    int firstArg;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 2, objv, "?-force? ?pattern pattern...?");
	return TCL_ERROR;
    }

    /*
     * Skip over the optional "-force" as the first argument.
     */

    firstArg = 2;
    if (firstArg < objc) {
	string = TclGetString(objv[firstArg]);
	if ((*string == '-') && (strcmp(string, "-force") == 0)) {
	    allowOverwrite = 1;
	    firstArg++;
	}
    } else {
	/*
	 * When objc == 2, command is just [namespace import]. Introspection
	 * form to return list of imported commands.
	 */

	Tcl_HashEntry *hPtr;
	Tcl_HashSearch search;
	Namespace *nsPtr = (Namespace *) TclGetCurrentNamespace(interp);
	Tcl_Obj *listPtr;

	TclNewObj(listPtr);
	for (hPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search);
		hPtr != NULL; hPtr = Tcl_NextHashEntry(&search)) {
	    Command *cmdPtr = Tcl_GetHashValue(hPtr);

	    if (cmdPtr->deleteProc == DeleteImportedCmd) {
		Tcl_ListObjAppendElement(NULL, listPtr, Tcl_NewStringObj(
			Tcl_GetHashKey(&nsPtr->cmdTable, hPtr) ,-1));
	    }
	}
	Tcl_SetObjResult(interp, listPtr);
	return TCL_OK;
    }

    /*
     * Handle the imports for each of the patterns.
     */

    for (i = firstArg;  i < objc;  i++) {
	pattern = TclGetString(objv[i]);
	result = Tcl_Import(interp, NULL, pattern, allowOverwrite);
	if (result != TCL_OK) {
	    return result;
	}
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespaceInscopeCmd --
 *
 *	Invoked to implement the "namespace inscope" command that executes a
 *	script in the context of a particular namespace. This command is not
 *	expected to be used directly by programmers; calls to it are generated
 *	implicitly when programs use "namespace code" commands to register
 *	callback scripts. Handles the following syntax:
 *
 *	    namespace inscope name arg ?arg...?
 *
 *	The "namespace inscope" command is much like the "namespace eval"
 *	command except that it has lappend semantics and the namespace must
 *	already exist. It treats the first argument as a list, and appends any
 *	arguments after the first onto the end as proper list elements. For
 *	example,
 *
 *	    namespace inscope ::foo {a b} c d e
 *
 *	is equivalent to
 *
 *	    namespace eval ::foo [concat {a b} [list c d e]]
 *
 *	This lappend semantics is important because many callback scripts are
 *	actually prefixes.
 *
 * Results:
 *	Returns TCL_OK to indicate success, or TCL_ERROR to indicate failure.
 *
 * Side effects:
 *	Returns a result in the Tcl interpreter's result object.
 *
 *----------------------------------------------------------------------
 */

static int
NamespaceInscopeCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{

```

tcl8.5 8.5.19, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclNamesp.c`, function `NamespaceForgetCmd`, lines 3509–3559. Retained reference `native_ensemble_holder_routing205-source-inspection208-8.5.19-tclNamesp.c`.

```text
static int
NamespaceForgetCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    char *pattern;
    register int i, result;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 2, objv, "?pattern pattern...?");
	return TCL_ERROR;
    }

    for (i = 2;  i < objc;  i++) {
	pattern = TclGetString(objv[i]);
	result = Tcl_ForgetImport(interp, NULL, pattern);
	if (result != TCL_OK) {
	    return result;
	}
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespaceImportCmd --
 *
 *	Invoked to implement the "namespace import" command that imports
 *	commands into a namespace. Handles the following syntax:
 *
 *	    namespace import ?-force? ?pattern pattern...?
 *
 *	Each pattern is a namespace-qualified name like "foo::*", "a::b::x*",
 *	or "bar::p". That is, the pattern may include the special pattern
 *	matching characters recognized by the "string match" command, but only
 *	in the command name at the end of the qualified name; the special
 *	pattern characters may not appear in a namespace name. All of the
 *	commands that match the pattern and which are exported from their
 *	namespace are made accessible from the current namespace context. This
 *	is done by creating a new "imported command" in the current namespace
 *	that points to the real command in its original namespace; when the
 *	imported command is called, it invokes the real command.
 *
 *	If an imported command conflicts with an existing command, it is
 *	treated as an error. But if the "-force" option is included, then
 *	existing commands are overwritten by the imported commands.
 *
 *	If there are no pattern arguments and the "-force" flag isn't given,

```

tcl8.5 8.5.19, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclProc.c`, function `Tcl_ProcObjCmd`, lines 112–206. Retained reference `native_compiled_local_formal_source272-8.5.19-tclProc.c`.

```text
int
Tcl_ProcObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *CONST objv[])	/* Argument objects. */
{
    register Interp *iPtr = (Interp *) interp;
    Proc *procPtr;
    char *fullName;
    CONST char *procName, *procArgs, *procBody;
    Namespace *nsPtr, *altNsPtr, *cxtNsPtr;
    Tcl_Command cmd;
    Tcl_DString ds;

    if (objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "name args body");
	return TCL_ERROR;
    }

    /*
     * Determine the namespace where the procedure should reside. Unless the
     * command name includes namespace qualifiers, this will be the current
     * namespace.
     */

    fullName = TclGetString(objv[1]);
    TclGetNamespaceForQualName(interp, fullName, NULL, 0,
	    &nsPtr, &altNsPtr, &cxtNsPtr, &procName);

    if (nsPtr == NULL) {
	Tcl_AppendResult(interp, "can't create procedure \"", fullName,
		"\": unknown namespace", NULL);
	return TCL_ERROR;
    }
    if (procName == NULL) {
	Tcl_AppendResult(interp, "can't create procedure \"", fullName,
		"\": bad procedure name", NULL);
	return TCL_ERROR;
    }
    if ((nsPtr != iPtr->globalNsPtr)
	    && (procName != NULL) && (procName[0] == ':')) {
	Tcl_AppendResult(interp, "can't create procedure \"", procName,
		"\" in non-global namespace with name starting with \":\"",
		NULL);
	return TCL_ERROR;
    }

    /*
     * Create the data structure to represent the procedure.
     */

    if (TclCreateProc(interp, nsPtr, procName, objv[2], objv[3],
	    &procPtr) != TCL_OK) {
	Tcl_AddErrorInfo(interp, "\n    (creating proc \"");
	Tcl_AddErrorInfo(interp, procName);
	Tcl_AddErrorInfo(interp, "\")");
	return TCL_ERROR;
    }

    /*
     * Now create a command for the procedure. This will initially be in the
     * current namespace unless the procedure's name included namespace
     * qualifiers. To create the new command in the right namespace, we
     * generate a fully qualified name for it.
     */

    Tcl_DStringInit(&ds);
    if (nsPtr != iPtr->globalNsPtr) {
	Tcl_DStringAppend(&ds, nsPtr->fullName, -1);
	Tcl_DStringAppend(&ds, "::", 2);
    }
    Tcl_DStringAppend(&ds, procName, -1);

    cmd = Tcl_CreateObjCommand(interp, Tcl_DStringValue(&ds),
	    TclObjInterpProc, (ClientData) procPtr, TclProcDeleteProc);

    Tcl_DStringFree(&ds);

    /*
     * Now initialize the new procedure's cmdPtr field. This will be used
     * later when the procedure is called to determine what namespace the
     * procedure will run in. This will be different than the current
     * namespace if the proc was renamed into a different namespace.
     */

    procPtr->cmdPtr = (Command *) cmd;

    /*
     * TIP #280: Remember the line the procedure body is starting on. In a
     * bytecode context we ask the engine to provide us with the necessary
     * information. This is for the initialization of the byte code compiler
     * when the body is used for the first time.
     *
     * This code is nearly identical to the #280 code in SetLambdaFromAny, see

```

tcl8.6 8.6.18, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclNamesp.c`, function `NamespaceExportCmd`, lines 3459–3521. Retained reference `native_oo_original_collision249-request-source-anchors-8.6.18-tclNamesp.c`.

```text
static int
NamespaceExportCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int firstArg, i;

    if (objc < 1) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-clear? ?pattern pattern...?");
	return TCL_ERROR;
    }

    /*
     * If no pattern arguments are given, and "-clear" isn't specified, return
     * the namespace's current export pattern list.
     */

    if (objc == 1) {
	Tcl_Obj *listPtr;

	TclNewObj(listPtr);
	(void)Tcl_AppendExportList(interp, NULL, listPtr);
	Tcl_SetObjResult(interp, listPtr);
	return TCL_OK;
    }

    /*
     * Process the optional "-clear" argument.
     */

    firstArg = 1;
    if (strcmp("-clear", TclGetString(objv[firstArg])) == 0) {
	Tcl_Export(interp, NULL, "::", 1);
	Tcl_ResetResult(interp);
	firstArg++;
    }

    /*
     * Add each pattern to the namespace's export pattern list.
     */

    for (i = firstArg;  i < objc;  i++) {
	int result = Tcl_Export(interp, NULL, TclGetString(objv[i]), 0);
	if (result != TCL_OK) {
	    return result;
	}
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespaceForgetCmd --
 *
 *	Invoked to implement the "namespace forget" command to remove imported
 *	commands from a namespace. Handles the following syntax:
 *
 *	    namespace forget ?pattern pattern...?
 *
 *	Each pattern is a name like "foo::*" or "a::b::x*". That is, the

```

tcl8.6 8.6.18, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclNamesp.c`, function `NamespaceImportCmd`, lines 3606–3716. Retained reference `native_oo_original_collision249-request-source-anchors-8.6.18-tclNamesp.c`.

```text
static int
NamespaceImportCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int allowOverwrite = 0;
    const char *string, *pattern;
    int i, result;
    int firstArg;

    if (objc < 1) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-force? ?pattern pattern...?");
	return TCL_ERROR;
    }

    /*
     * Skip over the optional "-force" as the first argument.
     */

    firstArg = 1;
    if (firstArg < objc) {
	string = TclGetString(objv[firstArg]);
	if ((*string == '-') && (strcmp(string, "-force") == 0)) {
	    allowOverwrite = 1;
	    firstArg++;
	}
    } else {
	/*
	 * When objc == 1, command is just [namespace import]. Introspection
	 * form to return list of imported commands.
	 */

	Tcl_HashEntry *hPtr;
	Tcl_HashSearch search;
	Namespace *nsPtr = (Namespace *) TclGetCurrentNamespace(interp);
	Tcl_Obj *listPtr;

	TclNewObj(listPtr);
	for (hPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search);
		hPtr != NULL; hPtr = Tcl_NextHashEntry(&search)) {
	    Command *cmdPtr = (Command *)Tcl_GetHashValue(hPtr);

	    if (cmdPtr->deleteProc == DeleteImportedCmd) {
		Tcl_ListObjAppendElement(NULL, listPtr, Tcl_NewStringObj(
			(char *)Tcl_GetHashKey(&nsPtr->cmdTable, hPtr), -1));
	    }
	}
	Tcl_SetObjResult(interp, listPtr);
	return TCL_OK;
    }

    /*
     * Handle the imports for each of the patterns.
     */

    for (i = firstArg;  i < objc;  i++) {
	pattern = TclGetString(objv[i]);
	result = Tcl_Import(interp, NULL, pattern, allowOverwrite);
	if (result != TCL_OK) {
	    return result;
	}
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespaceInscopeCmd --
 *
 *	Invoked to implement the "namespace inscope" command that executes a
 *	script in the context of a particular namespace. This command is not
 *	expected to be used directly by programmers; calls to it are generated
 *	implicitly when programs use "namespace code" commands to register
 *	callback scripts. Handles the following syntax:
 *
 *	    namespace inscope name arg ?arg...?
 *
 *	The "namespace inscope" command is much like the "namespace eval"
 *	command except that it has lappend semantics and the namespace must
 *	already exist. It treats the first argument as a list, and appends any
 *	arguments after the first onto the end as proper list elements. For
 *	example,
 *
 *	    namespace inscope ::foo {a b} c d e
 *
 *	is equivalent to
 *
 *	    namespace eval ::foo [concat {a b} [list c d e]]
 *
 *	This lappend semantics is important because many callback scripts are
 *	actually prefixes.
 *
 * Results:
 *	Returns TCL_OK to indicate success, or TCL_ERROR to indicate failure.
 *
 * Side effects:
 *	Returns a result in the Tcl interpreter's result object.
 *
 *----------------------------------------------------------------------
 */

static int
NamespaceInscopeCmd(
    ClientData clientData,	/* Arbitrary value passed to cmd. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{

```

tcl8.6 8.6.18, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclNamesp.c`, function `NamespaceForgetCmd`, lines 3541–3591. Retained reference `native_oo_original_collision249-request-source-anchors-8.6.18-tclNamesp.c`.

```text
static int
NamespaceForgetCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    const char *pattern;
    int i, result;

    if (objc < 1) {
	Tcl_WrongNumArgs(interp, 1, objv, "?pattern pattern...?");
	return TCL_ERROR;
    }

    for (i = 1;  i < objc;  i++) {
	pattern = TclGetString(objv[i]);
	result = Tcl_ForgetImport(interp, NULL, pattern);
	if (result != TCL_OK) {
	    return result;
	}
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespaceImportCmd --
 *
 *	Invoked to implement the "namespace import" command that imports
 *	commands into a namespace. Handles the following syntax:
 *
 *	    namespace import ?-force? ?pattern pattern...?
 *
 *	Each pattern is a namespace-qualified name like "foo::*", "a::b::x*",
 *	or "bar::p". That is, the pattern may include the special pattern
 *	matching characters recognized by the "string match" command, but only
 *	in the command name at the end of the qualified name; the special
 *	pattern characters may not appear in a namespace name. All of the
 *	commands that match the pattern and which are exported from their
 *	namespace are made accessible from the current namespace context. This
 *	is done by creating a new "imported command" in the current namespace
 *	that points to the real command in its original namespace; when the
 *	imported command is called, it invokes the real command.
 *
 *	If an imported command conflicts with an existing command, it is
 *	treated as an error. But if the "-force" option is included, then
 *	existing commands are overwritten by the imported commands.
 *
 *	If there are no pattern arguments and the "-force" flag isn't given,

```

tcl8.6 8.6.18, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclProc.c`, function `Tcl_ProcObjCmd`, lines 118–212. Retained reference `native_compiled_local_formal_source272-8.6.18-tclProc.c`.

```text
int
Tcl_ProcObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Interp *iPtr = (Interp *) interp;
    Proc *procPtr;
    const char *procName;
    const char *simpleName, *procArgs, *procBody;
    Namespace *nsPtr, *altNsPtr, *cxtNsPtr;
    Tcl_Command cmd;

    if (objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "name args body");
	return TCL_ERROR;
    }

    /*
     * Determine the namespace where the procedure should reside. Unless the
     * command name includes namespace qualifiers, this will be the current
     * namespace.
     */

    procName = TclGetString(objv[1]);
    TclGetNamespaceForQualName(interp, procName, NULL, 0,
	    &nsPtr, &altNsPtr, &cxtNsPtr, &simpleName);

    if (nsPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"can't create procedure \"%s\": unknown namespace",
		procName));
	Tcl_SetErrorCode(interp, "TCL", "VALUE", "COMMAND", (char *)NULL);
	return TCL_ERROR;
    }
    if (simpleName == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"can't create procedure \"%s\": bad procedure name",
		procName));
	Tcl_SetErrorCode(interp, "TCL", "VALUE", "COMMAND", (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Create the data structure to represent the procedure.
     */

    if (TclCreateProc(interp, nsPtr, simpleName, objv[2],
	    objv[3], &procPtr) != TCL_OK) {
	Tcl_AddErrorInfo(interp, "\n    (creating proc \"");
	Tcl_AddErrorInfo(interp, simpleName);
	Tcl_AddErrorInfo(interp, "\")");
	return TCL_ERROR;
    }

    cmd = TclNRCreateCommandInNs(interp, simpleName, (Tcl_Namespace *) nsPtr,
	TclObjInterpProc, TclNRInterpProc, procPtr, TclProcDeleteProc);

    /*
     * Now initialize the new procedure's cmdPtr field. This will be used
     * later when the procedure is called to determine what namespace the
     * procedure will run in. This will be different than the current
     * namespace if the proc was renamed into a different namespace.
     */

    procPtr->cmdPtr = (Command *) cmd;

    /*
     * TIP #280: Remember the line the procedure body is starting on. In a
     * bytecode context we ask the engine to provide us with the necessary
     * information. This is for the initialization of the byte code compiler
     * when the body is used for the first time.
     *
     * This code is nearly identical to the #280 code in SetLambdaFromAny, see
     * this file. The differences are the different index of the body in the
     * line array of the context, and the lambda code requires some special
     * processing. Find a way to factor the common elements into a single
     * function.
     */

    if (iPtr->cmdFramePtr) {
	CmdFrame *contextPtr = (CmdFrame *)TclStackAlloc(interp, sizeof(CmdFrame));

	*contextPtr = *iPtr->cmdFramePtr;
	if (contextPtr->type == TCL_LOCATION_BC) {
	    /*
	     * Retrieve source information from the bytecode, if possible. If
	     * the information is retrieved successfully, context.type will be
	     * TCL_LOCATION_SOURCE and the reference held by
	     * context.data.eval.path will be counted.
	     */

	    TclGetSrcInfoForPc(contextPtr);
	} else if (contextPtr->type == TCL_LOCATION_SOURCE) {

```

tcl9.0 9.0.4, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclNamesp.c`, function `NamespaceExportCmd`, lines 3665–3727. Retained reference `native_oo_original_collision249-request-source-anchors-9.0.4-tclNamesp.c`.

```text
static int
NamespaceExportCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int firstArg, i;

    if (objc < 1) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-clear? ?pattern pattern...?");
	return TCL_ERROR;
    }

    /*
     * If no pattern arguments are given, and "-clear" isn't specified, return
     * the namespace's current export pattern list.
     */

    if (objc == 1) {
	Tcl_Obj *listPtr;

	TclNewObj(listPtr);
	(void) Tcl_AppendExportList(interp, NULL, listPtr);
	Tcl_SetObjResult(interp, listPtr);
	return TCL_OK;
    }

    /*
     * Process the optional "-clear" argument.
     */

    firstArg = 1;
    if (strcmp("-clear", TclGetString(objv[firstArg])) == 0) {
	Tcl_Export(interp, NULL, "::", 1);
	Tcl_ResetResult(interp);
	firstArg++;
    }

    /*
     * Add each pattern to the namespace's export pattern list.
     */

    for (i = firstArg;  i < objc;  i++) {
	int result = Tcl_Export(interp, NULL, TclGetString(objv[i]), 0);
	if (result != TCL_OK) {
	    return result;
	}
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespaceForgetCmd --
 *
 *	Invoked to implement the "namespace forget" command to remove imported
 *	commands from a namespace. Handles the following syntax:
 *
 *	    namespace forget ?pattern pattern...?
 *
 *	Each pattern is a name like "foo::*" or "a::b::x*". That is, the

```

tcl9.0 9.0.4, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclNamesp.c`, function `NamespaceImportCmd`, lines 3812–3922. Retained reference `native_oo_original_collision249-request-source-anchors-9.0.4-tclNamesp.c`.

```text
static int
NamespaceImportCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int allowOverwrite = 0;
    const char *string, *pattern;
    int i, result;
    int firstArg;

    if (objc < 1) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-force? ?pattern pattern...?");
	return TCL_ERROR;
    }

    /*
     * Skip over the optional "-force" as the first argument.
     */

    firstArg = 1;
    if (firstArg < objc) {
	string = TclGetString(objv[firstArg]);
	if ((*string == '-') && (strcmp(string, "-force") == 0)) {
	    allowOverwrite = 1;
	    firstArg++;
	}
    } else {
	/*
	 * When objc == 1, command is just [namespace import]. Introspection
	 * form to return list of imported commands.
	 */

	Tcl_HashEntry *hPtr;
	Tcl_HashSearch search;
	Namespace *nsPtr = (Namespace *) TclGetCurrentNamespace(interp);
	Tcl_Obj *listPtr;

	TclNewObj(listPtr);
	for (hPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search);
		hPtr != NULL; hPtr = Tcl_NextHashEntry(&search)) {
	    Command *cmdPtr = (Command *) Tcl_GetHashValue(hPtr);

	    if (cmdPtr->deleteProc == DeleteImportedCmd) {
		Tcl_ListObjAppendElement(NULL, listPtr, Tcl_NewStringObj(
			(char *) Tcl_GetHashKey(&nsPtr->cmdTable, hPtr), -1));
	    }
	}
	Tcl_SetObjResult(interp, listPtr);
	return TCL_OK;
    }

    /*
     * Handle the imports for each of the patterns.
     */

    for (i = firstArg;  i < objc;  i++) {
	pattern = TclGetString(objv[i]);
	result = Tcl_Import(interp, NULL, pattern, allowOverwrite);
	if (result != TCL_OK) {
	    return result;
	}
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespaceInscopeCmd --
 *
 *	Invoked to implement the "namespace inscope" command that executes a
 *	script in the context of a particular namespace. This command is not
 *	expected to be used directly by programmers; calls to it are generated
 *	implicitly when programs use "namespace code" commands to register
 *	callback scripts. Handles the following syntax:
 *
 *	    namespace inscope name arg ?arg...?
 *
 *	The "namespace inscope" command is much like the "namespace eval"
 *	command except that it has lappend semantics and the namespace must
 *	already exist. It treats the first argument as a list, and appends any
 *	arguments after the first onto the end as proper list elements. For
 *	example,
 *
 *	    namespace inscope ::foo {a b} c d e
 *
 *	is equivalent to
 *
 *	    namespace eval ::foo [concat {a b} [list c d e]]
 *
 *	This lappend semantics is important because many callback scripts are
 *	actually prefixes.
 *
 * Results:
 *	Returns TCL_OK to indicate success, or TCL_ERROR to indicate failure.
 *
 * Side effects:
 *	Returns a result in the Tcl interpreter's result object.
 *
 *----------------------------------------------------------------------
 */

static int
NamespaceInscopeCmd(
    void *clientData,		/* Arbitrary value passed to cmd. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{

```

tcl9.0 9.0.4, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclNamesp.c`, function `NamespaceForgetCmd`, lines 3747–3797. Retained reference `native_oo_original_collision249-request-source-anchors-9.0.4-tclNamesp.c`.

```text
static int
NamespaceForgetCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    const char *pattern;
    int i, result;

    if (objc < 1) {
	Tcl_WrongNumArgs(interp, 1, objv, "?pattern pattern...?");
	return TCL_ERROR;
    }

    for (i = 1;  i < objc;  i++) {
	pattern = TclGetString(objv[i]);
	result = Tcl_ForgetImport(interp, NULL, pattern);
	if (result != TCL_OK) {
	    return result;
	}
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespaceImportCmd --
 *
 *	Invoked to implement the "namespace import" command that imports
 *	commands into a namespace. Handles the following syntax:
 *
 *	    namespace import ?-force? ?pattern pattern...?
 *
 *	Each pattern is a namespace-qualified name like "foo::*", "a::b::x*",
 *	or "bar::p". That is, the pattern may include the special pattern
 *	matching characters recognized by the "string match" command, but only
 *	in the command name at the end of the qualified name; the special
 *	pattern characters may not appear in a namespace name. All of the
 *	commands that match the pattern and which are exported from their
 *	namespace are made accessible from the current namespace context. This
 *	is done by creating a new "imported command" in the current namespace
 *	that points to the real command in its original namespace; when the
 *	imported command is called, it invokes the real command.
 *
 *	If an imported command conflicts with an existing command, it is
 *	treated as an error. But if the "-force" option is included, then
 *	existing commands are overwritten by the imported commands.
 *
 *	If there are no pattern arguments and the "-force" flag isn't given,

```

tcl9.0 9.0.4, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclProc.c`, function `Tcl_ProcObjCmd`, lines 157–251. Retained reference `native_compiled_local_formal_source272-9.0.4-tclProc.c`.

```text
int
Tcl_ProcObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Interp *iPtr = (Interp *) interp;
    Proc *procPtr;
    const char *procName;
    const char *simpleName, *procArgs, *procBody;
    Namespace *nsPtr, *altNsPtr, *cxtNsPtr;
    Tcl_Command cmd;

    if (objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "name args body");
	return TCL_ERROR;
    }

    /*
     * Determine the namespace where the procedure should reside. Unless the
     * command name includes namespace qualifiers, this will be the current
     * namespace.
     */

    procName = TclGetString(objv[1]);
    TclGetNamespaceForQualName(interp, procName, NULL, 0,
	    &nsPtr, &altNsPtr, &cxtNsPtr, &simpleName);

    if (nsPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"can't create procedure \"%s\": unknown namespace",
		procName));
	Tcl_SetErrorCode(interp, "TCL", "VALUE", "COMMAND", (char *)NULL);
	return TCL_ERROR;
    }
    if (simpleName == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"can't create procedure \"%s\": bad procedure name",
		procName));
	Tcl_SetErrorCode(interp, "TCL", "VALUE", "COMMAND", (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Create the data structure to represent the procedure.
     */

    if (TclCreateProc(interp, /*ignored nsPtr*/ NULL, simpleName, objv[2],
	    objv[3], &procPtr) != TCL_OK) {
	Tcl_AddErrorInfo(interp, "\n    (creating proc \"");
	Tcl_AddErrorInfo(interp, simpleName);
	Tcl_AddErrorInfo(interp, "\")");
	return TCL_ERROR;
    }

    cmd = TclNRCreateCommandInNs(interp, simpleName, (Tcl_Namespace *) nsPtr,
	TclObjInterpProc, NRInterpProc, procPtr, TclProcDeleteProc);

    /*
     * Now initialize the new procedure's cmdPtr field. This will be used
     * later when the procedure is called to determine what namespace the
     * procedure will run in. This will be different than the current
     * namespace if the proc was renamed into a different namespace.
     */

    procPtr->cmdPtr = (Command *) cmd;

    /*
     * TIP #280: Remember the line the procedure body is starting on. In a
     * bytecode context we ask the engine to provide us with the necessary
     * information. This is for the initialization of the byte code compiler
     * when the body is used for the first time.
     *
     * This code is nearly identical to the #280 code in SetLambdaFromAny, see
     * this file. The differences are the different index of the body in the
     * line array of the context, and the lambda code requires some special
     * processing. Find a way to factor the common elements into a single
     * function.
     */

    if (iPtr->cmdFramePtr) {
	CmdFrame *contextPtr = (CmdFrame *)TclStackAlloc(interp, sizeof(CmdFrame));

	*contextPtr = *iPtr->cmdFramePtr;
	if (contextPtr->type == TCL_LOCATION_BC) {
	    /*
	     * Retrieve source information from the bytecode, if possible. If
	     * the information is retrieved successfully, context.type will be
	     * TCL_LOCATION_SOURCE and the reference held by
	     * context.data.eval.path will be counted.
	     */

	    TclGetSrcInfoForPc(contextPtr);
	} else if (contextPtr->type == TCL_LOCATION_SOURCE) {

```

tcl9.1 9.1.0, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclNamesp.c`, function `NamespaceExportCmd`, lines 3637–3699. Retained reference `native_oo_original_collision249-request-source-anchors-9.1.0-tclNamesp.c`.

```text
static int
NamespaceExportCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Size firstArg, i;

    if (objc < 1) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-clear? ?pattern pattern...?");
	return TCL_ERROR;
    }

    /*
     * If no pattern arguments are given, and "-clear" isn't specified, return
     * the namespace's current export pattern list.
     */

    if (objc == 1) {
	Tcl_Obj *listPtr;

	TclNewObj(listPtr);
	(void) Tcl_AppendExportList(interp, NULL, listPtr);
	Tcl_SetObjResult(interp, listPtr);
	return TCL_OK;
    }

    /*
     * Process the optional "-clear" argument.
     */

    firstArg = 1;
    if (strcmp("-clear", TclGetString(objv[firstArg])) == 0) {
	Tcl_Export(interp, NULL, "::", 1);
	Tcl_ResetResult(interp);
	firstArg++;
    }

    /*
     * Add each pattern to the namespace's export pattern list.
     */

    for (i = firstArg;  i < objc;  i++) {
	int result = Tcl_Export(interp, NULL, TclGetString(objv[i]), 0);
	if (result != TCL_OK) {
	    return result;
	}
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespaceForgetCmd --
 *
 *	Invoked to implement the "namespace forget" command to remove imported
 *	commands from a namespace. Handles the following syntax:
 *
 *	    namespace forget ?pattern pattern...?
 *
 *	Each pattern is a name like "foo::*" or "a::b::x*". That is, the

```

tcl9.1 9.1.0, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclNamesp.c`, function `NamespaceImportCmd`, lines 3785–3895. Retained reference `native_oo_original_collision249-request-source-anchors-9.1.0-tclNamesp.c`.

```text
static int
NamespaceImportCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    bool allowOverwrite = false;
    const char *string, *pattern;
    Tcl_Size i, firstArg;
    int result;

    if (objc < 1) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-force? ?pattern pattern...?");
	return TCL_ERROR;
    }

    /*
     * Skip over the optional "-force" as the first argument.
     */

    firstArg = 1;
    if (firstArg < objc) {
	string = TclGetString(objv[firstArg]);
	if ((*string == '-') && (strcmp(string, "-force") == 0)) {
	    allowOverwrite = true;
	    firstArg++;
	}
    } else {
	/*
	 * When objc == 1, command is just [namespace import]. Introspection
	 * form to return list of imported commands.
	 */

	Tcl_HashEntry *hPtr;
	Tcl_HashSearch search;
	Namespace *nsPtr = (Namespace *) TclGetCurrentNamespace(interp);
	Tcl_Obj *listPtr;

	TclNewObj(listPtr);
	for (hPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search);
		hPtr != NULL; hPtr = Tcl_NextHashEntry(&search)) {
	    Command *cmdPtr = (Command *) Tcl_GetHashValue(hPtr);

	    if (cmdPtr->deleteProc == DeleteImportedCmd) {
		Tcl_ListObjAppendElement(NULL, listPtr, Tcl_NewStringObj(
			(char *) Tcl_GetHashKey(&nsPtr->cmdTable, hPtr), -1));
	    }
	}
	Tcl_SetObjResult(interp, listPtr);
	return TCL_OK;
    }

    /*
     * Handle the imports for each of the patterns.
     */

    for (i = firstArg;  i < objc;  i++) {
	pattern = TclGetString(objv[i]);
	result = Tcl_Import(interp, NULL, pattern, allowOverwrite);
	if (result != TCL_OK) {
	    return result;
	}
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespaceInscopeCmd --
 *
 *	Invoked to implement the "namespace inscope" command that executes a
 *	script in the context of a particular namespace. This command is not
 *	expected to be used directly by programmers; calls to it are generated
 *	implicitly when programs use "namespace code" commands to register
 *	callback scripts. Handles the following syntax:
 *
 *	    namespace inscope name arg ?arg...?
 *
 *	The "namespace inscope" command is much like the "namespace eval"
 *	command except that it has lappend semantics and the namespace must
 *	already exist. It treats the first argument as a list, and appends any
 *	arguments after the first onto the end as proper list elements. For
 *	example,
 *
 *	    namespace inscope ::foo {a b} c d e
 *
 *	is equivalent to
 *
 *	    namespace eval ::foo [concat {a b} [list c d e]]
 *
 *	This lappend semantics is important because many callback scripts are
 *	actually prefixes.
 *
 * Results:
 *	Returns TCL_OK to indicate success, or TCL_ERROR to indicate failure.
 *
 * Side effects:
 *	Returns a result in the Tcl interpreter's result object.
 *
 *----------------------------------------------------------------------
 */

static int
NamespaceInscopeCmd(
    void *clientData,		/* Arbitrary value passed to cmd. */
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{

```

tcl9.1 9.1.0, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclNamesp.c`, function `NamespaceForgetCmd`, lines 3719–3769. Retained reference `native_oo_original_collision249-request-source-anchors-9.1.0-tclNamesp.c`.

```text
static int
NamespaceForgetCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    const char *pattern;
    Tcl_Size i;
    int result;

    if (objc < 1) {
	Tcl_WrongNumArgs(interp, 1, objv, "?pattern pattern...?");
	return TCL_ERROR;
    }

    for (i = 1;  i < objc;  i++) {
	pattern = TclGetString(objv[i]);
	result = Tcl_ForgetImport(interp, NULL, pattern);
	if (result != TCL_OK) {
	    return result;
	}
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespaceImportCmd --
 *
 *	Invoked to implement the "namespace import" command that imports
 *	commands into a namespace. Handles the following syntax:
 *
 *	    namespace import ?-force? ?pattern pattern...?
 *
 *	Each pattern is a namespace-qualified name like "foo::*", "a::b::x*",
 *	or "bar::p". That is, the pattern may include the special pattern
 *	matching characters recognized by the "string match" command, but only
 *	in the command name at the end of the qualified name; the special
 *	pattern characters may not appear in a namespace name. All of the
 *	commands that match the pattern and which are exported from their
 *	namespace are made accessible from the current namespace context. This
 *	is done by creating a new "imported command" in the current namespace
 *	that points to the real command in its original namespace; when the
 *	imported command is called, it invokes the real command.
 *
 *	If an imported command conflicts with an existing command, it is
 *	treated as an error. But if the "-force" option is included, then
 *	existing commands are overwritten by the imported commands.
 *

```

tcl9.1 9.1.0, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclProc.c`, function `Tcl_ProcObjCmd`, lines 145–239. Retained reference `native_compiled_local_formal_source272-9.1.0-tclProc.c`.

```text
int
Tcl_ProcObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Interp *iPtr = (Interp *) interp;
    Proc *procPtr;
    const char *procName;
    const char *simpleName, *procArgs, *procBody;
    Namespace *nsPtr, *altNsPtr, *cxtNsPtr;
    Tcl_Command cmd;

    if (objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "name args body");
	return TCL_ERROR;
    }

    /*
     * Determine the namespace where the procedure should reside. Unless the
     * command name includes namespace qualifiers, this will be the current
     * namespace.
     */

    procName = TclGetString(objv[1]);
    TclGetNamespaceForQualName(interp, procName, NULL, 0,
	    &nsPtr, &altNsPtr, &cxtNsPtr, &simpleName);

    if (nsPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"can't create procedure \"%s\": unknown namespace",
		procName));
	Tcl_SetErrorCode(interp, "TCL", "VALUE", "COMMAND", (char *)NULL);
	return TCL_ERROR;
    }
    if (simpleName == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"can't create procedure \"%s\": bad procedure name",
		procName));
	Tcl_SetErrorCode(interp, "TCL", "VALUE", "COMMAND", (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Create the data structure to represent the procedure.
     */

    if (TclCreateProc(interp, /*ignored nsPtr*/ NULL, simpleName, objv[2],
	    objv[3], &procPtr) != TCL_OK) {
	Tcl_AddErrorInfo(interp, "\n    (creating proc \"");
	Tcl_AddErrorInfo(interp, simpleName);
	Tcl_AddErrorInfo(interp, "\")");
	return TCL_ERROR;
    }

    cmd = TclNRCreateCommandInNs(interp, simpleName, (Tcl_Namespace *) nsPtr,
	TclObjInterpProc2, TclNRInterpProc, procPtr, TclProcDeleteProc);

    /*
     * Now initialize the new procedure's cmdPtr field. This will be used
     * later when the procedure is called to determine what namespace the
     * procedure will run in. This will be different than the current
     * namespace if the proc was renamed into a different namespace.
     */

    procPtr->cmdPtr = (Command *) cmd;

    /*
     * TIP #280: Remember the line the procedure body is starting on. In a
     * bytecode context we ask the engine to provide us with the necessary
     * information. This is for the initialization of the byte code compiler
     * when the body is used for the first time.
     *
     * This code is nearly identical to the #280 code in SetLambdaFromAny, see
     * this file. The differences are the different index of the body in the
     * line array of the context, and the lambda code requires some special
     * processing. Find a way to factor the common elements into a single
     * function.
     */

    if (iPtr->cmdFramePtr) {
	CmdFrame *contextPtr = (CmdFrame *)TclStackAlloc(interp, sizeof(CmdFrame));

	*contextPtr = *iPtr->cmdFramePtr;
	if (contextPtr->type == TCL_LOCATION_BC) {
	    /*
	     * Retrieve source information from the bytecode, if possible. If
	     * the information is retrieved successfully, context.type will be
	     * TCL_LOCATION_SOURCE and the reference held by
	     * context.data.eval.path will be counted.
	     */

	    TclGetSrcInfoForPc(contextPtr);
	} else if (contextPtr->type == TCL_LOCATION_SOURCE) {

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; whole pinned source SHA retained`, `/workspace/.proofs/native-providers/jimtcl/nshelper.tcl`, function `proc {namespace import}`, lines 60–102. Retained reference `native_jim_alias_introspection274-nshelper.tcl`.

```text

proc {namespace import} {args} {
	set current [uplevel 1 ::namespace canon]

	foreach pattern $args {
		foreach cmd [info commands [namespace canon $current $pattern]] {
			if {[namespace qualifiers $cmd] eq $current} {
				return -code error "import pattern \"$pattern\" tries to import from namespace \"$current\" into itself"
			}
			# What if this alias would create a loop?
			# follow the target alias chain to see if we are creating a loop
			set newcmd ${current}::[namespace tail $cmd]

			set alias $cmd
			while {[exists -alias $alias]} {
				set alias [info alias $alias]
				if {$alias eq $newcmd} {
					return -code error "import pattern \"$pattern\" would create a loop"
				}
			}

			alias $newcmd $cmd
		}
	}
}

# namespace-aware info commands: procs, channels, globals, locals, vars
proc {namespace info} {cmd {pattern *}} {
	set current [uplevel 1 ::namespace canon]
	# Now we may need to strip $pattern
	if {[string first :: $pattern] == 0} {
		set global 1
		set prefix ::
	} else {
		set global 0
		set clen [string length $current]
		incr clen 2
	}
	set fqp [namespace canon $current $pattern]
	switch -glob -- $cmd {
		co* - p* {
			if {$global} {
				set result [info $cmd $fqp]

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; whole pinned source SHA retained`, `/workspace/.proofs/native-providers/jimtcl/jim-namespace.c`, function `JimNamespaceCmd`, lines 332–336. Retained reference `native_jim_alias_introspection274-jim-namespace.c`.

```text
    Jim_PackageProvideCheck(interp, "namespace");
    Jim_RegisterSimpleCmd(interp, "namespace", "subcommand ?arg ...?", 1, -1, JimNamespaceCmd);
    Jim_RegisterSimpleCmd(interp, "variable", "name ?value?", 1, 2, JimVariableCmd);
    return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; whole pinned source SHA retained`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_Obj *Jim_NewObj`, lines 2234–2274. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
/* Return a new initialized object. */
Jim_Obj *Jim_NewObj(Jim_Interp *interp)
{
    Jim_Obj *objPtr;

    /* -- Check if there are objects in the free list -- */
    if (interp->freeList != NULL) {
        /* -- Unlink the object from the free list -- */
        objPtr = interp->freeList;
        interp->freeList = objPtr->nextObjPtr;
    }
    else {
        /* -- No ready to use objects: allocate a new one -- */
        objPtr = Jim_Alloc(sizeof(*objPtr));
    }

    /* Object is returned with refCount of 0. Every
     * kind of GC implemented should take care to avoid
     * scanning objects with refCount == 0. */
    objPtr->refCount = 0;
    objPtr->taint = interp->taint;
    /* All the other fields are left uninitialized to save time.
     * The caller will probably want to set them to the right
     * value anyway. */

    /* -- Put the object into the live list -- */
    objPtr->prevObjPtr = NULL;
    objPtr->nextObjPtr = interp->liveList;
    if (interp->liveList)
        interp->liveList->prevObjPtr = objPtr;
    interp->liveList = objPtr;

    return objPtr;
}

/* Free an object. Actually objects are never freed, but
 * just moved to the free objects list, where they will be
 * reused by Jim_NewObj(). */
void Jim_FreeObj(Jim_Interp *interp, Jim_Obj *objPtr)
{
    /* Check if the object was already freed, panic. */

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; whole pinned source SHA retained`, `/workspace/.proofs/native-providers/jimtcl/jim.h`, function `typedef struct Jim_ObjType`, lines 445–457. Retained reference `native_jim_core_alias_prefix273-jim.h`.

```text

typedef struct Jim_ObjType {
    const char *name; /* The name of the type. */
    Jim_FreeInternalRepProc *freeIntRepProc;
    Jim_DupInternalRepProc *dupIntRepProc;
    Jim_UpdateStringProc *updateStringProc;
    int flags;
} Jim_ObjType;

/* Jim_ObjType flags */
#define JIM_TYPE_NONE 0        /* No flags */
#define JIM_TYPE_REFERENCES 1    /* The object may contain references. */

```

tcl8.4 8.4.20, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclProc.c`, function `TclCreateProc`, lines 314–618. Retained reference `native_child_colon_publication-full-source-8.4.20-tclProc.c`.

```text
TclCreateProc(interp, nsPtr, procName, argsPtr, bodyPtr, procPtrPtr)
    Tcl_Interp *interp;         /* interpreter containing proc */
    Namespace *nsPtr;           /* namespace containing this proc */
    CONST char *procName;       /* unqualified name of this proc */
    Tcl_Obj *argsPtr;           /* description of arguments */
    Tcl_Obj *bodyPtr;           /* command body */
    Proc **procPtrPtr;          /* returns:  pointer to proc data */
{
    Interp *iPtr = (Interp*)interp;
    CONST char **argArray = NULL;

    register Proc *procPtr;
    int i, length, result, numArgs;
    CONST char *args, *bytes, *p;
    register CompiledLocal *localPtr = NULL;
    Tcl_Obj *defPtr;
    int precompiled = 0;

    if (bodyPtr->typePtr == &tclProcBodyType) {
        /*
         * Because the body is a TclProProcBody, the actual body is already
         * compiled, and it is not shared with anyone else, so it's OK not to
         * unshare it (as a matter of fact, it is bad to unshare it, because
         * there may be no source code).
         *
         * We don't create and initialize a Proc structure for the procedure;
         * rather, we use what is in the body object. Note that
         * we initialize its cmdPtr field below after we've created the command
         * for the procedure. We increment the ref count of the Proc struct
         * since the command (soon to be created) will be holding a reference
         * to it.
         */

        procPtr = (Proc *) bodyPtr->internalRep.otherValuePtr;
        procPtr->iPtr = iPtr;
        procPtr->refCount++;
        precompiled = 1;
    } else {
        /*
         * If the procedure's body object is shared because its string value is
         * identical to, e.g., the body of another procedure, we must create a
         * private copy for this procedure to use. Such sharing of procedure
         * bodies is rare but can cause problems. A procedure body is compiled
         * in a context that includes the number of compiler-allocated "slots"
         * for local variables. Each formal parameter is given a local variable
         * slot (the "procPtr->numCompiledLocals = numArgs" assignment
         * below). This means that the same code can not be shared by two
         * procedures that have a different number of arguments, even if their
         * bodies are identical. Note that we don't use Tcl_DuplicateObj since
         * we would not want any bytecode internal representation.
         */

        if (Tcl_IsShared(bodyPtr)) {
#ifdef TCL_TIP280
	    Tcl_Obj* sharedBodyPtr = bodyPtr;
#endif
            bytes = Tcl_GetStringFromObj(bodyPtr, &length);
            bodyPtr = Tcl_NewStringObj(bytes, length);
#ifdef TCL_TIP280
	    /*
	     * TIP #280.
	     * Ensure that the continuation line data for the original body is
	     * not lost and applies to the new body as well.
	     */

	    TclContinuationsCopy (bodyPtr, sharedBodyPtr);
#endif
        }

        /*
         * Create and initialize a Proc structure for the procedure. Note that
         * we initialize its cmdPtr field below after we've created the command
         * for the procedure. We increment the ref count of the procedure's
         * body object since there will be a reference to it in the Proc
         * structure.
         */

        Tcl_IncrRefCount(bodyPtr);

        procPtr = (Proc *) ckalloc(sizeof(Proc));
        procPtr->iPtr = iPtr;
        procPtr->refCount = 1;
        procPtr->bodyPtr = bodyPtr;
        procPtr->numArgs  = 0;	/* actual argument count is set below. */
        procPtr->numCompiledLocals = 0;
        procPtr->firstLocalPtr = NULL;
        procPtr->lastLocalPtr = NULL;
    }

    /*
     * Break up the argument list into argument specifiers, then process
     * each argument specifier.
     * If the body is precompiled, processing is limited to checking that
     * the the parsed argument is consistent with the one stored in the
     * Proc.
     * THIS FAILS IF THE ARG LIST OBJECT'S STRING REP CONTAINS NULLS.
     */

    args = Tcl_GetStringFromObj(argsPtr, &length);
    result = Tcl_SplitList(interp, args, &numArgs, &argArray);
    if (result != TCL_OK) {
        goto procError;
    }

    if (precompiled) {
        if (numArgs > procPtr->numArgs) {
            char buf[64 + TCL_INTEGER_SPACE + TCL_INTEGER_SPACE];
            sprintf(buf, "\": arg list contains %d entries, precompiled header expects %d",
                    numArgs, procPtr->numArgs);
            Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
                    "procedure \"", procName,
                    buf, (char *) NULL);
            goto procError;
        }
        localPtr = procPtr->firstLocalPtr;
    } else {
        procPtr->numArgs = numArgs;
        procPtr->numCompiledLocals = numArgs;
    }
    for (i = 0;  i < numArgs;  i++) {
        int fieldCount, nameLength, valueLength;
        CONST char **fieldValues;

        /*
         * Now divide the specifier up into name and default.
         */

        result = Tcl_SplitList(interp, argArray[i], &fieldCount,
                &fieldValues);
        if (result != TCL_OK) {
            goto procError;
        }
        if (fieldCount > 2) {
            ckfree((char *) fieldValues);
            Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
                    "too many fields in argument specifier \"",
                    argArray[i], "\"", (char *) NULL);
            goto procError;
        }
        if ((fieldCount == 0) || (*fieldValues[0] == 0)) {
            ckfree((char *) fieldValues);
            Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
                    "procedure \"", procName,
                    "\" has argument with no name", (char *) NULL);
            goto procError;
        }

        nameLength = strlen(fieldValues[0]);
        if (fieldCount == 2) {
            valueLength = strlen(fieldValues[1]);
        } else {
            valueLength = 0;
        }

        /*
         * Check that the formal parameter name is a scalar.
         */

        p = fieldValues[0];
        while (*p != '\0') {
            if (*p == '(') {
                CONST char *q = p;
                do {
		    q++;
		} while (*q != '\0');
		q--;
		if (*q == ')') { /* we have an array element */
		    Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
		            "procedure \"", procName,
		            "\" has formal parameter \"", fieldValues[0],
			    "\" that is an array element",
			    (char *) NULL);
		    ckfree((char *) fieldValues);
		    goto procError;
		}
	    } else if ((*p == ':') && (*(p+1) == ':')) {
		Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
		        "procedure \"", procName,
		        "\" has formal parameter \"", fieldValues[0],
			"\" that is not a simple name",
			(char *) NULL);
		ckfree((char *) fieldValues);
		goto procError;
	    }
	    p++;
	}

	if (precompiled) {
	    /*
	     * Compare the parsed argument with the stored one.
	     * For the flags, we and out VAR_UNDEFINED to support bridging
	     * precompiled <= 8.3 code in 8.4 where this is now used as an
	     * optimization indicator.	Yes, this is a hack. -- hobbs
	     */

	    if ((localPtr->nameLength != nameLength)
		    || (strcmp(localPtr->name, fieldValues[0]))
		    || (localPtr->frameIndex != i)
		    || ((localPtr->flags & ~VAR_UNDEFINED)
			    != (VAR_SCALAR | VAR_ARGUMENT))
		    || ((localPtr->defValuePtr == NULL)
			    && (fieldCount == 2))
		    || ((localPtr->defValuePtr != NULL)
			    && (fieldCount != 2))) {
		char buf[80 + TCL_INTEGER_SPACE];
		sprintf(buf, "\": formal parameter %d is inconsistent with precompiled body",
			i);
		Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
			"procedure \"", procName,
			buf, (char *) NULL);
		ckfree((char *) fieldValues);
		goto procError;
	    }

            /*
             * compare the default value if any
             */

            if (localPtr->defValuePtr != NULL) {
                int tmpLength;
                char *tmpPtr = Tcl_GetStringFromObj(localPtr->defValuePtr,
                        &tmpLength);
                if ((valueLength != tmpLength)
                        || (strncmp(fieldValues[1], tmpPtr,
                                (size_t) tmpLength))) {
                    Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
                            "procedure \"", procName,
                            "\": formal parameter \"",
                            fieldValues[0],
                            "\" has default value inconsistent with precompiled body",
                            (char *) NULL);
                    ckfree((char *) fieldValues);
                    goto procError;
                }
            }

            localPtr = localPtr->nextPtr;
        } else {
            /*
             * Allocate an entry in the runtime procedure frame's array of
             * local variables for the argument.
             */

            localPtr = (CompiledLocal *) ckalloc((unsigned)
                    (sizeof(CompiledLocal) - sizeof(localPtr->name)
                            + nameLength+1));
            if (procPtr->firstLocalPtr == NULL) {
                procPtr->firstLocalPtr = procPtr->lastLocalPtr = localPtr;
            } else {
                procPtr->lastLocalPtr->nextPtr = localPtr;
                procPtr->lastLocalPtr = localPtr;
            }
            localPtr->nextPtr = NULL;
            localPtr->nameLength = nameLength;
            localPtr->frameIndex = i;
            localPtr->flags = VAR_SCALAR | VAR_ARGUMENT;
            localPtr->resolveInfo = NULL;

            if (fieldCount == 2) {
                localPtr->defValuePtr =
		    Tcl_NewStringObj(fieldValues[1], valueLength);
                Tcl_IncrRefCount(localPtr->defValuePtr);
            } else {
                localPtr->defValuePtr = NULL;
            }
            memcpy(localPtr->name, fieldValues[0], nameLength + 1);
	}

        ckfree((char *) fieldValues);
    }

    /*
     * Now initialize the new procedure's cmdPtr field. This will be used
     * later when the procedure is called to determine what namespace the
     * procedure will run in. This will be different than the current
     * namespace if the proc was renamed into a different namespace.
     */

    *procPtrPtr = procPtr;
    ckfree((char *) argArray);
    return TCL_OK;

procError:
    if (precompiled) {
        procPtr->refCount--;
    } else {
        Tcl_DecrRefCount(bodyPtr);
        while (procPtr->firstLocalPtr != NULL) {
            localPtr = procPtr->firstLocalPtr;
            procPtr->firstLocalPtr = localPtr->nextPtr;

            defPtr = localPtr->defValuePtr;
            if (defPtr != NULL) {
                Tcl_DecrRefCount(defPtr);
            }

            ckfree((char *) localPtr);
        }
        ckfree((char *) procPtr);
    }
    if (argArray != NULL) {
	ckfree((char *) argArray);
    }
    return TCL_ERROR;
}

```

tcl8.5 8.5.19, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclProc.c`, function `TclCreateProc`, lines 378–671. Retained reference `native_compiled_local_formal_source272-8.5.19-tclProc.c`.

```text
TclCreateProc(
    Tcl_Interp *interp,		/* Interpreter containing proc. */
    Namespace *nsPtr,		/* Namespace containing this proc. */
    CONST char *procName,	/* Unqualified name of this proc. */
    Tcl_Obj *argsPtr,		/* Description of arguments. */
    Tcl_Obj *bodyPtr,		/* Command body. */
    Proc **procPtrPtr)		/* Returns: pointer to proc data. */
{
    Interp *iPtr = (Interp *) interp;
    CONST char **argArray = NULL;

    register Proc *procPtr;
    int i, length, result, numArgs;
    CONST char *args, *bytes, *p;
    register CompiledLocal *localPtr = NULL;
    Tcl_Obj *defPtr;
    int precompiled = 0;

    if (bodyPtr->typePtr == &tclProcBodyType) {
	/*
	 * Because the body is a TclProProcBody, the actual body is already
	 * compiled, and it is not shared with anyone else, so it's OK not to
	 * unshare it (as a matter of fact, it is bad to unshare it, because
	 * there may be no source code).
	 *
	 * We don't create and initialize a Proc structure for the procedure;
	 * rather, we use what is in the body object. We increment the ref
	 * count of the Proc struct since the command (soon to be created)
	 * will be holding a reference to it.
	 */

	procPtr = bodyPtr->internalRep.twoPtrValue.ptr1;
	procPtr->iPtr = iPtr;
	procPtr->refCount++;
	precompiled = 1;
    } else {
	/*
	 * If the procedure's body object is shared because its string value
	 * is identical to, e.g., the body of another procedure, we must
	 * create a private copy for this procedure to use. Such sharing of
	 * procedure bodies is rare but can cause problems. A procedure body
	 * is compiled in a context that includes the number of "slots"
	 * allocated by the compiler for local variables. There is a local
	 * variable slot for each formal parameter (the
	 * "procPtr->numCompiledLocals = numArgs" assignment below). This
	 * means that the same code can not be shared by two procedures that
	 * have a different number of arguments, even if their bodies are
	 * identical. Note that we don't use Tcl_DuplicateObj since we would
	 * not want any bytecode internal representation.
	 */

	if (Tcl_IsShared(bodyPtr)) {
	    Tcl_Obj* sharedBodyPtr = bodyPtr;

	    bytes = TclGetStringFromObj(bodyPtr, &length);
	    bodyPtr = Tcl_NewStringObj(bytes, length);

	    /*
	     * TIP #280.
	     * Ensure that the continuation line data for the original body is
	     * not lost and applies to the new body as well.
	     */

	    TclContinuationsCopy (bodyPtr, sharedBodyPtr);
	}

	/*
	 * Create and initialize a Proc structure for the procedure. We
	 * increment the ref count of the procedure's body object since there
	 * will be a reference to it in the Proc structure.
	 */

	Tcl_IncrRefCount(bodyPtr);

	procPtr = (Proc *) ckalloc(sizeof(Proc));
	procPtr->iPtr = iPtr;
	procPtr->refCount = 1;
	procPtr->bodyPtr = bodyPtr;
	procPtr->numArgs = 0;	/* Actual argument count is set below. */
	procPtr->numCompiledLocals = 0;
	procPtr->firstLocalPtr = NULL;
	procPtr->lastLocalPtr = NULL;
    }

    /*
     * Break up the argument list into argument specifiers, then process each
     * argument specifier. If the body is precompiled, processing is limited
     * to checking that the parsed argument is consistent with the one stored
     * in the Proc.
     *
     * THIS FAILS IF THE ARG LIST OBJECT'S STRING REP CONTAINS NULS.
     */

    args = TclGetStringFromObj(argsPtr, &length);
    result = Tcl_SplitList(interp, args, &numArgs, &argArray);
    if (result != TCL_OK) {
	goto procError;
    }

    if (precompiled) {
	if (numArgs > procPtr->numArgs) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "procedure \"%s\": arg list contains %d entries, "
		    "precompiled header expects %d", procName, numArgs,
		    procPtr->numArgs));
	    goto procError;
	}
	localPtr = procPtr->firstLocalPtr;
    } else {
	procPtr->numArgs = numArgs;
	procPtr->numCompiledLocals = numArgs;
    }

    for (i = 0; i < numArgs; i++) {
	int fieldCount, nameLength, valueLength;
	CONST char **fieldValues;

	/*
	 * Now divide the specifier up into name and default.
	 */

	result = Tcl_SplitList(interp, argArray[i], &fieldCount,
		&fieldValues);
	if (result != TCL_OK) {
	    goto procError;
	}
	if (fieldCount > 2) {
	    ckfree((char *) fieldValues);
	    Tcl_AppendResult(interp,
		    "too many fields in argument specifier \"",
		    argArray[i], "\"", NULL);
	    goto procError;
	}
	if ((fieldCount == 0) || (*fieldValues[0] == 0)) {
	    ckfree((char *) fieldValues);
	    Tcl_AppendResult(interp, "argument with no name", NULL);
	    goto procError;
	}

	nameLength = strlen(fieldValues[0]);
	if (fieldCount == 2) {
	    valueLength = strlen(fieldValues[1]);
	} else {
	    valueLength = 0;
	}

	/*
	 * Check that the formal parameter name is a scalar.
	 */

	p = fieldValues[0];
	while (*p != '\0') {
	    if (*p == '(') {
		CONST char *q = p;
		do {
		    q++;
		} while (*q != '\0');
		q--;
		if (*q == ')') {	/* We have an array element. */
		    Tcl_AppendResult(interp, "formal parameter \"",
			    fieldValues[0],
			    "\" is an array element", NULL);
		    ckfree((char *) fieldValues);
		    goto procError;
		}
	    } else if ((*p == ':') && (*(p+1) == ':')) {
		Tcl_AppendResult(interp, "formal parameter \"",
			fieldValues[0],
			"\" is not a simple name", NULL);
		ckfree((char *) fieldValues);
		goto procError;
	    }
	    p++;
	}

	if (precompiled) {
	    /*
	     * Compare the parsed argument with the stored one. Note that the
	     * only flag value that makes sense at this point is VAR_ARGUMENT
	     * (its value was kept the same as pre VarReform to simplify
	     * tbcload's processing of older byetcodes).
	     *
	     * The only other flag vlaue that is important to retrieve from
	     * precompiled procs is VAR_TEMPORARY (also unchanged). It is
	     * needed later when retrieving the variable names.
	     */

	    if ((localPtr->nameLength != nameLength)
		    || (strcmp(localPtr->name, fieldValues[0]))
		    || (localPtr->frameIndex != i)
		    || !(localPtr->flags & VAR_ARGUMENT)
		    || (localPtr->defValuePtr == NULL && fieldCount == 2)
		    || (localPtr->defValuePtr != NULL && fieldCount != 2)) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"procedure \"%s\": formal parameter %d is "
			"inconsistent with precompiled body", procName, i));
		ckfree((char *) fieldValues);
		goto procError;
	    }

	    /*
	     * Compare the default value if any.
	     */

	    if (localPtr->defValuePtr != NULL) {
		int tmpLength;
		char *tmpPtr = TclGetStringFromObj(localPtr->defValuePtr,
			&tmpLength);

		if ((valueLength != tmpLength) ||
			strncmp(fieldValues[1], tmpPtr, (size_t) tmpLength)) {
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "procedure \"%s\": formal parameter \"%s\" has "
			    "default value inconsistent with precompiled body",
			    procName, fieldValues[0]));
		    ckfree((char *) fieldValues);
		    goto procError;
		}
	    }
	    if ((i == numArgs - 1)
		    && (localPtr->nameLength == 4)
		    && (localPtr->name[0] == 'a')
		    && (strcmp(localPtr->name, "args") == 0)) {
		localPtr->flags |= VAR_IS_ARGS;
	    }

	    localPtr = localPtr->nextPtr;
	} else {
	    /*
	     * Allocate an entry in the runtime procedure frame's array of
	     * local variables for the argument.
	     */

	    localPtr = (CompiledLocal *) ckalloc((unsigned)
		    (sizeof(CompiledLocal) - sizeof(localPtr->name)
			    + nameLength + 1));
	    if (procPtr->firstLocalPtr == NULL) {
		procPtr->firstLocalPtr = procPtr->lastLocalPtr = localPtr;
	    } else {
		procPtr->lastLocalPtr->nextPtr = localPtr;
		procPtr->lastLocalPtr = localPtr;
	    }
	    localPtr->nextPtr = NULL;
	    localPtr->nameLength = nameLength;
	    localPtr->frameIndex = i;
	    localPtr->flags = VAR_ARGUMENT;
	    localPtr->resolveInfo = NULL;

	    if (fieldCount == 2) {
		localPtr->defValuePtr =
			Tcl_NewStringObj(fieldValues[1], valueLength);
		Tcl_IncrRefCount(localPtr->defValuePtr);
	    } else {
		localPtr->defValuePtr = NULL;
	    }
	    memcpy(localPtr->name, fieldValues[0], nameLength + 1);
	    if ((i == numArgs - 1)
		    && (localPtr->nameLength == 4)
		    && (localPtr->name[0] == 'a')
		    && (strcmp(localPtr->name, "args") == 0)) {
		localPtr->flags |= VAR_IS_ARGS;
	    }
	}

	ckfree((char *) fieldValues);
    }

    *procPtrPtr = procPtr;
    ckfree((char *) argArray);
    return TCL_OK;

  procError:
    if (precompiled) {
	procPtr->refCount--;
    } else {
	Tcl_DecrRefCount(bodyPtr);
	while (procPtr->firstLocalPtr != NULL) {
	    localPtr = procPtr->firstLocalPtr;
	    procPtr->firstLocalPtr = localPtr->nextPtr;

	    defPtr = localPtr->defValuePtr;
	    if (defPtr != NULL) {
		Tcl_DecrRefCount(defPtr);
	    }

	    ckfree((char *) localPtr);
	}
	ckfree((char *) procPtr);
    }
    if (argArray != NULL) {
	ckfree((char *) argArray);
    }
    return TCL_ERROR;
}

```

tcl8.6 8.6.18, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclProc.c`, function `TclCreateProc`, lines 363–652. Retained reference `native_compiled_local_formal_source272-8.6.18-tclProc.c`.

```text
TclCreateProc(
    Tcl_Interp *interp,		/* Interpreter containing proc. */
    Namespace *nsPtr,		/* Namespace containing this proc. */
    const char *procName,	/* Unqualified name of this proc. */
    Tcl_Obj *argsPtr,		/* Description of arguments. */
    Tcl_Obj *bodyPtr,		/* Command body. */
    Proc **procPtrPtr)		/* Returns: pointer to proc data. */
{
    Interp *iPtr = (Interp *) interp;

    Proc *procPtr;
    int i, result, numArgs;
    CompiledLocal *localPtr = NULL;
    Tcl_Obj **argArray;
    int precompiled = 0;

    if (bodyPtr->typePtr == &tclProcBodyType) {
	/*
	 * Because the body is a TclProProcBody, the actual body is already
	 * compiled, and it is not shared with anyone else, so it's OK not to
	 * unshare it (as a matter of fact, it is bad to unshare it, because
	 * there may be no source code).
	 *
	 * We don't create and initialize a Proc structure for the procedure;
	 * rather, we use what is in the body object. We increment the ref
	 * count of the Proc struct since the command (soon to be created)
	 * will be holding a reference to it.
	 */

	procPtr = bodyPtr->internalRep.twoPtrValue.ptr1;
	procPtr->iPtr = iPtr;
	procPtr->refCount++;
	precompiled = 1;
    } else {
	/*
	 * If the procedure's body object is shared because its string value
	 * is identical to, e.g., the body of another procedure, we must
	 * create a private copy for this procedure to use. Such sharing of
	 * procedure bodies is rare but can cause problems. A procedure body
	 * is compiled in a context that includes the number of "slots"
	 * allocated by the compiler for local variables. There is a local
	 * variable slot for each formal parameter (the
	 * "procPtr->numCompiledLocals = numArgs" assignment below). This
	 * means that the same code can not be shared by two procedures that
	 * have a different number of arguments, even if their bodies are
	 * identical. Note that we don't use Tcl_DuplicateObj since we would
	 * not want any bytecode internal representation.
	 */

	if (Tcl_IsShared(bodyPtr)) {
	    const char *bytes;
	    int length;
	    Tcl_Obj *sharedBodyPtr = bodyPtr;

	    bytes = TclGetStringFromObj(bodyPtr, &length);
	    bodyPtr = Tcl_NewStringObj(bytes, length);

	    /*
	     * TIP #280.
	     * Ensure that the continuation line data for the original body is
	     * not lost and applies to the new body as well.
	     */

	    TclContinuationsCopy(bodyPtr, sharedBodyPtr);
	}

	/*
	 * Create and initialize a Proc structure for the procedure. We
	 * increment the ref count of the procedure's body object since there
	 * will be a reference to it in the Proc structure.
	 */

	Tcl_IncrRefCount(bodyPtr);

	procPtr = (Proc *)ckalloc(sizeof(Proc));
	procPtr->iPtr = iPtr;
	procPtr->refCount = 1;
	procPtr->bodyPtr = bodyPtr;
	procPtr->numArgs = 0;	/* Actual argument count is set below. */
	procPtr->numCompiledLocals = 0;
	procPtr->firstLocalPtr = NULL;
	procPtr->lastLocalPtr = NULL;
    }

    /*
     * Break up the argument list into argument specifiers, then process each
     * argument specifier. If the body is precompiled, processing is limited
     * to checking that the parsed argument is consistent with the one stored
     * in the Proc.
     */

    result = TclListObjGetElements(interp, argsPtr, &numArgs, &argArray);
    if (result != TCL_OK) {
	goto procError;
    }

    if (precompiled) {
	if (numArgs > procPtr->numArgs) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "procedure \"%s\": arg list contains %d entries, "
		    "precompiled header expects %d", procName, numArgs,
		    procPtr->numArgs));
	    Tcl_SetErrorCode(interp, "TCL", "OPERATION", "PROC",
		    "BYTECODELIES", (char *)NULL);
	    goto procError;
	}
	localPtr = procPtr->firstLocalPtr;
    } else {
	procPtr->numArgs = numArgs;
	procPtr->numCompiledLocals = numArgs;
    }

    for (i = 0; i < numArgs; i++) {
	const char *argname, *p, *last;
	int fieldCount, nameLength;
	Tcl_Obj **fieldValues;

	/*
	 * Now divide the specifier up into name and default.
	 */

	result = TclListObjGetElements(interp, argArray[i], &fieldCount,
		&fieldValues);
	if (result != TCL_OK) {
	    goto procError;
	}
	if (fieldCount > 2) {
	    Tcl_Obj *errorObj = Tcl_NewStringObj(
		"too many fields in argument specifier \"", -1);
	    Tcl_AppendObjToObj(errorObj, argArray[i]);
	    Tcl_AppendToObj(errorObj, "\"", -1);
	    Tcl_SetObjResult(interp, errorObj);
	    Tcl_SetErrorCode(interp, "TCL", "OPERATION", "PROC",
		    "FORMALARGUMENTFORMAT", (char *)NULL);
	    goto procError;
	}
	if ((fieldCount == 0) || (Tcl_GetCharLength(fieldValues[0]) == 0)) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "argument with no name", -1));
	    Tcl_SetErrorCode(interp, "TCL", "OPERATION", "PROC",
		    "FORMALARGUMENTFORMAT", (char *)NULL);
	    goto procError;
	}

	/*
	 * Check that the formal parameter name is a scalar.
	 */

	p = argname = TclGetStringFromObj(fieldValues[0], &nameLength);
	last = argname + nameLength;
	while (p < last) {
	    if (*p == '(') {
		if (last[-1] == ')') { /* We have an array element. */
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "formal parameter \"%s\" is an array element",
			    TclGetString(fieldValues[0])));
		    Tcl_SetErrorCode(interp, "TCL", "OPERATION", "PROC",
			    "FORMALARGUMENTFORMAT", (char *)NULL);
		    goto procError;
		}
	    } else if (p[0] == ':' && p[1] == ':') {
		Tcl_Obj *errorObj = Tcl_NewStringObj(
		    "formal parameter \"", -1);
		Tcl_AppendObjToObj(errorObj, fieldValues[0]);
		Tcl_AppendToObj(errorObj, "\" is not a simple name", -1);
		Tcl_SetObjResult(interp, errorObj);
		Tcl_SetErrorCode(interp, "TCL", "OPERATION", "PROC",
			"FORMALARGUMENTFORMAT", (char *)NULL);
		goto procError;
	    }
	    p++;
	}

	if (precompiled) {
	    /*
	     * Compare the parsed argument with the stored one. Note that the
	     * only flag value that makes sense at this point is VAR_ARGUMENT
	     * (its value was kept the same as pre VarReform to simplify
	     * tbcload's processing of older byetcodes).
	     *
	     * The only other flag value that is important to retrieve from
	     * precompiled procs is VAR_TEMPORARY (also unchanged). It is
	     * needed later when retrieving the variable names.
	     */

	    if ((localPtr->nameLength != nameLength)
		    || (memcmp(localPtr->name, argname, nameLength) != 0)
		    || (localPtr->frameIndex != i)
		    || !(localPtr->flags & VAR_ARGUMENT)
		    || (localPtr->defValuePtr == NULL && fieldCount == 2)
		    || (localPtr->defValuePtr != NULL && fieldCount != 2)) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"procedure \"%s\": formal parameter %d is "
			"inconsistent with precompiled body", procName, i));
		Tcl_SetErrorCode(interp, "TCL", "OPERATION", "PROC",
			"BYTECODELIES", (char *)NULL);
		goto procError;
	    }

	    /*
	     * Compare the default value if any.
	     */

	    if (localPtr->defValuePtr != NULL) {
		int tmpLength, valueLength;
		const char *tmpPtr = TclGetStringFromObj(localPtr->defValuePtr,
			&tmpLength);
		const char *value = TclGetStringFromObj(fieldValues[1],
			&valueLength);

		if ((valueLength != tmpLength)
		     || memcmp(value, tmpPtr, tmpLength) != 0
		) {
		    Tcl_Obj *errorObj = Tcl_ObjPrintf(
			    "procedure \"%s\": formal parameter \"", procName);
		    Tcl_AppendObjToObj(errorObj, fieldValues[0]);
		    Tcl_AppendToObj(errorObj, "\" has "
			"default value inconsistent with precompiled body", -1);
		    Tcl_SetObjResult(interp, errorObj);
		    Tcl_SetErrorCode(interp, "TCL", "OPERATION", "PROC",
			    "BYTECODELIES", (char *)NULL);
		    goto procError;
		}
	    }
	    if ((i == numArgs - 1)
		    && (localPtr->nameLength == 4)
		    && (localPtr->name[0] == 'a')
		    && (strcmp(localPtr->name, "args") == 0)) {
		localPtr->flags |= VAR_IS_ARGS;
	    }

	    localPtr = localPtr->nextPtr;
	} else {
	    /*
	     * Allocate an entry in the runtime procedure frame's array of
	     * local variables for the argument.
	     */

	    localPtr = (CompiledLocal *)ckalloc(
		    TclOffset(CompiledLocal, name) + 1U + fieldValues[0]->length);
	    if (procPtr->firstLocalPtr == NULL) {
		procPtr->firstLocalPtr = procPtr->lastLocalPtr = localPtr;
	    } else {
		procPtr->lastLocalPtr->nextPtr = localPtr;
		procPtr->lastLocalPtr = localPtr;
	    }
	    localPtr->nextPtr = NULL;
	    localPtr->nameLength = nameLength;
	    localPtr->frameIndex = i;
	    localPtr->flags = VAR_ARGUMENT;
	    localPtr->resolveInfo = NULL;

	    if (fieldCount == 2) {
		localPtr->defValuePtr = fieldValues[1];
		Tcl_IncrRefCount(localPtr->defValuePtr);
	    } else {
		localPtr->defValuePtr = NULL;
	    }
	    memcpy(localPtr->name, argname, fieldValues[0]->length + 1);
	    if ((i == numArgs - 1)
		    && (localPtr->nameLength == 4)
		    && (localPtr->name[0] == 'a')
		    && (memcmp(localPtr->name, "args", 4) == 0)) {
		localPtr->flags |= VAR_IS_ARGS;
	    }
	}
    }

    *procPtrPtr = procPtr;
    return TCL_OK;

  procError:
    if (precompiled) {
	procPtr->refCount--;
    } else {
	Tcl_DecrRefCount(bodyPtr);
	while (procPtr->firstLocalPtr != NULL) {
	    localPtr = procPtr->firstLocalPtr;
	    procPtr->firstLocalPtr = localPtr->nextPtr;

	    if (localPtr->defValuePtr != NULL) {
		Tcl_DecrRefCount(localPtr->defValuePtr);
	    }

	    ckfree(localPtr);
	}
	ckfree(procPtr);
    }
    return TCL_ERROR;
}

```

tcl9.0 9.0.4, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclProc.c`, function `TclCreateProc`, lines 404–701. Retained reference `native_compiled_local_formal_source272-9.0.4-tclProc.c`.

```text
TclCreateProc(
    Tcl_Interp *interp,		/* Interpreter containing proc. */
    TCL_UNUSED(Namespace *) /*nsPtr*/,
    const char *procName,	/* Unqualified name of this proc. */
    Tcl_Obj *argsPtr,		/* Description of arguments. */
    Tcl_Obj *bodyPtr,		/* Command body. */
    Proc **procPtrPtr)		/* Returns: pointer to proc data. */
{
    Interp *iPtr = (Interp *) interp;

    Proc *procPtr = NULL;
    Tcl_Size i, numArgs;
    CompiledLocal *localPtr = NULL;
    Tcl_Obj **argArray;
    int precompiled = 0, result;
    const char *errorCode = NULL;

    ProcGetInternalRep(bodyPtr, procPtr);
    if (procPtr != NULL) {
	/*
	 * Because the body is a TclProProcBody, the actual body is already
	 * compiled, and it is not shared with anyone else, so it's OK not to
	 * unshare it (as a matter of fact, it is bad to unshare it, because
	 * there may be no source code).
	 *
	 * We don't create and initialize a Proc structure for the procedure;
	 * rather, we use what is in the body object. We increment the ref
	 * count of the Proc struct since the command (soon to be created)
	 * will be holding a reference to it.
	 */

	procPtr->iPtr = iPtr;
	procPtr->refCount++;
	precompiled = 1;
    } else {
	/*
	 * If the procedure's body object is shared because its string value
	 * is identical to, e.g., the body of another procedure, we must
	 * create a private copy for this procedure to use. Such sharing of
	 * procedure bodies is rare but can cause problems. A procedure body
	 * is compiled in a context that includes the number of "slots"
	 * allocated by the compiler for local variables. There is a local
	 * variable slot for each formal parameter (the
	 * "procPtr->numCompiledLocals = numArgs" assignment below). This
	 * means that the same code can not be shared by two procedures that
	 * have a different number of arguments, even if their bodies are
	 * identical. Note that we don't use Tcl_DuplicateObj since we would
	 * not want any bytecode internal representation.
	 */

	if (Tcl_IsShared(bodyPtr)) {
	    const char *bytes;
	    Tcl_Size length;
	    Tcl_Obj *sharedBodyPtr = bodyPtr;

	    bytes = TclGetStringFromObj(bodyPtr, &length);
	    bodyPtr = Tcl_NewStringObj(bytes, length);

	    /*
	     * TIP #280.
	     * Ensure that the continuation line data for the original body is
	     * not lost and applies to the new body as well.
	     */

	    TclContinuationsCopy(bodyPtr, sharedBodyPtr);
	}

	/*
	 * Create and initialize a Proc structure for the procedure. We
	 * increment the ref count of the procedure's body object since there
	 * will be a reference to it in the Proc structure.
	 */

	Tcl_IncrRefCount(bodyPtr);

	procPtr = (Proc *)Tcl_Alloc(sizeof(Proc));
	procPtr->iPtr = iPtr;
	procPtr->refCount = 1;
	procPtr->bodyPtr = bodyPtr;
	procPtr->numArgs = 0;	/* Actual argument count is set below. */
	procPtr->numCompiledLocals = 0;
	procPtr->firstLocalPtr = NULL;
	procPtr->lastLocalPtr = NULL;
    }

    /*
     * Break up the argument list into argument specifiers, then process each
     * argument specifier. If the body is precompiled, processing is limited
     * to checking that the parsed argument is consistent with the one stored
     * in the Proc.
     */

    result = TclListObjGetElements(interp, argsPtr, &numArgs, &argArray);
    if (result != TCL_OK) {
	goto procError;
    }

    if (precompiled) {
	if (numArgs > procPtr->numArgs) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "procedure \"%s\": arg list contains %" TCL_SIZE_MODIFIER "d entries, "
		    "precompiled header expects %" TCL_SIZE_MODIFIER "d", procName, numArgs,
		    procPtr->numArgs));
	    errorCode = "BYTECODELIES";
	    goto procError;
	}
	localPtr = procPtr->firstLocalPtr;
    } else {
	procPtr->numArgs = numArgs;
	procPtr->numCompiledLocals = numArgs;
    }

    for (i = 0; i < numArgs; i++) {
	const char *argname, *argnamei, *argnamelast;
	Tcl_Size fieldCount, nameLength;
	Tcl_Obj **fieldValues;

	/*
	 * Now divide the specifier up into name and default.
	 */

	result = TclListObjGetElements(interp, argArray[i], &fieldCount,
		&fieldValues);
	if (result != TCL_OK) {
	    goto procError;
	}
	if (fieldCount > 2) {
	    Tcl_Obj *errorObj = Tcl_NewStringObj(
		"too many fields in argument specifier \"", -1);
	    Tcl_AppendObjToObj(errorObj, argArray[i]);
	    Tcl_AppendToObj(errorObj, "\"", -1);
	    Tcl_SetObjResult(interp, errorObj);
	    errorCode = "FORMALARGUMENTFORMAT";
	    goto procError;
	}
	if ((fieldCount == 0) || (Tcl_GetCharLength(fieldValues[0]) == 0)) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "argument with no name", -1));
	    errorCode = "FORMALARGUMENTFORMAT";
	    goto procError;
	}

	argname = TclGetStringFromObj(fieldValues[0], &nameLength);

	/*
	 * Check that the formal parameter name is a scalar.
	 */

	argnamei = argname;
	argnamelast = (nameLength > 0) ? (argname + nameLength - 1) : argname;
	while (argnamei < argnamelast) {
	    if (*argnamei == '(') {
		if (*argnamelast == ')') { /* We have an array element. */
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "formal parameter \"%s\" is an array element",
			    TclGetString(fieldValues[0])));
		    errorCode = "FORMALARGUMENTFORMAT";
		    goto procError;
		}
	    } else if (argnamei[0] == ':' && argnamei[1] == ':') {
		Tcl_Obj *errorObj = Tcl_NewStringObj(
			"formal parameter \"", -1);
		Tcl_AppendObjToObj(errorObj, fieldValues[0]);
		Tcl_AppendToObj(errorObj, "\" is not a simple name", -1);
		Tcl_SetObjResult(interp, errorObj);
		errorCode = "FORMALARGUMENTFORMAT";
		goto procError;
	    }
	    argnamei++;
	}

	if (precompiled) {
	    /*
	     * Compare the parsed argument with the stored one. Note that the
	     * only flag value that makes sense at this point is VAR_ARGUMENT
	     * (its value was kept the same as pre VarReform to simplify
	     * tbcload's processing of older byetcodes).
	     *
	     * The only other flag value that is important to retrieve from
	     * precompiled procs is VAR_TEMPORARY (also unchanged). It is
	     * needed later when retrieving the variable names.
	     */

	    if ((localPtr->nameLength != nameLength)
		    || (memcmp(localPtr->name, argname, nameLength) != 0)
		    || (localPtr->frameIndex != i)
		    || !(localPtr->flags & VAR_ARGUMENT)
		    || (localPtr->defValuePtr == NULL && fieldCount == 2)
		    || (localPtr->defValuePtr != NULL && fieldCount != 2)) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"procedure \"%s\": formal parameter %" TCL_SIZE_MODIFIER "d is "
			"inconsistent with precompiled body", procName, i));
		errorCode = "BYTECODELIES";
		goto procError;
	    }

	    /*
	     * Compare the default value if any.
	     */

	    if (localPtr->defValuePtr != NULL) {
		Tcl_Size tmpLength, valueLength;
		const char *tmpPtr = TclGetStringFromObj(localPtr->defValuePtr, &tmpLength);
		const char *value = TclGetStringFromObj(fieldValues[1], &valueLength);

		if ((valueLength != tmpLength)
			|| memcmp(value, tmpPtr, tmpLength) != 0) {
		    Tcl_Obj *errorObj = Tcl_ObjPrintf(
			    "procedure \"%s\": formal parameter \"", procName);
		    Tcl_AppendObjToObj(errorObj, fieldValues[0]);
		    Tcl_AppendToObj(errorObj, "\" has "
			"default value inconsistent with precompiled body", -1);
		    Tcl_SetObjResult(interp, errorObj);
		    errorCode = "BYTECODELIES";
		    goto procError;
		}
	    }
	    if ((i == numArgs - 1)
		    && (localPtr->nameLength == 4)
		    && (localPtr->name[0] == 'a')
		    && (strcmp(localPtr->name, "args") == 0)) {
		localPtr->flags |= VAR_IS_ARGS;
	    }

	    localPtr = localPtr->nextPtr;
	} else {
	    /*
	     * Allocate an entry in the runtime procedure frame's array of
	     * local variables for the argument.
	     */

	    localPtr = (CompiledLocal *)Tcl_AttemptAlloc(
		    offsetof(CompiledLocal, name) + 1U + fieldValues[0]->length);
	    if (!localPtr) {
		/* Don't set the interp result here. Since a malloc just failed,
		 * first clean up some memory before doing that */
		errorCode = TOOMANYARGS;
		goto procError;
	    }
	    if (procPtr->firstLocalPtr == NULL) {
		procPtr->firstLocalPtr = procPtr->lastLocalPtr = localPtr;
	    } else {
		procPtr->lastLocalPtr->nextPtr = localPtr;
		procPtr->lastLocalPtr = localPtr;
	    }
	    localPtr->nextPtr = NULL;
	    localPtr->nameLength = nameLength;
	    localPtr->frameIndex = i;
	    localPtr->flags = VAR_ARGUMENT;
	    localPtr->resolveInfo = NULL;

	    if (fieldCount == 2) {
		localPtr->defValuePtr = fieldValues[1];
		Tcl_IncrRefCount(localPtr->defValuePtr);
	    } else {
		localPtr->defValuePtr = NULL;
	    }
	    memcpy(localPtr->name, argname, fieldValues[0]->length + 1);
	    if ((i == numArgs - 1)
		    && (localPtr->nameLength == 4)
		    && (localPtr->name[0] == 'a')
		    && (memcmp(localPtr->name, "args", 4) == 0)) {
		localPtr->flags |= VAR_IS_ARGS;
	    }
	}
    }

    *procPtrPtr = procPtr;
    return TCL_OK;

  procError:
    if (precompiled) {
	procPtr->refCount--;
    } else {
	Tcl_DecrRefCount(bodyPtr);
	while (procPtr->firstLocalPtr != NULL) {
	    localPtr = procPtr->firstLocalPtr;
	    procPtr->firstLocalPtr = localPtr->nextPtr;

	    if (localPtr->defValuePtr != NULL) {
		Tcl_DecrRefCount(localPtr->defValuePtr);
	    }

	    Tcl_Free(localPtr);
	}
	Tcl_Free(procPtr);
    }
    if (errorCode) {
	if (errorCode == TOOMANYARGS) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "procedure \"%s\": arg list contains too many (%"
		    TCL_SIZE_MODIFIER "d) entries", procName, numArgs));
	}
	Tcl_SetErrorCode(interp, "TCL", "OPERATION", "PROC",
		errorCode, (char *)NULL);
    }
    return TCL_ERROR;
}

```

tcl9.1 9.1.0, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclProc.c`, function `TclCreateProc`, lines 392–685. Retained reference `native_compiled_local_formal_source272-9.1.0-tclProc.c`.

```text
TclCreateProc(
    Tcl_Interp *interp,		/* Interpreter containing proc. */
    TCL_UNUSED(Namespace *) /*nsPtr*/,
    const char *procName,	/* Unqualified name of this proc. */
    Tcl_Obj *argsPtr,		/* Description of arguments. */
    Tcl_Obj *bodyPtr,		/* Command body. */
    Proc **procPtrPtr)		/* Returns: pointer to proc data. */
{
    Interp *iPtr = (Interp *) interp;

    Proc *procPtr = NULL;
    Tcl_Size i, numArgs;
    CompiledLocal *localPtr = NULL;
    Tcl_Obj **argArray;
    int precompiled = 0, result;
    const char *errorCode = NULL;

    ProcGetInternalRep(bodyPtr, procPtr);
    if (procPtr != NULL) {
	/*
	 * Because the body is a TclProProcBody, the actual body is already
	 * compiled, and it is not shared with anyone else, so it's OK not to
	 * unshare it (as a matter of fact, it is bad to unshare it, because
	 * there may be no source code).
	 *
	 * We don't create and initialize a Proc structure for the procedure;
	 * rather, we use what is in the body object. We increment the ref
	 * count of the Proc struct since the command (soon to be created)
	 * will be holding a reference to it.
	 */

	procPtr->iPtr = iPtr;
	procPtr->refCount++;
	precompiled = 1;
    } else {
	/*
	 * If the procedure's body object is shared because its string value
	 * is identical to, e.g., the body of another procedure, we must
	 * create a private copy for this procedure to use. Such sharing of
	 * procedure bodies is rare but can cause problems. A procedure body
	 * is compiled in a context that includes the number of "slots"
	 * allocated by the compiler for local variables. There is a local
	 * variable slot for each formal parameter (the
	 * "procPtr->numCompiledLocals = numArgs" assignment below). This
	 * means that the same code can not be shared by two procedures that
	 * have a different number of arguments, even if their bodies are
	 * identical. Note that we don't use Tcl_DuplicateObj since we would
	 * not want any bytecode internal representation.
	 */

	if (Tcl_IsShared(bodyPtr)) {
	    const char *bytes;
	    Tcl_Size length;
	    Tcl_Obj *sharedBodyPtr = bodyPtr;

	    bytes = TclGetStringFromObj(bodyPtr, &length);
	    bodyPtr = Tcl_NewStringObj(bytes, length);

	    /*
	     * TIP #280.
	     * Ensure that the continuation line data for the original body is
	     * not lost and applies to the new body as well.
	     */

	    TclContinuationsCopy(bodyPtr, sharedBodyPtr);
	}

	/*
	 * Create and initialize a Proc structure for the procedure. We
	 * increment the ref count of the procedure's body object since there
	 * will be a reference to it in the Proc structure.
	 */

	Tcl_IncrRefCount(bodyPtr);

	procPtr = (Proc *)Tcl_Alloc(sizeof(Proc));
	procPtr->iPtr = iPtr;
	procPtr->refCount = 1;
	procPtr->cmdPtr = NULL;
	procPtr->bodyPtr = bodyPtr;
	procPtr->numArgs = 0;	/* Actual argument count is set below. */
	procPtr->numCompiledLocals = 0;
	procPtr->firstLocalPtr = NULL;
	procPtr->lastLocalPtr = NULL;
	procPtr->flags = 0;
    }

    /*
     * Break up the argument list into argument specifiers, then process each
     * argument specifier. If the body is precompiled, processing is limited
     * to checking that the parsed argument is consistent with the one stored
     * in the Proc.
     */

    result = TclListObjGetElements(interp, argsPtr, &numArgs, &argArray);
    if (result != TCL_OK) {
	goto procError;
    }

    if (precompiled) {
	if (numArgs > procPtr->numArgs) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "procedure \"%s\": arg list contains %" TCL_SIZE_MODIFIER "d entries, "
		    "precompiled header expects %" TCL_SIZE_MODIFIER "d", procName, numArgs,
		    procPtr->numArgs));
	    errorCode = "BYTECODELIES";
	    goto procError;
	}
	localPtr = procPtr->firstLocalPtr;
    } else {
	procPtr->numArgs = numArgs;
	procPtr->numCompiledLocals = numArgs;
    }

    for (i = 0; i < numArgs; i++) {
	const char *argname, *argnamei, *argnamelast;
	Tcl_Size fieldCount, nameLength;
	Tcl_Obj **fieldValues;

	/*
	 * Now divide the specifier up into name and default.
	 */

	result = TclListObjGetElements(interp, argArray[i], &fieldCount,
		&fieldValues);
	if (result != TCL_OK) {
	    goto procError;
	}
	if (fieldCount > 2) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "too many fields in argument specifier \"%s\"",
		    TclGetString(argArray[i])));
	    errorCode = "FORMALARGUMENTFORMAT";
	    goto procError;
	}
	if ((fieldCount == 0) || (Tcl_GetCharLength(fieldValues[0]) == 0)) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "argument with no name", -1));
	    errorCode = "FORMALARGUMENTFORMAT";
	    goto procError;
	}

	argname = TclGetStringFromObj(fieldValues[0], &nameLength);

	/*
	 * Check that the formal parameter name is a scalar.
	 */

	argnamei = argname;
	argnamelast = (nameLength > 0) ? (argname + nameLength - 1) : argname;
	while (argnamei < argnamelast) {
	    if (*argnamei == '(') {
		if (*argnamelast == ')') { /* We have an array element. */
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "formal parameter \"%s\" is an array element",
			    TclGetString(fieldValues[0])));
		    errorCode = "FORMALARGUMENTFORMAT";
		    goto procError;
		}
	    } else if (argnamei[0] == ':' && argnamei[1] == ':') {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"formal parameter \"%s\" is not a simple name",
			TclGetString(fieldValues[0])));
		errorCode = "FORMALARGUMENTFORMAT";
		goto procError;
	    }
	    argnamei++;
	}

	if (precompiled) {
	    /*
	     * Compare the parsed argument with the stored one. Note that the
	     * only flag value that makes sense at this point is VAR_ARGUMENT
	     * (its value was kept the same as pre VarReform to simplify
	     * tbcload's processing of older byetcodes).
	     *
	     * The only other flag value that is important to retrieve from
	     * precompiled procs is VAR_TEMPORARY (also unchanged). It is
	     * needed later when retrieving the variable names.
	     */

	    if ((localPtr->nameLength != nameLength)
		    || (memcmp(localPtr->name, argname, nameLength) != 0)
		    || (localPtr->frameIndex != i)
		    || !(localPtr->flags & VAR_ARGUMENT)
		    || (localPtr->defValuePtr == NULL && fieldCount == 2)
		    || (localPtr->defValuePtr != NULL && fieldCount != 2)) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"procedure \"%s\": formal parameter %" TCL_SIZE_MODIFIER "d is "
			"inconsistent with precompiled body", procName, i));
		errorCode = "BYTECODELIES";
		goto procError;
	    }

	    /*
	     * Compare the default value if any.
	     */

	    if (localPtr->defValuePtr != NULL) {
		Tcl_Size tmpLength, valueLength;
		const char *tmpPtr = TclGetStringFromObj(localPtr->defValuePtr, &tmpLength);
		const char *value = TclGetStringFromObj(fieldValues[1], &valueLength);

		if ((valueLength != tmpLength)
			|| memcmp(value, tmpPtr, tmpLength) != 0) {
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "procedure \"%s\": formal parameter \"%s\" has "
			    "default value inconsistent with precompiled body",
			    procName, TclGetString(fieldValues[0])));
		    errorCode = "BYTECODELIES";
		    goto procError;
		}
	    }
	    if ((i == numArgs - 1)
		    && (localPtr->nameLength == 4)
		    && (localPtr->name[0] == 'a')
		    && (strcmp(localPtr->name, "args") == 0)) {
		localPtr->flags |= VAR_IS_ARGS;
	    }

	    localPtr = localPtr->nextPtr;
	} else {
	    /*
	     * Allocate an entry in the runtime procedure frame's array of
	     * local variables for the argument.
	     */

	    localPtr = (CompiledLocal *)Tcl_AttemptAlloc(
		    offsetof(CompiledLocal, name) + 1U + fieldValues[0]->length);
	    if (!localPtr) {
		/* Don't set the interp result here. Since a malloc just failed,
		 * first clean up some memory before doing that */
		errorCode = TOOMANYARGS;
		goto procError;
	    }
	    if (procPtr->firstLocalPtr == NULL) {
		procPtr->firstLocalPtr = procPtr->lastLocalPtr = localPtr;
	    } else {
		procPtr->lastLocalPtr->nextPtr = localPtr;
		procPtr->lastLocalPtr = localPtr;
	    }
	    localPtr->nextPtr = NULL;
	    localPtr->nameLength = nameLength;
	    localPtr->frameIndex = i;
	    localPtr->flags = VAR_ARGUMENT;
	    localPtr->resolveInfo = NULL;

	    if (fieldCount == 2) {
		localPtr->defValuePtr = fieldValues[1];
		Tcl_IncrRefCount(localPtr->defValuePtr);
	    } else {
		localPtr->defValuePtr = NULL;
	    }
	    memcpy(localPtr->name, argname, fieldValues[0]->length + 1);
	    if ((i == numArgs - 1)
		    && (localPtr->nameLength == 4)
		    && (localPtr->name[0] == 'a')
		    && (memcmp(localPtr->name, "args", 4) == 0)) {
		localPtr->flags |= VAR_IS_ARGS;
	    }
	}
    }

    *procPtrPtr = procPtr;
    return TCL_OK;

  procError:
    if (precompiled) {
	procPtr->refCount--;
    } else {
	Tcl_DecrRefCount(bodyPtr);
	while (procPtr->firstLocalPtr != NULL) {
	    localPtr = procPtr->firstLocalPtr;
	    procPtr->firstLocalPtr = localPtr->nextPtr;

	    if (localPtr->defValuePtr != NULL) {
		Tcl_DecrRefCount(localPtr->defValuePtr);
	    }

	    Tcl_Free(localPtr);
	}
	Tcl_Free(procPtr);
    }
    if (errorCode) {
	if (errorCode == TOOMANYARGS) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "procedure \"%s\": arg list contains too many (%"
		    TCL_SIZE_MODIFIER "d) entries", procName, numArgs));
	}
	Tcl_SetErrorCode(interp, "TCL", "OPERATION", "PROC",
		errorCode, (char *)NULL);
    }
    return TCL_ERROR;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; whole source already launch-pinned`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimCreateProcedureCmd`, lines 4360–4445. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
static Jim_Cmd *JimCreateProcedureCmd(Jim_Interp *interp, Jim_Obj *argListObjPtr,
    Jim_Obj *staticsListObjPtr, Jim_Obj *bodyObjPtr, Jim_Obj *nsObj)
{
    Jim_Cmd *cmdPtr;
    int argListLen;
    int i;

    argListLen = Jim_ListLength(interp, argListObjPtr);

    /* Allocate space for both the command pointer and the arg list */
    cmdPtr = Jim_Alloc(sizeof(*cmdPtr) + sizeof(struct Jim_ProcArg) * argListLen);
    assert(cmdPtr);
    memset(cmdPtr, 0, sizeof(*cmdPtr));
    cmdPtr->inUse = 1;
    cmdPtr->flags = JIM_CMD_ISPROC;
    cmdPtr->u.proc.argListObjPtr = argListObjPtr;
    cmdPtr->u.proc.argListLen = argListLen;
    cmdPtr->u.proc.bodyObjPtr = bodyObjPtr;
    cmdPtr->u.proc.argsPos = -1;
    cmdPtr->u.proc.arglist = (struct Jim_ProcArg *)(cmdPtr + 1);
    cmdPtr->u.proc.nsObj = nsObj ? nsObj : interp->emptyObj;
    Jim_IncrRefCount(argListObjPtr);
    Jim_IncrRefCount(bodyObjPtr);
    Jim_IncrRefCount(cmdPtr->u.proc.nsObj);

    /* Create the statics hash table. */
    if (staticsListObjPtr && JimCreateProcedureStatics(interp, cmdPtr, staticsListObjPtr) != JIM_OK) {
        goto err;
    }

    /* Parse the args out into arglist, validating as we go */
    /* Examine the argument list for default parameters and 'args' */
    for (i = 0; i < argListLen; i++) {
        Jim_Obj *argPtr;
        Jim_Obj *nameObjPtr;
        Jim_Obj *defaultObjPtr;
        int len;

        /* Examine a parameter */
        argPtr = Jim_ListGetIndex(interp, argListObjPtr, i);
        len = Jim_ListLength(interp, argPtr);
        if (len == 0) {
            Jim_SetResultString(interp, "argument with no name", -1);
err:
            JimDecrCmdRefCount(interp, cmdPtr);
            return NULL;
        }
        if (len > 2) {
            Jim_SetResultFormatted(interp, "too many fields in argument specifier \"%#s\"", argPtr);
            goto err;
        }

        if (len == 2) {
            /* Optional parameter */
            nameObjPtr = Jim_ListGetIndex(interp, argPtr, 0);
            defaultObjPtr = Jim_ListGetIndex(interp, argPtr, 1);
        }
        else {
            /* Required parameter */
            nameObjPtr = argPtr;
            defaultObjPtr = NULL;
        }

        if (Jim_CompareStringImmediate(interp, nameObjPtr, "args")) {
            if (cmdPtr->u.proc.argsPos >= 0) {
                Jim_SetResultString(interp, "'args' specified more than once", -1);
                goto err;
            }
            cmdPtr->u.proc.argsPos = i;
        }
        else {
            if (len == 2) {
                cmdPtr->u.proc.optArity++;
            }
            else {
                cmdPtr->u.proc.reqArity++;
            }
        }

        cmdPtr->u.proc.arglist[i].nameObjPtr = nameObjPtr;
        cmdPtr->u.proc.arglist[i].defaultObjPtr = defaultObjPtr;
    }

    return cmdPtr;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; whole source already launch-pinned`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_ProcCoreCommand`, lines 14449–14481. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
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

```

## Consumer bindings

- [rust/tcl-vm/src/command/native_definition_order_tests.rs](../../../../rust/tcl-vm/src/command/native_definition_order_tests.rs), `command::native_definition_order_tests::original_procedure_public_completions_match_all_eighteen_native_definition_windows` (linked): Finite source-defined control: original procedure public completions match all eighteen native definition windows. Public worker and invocation code/result only: these values already have Strings and cannot assert the original extension's getter tape or counts. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_proc/original_argument_tests.rs](../../../../runtime/rust/src/cmd_proc/original_argument_tests.rs), `cmd_proc::original_argument_tests::original_procedure_definition_matches_all_c_name_body_and_formal_getter_windows` (linked): Finite source-defined control: original procedure definition matches all c name body and formal getter windows. Exact C499c original String reachability/public results, independently of body duplication, sharing counts, compiled code or Jim procedure semantics. Native499c supplies an original body already owned by its variable. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_proc/original_argument_tests.rs](../../../../runtime/rust/src/cmd_proc/original_argument_tests.rs), `cmd_proc::original_argument_tests::original_procedure_definition_matches_jim_formal_name_and_deferred_body_getters` (linked): Finite source-defined control: original procedure definition matches jim formal name and deferred body getters. Jim499d reaches the original formals and name at definition. The body updater is reached by case11's later invocation, not by definition. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_namespace/original_argument_tests.rs](../../../../runtime/rust/src/cmd_namespace/original_argument_tests.rs), `cmd_namespace::original_argument_tests::original_namespace_patterns_match_all_c_getter_and_public_effect_windows` (linked): Finite source-defined control: original namespace patterns match all c getter and public effect windows. Exact C499c selected windows only. C84's callback export axis stays unasserted; no binding-generation, header-count, Jim or whole-dispatch equivalence is claimed. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_namespace/original_argument_tests.rs](../../../../runtime/rust/src/cmd_namespace/original_argument_tests.rs), `cmd_namespace::original_argument_tests::original_namespace_patterns_match_reached_jim_getter_and_public_effect_windows` (linked): Finite source-defined control: original namespace patterns match reached jim getter and public effect windows. The independent Jim499d worker uses global original operands. Jim499c's missing-variable setup does not establish getter order. Export's ignored inputs and unsupported forget remain distinct from the measured imports. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 0 | 2 | 0 | 0 |
| `isolated` | 0 | 2 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_naming_argument_getter_effect_order_original](../../../../rust/tcl-registry/tests/data/native_naming_argument_getter_effect_order_original)
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
- [rust/tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/source-interpretation](../../../../rust/tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/source-interpretation)
