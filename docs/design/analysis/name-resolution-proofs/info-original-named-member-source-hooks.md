# naming.info.original-named-member-source-hooks

Kind: `source-anchor`

## Problem statement

The public info member name alone does not establish its release-selected compiler hook or named implementation registration. Exact pinned source tables are required for cmdtype, constant and consts without borrowing another release or physical token.

## Question

Which pinned C source info tables declare cmdtype, constant and consts and their Basic compiler hooks, and how is that source fact kept separate from captured Native compiler/worker admission?

## Conclusion

The inspected complete C8.4 subCmds selector table and C8.5/C8.6 defaultInfoMap contain no cmdtype, constant or consts member rows. C9.0 defaultInfoMap and C9.1 tclInfoImplMap declare cmdtype/InfoCmdTypeCmd/TclCompileBasic1ArgCmd, constant/TclInfoConstantCmd/TclCompileBasic1ArgCmd and consts/TclInfoConstsCmd/TclCompileBasic0Or1ArgCmd; the cmdtype row also retains its exact final flag1. Registry named_c9_member_compilation projects NamedEnsembleInvocation with implementation_from/hook_from C9.0, exact public arity and original named member lookup. This read-only source declaration fact supplies no current installed token, captured compiler selection, genuine opcode/argv, physical constant storage, successful Native worker or entered frame. Jim and hosted purposes cannot inherit those C source facts; no Jim/appliance source or native execution is measured here. Public info inventory and const-link observations remain independent questions.

## Scope

Five whole pinned canonical tclCmdIL.c files and exact selector/table excerpts retain original paths, line ranges and full-source/snippet SHA256. C8.4 ordinary subCmds is distinct from the C8.5/C8.6/C9 EnsembleImplMap declarations. The linked Registry source-row projection checks exact C5 declarations and release gating; it is not an executed Rust or Native assertion. No new native process, source file modification, provider source hash refresh or executable claim is made. Jim and BIG-IP are not tested.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Source-only exact pinned tclCmdIL.c; no interpreter/compiler execution.. Channel: Read-only whole source and exact selected table/selector excerpt.. Dialect: Pinned original C release source.

Complete subCmds excerpt contains no cmdtype/constant/consts member rows.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Source-only exact pinned tclCmdIL.c; no interpreter/compiler execution.. Channel: Read-only whole source and exact selected table/selector excerpt.. Dialect: Pinned original C release source.

Complete defaultInfoMap excerpt contains no cmdtype/constant/consts member rows.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Source-only exact pinned tclCmdIL.c; no interpreter/compiler execution.. Channel: Read-only whole source and exact selected table/selector excerpt.. Dialect: Pinned original C release source.

Complete defaultInfoMap excerpt contains no cmdtype/constant/consts member rows.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Source-only exact pinned tclCmdIL.c; no interpreter/compiler execution.. Channel: Read-only whole source and exact selected table/selector excerpt.. Dialect: Pinned original C release source.

Complete defaultInfoMap declares cmdtype and constant with TclCompileBasic1ArgCmd, consts with TclCompileBasic0Or1ArgCmd; cmdtype final flag1 remains exact. This is source declaration only, not actual installed hook/opcode admission.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Source-only exact pinned tclCmdIL.c; no interpreter/compiler execution.. Channel: Read-only whole source and exact selected table/selector excerpt.. Dialect: Pinned original C release source.

Complete tclInfoImplMap declares cmdtype and constant with TclCompileBasic1ArgCmd, consts with TclCompileBasic0Or1ArgCmd; cmdtype final flag1 remains exact. This is source declaration only, not actual installed hook/opcode admission.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Exact evidence

- `info-original-named-member-hooks262-inspection` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/basic-hooks-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/basic-hooks-source-anchors.json). SHA-256 `2f24d93e85026c050702cdc1bb483dbf44f053f93fe995cd8d47d30b33f687cd`. Exact read-only source inspection table/selector line ranges, original full source and excerpt digests; no native or Rust process observation.
- `info-named-member-source-8.4.20` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.4.20/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.4.20/tclCmdIL.c). SHA-256 `92695cd7731ce21ab5441fb6ce2d23e5cb44f020256a607397ebe836b5a9b14a`. Whole independently pinned original C provider source. Source table declarations are separate from actual runtime/compiler admission.
- `info-named-member-source-8.5.19` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.5.19/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.5.19/tclCmdIL.c). SHA-256 `51be49bd7e373aa319cb2affb341853473cbbf089694650bfff7689d3c925d8e`. Whole independently pinned original C provider source. Source table declarations are separate from actual runtime/compiler admission.
- `info-named-member-source-8.6.18` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18/tclCmdIL.c). SHA-256 `26d3d423720df253aefd7ad412e939165282a1f80fd6cd8c42900875d9627a23`. Whole independently pinned original C provider source. Source table declarations are separate from actual runtime/compiler admission.
- `info-named-member-source-9.0.4` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4/tclCmdIL.c). SHA-256 `af564d5d77f8f70cf120dbd49509a8580130cc89352f52424a01920fb0da4fa7`. Whole independently pinned original C provider source. Source table declarations are separate from actual runtime/compiler admission.
- `info-named-member-source-9.1.0` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0/tclCmdIL.c). SHA-256 `d5b92a5b9c8c923142db6b76f816cbe6ca857255861a3c3dc53787fa9b335e5a`. Whole independently pinned original C provider source. Source table declarations are separate from actual runtime/compiler admission.
- `naming-info-original-named-member-source-hooks-info_.rs` (implementation): [rust/tcl-registry/src/commands/tcl/info_.rs](../../../../rust/tcl-registry/src/commands/tcl/info_.rs). SHA-256 `cdd7f6374d370314fb202a062c17edb0a7e066d1fa2b9b65642410c23244383b`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.

