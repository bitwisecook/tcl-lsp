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

### tcl8.4

Status: `inspected`. Version: 8.4.20. Build: Pinned full original release tclNamesp.c implementation; no execution claim.. Channel: Exact original source inspection, not guest Tcl evaluation.. Dialect: tcl8.4.

TclTeardownNamespace restarts the first table entry after each deletion. Source text supplies no actual holder/token/callback or interpreter process result.

### tcl8.5

Status: `inspected`. Version: 8.5.19. Build: Pinned full original release tclNamesp.c implementation; no execution claim.. Channel: Exact original source inspection, not guest Tcl evaluation.. Dialect: tcl8.5.

TclTeardownNamespace restarts the first table entry after each deletion. Source text supplies no actual holder/token/callback or interpreter process result.

### tcl8.6

Status: `inspected`. Version: 8.6.18. Build: Pinned full original release tclNamesp.c implementation; no execution claim.. Channel: Exact original source inspection, not guest Tcl evaluation.. Dialect: tcl8.6.

TclTeardownNamespace retains a complete command-pointer pass and repeats until empty. Source text supplies no actual holder/token/callback or interpreter process result.

### tcl9.0

Status: `inspected`. Version: 9.0.4. Build: Pinned full original release tclNamesp.c implementation; no execution claim.. Channel: Exact original source inspection, not guest Tcl evaluation.. Dialect: tcl9.0.

TclTeardownNamespace retains a complete command-pointer pass and repeats until empty. Source text supplies no actual holder/token/callback or interpreter process result.

### tcl9.1

Status: `inspected`. Version: 9.1.0. Build: Pinned full original release tclNamesp.c implementation; no execution claim.. Channel: Exact original source inspection, not guest Tcl evaluation.. Dialect: tcl9.1.

TclTeardownNamespace retains a complete command-pointer pass and repeats until empty. Source text supplies no actual holder/token/callback or interpreter process result.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No exact source inspection or interpreter execution is recorded for this C teardown question.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No exact source inspection or interpreter execution is recorded for this C teardown question.

## Exact evidence

- `namespace-teardown227-question.json` (input): [rust/tcl-registry/tests/data/native_namespace_command_teardown227/question.json](../../../../rust/tcl-registry/tests/data/native_namespace_command_teardown227/question.json). SHA-256 `6266877387102d54b746a6a2b6c85004412c11512d9c4f36d49181c0ddca12ea`. Exact full original release source or source-inspection request/excerpt; no native execution result.
- `namespace-teardown227-tcl-8.4.20-command-teardown.txt` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-8.4.20-command-teardown.txt](../../../../rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-8.4.20-command-teardown.txt). SHA-256 `25e45517b8c90fabc792156b80bee403604b0fccda14ddfcabb75c319d14f9bb`. Exact full original release source or source-inspection request/excerpt; no native execution result.
- `namespace-teardown227-tcl-8.4.20-tclNamesp.c` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-8.4.20-tclNamesp.c](../../../../rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-8.4.20-tclNamesp.c). SHA-256 `cd23865ee8d4b0ce81874d519b24de5b84a16c116ed3593ead366946b0512cbb`. Exact full original release source or source-inspection request/excerpt; no native execution result.
- `namespace-teardown227-tcl-8.5.19-command-teardown.txt` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-8.5.19-command-teardown.txt](../../../../rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-8.5.19-command-teardown.txt). SHA-256 `eadfe9fb1eceab0de4305e3644fc2ebf142fdc1d9b8128d5ee0f5fc1e1e51422`. Exact full original release source or source-inspection request/excerpt; no native execution result.
- `namespace-teardown227-tcl-8.5.19-tclNamesp.c` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-8.5.19-tclNamesp.c](../../../../rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-8.5.19-tclNamesp.c). SHA-256 `60f944eb9ab669e183a546297c5aa6c8b62af0f79cdab115c74d37497c1fd83f`. Exact full original release source or source-inspection request/excerpt; no native execution result.
- `namespace-teardown227-tcl-8.6.18-command-teardown.txt` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-8.6.18-command-teardown.txt](../../../../rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-8.6.18-command-teardown.txt). SHA-256 `7254f4aaa3b4568bc07966467400ad6427ad72c7e14c0b12168a664df17af553`. Exact full original release source or source-inspection request/excerpt; no native execution result.
- `namespace-teardown227-tcl-8.6.18-tclNamesp.c` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-8.6.18-tclNamesp.c](../../../../rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-8.6.18-tclNamesp.c). SHA-256 `b4b3095f69b2192f13bf355d70a0efeb9aa78b5e6ceccf6b17536cc5e9be1e24`. Exact full original release source or source-inspection request/excerpt; no native execution result.
- `namespace-teardown227-tcl-9.0.4-command-teardown.txt` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-9.0.4-command-teardown.txt](../../../../rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-9.0.4-command-teardown.txt). SHA-256 `cc31f776e8d614edd90c8384b3f8c57d0f2b224ca1f20a1ecc725f1282bf5c6b`. Exact full original release source or source-inspection request/excerpt; no native execution result.
- `namespace-teardown227-tcl-9.0.4-tclNamesp.c` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-9.0.4-tclNamesp.c](../../../../rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-9.0.4-tclNamesp.c). SHA-256 `73e79904d662d12564dd7d35f922fd896566411524a6e14c6454325f2ce17b8d`. Exact full original release source or source-inspection request/excerpt; no native execution result.
- `namespace-teardown227-tcl-9.1.0-command-teardown.txt` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-9.1.0-command-teardown.txt](../../../../rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-9.1.0-command-teardown.txt). SHA-256 `cc31f776e8d614edd90c8384b3f8c57d0f2b224ca1f20a1ecc725f1282bf5c6b`. Exact full original release source or source-inspection request/excerpt; no native execution result.
- `namespace-teardown227-tcl-9.1.0-tclNamesp.c` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-9.1.0-tclNamesp.c](../../../../rust/tcl-registry/tests/data/native_namespace_command_teardown227/tcl-9.1.0-tclNamesp.c). SHA-256 `fce8ca8ee9a21f2e87f7a1917e7a2e4109f9fadf72fd0e585673d1450409b815`. Exact full original release source or source-inspection request/excerpt; no native execution result.
- `naming-namespace-command-teardown-frontier-native.rs` (implementation): [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs). SHA-256 `afaa4a361e346dc0edc267785d286e50174e5ec424c265e0970e09261bb168c5`. Current selected source/API owner; no executed assertion or native provider observation.
- `naming-namespace-command-teardown-frontier-native_namespace_name.rs` (implementation): [rust/tcl-syntax/src/native_namespace_name.rs](../../../../rust/tcl-syntax/src/native_namespace_name.rs). SHA-256 `bdf94315fab13c1f734be52fab731e0f6d7ff543e625387e5eb7057f34ea403a`. Current selected source/API owner; no executed assertion or native provider observation.

