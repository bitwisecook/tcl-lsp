# naming.tcloo.declared-variable-compiled-and-runtime-resolver

Kind: `native-observation`

## Problem statement

A counted TclOO variable declaration can retain bytes after raw zero while a method runtime name lookup reaches a CString resolver. Eagerly linking every declared full name makes dynamic access appear equivalent to a genuine compiler-selected local primary and can redirect writes into object storage. A successful declaration or a clipped variable presenter also cannot prove a compiled slot. The controls distinguish compiled and dynamic accesses to one declaring class and include a prefix-only negative and qualification after zero; they do not establish other object-private or compiler/header capabilities.

## Question

Do original counted declared member names resolve identically through literal compiled method accesses and dynamic set $target accesses, including raw zero, qualification after zero, surrogate and raw ff names?

## Conclusion

C8.6/C9.0/C9.1 literal compiled accesses to a matching raw-zero declaration read/write object STATIC, while dynamic full-name writes create ordinary locals, leave STATIC unchanged and fresh dynamic reads fail. Plain, surrogate and raw ff declarations resolve through both routes. A prefix-only declaration does not link the full raw-zero compiled name: its static reads fail, while a dynamic read after the dynamic write returns DYNAMIC. Qualification after zero is accepted by declaration, but the retained static and dynamic read controls fail and report no linked object variable. C8.6 info object vars clips the raw-zero presenter, whereas C9 retains it; that output alone does not identify the stored key. C8.4/C8.5/Jim lack OO in the observed command surface. No cache, original header, native local table or Rust execution authority is supplied.

## Scope

Exact public original Tcl_NewStringObj(pointer,count)/Tcl_EvalObjv declaration, formal-list, method-body and dynamic name operands. ASCII setup via Tcl_Eval; compiled literal method bodies contain actual raw bytes, not Unicode/source escapes. Six names/negative arrangements and static/dynamic/read/object-vars chronology, fresh provider interpreters. Jim only executes startup/OO-availability source. Source windows explain full counted compiled matching versus runtime strlen; a caller still needs an independent actual compiler primary/context/lifetime owner.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Actual executable SHA-256 0e17bb07b75c45530bb681665917eb4d23e27a004b0c93149fc395b16ce5fdae; public header/static library/Makefile/source-owner/compile command/process streams retained.. Channel: Original counted Tcl objects passed to Tcl_EvalObjv; body source contains actual raw native bytes, ASCII setup via NUL-terminated Tcl_Eval.. Dialect: Tcl.

Actual OO command query is empty; no TclOO member vector executes.

### tcl8.5

Status: `unsupported`. Version: 8.5.19. Build: Actual executable SHA-256 033288f5975075c036b506dcded54ba2a02aa99c8f0c241b1b427941a24f3ecc; public header/static library/Makefile/source-owner/compile command/process streams retained.. Channel: Original counted Tcl objects passed to Tcl_EvalObjv; body source contains actual raw native bytes, ASCII setup via NUL-terminated Tcl_Eval.. Dialect: Tcl.

Actual OO command query is empty; no TclOO member vector executes.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable SHA-256 8385b7c911059e6de2176e9c9974a715c8fdba90e3e364209898e21039e7ea8e; public header/static library/Makefile/source-owner/compile command/process streams retained.. Channel: Original counted Tcl objects passed to Tcl_EvalObjv; body source contains actual raw native bytes, ASCII setup via NUL-terminated Tcl_Eval.. Dialect: Tcl.

Matching raw-zero static object access stays STATIC after dynamic write; fresh dynamicread fails. Plain/FF/surrogate static reader changes to DYNAMIC after dynamic write. Prefix-only declaration and qualification-after-zero source controls do not link the compiled full name. The raw-zero object-vars presenter clips at zero.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable SHA-256 6f36c8cece0fea2bb47b8f5d531c0a3ff08d762619492a96131f748f56b72126; public header/static library/Makefile/source-owner/compile command/process streams retained.. Channel: Original counted Tcl objects passed to Tcl_EvalObjv; body source contains actual raw native bytes, ASCII setup via NUL-terminated Tcl_Eval.. Dialect: Tcl.

