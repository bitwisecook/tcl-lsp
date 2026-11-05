# naming.bootstrap.original-mathop-constructor-publication

Kind: `native-observation`

## Problem statement

Operator command availability alone cannot establish which original constructor owns namespace/command/export publication. A precise bootstrap contract needs genuine constructor observations before distribution initialisation, preserving release-specific unavailable query purposes and Jim unsupported commands.

## Question

Which original C/Jim constructor and distribution channels publish ::tcl::mathop, its +/operator command inventory and namespace exports, and what can C8.4 or Jim unavailable direct/namespace queries establish?

## Conclusion

The corrected request 424 original C8.5.19/C8.6.18/C9.0.4/C9.1.0 probes find both ::tcl::mathop and its original + command through genuine public Tcl_FindNamespace/Tcl_FindCommand immediately after Tcl_CreateInterp, before Tcl_Init. Independent Eval queries report the + command, full captured operator roster, namespace-exists 1 and export *. The same exact channels remain present after Tcl_Init returns 0. The corrected C8.4.20 probe instead records that those Find declarations are unavailable in its selected original public header; both create-only and distribution-initialised Eval queries succeed with empty +/family lists, namespace-exists 0 and the guarded ABSENT marker, with Tcl_Init returning 0. The original request 422 C8.4 compile exit 1 is retained with its complete diagnostic and no execution; it is not an absence observation. Current Jim 0.84-9-g5bac7c9 is measured separately at Jim_CreateInterp, Jim_RegisterCoreCommands and Jim_InitStaticExtensions returning 0: early queries fail with their exact unavailable command/namespace-helper results; after static extension initialisation, info commands +/family queries return empty lists. Jim namespace exists and the guarded export query both fail because exists is unsupported, so no Jim namespace absence or export-list conclusion follows. All 12 corrected request 424 compile/execution commands exit 0 with empty execute stderr. The original request 422 request retains 11 operations: five compile/execution pairs exit 0 and the C8.4 compile fails with exit 1. Their original streams are distinct evidence and every available repeated requests 422/424 execution stdout agrees. No operator execution, compiled-opcode recipe, formal/binding/admission/frame, private table/object identity, other extension roster, BIG-IP behaviour or Rust assertion pass is established.

## Scope

Both immutable requests preserve their exact C/Jim probes, selected public headers/static archives/configuration/source pins, version provenance and separately built 11 executable images. Fixed public ASCII Eval queries test the mathop namespace before entering it; the guarded export query creates no missing namespace. C public constructor table booleans are separate from Eval completion/code/count-preserving hex results and from later Tcl_Init. C8.4 direct table queries are purpose-unavailable in the selected header; public Eval is the measured channel. Jim constructor/core/extension channels are separate and unsupported namespace queries remain code 1 results, never negative existence facts. Version rows are explicitly reused original CLI receipts tied to the same required original executable/source/configuration pins; the bootstrap probes do not run a fresh patchlevel query. Actual compile/link argv, cwd and explicit TCL_LIBRARY overlay are preserved; static archive identity is verified. Compiler version, inherited environment and unrelated system library identities are unrecorded; no measured dynamic Tcl/Jim library is invented. The C8.5+ source windows explain constructor-owned namespace/export/command registration independently of the public capture. Entire pinned C8.4 tclBasic.c has no mathop token, which alone does not prove runtime absence. Complete original request 422 compile failure and corrected request 424 outputs remain separate; no raw evidence or previous public 60/84 mathop source control is replaced. These captures supply no Rust assertion coverage; software controls require their own exact source and execution receipts. BIG-IP is not tested.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Genuine original public C constructor/table/Eval before and after Tcl_Init; independent Jim CreateInterp/RegisterCoreCommands/InitStaticExtensions original Eval channels; counted result hex with exact completion codes. Version: tcl8.4 8.4.20.

Corrected424 constructor and Tcl_Init returning 0 Eval: +/family empty, namespace-exists 0, guarded export ABSENT. Original422 selected FindNamespace/FindCommand compile fails with exit 1 and never executes; corrected request 424 direct public-header table purpose is explicitly unavailable.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Genuine original public C constructor/table/Eval before and after Tcl_Init; independent Jim CreateInterp/RegisterCoreCommands/InitStaticExtensions original Eval channels; counted result hex with exact completion codes. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Public original table queries immediately after CreateInterp record namespace 1/plus 1 before Tcl_Init. Eval +/family presence, namespace-exists 1/export * and full counted roster agree before/after Tcl_Init returning 0; exact repeated requests 422/424 stdout agrees.

### jim

Status: `observed`. Channel: Genuine original public C constructor/table/Eval before and after Tcl_Init; independent Jim CreateInterp/RegisterCoreCommands/InitStaticExtensions original Eval channels; counted result hex with exact completion codes. Version: jim 0.84-9-g5bac7c9.

