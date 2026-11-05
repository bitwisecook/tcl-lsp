# naming.info.jim-original-selector-table

Kind: `native-observation`

## Problem statement

Inherited C command availability can falsely donate C info members or signatures to Jim; copied runtime tables can repeat the same error.

## Question

Which original public info members, exact selector arities and prefix matches belong to Jim084 instead of its C ancestry?

## Conclusion

The native and source inventories agree on Jim’s own 27 operations. C-only class/object/coroutine/functions/loaded/default/tclversion/cmdcount are absent; the source descriptor must select Jim’s own rows and arities. The helper-driven miss/-commands and successful operation semantics remain separate from this finite membership observation. The same independently selected source recipe serves both port parsers and reporting. Choice reports dispatch the actual current lsort; they do not replace its effects with a static sort or grant arbitrary selected member success. The retained INFO_EXISTS source window selects Jim_GetVariable with flags zero and reports a boolean from its null/non-null result. The already selected Jim descriptor shares the generic InfoExists source operation while retaining its own NoHook native compilation grammar. This source/metadata join grants no successful handler, C compiler admission, contents-read erasure or normal completion.

## Scope

52 fixed ASCII original-object argv controls per measured provider; one successful missing-variable existence query and public inventory/map requests, all other exact/prefix selectors deliberately have excessive argc. No opaque/raw-zero/C080 argv, handler result/value effects, object header/cache, source parser, normal completion or native compiler/worker admission. Jim’s actual lsort/helper side effects are source facts, not closed by this observation. C options and Jim result options were not queried. All 312 processes completed with process exit 0 with no external timeout. The additional five source windows scope the source-only immediate/partial selector and report/helper recipe. Raw-zero/C080 and replacement-helper Rust controls remain implementation/source obligations, not native observations. All 312 measured controls, receipts and channels and provider status remain unchanged. Valid unimplemented Jim member handlers explicitly refuse a missing host capability. The additional INFO_EXISTS reader window and marked Registry operation/NoHook control are source/metadata obligations only; all existing 312 process captures and provider answers remain unchanged.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `not-tested`. Channel: Fresh-process ASCII NUL-terminated C literals converted by Tcl_NewStringObj/Jim_NewStringObj length -1, then original object vectors via Tcl_EvalObjv(TCL_EVAL_DIRECT)/Jim_EvalObjVector; public counted result getter after invocation. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Captured under the separate C selector question; this question makes no counterpart-family semantic claim.

### jim

Status: `observed`. Channel: Fresh-process ASCII NUL-terminated C literals converted by Tcl_NewStringObj/Jim_NewStringObj length -1, then original object vectors via Tcl_EvalObjv(TCL_EVAL_DIRECT)/Jim_EvalObjVector; public counted result getter after invocation. Version: jim 0.84-9-g5bac7c9.

Jim reports exactly 27 subcommands: alias aliases args body channels commands complete exists frame globals help hostname level locals nameofexecutable patchlevel procs references returncodes script source stacktrace statics tainted usage vars version; cor/cl/cm are unknown, co and sta ambiguous, vers selects version and fails the deliberate excessive argument count. All 27 exact declared members fail excessive argc with their own usage. The C ensemble-map query itself rejects the unsupported configure form; it is not an info-map success.

### bigip

Status: `not-tested`. Channel: No vendor invocation.

No actual embedded provider result; no C/Jim inventory borrowed.

## Source inspection

jim 0.84-9-g5bac7c9, revision `exact configured native source pinned in receipt`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_InfoCoreCommand`, lines 15986–16053. Retained reference `e549e2d60cb87d1a2b351`.

```text
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

```

jim 0.84-9-g5bac7c9, revision `exact configured native source pinned in receipt`, `/workspace/.proofs/native-providers/jimtcl/jim-subcmd.c`, function `Jim_ParseSubCmd`, lines 99–179. Retained reference `ec3f480549c97c25d7528`.

```text
    NULL,
    NULL,
    JIM_TYPE_REFERENCES
};

