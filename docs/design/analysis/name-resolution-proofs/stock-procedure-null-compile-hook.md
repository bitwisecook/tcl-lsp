# naming.compiler.stock-procedure-null-compile-hook

Kind: `source-anchor`

## Problem statement

An unknown caller compilation mode can mean either direct argv evaluation or bytecode compilation. If the selected stock proc registration has a NULL compileProc, treating that uncertainty as an arbitrary compiler can withdraw command state before proc arguments run. The absence of a compiler must be established from the selected registration; a renamed label, caller words, or handler effects do not establish it.

## Question

Do the pinned C8.4 through C9.1 stock proc registrations have a compiler hook, and does their compiler callsite invoke a NULL hook?

## Conclusion

All five inspected stock proc registrations have a NULL compileProc, while return has a non-null compiler. The retained compiler callsites check for a non-null hook before invocation. This supports generic argv transport for an independently selected NoHook descriptor under unknown compilation mode; it supplies no handler success, procedure body entry, replacement-registration or native object authority.

## Scope

Pinned C sources for Tcl8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0; exact full-file and excerpt hashes. Source inspection only, no newly executed native control. Jim and BIG-IP are not inspected for this C registration question.

## Provider answers

### tcl8.4

Status: `inspected`. Version: 8.4.20. Build: No guest build or execution for this source inspection; exact release source hashes are retained.. Channel: Pinned C source registration and compiler callsite; no guest source input.. Dialect: Tcl.

Stock proc has NULL compileProc; stock return has a compiler; the compiler invokes only a non-null hook.

### tcl8.5

Status: `inspected`. Version: 8.5.19. Build: No guest build or execution for this source inspection; exact release source hashes are retained.. Channel: Pinned C source registration and compiler callsite; no guest source input.. Dialect: Tcl.

Stock proc has NULL compileProc; stock return has a compiler; the compiler invokes only a non-null hook.

### tcl8.6

Status: `inspected`. Version: 8.6.18. Build: No guest build or execution for this source inspection; exact release source hashes are retained.. Channel: Pinned C source registration and compiler callsite; no guest source input.. Dialect: Tcl.

Stock proc has NULL compileProc; stock return has a compiler; the compiler invokes only a non-null hook.

### tcl9.0

Status: `inspected`. Version: 9.0.4. Build: No guest build or execution for this source inspection; exact release source hashes are retained.. Channel: Pinned C source registration and compiler callsite; no guest source input.. Dialect: Tcl.

Stock proc has NULL compileProc; stock return has a compiler; the compiler invokes only a non-null hook.

### tcl9.1

Status: `inspected`. Version: 9.1.0. Build: No guest build or execution for this source inspection; exact release source hashes are retained.. Channel: Pinned C source registration and compiler callsite; no guest source input.. Dialect: Tcl.

Stock proc has NULL compileProc; stock return has a compiler; the compiler invokes only a non-null hook.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Historical references

