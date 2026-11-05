# naming.info.original-missing-and-empty-selector-dispatch

Kind: `native-observation`

## Problem statement

A missing info selector and an explicit empty selector enter different public diagnostic paths. C8.4 option dispatch, the other selected C releases and Jim require their own original usage and selector-list messages; a generic ensemble renderer can erase these distinctions.

## Question

What exact public completion and diagnostic do the two original info and info {} source commands publish on each of the six pinned interpreters?

## Conclusion

All twelve original sources complete with outer guest code 0 and report caught command code 1. Missing-selector usage is info option ?arg arg ...? on C8.4, info subcommand ?argument ...? on C8.5, and info subcommand ?arg ...? on C8.6, C9.0, C9.1 and Jim, within their whole wrong-arguments messages. The empty selector reports ambiguous option on C8.4, unknown or ambiguous subcommand on C8.5–C9.1, and info, ambiguous command on Jim. Every whole release-specific selector list is retained in its exact original result bytes. The independent C8.4 Tcl_InfoObjCmd excerpt supplies read-only source facts about its usage string and option noun; public messages alone do not identify a private dispatch worker, command object or lookup/header/table/cache. All host processes exit 0 with empty stderr. No executed Rust assertion or BIG-IP result is supplied.

## Scope

Two unchanged ASCII LF originals retain their own explicit catch/list witnesses; the counted-source driver adds no original source wrapper. Each source is evaluated in a fresh fully initialised selected interpreter. Both commands use genuinely available info APIs on all six providers, with no unavailable substitute. Original outer completion, caught command completion and whole result remain independent fields. Complete request/source/driver/compiler/ELF/process/stdout/stderr/version and required provider pins are retained. The separate C8.4 whole source and exact excerpt are read-only evidence, not a private native observation. No current command binding, opaque AST, compiler/instruction admission, object/header/cache/frame identity or arbitrary selector behavior follows from these finite outputs. The Registry software selectors keep C8.4 option lookup and C release-specific missing-selector usage separate from ensemble and Jim dispatch purposes. Both Runtime and VM controls compare all twelve whole unchanged Native292 original completion/results. A pure selected-protocol control checks C8.4 option versus C8.5 argument versus C8.6+ arg usage and unavailable Jim C protocol, independently of the measured public messages. Those software definitions add no private native worker, original header/table/cache or current executed Rust result. Independently retained exact Jim source registers InfoCore with minargs=1 and usage subcommand ?arg ...?. JimCallNative checks that registered worker arity before entering Jim_InfoCoreCommand/Jim_ParseSubCmd, so bare input and an empty selector have distinct selected mechanisms. JimCmdUsage provides its own final formatter boundary. The renamed and NUL head examples in the marked shared recipe are software renderer controls, not additional executed Native originals. Both linked backend definitions compare the same twelve complete original public windows without a current software pass claim; source interpretation supplies no private worker/header/table identity or generic ensemble execution result.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Two independent fresh original public source evaluations; separate VERSION and ORIGINAL counted-hex channels, no added source envelope or result normalization. Version: tcl8.4 8.4.20.

Both original outer completions are 0; both caught command completions are 1. Missing selector: wrong # args: should be "info option ?arg arg ...?". Empty selector: ambiguous option "": must be args, body, cmdcount, commands, complete, default, exists, functions, globals, hostname, level, library, loaded, locals, nameofexecutable, patchlevel, procs, script, sharedlibextension, tclversion, or vars. Whole measured messages and original selector lists remain public values only. Both fresh host processes exit 0 with empty stderr.

### tcl8.5

Status: `observed`. Channel: Two independent fresh original public source evaluations; separate VERSION and ORIGINAL counted-hex channels, no added source envelope or result normalization. Version: tcl8.5 8.5.19.

Both original outer completions are 0; both caught command completions are 1. Missing selector: wrong # args: should be "info subcommand ?argument ...?". Empty selector: unknown or ambiguous subcommand "": must be args, body, cmdcount, commands, complete, default, exists, frame, functions, globals, hostname, level, library, loaded, locals, nameofexecutable, patchlevel, procs, script, sharedlibextension, tclversion, or vars. Whole measured messages and original selector lists remain public values only. Both fresh host processes exit 0 with empty stderr.

