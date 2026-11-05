# naming.expression.compiler-function-name-literals

Kind: `source-anchor`

## Problem statement

Treating every ExprCall as an unknown compile-time command effect withdraws the source command world before a genuine readonly expression function occurrence can reach its diagnostic lookup. Runtime function dispatch and pooled object callbacks cannot be inferred from syntax.

## Question

Which name operation does the selected C8.5–9.1 expression compiler perform for an original function node before runtime invocation?

## Conclusion

The inspected compiler forms the counted tcl::mathfunc:: prefix plus original function identifier and registers it as a literal; this code does not execute or look up the function as a command while compiling. C8.5 selects namespace-scoped pooling, C8.6–9.1 command-name pooling. Existing pool string getters and reference release can expose object callbacks, so the Compiler consumer additionally requires current ordinary-pool effects and quiet callback-free lookup. Original bracket-script compiler visits remain recursive and exact. C8.4/Jim calls and Raw/foreign contexts stay unknown for this purpose. No evaluator, actual function registration, handler completion, Normal, cache, body compilation or native admission is granted.

## Scope

Pinned C8.5.19/8.6.18/9.0.4/9.1.0 FUNCTION emission and counted literal allocation/reuse/release; source inspection only. Existing call-free topology predicate is unchanged.

## Provider answers

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `inspected`. Channel: Inspected FUNCTION emission, literal allocation/reuse/release code and selected macro; no native input supplied. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Compiles a counted tcl::mathfunc:: qualified literal before emitting runtime invocation. C8.5 uses the namespace-scoped literal flag; C8.6–9.1 use command-name literals. Literal reuse may get a string from an existing pooled object; release decrements references. Object callbacks are an independent prerequisite.

### tcl8.4, jim, bigip

Status: `not-tested`. Channel: No input supplied for this modern C compiler purpose. Version: tcl8.4 8.4.20; jim 0.84-9-g5bac7c9.

No modern C command-literal compiler recipe is selected for this provider. No native function-dispatch or evaluator observation is claimed.

## Source inspection

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCompExpr.c`, function `CompileExprTree`, lines 2196–2221. Retained reference `tcl8.5-0`.

```text
	    case FUNCTION: {
		Tcl_DString cmdName;
		const char *p;
		int length;

		Tcl_DStringInit(&cmdName);
		Tcl_DStringAppend(&cmdName, "tcl::mathfunc::", -1);
		p = TclGetStringFromObj(*funcObjv, &length);
		funcObjv++;
		Tcl_DStringAppend(&cmdName, p, length);
		TclEmitPush(TclRegisterNewNSLiteral(envPtr,
			Tcl_DStringValue(&cmdName),
			Tcl_DStringLength(&cmdName)), envPtr);
		Tcl_DStringFree(&cmdName);

		/*
		 * Start a count of the number of words in this function
		 * command invocation.  In case there's already a count
		 * in progress (nested functions), save it in our unused
		 * "left" field for restoring later.
		 */

		nodePtr->left = numWords;
		numWords = 2;	/* Command plus one argument */
		break;
	    }

```

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclLiteral.c`, function `TclCreateLiteral`, lines 175–301. Retained reference `tcl8.5-1`.

