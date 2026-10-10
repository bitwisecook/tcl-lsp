# naming.interpreter.child-command-allocation-lifetime

Kind: `source-anchor`

## Problem statement

A child-table key, current command placement and actual child interpreter are different owners. Deleting by the original name can miss a renamed child command or remove a replacement.

## Question

Which owners do the pinned C implementations retain for child-command publication and deletion?

## Conclusion

C8.4 through C9.1 retain the child interpreter as command client data and store the actual command token in the child record. Interp delete deletes that token. Its delete callback unlinks the actual child-table entry, clears the command pointer and requests child deletion. Command rename does not replace these owners. This source inspection supplies no Runtime execution, native object/callback capability or arbitrary teardown completion claim.

## Scope

Exactly retained C child-creation, interp-delete and child-command-delete callback source windows. Jim and BIGIP source/behavior are not inspected for this question. Runtime adapters retain a weak actual child allocation and independent command generation; an executing activation owns the strong lifetime.

## Provider answers

### tcl8.4

Status: `inspected`. Version: 8.4.20. Build: Pinned source only; no launched build observation. Channel: Inspected source functions, not a guest source invocation. Dialect: Tcl.

Actual child allocation, child-table entry and command token retain separate lifetime and placement owners.

### tcl8.5

Status: `inspected`. Version: 8.5.19. Build: Pinned source only; no launched build observation. Channel: Inspected source functions, not a guest source invocation. Dialect: Tcl.

Actual child allocation, child-table entry and command token retain separate lifetime and placement owners.

### tcl8.6

Status: `inspected`. Version: 8.6.18. Build: Pinned source only; no launched build observation. Channel: Inspected source functions, not a guest source invocation. Dialect: Tcl.

Actual child allocation, child-table entry and command token retain separate lifetime and placement owners.

### tcl9.0

Status: `inspected`. Version: 9.0.4. Build: Pinned source only; no launched build observation. Channel: Inspected source functions, not a guest source invocation. Dialect: Tcl.

Actual child allocation, child-table entry and command token retain separate lifetime and placement owners.

### tcl9.1

Status: `inspected`. Version: 9.1.0. Build: Pinned source only; no launched build observation. Channel: Inspected source functions, not a guest source invocation. Dialect: Tcl.