const jim_subcmd_type *Jim_ParseSubCmd(Jim_Interp *interp, const jim_subcmd_type * command_table,
    int argc, Jim_Obj *const *argv)
{
    const jim_subcmd_type *ct;
    const jim_subcmd_type *partial = 0;
    int cmdlen;
    Jim_Obj *cmd;
    const char *cmdstr;
    int help = 0;
    int argsok = 1;

    if (argc < 2) {
        Jim_SetResultFormatted(interp, "wrong # args: should be \"%#s command ...\"\n"
            "Use \"%#s -help ?command?\" for help", argv[0], argv[0]);
        return 0;
    }

    cmd = argv[1];

    /* Use cached lookup if possible */
    if (cmd->typePtr == &subcmdLookupObjType) {
        if (cmd->internalRep.ptrIntValue.ptr == command_table) {
            ct = command_table + cmd->internalRep.ptrIntValue.int1;
            goto found;
        }
    }

    /* Check for the help command */
    if (Jim_CompareStringImmediate(interp, cmd, "-help")) {
        if (argc == 2) {
            /* Usage for the command, not the subcommand */
            show_cmd_usage(interp, command_table, argc, argv);
            return &dummy_subcmd;
        }
        help = 1;

        /* Skip the 'help' command */
        cmd = argv[2];
    }

    /* Check for special builtin '-commands' command first */
    if (Jim_CompareStringImmediate(interp, cmd, "-commands")) {
        Jim_SetResult(interp, subcmd_cmd_list(interp, command_table, " "));
        return &dummy_subcmd;
    }

    cmdstr = Jim_GetString(cmd, &cmdlen);

    for (ct = command_table; ct->cmd; ct++) {
        if (Jim_CompareStringImmediate(interp, cmd, ct->cmd)) {
            /* Found an exact match */
            break;
        }
        if (strncmp(cmdstr, ct->cmd, cmdlen) == 0) {
            if (partial) {
                /* Ambiguous */
                if (help) {
                    /* Just show the top level help here */
                    show_cmd_usage(interp, command_table, argc, argv);
                    return &dummy_subcmd;
                }
                bad_subcmd(interp, command_table, "ambiguous", argv[0], argv[1 + help]);
                return 0;
            }
            partial = ct;
        }
        continue;
    }

    /* If we had an unambiguous partial match */
    if (partial && !ct->cmd) {
        ct = partial;
    }

    if (!ct->cmd) {
        /* No matching command */

```

jim 0.84-9-g5bac7c9, revision `Exact original193/194 configured source/header/library/executable association retained in the corpus.`, `/workspace/.proofs/native-providers/jimtcl/jim-subcmd.c`, function `subcmd_cmd_list`, lines 32–75. Retained reference `info196-subcmd_cmd_list`.

```text
 * For example, if there are two commands, "def" and "abc", with a separator of "; ",
 * the returned string will be "abc; def"
 *
 * The returned object has a reference count of 0.
 */
static Jim_Obj *subcmd_cmd_list(Jim_Interp *interp, const jim_subcmd_type * ct, const char *sep)
{
    /* Create a list to sort before joining */
    Jim_Obj *listObj = Jim_NewListObj(interp, NULL, 0);
    Jim_Obj *sortCmd[2];

    for (; ct->cmd; ct++) {
        if (!(ct->flags & JIM_MODFLAG_HIDDEN)) {
            Jim_ListAppendElement(interp, listObj, Jim_NewStringObj(interp, ct->cmd, -1));
        }
    }

    /* There is no direct API to sort a list, so just invoke lsort here. */
    sortCmd[0] = Jim_NewStringObj(interp, "lsort", -1);
    sortCmd[1] = listObj;
    /* Leaves the result in the interpreter result */
    if (Jim_EvalObjVector(interp, 2, sortCmd) == JIM_OK) {
        return Jim_ListJoin(interp, Jim_GetResult(interp), sep, strlen(sep));
    }
    /* lsort can't really fail (normally), but if it does, just return the error as the result */
    return Jim_GetResult(interp);
}

static void bad_subcmd(Jim_Interp *interp, const jim_subcmd_type * command_table, const char *type,
    Jim_Obj *cmd, Jim_Obj *subcmd)
{
    Jim_SetResultFormatted(interp, "%#s, %s command \"%#s\": should be %#s", cmd, type,
        subcmd, subcmd_cmd_list(interp, command_table, ", "));
}

static void show_cmd_usage(Jim_Interp *interp, const jim_subcmd_type * command_table, int argc,
    Jim_Obj *const *argv)
{
    Jim_SetResultFormatted(interp, "Usage: \"%#s command ... \", where command is one of: %#s",
        argv[0], subcmd_cmd_list(interp, command_table, ", "));
}

static void add_cmd_usage(Jim_Interp *interp, const jim_subcmd_type * ct, Jim_Obj *cmd)
{

```

jim 0.84-9-g5bac7c9, revision `Exact original193/194 configured source/header/library/executable association retained in the corpus.`, `/workspace/.proofs/native-providers/jimtcl/jim-subcmd.c`, function `Jim_ParseSubCmd`, lines 99–250. Retained reference `info196-Jim_ParseSubCmd`.

```text
    NULL,
    NULL,
    JIM_TYPE_REFERENCES
};

const jim_subcmd_type *Jim_ParseSubCmd(Jim_Interp *interp, const jim_subcmd_type * command_table,
    int argc, Jim_Obj *const *argv)
{
    const jim_subcmd_type *ct;
    const jim_subcmd_type *partial = 0;
    int cmdlen;
    Jim_Obj *cmd;
    const char *cmdstr;
    int help = 0;
    int argsok = 1;

    if (argc < 2) {
        Jim_SetResultFormatted(interp, "wrong # args: should be \"%#s command ...\"\n"
            "Use \"%#s -help ?command?\" for help", argv[0], argv[0]);
        return 0;
    }

    cmd = argv[1];

    /* Use cached lookup if possible */
    if (cmd->typePtr == &subcmdLookupObjType) {
        if (cmd->internalRep.ptrIntValue.ptr == command_table) {
            ct = command_table + cmd->internalRep.ptrIntValue.int1;
            goto found;
        }
    }

    /* Check for the help command */
    if (Jim_CompareStringImmediate(interp, cmd, "-help")) {
        if (argc == 2) {
            /* Usage for the command, not the subcommand */
            show_cmd_usage(interp, command_table, argc, argv);
            return &dummy_subcmd;
        }
        help = 1;

        /* Skip the 'help' command */
        cmd = argv[2];
    }

    /* Check for special builtin '-commands' command first */
    if (Jim_CompareStringImmediate(interp, cmd, "-commands")) {
        Jim_SetResult(interp, subcmd_cmd_list(interp, command_table, " "));
        return &dummy_subcmd;
    }

    cmdstr = Jim_GetString(cmd, &cmdlen);

    for (ct = command_table; ct->cmd; ct++) {
        if (Jim_CompareStringImmediate(interp, cmd, ct->cmd)) {
            /* Found an exact match */
            break;
        }
        if (strncmp(cmdstr, ct->cmd, cmdlen) == 0) {
            if (partial) {
                /* Ambiguous */
                if (help) {
                    /* Just show the top level help here */
                    show_cmd_usage(interp, command_table, argc, argv);
                    return &dummy_subcmd;
                }
                bad_subcmd(interp, command_table, "ambiguous", argv[0], argv[1 + help]);
                return 0;
            }
            partial = ct;
        }
        continue;
    }

    /* If we had an unambiguous partial match */
    if (partial && !ct->cmd) {
        ct = partial;
    }

    if (!ct->cmd) {
        /* No matching command */
        if (help) {
            /* Just show the top level help here */
            show_cmd_usage(interp, command_table, argc, argv);
            return &dummy_subcmd;
        }
        bad_subcmd(interp, command_table, "unknown", argv[0], argv[1 + help]);
        return 0;
    }

    if (help) {
        Jim_SetResultString(interp, "Usage: ", -1);
        /* subcmd */
        add_cmd_usage(interp, ct, argv[0]);
        return &dummy_subcmd;
    }

    /* Cache the result for a successful non-help lookup */
    Jim_FreeIntRep(interp, cmd);
    cmd->typePtr = &subcmdLookupObjType;
    cmd->internalRep.ptrIntValue.ptr = (void *)command_table;
    cmd->internalRep.ptrIntValue.int1 = ct - command_table;

found:
    /* Check the number of args */

    if (argc - 2 < ct->minargs) {
        argsok = 0;
    }
    else if (ct->maxargs >= 0 && argc - 2 > ct->maxargs) {
        argsok = 0;
    }
    else if (ct->maxargs < -1 && (argc - 2) % -ct->maxargs != 0) {
        /* -2 means must have n * 2 args */
        argsok = 0;
    }
    if (!argsok) {
        Jim_SetResultString(interp, "wrong # args: should be \"", -1);
        /* subcmd */
        add_cmd_usage(interp, ct, argv[0]);
        Jim_AppendStrings(interp, Jim_GetResult(interp), "\"", NULL);

        return 0;
    }

    /* Good command */
    return ct;
}

int Jim_CallSubCmd(Jim_Interp *interp, const jim_subcmd_type * ct, int argc, Jim_Obj *const *argv)
{
    int ret = JIM_ERR;

    if (ct) {
        if ((ct->flags & JIM_MODFLAG_NOTAINT) && Jim_CheckTaint(interp, JIM_TAINT_ANY)) {
            ret = JIM_SUBCMD_TAINTED;
        }
        else if (ct->flags & JIM_MODFLAG_FULLARGV) {
            ret = ct->function(interp, argc, argv);
        }
        else {
            ret = ct->function(interp, argc - 2, argv + 2);
        }
        if (ret < 0) {
            if (ret == JIM_SUBCMD_TAINTED) {
                Jim_SetTaintError(interp, 2, argv);
            }
            else {
                Jim_SubCmdArgError(interp, ct, argv[0]);
            }
            ret = JIM_ERR;
        }

```

jim 0.84-9-g5bac7c9, revision `Exact original193/194 configured source/header/library/executable association retained in the corpus.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_CompareStringImmediate`, lines 3160–3177. Retained reference `info196-Jim_CompareStringImmediate`.

```text
int Jim_CompareStringImmediate(Jim_Interp *interp, Jim_Obj *objPtr, const char *str)
{
    if (objPtr->typePtr == &comparedStringObjType && objPtr->internalRep.ptr == str) {
        return 1;
    }
    else {
        if (strcmp(str, Jim_String(objPtr)) != 0)
            return 0;

        if (objPtr->typePtr != &comparedStringObjType) {
            Jim_FreeIntRep(interp, objPtr);
            objPtr->typePtr = &comparedStringObjType;
        }
        objPtr->internalRep.ptr = (char *)str;  /*ATTENTION: const cast */
        return 1;
    }
}

```

jim 0.84-9-g5bac7c9, revision `Exact original193/194 configured source/header/library/executable association retained in the corpus.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_ListJoin`, lines 7522–7535. Retained reference `info196-Jim_ListJoin`.

```text
Jim_Obj *Jim_ListJoin(Jim_Interp *interp, Jim_Obj *listObjPtr, const char *joinStr, int joinStrLen)
{
    int i;
    int listLen = Jim_ListLength(interp, listObjPtr);
    Jim_Obj *resObjPtr = Jim_NewEmptyStringObj(interp);

    for (i = 0; i < listLen; ) {
        Jim_AppendObj(interp, resObjPtr, Jim_ListGetIndex(interp, listObjPtr, i));
        if (++i != listLen) {
            Jim_AppendString(interp, resObjPtr, joinStr, joinStrLen);
        }
    }
    return resObjPtr;
}

```

jim 0.84-9-g5bac7c9, revision `Exact original193/194 configured source/header/library/executable association retained in the corpus.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_SetResultFormatted`, lines 17127–17183. Retained reference `info196-Jim_SetResultFormatted`.

```text
void Jim_SetResultFormatted(Jim_Interp *interp, const char *format, ...)
{
    /* Initial space needed */
    int len = strlen(format);
    int extra = 0;
    int n = 0;
    const char *params[5];
    int nobjparam = 0;
    Jim_Obj *objparam[5];
    char *buf;
    va_list args;
    int i;

    va_start(args, format);

    for (i = 0; i < len && n < 5; i++) {
        int l;

        if (strncmp(format + i, "%s", 2) == 0) {
            params[n] = va_arg(args, char *);

            l = strlen(params[n]);
        }
        else if (strncmp(format + i, "%#s", 3) == 0) {
            Jim_Obj *objPtr = va_arg(args, Jim_Obj *);

            params[n] = Jim_GetString(objPtr, &l);
            objparam[nobjparam++] = objPtr;
            Jim_IncrRefCount(objPtr);
        }
        else {
            if (format[i] == '%') {
                i++;
            }
            continue;
        }
        n++;
        extra += l;
    }

    len += extra;
    buf = Jim_Alloc(len + 1);
    len = snprintf(buf, len + 1, format, params[0], params[1], params[2], params[3], params[4]);

    va_end(args);

    Jim_SetResult(interp, Jim_NewStringObjNoAlloc(interp, buf, len));

    for (i = 0; i < nobjparam; i++) {
        Jim_DecrRefCount(interp, objparam[i]);
    }
}

/* Should be called as the first thing in a loadable module to verify
 * that the interpeter ABI is compatible with the ABI that the module was compiled against.
 * Returns JIM_ERR and sets an error if mismatch.
 */

```

jim 0.84-9-g5bac7c9, revision `exact configured native source pinned in receipt`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_InfoCoreCommand`, lines 16070–16080. Retained reference `jim-exists222-original-reader`.

```text
    }
    /* (ct - cmds) is the index into the table */
    int option = ct - cmds;

    switch (option) {
        case INFO_EXISTS:
            Jim_SetResultBool(interp, Jim_GetVariable(interp, argv[2], 0) != NULL);
            return JIM_OK;

        case INFO_ALIAS:{
            Jim_Cmd *cmdPtr;

```

## Consumer bindings

- [rust/tcl-registry/src/commands/tcl/info_jim.rs](../../../../rust/tcl-registry/src/commands/tcl/info_jim.rs), `spec`: Jim source metadata owns its operation roster/arity/roles and conservative current helper effects; no C worker or successful-handler transfer.
- [rust/tcl-registry/src/commands/tcl/info_jim.rs](../../../../rust/tcl-registry/src/commands/tcl/info_jim.rs), `native_jim_info_member_names`: Pure independently recipe-selected stock table; no current registration, lookup/cache or compiler preparation grant.
- [rust/tcl-registry/src/commands/tcl/info_jim.rs](../../../../rust/tcl-registry/src/commands/tcl/info_jim.rs), `InvocationDialect::native_jim_info_protocol`: Select only the independently retained Jim084 parser recipe; C/foreign/unavailable issuers decline, no current registration or physical cache issued.
- [rust/tcl-registry/src/commands/tcl/info_jim.rs](../../../../rust/tcl-registry/src/commands/tcl/info_jim.rs), `NativeJimInfoProtocol::dispatch`: Share original strcmp immediate matching, counted-length strncmp abbreviations and original operand ordinals including -help/-commands/-nons; no manufactured source or cache capability.
- [rust/tcl-registry/src/commands/tcl/info_jim.rs](../../../../rust/tcl-registry/src/commands/tcl/info_jim.rs), `NativeJimInfoReport::message`: Source-selected Jim AppendStrings/printf reporting extents; -commands retains counted helper result independently of formatted CString fields.
- [rust/tcl-vm/src/cmd_info/native_jim.rs](../../../../rust/tcl-vm/src/cmd_info/native_jim.rs), `report`: Invoke the actual current lsort with the genuine member list, preserve original head/selector objects, join checked result list objects only on helper success, and keep host refusals terminal.
- [runtime/rust/src/cmd_info/native_jim.rs](../../../../runtime/rust/src/cmd_info/native_jim.rs), `report`: Use the same selected parser/report receipt with real owning list/head/result objects across current lsort dispatch; no static callback-free replacement or C ensemble path.
- [rust/tcl-registry/src/commands/tcl/info_jim.rs](../../../../rust/tcl-registry/src/commands/tcl/info_jim.rs), `existence`: Select Jim existence source semantics with generic InfoExists identity and independent native NoHook grammar; no C worker or successful-handler transfer.
- [rust/tcl-registry/src/commands/tcl/info_jim.rs](../../../../rust/tcl-registry/src/commands/tcl/info_jim.rs), `commands::tcl::info_jim::tests::original_jim_info_inventory_and_arities_match_native_rows` (linked): Compare27 selected Jim rows against native -commands and each excessive-argc usage; check actual nearest-family selection, C/foreign/unavailable protocol negatives, optional missing-delimiter variable role and no borrowed C inline/success descriptors. Linked only; no Rust execution claim.
- [rust/tcl-registry/src/commands/tcl/info_jim.rs](../../../../rust/tcl-registry/src/commands/tcl/info_jim.rs), `commands::tcl::info_jim::tests::original_jim_parser_keeps_immediate_and_counted_prefix_purposes_separate` (linked): Implementation-only raw-zero versus encoded-zero selection/report and internal ordinal/help cases derive from exact retained Jim strcmp/strncmp/printf windows. These are not new native raw-byte observations, object cache or execution claims.
- [rust/tcl-vm/src/cmd_info/native_jim.rs](../../../../rust/tcl-vm/src/cmd_info/native_jim.rs), `cmd_info::native_jim::tests::original_jim_info_parser_reports_match_all_native_info_rows` (linked): Compare completion and exact public result bytes for all51 retained Jim info argv controls, each in a fresh actual Jim core; namespace ensemble map is a separate command and excluded. Linked only, no executed Rust claim.
- [rust/tcl-vm/src/cmd_info/native_jim.rs](../../../../rust/tcl-vm/src/cmd_info/native_jim.rs), `cmd_info::native_jim::tests::current_lsort_failure_is_reported_by_the_selected_jim_parser` (linked): Source-only implementation discriminator uses a genuinely replaced current lsort; its failure text feeds an outer Error miss and successful -commands report. No new native replacement-helper observation or successful arbitrary member claim.
- [runtime/rust/src/cmd_info/native_jim.rs](../../../../runtime/rust/src/cmd_info/native_jim.rs), `cmd_info::native_jim::tests::original_jim_info_parser_reports_match_all_native_info_rows` (linked): Compare completion and exact public result bytes for all51 retained Jim info argv controls, each in a fresh actual Jim core; namespace ensemble map is a separate command and excluded. Linked only, no executed Rust claim.
- [runtime/rust/src/cmd_info/native_jim.rs](../../../../runtime/rust/src/cmd_info/native_jim.rs), `cmd_info::native_jim::tests::current_lsort_failure_is_reported_by_the_selected_jim_parser` (linked): Source-only implementation discriminator uses a genuinely replaced current lsort; its failure text feeds an outer Error miss and successful -commands report. No new native replacement-helper observation or successful arbitrary member claim.
- [rust/tcl-registry/src/commands/tcl/info_jim.rs](../../../../rust/tcl-registry/src/commands/tcl/info_jim.rs), `commands::tcl::info_jim::tests::original_jim_existence_query_shares_operation_without_c_compiler_admission` (linked): The selected Jim exists source descriptor shares InfoExists operation, retains NoHook native compilation and naming-only VarRead ordinal0. This marked metadata assertion supplies no observed presence result, value read, handler success or compiler admission.

## Evidence

- [rust/tcl-registry/tests/data/native_info_inventory_original](../../../../rust/tcl-registry/tests/data/native_info_inventory_original)
- [rust/tcl-registry/tests/data/native_info_inventory_original/jim](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/jim)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim)
