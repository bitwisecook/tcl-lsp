# naming.class.original-unaddressable-holder-publication

Kind: `native-observation`

## Problem statement

A rendered object name can combine holder and simple-name colon boundaries into equal bytes. Public reporting must be considered separately from the original relative creation and method calls.

## Question

What returned names, namespace reports and original relative method-call results distinguish p created in current a: from :p created in current a, including two creations with equal displayed names?

## Conclusion

C8.6/C9.0/C9.1 retain ORIGINAL0 values `::a:::p ::a: 1 FIRST` and `::a:::p ::a 1 SECOND`; the joint original produces `::a:::p ::a:::p FIRST SECOND 1 1`. Equal returned reports therefore coexist with separately reached original relative calls and both original command queries. The independent copy-labelled input stops with ORIGINAL1 because ::source already names the stock source command; no copy invocation/result is established by that input. C8.4/C8.5/Jim reach ORIGINAL0/NOT_APPLICABLE through the actual stock capability branch. All24 host processes and six compiler commands exit0 with empty stderr. Full pinned C holder/name/publication source is retained as separate source inspection; the public rows establish no private object, holder pointer, command token/header, entered definition frame, general publication algorithm or successful Native compiler admission. For this question the linked Runtime control consumes all four unchanged holder originals across six public columns (24 comparisons), including the independent ::source setup failure. The VM creation-only control consumes the three creation originals across six columns (18 comparisons), with no installed or inferred copy API. Shared linked controls total48 Runtime and24 VM comparisons across the creation/collision/copy records; these are assertion definitions, not an executed pass or another native provider observation.

A marked shared VM definition binds the three original constructed/unaddressable holder programs as its independent 18-window public subset.

## Scope

Four unchanged ASCII LF sources run in separate fresh full-init counted C API processes. Three finite creation/call/query controls and one failed setup specimen retain their independent original completion/result bytes. Original requests, driver, launcher, executable/required provider build pins and whole streams/receipts are retained. The copy-labelled setup refusal cannot donate copy evidence; original-copy-holder-and-collision records independently reached copy inputs. BIG-IP and executed Rust assertions are not tested. The Runtime original-source comparison explicitly installs the shared NativeScriptedLibrary::ALL distribution for its selected Jim full-initialisation source ingress, matching the original driver's full core/static-extension setup. This conformance bootstrap cannot turn a core-only constructor, authoring profile or metadata source carrier into a loaded library, current helper binding or Native worker. The exact original sources, provider rows and completion boundaries remain unchanged; the linked assertion is not a claimed executed pass.

The fixture selects an actual software core/compiler and compares public original bytes, without launching new original processes or observing private native token/header/cache identity. Constructed-holder reporting and collision results remain separate original questions. No passing software outcome, oo::copy availability, copy permission, activation/frame or compiler instruction admission is attached.

## Provider answers

### tcl8.4, tcl8.5, jim

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; jim 0.84-9-g5bac7c9.

Every unchanged source reaches ORIGINAL0/NOT_APPLICABLE through its actual capability branch. No supported TclOO/coroutine API or substitute Jim/C implementation is inferred.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Complete ORIGINAL code/value fields: bare-create-current-colon-holder=0/::a:::p ::a: 1 FIRST; colon-tail-create-current-holder=0/::a:::p ::a 1 SECOND; distinct-constructed-slots-with-equal-reports=0/::a:::p ::a:::p FIRST SECOND 1 1; copy-original-relative-constructed-slots=1/can't create object "::source": command already exists with that name. The ::source setup failure in copy-original-relative-constructed-slots occurs before copying and supplies no copy result.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Source inspection

