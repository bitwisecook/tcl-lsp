# naming.interpreter.child-command-root-and-qualified-publication

Kind: `native-observation`

## Problem statement

Using script declaration placement for C child command registration incorrectly publishes an unqualified child in the current namespace.

## Question

Where do the measured C providers publish unqualified, absolute-qualified and relative-qualified child command names from a namespace activation?

## Conclusion

All five C providers publish bare kid at ::kid even while ::N is current. Absolute ::N::kid publishes there. Relative N::kid from ::N follows the current qualifier, so the exact ::N::kid call errors. The v2 intermediate controls preserve root/current command tables before failure; the original v1 final lookup failure remains intact. This supplies no arbitrary name or handler/observer grant.

## Scope

Exact counted ASCII source passed to Tcl_EvalEx or original Jim_NewStringObj/Jim_EvalObj on a fresh interpreter per source control. C public active-allocation control preserves Tcl_GetSlave actual interpreter through deletion. Finite cases: v1:namespace-publication, v1:namespace-delete-child-command, v2:namespace-publication-intermediate, v2:namespace-relative-root-lookup, v2:namespace-root-child-survives-delete, v2:namespace-qualified-publication, v2:namespace-relative-qualified-publication. Result primary/resident fields precede the public getter; retained fields do not independently prove object release/callback closure. No Rust execution or whole lifecycle Normal is inferred.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

All five C providers publish bare kid at ::kid even while ::N is current. Absolute ::N::kid publishes there. Relative N::kid from ::N follows the current qualifier, so the exact ::N::kid call errors. The v2 intermediate controls preserve root/current command tables before failure; the original v1 final lookup failure remains intact. This supplies no arbitrary name or handler/observer grant.

### jim

Status: `unsupported`. Channel: Original counted Jim string object passed to Jim_EvalObj; C API explicitly not tested. Version: jim 0.84-9-g5bac7c9.

Actual C-shaped interp source controls each return1/wrong # args: should be "interp". The C public active-allocation API is explicitly not tested; no corresponding child behavior is inferred.

### bigip

Status: `not-tested`.

No appliance observation.

## Source inspection

tcl8.4 8.4.20, revision `8.4.20`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclBasic.c`, function `Tcl_CreateObjCommand`, lines 1772–1786. Retained reference `tcl8.4-c-api-publication`.

```text
     * Determine where the command should reside. If its name contains 
     * namespace qualifiers, we put it in the specified namespace; 
     * otherwise, we always put it in the global namespace.
     */

    if (strstr(cmdName, "::") != NULL) {
       TclGetNamespaceForQualName(interp, cmdName, (Namespace *) NULL,
           CREATE_NS_IF_UNKNOWN, &nsPtr, &dummy1, &dummy2, &tail);
       if ((nsPtr == NULL) || (tail == NULL)) {
	    return (Tcl_Command) NULL;
	}
    } else {
	nsPtr = iPtr->globalNsPtr;
	tail = cmdName;
    }

```

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclBasic.c`, function `Tcl_CreateObjCommand`, lines 2033–2047. Retained reference `tcl8.5-c-api-publication`.

```text
     * Determine where the command should reside. If its name contains
     * namespace qualifiers, we put it in the specified namespace; otherwise,
     * we always put it in the global namespace.
     */

    if (strstr(cmdName, "::") != NULL) {
	TclGetNamespaceForQualName(interp, cmdName, NULL,
		TCL_CREATE_NS_IF_UNKNOWN, &nsPtr, &dummy1, &dummy2, &tail);
	if ((nsPtr == NULL) || (tail == NULL)) {
	    return (Tcl_Command) NULL;
	}
    } else {
	nsPtr = iPtr->globalNsPtr;
	tail = cmdName;
    }

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `Tcl_CreateObjCommand`, lines 2312–2328. Retained reference `tcl8.6-c-api-publication`.

```text
     * Determine where the command should reside. If its name contains
     * namespace qualifiers, we put it in the specified namespace;
     * otherwise, we always put it in the global namespace.
     */

    if (strstr(cmdName, "::") != NULL) {
	Namespace *dummy1, *dummy2;

	TclGetNamespaceForQualName(interp, cmdName, NULL,
	    TCL_CREATE_NS_IF_UNKNOWN, &nsPtr, &dummy1, &dummy2, &tail);
	if ((nsPtr == NULL) || (tail == NULL)) {
	    return (Tcl_Command) NULL;
	}
    } else {
	nsPtr = iPtr->globalNsPtr;
	tail = cmdName;
    }

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `Tcl_CreateObjCommand`, lines 2902–2918. Retained reference `tcl9.0-c-api-publication`.

```text
     * Determine where the command should reside. If its name contains
     * namespace qualifiers, we put it in the specified namespace;
     * otherwise, we always put it in the global namespace.
     */

    if (strstr(cmdName, "::") != NULL) {
	Namespace *dummy1, *dummy2;

	TclGetNamespaceForQualName(interp, cmdName, NULL,
		TCL_CREATE_NS_IF_UNKNOWN, &nsPtr, &dummy1, &dummy2, &tail);
	if ((nsPtr == NULL) || (tail == NULL)) {
	    return (Tcl_Command) NULL;
	}
    } else {
	nsPtr = iPtr->globalNsPtr;
	tail = cmdName;
    }

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `Tcl_CreateObjCommand`, lines 2857–2873. Retained reference `tcl9.1-c-api-publication`.

```text
     * Determine where the command should reside. If its name contains
     * namespace qualifiers, we put it in the specified namespace;
     * otherwise, we always put it in the global namespace.
     */

    if (strstr(cmdName, "::") != NULL) {
	Namespace *dummy1, *dummy2;

	TclGetNamespaceForQualName(interp, cmdName, NULL,
		TCL_CREATE_NS_IF_UNKNOWN, &nsPtr, &dummy1, &dummy2, &tail);
	if ((nsPtr == NULL) || (tail == NULL)) {
	    return (Tcl_Command) NULL;
	}
    } else {
	nsPtr = iPtr->globalNsPtr;
	tail = cmdName;
    }

```

## Consumer bindings

- [runtime/rust/src/namespace.rs](../../../../runtime/rust/src/namespace.rs), `Namespaces::command_c_api_publication_at`: Map the selected native C API ByteCommandSlot to actual namespace tables, without script-publication defaults.
- [runtime/rust/src/interp/native_children.rs](../../../../runtime/rust/src/interp/native_children.rs), `interp::native_children::tests::child_source_controls_match_native_creation_and_retirement_windows` (linked): Compare exact45 current v2 C guest code/result outcomes for unqualified/qualified source placement, expected exact lookup errors and namespace deletion.

## Evidence

- [runtime/rust/tests/data/native_child_command_lifetime/v1](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1)
- [runtime/rust/tests/data/native_child_command_lifetime/v2](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2)
- [runtime/rust/tests/data/native_child_command_lifetime/v1/capture](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture)
- [runtime/rust/tests/data/native_child_command_lifetime/v2/capture](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture)
- [runtime/rust/tests/data](../../../../runtime/rust/tests/data)
