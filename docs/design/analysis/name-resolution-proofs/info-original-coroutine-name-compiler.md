# naming.info.original-coroutine-name-compiler

Kind: `source-anchor`

## Problem statement

A public coroutine-name result does not establish which original C info table registers its compiler or the private worker argument shape and emitted instruction. Source declarations, current shared recipes and actual Native compiler/token admission need separate evidence.

## Question

Which original C info tables register the coroutine-name compiler, and what argument shape and instruction does its original worker define?

## Conclusion

The complete pinned C8.4 and C8.5 info tables contain no coroutine member. C8.6 and C9.0 defaultInfoMap and C9.1 tclInfoImplMap register TclInfoCoroutineCmd with TclCompileInfoCoroutineCmd. Each complete original compiler function rejects parsePtr->numWords other than1, emits one coroutine-name instruction and returns TCL_OK: C8.6/C9.0 use TclEmitOpcode(INST_COROUTINE_NAME), while C9.1 uses OP(COROUTINE_NAME). The private worker count includes its own head and therefore has no extra worker operands; it is not the original public info argv count. Current InfoCoroutine grammar, compile_native_coroutine and CodegenCtx::native_coroutine_tasks retain the selected zero-operand Name recipe and release/arity/expansion refusals. Their software recipe supplies no live original command/compiler token, frame, coroutine holder, body admission or completion result.

## Scope

Read-only inspection retains the unchanged question request, all eight independently hashed source excerpts and the three complete original compiler source files. The five complete info source files are independently byte-verified against their already retained official source copies. Three additional complete LF function excerpts and a bounded inspection index make the emitted-instruction ranges explicit; the original short excerpts remain unchanged. All five C provider answers are source observations only. Jim and BIG-IP are not tested for this C compiler question. The marked current recipe control checks original operand coordinates, selected release, zero arguments and extra/expanded argument decline as a Rust API definition, not an executed result. Native250 public coroutine construction/resumption/deletion observations retain their separate question; this inspection issues no object/header/cache identity, dynamic coroutine name, reached handler or native compiler process.

## Provider answers

### tcl8.4

Status: `inspected`. Version: 8.4.20. Build: Read-only exact pinned original source; no interpreter/compiler/build invocation.. Channel: Complete original C info table and, where registered, complete original TclCompileInfoCoroutineCmd LF function source.. Dialect: Pinned original C release source.

The complete original info table has no coroutine member or compiler hook.

### tcl8.5

Status: `inspected`. Version: 8.5.19. Build: Read-only exact pinned original source; no interpreter/compiler/build invocation.. Channel: Complete original C info table and, where registered, complete original TclCompileInfoCoroutineCmd LF function source.. Dialect: Pinned original C release source.

The complete original info table has no coroutine member or compiler hook.

### tcl8.6

Status: `inspected`. Version: 8.6.18. Build: Read-only exact pinned original source; no interpreter/compiler/build invocation.. Channel: Complete original C info table and, where registered, complete original TclCompileInfoCoroutineCmd LF function source.. Dialect: Pinned original C release source.

Original info table registers TclCompileInfoCoroutineCmd; its complete function rejects numWords !=1, emits one COROUTINE_NAME instruction and returns TCL_OK. This is source declaration/worker interpretation only, not actual installed compiler/token admission or public coroutine output.

### tcl9.0

Status: `inspected`. Version: 9.0.4. Build: Read-only exact pinned original source; no interpreter/compiler/build invocation.. Channel: Complete original C info table and, where registered, complete original TclCompileInfoCoroutineCmd LF function source.. Dialect: Pinned original C release source.

Original info table registers TclCompileInfoCoroutineCmd; its complete function rejects numWords !=1, emits one COROUTINE_NAME instruction and returns TCL_OK. This is source declaration/worker interpretation only, not actual installed compiler/token admission or public coroutine output.

### tcl9.1

Status: `inspected`. Version: 9.1.0. Build: Read-only exact pinned original source; no interpreter/compiler/build invocation.. Channel: Complete original C info table and, where registered, complete original TclCompileInfoCoroutineCmd LF function source.. Dialect: Pinned original C release source.

Original info table registers TclCompileInfoCoroutineCmd; its complete function rejects numWords !=1, emits one COROUTINE_NAME instruction and returns TCL_OK. This is source declaration/worker interpretation only, not actual installed compiler/token admission or public coroutine output.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