```text
TclCreateLiteral(
    Interp *iPtr,
    char *bytes,
    int length,
    unsigned int hash,       /* The string's hash. If -1, it will be computed here */
    int *newPtr,
    Namespace *nsPtr,
    int flags,
    LiteralEntry **globalPtrPtr)
{
    LiteralTable *globalTablePtr = &(iPtr->literalTable);
    LiteralEntry *globalPtr;
    int globalHash;
    Tcl_Obj *objPtr;

    /*
     * Is it in the interpreter's global literal table?
     */

    if (hash == (unsigned int) -1) {
	hash = HashString(bytes, length);
    }
    globalHash = (hash & globalTablePtr->mask);
    for (globalPtr=globalTablePtr->buckets[globalHash] ; globalPtr!=NULL;
	    globalPtr = globalPtr->nextPtr) {
	objPtr = globalPtr->objPtr;
	if ((globalPtr->nsPtr == nsPtr)
		&& (objPtr->length == length) && ((length == 0)
		|| ((objPtr->bytes[0] == bytes[0])
		&& (memcmp(objPtr->bytes, bytes, (unsigned) length) == 0)))) {
	    /*
	     * A literal was found: return it
	     */

	    if (newPtr) {
		*newPtr = 0;
	    }
	    if (globalPtrPtr) {
		*globalPtrPtr = globalPtr;
	    }
	    if (flags & LITERAL_ON_HEAP) {
		ckfree(bytes);
	    }
	    globalPtr->refCount++;
	    return objPtr;
	}
    }
    if (!newPtr) {
	if (flags & LITERAL_ON_HEAP) {
	    ckfree(bytes);
	}
	return NULL;
    }

    /*
     * The literal is new to the interpreter. Add it to the global literal
     * table.
     */

    TclNewObj(objPtr);
    Tcl_IncrRefCount(objPtr);
    if (flags & LITERAL_ON_HEAP) {
	objPtr->bytes = bytes;
	objPtr->length = length;
    } else {
	TclInitStringRep(objPtr, bytes, length);
    }

#ifdef TCL_COMPILE_DEBUG
    if (LookupLiteralEntry((Tcl_Interp *) iPtr, objPtr) != NULL) {
	Tcl_Panic("TclRegisterLiteral: literal \"%.*s\" found globally but shouldn't be",
		(length>60? 60 : length), bytes);
    }
#endif

    globalPtr = (LiteralEntry *) ckalloc((unsigned) sizeof(LiteralEntry));
    globalPtr->objPtr = objPtr;
    globalPtr->refCount = 1;
    globalPtr->nsPtr = nsPtr;
    globalPtr->nextPtr = globalTablePtr->buckets[globalHash];
    globalTablePtr->buckets[globalHash] = globalPtr;
    globalTablePtr->numEntries++;

    /*
     * If the global literal table has exceeded a decent size, rebuild it with
     * more buckets.
     */

    if (globalTablePtr->numEntries >= globalTablePtr->rebuildSize) {
	RebuildLiteralTable(globalTablePtr);
    }

#ifdef TCL_COMPILE_DEBUG
    TclVerifyGlobalLiteralTable(iPtr);
    {
	LiteralEntry *entryPtr;
	int found, i;

	found = 0;
	for (i=0 ; i<globalTablePtr->numBuckets ; i++) {
	    for (entryPtr=globalTablePtr->buckets[i]; entryPtr!=NULL ;
		    entryPtr=entryPtr->nextPtr) {
		if ((entryPtr == globalPtr) && (entryPtr->objPtr == objPtr)) {
		    found = 1;
		}
	    }
	}
	if (!found) {
	    Tcl_Panic("TclRegisterLiteral: literal \"%.*s\" wasn't global",
		    (length>60? 60 : length), bytes);
	}
    }
#endif /*TCL_COMPILE_DEBUG*/

#ifdef TCL_COMPILE_STATS
    iPtr->stats.numLiteralsCreated++;
    iPtr->stats.totalLitStringBytes += (double) (length + 1);
    iPtr->stats.currentLitStringBytes += (double) (length + 1);
    iPtr->stats.literalCount[TclLog2(length)]++;
#endif /*TCL_COMPILE_STATS*/

    if (globalPtrPtr) {
	*globalPtrPtr = globalPtr;
    }
    *newPtr = 1;
    return objPtr;
}

```

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclLiteral.c`, function `TclRegisterLiteral`, lines 331–412. Retained reference `tcl8.5-2`.

```text
TclRegisterLiteral(
    CompileEnv *envPtr,		/* Points to the CompileEnv in whose object
				 * array an object is found or created. */
    register char *bytes,	/* Points to string for which to find or
				 * create an object in CompileEnv's object
				 * array. */
    int length,			/* Number of bytes in the string. If < 0, the
				 * string consists of all bytes up to the
				 * first null character. */
    int flags)			/* If LITERAL_ON_HEAP then the caller already
				 * malloc'd bytes and ownership is passed to
				 * this function. If LITERAL_NS_SCOPE then
				 * the literal shouldnot be shared accross
				 * namespaces. */
{
    Interp *iPtr = envPtr->iPtr;
    LiteralTable *localTablePtr = &(envPtr->localLitTable);
    LiteralEntry *globalPtr, *localPtr;
    Tcl_Obj *objPtr;
    unsigned int hash;
    int localHash, objIndex, new;
    Namespace *nsPtr;

    if (length < 0) {
	length = (bytes ? strlen(bytes) : 0);
    }
    hash = HashString(bytes, length);

    /*
     * Is the literal already in the CompileEnv's local literal array? If so,
     * just return its index.
     */

    localHash = (hash & localTablePtr->mask);
    for (localPtr=localTablePtr->buckets[localHash] ; localPtr!=NULL;
	    localPtr = localPtr->nextPtr) {
	objPtr = localPtr->objPtr;
	if ((objPtr->length == length) && ((length == 0)
		|| ((objPtr->bytes[0] == bytes[0])
		&& (memcmp(objPtr->bytes, bytes, (unsigned) length) == 0)))) {
	    if (flags & LITERAL_ON_HEAP) {
		ckfree(bytes);
	    }
	    objIndex = (localPtr - envPtr->literalArrayPtr);
#ifdef TCL_COMPILE_DEBUG
	    TclVerifyLocalLiteralTable(envPtr);
#endif /*TCL_COMPILE_DEBUG*/

	    return objIndex;
	}
    }

    /*
     * The literal is new to this CompileEnv. Should it be shared accross
     * namespaces? If it is a fully qualified name, the namespace
     * specification is not needed to avoid sharing.
     */

    if ((flags & LITERAL_NS_SCOPE) && iPtr->varFramePtr
	    && ((length <2) || (bytes[0] != ':') || (bytes[1] != ':'))) {
	nsPtr = iPtr->varFramePtr->nsPtr;
    } else {
	nsPtr = NULL;
    }

    /*
     * Is it in the interpreter's global literal table? If not, create it.
     */

    objPtr = TclCreateLiteral(iPtr, bytes, length, hash, &new, nsPtr,
	    flags, &globalPtr);
    objIndex = AddLocalLiteralEntry(envPtr, objPtr, localHash);

#ifdef TCL_COMPILE_DEBUG
    if (globalPtr->refCount < 1) {
	Tcl_Panic("TclRegisterLiteral: global literal \"%.*s\" had bad refCount %d",
		(length>60? 60 : length), bytes, globalPtr->refCount);
    }
    TclVerifyLocalLiteralTable(envPtr);
#endif /*TCL_COMPILE_DEBUG*/
    return objIndex;
}

```

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclLiteral.c`, function `TclReleaseLiteral`, lines 750–807. Retained reference `tcl8.5-3`.

