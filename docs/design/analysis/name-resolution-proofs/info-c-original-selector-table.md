# naming.info.c-original-selector-table

Kind: `native-observation`

## Problem statement

A single copied maximum-release table can change genuine C subcommand presence, prefix ambiguity and error choices, including configured frame and separate TclOO additions.

## Question

Which actual C8.4–9.1 public info table and ensemble-map members are selected by these fresh configured providers?

## Conclusion

The configured C8.4 table has21members; C8.5 has22; C8.6 adds coroutine/errorstack and TclOO class/object; C9 adds cmdtype/constant/consts. C8.4 frame is compiled out in this provider. Modern public map results and whole public miss inventories are retained independently because TclOO augments selection beyond the map projection.

## Scope

52 fixed ASCII original-object argv controls per measured provider; one successful missing-variable existence query and public inventory/map requests, all other exact/prefix selectors deliberately have excessive argc. No opaque/raw-zero/C080 argv, handler result/value effects, object header/cache, source parser, normal completion or native compiler/worker admission. Jim’s actual lsort/helper side effects are source facts, not closed by this observation. C options and Jim result options were not queried. All 312 processes completed with process exit 0 with no external timeout.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Fresh-process ASCII NUL-terminated C literals converted by Tcl_NewStringObj/Jim_NewStringObj length -1, then original object vectors via Tcl_EvalObjv(TCL_EVAL_DIRECT)/Jim_EvalObjVector; public counted result getter after invocation. Version: tcl8.4 8.4.20.

Captured public miss: bad option "__r2286_absent__": must be args, body, cmdcount, commands, complete, default, exists, functions, globals, hostname, level, library, loaded, locals, nameofexecutable, patchlevel, procs, script, sharedlibextension, tclversion, or vars. Query info exists missing ->0. Public ensemble map query fails before a map is returned on8.4. Exact/prefix controls retain their own original arity/unknown diagnostics; no successful member behaviour inferred.

### tcl8.5

Status: `observed`. Channel: Fresh-process ASCII NUL-terminated C literals converted by Tcl_NewStringObj/Jim_NewStringObj length -1, then original object vectors via Tcl_EvalObjv(TCL_EVAL_DIRECT)/Jim_EvalObjVector; public counted result getter after invocation. Version: tcl8.5 8.5.19.

Captured public miss: unknown or ambiguous subcommand "__r2286_absent__": must be args, body, cmdcount, commands, complete, default, exists, frame, functions, globals, hostname, level, library, loaded, locals, nameofexecutable, patchlevel, procs, script, sharedlibextension, tclversion, or vars. Query info exists missing ->0. Public ensemble map query succeeds; actual root miss additionally lists TclOO registrations on 8.6+ independently of map enumeration. Exact/prefix controls retain their own original arity/unknown diagnostics; no successful member behaviour inferred.

### tcl8.6

Status: `observed`. Channel: Fresh-process ASCII NUL-terminated C literals converted by Tcl_NewStringObj/Jim_NewStringObj length -1, then original object vectors via Tcl_EvalObjv(TCL_EVAL_DIRECT)/Jim_EvalObjVector; public counted result getter after invocation. Version: tcl8.6 8.6.18.

Captured public miss: unknown or ambiguous subcommand "__r2286_absent__": must be args, body, class, cmdcount, commands, complete, coroutine, default, errorstack, exists, frame, functions, globals, hostname, level, library, loaded, locals, nameofexecutable, object, patchlevel, procs, script, sharedlibextension, tclversion, or vars. Query info exists missing ->0. Public ensemble map query succeeds; actual root miss additionally lists TclOO registrations on 8.6+ independently of map enumeration. Exact/prefix controls retain their own original arity/unknown diagnostics; no successful member behaviour inferred.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Fresh-process ASCII NUL-terminated C literals converted by Tcl_NewStringObj/Jim_NewStringObj length -1, then original object vectors via Tcl_EvalObjv(TCL_EVAL_DIRECT)/Jim_EvalObjVector; public counted result getter after invocation. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Captured public miss: unknown or ambiguous subcommand "__r2286_absent__": must be args, body, class, cmdcount, cmdtype, commands, complete, constant, consts, coroutine, default, errorstack, exists, frame, functions, globals, hostname, level, library, loaded, locals, nameofexecutable, object, patchlevel, procs, script, sharedlibextension, tclversion, or vars. Query info exists missing ->0. Public ensemble map query succeeds; actual root miss additionally lists TclOO registrations on 8.6+ independently of map enumeration. Exact/prefix controls retain their own original arity/unknown diagnostics; no successful member behaviour inferred.