Matching raw-zero static object access stays STATIC after dynamic write; fresh dynamicread fails. Plain/FF/surrogate static reader changes to DYNAMIC after dynamic write. Prefix-only declaration and qualification-after-zero source controls do not link the compiled full name. The raw-zero object-vars presenter retains all counted bytes.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable SHA-256 b19ef9a53f24b8add9ef25219ecc9cffaa2bd9c3fa7e23a1b6f34ed061e0e59f; public header/static library/Makefile/source-owner/compile command/process streams retained.. Channel: Original counted Tcl objects passed to Tcl_EvalObjv; body source contains actual raw native bytes, ASCII setup via NUL-terminated Tcl_Eval.. Dialect: Tcl.

Matching raw-zero static object access stays STATIC after dynamic write; fresh dynamicread fails. Plain/FF/surrogate static reader changes to DYNAMIC after dynamic write. Prefix-only declaration and qualification-after-zero source controls do not link the compiled full name. The raw-zero object-vars presenter retains all counted bytes.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual executable SHA-256 39e4c0d090e5c0a76672b61b99510fdd3974cbf4eaadca10da0c5ba8ecd43117; public header/static library/Makefile/source-owner/compile command/process streams retained.. Channel: ASCII NUL-terminated Jim_Eval startup and OO-availability query only.. Dialect: Jim Tcl.

Actual OO command query is empty; no TclOO member vector executes.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation for this exact counted TclOO resolver question.

## Exact evidence

