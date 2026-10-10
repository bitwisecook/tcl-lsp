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

## Exact evidence

- `e0` (source-anchor): [rust/tcl-registry/tests/data/native_no_hook_source/8.4.20-registrations.txt](../../../../rust/tcl-registry/tests/data/native_no_hook_source/8.4.20-registrations.txt). SHA-256 `651d31494953853c211fe2e9ec9333018710c0d3dc4079820f82a189947a35dd`. Exact builtInCmds source excerpt; original full-file SHA and lines are retained separately.
- `e1` (source-anchor): [rust/tcl-registry/tests/data/native_no_hook_source/8.4.20-compiler-call.txt](../../../../rust/tcl-registry/tests/data/native_no_hook_source/8.4.20-compiler-call.txt). SHA-256 `54023432b224c8f21db9cc676d3d9c01a9ede1b5856e49c92bf24a7cd5869178`. Exact TclCompileScript source excerpt; original full-file SHA and lines are retained separately.
- `e2` (source-anchor): [rust/tcl-registry/tests/data/native_no_hook_source/8.5.19-registrations.txt](../../../../rust/tcl-registry/tests/data/native_no_hook_source/8.5.19-registrations.txt). SHA-256 `5d566598fa4e60e7a5740882bbce809813869994a21e06a317257459a1ce1c60`. Exact builtInCmds source excerpt; original full-file SHA and lines are retained separately.
- `e3` (source-anchor): [rust/tcl-registry/tests/data/native_no_hook_source/8.5.19-compiler-call.txt](../../../../rust/tcl-registry/tests/data/native_no_hook_source/8.5.19-compiler-call.txt). SHA-256 `971d284c64dbb2a417a5e44d6100ac57b58820fe915e61885238e9e1b916c73b`. Exact TclCompileScript source excerpt; original full-file SHA and lines are retained separately.
- `e4` (source-anchor): [rust/tcl-registry/tests/data/native_no_hook_source/8.6.18-registrations.txt](../../../../rust/tcl-registry/tests/data/native_no_hook_source/8.6.18-registrations.txt). SHA-256 `1c95fcd7477dc5193bc8cc9189cbb25647b912c67bc5ce6c3b1f4fc883b2de62`. Exact builtInCmds source excerpt; original full-file SHA and lines are retained separately.
- `e5` (source-anchor): [rust/tcl-registry/tests/data/native_no_hook_source/8.6.18-compiler-call.txt](../../../../rust/tcl-registry/tests/data/native_no_hook_source/8.6.18-compiler-call.txt). SHA-256 `11d13a1e2b8c109bea5b3d87750adc9579516628b1ae79cae9b49f6ad741e67f`. Exact CompileCommandTokens source excerpt; original full-file SHA and lines are retained separately.
- `e6` (source-anchor): [rust/tcl-registry/tests/data/native_no_hook_source/9.0.4-registrations.txt](../../../../rust/tcl-registry/tests/data/native_no_hook_source/9.0.4-registrations.txt). SHA-256 `280338db7b87258e9475c5e6b37d8537479044d0ced867c0c9ba9510b0eb4fa5`. Exact builtInCmds source excerpt; original full-file SHA and lines are retained separately.
- `e7` (source-anchor): [rust/tcl-registry/tests/data/native_no_hook_source/9.0.4-compiler-call.txt](../../../../rust/tcl-registry/tests/data/native_no_hook_source/9.0.4-compiler-call.txt). SHA-256 `1803863626befe725c472ff52c33e0a3d9919044d3920e17af30aa5e096dabef`. Exact CompileCommandTokens source excerpt; original full-file SHA and lines are retained separately.
- `e8` (source-anchor): [rust/tcl-registry/tests/data/native_no_hook_source/9.1.0-registrations.txt](../../../../rust/tcl-registry/tests/data/native_no_hook_source/9.1.0-registrations.txt). SHA-256 `932ce572de6b1c38032a93a1f673baee4339cbd25e2c6588c0e7c9a512fed4e8`. Exact builtInCmds source excerpt; original full-file SHA and lines are retained separately.
- `e9` (source-anchor): [rust/tcl-registry/tests/data/native_no_hook_source/9.1.0-compiler-call.txt](../../../../rust/tcl-registry/tests/data/native_no_hook_source/9.1.0-compiler-call.txt). SHA-256 `c1d6c7e554b93ce3728a1e0b7799be86c0be49d0514047c9d729937cf7cef099`. Exact CompileCommandTokens source excerpt; original full-file SHA and lines are retained separately.