Actual CreateInterp/core-registration queries retain exact unavailable results. Static extensions initialise with code 0; +/family command inventories are empty. Namespace exists and guarded export queries return unsupported-exists code 1; namespace absence and export-list contents are not established.

### bigip

Status: `not-tested`.

No BIG-IP constructor, operator inventory, namespace or export observation is executed by these requests.

## Source inspection

tcl8.4 8.4.20, revision `Pinned original source; source-control revision unrecorded except current Jim 0.84-9-g5bac7c9 version provenance.`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclBasic.c`, function `Tcl_CreateInterp original constructor entry; absence is bounded to entire retained tclBasic.c`, lines 286–311. Retained reference `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclBasic.c`.

```text
 *----------------------------------------------------------------------
 */

Tcl_Interp *
Tcl_CreateInterp()
{
    Interp *iPtr;
    Tcl_Interp *interp;
    Command *cmdPtr;
    BuiltinFunc *builtinFuncPtr;
    MathFunc *mathFuncPtr;
    Tcl_HashEntry *hPtr;
    CONST CmdInfo *cmdInfoPtr;
    int i;
    union {
	char c[sizeof(short)];
	short s;
    } order;
#ifdef TCL_COMPILE_STATS
    ByteCodeStats *statsPtr;
#endif /* TCL_COMPILE_STATS */

    TclInitSubsystems(NULL);

    /*
     * Panic if someone updated the CallFrame structure without

```

tcl8.5 8.5.19, revision `Pinned original source; source-control revision unrecorded except current Jim 0.84-9-g5bac7c9 version provenance.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclBasic.c`, function `Tcl_CreateInterp original mathematical operator namespace/command/export registration`, lines 750–792. Retained reference `native_info_commands_literal_original-original-inputs-8.5.19-generic-tclBasic.c`.

```text
    /*
     * Register the mathematical "operator" commands. [TIP #174]
     */

    mathopNSPtr = Tcl_CreateNamespace(interp, "::tcl::mathop", NULL, NULL);
#define MATH_OP_PREFIX_LEN 15 /* == strlen("::tcl::mathop::") */
    if (mathopNSPtr == NULL) {
	Tcl_Panic("can't create math operator namespace");
    }
    (void) Tcl_Export(interp, mathopNSPtr, "*", 1);
    strcpy(mathFuncName, "::tcl::mathop::");
    for (opcmdInfoPtr=mathOpCmds ; opcmdInfoPtr->name!=NULL ; opcmdInfoPtr++){
	TclOpCmdClientData *occdPtr = (TclOpCmdClientData *)
		ckalloc(sizeof(TclOpCmdClientData));

	occdPtr->op = opcmdInfoPtr->name;
	occdPtr->i.numArgs = opcmdInfoPtr->i.numArgs;
	occdPtr->expected = opcmdInfoPtr->expected;
	strcpy(mathFuncName + MATH_OP_PREFIX_LEN, opcmdInfoPtr->name);
	cmdPtr = (Command *) Tcl_CreateObjCommand(interp, mathFuncName,
		opcmdInfoPtr->objProc, occdPtr, DeleteOpCmdClientData);
	if (cmdPtr == NULL) {
	    Tcl_Panic("failed to create math operator %s",
		    opcmdInfoPtr->name);
	} else if (opcmdInfoPtr->compileProc != NULL) {
	    cmdPtr->compileProc = opcmdInfoPtr->compileProc;
	}
    }

    /*
     * Do Multiple/Safe Interps Tcl init stuff
     */

    TclInterpInit(interp);
    TclSetupEnv(interp);

    /*
     * TIP #59: Make embedded configuration information available.
     */

    TclInitEmbeddedConfigurationInformation(interp);

    /*

```

tcl8.6 8.6.18, revision `Pinned original source; source-control revision unrecorded except current Jim 0.84-9-g5bac7c9 version provenance.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `Tcl_CreateInterp original mathematical operator namespace/command/export registration`, lines 935–977. Retained reference `native_oo_original_collision249-request-source-anchors-8.6.18-tclBasic.c`.

```text
    if (nsPtr == NULL) {
	Tcl_Panic("can't create math operator namespace");
    }
    Tcl_Export(interp, nsPtr, "*", 1);
#define MATH_OP_PREFIX_LEN 15 /* == strlen("::tcl::mathop::") */
    memcpy(mathFuncName, "::tcl::mathop::", MATH_OP_PREFIX_LEN);
    for (opcmdInfoPtr=mathOpCmds ; opcmdInfoPtr->name!=NULL ; opcmdInfoPtr++){
	TclOpCmdClientData *occdPtr = (TclOpCmdClientData *)ckalloc(sizeof(TclOpCmdClientData));

	occdPtr->op = opcmdInfoPtr->name;
	occdPtr->i.numArgs = opcmdInfoPtr->i.numArgs;
	occdPtr->expected = opcmdInfoPtr->expected;
	strcpy(mathFuncName + MATH_OP_PREFIX_LEN, opcmdInfoPtr->name);
	cmdPtr = (Command *) Tcl_CreateObjCommand(interp, mathFuncName,
		opcmdInfoPtr->objProc, occdPtr, DeleteOpCmdClientData);
	if (cmdPtr == NULL) {
	    Tcl_Panic("failed to create math operator %s",
		    opcmdInfoPtr->name);
	} else if (opcmdInfoPtr->compileProc != NULL) {
	    cmdPtr->compileProc = opcmdInfoPtr->compileProc;
	}
    }

    /*
     * Do Multiple/Safe Interps Tcl init stuff
     */

    TclInterpInit(interp);
    TclSetupEnv(interp);

    /*
     * TIP #59: Make embedded configuration information available.
     */

    TclInitEmbeddedConfigurationInformation(interp);

    /*
     * TIP #440: Declare the name of the script engine to be "Tcl".
     */

    Tcl_SetVar2(interp, "tcl_platform", "engine", "Tcl",
	    TCL_GLOBAL_ONLY);

```

tcl9.0 9.0.4, revision `Pinned original source; source-control revision unrecorded except current Jim 0.84-9-g5bac7c9 version provenance.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `Tcl_CreateInterp original mathematical operator namespace/command/export registration`, lines 1282–1324. Retained reference `native_oo_original_collision249-request-source-anchors-9.0.4-tclBasic.c`.

```text
    nsPtr = Tcl_CreateNamespace(interp, "::tcl::mathop", NULL, NULL);
    if (nsPtr == NULL) {
	Tcl_Panic("cannot create math operator namespace");
    }
    Tcl_Export(interp, nsPtr, "*", 1);
#define MATH_OP_PREFIX_LEN 15 /* == strlen("::tcl::mathop::") */
    memcpy(mathFuncName, "::tcl::mathop::", MATH_OP_PREFIX_LEN);
    for (opcmdInfoPtr=mathOpCmds ; opcmdInfoPtr->name!=NULL ; opcmdInfoPtr++){
	TclOpCmdClientData *occdPtr = (TclOpCmdClientData *)Tcl_Alloc(sizeof(TclOpCmdClientData));

	occdPtr->op = opcmdInfoPtr->name;
	occdPtr->i.numArgs = opcmdInfoPtr->i.numArgs;
	occdPtr->expected = opcmdInfoPtr->expected;
	strcpy(mathFuncName + MATH_OP_PREFIX_LEN, opcmdInfoPtr->name);
	cmdPtr = (Command *) Tcl_CreateObjCommand(interp, mathFuncName,
		opcmdInfoPtr->objProc, occdPtr, DeleteOpCmdClientData);
	if (cmdPtr == NULL) {
	    Tcl_Panic("failed to create math operator %s",
		    opcmdInfoPtr->name);
	} else if (opcmdInfoPtr->compileProc != NULL) {
	    cmdPtr->compileProc = opcmdInfoPtr->compileProc;
	}
    }

    /*
     * Do Multiple/Safe Interps Tcl init stuff
     */

    TclInterpInit(interp);
    TclSetupEnv(interp);

    /*
     * TIP #59: Make embedded configuration information available.
     */

    TclInitEmbeddedConfigurationInformation(interp);

    /*
     * TIP #440: Declare the name of the script engine to be "Tcl".
     */

    Tcl_SetVar2(interp, "tcl_platform", "engine", "Tcl",
	    TCL_GLOBAL_ONLY);

```

tcl9.1 9.1.0, revision `Pinned original source; source-control revision unrecorded except current Jim 0.84-9-g5bac7c9 version provenance.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `Tcl_CreateInterp original mathematical operator namespace/command/export registration`, lines 1218–1260. Retained reference `native_oo_original_collision249-request-source-anchors-9.1.0-tclBasic.c`.

```text
    if (nsPtr == NULL) {
	Tcl_Panic("cannot create math operator namespace");
    }
    Tcl_Export(interp, nsPtr, "*", 1);
#define MATH_OP_PREFIX_LEN 15 /* == strlen("::tcl::mathop::") */
    memcpy(mathFuncName, "::tcl::mathop::", MATH_OP_PREFIX_LEN);
    for (opcmdInfoPtr=mathOpCmds ; opcmdInfoPtr->name!=NULL ; opcmdInfoPtr++){
	TclOpCmdClientData *occdPtr = (TclOpCmdClientData *)Tcl_Alloc(sizeof(TclOpCmdClientData));

	occdPtr->op = opcmdInfoPtr->name;
	occdPtr->numArgs = opcmdInfoPtr->numArgs;
	occdPtr->expected = opcmdInfoPtr->expected;
	strcpy(mathFuncName + MATH_OP_PREFIX_LEN, opcmdInfoPtr->name);
	cmdPtr = (Command *) Tcl_CreateObjCommand2(interp, mathFuncName,
		opcmdInfoPtr->objProc, occdPtr, DeleteOpCmdClientData);
	if (cmdPtr == NULL) {
	    Tcl_Panic("failed to create math operator %s",
		    opcmdInfoPtr->name);
	} else if (opcmdInfoPtr->compileProc != NULL) {
	    cmdPtr->compileProc = opcmdInfoPtr->compileProc;
	}
    }

    /*
     * Do Multiple/Safe Interps Tcl init stuff
     */

    TclInterpInit(interp);
    TclSetupEnv(interp);

    /*
     * TIP #59: Make embedded configuration information available.
     */

    TclInitEmbeddedConfigurationInformation(interp);

    /*
     * TIP #440: Declare the name of the script engine to be "Tcl".
     */

    Tcl_SetVar2(interp, "tcl_platform", "engine", "Tcl",
	    TCL_GLOBAL_ONLY);

```

jim 0.84-9-g5bac7c9, revision `Pinned original source; source-control revision unrecorded except current Jim 0.84-9-g5bac7c9 version provenance.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_CreateInterp original constructor`, lines 5858–5892. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
Jim_Interp *Jim_CreateInterp(void)
{
    Jim_Interp *i = Jim_Alloc(sizeof(*i));

    memset(i, 0, sizeof(*i));

    i->maxCallFrameDepth = JIM_MAX_CALLFRAME_DEPTH;
    i->maxEvalDepth = JIM_MAX_EVAL_DEPTH;
    i->lastCollectTime = Jim_GetTimeUsec(CLOCK_MONOTONIC_RAW);

    /* Note that we can create objects only after the
     * interpreter liveList and freeList pointers are
     * initialized to NULL. */
    Jim_InitHashTable(&i->commands, &JimCommandsHashTableType, i);
#ifdef JIM_REFERENCES
    Jim_InitHashTable(&i->references, &JimReferencesHashTableType, i);
#endif
    Jim_InitHashTable(&i->assocData, &JimAssocDataHashTableType, i);
    Jim_InitHashTable(&i->packages, &JimPackageHashTableType, NULL);
    i->emptyObj = Jim_NewEmptyStringObj(i);
    i->trueObj = Jim_NewIntObj(i, 1);
    i->falseObj = Jim_NewIntObj(i, 0);
    i->framePtr = i->topFramePtr = JimCreateCallFrame(i, NULL, i->emptyObj);
    i->result = i->emptyObj;
    i->stackTrace = Jim_NewListObj(i, NULL, 0);
    i->unknown = Jim_NewStringObj(i, "unknown", -1);
    i->defer = Jim_NewStringObj(i, "jim::defer", -1);
    i->errorProc = i->emptyObj;
    i->nullScriptObj = Jim_NewEmptyStringObj(i);
    i->evalFrame = &i->topEvalFrame;
    i->currentFilenameObj = Jim_NewEmptyStringObj(i);
    Jim_IncrRefCount(i->emptyObj);
    Jim_IncrRefCount(i->result);
    Jim_IncrRefCount(i->stackTrace);
    Jim_IncrRefCount(i->unknown);

```

jim 0.84-9-g5bac7c9, revision `Pinned original source; source-control revision unrecorded except current Jim 0.84-9-g5bac7c9 version provenance.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_RegisterCoreCommands original core table registration`, lines 16919–16930. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
void Jim_RegisterCoreCommands(Jim_Interp *interp)
{
    const struct jim_core_cmd_def_t *c;

    for (c = Jim_CoreCommandsTable; c->name; c++) {
        /* All core commands must have usage */
        assert(c->usage);
        Jim_RegisterCmd(interp, c->name, c->usage, c->minargs, c->maxargs, c->cmdProc, NULL, NULL, c->flags);
    }
}

/* -----------------------------------------------------------------------------

```

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_mathop_constructor_publication422/providers/8.4.20](../../../../rust/tcl-registry/tests/data/native_mathop_constructor_publication422/providers/8.4.20)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources)
- [rust/tcl-registry/tests/data/native_jim_class_source269](../../../../rust/tcl-registry/tests/data/native_jim_class_source269)
- [rust/tcl-registry/tests/data/native_c_alias_holder197/8.4.20](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.4.20)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs)
- [rust/tcl-registry/tests/data/native_child_bootstrap_context303/source](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303/source)
- [rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/jim](../../../../rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/jim)