### tcl8.6

Status: `observed`. Channel: Two independent fresh original public source evaluations; separate VERSION and ORIGINAL counted-hex channels, no added source envelope or result normalization. Version: tcl8.6 8.6.18.

Both original outer completions are 0; both caught command completions are 1. Missing selector: wrong # args: should be "info subcommand ?arg ...?". Empty selector: unknown or ambiguous subcommand "": must be args, body, class, cmdcount, commands, complete, coroutine, default, errorstack, exists, frame, functions, globals, hostname, level, library, loaded, locals, nameofexecutable, object, patchlevel, procs, script, sharedlibextension, tclversion, or vars. Whole measured messages and original selector lists remain public values only. Both fresh host processes exit 0 with empty stderr.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Two independent fresh original public source evaluations; separate VERSION and ORIGINAL counted-hex channels, no added source envelope or result normalization. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Both original outer completions are 0; both caught command completions are 1. Missing selector: wrong # args: should be "info subcommand ?arg ...?". Empty selector: unknown or ambiguous subcommand "": must be args, body, class, cmdcount, cmdtype, commands, complete, constant, consts, coroutine, default, errorstack, exists, frame, functions, globals, hostname, level, library, loaded, locals, nameofexecutable, object, patchlevel, procs, script, sharedlibextension, tclversion, or vars. Whole measured messages and original selector lists remain public values only. Both fresh host processes exit 0 with empty stderr.

### jim

Status: `observed`. Channel: Two independent fresh original public source evaluations; separate VERSION and ORIGINAL counted-hex channels, no added source envelope or result normalization. Version: jim 0.84-9-g5bac7c9.

Both original outer completions are 0; both caught command completions are 1. Missing selector: wrong # args: should be "info subcommand ?arg ...?". Empty selector: info, ambiguous command "": should be alias, aliases, args, body, channels, commands, complete, exists, frame, globals, help, hostname, level, locals, nameofexecutable, patchlevel, procs, references, returncodes, script, source, stacktrace, statics, tainted, usage, vars, version. Whole measured messages and original selector lists remain public values only. Both fresh host processes exit 0 with empty stderr.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Source inspection

tcl8.4 8.4.20; independent pinned source inspection, revision `Source-control revision not recorded by this appendix.`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdIL.c`, function `Tcl_InfoObjCmd`, lines 416–455. Retained reference `native_info_original_dispatch292-request-tcl84-tclCmdIL.c`.

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
	    (int *) &index);
    if (result != TCL_OK) {
	return result;
    }

```

jim 0.84-9-g5bac7c9; exact source inspection distinct from public execution, revision `Exact source SHA also retained by the actual Native292 Jim compile receipt; no new revision or process claimed.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimCmdUsage`, lines 11828–11877. Retained reference `native-info-jim-core-arity-source300-jim.c`.

```text
/* Returns a zero-ref count usage string for the command.
 * If no usage is availabe, just returns "cmd ..."
 */
static Jim_Obj *JimCmdUsage(Jim_Interp *interp, Jim_Obj *cmdNameObj, Jim_Cmd *cmd)
{
    Jim_Obj *usage = Jim_DuplicateObj(interp, cmdNameObj);

    if (cmd->flags & JIM_CMD_ISPROC) {
        int i;
        for (i = 0; i < cmd->u.proc.argListLen; i++) {
            Jim_AppendString(interp, usage, " ", 1);

            if (i == cmd->u.proc.argsPos) {
                if (cmd->u.proc.arglist[i].defaultObjPtr) {
                    /* Renamed args */
                    Jim_AppendString(interp, usage, "?", 1);
                    Jim_AppendObj(interp, usage, cmd->u.proc.arglist[i].defaultObjPtr);
                    Jim_AppendString(interp, usage, " ...?", -1);
                }
                else {
                    /* We have plain args */
                    Jim_AppendString(interp, usage, "?arg ...?", -1);
                }
            }
            else {
                if (cmd->u.proc.arglist[i].defaultObjPtr) {
                    Jim_AppendString(interp, usage, "?", 1);
                    Jim_AppendObj(interp, usage, cmd->u.proc.arglist[i].nameObjPtr);
                    Jim_AppendString(interp, usage, "?", 1);
                }
                else {
                    const char *arg = Jim_String(cmd->u.proc.arglist[i].nameObjPtr);
                    if (*arg == '&') {
                        arg++;
                    }
                    Jim_AppendString(interp, usage, arg, -1);
                }
            }
        }
    }
    else if (cmd->u.native.usage) {
        if (*cmd->u.native.usage) {
            Jim_AppendStrings(interp, usage, " ", cmd->u.native.usage, NULL);
        }
    }
    else {
        Jim_AppendString(interp, usage, " ...", -1);
    }

    return usage;

```

