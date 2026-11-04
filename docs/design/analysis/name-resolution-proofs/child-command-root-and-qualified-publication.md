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

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual executable linked to library and header; compiled/exited0.. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Dialect: Tcl.

All five C providers publish bare kid at ::kid even while ::N is current. Absolute ::N::kid publishes there. Relative N::kid from ::N follows the current qualifier, so the exact ::N::kid call errors. The v2 intermediate controls preserve root/current command tables before failure; the original v1 final lookup failure remains intact. This supplies no arbitrary name or handler/observer grant.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual executable linked to library and header; compiled/exited0.. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Dialect: Tcl.

All five C providers publish bare kid at ::kid even while ::N is current. Absolute ::N::kid publishes there. Relative N::kid from ::N follows the current qualifier, so the exact ::N::kid call errors. The v2 intermediate controls preserve root/current command tables before failure; the original v1 final lookup failure remains intact. This supplies no arbitrary name or handler/observer grant.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable linked to library and header; compiled/exited0.. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Dialect: Tcl.

All five C providers publish bare kid at ::kid even while ::N is current. Absolute ::N::kid publishes there. Relative N::kid from ::N follows the current qualifier, so the exact ::N::kid call errors. The v2 intermediate controls preserve root/current command tables before failure; the original v1 final lookup failure remains intact. This supplies no arbitrary name or handler/observer grant.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable linked to library and header; compiled/exited0.. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Dialect: Tcl.

All five C providers publish bare kid at ::kid even while ::N is current. Absolute ::N::kid publishes there. Relative N::kid from ::N follows the current qualifier, so the exact ::N::kid call errors. The v2 intermediate controls preserve root/current command tables before failure; the original v1 final lookup failure remains intact. This supplies no arbitrary name or handler/observer grant.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable linked to library and header; compiled/exited0.. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Dialect: Tcl.

All five C providers publish bare kid at ::kid even while ::N is current. Absolute ::N::kid publishes there. Relative N::kid from ::N follows the current qualifier, so the exact ::N::kid call errors. The v2 intermediate controls preserve root/current command tables before failure; the original v1 final lookup failure remains intact. This supplies no arbitrary name or handler/observer grant.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual executable linked to library and header; compiled/exited0.. Channel: Original counted Jim string object passed to Jim_EvalObj; C API explicitly not tested. Dialect: Jim Tcl.

Actual C-shaped interp source controls each return1/wrong # args: should be "interp". The C public active-allocation API is explicitly not tested; no corresponding child behavior is inferred.

### bigip

Status: `not-tested`. Version: not recorded. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No appliance observation.

## Historical references

- `v1-protocol-probe.c` (input): [runtime/rust/tests/data/native_child_command_lifetime/v1/protocol/probe.c](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/protocol/probe.c). Exact retained v1 protocol/probe.c; each guest code is independent of compile/process exit.
- `v1-protocol-inputs.json` (input): [runtime/rust/tests/data/native_child_command_lifetime/v1/protocol/inputs.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/protocol/inputs.json). Exact retained v1 protocol/inputs.json; each guest code is independent of compile/process exit.
- `v1-protocol-cases.h` (input): [runtime/rust/tests/data/native_child_command_lifetime/v1/protocol/cases.h](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/protocol/cases.h). Exact retained v1 protocol/cases.h; each guest code is independent of compile/process exit.
- `v1-capture-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/receipt.json). Exact retained v1 capture/receipt.json; each guest code is independent of compile/process exit.
- `v2-protocol-probe.c` (input): [runtime/rust/tests/data/native_child_command_lifetime/v2/protocol/probe.c](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/protocol/probe.c). Exact retained v2 protocol/probe.c; each guest code is independent of compile/process exit.
- `v2-protocol-inputs.json` (input): [runtime/rust/tests/data/native_child_command_lifetime/v2/protocol/inputs.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/protocol/inputs.json). Exact retained v2 protocol/inputs.json; each guest code is independent of compile/process exit.
- `v2-protocol-cases.h` (input): [runtime/rust/tests/data/native_child_command_lifetime/v2/protocol/cases.h](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/protocol/cases.h). Exact retained v2 protocol/cases.h; each guest code is independent of compile/process exit.
- `v2-capture-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/receipt.json). Exact retained v2 capture/receipt.json; each guest code is independent of compile/process exit.
- `v1-tcl8.4-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.4.20/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.4.20/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.4-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.4.20/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.4.20/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.4-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.4.20/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.4.20/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.4-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.4.20/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.4.20/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.4-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.4.20/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.4.20/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.4-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.4.20/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.4.20/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.5-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.5.19/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.5.19/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.5-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.5.19/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.5.19/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.5-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.5.19/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.5.19/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.5-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.5.19/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.5.19/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.5-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.5.19/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.5.19/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.5-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.5.19/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.5.19/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.6-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.6.18/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.6.18/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.6-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.6.18/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.6.18/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.6-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.6.18/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.6.18/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.6-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.6.18/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.6.18/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.6-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.6.18/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.6.18/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.6-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.6.18/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.6.18/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl9.0-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.0.4/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.0.4/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl9.0-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.0.4/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.0.4/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl9.0-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.0.4/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.0.4/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl9.0-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.0.4/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.0.4/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl9.0-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.0.4/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.0.4/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl9.0-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.0.4/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.0.4/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl9.1-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.1.0/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.1.0/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl9.1-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.1.0/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.1.0/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl9.1-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.1.0/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.1.0/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl9.1-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.1.0/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.1.0/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl9.1-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.1.0/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.1.0/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl9.1-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.1.0/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.1.0/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-jim-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/jim/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/jim/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-jim-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/jim/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/jim/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-jim-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/jim/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/jim/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-jim-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/jim/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/jim/receipt.json). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-jim-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/jim/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/jim/stdout.tsv). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-jim-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/jim/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/jim/stderr). Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `tcl8.4-c-api-publication` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl8.4-basic-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl8.4-basic-source.json). JSON pointer `/windows/0/snippet`. Pinned source-only C API publication or deleted evaluator ready check; no extra execution is inferred from this excerpt.
- `tcl8.5-c-api-publication` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl8.5-basic-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl8.5-basic-source.json). JSON pointer `/windows/0/snippet`. Pinned source-only C API publication or deleted evaluator ready check; no extra execution is inferred from this excerpt.
- `tcl8.6-c-api-publication` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl8.6-basic-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl8.6-basic-source.json). JSON pointer `/windows/0/snippet`. Pinned source-only C API publication or deleted evaluator ready check; no extra execution is inferred from this excerpt.
- `tcl9.0-c-api-publication` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl9.0-basic-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl9.0-basic-source.json). JSON pointer `/windows/0/snippet`. Pinned source-only C API publication or deleted evaluator ready check; no extra execution is inferred from this excerpt.
- `tcl9.1-c-api-publication` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl9.1-basic-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl9.1-basic-source.json). JSON pointer `/windows/0/snippet`. Pinned source-only C API publication or deleted evaluator ready check; no extra execution is inferred from this excerpt.

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

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "runtime/rust/tests/data/native_child_command_lifetime/replay.py",
  "--verify-only"
]
```

Offline exact source/input/stream/receipt verification only. For new native launches use the retained original compile commands with fresh output directory and the exact source/header/static-library association. C8.4 return-options getter unavailable; Jim C public child API not tested; no BIGIP execution. Original v1 errors remain separate from v2 intermediate observations.