## Source inspection

tcl8.4 8.4.20, revision `Pinned complete original provider source`, `/workspace/tcl-lsp/rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.4.20/tclCmdIL.c`, function `subCmds`, lines 425–435. Full-source SHA-256 `92695cd7731ce21ab5441fb6ce2d23e5cb44f020256a607397ebe836b5a9b14a`; snippet SHA-256 `b4bfdb3fb6e7dbf4f781d0f95064579d75d4b75453b24a6d48f65df175da96a6`; retained evidence `info-named-member-source-8.4.20`.

```text
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

```

tcl8.5 8.5.19, revision `Pinned complete original provider source`, `/workspace/tcl-lsp/rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.5.19/tclCmdIL.c`, function `defaultInfoMap`, lines 155–179. Full-source SHA-256 `51be49bd7e373aa319cb2affb341853473cbbf089694650bfff7689d3c925d8e`; snippet SHA-256 `995bafaf40d8072c0cc4184721765f5da7bed258bca61e58142d87f0ac2a32f3`; retained evidence `info-named-member-source-8.5.19`.

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

tcl8.6 8.6.18, revision `Pinned complete original provider source`, `/workspace/tcl-lsp/rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18/tclCmdIL.c`, function `defaultInfoMap`, lines 155–181. Full-source SHA-256 `26d3d423720df253aefd7ad412e939165282a1f80fd6cd8c42900875d9627a23`; snippet SHA-256 `703fc9501224cc267f80172c041d74b0b5497403423e4ae8767c34c949c4d48a`; retained evidence `info-named-member-source-8.6.18`.

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

```

tcl9.0 9.0.4, revision `Pinned complete original provider source`, `/workspace/tcl-lsp/rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4/tclCmdIL.c`, function `defaultInfoMap`, lines 151–180. Full-source SHA-256 `af564d5d77f8f70cf120dbd49509a8580130cc89352f52424a01920fb0da4fa7`; snippet SHA-256 `f009f3959451b71592a39bd08dc04346eace9fb33f17d9d199f09d39dfc6a664`; retained evidence `info-named-member-source-9.0.4`.

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

```

tcl9.1 9.1.0, revision `Pinned complete original provider source`, `/workspace/tcl-lsp/rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0/tclCmdIL.c`, function `tclInfoImplMap`, lines 152–181. Full-source SHA-256 `d5b92a5b9c8c923142db6b76f816cbe6ca857255861a3c3dc53787fa9b335e5a`; snippet SHA-256 `28b0dddebe9a2046b4220c08e3c970fbb246f8f88402c4417988a90421cf63fd`; retained evidence `info-named-member-source-9.1.0`.

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

```


## Consumer bindings

- [rust/tcl-registry/src/commands/tcl/info_.rs](../../../../rust/tcl-registry/src/commands/tcl/info_.rs), `named_c9_member_compilation`: Project exact C9 named Basic member source declarations with release floor/arity while keeping actual captured compiler/token/storage admission independent.
- [rust/tcl-registry/src/commands/tcl/info_.rs](../../../../rust/tcl-registry/src/commands/tcl/info_.rs), `commands::tcl::info_::tests::original_c9_basic_info_members_keep_their_selected_source_hooks` (linked): Exact whole C5 source tables and source rows retain cmdtype/constant Basic1 and consts Basic0Or1 declarations only in C9; Registry named registration descriptor retains implementation/hook floor and lookup member. Source projections do not assert a live Native token/compiler/constant table.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named Rust coverage is linked, not an executed-result claim. Exact source/executable receipts retain actual outcomes separately. Native evidence and runtime/effect/frame/edit obligations remain independent.
