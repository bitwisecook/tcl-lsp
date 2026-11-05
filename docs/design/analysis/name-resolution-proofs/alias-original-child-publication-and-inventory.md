# naming.alias.original-child-publication-and-inventory

Kind: `native-observation`

## Problem statement

Child-alias publication, counted invocation and public command/alias inventories use distinct provider API paths. A child declaration cannot borrow the standalone core-alias name storage or inventory purpose; rooted spelling, produced NUL/FF bytes and literal Unicode need their own original controls.

## Question

For each unchanged original child-alias source, which name is published, callable and independently reported by command and alias inventories on the six authentic providers, including rooted spelling and counted binary/Unicode boundaries?

## Conclusion

All thirty-six fresh originals complete with outer guest code 0; all host processes exit 0 with empty stderr. The five C releases report successful definition and VALUE calls in every case. Their command inventory unroots the two rooted names while interp aliases retains the original rooted report; produced NUL names remain callable, with their counted public member and encoding fields retained. Jim reports successful definition with an empty result, retains rooted command reports, and returns VALUE for rooted, extra-rooted, literal Unicode and produced FF names. Its two produced-NUL declarations publish the prefix reported by command inventory, while calling the original counted name fails; both whole errors remain retained. Jim child info aliases is empty in every case. The incompatible opposite alias API is independently caught on every provider. Exact read-only Jim source shows child JimInterpAliasProc installation through Jim_RegisterCmd/Jim_NewStringObj(...,-1), and retains the separate Jim_RegisterCommand/Jim_CreateCommand registration entry. These source facts and public inventories supply no private key/header/table/prefix identity, actual parent-resource ownership or software assertion pass.

## Scope

Six unchanged LF originals use genuinely available C interp alias versus Jim child-handle alias APIs in fresh fully initialized interpreters. Five sources are ASCII; one contains literal UTF-8 U+00E9. NUL and FF are produced by binary format H* and are absent as literal source bytes. NAME_BEFORE, caught definition, counted call, command inventory, selected alias inventory and opposite-surface failure are independent original source fields. C inventory member hex uses explicit encoding convertto utf-8; Jim uses raw binary scan. Whole result order, original character lengths, hex and every caught message are retained without normalisation or a hash-order contract. The counted-source driver adds no original script envelope. Complete actual request/driver/source/compiler/ELF/process/version/stdout/stderr and required pins are preserved. Exact Jim child installer/macro and command registration source excerpts are separate read-only evidence; public empty aliases does not identify a private class or table. BIG-IP is not tested, and no general alias, child bootstrap, header/cache/frame/compiler admission or runtime result follows beyond these fixed controls. The shared ChildAliasPublication source projection retains the complete counted original input, selected C-string registration extent and independently selected alias report. C alias registration reports retain original counted naming units; Jim child reports use its actual C-string registration extent, separately from standalone counted AliasPublication. These pure projections and actual Runtime/VM parent/child owner adapters grant no original native header, prefix pointer, current table or entered frame. The linked Runtime and VM controls each compare all thirty-six whole original public completion/results, including opposite-API failure and counted-call behavior; their definitions establish no executed software pass. Missing/foreign issuer controls are independently authored host assertions and supply no Native failure observation. The independently pinned Jim_GetCommand source retains its exact invalid-command producer: on original lookup failure it passes the original object to Jim_SetResultFormatted with the %#s conversion. This source interpretation remains separate from the fixed produced-NUL Native298 diagnostic rows and from command registration/inventory extents. Runtime invalid_command_result and VM command_lookup_error_value consume the shared CommandLookupError report purpose under the actual engine policy rather than a local registration-name projection. The same unchanged thirty-six-window comparator definition supplies no current executed pass; independently authored child refusal state is not a Native failure observation. Exact Jim_SetResultFormatted source obtains the counted object string length for allocation, then uses final snprintf string formatting and its returned length for the result object. These source facts describe that producer/formatter boundary only; they establish no new executed NUL specimen, original header identity or software pass.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Six independent fresh fullInit counted original source evaluations; actual VERSION/ORIGINAL fields and original caught APIs, no added envelope or normalisation. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

All six original outer completions, child declarations and calls are0; every counted call returns VALUE. Command inventory unroots rooted ::/:::: names, while interp aliases reports original rooted spelling. Produced NUL names remain callable with whole counted original reports retained; literal U+00E9 and produced FF retain their explicit original versus encoding-convertto-UTF8 inventory channels. The opposite info aliases API is independently caught1 with this release's whole diagnostic. Exact character lengths, member hex and complete result bytes are retained. Every fresh host process exits0 with empty stderr. Public outcomes supply no private name/header/table/prefix identity or software result.

### jim

Status: `observed`. Channel: Six independent fresh fullInit counted original source evaluations; actual VERSION/ORIGINAL fields and original caught APIs, no added envelope or normalisation. Version: jim 0.84-9-g5bac7c9.

All six original outer completions and child declarations are 0, with empty declaration result. Rooted reports retain :: and ::::; counted rooted/Unicode/produced-FF calls return VALUE. Produced NUL before/after suffix publishes command report r2286/r2286_tail, while original counted calls return caught code1 and their retained invalid-command diagnostics. All six child info aliases inventories are empty. The opposite interp aliases API is separately caught1. Exact raw member length/hex, complete results and source encoding channels remain independent. Every fresh host process exits0 with empty stderr. Public outcomes supply no private name/header/table/prefix identity or software result.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Source inspection