jim 0.84-9-g5bac7c9; exact source inspection distinct from public execution, revision `Exact source SHA also retained by the actual Native292 Jim compile receipt; no new revision or process claimed.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimCallNative`, lines 12038–12066. Retained reference `native-info-jim-core-arity-source300-jim.c`.

```text
static int JimCallNative(Jim_Interp *interp, Jim_Cmd *cmd, int argc, Jim_Obj *const *argv)
{
    int argsok = 1;
    int ret;

    /* Check arg count */
    if (argc - 1 < cmd->u.native.minargs) {
        argsok = 0;
    }
    else if (cmd->u.native.maxargs >= 0 && argc - 1 > cmd->u.native.maxargs) {
        argsok = 0;
    }
    else if (cmd->u.native.maxargs < -1 && (argc - 1) % -cmd->u.native.maxargs != 0) {
        /* -2 means must have n * 2 args */
        argsok = 0;
    }
    if (argsok) {
        interp->cmdPrivData = cmd->u.native.privData;
        ret = cmd->u.native.cmdProc(interp, argc, argv);
        if (ret != JIM_USAGE) {
            return ret;
        }
        /* This means an argument error */
    }

    //printf("Wrong args for %s, argc=%d, minargs=%d, maxargs=%d\n", Jim_String(argv[0]), argc, cmd->u.native.minargs, cmd->u.native.maxargs);
    Jim_SetResultFormatted(interp, "wrong # args: should be \"%#s\"", JimCmdUsage(interp, argv[0], cmd));
    return JIM_ERR;
}

```

jim 0.84-9-g5bac7c9; exact source inspection distinct from public execution, revision `Exact source SHA also retained by the actual Native292 Jim compile receipt; no new revision or process claimed.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_InfoCoreCommand`, lines 15985–16069. Retained reference `native-info-jim-core-arity-source300-jim.c`.

