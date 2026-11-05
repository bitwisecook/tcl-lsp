# naming.jim.rooted-alias-publication-and-lookup

Kind: `native-observation`

## Problem statement

An alias command can retain its original returned spelling while replacing a procedure and supporting multiple rooted call spellings. Those public distinctions require independent measurements before assigning source or port comparison keys, and namespace-local competition remains a separate lookup condition.

## Question

What original alias spellings, caught call results and body-query statuses do rooted, multiword and extra-root publications produce, and how do independently authored namespace-local and global commands compete?

## Conclusion

Recorded Jim 0.84-9-g5bac7c9 has the standalone alias command. A rooted alias publication returns ::alias_slot and both bare/rooted caught calls return ROOT_ALIAS; info body on the authored procedure name fails with code one. A rooted multiword alias returns {::AliasProbe slot}, both call spellings return MULTIWORD_ALIAS, and the body query fails. An extra-root publication returns ::::colon_slot while bare, two-colon and four-colon calls return COLON_ALIAS. Namespace competition returns GLOBAL_ALIAS from the global/rooted calls and LOCAL_PROC from the independently authored local procedure. The exact returned spellings and caught codes are retained in four rows. All five stock C processes expose no standalone alias command and print alias_publication_scope not_applicable; no interp alias substitute is executed or inferred. Six processes exit zero with empty stderr. Two independently pinned Jim source excerpts retain original command-name object registration separately from counted leading-root stripping in command-table hash/compare; source inspection is explanatory association rather than native table introspection or another executed control. BIG-IP is not tested. These ASCII results supply no NUL/Unicode, private alias/object/header/frame identity, complete namespace resolution, full OO/package, original source-carrier authority or edit claim. The Runtime standalone-source comparison selects its actual Jim core alias registration and consumes the whole original194 probe and four public rows, keeping reported version metadata separate. jim_alias retains the original name result and original prefix members through install_alias_with_original; those software object owners are separate from the original public byte results and do not obtain private provider identity from stdout. Alias dispatch uses its selected actual caller context; C core does not acquire a standalone alias command from this Jim purpose.

## Scope

One fixed ASCII LF CLI file per fresh provider process uses the original standalone alias command, returned public naming spelling, caught ordinary call results and info-body status. Four exact controls cover rooted, multiword and extra-root spellings plus independent namespace/global competition. No NUL, non-ASCII, continuation/control escape, alias enumeration, arbitrary namespace path or class/package branch is tested. Exact input/command/executable/library/header/build/source associations are checked independently. Source hash/registration excerpts do not prove compiled-source identity or a cache/object lifetime law. Runtime API and VM consumers require their separate supported ingress and source requirements; matching caller projection cannot attest an unavailable standalone port alias command. A marked Runtime control compares all four unchanged Jim original public rows under actual core alias registration. Its linked definition is implementation coverage, not an executed Rust pass, new native run or public header/pointer observer. Actual source-distribution bootstrap and C interp/child alias APIs retain their separate purposes.

## Provider answers

### tcl8.4

Status: `unsupported`. Channel: One fixed ASCII LF CLI source file. Public alias return spelling, caught command invocation values and info-body success code only. No NUL, non-ASCII, private table/frame/object identity, alias-query enumeration, or class/package scope claims. Version: tcl8.4 8.4.20.

Actual stock 8.4.20 exposes no standalone alias and prints alias_publication_scope not_applicable. All four alias controls are unexecuted here; no interp alias or another installation/package capability is substituted. Process exits zero with empty stderr.

### tcl8.5

Status: `unsupported`. Channel: One fixed ASCII LF CLI source file. Public alias return spelling, caught command invocation values and info-body success code only. No NUL, non-ASCII, private table/frame/object identity, alias-query enumeration, or class/package scope claims. Version: tcl8.5 8.5.19.

Actual stock 8.5.19 exposes no standalone alias and prints alias_publication_scope not_applicable. All four alias controls are unexecuted here; no interp alias or another installation/package capability is substituted. Process exits zero with empty stderr.

### tcl8.6

Status: `unsupported`. Channel: One fixed ASCII LF CLI source file. Public alias return spelling, caught command invocation values and info-body success code only. No NUL, non-ASCII, private table/frame/object identity, alias-query enumeration, or class/package scope claims. Version: tcl8.6 8.6.18.

Actual stock 8.6.18 exposes no standalone alias and prints alias_publication_scope not_applicable. All four alias controls are unexecuted here; no interp alias or another installation/package capability is substituted. Process exits zero with empty stderr.

### tcl9.0

Status: `unsupported`. Channel: One fixed ASCII LF CLI source file. Public alias return spelling, caught command invocation values and info-body success code only. No NUL, non-ASCII, private table/frame/object identity, alias-query enumeration, or class/package scope claims. Version: tcl9.0 9.0.4.

Actual stock 9.0.4 exposes no standalone alias and prints alias_publication_scope not_applicable. All four alias controls are unexecuted here; no interp alias or another installation/package capability is substituted. Process exits zero with empty stderr.

### tcl9.1

Status: `unsupported`. Channel: One fixed ASCII LF CLI source file. Public alias return spelling, caught command invocation values and info-body success code only. No NUL, non-ASCII, private table/frame/object identity, alias-query enumeration, or class/package scope claims. Version: tcl9.1 9.1.0.

Actual stock 9.1.0 exposes no standalone alias and prints alias_publication_scope not_applicable. All four alias controls are unexecuted here; no interp alias or another installation/package capability is substituted. Process exits zero with empty stderr.

### jim