- `e0` (source-anchor): [rust/tcl-registry/tests/data/native_no_hook_source/8.4.20-registrations.txt](../../../../rust/tcl-registry/tests/data/native_no_hook_source/8.4.20-registrations.txt). Exact builtInCmds source excerpt; original full-file SHA and lines are retained separately.
- `e1` (source-anchor): [rust/tcl-registry/tests/data/native_no_hook_source/8.4.20-compiler-call.txt](../../../../rust/tcl-registry/tests/data/native_no_hook_source/8.4.20-compiler-call.txt). Exact TclCompileScript source excerpt; original full-file SHA and lines are retained separately.
- `e2` (source-anchor): [rust/tcl-registry/tests/data/native_no_hook_source/8.5.19-registrations.txt](../../../../rust/tcl-registry/tests/data/native_no_hook_source/8.5.19-registrations.txt). Exact builtInCmds source excerpt; original full-file SHA and lines are retained separately.
- `e3` (source-anchor): [rust/tcl-registry/tests/data/native_no_hook_source/8.5.19-compiler-call.txt](../../../../rust/tcl-registry/tests/data/native_no_hook_source/8.5.19-compiler-call.txt). Exact TclCompileScript source excerpt; original full-file SHA and lines are retained separately.
- `e4` (source-anchor): [rust/tcl-registry/tests/data/native_no_hook_source/8.6.18-registrations.txt](../../../../rust/tcl-registry/tests/data/native_no_hook_source/8.6.18-registrations.txt). Exact builtInCmds source excerpt; original full-file SHA and lines are retained separately.
- `e5` (source-anchor): [rust/tcl-registry/tests/data/native_no_hook_source/8.6.18-compiler-call.txt](../../../../rust/tcl-registry/tests/data/native_no_hook_source/8.6.18-compiler-call.txt). Exact CompileCommandTokens source excerpt; original full-file SHA and lines are retained separately.
- `e6` (source-anchor): [rust/tcl-registry/tests/data/native_no_hook_source/9.0.4-registrations.txt](../../../../rust/tcl-registry/tests/data/native_no_hook_source/9.0.4-registrations.txt). Exact builtInCmds source excerpt; original full-file SHA and lines are retained separately.
- `e7` (source-anchor): [rust/tcl-registry/tests/data/native_no_hook_source/9.0.4-compiler-call.txt](../../../../rust/tcl-registry/tests/data/native_no_hook_source/9.0.4-compiler-call.txt). Exact CompileCommandTokens source excerpt; original full-file SHA and lines are retained separately.
- `e8` (source-anchor): [rust/tcl-registry/tests/data/native_no_hook_source/9.1.0-registrations.txt](../../../../rust/tcl-registry/tests/data/native_no_hook_source/9.1.0-registrations.txt). Exact builtInCmds source excerpt; original full-file SHA and lines are retained separately.
- `e9` (source-anchor): [rust/tcl-registry/tests/data/native_no_hook_source/9.1.0-compiler-call.txt](../../../../rust/tcl-registry/tests/data/native_no_hook_source/9.1.0-compiler-call.txt). Exact CompileCommandTokens source excerpt; original full-file SHA and lines are retained separately.

## Source inspection

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclBasic.c`, function `builtInCmds`, lines 168–177. Retained reference `e0`.

```text
    {"proc",		(Tcl_CmdProc *) NULL,	Tcl_ProcObjCmd,	
        (CompileProc *) NULL,		1},
    {"regexp",		(Tcl_CmdProc *) NULL,	Tcl_RegexpObjCmd,
        TclCompileRegexpCmd,		1},
    {"regsub",		(Tcl_CmdProc *) NULL,	Tcl_RegsubObjCmd,
        (CompileProc *) NULL,		1},
    {"rename",		(Tcl_CmdProc *) NULL,	Tcl_RenameObjCmd,
        (CompileProc *) NULL,		1},
    {"return",		(Tcl_CmdProc *) NULL,	Tcl_ReturnObjCmd,	
        TclCompileReturnCmd,		1},

```

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclCompile.c`, function `TclCompileScript`, lines 1277–1292. Retained reference `e1`.

```text
				Tcl_DStringValue(&ds),
			        (Tcl_Namespace *) cmdNsPtr, /*flags*/ 0);

			if ((cmdPtr != NULL)
			        && (cmdPtr->compileProc != NULL)
			        && !(cmdPtr->flags & CMD_HAS_EXEC_TRACES)
			        && !(iPtr->flags & DONT_COMPILE_CMDS_INLINE)) {
			    int savedNumCmds = envPtr->numCommands;
			    unsigned int savedCodeNext =
				    envPtr->codeNext - envPtr->codeStart;

			    code = (*(cmdPtr->compileProc))(interp, &parse,
			            envPtr);
			    if (code == TCL_OK) {
				goto finishCommand;
			    } else if (code == TCL_OUT_LINE_COMPILE) {

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclBasic.c`, function `builtInCmds`, lines 159–163. Retained reference `e2`.

```text
    {"proc",		Tcl_ProcObjCmd,		NULL,			1},
    {"regexp",		Tcl_RegexpObjCmd,	TclCompileRegexpCmd,	1},
    {"regsub",		Tcl_RegsubObjCmd,	NULL,			1},
    {"rename",		Tcl_RenameObjCmd,	NULL,			1},
    {"return",		Tcl_ReturnObjCmd,	TclCompileReturnCmd,	1},

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompile.c`, function `TclCompileScript`, lines 1402–1415. Retained reference `e3`.