```text
/* [info] */
static int Jim_InfoCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_Obj *objPtr;
    int mode = 1;

    /* Must be kept in order with the array below */
    enum {
        INFO_ALIAS,
        INFO_ALIASES,
        INFO_ARGS,
        INFO_BODY,
        INFO_CHANNELS,
        INFO_COMMANDS,
        INFO_COMPLETE,
        INFO_EXISTS,
        INFO_FRAME,
        INFO_GLOBALS,
        INFO_HELP,
        INFO_HOSTNAME,
        INFO_LEVEL,
        INFO_LOCALS,
        INFO_NAMEOFEXECUTABLE,
        INFO_PATCHLEVEL,
        INFO_PROCS,
        INFO_REFERENCES,
        INFO_RETURNCODES,
        INFO_SCRIPT,
        INFO_SOURCE,
        INFO_STACKTRACE,
        INFO_STATICS,
        INFO_TAINTED,
        INFO_USAGE,
        INFO_VARS,
        INFO_VERSION,
        INFO_COUNT
    };
    static const jim_subcmd_type cmds[INFO_COUNT + 1] = {
        JIM_DEF_SUBCMD("alias", "command", 1, 1),
        JIM_DEF_SUBCMD("aliases", "?-all? ?pattern?", 0, 2),
        JIM_DEF_SUBCMD("args", "procname", 1, 1),
        JIM_DEF_SUBCMD("body", "procname", 1, 1),
        JIM_DEF_SUBCMD("channels", "?-all? ?pattern?", 0, 2),
        JIM_DEF_SUBCMD("commands", "?-all? ?pattern?", 0, 2),
        JIM_DEF_SUBCMD("complete", "script ?missing?", 1, 2),
        JIM_DEF_SUBCMD("exists", "varName", 1, 1),
        JIM_DEF_SUBCMD("frame", "?levelNum?", 0, 1),
        JIM_DEF_SUBCMD("globals", "?pattern?", 0, 1),
        JIM_DEF_SUBCMD("help", "command", 1, 1),
        JIM_DEF_SUBCMD("hostname", NULL, 0, 0),
        JIM_DEF_SUBCMD("level", "?levelNum?", 0, 1),
        JIM_DEF_SUBCMD("locals", "?pattern?", 0, 1),
        JIM_DEF_SUBCMD("nameofexecutable", NULL, 0, 0),
        JIM_DEF_SUBCMD("patchlevel", NULL, 0, 0),
        JIM_DEF_SUBCMD("procs", "?-all? ?pattern?", 0, 2),
        JIM_DEF_SUBCMD("references", NULL, 0, 0),
        JIM_DEF_SUBCMD("returncodes", "?code?", 0, 1),
        JIM_DEF_SUBCMD("script", "?filename?", 0, 1),
        JIM_DEF_SUBCMD("source", "source ?filename line?", 1, 3),
        JIM_DEF_SUBCMD("stacktrace", NULL, 0, 0),
        JIM_DEF_SUBCMD("statics", "procname", 1, 1),
        JIM_DEF_SUBCMD("tainted", "value", 1, 1),
        JIM_DEF_SUBCMD("usage", "command", 1, 1),
        JIM_DEF_SUBCMD("vars", "?pattern?", 0, 1),
        JIM_DEF_SUBCMD("version", NULL, 0, 0),
        { NULL }
    };
    const jim_subcmd_type *ct;
#ifdef jim_ext_namespace
    int nons = 0;

    if (argc > 2 && Jim_CompareStringImmediate(interp, argv[1], "-nons")) {
        /* This is for internal use only */
        argc--;
        argv++;
        nons = 1;
    }
#endif
    ct = Jim_ParseSubCmd(interp, cmds, argc, argv);
    if (!ct) {
        return JIM_ERR;
    }
    if (ct->function) {
        /* This is -help or -commands */
        return ct->function(interp, argc, argv);

```

jim 0.84-9-g5bac7c9; exact source inspection distinct from public execution, revision `Exact source SHA also retained by the actual Native292 Jim compile receipt; no new revision or process claimed.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_CoreCommandsTable_Info`, lines 16822–16865. Retained reference `native-info-jim-core-arity-source300-jim.c`.

```text

static const struct jim_core_cmd_def_t {
    const char *name;
    Jim_CmdProc *cmdProc;
    short minargs;
    short maxargs;
    const char *usage;
    int flags;
} Jim_CoreCommandsTable[] = {
    {"*", Jim_MulCoreCommand, 0, -1, "?number ...?"  },
    {"+", Jim_AddCoreCommand, 0, -1, "?number ...?"  },
    {"-", Jim_SubCoreCommand, 1, -1, "number ?number ...?" },
    {"/", Jim_DivCoreCommand, 1, -1, "number ?number ...?" },
    {"alias", Jim_AliasCoreCommand, 2, -1, "newname command ?args ...?" },
    {"append", Jim_AppendCoreCommand, 1, -1, "varName ?value ...?" },
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
    {"foreach", Jim_ForeachCoreCommand, 3, -1, "varList list ?varList list ...? script" },
    {"format", Jim_FormatCoreCommand, 1, -1, "formatString ?arg arg ...?" },
    {"global", Jim_GlobalCoreCommand, 1, -1, "varName ?varName ...?" },
    {"if", Jim_IfCoreCommand, 2, -1, "condition ?then? trueBody ?elseif ...? ?else? ?falseBody?" },
    {"incr", Jim_IncrCoreCommand, 1, 2, "varName ?increment?" },
    {"info", Jim_InfoCoreCommand, 1, -1, "subcommand ?arg ...?"},
    {"join", Jim_JoinCoreCommand, 1, 2, "list ?joinString?" },
    {"lappend", Jim_LappendCoreCommand, 1, -1, "varName ?value value ...?" },
    {"lassign", Jim_LassignCoreCommand, 2, -1, "varList list ?varName ...?" },

```

## Consumer bindings