### jim

Status: `not-tested`. Channel: Fresh-process ASCII NUL-terminated C literals converted by Tcl_NewStringObj/Jim_NewStringObj length -1, then original object vectors via Tcl_EvalObjv(TCL_EVAL_DIRECT)/Jim_EvalObjVector; public counted result getter after invocation. Version: jim 0.84-9-g5bac7c9.

Captured under the separate Jim selector question; this question makes no counterpart-family semantic claim.

### bigip

Status: `not-tested`. Channel: No vendor invocation.

No actual embedded provider result; no C/Jim inventory borrowed.

## Source inspection

tcl8.4 8.4.20, revision `exact configured native source pinned in receipt`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdIL.c`, function `Tcl_InfoObjCmd`, lines 416–451. Retained reference `eb05e32e0658a50901745`.

```text
Tcl_InfoObjCmd(clientData, interp, objc, objv)
    ClientData clientData;	/* Arbitrary value passed to the command. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    static CONST char *subCmds[] = {
	     "args", "body", "cmdcount", "commands",
	     "complete", "default", "exists",
#ifdef TCL_TIP280
	     "frame",
#endif
	     "functions",
	     "globals", "hostname", "level", "library", "loaded",
	     "locals", "nameofexecutable", "patchlevel", "procs",
	     "script", "sharedlibextension", "tclversion", "vars",
	     (char *) NULL};
    enum ISubCmdIdx {
	    IArgsIdx, IBodyIdx, ICmdCountIdx, ICommandsIdx,
	    ICompleteIdx, IDefaultIdx, IExistsIdx,
#ifdef TCL_TIP280
	    IFrameIdx,
#endif
	    IFunctionsIdx,
	    IGlobalsIdx, IHostnameIdx, ILevelIdx, ILibraryIdx, ILoadedIdx,
	    ILocalsIdx, INameOfExecutableIdx, IPatchLevelIdx, IProcsIdx,
	    IScriptIdx, ISharedLibExtensionIdx, ITclVersionIdx, IVarsIdx
    };
    int index, result;

    if (objc < 2) {
        Tcl_WrongNumArgs(interp, 1, objv, "option ?arg arg ...?");
        return TCL_ERROR;
    }
    
    result = Tcl_GetIndexFromObj(interp, objv[1], subCmds, "option", 0,

```

tcl8.5 8.5.19, revision `exact configured native source pinned in receipt`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdIL.c`, function `defaultInfoMap`, lines 155–179. Retained reference `ef8273a04b205f42a044a`.

```text
static const EnsembleImplMap defaultInfoMap[] = {
    {"args",		   InfoArgsCmd,		    NULL},
    {"body",		   InfoBodyCmd,		    NULL},
    {"cmdcount",	   InfoCmdCountCmd,	    NULL},
    {"commands",	   InfoCommandsCmd,	    NULL},
    {"complete",	   InfoCompleteCmd,	    NULL},
    {"default",		   InfoDefaultCmd,	    NULL},
    {"exists",		   TclInfoExistsCmd,	    TclCompileInfoExistsCmd},
    {"frame",		   InfoFrameCmd,	    NULL},
    {"functions",	   InfoFunctionsCmd,	    NULL},
    {"globals",		   TclInfoGlobalsCmd,	    NULL},
    {"hostname",	   InfoHostnameCmd,	    NULL},
    {"level",		   InfoLevelCmd,	    NULL},
    {"library",		   InfoLibraryCmd,	    NULL},
    {"loaded",		   InfoLoadedCmd,	    NULL},
    {"locals",		   TclInfoLocalsCmd,	    NULL},
    {"nameofexecutable",   InfoNameOfExecutableCmd, NULL},
    {"patchlevel",	   InfoPatchLevelCmd,	    NULL},
    {"procs",		   InfoProcsCmd,	    NULL},
    {"script",		   InfoScriptCmd,	    NULL},
    {"sharedlibextension", InfoSharedlibCmd,	    NULL},
    {"tclversion",	   InfoTclVersionCmd,	    NULL},
    {"vars",		   TclInfoVarsCmd,	    NULL},
    {NULL, NULL, NULL}
};

```

tcl8.6 8.6.18, revision `exact configured native source pinned in receipt`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdIL.c`, function `defaultInfoMap`, lines 155–183. Retained reference `ec61deb0928b7098a6164`.

```text
static const EnsembleImplMap defaultInfoMap[] = {
    {"args",		   InfoArgsCmd,		    TclCompileBasic1ArgCmd, NULL, NULL, 0},
    {"body",		   InfoBodyCmd,		    TclCompileBasic1ArgCmd, NULL, NULL, 0},
    {"cmdcount",	   InfoCmdCountCmd,	    TclCompileBasic0ArgCmd, NULL, NULL, 0},
    {"commands",	   InfoCommandsCmd,	    TclCompileInfoCommandsCmd, NULL, NULL, 0},
    {"complete",	   InfoCompleteCmd,	    TclCompileBasic1ArgCmd, NULL, NULL, 0},
    {"coroutine",	   TclInfoCoroutineCmd,     TclCompileInfoCoroutineCmd, NULL, NULL, 0},
    {"default",		   InfoDefaultCmd,	    TclCompileBasic3ArgCmd, NULL, NULL, 0},
    {"errorstack",	   InfoErrorStackCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"exists",		   TclInfoExistsCmd,	    TclCompileInfoExistsCmd, NULL, NULL, 0},
    {"frame",		   InfoFrameCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"functions",	   InfoFunctionsCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"globals",		   TclInfoGlobalsCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"hostname",	   InfoHostnameCmd,	    TclCompileBasic0ArgCmd, NULL, NULL, 0},
    {"level",		   InfoLevelCmd,	    TclCompileInfoLevelCmd, NULL, NULL, 0},
    {"library",		   InfoLibraryCmd,	    TclCompileBasic0ArgCmd, NULL, NULL, 0},
    {"loaded",		   InfoLoadedCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"locals",		   TclInfoLocalsCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"nameofexecutable",   InfoNameOfExecutableCmd, TclCompileBasic0ArgCmd, NULL, NULL, 0},
    {"patchlevel",	   InfoPatchLevelCmd,	    TclCompileBasic0ArgCmd, NULL, NULL, 0},
    {"procs",		   InfoProcsCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"script",		   InfoScriptCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"sharedlibextension", InfoSharedlibCmd,	    TclCompileBasic0ArgCmd, NULL, NULL, 0},
    {"tclversion",	   InfoTclVersionCmd,	    TclCompileBasic0ArgCmd, NULL, NULL, 0},
    {"vars",		   TclInfoVarsCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {NULL, NULL, NULL, NULL, NULL, 0}
};

/*

```

tcl8.6 8.6.18, revision `exact configured native source pinned in receipt`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOInfo.c`, function `TclOOInitInfo`, lines 90–122. Retained reference `e3aa9f151f946ce9d1b27`.

```text
 * TclOOInitInfo --
 *
 *	Adjusts the Tcl core [info] command to contain subcommands ("object"
 *	and "class") for introspection of objects and classes.
 *
 * ----------------------------------------------------------------------
 */