## Source inspection

tcl8.4 8.4.20, revision `Source-control revision unrecorded; exact release source and excerpt SHA retained.`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclNamesp.c`, function `TclTeardownNamespace`, lines 746–759. Full-source SHA-256 `cd23865ee8d4b0ce81874d519b24de5b84a16c116ed3593ead366946b0512cbb`; snippet SHA-256 `25e45517b8c90fabc792156b80bee403604b0fccda14ddfcabb75c319d14f9bb`; retained evidence `namespace-teardown227-tcl-8.4.20-command-teardown.txt`.

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

tcl8.5 8.5.19, revision `Source-control revision unrecorded; exact release source and excerpt SHA retained.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclNamesp.c`, function `TclTeardownNamespace`, lines 1107–1122. Full-source SHA-256 `60f944eb9ab669e183a546297c5aa6c8b62af0f79cdab115c74d37497c1fd83f`; snippet SHA-256 `eadfe9fb1eceab0de4305e3644fc2ebf142fdc1d9b8128d5ee0f5fc1e1e51422`; retained evidence `namespace-teardown227-tcl-8.5.19-command-teardown.txt`.

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

tcl8.6 8.6.18, revision `Source-control revision unrecorded; exact release source and excerpt SHA retained.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclNamesp.c`, function `TclTeardownNamespace`, lines 1090–1119. Full-source SHA-256 `b4b3095f69b2192f13bf355d70a0efeb9aa78b5e6ceccf6b17536cc5e9be1e24`; snippet SHA-256 `7254f4aaa3b4568bc07966467400ad6427ad72c7e14c0b12168a664df17af553`; retained evidence `namespace-teardown227-tcl-8.6.18-command-teardown.txt`.

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

tcl9.0 9.0.4, revision `Source-control revision unrecorded; exact release source and excerpt SHA retained.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclNamesp.c`, function `TclTeardownNamespace`, lines 1274–1303. Full-source SHA-256 `73e79904d662d12564dd7d35f922fd896566411524a6e14c6454325f2ce17b8d`; snippet SHA-256 `cc31f776e8d614edd90c8384b3f8c57d0f2b224ca1f20a1ecc725f1282bf5c6b`; retained evidence `namespace-teardown227-tcl-9.0.4-command-teardown.txt`.

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

tcl9.1 9.1.0, revision `Source-control revision unrecorded; exact release source and excerpt SHA retained.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclNamesp.c`, function `TclTeardownNamespace`, lines 1273–1302. Full-source SHA-256 `fce8ca8ee9a21f2e87f7a1917e7a2e4109f9fadf72fd0e585673d1450409b815`; snippet SHA-256 `cc31f776e8d614edd90c8384b3f8c57d0f2b224ca1f20a1ecc725f1282bf5c6b`; retained evidence `namespace-teardown227-tcl-9.1.0-command-teardown.txt`.

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

A named test is a coverage binding, not a claim that it executed.

## Replay

Exact independently pinned source copies and LF-counted excerpt lines retained. Source inspection is read-only; no provider/build/Rust command is launched. Original Native202 public receipts remain independent.
