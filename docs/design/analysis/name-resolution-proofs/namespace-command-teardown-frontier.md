# naming.namespace.command-teardown-frontier

Kind: `source-anchor`

## Problem statement

Command-delete callbacks can create or replace command entries while a namespace is dismantled. The selected release source determines whether the command frontier restarts after each deletion or retains a complete table pass.

## Question

Which pinned C release implementations restart the command-table first entry after each deletion, and which retain a complete frontier per deletion pass?

## Conclusion

Pinned C8.4.20/C8.5.19 TclTeardownNamespace calls Tcl_FirstHashEntry for each command deletion until the table is empty. Pinned C8.6.18/C9.0.4/C9.1.0 retain command pointers for a complete pass and repeat until empty. The pure NativeNamespaceCommandTeardown recipe selects RepeatedFirstEntry or SnapshotPass accordingly; frontier_len selects one available entry for the first-entry policy and the whole available frontier for a pass. NativeNameProtocol declines the C table recipe for Jim. Source inspection and pure recipe selection supply no actual namespace holder, command token, deletion callback entry, runtime ordering or Rust assertion result. Independent original public callback-created/replaced teardown completions belong to naming.namespace.original-command-holder-routing.

## Scope

Five exact full tclNamesp.c source copies and byte-identical original line excerpts are inspected. This question records source text, not newly executed provider processes. Runtime/VM consumers retain their separate actual state/holder requirements. Jim and BIG-IP are not inspected or executed for this C source question.

## Provider answers

### tcl8.4, tcl8.5

Status: `inspected`. Channel: Exact original source inspection, not guest Tcl evaluation. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

TclTeardownNamespace restarts the first table entry after each deletion. Source text supplies no actual holder/token/callback or interpreter process result.

### tcl8.6, tcl9.0, tcl9.1

Status: `inspected`. Channel: Exact original source inspection, not guest Tcl evaluation. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

TclTeardownNamespace retains a complete command-pointer pass and repeats until empty. Source text supplies no actual holder/token/callback or interpreter process result.

### jim, bigip

Status: `not-tested`.

No exact source inspection or interpreter execution is recorded for this C teardown question.

## Source inspection

tcl8.4 8.4.20, revision `Source-control revision unrecorded; exact release source and excerpt SHA retained.`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclNamesp.c`, function `TclTeardownNamespace`, lines 746–759. Retained reference `namespace-teardown227-tcl-8.4.20-command-teardown.txt`.

```text
    /*
     * Delete all commands in this namespace. Be careful when traversing the
     * hash table: when each command is deleted, it removes itself from the
     * command table.
     */

    for (entryPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search);
            entryPtr != NULL;
            entryPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search)) {
        cmd = (Tcl_Command) Tcl_GetHashValue(entryPtr);
        Tcl_DeleteCommandFromToken((Tcl_Interp *) iPtr, cmd);
    }
    Tcl_DeleteHashTable(&nsPtr->cmdTable);
    Tcl_InitHashTable(&nsPtr->cmdTable, TCL_STRING_KEYS);
```

tcl8.5 8.5.19, revision `Source-control revision unrecorded; exact release source and excerpt SHA retained.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclNamesp.c`, function `TclTeardownNamespace`, lines 1107–1122. Retained reference `namespace-teardown227-tcl-8.5.19-command-teardown.txt`.

```text
    /*
     * Delete all commands in this namespace. Be careful when traversing the
     * hash table: when each command is deleted, it removes itself from the
     * command table.
     *
     * Don't optimize to Tcl_NextHashEntry() because of traces.
     */

    for (entryPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search);
	    entryPtr != NULL;
	    entryPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search)) {
	cmd = Tcl_GetHashValue(entryPtr);
	Tcl_DeleteCommandFromToken((Tcl_Interp *) iPtr, cmd);
    }
    Tcl_DeleteHashTable(&nsPtr->cmdTable);
    Tcl_InitHashTable(&nsPtr->cmdTable, TCL_STRING_KEYS);
```

tcl8.6 8.6.18, revision `Source-control revision unrecorded; exact release source and excerpt SHA retained.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclNamesp.c`, function `TclTeardownNamespace`, lines 1090–1119. Retained reference `namespace-teardown227-tcl-8.6.18-command-teardown.txt`.

```text
    /*
     * Delete all commands in this namespace. Be careful when traversing the
     * hash table: when each command is deleted, it removes itself from the
     * command table. Because of traces (and the desire to avoid the quadratic
     * problems of just using Tcl_FirstHashEntry over and over, [Bug
     * f97d4ee020]) we copy to a temporary array and then delete all those
     * commands.
     */

    while (nsPtr->cmdTable.numEntries > 0) {
	int length = nsPtr->cmdTable.numEntries;
	Command **cmds = (Command **)TclStackAlloc(interp,
		sizeof(Command *) * length);

	i = 0;
	for (entryPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search);
		entryPtr != NULL;
		entryPtr = Tcl_NextHashEntry(&search)) {
	    cmds[i] = (Command *)Tcl_GetHashValue(entryPtr);
	    cmds[i]->refCount++;
	    i++;
	}
	for (i = 0 ; i < length ; i++) {
	    Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmds[i]);
	    TclCleanupCommandMacro(cmds[i]);
	}
	TclStackFree(interp, cmds);
    }
    Tcl_DeleteHashTable(&nsPtr->cmdTable);
    Tcl_InitHashTable(&nsPtr->cmdTable, TCL_STRING_KEYS);