tcl8.6 8.6.18, revision `Pinned independent original provider source`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOO.c`, function `TclNewObjectInstanceCommon`, lines 1792–1841. Retained reference `native_oo_original_holder248-request-source-anchors-8.6.18-tclOO.c`.

```text
TclNewObjectInstanceCommon(
    Tcl_Interp *interp,
    Class *classPtr,
    const char *nameStr,
    const char *nsNameStr)
{
    Tcl_HashEntry *hPtr;
    Foundation *fPtr = GetFoundation(interp);
    Object *oPtr;
    const char *simpleName = NULL;
    Namespace *nsPtr = NULL, *dummy,
	*inNsPtr = (Namespace *)TclGetCurrentNamespace(interp);

    if (nameStr) {
	TclGetNamespaceForQualName(interp, nameStr, inNsPtr,
		TCL_CREATE_NS_IF_UNKNOWN, &nsPtr, &dummy, &dummy, &simpleName);

	/*
	 * Disallow creation of an object over an existing command.
	 */

	hPtr = Tcl_FindHashEntry(&nsPtr->cmdTable, simpleName);
	if (hPtr) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "can't create object \"%s\": command already exists with"
		    " that name", nameStr));
	    Tcl_SetErrorCode(interp, "TCL", "OO", "OVERWRITE_OBJECT", (char *)NULL);
	    return NULL;
	}
    }

    /*
     * Create the object.
     */

    oPtr = AllocObject(interp, simpleName, nsPtr, nsNameStr);
    if (oPtr == NULL) {
	return NULL;
    }
    oPtr->selfCls = classPtr;
    AddRef(classPtr->thisPtr);
    TclOOAddToInstances(oPtr, classPtr);

    /*
     * Check to see if we're really creating a class. If so, allocate the
     * class structure as well.
     */

    if (TclOOIsReachable(fPtr->classCls, classPtr)) {
	/*

```

tcl8.6 8.6.18, revision `Pinned independent original provider source`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `TclCreateObjCommandInNs`, lines 2354–2403. Retained reference `native_oo_original_holder248-request-source-anchors-8.6.18-tclBasic.c`.

```text
TclCreateObjCommandInNs(
    Tcl_Interp *interp,
    const char *cmdName,	/* Name of command, without any namespace
				 * components. */
    Tcl_Namespace *namesp,	/* The namespace to create the command in */
    Tcl_ObjCmdProc *proc,	/* Object-based function to associate with
				 * name. */
    ClientData clientData,	/* Arbitrary value to pass to object
				 * function. */
    Tcl_CmdDeleteProc *deleteProc)
				/* If not NULL, gives a function to call when
				 * this command is deleted. */
{
    int deleted = 0, isNew = 0;
    Command *cmdPtr;
    ImportRef *oldRefPtr = NULL;
    ImportedCmdData *dataPtr;
    Tcl_HashEntry *hPtr;
    Namespace *nsPtr = (Namespace *) namesp;

    /*
     * If the command name we seek to create already exists, we need to delete
     * that first. That can be tricky in the presence of traces. Loop until we
     * no longer find an existing command in the way, or until we've deleted
     * one command and that didn't finish the job.
     */

    while (1) {
	hPtr = Tcl_CreateHashEntry(&nsPtr->cmdTable, cmdName, &isNew);

	if (isNew || deleted) {
	    /*
	     * isNew - No conflict with existing command.
	     * deleted - We've already deleted a conflicting command
	     */
	    break;
	}

	/*
	 * An existing command conflicts. Try to delete it...
	 */

	cmdPtr = (Command *)Tcl_GetHashValue(hPtr);

	/*
	 * [***] This is wrong.  See Tcl Bug a16752c252.
	 * However, this buggy behavior is kept under particular circumstances
	 * to accommodate deployed binaries of the "tclcompiler" program
	 * <http://sourceforge.net/projects/tclpro/> that crash if the bug is
	 * fixed.

```

tcl8.6 8.6.18, revision `Pinned independent original provider source`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclNamesp.c`, function `TclGetNamespaceForQualName`, lines 2141–2190. Retained reference `native_oo_original_holder248-request-source-anchors-8.6.18-tclNamesp.c`.

```text
TclGetNamespaceForQualName(
    Tcl_Interp *interp,		/* Interpreter in which to find the namespace
				 * containing qualName. */
    const char *qualName,	/* A namespace-qualified name of an command,
				 * variable, or namespace. */
    Namespace *cxtNsPtr,	/* The namespace in which to start the search
				 * for qualName's namespace. If NULL start
				 * from the current namespace. Ignored if
				 * TCL_GLOBAL_ONLY is set. */
    int flags,			/* Flags controlling the search: an OR'd
				 * combination of TCL_GLOBAL_ONLY,
				 * TCL_NAMESPACE_ONLY, TCL_FIND_ONLY_NS, and
				 * TCL_CREATE_NS_IF_UNKNOWN. */
    Namespace **nsPtrPtr,	/* Address where function stores a pointer to
				 * containing namespace if qualName is found
				 * starting from *cxtNsPtr or, if
				 * TCL_GLOBAL_ONLY is set, if qualName is
				 * found in the global :: namespace. NULL is
				 * stored otherwise. */
    Namespace **altNsPtrPtr,	/* Address where function stores a pointer to
				 * containing namespace if qualName is found
				 * starting from the global :: namespace.
				 * NULL is stored if qualName isn't found
				 * starting from :: or if the TCL_GLOBAL_ONLY,
				 * TCL_NAMESPACE_ONLY, TCL_FIND_ONLY_NS,
				 * TCL_CREATE_NS_IF_UNKNOWN flag is set. */
    Namespace **actualCxtPtrPtr,/* Address where function stores a pointer to
				 * the actual namespace from which the search
				 * started. This is either cxtNsPtr, the ::
				 * namespace if TCL_GLOBAL_ONLY was specified,
				 * or the current namespace if cxtNsPtr was
				 * NULL. */
    const char **simpleNamePtr) /* Address where function stores the simple
				 * name at end of the qualName, or NULL if
				 * qualName is "::" or the flag
				 * TCL_FIND_ONLY_NS was specified. */
{
    Interp *iPtr = (Interp *) interp;
    Namespace *nsPtr = cxtNsPtr, *lastNsPtr = NULL, *lastAltNsPtr = NULL;
    Namespace *altNsPtr;
    Namespace *globalNsPtr = iPtr->globalNsPtr;
    const char *start, *end;
    const char *nsName;
    Tcl_HashEntry *entryPtr;
    Tcl_DString buffer;
    int len;

    /*
     * Determine the context namespace nsPtr in which to start the primary
     * search. If the qualName name starts with a "::" or TCL_GLOBAL_ONLY was

```

tcl9.0 9.0.4, revision `Pinned independent original provider source`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.c`, function `TclNewObjectInstanceCommon`, lines 2134–2183. Retained reference `native_oo_original_holder248-request-source-anchors-9.0.4-tclOO.c`.

```text
TclNewObjectInstanceCommon(
    Tcl_Interp *interp,
    Class *classPtr,
    const char *nameStr,
    const char *nsNameStr)
{
    Tcl_HashEntry *hPtr;
    Foundation *fPtr = GetFoundation(interp);
    Object *oPtr;
    const char *simpleName = NULL;
    Namespace *nsPtr = NULL, *dummy;
    Namespace *inNsPtr = (Namespace *) TclGetCurrentNamespace(interp);

    if (nameStr) {
	TclGetNamespaceForQualName(interp, nameStr, inNsPtr,
		TCL_CREATE_NS_IF_UNKNOWN, &nsPtr, &dummy, &dummy, &simpleName);

	/*
	 * Disallow creation of an object over an existing command.
	 */

	hPtr = Tcl_FindHashEntry(&nsPtr->cmdTable, simpleName);
	if (hPtr) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "can't create object \"%s\": command already exists with"
		    " that name", nameStr));
	    OO_ERROR(interp, OVERWRITE_OBJECT);
	    return NULL;
	}
    }

    /*
     * Create the object.
     */

    oPtr = AllocObject(interp, simpleName, nsPtr, nsNameStr);
    if (oPtr == NULL) {
	return NULL;
    }
    oPtr->selfCls = classPtr;
    AddRef(classPtr->thisPtr);
    TclOOAddToInstances(oPtr, classPtr);

    /*
     * Check to see if we're really creating a class. If so, allocate the
     * class structure as well.
     */

    if (TclOOIsReachable(fPtr->classCls, classPtr)) {
	/*

```

tcl9.0 9.0.4, revision `Pinned independent original provider source`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `TclCreateObjCommandInNs`, lines 2946–2995. Retained reference `native_oo_original_holder248-request-source-anchors-9.0.4-tclBasic.c`.

```text
TclCreateObjCommandInNs(
    Tcl_Interp *interp,
    const char *cmdName,	/* Name of command, without any namespace
				 * components. */
    Tcl_Namespace *namesp,	/* The namespace to create the command in */
    Tcl_ObjCmdProc *proc,	/* Object-based function to associate with
				 * name. */
    void *clientData,		/* Arbitrary value to pass to object
				 * function. */
    Tcl_CmdDeleteProc *deleteProc)
				/* If not NULL, gives a function to call when
				 * this command is deleted. */
{
    int deleted = 0, isNew = 0;
    Command *cmdPtr;
    ImportRef *oldRefPtr = NULL;
    ImportedCmdData *dataPtr;
    Tcl_HashEntry *hPtr;
    Namespace *nsPtr = (Namespace *) namesp;

    /*
     * If the command name we seek to create already exists, we need to delete
     * that first. That can be tricky in the presence of traces. Loop until we
     * no longer find an existing command in the way, or until we've deleted
     * one command and that didn't finish the job.
     */

    while (1) {
	hPtr = Tcl_CreateHashEntry(&nsPtr->cmdTable, cmdName, &isNew);

	if (isNew || deleted) {
	    /*
	     * isNew - No conflict with existing command.
	     * deleted - We've already deleted a conflicting command
	     */
	    break;
	}

	/*
	 * An existing command conflicts. Try to delete it...
	 */

	cmdPtr = (Command *)Tcl_GetHashValue(hPtr);

	/*
	 * Command already exists; delete it. Be careful to preserve any
	 * existing import links so we can restore them down below. That way,
	 * you can redefine a command and its import status will remain
	 * intact.
	 */

```

tcl9.0 9.0.4, revision `Pinned independent original provider source`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclNamesp.c`, function `TclGetNamespaceForQualName`, lines 2301–2350. Retained reference `native_oo_original_holder248-request-source-anchors-9.0.4-tclNamesp.c`.

```text
TclGetNamespaceForQualName(
    Tcl_Interp *interp,		/* Interpreter in which to find the namespace
				 * containing qualName. */
    const char *qualName,	/* A namespace-qualified name of an command,
				 * variable, or namespace. */
    Namespace *cxtNsPtr,	/* The namespace in which to start the search
				 * for qualName's namespace. If NULL start
				 * from the current namespace. Ignored if
				 * TCL_GLOBAL_ONLY is set. */
    int flags,			/* Flags controlling the search: an OR'd
				 * combination of TCL_GLOBAL_ONLY,
				 * TCL_NAMESPACE_ONLY, TCL_FIND_ONLY_NS, and
				 * TCL_CREATE_NS_IF_UNKNOWN. */
    Namespace **nsPtrPtr,	/* Address where function stores a pointer to
				 * containing namespace if qualName is found
				 * starting from *cxtNsPtr or, if
				 * TCL_GLOBAL_ONLY is set, if qualName is
				 * found in the global :: namespace. NULL is
				 * stored otherwise. */
    Namespace **altNsPtrPtr,	/* Address where function stores a pointer to
				 * containing namespace if qualName is found
				 * starting from the global :: namespace.
				 * NULL is stored if qualName isn't found
				 * starting from :: or if the TCL_GLOBAL_ONLY,
				 * TCL_NAMESPACE_ONLY, TCL_FIND_ONLY_NS,
				 * TCL_CREATE_NS_IF_UNKNOWN flag is set. */
    Namespace **actualCxtPtrPtr,/* Address where function stores a pointer to
				 * the actual namespace from which the search
				 * started. This is either cxtNsPtr, the ::
				 * namespace if TCL_GLOBAL_ONLY was specified,
				 * or the current namespace if cxtNsPtr was
				 * NULL. */
    const char **simpleNamePtr) /* Address where function stores the simple
				 * name at end of the qualName, or NULL if
				 * qualName is "::" or the flag
				 * TCL_FIND_ONLY_NS was specified. */
{
    Interp *iPtr = (Interp *) interp;
    Namespace *nsPtr = cxtNsPtr, *lastNsPtr = NULL, *lastAltNsPtr = NULL;
    Namespace *altNsPtr;
    Namespace *globalNsPtr = iPtr->globalNsPtr;
    const char *start, *end;
    const char *nsName;
    Tcl_HashEntry *entryPtr;
    Tcl_DString buffer;
    int len;

    /*
     * Determine the context namespace nsPtr in which to start the primary
     * search. If the qualName name starts with a "::" or TCL_GLOBAL_ONLY was

```

tcl9.1 9.1.0, revision `Pinned independent original provider source`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.c`, function `TclNewObjectInstanceCommon`, lines 2115–2164. Retained reference `native_oo_original_holder248-request-source-anchors-9.1.0-tclOO.c`.

```text
TclNewObjectInstanceCommon(
    Tcl_Interp *interp,
    Class *classPtr,
    const char *nameStr,
    const char *nsNameStr)
{
    Tcl_HashEntry *hPtr;
    Foundation *fPtr = GetFoundation(interp);
    Object *oPtr;
    const char *simpleName = NULL;
    Namespace *nsPtr = NULL, *dummy;
    Namespace *inNsPtr = (Namespace *) TclGetCurrentNamespace(interp);

    if (nameStr) {
	TclGetNamespaceForQualName(interp, nameStr, inNsPtr,
		TCL_CREATE_NS_IF_UNKNOWN, &nsPtr, &dummy, &dummy, &simpleName);

	/*
	 * Disallow creation of an object over an existing command.
	 */

	hPtr = Tcl_FindHashEntry(&nsPtr->cmdTable, simpleName);
	if (hPtr) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "can't create object \"%s\": command already exists with"
		    " that name", nameStr));
	    OO_ERROR(interp, OVERWRITE_OBJECT);
	    return NULL;
	}
    }

    /*
     * Create the object.
     */

    oPtr = AllocObject(interp, simpleName, nsPtr, nsNameStr);
    if (oPtr == NULL) {
	return NULL;
    }
    oPtr->selfCls = classPtr;
    AddRef(classPtr->thisPtr);
    TclOOAddToInstances(oPtr, classPtr);

    /*
     * Check to see if we're really creating a class. If so, allocate the
     * class structure as well.
     */

    if (TclOOIsReachable(fPtr->classCls, classPtr)) {
	/*

```

tcl9.1 9.1.0, revision `Pinned independent original provider source`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `TclCreateObjCommandInNs`, lines 2902–2951. Retained reference `native_oo_original_holder248-request-source-anchors-9.1.0-tclBasic.c`.

```text
TclCreateObjCommandInNs(
    Tcl_Interp *interp,
    const char *cmdName,	/* Name of command, without any namespace
				 * components. */
    Tcl_Namespace *namesp,	/* The namespace to create the command in */
    Tcl_ObjCmdProc2 *proc,	/* Object-based function to associate with
				 * name. */
    void *clientData,		/* Arbitrary value to pass to object
				 * function. */
    Tcl_CmdDeleteProc *deleteProc)
				/* If not NULL, gives a function to call when
				 * this command is deleted. */
{
    bool deleted = false;
    int isNew = 0;
    Command *cmdPtr;
    ImportRef *oldRefPtr = NULL;
    ImportedCmdData *dataPtr;
    Tcl_HashEntry *hPtr;
    Namespace *nsPtr = (Namespace *) namesp;

    /*
     * If the command name we seek to create already exists, we need to delete
     * that first. That can be tricky in the presence of traces. Loop until we
     * no longer find an existing command in the way, or until we've deleted
     * one command and that didn't finish the job.
     */

    while (1) {
	hPtr = Tcl_CreateHashEntry(&nsPtr->cmdTable, cmdName, &isNew);

	if (isNew || deleted) {
	    /*
	     * isNew - No conflict with existing command.
	     * deleted - We've already deleted a conflicting command
	     */
	    break;
	}

	/*
	 * An existing command conflicts. Try to delete it...
	 */

	cmdPtr = (Command *)Tcl_GetHashValue(hPtr);

	/*
	 * Command already exists; delete it. Be careful to preserve any
	 * existing import links so we can restore them down below. That way,
	 * you can redefine a command and its import status will remain
	 * intact.

```

tcl9.1 9.1.0, revision `Pinned independent original provider source`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclNamesp.c`, function `TclGetNamespaceForQualName`, lines 2298–2347. Retained reference `native_oo_original_holder248-request-source-anchors-9.1.0-tclNamesp.c`.

```text
TclGetNamespaceForQualName(
    Tcl_Interp *interp,		/* Interpreter in which to find the namespace
				 * containing qualName. */
    const char *qualName,	/* A namespace-qualified name of an command,
				 * variable, or namespace. */
    Namespace *cxtNsPtr,	/* The namespace in which to start the search
				 * for qualName's namespace. If NULL start
				 * from the current namespace. Ignored if
				 * TCL_GLOBAL_ONLY is set. */
    int flags,			/* Flags controlling the search: an OR'd
				 * combination of TCL_GLOBAL_ONLY,
				 * TCL_NAMESPACE_ONLY, TCL_FIND_ONLY_NS, and
				 * TCL_CREATE_NS_IF_UNKNOWN. */
    Namespace **nsPtrPtr,	/* Address where function stores a pointer to
				 * containing namespace if qualName is found
				 * starting from *cxtNsPtr or, if
				 * TCL_GLOBAL_ONLY is set, if qualName is
				 * found in the global :: namespace. NULL is
				 * stored otherwise. */
    Namespace **altNsPtrPtr,	/* Address where function stores a pointer to
				 * containing namespace if qualName is found
				 * starting from the global :: namespace.
				 * NULL is stored if qualName isn't found
				 * starting from :: or if the TCL_GLOBAL_ONLY,
				 * TCL_NAMESPACE_ONLY, TCL_FIND_ONLY_NS,
				 * TCL_CREATE_NS_IF_UNKNOWN flag is set. */
    Namespace **actualCxtPtrPtr,/* Address where function stores a pointer to
				 * the actual namespace from which the search
				 * started. This is either cxtNsPtr, the ::
				 * namespace if TCL_GLOBAL_ONLY was specified,
				 * or the current namespace if cxtNsPtr was
				 * NULL. */
    const char **simpleNamePtr)	/* Address where function stores the simple
				 * name at end of the qualName, or NULL if
				 * qualName is "::" or the flag
				 * TCL_FIND_ONLY_NS was specified. */
{
    Interp *iPtr = (Interp *) interp;
    Namespace *nsPtr = cxtNsPtr, *lastNsPtr = NULL, *lastAltNsPtr = NULL;
    Namespace *altNsPtr;
    Namespace *globalNsPtr = iPtr->globalNsPtr;
    const char *start, *end;
    const char *nsName;
    Tcl_HashEntry *entryPtr;
    Tcl_DString buffer;
    int len;

    /*
     * Determine the context namespace nsPtr in which to start the primary
     * search. If the qualName name starts with a "::" or TCL_GLOBAL_ONLY was

```

## Consumer bindings

- [runtime/rust/src/cmd_oo/native_original_holder_tests.rs](../../../../runtime/rust/src/cmd_oo/native_original_holder_tests.rs), `cmd_oo::native_original_holder_tests::original_oo_publication_preserves_constructed_holder_and_collision_results` (linked): Compare the three untouched creation originals across six public code/result columns. VM copy support is absent and cannot be inferred from creation or public report equality. Selected Jim full-init conformance ingress explicitly installs the shared source distribution; bootstrap is distinct from core-only and metadata purposes, with unchanged original rows and no executed pass claim.

- [rust/tcl-vm/src/cmd_oo/native_original_holder_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_original_holder_tests.rs), `original_result`: Decode the exact original OO completion/result fields, preserving holder-creation and collision fixture groups independently of reporting labels.
- [rust/tcl-vm/src/cmd_oo/native_original_holder_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_original_holder_tests.rs), `cmd_oo::native_original_holder_tests::original_oo_publication_preserves_constructed_holder_and_collision_results` (linked): The shared VM body compares24 public completion/result windows across four unchanged ASCII originals and six selected core/compiler fixtures. This question binds only the three original constructed/unaddressable holder programs, 18 of those windows; equal reporting names cannot prove holder equality and no oo::copy entry is inferred.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Evidence

- [rust/tcl-registry/tests/data/native_oo_original_holder248/8.4.20](../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/8.4.20)
- [rust/tcl-registry/tests/data/native_oo_original_holder248](../../../../rust/tcl-registry/tests/data/native_oo_original_holder248)
- [rust/tcl-registry/tests/data/native_oo_original_holder248/8.5.19](../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/8.5.19)
- [rust/tcl-registry/tests/data/native_oo_original_holder248/8.6.18](../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/8.6.18)
- [rust/tcl-registry/tests/data/native_oo_original_holder248/9.0.4](../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/9.0.4)
- [rust/tcl-registry/tests/data/native_oo_original_holder248/9.1.0](../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/9.1.0)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_oo_original_holder248/jim](../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/jim)