```text
		    cmdPtr = (Command *) Tcl_FindCommand(interp,
			    Tcl_DStringValue(&ds),
			    (Tcl_Namespace *) cmdNsPtr, /*flags*/ 0);

		    if ((cmdPtr != NULL)
			    && (cmdPtr->compileProc != NULL)
			    && !(cmdPtr->flags & CMD_HAS_EXEC_TRACES)
			    && !(iPtr->flags & DONT_COMPILE_CMDS_INLINE)) {
			int savedNumCmds = envPtr->numCommands;
			unsigned savedCodeNext =
				envPtr->codeNext - envPtr->codeStart;
			int update = 0, code;

			/*

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclBasic.c`, function `builtInCmds`, lines 274–278. Retained reference `e4`.

```text
    {"proc",		Tcl_ProcObjCmd,		NULL,			NULL,	CMD_IS_SAFE},
    {"regexp",		Tcl_RegexpObjCmd,	TclCompileRegexpCmd,	NULL,	CMD_IS_SAFE},
    {"regsub",		Tcl_RegsubObjCmd,	TclCompileRegsubCmd,	NULL,	CMD_IS_SAFE},
    {"rename",		Tcl_RenameObjCmd,	NULL,			NULL,	CMD_IS_SAFE},
    {"return",		Tcl_ReturnObjCmd,	TclCompileReturnCmd,	NULL,	CMD_IS_SAFE},

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompile.c`, function `CompileCommandTokens`, lines 2030–2060. Retained reference `e5`.

```text
    tokenPtr = parsePtr->tokenPtr;
    cmdKnown = TclWordKnownAtCompileTime(tokenPtr, cmdObj);

    /* Is this a command we should (try to) compile with a compileProc ? */
    if (cmdKnown && !(iPtr->flags & DONT_COMPILE_CMDS_INLINE)) {
	cmdPtr = (Command *) Tcl_GetCommandFromObj(interp, cmdObj);
	if (cmdPtr) {
	    /*
	     * Found a command.  Test the ways we can be told not to attempt
	     * to compile it.
	     */
	    if ((cmdPtr->compileProc == NULL)
		    || (cmdPtr->nsPtr->flags & NS_SUPPRESS_COMPILATION)
		    || (cmdPtr->flags & CMD_HAS_EXEC_TRACES)) {
		cmdPtr = NULL;
	    }
	}
	if (cmdPtr && !(cmdPtr->flags & CMD_COMPILES_EXPANDED)) {
	    expand = ExpandRequested(parsePtr->tokenPtr, parsePtr->numWords);
	    if (expand) {
		/* We need to expand, but compileProc cannot. */
		cmdPtr = NULL;
	    }
	}
    }

    /* If cmdPtr != NULL, try to call cmdPtr->compileProc */
    if (cmdPtr) {
	code = CompileCmdCompileProc(interp, parsePtr, cmdPtr, envPtr);
    }


```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclBasic.c`, function `builtInCmds`, lines 386–390. Retained reference `e6`.

```text
    {"proc",		ProcObjCmd,		NULL,			NULL,	CMD_IS_SAFE},
    {"regexp",		Tcl_RegexpObjCmd,	TclCompileRegexpCmd,	NULL,	CMD_IS_SAFE},
    {"regsub",		Tcl_RegsubObjCmd,	TclCompileRegsubCmd,	NULL,	CMD_IS_SAFE},
    {"rename",		Tcl_RenameObjCmd,	NULL,			NULL,	CMD_IS_SAFE},
    {"return",		Tcl_ReturnObjCmd,	TclCompileReturnCmd,	NULL,	CMD_IS_SAFE},

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompile.c`, function `CompileCommandTokens`, lines 2073–2103. Retained reference `e7`.

```text
    tokenPtr = parsePtr->tokenPtr;
    cmdKnown = TclWordKnownAtCompileTime(tokenPtr, cmdObj);

    /* Is this a command we should (try to) compile with a compileProc ? */
    if (cmdKnown && !(iPtr->flags & DONT_COMPILE_CMDS_INLINE)) {
	cmdPtr = (Command *) Tcl_GetCommandFromObj(interp, cmdObj);
	if (cmdPtr) {
	    /*
	     * Found a command.  Test the ways we can be told not to attempt
	     * to compile it.
	     */
	    if ((cmdPtr->compileProc == NULL)
		    || (cmdPtr->nsPtr->flags & NS_SUPPRESS_COMPILATION)
		    || (cmdPtr->flags & CMD_HAS_EXEC_TRACES)) {
		cmdPtr = NULL;
	    }
	}
	if (cmdPtr && !(cmdPtr->flags & CMD_COMPILES_EXPANDED)) {
	    expand = ExpandRequested(parsePtr->tokenPtr, (int)parsePtr->numWords);
	    if (expand) {
		/* We need to expand, but compileProc cannot. */
		cmdPtr = NULL;
	    }
	}
    }

    /* If cmdPtr != NULL, try to call cmdPtr->compileProc */
    if (cmdPtr) {
	code = CompileCmdCompileProc(interp, parsePtr, cmdPtr, envPtr);
    }