## Source inspection

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclBasic.c`, function `builtInCmds`, lines 168–177. Full-source SHA-256 `cab6d4decd1a3e365fe3cef49a2d5430dfef01cd775dd7d34f98064032f3a81a`; snippet SHA-256 `651d31494953853c211fe2e9ec9333018710c0d3dc4079820f82a189947a35dd`; retained evidence `e0`.

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

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclCompile.c`, function `TclCompileScript`, lines 1277–1292. Full-source SHA-256 `0bc55b283d6cb62a4298d4dc30e050b587d5d1b4e7f145e4f644e237dd7639f8`; snippet SHA-256 `54023432b224c8f21db9cc676d3d9c01a9ede1b5856e49c92bf24a7cd5869178`; retained evidence `e1`.

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

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclBasic.c`, function `builtInCmds`, lines 159–163. Full-source SHA-256 `d09c16386ea0376dc3590a74ded2c6e2bcef5079f2b1a3785545bad37c7ead63`; snippet SHA-256 `5d566598fa4e60e7a5740882bbce809813869994a21e06a317257459a1ce1c60`; retained evidence `e2`.

```text
    {"proc",		Tcl_ProcObjCmd,		NULL,			1},
    {"regexp",		Tcl_RegexpObjCmd,	TclCompileRegexpCmd,	1},
    {"regsub",		Tcl_RegsubObjCmd,	NULL,			1},
    {"rename",		Tcl_RenameObjCmd,	NULL,			1},
    {"return",		Tcl_ReturnObjCmd,	TclCompileReturnCmd,	1},

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompile.c`, function `TclCompileScript`, lines 1402–1415. Full-source SHA-256 `8c2ec76dbbe697201bcad79f9e32c0c1db23faf7ca01eb19e08f3ea9e68dd988`; snippet SHA-256 `971d284c64dbb2a417a5e44d6100ac57b58820fe915e61885238e9e1b916c73b`; retained evidence `e3`.

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

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclBasic.c`, function `builtInCmds`, lines 274–278. Full-source SHA-256 `19e28c2c9fbcd27e7e538a06960f0d806d04f8c25d94499cf9eb9031f8440198`; snippet SHA-256 `1c95fcd7477dc5193bc8cc9189cbb25647b912c67bc5ce6c3b1f4fc883b2de62`; retained evidence `e4`.

```text
    {"proc",		Tcl_ProcObjCmd,		NULL,			NULL,	CMD_IS_SAFE},
    {"regexp",		Tcl_RegexpObjCmd,	TclCompileRegexpCmd,	NULL,	CMD_IS_SAFE},
    {"regsub",		Tcl_RegsubObjCmd,	TclCompileRegsubCmd,	NULL,	CMD_IS_SAFE},
    {"rename",		Tcl_RenameObjCmd,	NULL,			NULL,	CMD_IS_SAFE},
    {"return",		Tcl_ReturnObjCmd,	TclCompileReturnCmd,	NULL,	CMD_IS_SAFE},

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompile.c`, function `CompileCommandTokens`, lines 2030–2060. Full-source SHA-256 `d1a494975cb56b8850804669e17fb7c5aacc1d70de505ea661f59ca365ab45c8`; snippet SHA-256 `11d13a1e2b8c109bea5b3d87750adc9579516628b1ae79cae9b49f6ad741e67f`; retained evidence `e5`.

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

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclBasic.c`, function `builtInCmds`, lines 386–390. Full-source SHA-256 `6d995c1af94fea92717183882ca41775fe8c0e088def163f6ac8fe699cbffc67`; snippet SHA-256 `280338db7b87258e9475c5e6b37d8537479044d0ced867c0c9ba9510b0eb4fa5`; retained evidence `e6`.

```text
    {"proc",		ProcObjCmd,		NULL,			NULL,	CMD_IS_SAFE},
    {"regexp",		Tcl_RegexpObjCmd,	TclCompileRegexpCmd,	NULL,	CMD_IS_SAFE},
    {"regsub",		Tcl_RegsubObjCmd,	TclCompileRegsubCmd,	NULL,	CMD_IS_SAFE},
    {"rename",		Tcl_RenameObjCmd,	NULL,			NULL,	CMD_IS_SAFE},
    {"return",		Tcl_ReturnObjCmd,	TclCompileReturnCmd,	NULL,	CMD_IS_SAFE},

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompile.c`, function `CompileCommandTokens`, lines 2073–2103. Full-source SHA-256 `51b709e7bf2c1f25bb7b3ab371c7dcbd6cf6d2bce509931397303cb770ee21f3`; snippet SHA-256 `1803863626befe725c472ff52c33e0a3d9919044d3920e17af30aa5e096dabef`; retained evidence `e7`.

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

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclBasic.c`, function `builtInCmds`, lines 355–360. Full-source SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`; snippet SHA-256 `932ce572de6b1c38032a93a1f673baee4339cbd25e2c6588c0e7c9a512fed4e8`; retained evidence `e8`.

```text
    {"proc",		Tcl_ProcObjCmd,		NULL,			NULL,	CMD_IS_SAFE},
    {"regexp",		Tcl_RegexpObjCmd,	TclCompileRegexpCmd,	NULL,	CMD_IS_SAFE},
    {"regsub",		Tcl_RegsubObjCmd,	TclCompileRegsubCmd,	NULL,	CMD_IS_SAFE},
    {"remquo",		RemQuoObjCmd,		NULL,			NULL,	CMD_IS_SAFE},
    {"rename",		Tcl_RenameObjCmd,	NULL,			NULL,	CMD_IS_SAFE},
    {"return",		Tcl_ReturnObjCmd,	TclCompileReturnCmd,	NULL,	CMD_IS_SAFE},

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompile.c`, function `CompileCommandTokens`, lines 2391–2421. Full-source SHA-256 `941eaa2c57e4d7d5266134c5b713dca5218d29eea951c89a0676c14b8260eb4f`; snippet SHA-256 `c1d6c7e554b93ce3728a1e0b7799be86c0be49d0514047c9d729937cf7cef099`; retained evidence `e9`.

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