void
TclOOInitInfo(
    Tcl_Interp *interp)
{
    Tcl_Command infoCmd;
    Tcl_Obj *mapDict;

    /*
     * Build the ensembles used to implement [info object] and [info class].
     */

    TclMakeEnsemble(interp, "::oo::InfoObject", infoObjectCmds);
    TclMakeEnsemble(interp, "::oo::InfoClass", infoClassCmds);

    /*
     * Install into the [info] ensemble.
     */

    infoCmd = Tcl_FindCommand(interp, "info", NULL, TCL_GLOBAL_ONLY);
    if (infoCmd) {
	Tcl_GetEnsembleMappingDict(NULL, infoCmd, &mapDict);
	TclDictPutString(NULL, mapDict, "object", "::oo::InfoObject");
	TclDictPutString(NULL, mapDict, "class", "::oo::InfoClass");
	Tcl_SetEnsembleMappingDict(interp, infoCmd, mapDict);
    }

```

tcl9.0 9.0.4, revision `exact configured native source pinned in receipt`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdIL.c`, function `defaultInfoMap`, lines 151–182. Retained reference `e1aa552c4dc622cb3eacf`.

```text
static const EnsembleImplMap defaultInfoMap[] = {
    {"args",		   InfoArgsCmd,		    TclCompileBasic1ArgCmd, NULL, NULL, 0},
    {"body",		   InfoBodyCmd,		    TclCompileBasic1ArgCmd, NULL, NULL, 0},
    {"cmdcount",	   InfoCmdCountCmd,	    TclCompileBasic0ArgCmd, NULL, NULL, 0},
    {"cmdtype",		   InfoCmdTypeCmd,	    TclCompileBasic1ArgCmd, NULL, NULL, 1},
    {"commands",	   InfoCommandsCmd,	    TclCompileInfoCommandsCmd, NULL, NULL, 0},
    {"complete",	   InfoCompleteCmd,	    TclCompileBasic1ArgCmd, NULL, NULL, 0},
    {"constant",	   TclInfoConstantCmd,	    TclCompileBasic1ArgCmd, NULL, NULL, 0},
    {"consts",		   TclInfoConstsCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"coroutine",	   TclInfoCoroutineCmd,     TclCompileInfoCoroutineCmd, NULL, NULL, 0},
    {"default",		   InfoDefaultCmd,	    TclCompileBasic3ArgCmd, NULL, NULL, 0},
    {"errorstack",	   InfoErrorStackCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"exists",		   TclInfoExistsCmd,	    TclCompileInfoExistsCmd, NULL, NULL, 0},
    {"frame",		   InfoFrameCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"functions",	   InfoFunctionsCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"globals",		   TclInfoGlobalsCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"hostname",	   InfoHostnameCmd,	    TclCompileBasic0ArgCmd, NULL, NULL, 0},
    {"level",		   InfoLevelCmd,	    TclCompileInfoLevelCmd, NULL, NULL, 0},
    {"library",		   InfoLibraryCmd,	    TclCompileBasic0ArgCmd, NULL, NULL, 0},
    {"loaded",		   InfoLoadedCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"locals",		   TclInfoLocalsCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"nameofexecutable",   InfoNameOfExecutableCmd, TclCompileBasic0ArgCmd, NULL, NULL, 1},
    {"patchlevel",	   InfoPatchLevelCmd,	    TclCompileBasic0ArgCmd, NULL, NULL, 0},
    {"procs",		   InfoProcsCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"script",		   InfoScriptCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"sharedlibextension", InfoSharedlibCmd,	    TclCompileBasic0ArgCmd, NULL, NULL, 0},
    {"tclversion",	   InfoTclVersionCmd,	    TclCompileBasic0ArgCmd, NULL, NULL, 0},
    {"vars",		   TclInfoVarsCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {NULL, NULL, NULL, NULL, NULL, 0}
};

/*

```

tcl9.0 9.0.4, revision `exact configured native source pinned in receipt`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOInfo.c`, function `TclOOInitInfo`, lines 112–144. Retained reference `e0042f5cac458f2294aca`.

```text
 * TclOOInitInfo --
 *
 *	Adjusts the Tcl core [info] command to contain subcommands ("object"
 *	and "class") for introspection of objects and classes.
 *
 * ----------------------------------------------------------------------
 */

void
TclOOInitInfo(
    Tcl_Interp *interp)
{
    Tcl_Command infoCmd;
    Tcl_Obj *mapDict;

    /*
     * Build the ensembles used to implement [info object] and [info class].
     */

    TclMakeEnsemble(interp, "::oo::InfoObject", infoObjectCmds);
    TclMakeEnsemble(interp, "::oo::InfoClass", infoClassCmds);

    /*
     * Install into the [info] ensemble.
     */

    infoCmd = Tcl_FindCommand(interp, "info", NULL, TCL_GLOBAL_ONLY);
    if (infoCmd) {
	Tcl_GetEnsembleMappingDict(NULL, infoCmd, &mapDict);
	TclDictPutString(NULL, mapDict, "object", "::oo::InfoObject");
	TclDictPutString(NULL, mapDict, "class", "::oo::InfoClass");
	Tcl_SetEnsembleMappingDict(interp, infoCmd, mapDict);
    }

```

tcl9.1 9.1.0, revision `exact configured native source pinned in receipt`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdIL.c`, function `tclInfoImplMap`, lines 151–183. Retained reference `ed79c87cb9f8b610db3c2`.

```text

const EnsembleImplMap tclInfoImplMap[] = {
    {"args",		   InfoArgsCmd,		    TclCompileBasic1ArgCmd, NULL, NULL, 0},
    {"body",		   InfoBodyCmd,		    TclCompileBasic1ArgCmd, NULL, NULL, 0},
    {"cmdcount",	   InfoCmdCountCmd,	    TclCompileBasic0ArgCmd, NULL, NULL, 0},
    {"cmdtype",		   InfoCmdTypeCmd,	    TclCompileBasic1ArgCmd, NULL, NULL, 1},
    {"commands",	   InfoCommandsCmd,	    TclCompileInfoCommandsCmd, NULL, NULL, 0},
    {"complete",	   InfoCompleteCmd,	    TclCompileBasic1ArgCmd, NULL, NULL, 0},
    {"constant",	   TclInfoConstantCmd,	    TclCompileBasic1ArgCmd, NULL, NULL, 0},
    {"consts",		   TclInfoConstsCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"coroutine",	   TclInfoCoroutineCmd,     TclCompileInfoCoroutineCmd, NULL, NULL, 0},
    {"default",		   InfoDefaultCmd,	    TclCompileBasic3ArgCmd, NULL, NULL, 0},
    {"errorstack",	   InfoErrorStackCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"exists",		   TclInfoExistsCmd,	    TclCompileInfoExistsCmd, NULL, NULL, 0},
    {"frame",		   InfoFrameCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"functions",	   InfoFunctionsCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"globals",		   TclInfoGlobalsCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"hostname",	   InfoHostnameCmd,	    TclCompileBasic0ArgCmd, NULL, NULL, 0},
    {"level",		   InfoLevelCmd,	    TclCompileInfoLevelCmd, NULL, NULL, 0},
    {"library",		   InfoLibraryCmd,	    TclCompileBasic0ArgCmd, NULL, NULL, 0},
    {"loaded",		   InfoLoadedCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"locals",		   TclInfoLocalsCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"nameofexecutable",   InfoNameOfExecutableCmd, TclCompileBasic0ArgCmd, NULL, NULL, 1},
    {"patchlevel",	   InfoPatchLevelCmd,	    TclCompileBasic0ArgCmd, NULL, NULL, 0},
    {"procs",		   InfoProcsCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"script",		   InfoScriptCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"sharedlibextension", InfoSharedlibCmd,	    TclCompileBasic0ArgCmd, NULL, NULL, 0},
    {"tclversion",	   InfoTclVersionCmd,	    TclCompileBasic0ArgCmd, NULL, NULL, 0},
    {"vars",		   TclInfoVarsCmd,	    TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {NULL, NULL, NULL, NULL, NULL, 0}
};

/*

```

tcl9.1 9.1.0, revision `exact configured native source pinned in receipt`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInfo.c`, function `TclOOInitInfo`, lines 127–162. Retained reference `ee0317ef5cf255c936716`.

```text
 * TclOOInitInfo --
 *
 *	Adjusts the Tcl core [info] command to contain subcommands ("object"
 *	and "class") for introspection of objects and classes.
 *
 * ----------------------------------------------------------------------
 */

void
TclOOInitInfo(
    Tcl_Interp *interp)
{
    Tcl_Command infoCmd;
    Tcl_Obj *mapDict;

    /*
     * Build the ensembles used to implement [info object] and [info class].
     */

    TclMakeEnsemble(interp, "::oo::InfoObject", infoObjectImplMap);
    TclMakeEnsemble(interp, "::oo::InfoClass", infoClassImplMap);

    /*
     * Install into the [info] ensemble.
     * We keep the subcommands with their existing names instead of the
     * auto-generated ones supported by the ensemble guts because we're
     * somewhat documented to work this way.
     */

    infoCmd = Tcl_FindCommand(interp, "info", NULL, TCL_GLOBAL_ONLY);
    if (infoCmd) {
	Tcl_GetEnsembleMappingDict(NULL, infoCmd, &mapDict);
	TclDictPutString(NULL, mapDict, "object", "::oo::InfoObject");
	TclDictPutString(NULL, mapDict, "class", "::oo::InfoClass");
	Tcl_SetEnsembleMappingDict(interp, infoCmd, mapDict);
    }

```

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_info_inventory_original](../../../../rust/tcl-registry/tests/data/native_info_inventory_original)
- [rust/tcl-registry/tests/data/native_info_inventory_original/8.4.20](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/8.4.20)
- [rust/tcl-registry/tests/data/native_info_inventory_original/8.5.19](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/8.5.19)
- [rust/tcl-registry/tests/data/native_info_inventory_original/8.6.18](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/8.6.18)
- [rust/tcl-registry/tests/data/native_info_inventory_original/9.0.4](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/9.0.4)
- [rust/tcl-registry/tests/data/native_info_inventory_original/9.1.0](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/9.1.0)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources)