```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclBasic.c`, function `builtInCmds`, lines 355–360. Retained reference `e8`.

```text
    {"proc",		Tcl_ProcObjCmd,		NULL,			NULL,	CMD_IS_SAFE},
    {"regexp",		Tcl_RegexpObjCmd,	TclCompileRegexpCmd,	NULL,	CMD_IS_SAFE},
    {"regsub",		Tcl_RegsubObjCmd,	TclCompileRegsubCmd,	NULL,	CMD_IS_SAFE},
    {"remquo",		RemQuoObjCmd,		NULL,			NULL,	CMD_IS_SAFE},
    {"rename",		Tcl_RenameObjCmd,	NULL,			NULL,	CMD_IS_SAFE},
    {"return",		Tcl_ReturnObjCmd,	TclCompileReturnCmd,	NULL,	CMD_IS_SAFE},

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompile.c`, function `CompileCommandTokens`, lines 2391–2421. Retained reference `e9`.

```text
    tokenPtr = parsePtr->tokenPtr;
    cmdKnown = TclWordKnownAtCompileTime(tokenPtr, cmdObj);

    /* Is this a command we should (try to) compile with a compileProc ? */
    if (cmdKnown && !(iPtr->flags & DONT_COMPILE_CMDS_INLINE)) {
	cmdPtr = (Command *) Tcl_GetCommandFromObj(interp, cmdObj);
	if (cmdPtr) {
	    /*
	     * Found a command.  Test the ways we can be told not to attempt
	     * to compile it.
	     */
	    if ((cmdPtr->compileProc == NULL)
		    || (cmdPtr->nsPtr->flags & NS_SUPPRESS_COMPILATION)
		    || (cmdPtr->flags & CMD_HAS_EXEC_TRACES)) {
		cmdPtr = NULL;
	    }
	}
	if (cmdPtr && !(cmdPtr->flags & CMD_COMPILES_EXPANDED)) {
	    expand = ExpandRequested(parsePtr->tokenPtr, numWords);
	    if (expand) {
		/* We need to expand, but compileProc cannot. */
		cmdPtr = NULL;
	    }
	}
    }

    /* If cmdPtr != NULL, try to call cmdPtr->compileProc */
    if (cmdPtr) {
	code = CompileCmdCompileProc(interp, parsePtr, cmdPtr, envPtr);
    }


```


## Consumer bindings

- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompilationSpec::select_native_words`: Project selected hook absence independently of caller compilation mode; preserve original vector and dialect guards.
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `native_compilation::tests::selected_no_hook_registration_keeps_generic_argv_under_unknown_mode` (linked): The current model separates selected stock proc NoHook from stock return hook and retains full original native argv; this assertion is not native execution.

A named test is a coverage binding, not a claim that it executed.

## Replay

Inspect the exact released source files and hashes against the retained excerpts. Rust contract assertions use the selected source configuration and are not claimed executed here.
