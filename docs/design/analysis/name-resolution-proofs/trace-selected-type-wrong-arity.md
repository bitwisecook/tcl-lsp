# naming.trace.selected-type-wrong-arity

Kind: `source-anchor`

## Problem statement

After a native trace type lookup succeeds, its original object may still contain an abbreviated or opaque string. Decoding that original string for a wrong-arity header can panic or show a different member than the native index-backed presenter.

## Question

Does a delegated trace wrong-arity error print the original type string or the successfully selected native index table member?

## Conclusion

All pinned C5 dispatchers select the trace type with Tcl_GetIndexFromObj before delegating. Delegated arity errors pass the first three original argv objects to Tcl_WrongNumArgs. Its index-type branch prints the selected table member (EXPAND_OF), rather than reading the original string. This closes only the selected type spelling in the usage header; command/ensemble prefix rewriting, original object conversion, type selection, error options and observer effects remain independently owned.

## Scope

Pinned C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 source inspection, selected trace type and wrong-arity reporting. No native or Rust launch, registration/callback capability, general opaque-string rendering, Jim or BIG-IP behavior.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `inspected`. Channel: Inspected native trace dispatcher and usage presenter; no guest input. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Successful trace type selection precedes delegated arity validation; the index-backed usage presenter emits the selected table member rather than the original string.

### jim, bigip

Status: `not-tested`. Channel: No source/input inspected for this question.

No matching provider source or native observation; no C trace law is borrowed.

## Source inspection

tcl8.4 8.4.20, revision `8.4.20`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdMZ.c`, function `Tcl_TraceObjCmd`, lines 3123–3128. Retained reference `tcl8.4-selected-type`.

```text
	    if (Tcl_GetIndexFromObj(interp, objv[2], traceTypeOptions,
			"option", 0, &typeIndex) != TCL_OK) {
		return TCL_ERROR;
	    }
	    return (traceSubCmds[typeIndex])(interp, optionIndex, objc, objv);
	}

```

tcl8.4 8.4.20, revision `8.4.20`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdMZ.c`, function `TclTraceExecutionObjCmd`, lines 3282–3285. Retained reference `tcl8.4-arity-header`.

```text
	    if (objc != 6) {
		Tcl_WrongNumArgs(interp, 3, objv, "name opList command");
		return TCL_ERROR;
	    }

```