- `input` (input): [rust/tcl-vm/tests/data/native_oo_variable_resolver/probe.c](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/probe.c). SHA-256 `cf1bbe6957a7cf37051612c012d3cd2734cd2ffa3bd70e568bca06fc6119888e`. Exact original counted declaration/body/name arrays or complete actual six-provider compilation/process correspondence.
- `aggregate` (provider): [rust/tcl-vm/tests/data/native_oo_variable_resolver/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/receipt.json). SHA-256 `5c2e5d7e680435ce191353bda889274eb6bbbf3ee1332ce654d90d139ca235aa`. Exact original counted declaration/body/name arrays or complete actual six-provider compilation/process correspondence.
- `tcl8.4-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_variable_resolver/8.4.20/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/8.4.20/receipt.json). SHA-256 `3b38d7638f0e7285c2a84d5f2055c997c9752a460153b96cf07a610b914dd3a1`. Actual provider version/build and process stream; guest read errors remain rows, harness compile/process exits are zero.
- `tcl8.4-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_variable_resolver/8.4.20/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/8.4.20/stdout.tsv). SHA-256 `46f05ae01d71cf78a58b0557e6cab7ddc56fe225a9872ef14387c1c9227f2fc3`. Actual provider version/build and process stream; guest read errors remain rows, harness compile/process exits are zero.
- `tcl8.4-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_variable_resolver/8.4.20/stderr](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual provider version/build and process stream; guest read errors remain rows, harness compile/process exits are zero.
- `tcl8.5-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_variable_resolver/8.5.19/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/8.5.19/receipt.json). SHA-256 `a98318b2fea5ee0f8e9159918d3f2a52c1ce461edf8e2f1c988a8c502158c407`. Actual provider version/build and process stream; guest read errors remain rows, harness compile/process exits are zero.
- `tcl8.5-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_variable_resolver/8.5.19/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/8.5.19/stdout.tsv). SHA-256 `6763de318120d961bc97b57f9f5476dfdd1a9bc433a782898d4c618ca28564d0`. Actual provider version/build and process stream; guest read errors remain rows, harness compile/process exits are zero.
- `tcl8.5-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_variable_resolver/8.5.19/stderr](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual provider version/build and process stream; guest read errors remain rows, harness compile/process exits are zero.
- `tcl8.6-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_variable_resolver/8.6.18/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/8.6.18/receipt.json). SHA-256 `f3800b994ef796a4358fbd7bb2adc68fdc626c958da1bff8376c760faf78495f`. Actual provider version/build and process stream; guest read errors remain rows, harness compile/process exits are zero.
- `tcl8.6-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_variable_resolver/8.6.18/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/8.6.18/stdout.tsv). SHA-256 `f6e02b194a132fdd8e6c6626d8d96a7172440a5ba1c26efe62788d87eaaa7a3c`. Actual provider version/build and process stream; guest read errors remain rows, harness compile/process exits are zero.
- `tcl8.6-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_variable_resolver/8.6.18/stderr](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual provider version/build and process stream; guest read errors remain rows, harness compile/process exits are zero.
- `tcl8.6-resolver-source` (source-anchor): [rust/tcl-vm/tests/data/native_oo_variable_resolver/source-anchors/8.6.18-resolver.json](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/source-anchors/8.6.18-resolver.json). SHA-256 `1837a2c144fb36033a12b3f44991a17ff24674fcc97280338fbf3d1ce94fc18d`. JSON pointer `/snippet`. Actual source window for distinct compiled full-count and runtime CString resolvers; independent of native output and Rust correctness.
- `tcl9.0-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_variable_resolver/9.0.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/9.0.4/receipt.json). SHA-256 `b9057de835d75538b7ba4039eab455e209734b04355540890a71f9d467dc1d4c`. Actual provider version/build and process stream; guest read errors remain rows, harness compile/process exits are zero.
- `tcl9.0-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_variable_resolver/9.0.4/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/9.0.4/stdout.tsv). SHA-256 `74c63b93d672217ae34abe6ab104ce7f4c8ec070785e97606a4ab39c31f7c705`. Actual provider version/build and process stream; guest read errors remain rows, harness compile/process exits are zero.
- `tcl9.0-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_variable_resolver/9.0.4/stderr](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual provider version/build and process stream; guest read errors remain rows, harness compile/process exits are zero.
- `tcl9.0-resolver-source` (source-anchor): [rust/tcl-vm/tests/data/native_oo_variable_resolver/source-anchors/9.0.4-resolver.json](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/source-anchors/9.0.4-resolver.json). SHA-256 `adfc5fed1f5f770eae39b4fffc12070c20efd6edf63ad4574be8b1879c90c2a3`. JSON pointer `/snippet`. Actual source window for distinct compiled full-count and runtime CString resolvers; independent of native output and Rust correctness.
- `tcl9.1-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_variable_resolver/9.1.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/9.1.0/receipt.json). SHA-256 `2d61fb92b11877ddaa45ecda354eb19df26848f9e6890afb1e8d492243187f34`. Actual provider version/build and process stream; guest read errors remain rows, harness compile/process exits are zero.
- `tcl9.1-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_variable_resolver/9.1.0/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/9.1.0/stdout.tsv). SHA-256 `59ad2585af05d7b4553c8c63c02ef9cde82ddb443e56aa0e6cb494be752608e4`. Actual provider version/build and process stream; guest read errors remain rows, harness compile/process exits are zero.
- `tcl9.1-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_variable_resolver/9.1.0/stderr](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual provider version/build and process stream; guest read errors remain rows, harness compile/process exits are zero.
- `tcl9.1-resolver-source` (source-anchor): [rust/tcl-vm/tests/data/native_oo_variable_resolver/source-anchors/9.1.0-resolver.json](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/source-anchors/9.1.0-resolver.json). SHA-256 `40aaa9e6a7163a9496186f873d523ff569bf9018bcd4ba77972303426a369ef1`. JSON pointer `/snippet`. Actual source window for distinct compiled full-count and runtime CString resolvers; independent of native output and Rust correctness.
- `jim-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_variable_resolver/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/jim/receipt.json). SHA-256 `e33b4008babf74ba515a0c4f418b57fd8ab1ba8cba9c7383ae6685cfe17595ac`. Actual provider version/build and process stream; guest read errors remain rows, harness compile/process exits are zero.
- `jim-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_variable_resolver/jim/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/jim/stdout.tsv). SHA-256 `5707d1b91c4f468cfe79a38218291016830110fb0bc37c8d942c2c7eff73518a`. Actual provider version/build and process stream; guest read errors remain rows, harness compile/process exits are zero.
- `jim-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_variable_resolver/jim/stderr](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual provider version/build and process stream; guest read errors remain rows, harness compile/process exits are zero.

## Source inspection

tcl8.6 8.6.18, revision `Pinned Tcl 8.6.18 release source, retained full-file SHA-256`, `generic/tclOOMethod.c`, function `ProcedureMethodVarResolver / ProcedureMethodCompiledVarConnect / ProcedureMethodCompiledVarResolver`, lines 914–1075. Full-source SHA-256 `c9212d902416d74246b9c6283e17b094c749171b65c86213b7b0af590da7a728`; snippet SHA-256 `2f6569315414f326d73d8da807ff83574a901a84c31b82f903c79263f9e8a1ae`; retained evidence `tcl8.6-resolver-source`.

```text
ProcedureMethodVarResolver(
    Tcl_Interp *interp,
    const char *varName,
    Tcl_Namespace *contextNs,
    int flags,
    Tcl_Var *varPtr)
{
    int result;
    Tcl_ResolvedVarInfo *rPtr = NULL;

    result = ProcedureMethodCompiledVarResolver(interp, varName,
	    strlen(varName), contextNs, &rPtr);

    if (result != TCL_OK) {
	return result;
    }

    *varPtr = rPtr->fetchProc(interp, rPtr);

    /*
     * Must not retain reference to resolved information. [Bug 3105999]
     */

    rPtr->deleteProc(rPtr);
    return (*varPtr ? TCL_OK : TCL_CONTINUE);
}

static Tcl_Var
ProcedureMethodCompiledVarConnect(
    Tcl_Interp *interp,
    Tcl_ResolvedVarInfo *rPtr)
{
    OOResVarInfo *infoPtr = (OOResVarInfo *) rPtr;
    Interp *iPtr = (Interp *) interp;
    CallFrame *framePtr = iPtr->varFramePtr;
    CallContext *contextPtr;
    Tcl_Obj *variableObj;
    Tcl_HashEntry *hPtr;
    int i, isNew, cacheIt, varLen, len;
    const char *match, *varName;

    /*
     * Check that the variable is being requested in a context that is also a
     * method call; if not (i.e. we're evaluating in the object's namespace or
     * in a procedure of that namespace) then we do nothing.
     */

    if (framePtr == NULL || !(framePtr->isProcCallFrame & FRAME_IS_METHOD)) {
	return NULL;
    }
    contextPtr = (CallContext *)framePtr->clientData;

    /*
     * If we've done the work before (in a comparable context) then reuse that
     * rather than performing resolution ourselves.
     */

    if (infoPtr->cachedObjectVar) {
	return infoPtr->cachedObjectVar;
    }

    /*
     * Check if the variable is one we want to resolve at all (i.e. whether it
     * is in the list provided by the user). If not, we mustn't do anything
     * either.
     */

    varName = TclGetStringFromObj(infoPtr->variableObj, &varLen);
    if (contextPtr->callPtr->chain[contextPtr->index]
	    .mPtr->declaringClassPtr != NULL) {
	FOREACH(variableObj, contextPtr->callPtr->chain[contextPtr->index]
		.mPtr->declaringClassPtr->variables) {
	    match = TclGetStringFromObj(variableObj, &len);
	    if ((len == varLen) && !memcmp(match, varName, len)) {
		cacheIt = 0;
		goto gotMatch;
	    }
	}
    } else {
	FOREACH(variableObj, contextPtr->oPtr->variables) {
	    match = TclGetStringFromObj(variableObj, &len);
	    if ((len == varLen) && !memcmp(match, varName, len)) {
		cacheIt = 1;
		goto gotMatch;
	    }
	}
    }
    return NULL;

    /*
     * It is a variable we want to resolve, so resolve it.
     */

  gotMatch:
    hPtr = Tcl_CreateHashEntry(TclVarTable(contextPtr->oPtr->namespacePtr),
	    (char *) variableObj, &isNew);
    if (isNew) {
	TclSetVarNamespaceVar((Var *) TclVarHashGetValue(hPtr));
    }
    if (cacheIt) {
	infoPtr->cachedObjectVar = TclVarHashGetValue(hPtr);

	/*
	 * We must keep a reference to the variable so everything will
	 * continue to work correctly even if it is unset; being unset does
	 * not end the life of the variable at this level. [Bug 3185009]
	 */

	VarHashRefCount(infoPtr->cachedObjectVar)++;
    }
    return TclVarHashGetValue(hPtr);
}

static void
ProcedureMethodCompiledVarDelete(
    Tcl_ResolvedVarInfo *rPtr)
{
    OOResVarInfo *infoPtr = (OOResVarInfo *) rPtr;

    /*
     * Release the reference to the variable if we were holding it.
     */

    if (infoPtr->cachedObjectVar) {
	VarHashRefCount(infoPtr->cachedObjectVar)--;
	TclCleanupVar((Var *) infoPtr->cachedObjectVar, NULL);
    }
    Tcl_DecrRefCount(infoPtr->variableObj);
    ckfree(infoPtr);
}

static int
ProcedureMethodCompiledVarResolver(
    Tcl_Interp *interp,
    const char *varName,
    int length,
    Tcl_Namespace *contextNs,
    Tcl_ResolvedVarInfo **rPtrPtr)
{
    OOResVarInfo *infoPtr;
    Tcl_Obj *variableObj = Tcl_NewStringObj(varName, length);

    /*
     * Do not create resolvers for cases that contain namespace separators or
     * which look like array accesses. Both will lead us astray.
     */

    if (strstr(Tcl_GetString(variableObj), "::") != NULL ||
	    Tcl_StringMatch(Tcl_GetString(variableObj), "*(*)")) {
	Tcl_DecrRefCount(variableObj);
	return TCL_CONTINUE;
    }

    infoPtr = (OOResVarInfo *)ckalloc(sizeof(OOResVarInfo));
    infoPtr->info.fetchProc = ProcedureMethodCompiledVarConnect;
    infoPtr->info.deleteProc = ProcedureMethodCompiledVarDelete;
    infoPtr->cachedObjectVar = NULL;
    infoPtr->variableObj = variableObj;
    Tcl_IncrRefCount(variableObj);
    *rPtrPtr = &infoPtr->info;
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `Pinned Tcl 9.0.4 release source, retained full-file SHA-256`, `generic/tclOOMethod.c`, function `ProcedureMethodVarResolver / ProcedureMethodCompiledVarConnect / ProcedureMethodCompiledVarResolver`, lines 1067–1247. Full-source SHA-256 `ad958b2a136ed614f53f2c1fc132a844738d4ff24298e098ce3bc5986525447c`; snippet SHA-256 `b89c791b057a92a6cba072dbd233d762b34e2c1d7e644738612b845685a81d83`; retained evidence `tcl9.0-resolver-source`.

```text
ProcedureMethodVarResolver(
    Tcl_Interp *interp,
    const char *varName,
    Tcl_Namespace *contextNs,
    TCL_UNUSED(int) /*flags*/,	// Ignoring variable access flags (???)
    Tcl_Var *varPtr)
{
    int result;
    Tcl_ResolvedVarInfo *rPtr = NULL;

    result = ProcedureMethodCompiledVarResolver(interp, varName,
	    strlen(varName), contextNs, &rPtr);

    if (result != TCL_OK) {
	return result;
    }

    *varPtr = rPtr->fetchProc(interp, rPtr);

    /*
     * Must not retain reference to resolved information. [Bug 3105999]
     */

    rPtr->deleteProc(rPtr);
    return (*varPtr ? TCL_OK : TCL_CONTINUE);
}

static Tcl_Var
ProcedureMethodCompiledVarConnect(
    Tcl_Interp *interp,
    Tcl_ResolvedVarInfo *rPtr)
{
    OOResVarInfo *infoPtr = (OOResVarInfo *) rPtr;
    Interp *iPtr = (Interp *) interp;
    CallFrame *framePtr = iPtr->varFramePtr;
    CallContext *contextPtr;
    Tcl_Obj *variableObj;
    PrivateVariableMapping *privateVar;
    Tcl_HashEntry *hPtr;
    int isNew, cacheIt;
    Tcl_Size i, varLen, len;
    const char *match, *varName;

    /*
     * Check that the variable is being requested in a context that is also a
     * method call; if not (i.e. we're evaluating in the object's namespace or
     * in a procedure of that namespace) then we do nothing.
     */

    if (framePtr == NULL || !(framePtr->isProcCallFrame & FRAME_IS_METHOD)) {
	return NULL;
    }
    contextPtr = (CallContext *) framePtr->clientData;

    /*
     * If we've done the work before (in a comparable context) then reuse that
     * rather than performing resolution ourselves.
     */

    if (infoPtr->cachedObjectVar) {
	return infoPtr->cachedObjectVar;
    }

    /*
     * Check if the variable is one we want to resolve at all (i.e. whether it
     * is in the list provided by the user). If not, we mustn't do anything
     * either.
     */

    varName = Tcl_GetStringFromObj(infoPtr->variableObj, &varLen);
    if (contextPtr->callPtr->chain[contextPtr->index]
	    .mPtr->declaringClassPtr != NULL) {
	FOREACH_STRUCT(privateVar, contextPtr->callPtr->chain[contextPtr->index]
		.mPtr->declaringClassPtr->privateVariables) {
	    match = Tcl_GetStringFromObj(privateVar->variableObj, &len);
	    if ((len == varLen) && !memcmp(match, varName, len)) {
		variableObj = privateVar->fullNameObj;
		cacheIt = 0;
		goto gotMatch;
	    }
	}
	FOREACH(variableObj, contextPtr->callPtr->chain[contextPtr->index]
		.mPtr->declaringClassPtr->variables) {
	    match = Tcl_GetStringFromObj(variableObj, &len);
	    if ((len == varLen) && !memcmp(match, varName, len)) {
		cacheIt = 0;
		goto gotMatch;
	    }
	}
    } else {
	FOREACH_STRUCT(privateVar, contextPtr->oPtr->privateVariables) {
	    match = Tcl_GetStringFromObj(privateVar->variableObj, &len);
	    if ((len == varLen) && !memcmp(match, varName, len)) {
		variableObj = privateVar->fullNameObj;
		cacheIt = 1;
		goto gotMatch;
	    }
	}
	FOREACH(variableObj, contextPtr->oPtr->variables) {
	    match = Tcl_GetStringFromObj(variableObj, &len);
	    if ((len == varLen) && !memcmp(match, varName, len)) {
		cacheIt = 1;
		goto gotMatch;
	    }
	}
    }
    return NULL;

    /*
     * It is a variable we want to resolve, so resolve it.
     */

  gotMatch:
    hPtr = Tcl_CreateHashEntry(TclVarTable(contextPtr->oPtr->namespacePtr),
	    variableObj, &isNew);
    if (isNew) {
	TclSetVarNamespaceVar((Var *) TclVarHashGetValue(hPtr));
    }
    if (cacheIt) {
	infoPtr->cachedObjectVar = TclVarHashGetValue(hPtr);

	/*
	 * We must keep a reference to the variable so everything will
	 * continue to work correctly even if it is unset; being unset does
	 * not end the life of the variable at this level. [Bug 3185009]
	 */

	VarHashRefCount(infoPtr->cachedObjectVar)++;
    }
    return TclVarHashGetValue(hPtr);
}

static void
ProcedureMethodCompiledVarDelete(
    Tcl_ResolvedVarInfo *rPtr)
{
    OOResVarInfo *infoPtr = (OOResVarInfo *) rPtr;

    /*
     * Release the reference to the variable if we were holding it.
     */

    if (infoPtr->cachedObjectVar) {
	VarHashRefCount(infoPtr->cachedObjectVar)--;
	TclCleanupVar((Var *) infoPtr->cachedObjectVar, NULL);
    }
    Tcl_DecrRefCount(infoPtr->variableObj);
    Tcl_Free(infoPtr);
}

static int
ProcedureMethodCompiledVarResolver(
    TCL_UNUSED(Tcl_Interp *),
    const char *varName,
    Tcl_Size length,
    TCL_UNUSED(Tcl_Namespace *),
    Tcl_ResolvedVarInfo **rPtrPtr)
{
    OOResVarInfo *infoPtr;
    Tcl_Obj *variableObj = Tcl_NewStringObj(varName, length);

    /*
     * Do not create resolvers for cases that contain namespace separators or
     * which look like array accesses. Both will lead us astray.
     */

    if (strstr(TclGetString(variableObj), "::") != NULL ||
	    Tcl_StringMatch(TclGetString(variableObj), "*(*)")) {
	Tcl_DecrRefCount(variableObj);
	return TCL_CONTINUE;
    }

    infoPtr = (OOResVarInfo *) Tcl_Alloc(sizeof(OOResVarInfo));
    infoPtr->info.fetchProc = ProcedureMethodCompiledVarConnect;
    infoPtr->info.deleteProc = ProcedureMethodCompiledVarDelete;
    infoPtr->cachedObjectVar = NULL;
    infoPtr->variableObj = variableObj;
    Tcl_IncrRefCount(variableObj);
    *rPtrPtr = &infoPtr->info;
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `Pinned Tcl 9.1.0 release source, retained full-file SHA-256`, `generic/tclOOMethod.c`, function `ProcedureMethodVarResolver / ProcedureMethodCompiledVarConnect / ProcedureMethodCompiledVarResolver / GetRealVarName`, lines 1095–1310. Full-source SHA-256 `1424048c7a16891c68535ae215359d440f28654a72f72eefadc9b5aada54f1d1`; snippet SHA-256 `f0ee40ea2a97375403713e263d8d609d06ff0ef15f9b5129134130591a0c95fb`; retained evidence `tcl9.1-resolver-source`.

```text
GetRealVarName(
    CallContext *contextPtr,
    const char *varName,
    Tcl_Size varNameLen,
    bool *isInstanceVar)
{
    const Class *clsPtr = contextPtr->callPtr->chain[contextPtr->index]
	    .mPtr->declaringClassPtr;
    const PrivateVariableMapping *privateVar;
    Tcl_Size i;
    Tcl_Obj *variableObj;
    const char *name;
    Tcl_Size len;

    if (clsPtr) {
	FOREACH_STRUCT(privateVar, clsPtr->privateVariables) {
	    name = TclGetStringFromObj(privateVar->variableObj, &len);
	    if (varNameLen == len && !memcmp(varName, name, len)) {
		if (isInstanceVar) {
		    *isInstanceVar = false;
		}
		return privateVar->fullNameObj;
	    }
	}
	FOREACH(variableObj, clsPtr->variables) {
	    name = TclGetStringFromObj(variableObj, &len);
	    if (varNameLen == len && !memcmp(varName, name, len)) {
		if (isInstanceVar) {
		    *isInstanceVar = false;
		}
		return variableObj;
	    }
	}
    } else {
	const Object *oPtr = contextPtr->oPtr;
	FOREACH_STRUCT(privateVar, oPtr->privateVariables) {
	    name = TclGetStringFromObj(privateVar->variableObj, &len);
	    if (varNameLen == len && !memcmp(varName, name, len)) {
		if (isInstanceVar) {
		    *isInstanceVar = true;
		}
		return privateVar->fullNameObj;
	    }
	}
	FOREACH(variableObj, oPtr->variables) {
	    name = TclGetStringFromObj(variableObj, &len);
	    if (varNameLen == len && !memcmp(varName, name, len)) {
		if (isInstanceVar) {
		    *isInstanceVar = true;
		}
		return variableObj;
	    }
	}
    }
    if (isInstanceVar) {
	*isInstanceVar = false;
    }
    return NULL;
}

// Get or allocate a variable in an object.
static inline Tcl_Var
GetObjectVar(
    Object *oPtr,
    Tcl_Obj *variableObj)
{
    int isNew;
    Tcl_HashEntry *hPtr = Tcl_CreateHashEntry(
	    TclVarTable(oPtr->namespacePtr), variableObj, &isNew);
    Tcl_Var var = TclVarHashGetValue(hPtr);
    if (isNew) {
	TclSetVarNamespaceVar((Var *) var);
    }
    return var;
}

// Called on entry to a compiled context to connect the local variables to
// be resolved to the actual variables in the object instance. If we want to
// connect it, we return the variable; otherwise NULL.
// This is the core of the variable resolver.
static Tcl_Var
ProcedureMethodCompiledVarConnect(
    Tcl_Interp *interp,
    Tcl_ResolvedVarInfo *rPtr)
{
    OOResVarInfo *infoPtr = (OOResVarInfo *) rPtr;

    /*
     * Check that the variable is being requested in a context that is also a
     * method call; if not (i.e. we're evaluating in the object's namespace or
     * in a procedure of that namespace) then we do nothing.
     */

    CallContext *contextPtr = GetCurrentCallContext(interp);
    if (!contextPtr) {
	return NULL;
    }

    /*
     * If we've done the work before (in a comparable context) then reuse that
     * rather than performing resolution ourselves.
     */

    if (infoPtr->cachedObjectVar) {
	return infoPtr->cachedObjectVar;
    }

    /*
     * Check if the variable is one we want to resolve at all (i.e. whether it
     * is in the list provided by the user). If not, we mustn't do anything
     * either.
     */

    bool cacheIt;
    Tcl_Obj *variableObj = GetRealVarName(contextPtr, infoPtr->varName,
	    infoPtr->varNameLen, &cacheIt);
    if (!variableObj) {
	return NULL;
    }

    /*
     * It is a variable we want to resolve, so resolve it.
     */

    Tcl_Var var = GetObjectVar(contextPtr->oPtr, variableObj);
    if (cacheIt) {
	infoPtr->cachedObjectVar = var;

	/*
	 * We must keep a reference to the variable so everything will
	 * continue to work correctly even if it is unset; being unset does
	 * not end the life of the variable at this level. [Bug 3185009]
	 */

	VarHashRefCount(infoPtr->cachedObjectVar)++;
    }
    return var;
}

static void
ProcedureMethodCompiledVarDelete(
    Tcl_ResolvedVarInfo *rPtr)
{
    OOResVarInfo *infoPtr = (OOResVarInfo *) rPtr;

    /*
     * Release the reference to the variable if we were holding it.
     */

    if (infoPtr->cachedObjectVar) {
	VarHashRefCount(infoPtr->cachedObjectVar)--;
	TclCleanupVar((Var *) infoPtr->cachedObjectVar, NULL);
    }
    Tcl_Free(infoPtr);
}

static int
ProcedureMethodVarResolver(
    Tcl_Interp *interp,		// Calling context holder.
    const char *varName,	// Variable name to look for. Not array element.
    TCL_UNUSED(Tcl_Namespace *) /*contextNs*/,
    TCL_UNUSED(int) /*flags*/,	// Ignoring variable access flags (???)
    Tcl_Var *varPtr)		// Where to write found variable.
{
    /*
     * Check that the variable is being requested in a context that is also a
     * method call; if not (i.e. we're evaluating in the object's namespace or
     * in a procedure of that namespace) then we do nothing.
     */

    CallContext *contextPtr = GetCurrentCallContext(interp);
    if (!contextPtr) {
	return TCL_CONTINUE;
    }

    /*
     * Do not resolve for cases that contain namespace separators.
     */

    if ((varName[0] == ':' && varName[1] == ':')
	    || strstr(varName, "::") != NULL) {
	return TCL_CONTINUE;
    }

    Tcl_Obj *realVarObj = GetRealVarName(contextPtr, varName,
	    (Tcl_Size) strlen(varName), NULL);

    Tcl_Var var = NULL;
    if (realVarObj) {
	/*
	* It is a variable we want to resolve, so resolve it.
	*/

	*varPtr = var = GetObjectVar(contextPtr->oPtr, realVarObj);
    }
    return (var ? TCL_OK : TCL_CONTINUE);
}

static int
ProcedureMethodCompiledVarResolver(
    TCL_UNUSED(Tcl_Interp *)/*interp*/,
    const char *varName,
    Tcl_Size length,
    TCL_UNUSED(Tcl_Namespace *)/*contextNs*/,
    Tcl_ResolvedVarInfo **rPtrPtr)
{
    OOResVarInfo *infoPtr = (OOResVarInfo *) Tcl_Alloc(
	    offsetof(OOResVarInfo, varName) + sizeof(char) * (length + 1));
    infoPtr->info.fetchProc = ProcedureMethodCompiledVarConnect;
    infoPtr->info.deleteProc = ProcedureMethodCompiledVarDelete;
    infoPtr->cachedObjectVar = NULL;
    infoPtr->varNameLen = length;
    memcpy(infoPtr->varName, varName, sizeof(char) * (length + 1));
    *rPtrPtr = &infoPtr->info;
    return TCL_OK;
}

```


## Consumer bindings

- [rust/tcl-vm/src/cmd_oo/native_variables.rs](../../../../rust/tcl-vm/src/cmd_oo/native_variables.rs), `runtime_target`: Selects declaring-provider runtime namespace storage with the independent RuntimeRoot CString recipe; it does not create a compiled local.
- [runtime/rust/src/cmd_oo/native_variables.rs](../../../../runtime/rust/src/cmd_oo/native_variables.rs), `NativeOoVariableResolver::runtime_target`: Retains an actual reached method frame and current declaring inventory before selected runtime root comparison.
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `Vm::add_tcloo_compiled_instance_link`: Connects an independently admitted compiled local ordinal to actual object namespace storage; declaration equality alone cannot issue a slot.
- [rust/tcl-vm/src/cmd_oo/native_variables.rs](../../../../rust/tcl-vm/src/cmd_oo/native_variables.rs), `cmd_oo::native_variables::tests::compiled_and_dynamic_declared_variables_match_original_native_controls` (linked): Compares all 48 retained counted declaration/static/dynamic/read/object-vars rows for each of C8.6/C9.0/C9.1 through independently selected native VM or Runtime interpreter contexts. It does not grant compiler preparation to source analysis or assert test execution from the native capture.
- [runtime/rust/src/cmd_oo/native_variables.rs](../../../../runtime/rust/src/cmd_oo/native_variables.rs), `cmd_oo::native_variables::tests::compiled_and_dynamic_declared_variables_match_original_native_controls` (linked): Compares all 48 retained counted declaration/static/dynamic/read/object-vars rows for each of C8.6/C9.0/C9.1 through independently selected native VM or Runtime interpreter contexts. It does not grant compiler preparation to source analysis or assert test execution from the native capture.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-vm/tests/data/native_oo_variable_resolver/replay.py",
  "--c-root",
  "tmp",
  "--jim-root",
  "/workspace/.proofs/native-providers/jimtcl",
  "--output",
  "/tmp/native-oo-variable-resolver-reconfirmation"
]
```

Requires exact retained probe/public-header/static-library/Makefile/source-owner hashes and matching pinned providers. Recompile/run and compare complete process exit/stdout/stderr; guest read errors/OO absence remain expected rows. --verify-only checks bytes without native launch. No cache/header/local table or Rust execution is asserted.