Actual child allocation, child-table entry and command token retain separate lifetime and placement owners.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `tcl8.4-creation` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl8.4-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl8.4-source.json). SHA-256 `7b2b7ecde38180b5b6529c8f45db7df86d95fbf1b8e1b8c4a28ce43d4275638d`. JSON pointer `/windows/0/snippet`. Exact pinned source-only creation window; no launch or linked-library observation.
- `tcl8.4-interp-delete` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl8.4-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl8.4-source.json). SHA-256 `7b2b7ecde38180b5b6529c8f45db7df86d95fbf1b8e1b8c4a28ce43d4275638d`. JSON pointer `/windows/1/snippet`. Exact pinned source-only interp-delete window; no launch or linked-library observation.
- `tcl8.4-delete-callback` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl8.4-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl8.4-source.json). SHA-256 `7b2b7ecde38180b5b6529c8f45db7df86d95fbf1b8e1b8c4a28ce43d4275638d`. JSON pointer `/windows/2/snippet`. Exact pinned source-only delete-callback window; no launch or linked-library observation.
- `tcl8.5-creation` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl8.5-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl8.5-source.json). SHA-256 `72d65ebe3f2250cc2a44efc34ab91059522d623d1f0e1804ea1401bae5fafbaf`. JSON pointer `/windows/0/snippet`. Exact pinned source-only creation window; no launch or linked-library observation.
- `tcl8.5-interp-delete` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl8.5-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl8.5-source.json). SHA-256 `72d65ebe3f2250cc2a44efc34ab91059522d623d1f0e1804ea1401bae5fafbaf`. JSON pointer `/windows/1/snippet`. Exact pinned source-only interp-delete window; no launch or linked-library observation.
- `tcl8.5-delete-callback` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl8.5-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl8.5-source.json). SHA-256 `72d65ebe3f2250cc2a44efc34ab91059522d623d1f0e1804ea1401bae5fafbaf`. JSON pointer `/windows/2/snippet`. Exact pinned source-only delete-callback window; no launch or linked-library observation.
- `tcl8.6-creation` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl8.6-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl8.6-source.json). SHA-256 `bd42a5ad3a2c9ce770a529c54185414eb07a51dcd4b63d564ea6db4042e01dfc`. JSON pointer `/windows/0/snippet`. Exact pinned source-only creation window; no launch or linked-library observation.
- `tcl8.6-interp-delete` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl8.6-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl8.6-source.json). SHA-256 `bd42a5ad3a2c9ce770a529c54185414eb07a51dcd4b63d564ea6db4042e01dfc`. JSON pointer `/windows/1/snippet`. Exact pinned source-only interp-delete window; no launch or linked-library observation.
- `tcl8.6-delete-callback` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl8.6-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl8.6-source.json). SHA-256 `bd42a5ad3a2c9ce770a529c54185414eb07a51dcd4b63d564ea6db4042e01dfc`. JSON pointer `/windows/2/snippet`. Exact pinned source-only delete-callback window; no launch or linked-library observation.
- `tcl9.0-creation` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl9.0-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl9.0-source.json). SHA-256 `6e5491d559829b7a24ebc14ffa293cfeeb4fb2e7b424acc981de12304b5942f6`. JSON pointer `/windows/0/snippet`. Exact pinned source-only creation window; no launch or linked-library observation.
- `tcl9.0-interp-delete` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl9.0-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl9.0-source.json). SHA-256 `6e5491d559829b7a24ebc14ffa293cfeeb4fb2e7b424acc981de12304b5942f6`. JSON pointer `/windows/1/snippet`. Exact pinned source-only interp-delete window; no launch or linked-library observation.
- `tcl9.0-delete-callback` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl9.0-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl9.0-source.json). SHA-256 `6e5491d559829b7a24ebc14ffa293cfeeb4fb2e7b424acc981de12304b5942f6`. JSON pointer `/windows/2/snippet`. Exact pinned source-only delete-callback window; no launch or linked-library observation.
- `tcl9.1-creation` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl9.1-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl9.1-source.json). SHA-256 `32ed4747a8937687a376382ac79af48bce3f78b553002a9ba343e657fe2c27c8`. JSON pointer `/windows/0/snippet`. Exact pinned source-only creation window; no launch or linked-library observation.
- `tcl9.1-interp-delete` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl9.1-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl9.1-source.json). SHA-256 `32ed4747a8937687a376382ac79af48bce3f78b553002a9ba343e657fe2c27c8`. JSON pointer `/windows/1/snippet`. Exact pinned source-only interp-delete window; no launch or linked-library observation.
- `tcl9.1-delete-callback` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl9.1-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl9.1-source.json). SHA-256 `32ed4747a8937687a376382ac79af48bce3f78b553002a9ba343e657fe2c27c8`. JSON pointer `/windows/2/snippet`. Exact pinned source-only delete-callback window; no launch or linked-library observation.

## Source inspection

tcl8.4 8.4.20, revision `8.4.20`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInterp.c`, function `ChildCreate/SlaveCreate`, lines 1811–1817. Full-source SHA-256 `76c22a84c35e960f7936999f0f19a1aef60c83756a4f3d26d885cfdbd4288230`; snippet SHA-256 `9d4574f44b2dd6292ae23ba1cb56ddd3c478173f9946ca48605b8f1a4f426311`; retained evidence `tcl8.4-creation`.

```text
    slavePtr->masterInterp = masterInterp;
    slavePtr->slaveEntryPtr = hPtr;
    slavePtr->slaveInterp = slaveInterp;
    slavePtr->interpCmd = Tcl_CreateObjCommand(masterInterp, path,
            SlaveObjCmd, (ClientData) slaveInterp, SlaveObjCmdDeleteProc);
    Tcl_InitHashTable(&slavePtr->aliasTable, TCL_STRING_KEYS);
    Tcl_SetHashValue(hPtr, (ClientData) slavePtr);

```

tcl8.4 8.4.20, revision `8.4.20`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInterp.c`, function `Tcl_InterpObjCmd/OPT_DELETE`, lines 494–515. Full-source SHA-256 `76c22a84c35e960f7936999f0f19a1aef60c83756a4f3d26d885cfdbd4288230`; snippet SHA-256 `c39192a6bef46bf31506f5ffb2ac7569c2c5bd22be0938693c18ed442fb74a1e`; retained evidence `tcl8.4-interp-delete`.

