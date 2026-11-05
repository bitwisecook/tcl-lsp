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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `inspected`. Channel: Inspected source functions, not a guest source invocation. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Actual child allocation, child-table entry and command token retain separate lifetime and placement owners.

### jim

Status: `not-tested`.

No observation for this question.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Source inspection

tcl8.4 8.4.20, revision `8.4.20`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInterp.c`, function `ChildCreate/SlaveCreate`, lines 1811–1817. Retained reference `tcl8.4-creation`.

```text
    slavePtr->masterInterp = masterInterp;
    slavePtr->slaveEntryPtr = hPtr;
    slavePtr->slaveInterp = slaveInterp;
    slavePtr->interpCmd = Tcl_CreateObjCommand(masterInterp, path,
            SlaveObjCmd, (ClientData) slaveInterp, SlaveObjCmdDeleteProc);
    Tcl_InitHashTable(&slavePtr->aliasTable, TCL_STRING_KEYS);
    Tcl_SetHashValue(hPtr, (ClientData) slavePtr);

```

tcl8.4 8.4.20, revision `8.4.20`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInterp.c`, function `Tcl_InterpObjCmd/OPT_DELETE`, lines 494–515. Retained reference `tcl8.4-interp-delete`.

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

tcl8.4 8.4.20, revision `8.4.20`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInterp.c`, function `SlaveObjCmdDeleteProc`, lines 2033–2059. Retained reference `tcl8.4-delete-callback`.

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

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInterp.c`, function `ChildCreate/SlaveCreate`, lines 2183–2189. Retained reference `tcl8.5-creation`.

```text
    slavePtr->masterInterp = masterInterp;
    slavePtr->slaveEntryPtr = hPtr;
    slavePtr->slaveInterp = slaveInterp;
    slavePtr->interpCmd = Tcl_CreateObjCommand(masterInterp, path,
	    SlaveObjCmd, slaveInterp, SlaveObjCmdDeleteProc);
    Tcl_InitHashTable(&slavePtr->aliasTable, TCL_STRING_KEYS);
    Tcl_SetHashValue(hPtr, slavePtr);

```

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInterp.c`, function `Tcl_InterpObjCmd/OPT_DELETE`, lines 758–777. Retained reference `tcl8.5-interp-delete`.

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

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInterp.c`, function `SlaveObjCmdDeleteProc`, lines 2473–2499. Retained reference `tcl8.5-delete-callback`.

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

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclInterp.c`, function `ChildCreate/SlaveCreate`, lines 2357–2363. Retained reference `tcl8.6-creation`.

```text
    childPtr->parentInterp = parentInterp;
    childPtr->childEntryPtr = hPtr;
    childPtr->childInterp = childInterp;
    childPtr->interpCmd = Tcl_NRCreateCommand(parentInterp, path,
	    ChildObjCmd, NRChildCmd, childInterp, ChildObjCmdDeleteProc);
    Tcl_InitHashTable(&childPtr->aliasTable, TCL_STRING_KEYS);
    Tcl_SetHashValue(hPtr, childPtr);

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclInterp.c`, function `Tcl_InterpObjCmd/OPT_DELETE`, lines 836–856. Retained reference `tcl8.6-interp-delete`.

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

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclInterp.c`, function `ChildObjCmdDeleteProc`, lines 2660–2686. Retained reference `tcl8.6-delete-callback`.

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

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInterp.c`, function `ChildCreate/SlaveCreate`, lines 2432–2438. Retained reference `tcl9.0-creation`.

```text
    childPtr->parentInterp = parentInterp;
    childPtr->childEntryPtr = hPtr;
    childPtr->childInterp = childInterp;
    childPtr->interpCmd = Tcl_NRCreateCommand(parentInterp, path,
	    TclChildObjCmd, NRChildCmd, childInterp, ChildObjCmdDeleteProc);
    Tcl_InitHashTable(&childPtr->aliasTable, TCL_STRING_KEYS);
    Tcl_SetHashValue(hPtr, childPtr);

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInterp.c`, function `Tcl_InterpObjCmd/OPT_DELETE`, lines 890–909. Retained reference `tcl9.0-interp-delete`.

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

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInterp.c`, function `ChildObjCmdDeleteProc`, lines 2733–2759. Retained reference `tcl9.0-delete-callback`.

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

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInterp.c`, function `ChildCreate/SlaveCreate`, lines 2817–2823. Retained reference `tcl9.1-creation`.

```text
    childPtr->parentInterp = parentInterp;
    childPtr->childEntryPtr = hPtr;
    childPtr->childInterp = childInterp;
    childPtr->interpCmd = Tcl_NRCreateCommand2(parentInterp, path,
	    TclChildObjCmd, NRChildCmd, childInterp, ChildObjCmdDeleteProc);
    Tcl_InitHashTable(&childPtr->aliasTable, TCL_STRING_KEYS);
    Tcl_SetHashValue(hPtr, childPtr);

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInterp.c`, function `Tcl_InterpObjCmd/OPT_DELETE`, lines 1264–1283. Retained reference `tcl9.1-interp-delete`.

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

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInterp.c`, function `ChildObjCmdDeleteProc`, lines 3124–3150. Retained reference `tcl9.1-delete-callback`.

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

## Evidence

- [runtime/rust/tests/data/native_child_command_lifetime/tcl8.4-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl8.4-source.json)
- [runtime/rust/tests/data/native_child_command_lifetime/tcl8.5-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl8.5-source.json)
- [runtime/rust/tests/data/native_child_command_lifetime/tcl8.6-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl8.6-source.json)
- [runtime/rust/tests/data/native_child_command_lifetime/tcl9.0-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl9.0-source.json)
- [runtime/rust/tests/data/native_child_command_lifetime/tcl9.1-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl9.1-source.json)