tcl8.4 8.4.20, revision `8.4.20`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclIndexObj.c`, function `Tcl_WrongNumArgs`, lines 462–475. Retained reference `tcl8.4-indexed-presentation`.

```text
	 * If the object is an index type use the index table which allows
	 * for the correct error message even if the subcommand was
	 * abbreviated.  Otherwise, just use the string rep.
	 */
	
	if (objv[i]->typePtr == &tclIndexType) {
	    indexRep = (IndexRep *) objv[i]->internalRep.otherValuePtr;
	    Tcl_AppendStringsToObj(objPtr, EXPAND_OF(indexRep), (char *) NULL);
	} else {
	    Tcl_AppendStringsToObj(objPtr, Tcl_GetString(objv[i]),
		    (char *) NULL);
	}

	/*

```

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclTrace.c`, function `Tcl_TraceObjCmd`, lines 220–225. Retained reference `tcl8.5-selected-type`.

```text
	if (Tcl_GetIndexFromObj(interp, objv[2], traceTypeOptions, "option",
		0, &typeIndex) != TCL_OK) {
	    return TCL_ERROR;
	}
	return (traceSubCmds[typeIndex])(interp, optionIndex, objc, objv);
    }

```

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclTrace.c`, function `TraceExecutionObjCmd`, lines 408–411. Retained reference `tcl8.5-arity-header`.

```text
	if (objc != 6) {
	    Tcl_WrongNumArgs(interp, 3, objv, "name opList command");
	    return TCL_ERROR;
	}

```

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclIndexObj.c`, function `Tcl_WrongNumArgs`, lines 586–599. Retained reference `tcl8.5-indexed-presentation`.

```text
	 * If the object is an index type use the index table which allows for
	 * the correct error message even if the subcommand was abbreviated.
	 * Otherwise, just use the string rep.
	 */

	if (objv[i]->typePtr == &indexType) {
	    register IndexRep *indexRep = objv[i]->internalRep.twoPtrValue.ptr1;

	    Tcl_AppendStringsToObj(objPtr, EXPAND_OF(indexRep), NULL);
	} else if (objv[i]->typePtr == &tclEnsembleCmdType) {
	    register EnsembleCmdRep *ecrPtr =
		    objv[i]->internalRep.twoPtrValue.ptr1;

	    Tcl_AppendStringsToObj(objPtr, ecrPtr->fullSubcmdName, NULL);

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclTrace.c`, function `Tcl_TraceObjCmd`, lines 237–242. Retained reference `tcl8.6-selected-type`.

```text
	if (Tcl_GetIndexFromObj(interp, objv[2], traceTypeOptions, "option",
		0, &typeIndex) != TCL_OK) {
	    return TCL_ERROR;
	}
	return traceSubCmds[typeIndex](interp, optionIndex, objc, objv);
    }

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclTrace.c`, function `TraceExecutionObjCmd`, lines 427–430. Retained reference `tcl8.6-arity-header`.

```text
	if (objc != 6) {
	    Tcl_WrongNumArgs(interp, 3, objv, "name opList command");
	    return TCL_ERROR;
	}

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclIndexObj.c`, function `Tcl_WrongNumArgs`, lines 966–979. Retained reference `tcl8.6-indexed-presentation`.

```text
	 * If the object is an index type, use the index table which allows for
	 * the correct error message even if the subcommand was abbreviated.
	 * Otherwise, just use the string rep.
	 */

	if (objv[i]->typePtr == &indexType) {
	    IndexRep *indexRep = (IndexRep *)objv[i]->internalRep.twoPtrValue.ptr1;

	    Tcl_AppendStringsToObj(objPtr, EXPAND_OF(indexRep), (char *)NULL);
	} else {
	    /*
	     * Quote the argument if it contains spaces (Bug 942757).
	     */

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclTrace.c`, function `Tcl_TraceObjCmd`, lines 225–230. Retained reference `tcl9.0-selected-type`.

```text
	if (Tcl_GetIndexFromObj(interp, objv[2], traceTypeOptions, "option",
		0, &typeIndex) != TCL_OK) {
	    return TCL_ERROR;
	}
	return traceSubCmds[typeIndex](interp, optionIndex, objc, objv);
    }

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclTrace.c`, function `TraceExecutionObjCmd`, lines 303–306. Retained reference `tcl9.0-arity-header`.

```text
	if (objc != 6) {
	    Tcl_WrongNumArgs(interp, 3, objv, "name opList command");
	    return TCL_ERROR;
	}

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclIndexObj.c`, function `Tcl_WrongNumArgs`, lines 911–924. Retained reference `tcl9.0-indexed-presentation`.

```text
	 * If the object is an index type, use the index table which allows for
	 * the correct error message even if the subcommand was abbreviated.
	 * Otherwise, just use the string rep.
	 */
	const Tcl_ObjInternalRep *irPtr;

	if ((irPtr = TclFetchInternalRep(objv[i], &tclIndexType))) {
	    IndexRep *indexRep = (IndexRep *)irPtr->twoPtrValue.ptr1;

	    Tcl_AppendStringsToObj(objPtr, EXPAND_OF(indexRep), (char *)NULL);
	} else {
	    /*
	     * Quote the argument if it contains spaces (Bug 942757).
	     */

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclTrace.c`, function `Tcl_TraceObjCmd`, lines 230–235. Retained reference `tcl9.1-selected-type`.

```text
	if (Tcl_GetIndexFromObj(interp, objv[2], traceTypeOptions, "option",
		0, &typeIndex) != TCL_OK) {
	    return TCL_ERROR;
	}
	return traceSubCmds[typeIndex](interp, optionIndex, objc, objv);
    }

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclTrace.c`, function `TraceExecutionObjCmd`, lines 308–311. Retained reference `tcl9.1-arity-header`.

```text
	if (objc != 6) {
	    Tcl_WrongNumArgs(interp, 3, objv, "name opList command");
	    return TCL_ERROR;
	}

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclIndexObj.c`, function `Tcl_WrongNumArgs`, lines 913–926. Retained reference `tcl9.1-indexed-presentation`.

```text
	 * If the object is an index type, use the index table which allows for
	 * the correct error message even if the subcommand was abbreviated.
	 * Otherwise, just use the string rep.
	 */
	const Tcl_ObjInternalRep *irPtr;

	if ((irPtr = TclFetchInternalRep(objv[i], &tclIndexType))) {
	    IndexRep *indexRep = (IndexRep *)irPtr->twoPtrValue.ptr1;

	    Tcl_AppendStringsToObj(objPtr, EXPAND_OF(indexRep), (char *)NULL);
	} else {
	    /*
	     * Quote the argument if it contains spaces (Bug 942757).
	     */

```

## Consumer bindings

- [rust/tcl-cmd-core/src/trace.rs](../../../../rust/tcl-cmd-core/src/trace.rs), `TraceKind::canonical_name`: Report only the member of the already selected native type table.
- [rust/tcl-vm/src/cmd_trace.rs](../../../../rust/tcl-vm/src/cmd_trace.rs), `trace_add_remove`: Consume the selected type member for usage without decoding the opaque original type object.
- [rust/tcl-vm/src/cmd_trace.rs](../../../../rust/tcl-vm/src/cmd_trace.rs), `trace_info`: Consume the selected type member for usage without decoding the opaque original type object.
- [runtime/rust/src/cmd_trace.rs](../../../../runtime/rust/src/cmd_trace.rs), `cmd_trace_add_remove`: Use the same selected type table reporting owner independently of actual trace registration and callback effects.
- [runtime/rust/src/cmd_trace.rs](../../../../runtime/rust/src/cmd_trace.rs), `cmd_trace_info`: Use the same selected type table reporting owner independently of actual trace registration and callback effects.
- [rust/tcl-vm/src/cmd_trace/native_original_tests.rs](../../../../rust/tcl-vm/src/cmd_trace/native_original_tests.rs), `cmd_trace::native_original_tests::trace_type_wrong_arity_uses_selected_member_without_decoding_original_operand` (linked): The linked VM regression compares thirty retained direct raw-zero C code/result controls for canonical selected-type wrong-arity reporting. Source inspection explains the selected member; it supplies no execution, option, callback or registration conclusion.

## Evidence

- [rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.4-source.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.4-source.json)
- [rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.5-source.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.5-source.json)
- [rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.6-source.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.6-source.json)
- [rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl9.0-source.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl9.0-source.json)
- [rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl9.1-source.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl9.1-source.json)