Jim/BIG-IP are not inspected or executed for this original C compiler question. Current software recipe definitions supply no native command/token/frame/holder or completion observation.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

Jim/BIG-IP are not inspected or executed for this original C compiler question. Current software recipe definitions supply no native command/token/frame/holder or completion observation.

## Historical references

- `info-coroutine-source317-request.json` (source-anchor): [rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/request.json](../../../../rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/request.json). Unchanged original read-only question request with complete source/excerpt pins and original scope; no native launch.
- `info-coroutine-source317-8.4.20-tclCmdIL.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.4.20/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.4.20/tclCmdIL.c). Complete independently pinned original C info table source, reused byte-for-byte from its official retained source copy; no new compile or execution.
- `info-coroutine-source317-8.4.20-tclCmdIL.excerpt` (source-anchor): [rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/8.4.20/tclCmdIL.excerpt](../../../../rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/8.4.20/tclCmdIL.excerpt). Exact original independently hashed request excerpt; its bytes and declared LF range remain unchanged.
- `info-coroutine-source317-8.5.19-tclCmdIL.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.5.19/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.5.19/tclCmdIL.c). Complete independently pinned original C info table source, reused byte-for-byte from its official retained source copy; no new compile or execution.
- `info-coroutine-source317-8.5.19-tclCmdIL.excerpt` (source-anchor): [rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/8.5.19/tclCmdIL.excerpt](../../../../rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/8.5.19/tclCmdIL.excerpt). Exact original independently hashed request excerpt; its bytes and declared LF range remain unchanged.
- `info-coroutine-source317-8.6.18-tclCmdIL.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18/tclCmdIL.c). Complete independently pinned original C info table source, reused byte-for-byte from its official retained source copy; no new compile or execution.
- `info-coroutine-source317-8.6.18-tclCmdIL.excerpt` (source-anchor): [rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/8.6.18/tclCmdIL.excerpt](../../../../rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/8.6.18/tclCmdIL.excerpt). Exact original independently hashed request excerpt; its bytes and declared LF range remain unchanged.
- `info-coroutine-source317-8.6.18-tclCompCmdsGR.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/8.6.18/tclCompCmdsGR.c](../../../../rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/8.6.18/tclCompCmdsGR.c). Whole independently pinned original C compiler implementation; original worker shape/opcode source is independent of actual Native admission.
- `info-coroutine-source317-8.6.18-TclCompileInfoCoroutineCmd.complete.excerpt` (source-anchor): [rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/8.6.18/TclCompileInfoCoroutineCmd.complete.excerpt](../../../../rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/8.6.18/TclCompileInfoCoroutineCmd.complete.excerpt). Independently selected complete LF function excerpt, including original worker rejection and emitted coroutine-name instruction; short request excerpt remains unchanged.
- `info-coroutine-source317-8.6.18-tclCompCmdsGR.excerpt` (source-anchor): [rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/8.6.18/tclCompCmdsGR.excerpt](../../../../rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/8.6.18/tclCompCmdsGR.excerpt). Exact original independently hashed request excerpt; its bytes and declared LF range remain unchanged.
- `info-coroutine-source317-9.0.4-tclCmdIL.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4/tclCmdIL.c). Complete independently pinned original C info table source, reused byte-for-byte from its official retained source copy; no new compile or execution.
- `info-coroutine-source317-9.0.4-tclCmdIL.excerpt` (source-anchor): [rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/9.0.4/tclCmdIL.excerpt](../../../../rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/9.0.4/tclCmdIL.excerpt). Exact original independently hashed request excerpt; its bytes and declared LF range remain unchanged.
- `info-coroutine-source317-9.0.4-tclCompCmdsGR.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/9.0.4/tclCompCmdsGR.c](../../../../rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/9.0.4/tclCompCmdsGR.c). Whole independently pinned original C compiler implementation; original worker shape/opcode source is independent of actual Native admission.
- `info-coroutine-source317-9.0.4-TclCompileInfoCoroutineCmd.complete.excerpt` (source-anchor): [rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/9.0.4/TclCompileInfoCoroutineCmd.complete.excerpt](../../../../rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/9.0.4/TclCompileInfoCoroutineCmd.complete.excerpt). Independently selected complete LF function excerpt, including original worker rejection and emitted coroutine-name instruction; short request excerpt remains unchanged.
- `info-coroutine-source317-9.0.4-tclCompCmdsGR.excerpt` (source-anchor): [rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/9.0.4/tclCompCmdsGR.excerpt](../../../../rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/9.0.4/tclCompCmdsGR.excerpt). Exact original independently hashed request excerpt; its bytes and declared LF range remain unchanged.
- `info-coroutine-source317-9.1.0-tclCmdIL.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0/tclCmdIL.c). Complete independently pinned original C info table source, reused byte-for-byte from its official retained source copy; no new compile or execution.
- `info-coroutine-source317-9.1.0-tclCmdIL.excerpt` (source-anchor): [rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/9.1.0/tclCmdIL.excerpt](../../../../rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/9.1.0/tclCmdIL.excerpt). Exact original independently hashed request excerpt; its bytes and declared LF range remain unchanged.
- `info-coroutine-source317-9.1.0-tclCompCmdsGR.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/9.1.0/tclCompCmdsGR.c](../../../../rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/9.1.0/tclCompCmdsGR.c). Whole independently pinned original C compiler implementation; original worker shape/opcode source is independent of actual Native admission.
- `info-coroutine-source317-9.1.0-TclCompileInfoCoroutineCmd.complete.excerpt` (source-anchor): [rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/9.1.0/TclCompileInfoCoroutineCmd.complete.excerpt](../../../../rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/9.1.0/TclCompileInfoCoroutineCmd.complete.excerpt). Independently selected complete LF function excerpt, including original worker rejection and emitted coroutine-name instruction; short request excerpt remains unchanged.
- `info-coroutine-source317-9.1.0-tclCompCmdsGR.excerpt` (source-anchor): [rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/9.1.0/tclCompCmdsGR.excerpt](../../../../rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/9.1.0/tclCompCmdsGR.excerpt). Exact original independently hashed request excerpt; its bytes and declared LF range remain unchanged.
- `info-coroutine-source317-source-inspection.json` (source-anchor): [rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/source-inspection.json](../../../../rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/source-inspection.json). Bounded independently verified complete-worker LF source inspection index; no compile receipt or Native execution provenance.
- `naming-info-original-coroutine-name-compiler-rust-tcl-compiler-src-codegen-native_coroutine.rs` (implementation): [rust/tcl-compiler/src/codegen/native_coroutine.rs](../../../../rust/tcl-compiler/src/codegen/native_coroutine.rs). Current source/API owner and software assertion definition; no executed Rust or native provider result.
- `naming-info-original-coroutine-name-compiler-rust-tcl-registry-src-commands-tcl-info_.rs` (implementation): [rust/tcl-registry/src/commands/tcl/info_.rs](../../../../rust/tcl-registry/src/commands/tcl/info_.rs). Current source/API owner and software assertion definition; no executed Rust or native provider result.
- `naming-info-original-coroutine-name-compiler-rust-tcl-registry-src-native_compilation.rs` (implementation): [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs). Current source/API owner and software assertion definition; no executed Rust or native provider result.
- `naming-info-original-coroutine-name-compiler-rust-tcl-registry-src-native_coroutine_compilation.rs` (implementation): [rust/tcl-registry/src/native_coroutine_compilation.rs](../../../../rust/tcl-registry/src/native_coroutine_compilation.rs). Current source/API owner and software assertion definition; no executed Rust or native provider result.