```text
	case OPT_DELETE: {
	    int i;
	    InterpInfo *iiPtr;
	    Tcl_Interp *slaveInterp;
	    
	    for (i = 2; i < objc; i++) {
		slaveInterp = GetInterp(interp, objv[i]);
		if (slaveInterp == NULL) {
		    return TCL_ERROR;
		} else if (slaveInterp == interp) {
		    Tcl_ResetResult(interp);
		    Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
			    "cannot delete the current interpreter",
			    (char *) NULL);
		    return TCL_ERROR;
		}
		iiPtr = (InterpInfo *) ((Interp *) slaveInterp)->interpInfo;
		Tcl_DeleteCommandFromToken(iiPtr->slave.masterInterp,
			iiPtr->slave.interpCmd);
	    }
	    return TCL_OK;
	}

```

tcl8.4 8.4.20, revision `8.4.20`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInterp.c`, function `SlaveObjCmdDeleteProc`, lines 2033–2059. Full-source SHA-256 `76c22a84c35e960f7936999f0f19a1aef60c83756a4f3d26d885cfdbd4288230`; snippet SHA-256 `0668b4fa49727a4ad04ae0d02a8b9dfd06b9fff8616b374e15916fe2f1464bac`; retained evidence `tcl8.4-delete-callback`.

```text
SlaveObjCmdDeleteProc(clientData)
    ClientData clientData;		/* The SlaveRecord for the command. */
{
    Slave *slavePtr;			/* Interim storage for Slave record. */
    Tcl_Interp *slaveInterp;		/* And for a slave interp. */

    slaveInterp = (Tcl_Interp *) clientData;
    slavePtr = &((InterpInfo *) ((Interp *) slaveInterp)->interpInfo)->slave;

    /*
     * Unlink the slave from its master interpreter.
     */

    Tcl_DeleteHashEntry(slavePtr->slaveEntryPtr);

    /*
     * Set to NULL so that when the InterpInfo is cleaned up in the slave
     * it does not try to delete the command causing all sorts of grief.
     * See SlaveRecordDeleteProc().
     */

    slavePtr->interpCmd = NULL;

    if (slavePtr->slaveInterp != NULL) {
	Tcl_DeleteInterp(slavePtr->slaveInterp);
    }
}

```

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInterp.c`, function `ChildCreate/SlaveCreate`, lines 2183–2189. Full-source SHA-256 `958e2492c5afe9e57a7776cd774ed995224d9c93402b02708ef8c6611588cc04`; snippet SHA-256 `9acb2768c0ef1eaa69546d55d1a052eefa1f1c0b4ad09d2e501f2ed1dcf488af`; retained evidence `tcl8.5-creation`.

```text
    slavePtr->masterInterp = masterInterp;
    slavePtr->slaveEntryPtr = hPtr;
    slavePtr->slaveInterp = slaveInterp;
    slavePtr->interpCmd = Tcl_CreateObjCommand(masterInterp, path,
	    SlaveObjCmd, slaveInterp, SlaveObjCmdDeleteProc);
    Tcl_InitHashTable(&slavePtr->aliasTable, TCL_STRING_KEYS);
    Tcl_SetHashValue(hPtr, slavePtr);

```

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInterp.c`, function `Tcl_InterpObjCmd/OPT_DELETE`, lines 758–777. Full-source SHA-256 `958e2492c5afe9e57a7776cd774ed995224d9c93402b02708ef8c6611588cc04`; snippet SHA-256 `94c4c95eaf90f0f9df1453fff2981ae0fcdcf9b7139457cb85eb370b2ebbc38e`; retained evidence `tcl8.5-interp-delete`.

```text
    case OPT_DELETE: {
	int i;
	InterpInfo *iiPtr;
	Tcl_Interp *slaveInterp;

	for (i = 2; i < objc; i++) {
	    slaveInterp = GetInterp(interp, objv[i]);
	    if (slaveInterp == NULL) {
		return TCL_ERROR;
	    } else if (slaveInterp == interp) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"cannot delete the current interpreter", -1));
		return TCL_ERROR;
	    }
	    iiPtr = (InterpInfo *) ((Interp *) slaveInterp)->interpInfo;
	    Tcl_DeleteCommandFromToken(iiPtr->slave.masterInterp,
		    iiPtr->slave.interpCmd);
	}
	return TCL_OK;
    }