```

tcl9.0 9.0.4, revision `Source-control revision unrecorded; exact release source and excerpt SHA retained.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclNamesp.c`, function `TclTeardownNamespace`, lines 1274–1303. Retained reference `namespace-teardown227-tcl-9.0.4-command-teardown.txt`.

```text
    /*
     * Delete all commands in this namespace. Be careful when traversing the
     * hash table: when each command is deleted, it removes itself from the
     * command table. Because of traces (and the desire to avoid the quadratic
     * problems of just using Tcl_FirstHashEntry over and over, [Bug
     * f97d4ee020]) we copy to a temporary array and then delete all those
     * commands.
     */

    while (nsPtr->cmdTable.numEntries > 0) {
	Tcl_Size length = nsPtr->cmdTable.numEntries;
	Command **cmds = (Command **)TclStackAlloc(interp,
		sizeof(Command *) * length);

	i = 0;
	for (entryPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search);
		entryPtr != NULL;
		entryPtr = Tcl_NextHashEntry(&search)) {
	    cmds[i] = (Command *) Tcl_GetHashValue(entryPtr);
	    cmds[i]->refCount++;
	    i++;
	}
	for (i = 0 ; i < length ; i++) {
	    Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmds[i]);
	    TclCleanupCommandMacro(cmds[i]);
	}
	TclStackFree(interp, cmds);
    }
    Tcl_DeleteHashTable(&nsPtr->cmdTable);
    Tcl_InitHashTable(&nsPtr->cmdTable, TCL_STRING_KEYS);
```

tcl9.1 9.1.0, revision `Source-control revision unrecorded; exact release source and excerpt SHA retained.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclNamesp.c`, function `TclTeardownNamespace`, lines 1273–1302. Retained reference `namespace-teardown227-tcl-9.1.0-command-teardown.txt`.

```text
    /*
     * Delete all commands in this namespace. Be careful when traversing the
     * hash table: when each command is deleted, it removes itself from the
     * command table. Because of traces (and the desire to avoid the quadratic
     * problems of just using Tcl_FirstHashEntry over and over, [Bug
     * f97d4ee020]) we copy to a temporary array and then delete all those
     * commands.
     */

    while (nsPtr->cmdTable.numEntries > 0) {
	Tcl_Size length = nsPtr->cmdTable.numEntries;
	Command **cmds = (Command **)TclStackAlloc(interp,
		sizeof(Command *) * length);

	i = 0;
	for (entryPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search);
		entryPtr != NULL;
		entryPtr = Tcl_NextHashEntry(&search)) {
	    cmds[i] = (Command *) Tcl_GetHashValue(entryPtr);
	    cmds[i]->refCount++;
	    i++;
	}
	for (i = 0 ; i < length ; i++) {
	    Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmds[i]);
	    TclCleanupCommandMacro(cmds[i]);
	}
	TclStackFree(interp, cmds);
    }
    Tcl_DeleteHashTable(&nsPtr->cmdTable);
    Tcl_InitHashTable(&nsPtr->cmdTable, TCL_STRING_KEYS);
```

## Consumer bindings

- [rust/tcl-syntax/src/native_namespace_name.rs](../../../../rust/tcl-syntax/src/native_namespace_name.rs), `NativeNamespaceNameRecipe::command_teardown`: Select pure C release command-retirement order without holder/token authority.
- [rust/tcl-syntax/src/native_namespace_name.rs](../../../../rust/tcl-syntax/src/native_namespace_name.rs), `NativeNamespaceCommandTeardown::frontier_len`: Select one current first entry or a complete available pass according to the pure order recipe.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::namespace_command_teardown`: Expose the selected C recipe and refuse Jim without manufacturing a namespace/command owner.

## Evidence

- [rust/tcl-registry/tests/data/native_namespace_command_teardown227/question.json](../../../../rust/tcl-registry/tests/data/native_namespace_command_teardown227/question.json)
- [rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-8.4.20-command-teardown.txt](../../../../rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-8.4.20-command-teardown.txt)
- [rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-8.5.19-command-teardown.txt](../../../../rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-8.5.19-command-teardown.txt)
- [rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-8.6.18-command-teardown.txt](../../../../rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-8.6.18-command-teardown.txt)
- [rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-9.0.4-command-teardown.txt](../../../../rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-9.0.4-command-teardown.txt)
- [rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-9.1.0-command-teardown.txt](../../../../rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-9.1.0-command-teardown.txt)
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs)
- [rust/tcl-syntax/src/native_namespace_name.rs](../../../../rust/tcl-syntax/src/native_namespace_name.rs)