```text
TclReleaseLiteral(
    Tcl_Interp *interp,		/* Interpreter for which objPtr was created to
				 * hold a literal. */
    register Tcl_Obj *objPtr)	/* Points to a literal object that was
				 * previously created by a call to
				 * TclRegisterLiteral. */
{
    Interp *iPtr = (Interp *) interp;
    LiteralTable *globalTablePtr = &(iPtr->literalTable);
    register LiteralEntry *entryPtr, *prevPtr;
    char *bytes;
    int length, index;

    bytes = TclGetStringFromObj(objPtr, &length);
    index = (HashString(bytes, length) & globalTablePtr->mask);

    /*
     * Check to see if the object is in the global literal table and remove
     * this reference. The object may not be in the table if it is a hidden
     * local literal.
     */

    for (prevPtr=NULL, entryPtr=globalTablePtr->buckets[index];
	    entryPtr!=NULL ; prevPtr=entryPtr, entryPtr=entryPtr->nextPtr) {
	if (entryPtr->objPtr == objPtr) {
	    entryPtr->refCount--;

	    /*
	     * If the literal is no longer being used by any ByteCode, delete
	     * the entry then remove the reference corresponding to the global
	     * literal table entry (decrement the ref count of the object).
	     */

	    if (entryPtr->refCount == 0) {
		if (prevPtr == NULL) {
		    globalTablePtr->buckets[index] = entryPtr->nextPtr;
		} else {
		    prevPtr->nextPtr = entryPtr->nextPtr;
		}
		ckfree((char *) entryPtr);
		globalTablePtr->numEntries--;

		TclDecrRefCount(objPtr);

#ifdef TCL_COMPILE_STATS
		iPtr->stats.currentLitStringBytes -= (double) (length + 1);
#endif /*TCL_COMPILE_STATS*/
	    }
	    break;
	}
    }

    /*
     * Remove the reference corresponding to the local literal table entry.
     */

    Tcl_DecrRefCount(objPtr);
}

```

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCompile.h`, function `TclRegisterNewNSLiteral`, lines 989–991. Retained reference `tcl8.5-4`.

```text
#define TclRegisterNewNSLiteral(envPtr, bytes, length) \
	TclRegisterLiteral(envPtr, (char *)(bytes), length, \
		/*flags*/ LITERAL_NS_SCOPE)

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCompExpr.c`, function `CompileExprTree`, lines 2270–2295. Retained reference `tcl8.6-0`.

```text
	    case FUNCTION: {
		Tcl_DString cmdName;
		const char *p;
		int length;

		Tcl_DStringInit(&cmdName);
		TclDStringAppendLiteral(&cmdName, "tcl::mathfunc::");
		p = TclGetStringFromObj(*funcObjv, &length);
		funcObjv++;
		Tcl_DStringAppend(&cmdName, p, length);
		TclEmitPush(TclRegisterNewCmdLiteral(envPtr,
			Tcl_DStringValue(&cmdName),
			Tcl_DStringLength(&cmdName)), envPtr);
		Tcl_DStringFree(&cmdName);

		/*
		 * Start a count of the number of words in this function
		 * command invocation. In case there's already a count in
		 * progress (nested functions), save it in our unused "left"
		 * field for restoring later.
		 */

		nodePtr->left = numWords;
		numWords = 2;	/* Command plus one argument */
		break;
	    }

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclLiteral.c`, function `TclCreateLiteral`, lines 175–324. Retained reference `tcl8.6-1`.

```text
TclCreateLiteral(
    Interp *iPtr,
    char *bytes,		/* The start of the string. Note that this is
				 * not a NUL-terminated string. */
    int length,			/* Number of bytes in the string. */
    unsigned hash,		/* The string's hash. If -1, it will be
				 * computed here. */
    int *newPtr,
    Namespace *nsPtr,
    int flags,
    LiteralEntry **globalPtrPtr)
{
    LiteralTable *globalTablePtr = &iPtr->literalTable;
    LiteralEntry *globalPtr;
    int globalHash;
    Tcl_Obj *objPtr;

    /*
     * Is it in the interpreter's global literal table?
     */

    if (hash == (unsigned) -1) {
	hash = HashString(bytes, length);
    }
    globalHash = (hash & globalTablePtr->mask);
    for (globalPtr=globalTablePtr->buckets[globalHash] ; globalPtr!=NULL;
	    globalPtr = globalPtr->nextPtr) {
	objPtr = globalPtr->objPtr;
	if (globalPtr->nsPtr == nsPtr) {
	    /*
	     * Literals should always have UTF-8 representations... but this
	     * is not guaranteed so we need to be careful anyway.
	     *
	     * https://stackoverflow.com/q/54337750/301832
	     */

	    int objLength;
	    char *objBytes = TclGetStringFromObj(objPtr, &objLength);

	    if ((objLength == length) && ((length == 0)
		    || ((objBytes[0] == bytes[0])
		    && (memcmp(objBytes, bytes, length) == 0)))) {
		/*
		 * A literal was found: return it
		 */

		if (newPtr) {
		    *newPtr = 0;
		}
		if (globalPtrPtr) {
		    *globalPtrPtr = globalPtr;
		}
		if (flags & LITERAL_ON_HEAP) {
		    ckfree(bytes);
		}
		globalPtr->refCount++;
		return objPtr;
	    }
	}
    }
    if (!newPtr) {
	if ((flags & LITERAL_ON_HEAP)) {
	    ckfree(bytes);
	}
	return NULL;
    }

    /*
     * The literal is new to the interpreter. Add it to the global literal
     * table.
     */

    TclNewObj(objPtr);
    if ((flags & LITERAL_ON_HEAP)) {
	objPtr->bytes = bytes;
	objPtr->length = length;
    } else {
	TclInitStringRep(objPtr, bytes, length);
    }

    if ((flags & LITERAL_UNSHARED)) {
	/*
	 * Make clear, that no global value is returned
	 */
	if (globalPtrPtr != NULL) {
	    *globalPtrPtr = NULL;
	}
	return objPtr;
    }

#ifdef TCL_COMPILE_DEBUG
    if (LookupLiteralEntry((Tcl_Interp *) iPtr, objPtr) != NULL) {
	Tcl_Panic("%s: literal \"%.*s\" found globally but shouldn't be",
		"TclRegisterLiteral", (length>60? 60 : length), bytes);
    }
#endif

    globalPtr = (LiteralEntry *)ckalloc(sizeof(LiteralEntry));
    globalPtr->objPtr = objPtr;
    Tcl_IncrRefCount(objPtr);
    globalPtr->refCount = 1;
    globalPtr->nsPtr = nsPtr;
    globalPtr->nextPtr = globalTablePtr->buckets[globalHash];
    globalTablePtr->buckets[globalHash] = globalPtr;
    globalTablePtr->numEntries++;

    /*
     * If the global literal table has exceeded a decent size, rebuild it with
     * more buckets.
     */

    if (globalTablePtr->numEntries >= globalTablePtr->rebuildSize) {
	RebuildLiteralTable(globalTablePtr);
    }

#ifdef TCL_COMPILE_DEBUG
    TclVerifyGlobalLiteralTable(iPtr);
    {
	LiteralEntry *entryPtr;
	int found, i;

	found = 0;
	for (i=0 ; i<globalTablePtr->numBuckets ; i++) {
	    for (entryPtr=globalTablePtr->buckets[i]; entryPtr!=NULL ;
		    entryPtr=entryPtr->nextPtr) {
		if ((entryPtr == globalPtr) && (entryPtr->objPtr == objPtr)) {
		    found = 1;
		}
	    }
	}
	if (!found) {
	    Tcl_Panic("%s: literal \"%.*s\" wasn't global",
		    "TclRegisterLiteral", (length>60? 60 : length), bytes);
	}
    }
#endif /*TCL_COMPILE_DEBUG*/

#ifdef TCL_COMPILE_STATS
    iPtr->stats.numLiteralsCreated++;
    iPtr->stats.totalLitStringBytes += (double) (length + 1);
    iPtr->stats.currentLitStringBytes += (double) (length + 1);
    iPtr->stats.literalCount[TclLog2(length)]++;
#endif /*TCL_COMPILE_STATS*/

    if (globalPtrPtr) {
	*globalPtrPtr = globalPtr;
    }
    *newPtr = 1;
    return objPtr;
}

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclLiteral.c`, function `TclRegisterLiteral`, lines 381–469. Retained reference `tcl8.6-2`.

```text
TclRegisterLiteral(
    void *ePtr,		/* Points to the CompileEnv in whose object
				 * array an object is found or created. */
    char *bytes,	/* Points to string for which to find or
				 * create an object in CompileEnv's object
				 * array. */
    int length,			/* Number of bytes in the string. If < 0, the
				 * string consists of all bytes up to the
				 * first null character. */
    int flags)			/* If LITERAL_ON_HEAP then the caller already
				 * malloc'd bytes and ownership is passed to
				 * this function. If LITERAL_CMD_NAME then
				 * the literal should not be shared across
				 * namespaces. */
{
    CompileEnv *envPtr = (CompileEnv *)ePtr;
    Interp *iPtr = envPtr->iPtr;
    LiteralTable *localTablePtr = &envPtr->localLitTable;
    LiteralEntry *globalPtr, *localPtr;
    Tcl_Obj *objPtr;
    unsigned hash;
    int localHash, objIndex, isNew;
    Namespace *nsPtr;

    if (length < 0) {
	length = (bytes ? strlen(bytes) : 0);
    }
    hash = HashString(bytes, length);

    /*
     * Is the literal already in the CompileEnv's local literal array? If so,
     * just return its index.
     */

    localHash = (hash & localTablePtr->mask);
    for (localPtr=localTablePtr->buckets[localHash] ; localPtr!=NULL;
	    localPtr = localPtr->nextPtr) {
	objPtr = localPtr->objPtr;
	if ((objPtr->length == length) && ((length == 0)
		|| ((objPtr->bytes[0] == bytes[0])
		&& (memcmp(objPtr->bytes, bytes, length) == 0)))) {
	    if ((flags & LITERAL_ON_HEAP)) {
		ckfree(bytes);
	    }
	    objIndex = (localPtr - envPtr->literalArrayPtr);
#ifdef TCL_COMPILE_DEBUG
	    TclVerifyLocalLiteralTable(envPtr);
#endif /*TCL_COMPILE_DEBUG*/

	    return objIndex;
	}
    }

    /*
     * The literal is new to this CompileEnv. If it is a command name, avoid
     * sharing it across namespaces, and try not to share it with non-cmd
     * literals. Note that FQ command names can be shared, so that we register
     * the namespace as the interp's global NS.
     */

    if ((flags & LITERAL_CMD_NAME)) {
	if ((length >= 2) && (bytes[0] == ':') && (bytes[1] == ':')) {
	    nsPtr = iPtr->globalNsPtr;
	} else {
	    nsPtr = iPtr->varFramePtr->nsPtr;
	}
    } else {
	nsPtr = NULL;
    }

    /*
     * Is it in the interpreter's global literal table? If not, create it.
     */

    globalPtr = NULL;
    objPtr = TclCreateLiteral(iPtr, bytes, length, hash, &isNew, nsPtr, flags,
	    &globalPtr);
    objIndex = AddLocalLiteralEntry(envPtr, objPtr, localHash);

#ifdef TCL_COMPILE_DEBUG
    if (globalPtr != NULL && globalPtr->refCount < 1) {
	Tcl_Panic("%s: global literal \"%.*s\" had bad refCount %d",
		"TclRegisterLiteral", (length>60? 60 : length), bytes,
		globalPtr->refCount);
    }
    TclVerifyLocalLiteralTable(envPtr);
#endif /*TCL_COMPILE_DEBUG*/
    return objIndex;
}

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclLiteral.c`, function `TclReleaseLiteral`, lines 807–870. Retained reference `tcl8.6-3`.

```text
TclReleaseLiteral(
    Tcl_Interp *interp,		/* Interpreter for which objPtr was created to
				 * hold a literal. */
    Tcl_Obj *objPtr)	/* Points to a literal object that was
				 * previously created by a call to
				 * TclRegisterLiteral. */
{
    Interp *iPtr = (Interp *) interp;
    LiteralTable *globalTablePtr;
    LiteralEntry *entryPtr, *prevPtr;
    const char *bytes;
    int length, index;

    if (iPtr == NULL) {
	goto done;
    }

    globalTablePtr = &iPtr->literalTable;
    bytes = TclGetStringFromObj(objPtr, &length);
    index = (HashString(bytes, length) & globalTablePtr->mask);

    /*
     * Check to see if the object is in the global literal table and remove
     * this reference. The object may not be in the table if it is a hidden
     * local literal.
     */

    for (prevPtr=NULL, entryPtr=globalTablePtr->buckets[index];
	    entryPtr!=NULL ; prevPtr=entryPtr, entryPtr=entryPtr->nextPtr) {
	if (entryPtr->objPtr == objPtr) {
	    entryPtr->refCount--;

	    /*
	     * If the literal is no longer being used by any ByteCode, delete
	     * the entry then remove the reference corresponding to the global
	     * literal table entry (decrement the ref count of the object).
	     */

	    if (entryPtr->refCount == 0) {
		if (prevPtr == NULL) {
		    globalTablePtr->buckets[index] = entryPtr->nextPtr;
		} else {
		    prevPtr->nextPtr = entryPtr->nextPtr;
		}
		ckfree(entryPtr);
		globalTablePtr->numEntries--;

		TclDecrRefCount(objPtr);

#ifdef TCL_COMPILE_STATS
		iPtr->stats.currentLitStringBytes -= (double) (length + 1);
#endif /*TCL_COMPILE_STATS*/
	    }
	    break;
	}
    }

    /*
     * Remove the reference corresponding to the local literal table entry.
     */

    done:
    Tcl_DecrRefCount(objPtr);
}

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCompile.h`, function `TclRegisterNewCmdLiteral`, lines 1249–1250. Retained reference `tcl8.6-4`.

```text
#define TclRegisterNewCmdLiteral(envPtr, bytes, length) \
    TclRegisterLiteral(envPtr, (char *)(bytes), length, LITERAL_CMD_NAME)

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCompExpr.c`, function `CompileExprTree`, lines 2335–2360. Retained reference `tcl9.0-0`.

```text
	    case FUNCTION: {
		Tcl_DString cmdName;
		const char *p;
		Tcl_Size length;

		Tcl_DStringInit(&cmdName);
		TclDStringAppendLiteral(&cmdName, "tcl::mathfunc::");
		p = TclGetStringFromObj(*funcObjv, &length);
		funcObjv++;
		Tcl_DStringAppend(&cmdName, p, length);
		TclEmitPush(TclRegisterLiteral(envPtr,
			Tcl_DStringValue(&cmdName),
			Tcl_DStringLength(&cmdName), LITERAL_CMD_NAME), envPtr);
		Tcl_DStringFree(&cmdName);

		/*
		 * Start a count of the number of words in this function
		 * command invocation. In case there's already a count in
		 * progress (nested functions), save it in our unused "left"
		 * field for restoring later.
		 */

		nodePtr->left = numWords;
		numWords = 2;	/* Command plus one argument */
		break;
	    }

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclLiteral.c`, function `TclCreateLiteral`, lines 177–334. Retained reference `tcl9.0-1`.

```text
TclCreateLiteral(
    Interp *iPtr,
    const char *bytes,	/* The start of the string. Note that this is
				 * not a NUL-terminated string. */
    Tcl_Size length,	/* Number of bytes in the string. */
    size_t hash, /* The string's hash. If the value is
					 * TCL_INDEX_NONE, it will be computed here. */
    int *newPtr,
    Namespace *nsPtr,
    int flags,
    LiteralEntry **globalPtrPtr)
{
    LiteralTable *globalTablePtr = &iPtr->literalTable;
    LiteralEntry *globalPtr;
    size_t globalHash;
    Tcl_Obj *objPtr;

    /*
     * Is it in the interpreter's global literal table?
     */

    if (hash == (size_t) TCL_INDEX_NONE) {
	hash = HashString(bytes, length);
    }
    globalHash = (hash & globalTablePtr->mask);
    for (globalPtr=globalTablePtr->buckets[globalHash] ; globalPtr!=NULL;
	    globalPtr = globalPtr->nextPtr) {
	objPtr = globalPtr->objPtr;
	if (globalPtr->nsPtr == nsPtr) {
	    /*
	     * Literals should always have UTF-8 representations... but this
	     * is not guaranteed so we need to be careful anyway.
	     *
	     * https://stackoverflow.com/q/54337750/301832
	     */

	    Tcl_Size objLength;
	    const char *objBytes = TclGetStringFromObj(objPtr, &objLength);

	    if ((objLength == length) && ((length == 0)
		    || ((objBytes[0] == bytes[0])
		    && (memcmp(objBytes, bytes, length) == 0)))) {
		/*
		 * A literal was found: return it
		 */

		if (newPtr) {
		    *newPtr = 0;
		}
		if (globalPtrPtr) {
		    *globalPtrPtr = globalPtr;
		}
		if (flags & LITERAL_ON_HEAP) {
		    Tcl_Free((void *)bytes);
		}
		if (globalPtr->refCount != TCL_INDEX_NONE) {
		    globalPtr->refCount++;
		}
		return objPtr;
	    }
	}
    }
    if (!newPtr) {
	if ((flags & LITERAL_ON_HEAP)) {
	    Tcl_Free((void *)bytes);
	}
	return NULL;
    }

    /*
     * The literal is new to the interpreter.
     */

    TclNewObj(objPtr);
    if ((flags & LITERAL_ON_HEAP)) {
	objPtr->bytes = (char *) bytes;
	objPtr->length = length;
    } else {
	TclInitStringRep(objPtr, bytes, length);
    }

    /* Should the new literal be shared globally? */

    if ((flags & LITERAL_UNSHARED)) {
	/*
	 * No, do *not* add it the global literal table
	 * Make clear, that no global value is returned
	 */
	if (globalPtrPtr != NULL) {
	    *globalPtrPtr = NULL;
	}
	return objPtr;
    }

    /*
     * Yes, add it to the global literal table.
     */
#ifdef TCL_COMPILE_DEBUG
    if (LookupLiteralEntry((Tcl_Interp *) iPtr, objPtr) != NULL) {
	Tcl_Panic("%s: literal \"%.*s\" found globally but shouldn't be",
		"TclRegisterLiteral", (length>60? 60 : (int)length), bytes);
    }
#endif

    globalPtr = (LiteralEntry *)Tcl_Alloc(sizeof(LiteralEntry));
    globalPtr->objPtr = objPtr;
    Tcl_IncrRefCount(objPtr);
    globalPtr->refCount = 1;
    globalPtr->nsPtr = nsPtr;
    globalPtr->nextPtr = globalTablePtr->buckets[globalHash];
    globalTablePtr->buckets[globalHash] = globalPtr;
    globalTablePtr->numEntries++;

    /*
     * If the global literal table has exceeded a decent size, rebuild it with
     * more buckets.
     */

    if (globalTablePtr->numEntries >= globalTablePtr->rebuildSize) {
	RebuildLiteralTable(globalTablePtr);
    }

#ifdef TCL_COMPILE_DEBUG
    TclVerifyGlobalLiteralTable(iPtr);
    {
	LiteralEntry *entryPtr;
	int found;
	size_t i;

	found = 0;
	for (i=0 ; i<globalTablePtr->numBuckets ; i++) {
	    for (entryPtr=globalTablePtr->buckets[i]; entryPtr!=NULL ;
		    entryPtr=entryPtr->nextPtr) {
		if ((entryPtr == globalPtr) && (entryPtr->objPtr == objPtr)) {
		    found = 1;
		}
	    }
	}
	if (!found) {
	    Tcl_Panic("%s: literal \"%.*s\" wasn't global",
		    "TclRegisterLiteral", (length>60? 60 : (int)length), bytes);
	}
    }
#endif /*TCL_COMPILE_DEBUG*/

#ifdef TCL_COMPILE_STATS
    iPtr->stats.numLiteralsCreated++;
    iPtr->stats.totalLitStringBytes += (double) (length + 1);
    iPtr->stats.currentLitStringBytes += (double) (length + 1);
    iPtr->stats.literalCount[TclLog2(length)]++;
#endif /*TCL_COMPILE_STATS*/

    if (globalPtrPtr) {
	*globalPtrPtr = globalPtr;
    }
    *newPtr = 1;
    return objPtr;
}

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclLiteral.c`, function `TclRegisterLiteral`, lines 391–486. Retained reference `tcl9.0-2`.

```text
TclRegisterLiteral(
    void *ePtr,		/* Points to the CompileEnv in whose object
				 * array an object is found or created. */
    const char *bytes,	/* Points to string for which to find or
				 * create an object in CompileEnv's object
				 * array. */
    Tcl_Size length,			/* Number of bytes in the string. If -1, the
				 * string consists of all bytes up to the
				 * first null character. */
    int flags)			/* If LITERAL_ON_HEAP then the caller already
				 * malloc'd bytes and ownership is passed to
				 * this function. If LITERAL_CMD_NAME then
				 * the literal should not be shared across
				 * namespaces. */
{
    CompileEnv *envPtr = (CompileEnv *)ePtr;
    Interp *iPtr = envPtr->iPtr;
    LiteralTable *localTablePtr = &envPtr->localLitTable;
    LiteralEntry *globalPtr, *localPtr;
    Tcl_Obj *objPtr;
    size_t hash, localHash, objIndex;
    int isNew;
    Namespace *nsPtr;

    if (length < 0) {
	length = (bytes ? strlen(bytes) : 0);
    }
    hash = HashString(bytes, length);

    /*
     * Is the literal already in the CompileEnv's local literal array? If so,
     * just return its index.
     */

    localHash = (hash & localTablePtr->mask);
    for (localPtr=localTablePtr->buckets[localHash] ; localPtr!=NULL;
	    localPtr = localPtr->nextPtr) {
	objPtr = localPtr->objPtr;
	if ((objPtr->length == length) && ((length == 0)
		|| ((objPtr->bytes[0] == bytes[0])
		&& (memcmp(objPtr->bytes, bytes, length) == 0)))) {
	    if ((flags & LITERAL_ON_HEAP)) {
		Tcl_Free((void *)bytes);
	    }
	    objIndex = (localPtr - envPtr->literalArrayPtr);
#ifdef TCL_COMPILE_DEBUG
	    TclVerifyLocalLiteralTable(envPtr);
#endif /*TCL_COMPILE_DEBUG*/

	    if (objIndex > INT_MAX) {
		Tcl_Panic("Literal table index too large. Cannot be handled by TclEmitPush");
	    }
	    return objIndex;
	}
    }

    /*
     * The literal is new to this CompileEnv. If it is a command name, avoid
     * sharing it across namespaces, and try not to share it with non-cmd
     * literals. Note that FQ command names can be shared, so that we register
     * the namespace as the interp's global NS.
     */

    if ((flags & LITERAL_CMD_NAME)) {
	if ((length >= 2) && (bytes[0] == ':') && (bytes[1] == ':')) {
	    nsPtr = iPtr->globalNsPtr;
	} else {
	    nsPtr = iPtr->varFramePtr->nsPtr;
	}
    } else {
	nsPtr = NULL;
    }

    /*
     * Is it in the interpreter's global literal table? If not, create it.
     */

    globalPtr = NULL;
    objPtr = TclCreateLiteral(iPtr, bytes, length, hash, &isNew, nsPtr, flags,
	    &globalPtr);
    objIndex = AddLocalLiteralEntry(envPtr, objPtr, localHash);

#ifdef TCL_COMPILE_DEBUG
    if (globalPtr != NULL && (globalPtr->refCount + 1 < 2)) {
	Tcl_Panic("%s: global literal \"%.*s\" had bad refCount %" TCL_Z_MODIFIER "u",
		"TclRegisterLiteral", (length>60? 60 : (int)length), bytes,
		globalPtr->refCount);
    }
    TclVerifyLocalLiteralTable(envPtr);
#endif /*TCL_COMPILE_DEBUG*/
    if (objIndex > INT_MAX) {
	Tcl_Panic(
	    "Literal table index too large. Cannot be handled by TclEmitPush");
    }
    return objIndex;
}

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclLiteral.c`, function `TclReleaseLiteral`, lines 831–893. Retained reference `tcl9.0-3`.

```text
TclReleaseLiteral(
    Tcl_Interp *interp,		/* Interpreter for which objPtr was created to
				 * hold a literal. */
    Tcl_Obj *objPtr)	/* Points to a literal object that was
				 * previously created by a call to
				 * TclRegisterLiteral. */
{
    Interp *iPtr = (Interp *) interp;
    LiteralTable *globalTablePtr;
    LiteralEntry *entryPtr, *prevPtr;
    const char *bytes;
    size_t index;
    Tcl_Size length;

    if (iPtr == NULL) {
	goto done;
    }

    globalTablePtr = &iPtr->literalTable;
    bytes = TclGetStringFromObj(objPtr, &length);
    index = HashString(bytes, length) & globalTablePtr->mask;

    /*
     * Check to see if the object is in the global literal table and remove
     * this reference. The object may not be in the table if it is a hidden
     * local literal.
     */

    for (prevPtr=NULL, entryPtr=globalTablePtr->buckets[index];
	    entryPtr!=NULL ; prevPtr=entryPtr, entryPtr=entryPtr->nextPtr) {
	if (entryPtr->objPtr == objPtr) {
	    /*
	     * If the literal is no longer being used by any ByteCode, delete
	     * the entry then remove the reference corresponding to the global
	     * literal table entry (decrement the ref count of the object).
	     */

	    if ((entryPtr->refCount != TCL_INDEX_NONE) && (entryPtr->refCount-- <= 1)) {
		if (prevPtr == NULL) {
		    globalTablePtr->buckets[index] = entryPtr->nextPtr;
		} else {
		    prevPtr->nextPtr = entryPtr->nextPtr;
		}
		Tcl_Free(entryPtr);
		globalTablePtr->numEntries--;

		TclDecrRefCount(objPtr);

#ifdef TCL_COMPILE_STATS
		iPtr->stats.currentLitStringBytes -= (double) (length + 1);
#endif /*TCL_COMPILE_STATS*/
	    }
	    break;
	}
    }

    /*
     * Remove the reference corresponding to the local literal table entry.
     */

    done:
    Tcl_DecrRefCount(objPtr);
}

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCompExpr.c`, function `CompileExprTree`, lines 2343–2361. Retained reference `tcl9.1-0`.

```text
	    case FUNCTION: {
		Tcl_Obj *cmdName;

		TclNewLiteralStringObj(cmdName, "tcl::mathfunc::");
		Tcl_AppendObjToObj(cmdName, *funcObjv);
		funcObjv++;
		PUSH_OBJ_FLAGS(cmdName, LITERAL_CMD_NAME);

		/*
		 * Start a count of the number of words in this function
		 * command invocation. In case there's already a count in
		 * progress (nested functions), save it in our unused "left"
		 * field for restoring later.
		 */

		nodePtr->left = numWords;
		numWords = 2;	/* Command plus one argument */
		break;
	    }

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclLiteral.c`, function `TclCreateLiteral`, lines 176–334. Retained reference `tcl9.1-1`.

```text
TclCreateLiteral(
    Interp *iPtr,
    const char *bytes,		/* The start of the string. Note that this is
				 * not a NUL-terminated string. */
    Tcl_Size length,		/* Number of bytes in the string. */
    size_t hash,		/* The string's hash. If the value is
				 * TCL_INDEX_NONE, it will be computed here. */
    int *newPtr,
    Namespace *nsPtr,
    int flags,
    LiteralEntry **globalPtrPtr)
{
    LiteralTable *globalTablePtr = &iPtr->literalTable;
    LiteralEntry *globalPtr;
    size_t globalHash;
    Tcl_Obj *objPtr;

    /*
     * Is it in the interpreter's global literal table?
     */

    if (hash == (size_t) TCL_INDEX_NONE) {
	hash = HashString(bytes, length);
    }
    globalHash = (hash & globalTablePtr->mask);
    for (globalPtr=globalTablePtr->buckets[globalHash] ; globalPtr!=NULL;
	    globalPtr = globalPtr->nextPtr) {
	objPtr = globalPtr->objPtr;
	if (globalPtr->nsPtr == nsPtr) {
	    /*
	     * Literals should always have UTF-8 representations... but this
	     * is not guaranteed so we need to be careful anyway.
	     *
	     * https://stackoverflow.com/q/54337750/301832
	     */

	    Tcl_Size objLength;
	    const char *objBytes = TclGetStringFromObj(objPtr, &objLength);

	    if ((objLength == length) && ((length == 0)
		    || ((objBytes[0] == bytes[0])
		    && (memcmp(objBytes, bytes, length) == 0)))) {
		/*
		 * A literal was found: return it
		 */

		if (newPtr) {
		    *newPtr = 0;
		}
		if (globalPtrPtr) {
		    *globalPtrPtr = globalPtr;
		}
		if (flags & LITERAL_ON_HEAP) {
		    Tcl_Free((void *)bytes);
		}
		if (globalPtr->refCount != TCL_INDEX_NONE) {
		    globalPtr->refCount++;
		}
		return objPtr;
	    }
	}
    }
    if (!newPtr) {
	if ((flags & LITERAL_ON_HEAP)) {
	    Tcl_Free((void *)bytes);
	}
	return NULL;
    }

    /*
     * The literal is new to the interpreter.
     */

    TclNewObj(objPtr);
    if ((flags & LITERAL_ON_HEAP)) {
	objPtr->bytes = (char *) bytes;
	objPtr->length = length;
    } else if (!TclAttemptInitStringRep(objPtr, bytes, length)) {
	Tcl_DecrRefCount(objPtr);
	return NULL;
    }

    /* Should the new literal be shared globally? */

    if ((flags & LITERAL_UNSHARED)) {
	/*
	 * No, do *not* add it the global literal table
	 * Make clear, that no global value is returned
	 */
	if (globalPtrPtr != NULL) {
	    *globalPtrPtr = NULL;
	}
	return objPtr;
    }

    /*
     * Yes, add it to the global literal table.
     */
#ifdef TCL_COMPILE_DEBUG
    if (LookupLiteralEntry((Tcl_Interp *) iPtr, objPtr) != NULL) {
	Tcl_Panic("%s: literal \"%.*s\" found globally but shouldn't be",
		"TclRegisterLiteral", (length>60? 60 : (int)length), bytes);
    }
#endif

    globalPtr = (LiteralEntry *)Tcl_Alloc(sizeof(LiteralEntry));
    globalPtr->objPtr = objPtr;
    Tcl_IncrRefCount(objPtr);
    globalPtr->refCount = 1;
    globalPtr->nsPtr = nsPtr;
    globalPtr->nextPtr = globalTablePtr->buckets[globalHash];
    globalTablePtr->buckets[globalHash] = globalPtr;
    globalTablePtr->numEntries++;

    /*
     * If the global literal table has exceeded a decent size, rebuild it with
     * more buckets.
     */

    if (globalTablePtr->numEntries >= globalTablePtr->rebuildSize) {
	RebuildLiteralTable(globalTablePtr);
    }

#ifdef TCL_COMPILE_DEBUG
    TclVerifyGlobalLiteralTable(iPtr);
    {
	LiteralEntry *entryPtr;
	int found;
	size_t i;

	found = 0;
	for (i=0 ; i<globalTablePtr->numBuckets ; i++) {
	    for (entryPtr=globalTablePtr->buckets[i]; entryPtr!=NULL ;
		    entryPtr=entryPtr->nextPtr) {
		if ((entryPtr == globalPtr) && (entryPtr->objPtr == objPtr)) {
		    found = 1;
		}
	    }
	}
	if (!found) {
	    Tcl_Panic("%s: literal \"%.*s\" wasn't global",
		    "TclRegisterLiteral", (length>60? 60 : (int)length), bytes);
	}
    }
#endif /*TCL_COMPILE_DEBUG*/

#ifdef TCL_COMPILE_STATS
    iPtr->stats.numLiteralsCreated++;
    iPtr->stats.totalLitStringBytes += (double) (length + 1);
    iPtr->stats.currentLitStringBytes += (double) (length + 1);
    iPtr->stats.literalCount[TclLog2(length)]++;
#endif /*TCL_COMPILE_STATS*/

    if (globalPtrPtr) {
	*globalPtrPtr = globalPtr;
    }
    *newPtr = 1;
    return objPtr;
}

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclLiteral.c`, function `TclRegisterLiteral`, lines 391–486. Retained reference `tcl9.1-2`.

```text
TclRegisterLiteral(
    void *ePtr,			/* Points to the CompileEnv in whose object
				 * array an object is found or created. */
    const char *bytes,		/* Points to string for which to find or
				 * create an object in CompileEnv's object
				 * array. */
    Tcl_Size length,		/* Number of bytes in the string. If -1, the
				 * string consists of all bytes up to the
				 * first null character. */
    int flags)			/* If LITERAL_ON_HEAP then the caller already
				 * malloc'd bytes and ownership is passed to
				 * this function. If LITERAL_CMD_NAME then
				 * the literal should not be shared across
				 * namespaces. */
{
    CompileEnv *envPtr = (CompileEnv *)ePtr;
    Interp *iPtr = envPtr->iPtr;
    LiteralTable *localTablePtr = &envPtr->localLitTable;
    LiteralEntry *globalPtr, *localPtr;
    Tcl_Obj *objPtr;
    size_t hash, localHash, objIndex;
    int isNew;
    Namespace *nsPtr;

    if (length < 0) {
	length = (bytes ? strlen(bytes) : 0);
    }
    hash = HashString(bytes, length);

    /*
     * Is the literal already in the CompileEnv's local literal array? If so,
     * just return its index.
     */

    localHash = (hash & localTablePtr->mask);
    for (localPtr=localTablePtr->buckets[localHash] ; localPtr!=NULL;
	    localPtr = localPtr->nextPtr) {
	objPtr = localPtr->objPtr;
	if ((objPtr->length == length) && ((length == 0)
		|| ((objPtr->bytes[0] == bytes[0])
		&& (memcmp(objPtr->bytes, bytes, length) == 0)))) {
	    if ((flags & LITERAL_ON_HEAP)) {
		Tcl_Free((void *)bytes);
	    }
	    objIndex = (localPtr - envPtr->literalArrayPtr);
#ifdef TCL_COMPILE_DEBUG
	    TclVerifyLocalLiteralTable(envPtr);
#endif /*TCL_COMPILE_DEBUG*/

	    if (objIndex > INT_MAX) {
		Tcl_Panic("Literal table index too large. Cannot be handled by TclEmitPush");
	    }
	    return (int)objIndex;
	}
    }

    /*
     * The literal is new to this CompileEnv. If it is a command name, avoid
     * sharing it across namespaces, and try not to share it with non-cmd
     * literals. Note that FQ command names can be shared, so that we register
     * the namespace as the interp's global NS.
     */

    if ((flags & LITERAL_CMD_NAME)) {
	if ((length >= 2) && (bytes[0] == ':') && (bytes[1] == ':')) {
	    nsPtr = iPtr->globalNsPtr;
	} else {
	    nsPtr = iPtr->varFramePtr->nsPtr;
	}
    } else {
	nsPtr = NULL;
    }

    /*
     * Is it in the interpreter's global literal table? If not, create it.
     */

    globalPtr = NULL;
    objPtr = TclCreateLiteral(iPtr, bytes, length, hash, &isNew, nsPtr, flags,
	    &globalPtr);
    objIndex = AddLocalLiteralEntry(envPtr, objPtr, localHash);

#ifdef TCL_COMPILE_DEBUG
    if (globalPtr != NULL && (globalPtr->refCount + 1 < 2)) {
	Tcl_Panic("%s: global literal \"%.*s\" had bad refCount %" TCL_Z_MODIFIER "u",
		"TclRegisterLiteral", (length>60? 60 : (int)length), bytes,
		globalPtr->refCount);
    }
    TclVerifyLocalLiteralTable(envPtr);
#endif /*TCL_COMPILE_DEBUG*/
    if (objIndex > INT_MAX) {
	Tcl_Panic("Literal table index too large. "
		"Cannot be handled by TclEmitPush");
    }
    return (int)objIndex;
}

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclLiteral.c`, function `TclReleaseLiteral`, lines 873–935. Retained reference `tcl9.1-3`.

```text
TclReleaseLiteral(
    Tcl_Interp *interp,		/* Interpreter for which objPtr was created to
				 * hold a literal. */
    Tcl_Obj *objPtr)		/* Points to a literal object that was
				 * previously created by a call to
				 * TclRegisterLiteral. */
{
    Interp *iPtr = (Interp *) interp;
    LiteralTable *globalTablePtr;
    LiteralEntry *entryPtr, *prevPtr;
    const char *bytes;
    size_t index;
    Tcl_Size length;

    if (iPtr == NULL) {
	goto done;
    }

    globalTablePtr = &iPtr->literalTable;
    bytes = TclGetStringFromObj(objPtr, &length);
    index = HashString(bytes, length) & globalTablePtr->mask;

    /*
     * Check to see if the object is in the global literal table and remove
     * this reference. The object may not be in the table if it is a hidden
     * local literal.
     */

    for (prevPtr=NULL, entryPtr=globalTablePtr->buckets[index];
	    entryPtr!=NULL ; prevPtr=entryPtr, entryPtr=entryPtr->nextPtr) {
	if (entryPtr->objPtr == objPtr) {
	    /*
	     * If the literal is no longer being used by any ByteCode, delete
	     * the entry then remove the reference corresponding to the global
	     * literal table entry (decrement the ref count of the object).
	     */

	    if ((entryPtr->refCount != TCL_INDEX_NONE) && (entryPtr->refCount-- <= 1)) {
		if (prevPtr == NULL) {
		    globalTablePtr->buckets[index] = entryPtr->nextPtr;
		} else {
		    prevPtr->nextPtr = entryPtr->nextPtr;
		}
		Tcl_Free(entryPtr);
		globalTablePtr->numEntries--;

		TclDecrRefCount(objPtr);

#ifdef TCL_COMPILE_STATS
		iPtr->stats.currentLitStringBytes -= (double) (length + 1);
#endif /*TCL_COMPILE_STATS*/
	    }
	    break;
	}
    }

    /*
     * Remove the reference corresponding to the local literal table entry.
     */

    done:
    Tcl_DecrRefCount(objPtr);
}