## Source inspection

tcl8.4 8.4.20, revision `Independently pinned complete original C source content; no source-control revision or build invocation claimed.`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdIL.c`, function `subCmds`, lines 422–432. Retained reference `info-coroutine-source317-8.4.20-tclCmdIL.c`.

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

tcl8.5 8.5.19, revision `Independently pinned complete original C source content; no source-control revision or build invocation claimed.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdIL.c`, function `defaultInfoMap`, lines 155–179. Retained reference `info-coroutine-source317-8.5.19-tclCmdIL.c`.

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

tcl8.6 8.6.18, revision `Independently pinned complete original C source content; no source-control revision or build invocation claimed.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdIL.c`, function `defaultInfoMap`, lines 155–181. Retained reference `info-coroutine-source317-8.6.18-tclCmdIL.c`.

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

tcl8.6 8.6.18, revision `Independently pinned complete original C source content; no source-control revision or build invocation claimed.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCompCmdsGR.c`, function `TclCompileInfoCoroutineCmd`, lines 648–671. Retained reference `info-coroutine-source317-8.6.18-tclCompCmdsGR.c`.

```text
int
TclCompileInfoCoroutineCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    /*
     * Only compile [info coroutine] without arguments.
     */

    if (parsePtr->numWords != 1) {
	return TCL_ERROR;
    }

    /*
     * Not much to do; we compile to a single instruction...
     */

    TclEmitOpcode(		INST_COROUTINE_NAME,		envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `Independently pinned complete original C source content; no source-control revision or build invocation claimed.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdIL.c`, function `defaultInfoMap`, lines 151–180. Retained reference `info-coroutine-source317-9.0.4-tclCmdIL.c`.

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

tcl9.0 9.0.4, revision `Independently pinned complete original C source content; no source-control revision or build invocation claimed.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCompCmdsGR.c`, function `TclCompileInfoCoroutineCmd`, lines 640–662. Retained reference `info-coroutine-source317-9.0.4-tclCompCmdsGR.c`.

```text
int
TclCompileInfoCoroutineCmd(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    /*
     * Only compile [info coroutine] without arguments.
     */

    if (parsePtr->numWords != 1) {
	return TCL_ERROR;
    }

    /*
     * Not much to do; we compile to a single instruction...
     */

    TclEmitOpcode(		INST_COROUTINE_NAME,		envPtr);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `Independently pinned complete original C source content; no source-control revision or build invocation claimed.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdIL.c`, function `tclInfoImplMap`, lines 152–181. Retained reference `info-coroutine-source317-9.1.0-tclCmdIL.c`.

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

tcl9.1 9.1.0, revision `Independently pinned complete original C source content; no source-control revision or build invocation claimed.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCompCmdsGR.c`, function `TclCompileInfoCoroutineCmd`, lines 592–614. Retained reference `info-coroutine-source317-9.1.0-tclCompCmdsGR.c`.

```text
int
TclCompileInfoCoroutineCmd(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    /*
     * Only compile [info coroutine] without arguments.
     */

    if (parsePtr->numWords != 1) {
	return TCL_ERROR;
    }

    /*
     * Not much to do; we compile to a single instruction...
     */

    OP(				COROUTINE_NAME);
    return TCL_OK;
}

```


## Consumer bindings

- [rust/tcl-registry/src/commands/tcl/info_.rs](../../../../rust/tcl-registry/src/commands/tcl/info_.rs), `spec`: Publish the authored info SubCommand table through CommandSpec.subcommands, independently of actual selected original C ensemble/compiler registration.
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompilationSpec::select_registered_worker_native_words`: Use original effective worker coordinates and the selected zero-argument InfoCoroutine grammar; independent Native token/compiler entry remains required.
- [rust/tcl-registry/src/native_coroutine_compilation.rs](../../../../rust/tcl-registry/src/native_coroutine_compilation.rs), `compile_native_coroutine`: Retain selected C8.6/C9 zero-worker-operand Name recipe, exact release/arity and expanded-argument decline; no original live holder grant.
- [rust/tcl-compiler/src/codegen/native_coroutine.rs](../../../../rust/tcl-compiler/src/codegen/native_coroutine.rs), `CodegenCtx::native_coroutine_tasks`: Project the selected Name recipe to the coroutine-name instruction plan without argument work; original compilation/body/execution admission remains independent.
- [rust/tcl-registry/src/native_coroutine_compilation.rs](../../../../rust/tcl-registry/src/native_coroutine_compilation.rs), `native_coroutine_compilation::coroutine_name_tests::original_coroutine_name_compiler_keeps_zero_operand_recipe_and_native_decline` (linked): Actual selected C release and original public/direct worker operand coordinates retain the zero-operand Name recipe only from C8.6. Extra or expanded arguments decline; C8.4/8.5 remain unavailable. This marked source/API definition executes no external compiler/provider and grants no Native token/frame/holder/result.

A named test is a coverage binding, not a claim that it executed.

## Replay

Read-only whole-source/excerpt/hash inspection and one marked current software recipe definition. No native/compiler/Rust command is run, no compile provenance is manufactured, and public coroutine observations retain their independent original receipts.