```

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInterp.c`, function `SlaveObjCmdDeleteProc`, lines 2473–2499. Full-source SHA-256 `958e2492c5afe9e57a7776cd774ed995224d9c93402b02708ef8c6611588cc04`; snippet SHA-256 `938f7c621da7d32c7c132f8af8a108def658a9abb2cf28ede3d5e693ee2bb22e`; retained evidence `tcl8.5-delete-callback`.

```text
SlaveObjCmdDeleteProc(
    ClientData clientData)	/* The SlaveRecord for the command. */
{
    Slave *slavePtr;		/* Interim storage for Slave record. */
    Tcl_Interp *slaveInterp = clientData;
				/* And for a slave interp. */

    slavePtr = &((InterpInfo *) ((Interp *) slaveInterp)->interpInfo)->slave;

    /*
     * Unlink the slave from its master interpreter.
     */

    Tcl_DeleteHashEntry(slavePtr->slaveEntryPtr);

    /*
     * Set to NULL so that when the InterpInfo is cleaned up in the slave it
     * does not try to delete the command causing all sorts of grief. See
     * SlaveRecordDeleteProc().
     */

    slavePtr->interpCmd = NULL;

    if (slavePtr->slaveInterp != NULL) {
	Tcl_DeleteInterp(slavePtr->slaveInterp);
    }
}

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclInterp.c`, function `ChildCreate/SlaveCreate`, lines 2357–2363. Full-source SHA-256 `b2597dfae723709bd21c6dfe9dd0cd4eb6d38f90ea082419b59a2aa2c00881ca`; snippet SHA-256 `355e5db97ad10426003809f3a0293c83eb6c515e9b87c5b48a24b5049cbbe669`; retained evidence `tcl8.6-creation`.

```text
    childPtr->parentInterp = parentInterp;
    childPtr->childEntryPtr = hPtr;
    childPtr->childInterp = childInterp;
    childPtr->interpCmd = Tcl_NRCreateCommand(parentInterp, path,
	    ChildObjCmd, NRChildCmd, childInterp, ChildObjCmdDeleteProc);
    Tcl_InitHashTable(&childPtr->aliasTable, TCL_STRING_KEYS);
    Tcl_SetHashValue(hPtr, childPtr);

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclInterp.c`, function `Tcl_InterpObjCmd/OPT_DELETE`, lines 836–856. Full-source SHA-256 `b2597dfae723709bd21c6dfe9dd0cd4eb6d38f90ea082419b59a2aa2c00881ca`; snippet SHA-256 `2f37e2859c960d6b37d5a4907193cdba358665b7e5042a96e12b65f998ddb5b1`; retained evidence `tcl8.6-interp-delete`.

```text
    case OPT_DELETE: {
	int i;
	InterpInfo *iiPtr;

	for (i = 2; i < objc; i++) {
	    childInterp = GetInterp(interp, objv[i]);
	    if (childInterp == NULL) {
		return TCL_ERROR;
	    } else if (childInterp == interp) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"cannot delete the current interpreter", -1));
		Tcl_SetErrorCode(interp, "TCL", "OPERATION", "INTERP",
			"DELETESELF", (char *)NULL);
		return TCL_ERROR;
	    }
	    iiPtr = (InterpInfo *) ((Interp *) childInterp)->interpInfo;
	    Tcl_DeleteCommandFromToken(iiPtr->child.parentInterp,
		    iiPtr->child.interpCmd);
	}
	return TCL_OK;
    }

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclInterp.c`, function `ChildObjCmdDeleteProc`, lines 2660–2686. Full-source SHA-256 `b2597dfae723709bd21c6dfe9dd0cd4eb6d38f90ea082419b59a2aa2c00881ca`; snippet SHA-256 `f54c68afd757f139307b9eeae6417076bf6aeb2adf07bd8faf6e18fc7bad4590`; retained evidence `tcl8.6-delete-callback`.