Status: `observed`. Channel: One fixed ASCII LF CLI source file. Public alias return spelling, caught command invocation values and info-body success code only. No NUL, non-ASCII, private table/frame/object identity, alias-query enumeration, or class/package scope claims. Version: jim 0.84-9-g5bac7c9.

Actual Jim 0.84-9-g5bac7c9 exposes standalone alias. Exact public rows: rooted_replacement ::alias_slot 0 ROOT_ALIAS 0 ROOT_ALIAS 1; multiword_replacement {::AliasProbe slot} 0 MULTIWORD_ALIAS 0 MULTIWORD_ALIAS 1; extra_root_extent ::::colon_slot 0 COLON_ALIAS 0 COLON_ALIAS 0 COLON_ALIAS; namespace_competition ::competition_slot 0 GLOBAL_ALIAS 0 LOCAL_PROC 0 GLOBAL_ALIAS. Returned original alias spellings remain distinct from their bare/rooted/extra-root caller results. Prior procedure info-body queries fail; namespace-local competition returns its independent LOCAL_PROC while root calls return GLOBAL_ALIAS. Process exits zero with empty stderr. No private alias table/object/header/frame or untested byte extent is observed.

### bigip

Status: `not-tested`.

No appliance execution answers this standalone Jim alias question; stock C/Jim caller projections supply no hosted result.

## Source inspection

jim 0.84-9-g5bac7c9, revision `pinned Jim source associated with retained library/executable`, `jim.c`, function `Jim_GetStringNoQualifier/JimCommandsHT_HashFunction/JimCommandsHT_KeyCompare`, lines 4030–4059. Retained reference `alias194-source-anchors-Jim_GetStringNoQualifier.c`.

```text
static const char *Jim_GetStringNoQualifier(Jim_Obj *objPtr, int *length)
{
    int len;
    const char *str = Jim_GetString(objPtr, &len);
    if (len >= 2 && str[0] == ':' && str[1] == ':') {
        while (len && *str == ':') {
            len--;
            str++;
        }
    }
    *length = len;
    return str;
}

/* Hash a command-table key, ignoring any namespace prefix. */
static unsigned int JimCommandsHT_HashFunction(const void *key)
{
    int len;
    const char *str = Jim_GetStringNoQualifier((Jim_Obj *)key, &len);
    return Jim_GenHashFunction((const unsigned char *)str, len);
}

/* Compare command names, allowing namespace-qualified lookups to match local entries. */
static int JimCommandsHT_KeyCompare(void *privdata, const void *key1, const void *key2)
{
    int len1, len2;
    const char *str1 = Jim_GetStringNoQualifier((Jim_Obj *)key1, &len1);
    const char *str2 = Jim_GetStringNoQualifier((Jim_Obj *)key2, &len2);
    return len1 == len2 && *str1 == *str2 && memcmp(str1, str2, len1) == 0;
}

```

jim 0.84-9-g5bac7c9, revision `pinned Jim source associated with retained library/executable`, `jim.c`, function `Jim_RegisterCommand`, lines 4168–4197. Retained reference `alias194-source-anchors-Jim_RegisterCommand.c`.

```text
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

```

## Consumer bindings

- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `jim_alias`: Implement the selected actual Jim standalone core command with original alias-name result and retained original prefix objects; public stdout supplies no private object identity.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::install_alias_with_original`: Keep authentic original prefix members through alias installation independently of rendered names, source advice and separate child crossing purposes.
- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `cmd_alias::tests::jim_alias_owner_replaces_canonical_root_slots_and_keeps_local_competition` (linked): Supported Runtime alias installation API compares measured invocation projection, shared root replacement and local/global competition; it attests no absent standalone alias-command return spelling.
- [rust/tcl-vm/tests/native_compilation_conformance.rs](../../../../rust/tcl-vm/tests/native_compilation_conformance.rs), `native_compilation_conformance::jim_rooted_alias_publication_matches_original_native_source_and_values` (linked): Full original CLI source and exact Jim stdout compare four complete public control rows after provider-version metadata. Test binding supplies no additional native process or private cell identity.
- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `cmd_alias::tests::original_jim_core_alias_matches_all_four_standalone_native_public_rows` (linked): Actual Jim core registration runs the untouched original194 probe and compares all four whole public rows with reported version metadata excluded; no private object/header/frame identity follows from native stdout, and linked status claims no executed pass.

## Evidence

- [rust/tcl-registry/tests/data/native_jim_alias_publication194](../../../../rust/tcl-registry/tests/data/native_jim_alias_publication194)
- [rust/tcl-registry/tests/data/native_jim_alias_publication194/8.4.20](../../../../rust/tcl-registry/tests/data/native_jim_alias_publication194/8.4.20)
- [rust/tcl-registry/tests/data/native_jim_alias_publication194/8.5.19](../../../../rust/tcl-registry/tests/data/native_jim_alias_publication194/8.5.19)
- [rust/tcl-registry/tests/data/native_jim_alias_publication194/8.6.18](../../../../rust/tcl-registry/tests/data/native_jim_alias_publication194/8.6.18)
- [rust/tcl-registry/tests/data/native_jim_alias_publication194/9.0.4](../../../../rust/tcl-registry/tests/data/native_jim_alias_publication194/9.0.4)
- [rust/tcl-registry/tests/data/native_jim_alias_publication194/9.1.0](../../../../rust/tcl-registry/tests/data/native_jim_alias_publication194/9.1.0)
- [rust/tcl-registry/tests/data/native_jim_alias_publication194/jim](../../../../rust/tcl-registry/tests/data/native_jim_alias_publication194/jim)
- [rust/tcl-registry/tests/data/native_jim_alias_publication194/source-anchors](../../../../rust/tcl-registry/tests/data/native_jim_alias_publication194/source-anchors)