jim 0.84-9-g5bac7c9; source inspection distinct from public execution, revision `Exact independently pinned source SHA; no source-control revision beyond observed version string claimed.`, `/workspace/.proofs/native-providers/jimtcl/jim-interp.c`, function `interp_cmd_alias/JimInterpAliasProc`, lines 58–94. Retained reference `native_child_alias_inventory298-request-jim-interp.c`.

```text
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

```

jim 0.84-9-g5bac7c9; source inspection distinct from public execution, revision `Exact independently pinned source SHA; no source-control revision beyond observed version string claimed.`, `/workspace/.proofs/native-providers/jimtcl/jim.h`, function `Jim_RegisterCmd`, lines 930–938. Retained reference `native_child_alias_inventory298-request-jim.h`.

```text
/* Simplify creating commands that specify minargs, maxargs and usage but
 * don't need delProc or privData
 */
#define Jim_RegisterSimpleCmd(interp, name, usage, minargs, maxargs, cmdproc) \
        Jim_RegisterCommand(interp, Jim_NewStringObj(interp, name, -1), cmdproc, NULL, usage, NULL, minargs, maxargs, 0, NULL)
/* And also slightly more complex where delProc, privData and flags may be needed */
#define Jim_RegisterCmd(interp, name, usage, minargs, maxargs, cmdproc, delproc, privdata, flags) \
        Jim_RegisterCommand(interp, Jim_NewStringObj(interp, name, -1), cmdproc, delproc, usage, NULL, minargs, maxargs, flags, privdata)

```

jim 0.84-9-g5bac7c9; source inspection distinct from public execution, revision `Exact independently pinned source SHA; no source-control revision beyond observed version string claimed.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_RegisterCommand/Jim_CreateCommand`, lines 4159–4209. Retained reference `native_child_alias_inventory298-request-jim.c`.

```text

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

```

jim 0.84-9-g5bac7c9; exact independently pinned source inspection, revision `Exact retained source SHA; no new process or source-control revision claimed.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_GetCommand`, lines 4546–4579. Retained reference `native_child_alias_inventory298-jim.c`.

```text
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

```

jim 0.84-9-g5bac7c9; exact independently pinned source inspection, revision `Exact retained source SHA; no new process or source-control revision claimed.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_SetResultFormatted`, lines 17127–17183. Retained reference `native_child_alias_inventory298-jim.c`.

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

## Consumer bindings

- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::child_alias_publication_input`: Retain the genuine counted child-alias naming input as a purpose separate from standalone alias publication.
- [rust/tcl-syntax/src/naming/native/routing.rs](../../../../rust/tcl-syntax/src/naming/native/routing.rs), `NativeNameProtocol::child_alias_publication_projection`: Keep original counted input, selected C-string registration and provider-selected alias report in distinct typed fields.
- [runtime/rust/src/interp/native_object_vector.rs](../../../../runtime/rust/src/interp/native_object_vector.rs), `Interp::invalid_command_result`: Present the original command lookup failure through shared CommandLookupError reporting under actual name policy, independently of child registration extent.
- [rust/tcl-vm/src/exec.rs](../../../../rust/tcl-vm/src/exec.rs), `command_lookup_error_value`: Consume the same shared original lookup-error naming purpose without donating registration/header identity.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `naming::native::child_alias_publication_has_its_own_counted_input_and_c_string_registration` (linked): Across selected C releases and Jim, rooted, NUL, Unicode and FF source inputs retain complete original units separately from C-string registration. Jim reports that registration extent while C alias reports retain counted input; standalone alias publication remains distinct. This pure control asserts no native pointer or current table.
- [runtime/rust/src/interp/native_child_alias_publication_tests.rs](../../../../runtime/rust/src/interp/native_child_alias_publication_tests.rs), `interp::native_child_alias_publication_tests::original_child_alias_publication_matches_all_36_native_public_windows` (linked): Each backend definition compares the exact six unchanged originals across all six actual provider columns: complete definition/call/inventory/opposite-API completion/results remain independent. This links fixed public observations and supplies no executed software result, native private identity or parent-resource ownership proof.
- [rust/tcl-vm/src/interp/native_child_alias_publication_tests.rs](../../../../rust/tcl-vm/src/interp/native_child_alias_publication_tests.rs), `interp::native_child_alias_publication_tests::original_child_alias_publication_matches_all_36_native_public_windows` (linked): Each backend definition compares the exact six unchanged originals across all six actual provider columns: complete definition/call/inventory/opposite-API completion/results remain independent. This links fixed public observations and supplies no executed software result, native private identity or parent-resource ownership proof.

## Evidence

- [rust/tcl-registry/tests/data/native_child_alias_inventory298](../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298)
- [rust/tcl-registry/tests/data/native_child_alias_inventory298/8.4.20](../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/8.4.20)
- [rust/tcl-registry/tests/data/native_child_alias_inventory298/8.5.19](../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/8.5.19)
- [rust/tcl-registry/tests/data/native_child_alias_inventory298/8.6.18](../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/8.6.18)
- [rust/tcl-registry/tests/data/native_child_alias_inventory298/9.0.4](../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/9.0.4)
- [rust/tcl-registry/tests/data/native_child_alias_inventory298/9.1.0](../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/9.1.0)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_child_alias_inventory298/jim](../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/jim)