```text
ChildObjCmdDeleteProc(
    ClientData clientData)	/* The ChildRecord for the command. */
{
    Child *childPtr;		/* Interim storage for Child record. */
    Tcl_Interp *childInterp = (Tcl_Interp *)clientData;
				/* And for a child interp. */

    childPtr = &((InterpInfo *) ((Interp *) childInterp)->interpInfo)->child;

    /*
     * Unlink the child from its parent interpreter.
     */

    Tcl_DeleteHashEntry(childPtr->childEntryPtr);

    /*
     * Set to NULL so that when the InterpInfo is cleaned up in the child it
     * does not try to delete the command causing all sorts of grief. See
     * ChildRecordDeleteProc().
     */

    childPtr->interpCmd = NULL;

    if (childPtr->childInterp != NULL) {
	Tcl_DeleteInterp(childPtr->childInterp);
    }
}

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInterp.c`, function `ChildCreate/SlaveCreate`, lines 2432–2438. Full-source SHA-256 `5c944998c2f89689ef9f591983ea7b0e322dd40756fe5baca641e2258969b125`; snippet SHA-256 `10c9ad92cd9ca9387a395eca44945c9b25acc51553f9a03917894fe0040082e3`; retained evidence `tcl9.0-creation`.

```text
    childPtr->parentInterp = parentInterp;
    childPtr->childEntryPtr = hPtr;
    childPtr->childInterp = childInterp;
    childPtr->interpCmd = Tcl_NRCreateCommand(parentInterp, path,
	    TclChildObjCmd, NRChildCmd, childInterp, ChildObjCmdDeleteProc);
    Tcl_InitHashTable(&childPtr->aliasTable, TCL_STRING_KEYS);
    Tcl_SetHashValue(hPtr, childPtr);

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInterp.c`, function `Tcl_InterpObjCmd/OPT_DELETE`, lines 890–909. Full-source SHA-256 `5c944998c2f89689ef9f591983ea7b0e322dd40756fe5baca641e2258969b125`; snippet SHA-256 `913bbf31d6064cce8e8c6f4c46bb37ac6db5ebd0ea541811dcfad023864c0822`; retained evidence `tcl9.0-interp-delete`.

```text
    case OPT_DELETE: {
	InterpInfo *iiPtr;

	for (i = 2; i < objc; i++) {
	    childInterp = GetInterp(interp, objv[i]);
	    if (childInterp == NULL) {
		return TCL_ERROR;
	    } else if (childInterp == interp) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"cannot delete the current interpreter", -1));
		Tcl_SetErrorCode(interp, "TCL", "OPERATION", "INTERP",
			"DELETESELF", (char *)NULL);
		return TCL_ERROR;
	    }
	    iiPtr = INTERP_INFO(childInterp);
	    Tcl_DeleteCommandFromToken(iiPtr->child.parentInterp,
		    iiPtr->child.interpCmd);
	}
	return TCL_OK;
    }

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInterp.c`, function `ChildObjCmdDeleteProc`, lines 2733–2759. Full-source SHA-256 `5c944998c2f89689ef9f591983ea7b0e322dd40756fe5baca641e2258969b125`; snippet SHA-256 `27d942f4c60ee0fac0848caf67951b402693280322212145c6614c2df152cf09`; retained evidence `tcl9.0-delete-callback`.

```text
ChildObjCmdDeleteProc(
    void *clientData)		/* The ChildRecord for the command. */
{
    Child *childPtr;		/* Interim storage for Child record. */
    Tcl_Interp *childInterp = (Tcl_Interp *) clientData;
				/* And for a child interp. */

    childPtr = &INTERP_INFO(childInterp)->child;

    /*
     * Unlink the child from its parent interpreter.
     */

    Tcl_DeleteHashEntry(childPtr->childEntryPtr);

    /*
     * Set to NULL so that when the InterpInfo is cleaned up in the child it
     * does not try to delete the command causing all sorts of grief. See
     * ChildRecordDeleteProc().
     */

    childPtr->interpCmd = NULL;

    if (childPtr->childInterp != NULL) {
	Tcl_DeleteInterp(childPtr->childInterp);
    }
}

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInterp.c`, function `ChildCreate/SlaveCreate`, lines 2817–2823. Full-source SHA-256 `f7880cfc3cd9787502134b22832475df5ebad0432e66c8fae2281ddd98abe59d`; snippet SHA-256 `5c5468db1c54d59efe92f3f598921849a688ae83c08d056fcb5ca446cb0f51ee`; retained evidence `tcl9.1-creation`.