- [rust/tcl-registry/src/native_index_lookup.rs](../../../../rust/tcl-registry/src/native_index_lookup.rs), `InvocationDialect::native_info_original_option_protocol`: Select the C8.4 original option lookup purpose independently of ensemble/Jim and actual table/header/getter/cache ownership.
- [rust/tcl-registry/src/native_index_lookup.rs](../../../../rust/tcl-registry/src/native_index_lookup.rs), `InvocationDialect::native_info_original_missing_selector_usage`: Select exact C release-specific info missing-selector usage; Jim presentation retains its independent protocol.
- [rust/tcl-registry/src/commands/tcl/info_jim.rs](../../../../rust/tcl-registry/src/commands/tcl/info_jim.rs), `NativeJimInfoProtocol::dispatch`: Keep the selected registered InfoCore minimum arity before generic missing/empty-selector parser reporting, independently of the written head spelling.
- [rust/tcl-registry/src/native_usage.rs](../../../../rust/tcl-registry/src/native_usage.rs), `NativeUsageProtocol::render_command_usage_message`: Preserve the separately selected called-name and final usage formatter boundary; pure renamed/NUL examples are not measured native invocations.
- [rust/tcl-registry/src/native_index_lookup.rs](../../../../rust/tcl-registry/src/native_index_lookup.rs), `native_index_lookup::tests::original_info_option_lookup_is_separate_from_ensemble_and_jim_dispatch` (linked): Pure selected info protocol keeps C8.4 option lookup and C8.5 argument/C8.6+ arg suffixes separate from Jim. It proves neither an actual lookup result nor private native header/table/cache identity.
- [runtime/rust/src/cmd_info/native_jim_inventory_tests.rs](../../../../runtime/rust/src/cmd_info/native_jim_inventory_tests.rs), `cmd_info::native_jim_inventory_tests::original_info_dispatch_matches_all_12_native_public_windows` (linked): Both unchanged original info sources compare all twelve whole Native292 public completion/results, retaining C8.4, C8.5 argument, C8.6+ arg and Jim messages separately. Definition linkage grants no current executed software pass or private native dispatch/header/table purpose.
- [rust/tcl-registry/src/commands/tcl/info_jim.rs](../../../../rust/tcl-registry/src/commands/tcl/info_jim.rs), `commands::tcl::info_jim::tests::original_jim_core_arity_precedes_generic_info_parser_reporting` (linked): Selected registered InfoCore arity reports its own usage before the generic parser. Empty selector enters the independent ambiguity recipe. Renamed and NUL called heads are pure formatter examples; Native292 remains the unchanged bare/empty public observation.
- [rust/tcl-vm/src/cmd_info/native_jim_inventory_tests.rs](../../../../rust/tcl-vm/src/cmd_info/native_jim_inventory_tests.rs), `cmd_info::native_jim_inventory_tests::original_info_dispatch_matches_all_12_native_public_windows` (linked): Both unchanged original info sources compare all twelve whole Native292 public completion/results, retaining C8.4, C8.5 argument, C8.6+ arg and Jim messages separately. Definition linkage grants no current executed software pass or private native dispatch/header/table purpose.

## Evidence

- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_info_original_dispatch292/8.4.20](../../../../rust/tcl-registry/tests/data/native_info_original_dispatch292/8.4.20)
- [rust/tcl-registry/tests/data/native_info_original_dispatch292/8.5.19](../../../../rust/tcl-registry/tests/data/native_info_original_dispatch292/8.5.19)
- [rust/tcl-registry/tests/data/native_info_original_dispatch292/8.6.18](../../../../rust/tcl-registry/tests/data/native_info_original_dispatch292/8.6.18)
- [rust/tcl-registry/tests/data/native_info_original_dispatch292/9.0.4](../../../../rust/tcl-registry/tests/data/native_info_original_dispatch292/9.0.4)
- [rust/tcl-registry/tests/data/native_info_original_dispatch292/9.1.0](../../../../rust/tcl-registry/tests/data/native_info_original_dispatch292/9.1.0)
- [rust/tcl-registry/tests](../../../../rust/tcl-registry/tests)
- [rust/tcl-registry/tests/data/native_info_original_dispatch292/jim](../../../../rust/tcl-registry/tests/data/native_info_original_dispatch292/jim)