```

## Consumer bindings

- [rust/tcl-registry/src/native_expression_program.rs](../../../../rust/tcl-registry/src/native_expression_program.rs), `NativeExpressionProgram::compiler_name_effects`: Select source-only checked expression compiler name operations, separately from runtime function dispatch.
- [rust/tcl-compiler/src/command_binding/original_compiler_effects.rs](../../../../rust/tcl-compiler/src/command_binding/original_compiler_effects.rs), `expression_compiler_names_preserved`: Require current ordinary literal-pool and quiet lookup effects before modern function-literal registration.
- [rust/tcl-compiler/src/command_binding/original_compiler_effects.rs](../../../../rust/tcl-compiler/src/command_binding/original_compiler_effects.rs), `append_visits`: Preserve exact recursive compilation visits for original argument bracket bodies without minting native admission.
- [rust/tcl-registry/src/native_expression_program.rs](../../../../rust/tcl-registry/src/native_expression_program.rs), `native_expression_program::tests::compiler_function_name_effects_are_selected_without_runtime_dispatch` (linked): Original checked C5 expression programs select call-free versus modern literal-registration purposes; Raw, foreign context and Jim calls decline. This is an implementation obligation, not an executed result.
- [rust/tcl-compiler/src/command_binding/original_compiler_effects.rs](../../../../rust/tcl-compiler/src/command_binding/original_compiler_effects.rs), `command_binding::original_compiler_effects::tests::function_name_registration_requires_original_pool_and_quiet_lookup` (linked): Current authored pool permits only name-effect recipe; unavailable pool, unknown entry or retained execution observer declines. No dispatch/Normal/native admission is issued.
- [rust/tcl-compiler/src/command_binding/original_compiler_effects.rs](../../../../rust/tcl-compiler/src/command_binding/original_compiler_effects.rs), `command_binding::original_compiler_effects::tests::function_argument_compilation_keeps_original_nested_script_visits` (linked): Argument brackets retain exact native image/config/body spans in recursive compiler visits; unavailable pool withdraws.

## Exact original-source Compiler software outcomes

The table reports only admitted selectors attached to this question by exact comments in the retained original test bodies. Each result is limited to its named source snapshot, owned Compiler image and selector invocation. No other source/image, unselected assertion, external-provider observation or performance conclusion follows.

| Original operation | Passed | Failed | Incomplete |
| --- | ---: | ---: | ---: |
| `integration-compiler-focused-consumer-tests344` | 1 | 0 | 0 |

Exact original-source body hashes, receipts, streams, inventories and owned images preserve the per-selector events and independent producer/admission limits. The existing provider answers retain their own experimental scope.

## Exact original-source347 software outcomes

These results belong only to the retained source347 body/question pairs and their owning Compiler or Core image. They do not establish an outcome for current source, unselected controls or external providers.

| Original operation | Passed | Failed |
| --- | ---: | ---: |
| `integration-compiler-focused-consumer-tests347` | 1 | 0 |

`Exact original body hashes, producer/list/admission receipts, raw logs and owned images` retain the finite per-selector results and independent failed Runtime producer.