```text
    childPtr->parentInterp = parentInterp;
    childPtr->childEntryPtr = hPtr;
    childPtr->childInterp = childInterp;
    childPtr->interpCmd = Tcl_NRCreateCommand2(parentInterp, path,
	    TclChildObjCmd, NRChildCmd, childInterp, ChildObjCmdDeleteProc);
    Tcl_InitHashTable(&childPtr->aliasTable, TCL_STRING_KEYS);
    Tcl_SetHashValue(hPtr, childPtr);

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInterp.c`, function `Tcl_InterpObjCmd/OPT_DELETE`, lines 1264–1283. Full-source SHA-256 `f7880cfc3cd9787502134b22832475df5ebad0432e66c8fae2281ddd98abe59d`; snippet SHA-256 `913bbf31d6064cce8e8c6f4c46bb37ac6db5ebd0ea541811dcfad023864c0822`; retained evidence `tcl9.1-interp-delete`.

```text
    case OPT_DELETE: {
	InterpInfo *iiPtr;

	for (i = 2; i < objc; i++) {
	    childInterp = GetInterp(interp, objv[i]);
	    if (childInterp == NULL) {
		return TCL_ERROR;
	    } else if (childInterp == interp) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"cannot delete the current interpreter", -1));
		Tcl_SetErrorCode(interp, "TCL", "OPERATION", "INTERP",
			"DELETESELF", (char *)NULL);
		return TCL_ERROR;
	    }
	    iiPtr = INTERP_INFO(childInterp);
	    Tcl_DeleteCommandFromToken(iiPtr->child.parentInterp,
		    iiPtr->child.interpCmd);
	}
	return TCL_OK;
    }

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInterp.c`, function `ChildObjCmdDeleteProc`, lines 3124–3150. Full-source SHA-256 `f7880cfc3cd9787502134b22832475df5ebad0432e66c8fae2281ddd98abe59d`; snippet SHA-256 `27d942f4c60ee0fac0848caf67951b402693280322212145c6614c2df152cf09`; retained evidence `tcl9.1-delete-callback`.

```text
ChildObjCmdDeleteProc(
    void *clientData)		/* The ChildRecord for the command. */
{
    Child *childPtr;		/* Interim storage for Child record. */
    Tcl_Interp *childInterp = (Tcl_Interp *) clientData;
				/* And for a child interp. */

    childPtr = &INTERP_INFO(childInterp)->child;

    /*
     * Unlink the child from its parent interpreter.
     */

    Tcl_DeleteHashEntry(childPtr->childEntryPtr);

    /*
     * Set to NULL so that when the InterpInfo is cleaned up in the child it
     * does not try to delete the command causing all sorts of grief. See
     * ChildRecordDeleteProc().
     */

    childPtr->interpCmd = NULL;

    if (childPtr->childInterp != NULL) {
	Tcl_DeleteInterp(childPtr->childInterp);
    }
}

```


## Consumer bindings

- [runtime/rust/src/interp/native_children.rs](../../../../runtime/rust/src/interp/native_children.rs), `ChildInterpreterCommand::retire`: Retire the independently actual child allocation without owning its teardown lifetime.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::delete_child`: Use retained child command generation and actual current placement, preserving replacements.
- [runtime/rust/src/interp/native_children.rs](../../../../runtime/rust/src/interp/native_children.rs), `interp::native_children::tests::child_commands_keep_namespace_and_allocation_through_rename_and_recreation` (linked): C API unqualified root publication, exact token-directed rename/deletion and refusal to retarget a retained old child command to a new allocation.
- [runtime/rust/src/interp/native_children.rs](../../../../runtime/rust/src/interp/native_children.rs), `interp::native_children::tests::replacing_or_deleting_child_command_retires_only_its_actual_interpreter` (linked): Replacement and active child deletion withdraw the actual old child while preserving new same-name recreation.

A named test is a coverage binding, not a claim that it executed.

## Replay

Reproduce source excerpts against their retained full-file hashes and exact line ranges. No fresh C/Jim/Rust execution is claimed by this packet. Full child interpreter teardown callbacks and cross-interpreter alias target retirement require their independent owners. Source line coordinates count LF only. The separately retained source metadata preserves its exact artifact identity under retained-source-metadata.
