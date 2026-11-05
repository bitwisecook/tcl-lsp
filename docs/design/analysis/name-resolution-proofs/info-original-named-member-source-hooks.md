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

Status: `inspected`. Channel: Read-only whole source and exact selected table/selector excerpt. Version: tcl8.4 8.4.20.

Complete subCmds excerpt contains no cmdtype/constant/consts member rows.

### tcl8.5, tcl8.6

Status: `inspected`. Channel: Read-only whole source and exact selected table/selector excerpt. Version: tcl8.5 8.5.19; tcl8.6 8.6.18.

Complete defaultInfoMap excerpt contains no cmdtype/constant/consts member rows.

### tcl9.0

Status: `inspected`. Channel: Read-only whole source and exact selected table/selector excerpt. Version: tcl9.0 9.0.4.

Complete defaultInfoMap declares cmdtype and constant with TclCompileBasic1ArgCmd, consts with TclCompileBasic0Or1ArgCmd; cmdtype final flag1 remains exact. This is source declaration only, not actual installed hook/opcode admission.

### tcl9.1

Status: `inspected`. Channel: Read-only whole source and exact selected table/selector excerpt. Version: tcl9.1 9.1.0.

Complete tclInfoImplMap declares cmdtype and constant with TclCompileBasic1ArgCmd, consts with TclCompileBasic0Or1ArgCmd; cmdtype final flag1 remains exact. This is source declaration only, not actual installed hook/opcode admission.

### jim, bigip

Status: `not-tested`.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Source inspection

tcl8.4 8.4.20, revision `Pinned complete original provider source`, `/workspace/tcl-lsp/rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.4.20/tclCmdIL.c`, function `subCmds`, lines 425–435. Retained reference `info-named-member-source-8.4.20`.

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

tcl8.5 8.5.19, revision `Pinned complete original provider source`, `/workspace/tcl-lsp/rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.5.19/tclCmdIL.c`, function `defaultInfoMap`, lines 155–179. Retained reference `info-named-member-source-8.5.19`.

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

tcl8.6 8.6.18, revision `Pinned complete original provider source`, `/workspace/tcl-lsp/rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18/tclCmdIL.c`, function `defaultInfoMap`, lines 155–181. Retained reference `info-named-member-source-8.6.18`.

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

tcl9.0 9.0.4, revision `Pinned complete original provider source`, `/workspace/tcl-lsp/rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4/tclCmdIL.c`, function `defaultInfoMap`, lines 151–180. Retained reference `info-named-member-source-9.0.4`.

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

tcl9.1 9.1.0, revision `Pinned complete original provider source`, `/workspace/tcl-lsp/rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0/tclCmdIL.c`, function `tclInfoImplMap`, lines 152–181. Retained reference `info-named-member-source-9.1.0`.

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

## Evidence

- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.4.20/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.4.20/tclCmdIL.c)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.5.19/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.5.19/tclCmdIL.c)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18/tclCmdIL.c)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4/tclCmdIL.c)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0/tclCmdIL.c)
- [rust/tcl-registry/src/commands/tcl/info_.rs](../../../../rust/tcl-registry/src/commands/tcl/info_.rs)
