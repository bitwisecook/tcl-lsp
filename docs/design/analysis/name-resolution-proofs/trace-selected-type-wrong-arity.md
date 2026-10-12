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

### tcl8.4

Status: `inspected`. Version: 8.4.20. Build: Source inspection only; no executed build association.. Channel: Inspected native trace dispatcher and usage presenter; no guest input.. Dialect: Tcl.

Successful trace type selection precedes delegated arity validation; the index-backed usage presenter emits the selected table member rather than the original string.

### tcl8.5

Status: `inspected`. Version: 8.5.19. Build: Source inspection only; no executed build association.. Channel: Inspected native trace dispatcher and usage presenter; no guest input.. Dialect: Tcl.

Successful trace type selection precedes delegated arity validation; the index-backed usage presenter emits the selected table member rather than the original string.

### tcl8.6

Status: `inspected`. Version: 8.6.18. Build: Source inspection only; no executed build association.. Channel: Inspected native trace dispatcher and usage presenter; no guest input.. Dialect: Tcl.

Successful trace type selection precedes delegated arity validation; the index-backed usage presenter emits the selected table member rather than the original string.

### tcl9.0

Status: `inspected`. Version: 9.0.4. Build: Source inspection only; no executed build association.. Channel: Inspected native trace dispatcher and usage presenter; no guest input.. Dialect: Tcl.

Successful trace type selection precedes delegated arity validation; the index-backed usage presenter emits the selected table member rather than the original string.

### tcl9.1

Status: `inspected`. Version: 9.1.0. Build: Source inspection only; no executed build association.. Channel: Inspected native trace dispatcher and usage presenter; no guest input.. Dialect: Tcl.

Successful trace type selection precedes delegated arity validation; the index-backed usage presenter emits the selected table member rather than the original string.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: No source/input inspected for this question.. Dialect: Jim Tcl.

No matching provider source or native observation; no C trace law is borrowed.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: No source/input inspected for this question.. Dialect: F5 iRules.

No matching provider source or native observation; no C trace law is borrowed.

## Exact evidence

- `tcl8.4-selected-type` (source-anchor): [rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.4-source.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.4-source.json). SHA-256 `c0c2bfc4718cacd212f4cdcba20bb70ca430e79a5298caffc45c5ea94180e9d9`. JSON pointer `/windows/0/snippet`. Exact selected trace type, delegated arity prefix, and native index-backed usage presentation excerpts; no guest or Rust launch.
- `tcl8.4-arity-header` (source-anchor): [rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.4-source.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.4-source.json). SHA-256 `c0c2bfc4718cacd212f4cdcba20bb70ca430e79a5298caffc45c5ea94180e9d9`. JSON pointer `/windows/1/snippet`. Exact selected trace type, delegated arity prefix, and native index-backed usage presentation excerpts; no guest or Rust launch.
- `tcl8.4-indexed-presentation` (source-anchor): [rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.4-source.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.4-source.json). SHA-256 `c0c2bfc4718cacd212f4cdcba20bb70ca430e79a5298caffc45c5ea94180e9d9`. JSON pointer `/windows/2/snippet`. Exact selected trace type, delegated arity prefix, and native index-backed usage presentation excerpts; no guest or Rust launch.
- `tcl8.5-selected-type` (source-anchor): [rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.5-source.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.5-source.json). SHA-256 `0b184136bcb01029b24ef1bcacaef3cd9f28ee67c4a44180e668bf9798fea608`. JSON pointer `/windows/0/snippet`. Exact selected trace type, delegated arity prefix, and native index-backed usage presentation excerpts; no guest or Rust launch.
- `tcl8.5-arity-header` (source-anchor): [rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.5-source.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.5-source.json). SHA-256 `0b184136bcb01029b24ef1bcacaef3cd9f28ee67c4a44180e668bf9798fea608`. JSON pointer `/windows/1/snippet`. Exact selected trace type, delegated arity prefix, and native index-backed usage presentation excerpts; no guest or Rust launch.
- `tcl8.5-indexed-presentation` (source-anchor): [rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.5-source.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.5-source.json). SHA-256 `0b184136bcb01029b24ef1bcacaef3cd9f28ee67c4a44180e668bf9798fea608`. JSON pointer `/windows/2/snippet`. Exact selected trace type, delegated arity prefix, and native index-backed usage presentation excerpts; no guest or Rust launch.
- `tcl8.6-selected-type` (source-anchor): [rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.6-source.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.6-source.json). SHA-256 `9c782eb3184692a0ccc931ba2b8750fdc936822fb382b129152ebfc83d890e45`. JSON pointer `/windows/0/snippet`. Exact selected trace type, delegated arity prefix, and native index-backed usage presentation excerpts; no guest or Rust launch.
- `tcl8.6-arity-header` (source-anchor): [rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.6-source.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.6-source.json). SHA-256 `9c782eb3184692a0ccc931ba2b8750fdc936822fb382b129152ebfc83d890e45`. JSON pointer `/windows/1/snippet`. Exact selected trace type, delegated arity prefix, and native index-backed usage presentation excerpts; no guest or Rust launch.
- `tcl8.6-indexed-presentation` (source-anchor): [rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.6-source.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl8.6-source.json). SHA-256 `9c782eb3184692a0ccc931ba2b8750fdc936822fb382b129152ebfc83d890e45`. JSON pointer `/windows/2/snippet`. Exact selected trace type, delegated arity prefix, and native index-backed usage presentation excerpts; no guest or Rust launch.
- `tcl9.0-selected-type` (source-anchor): [rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl9.0-source.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl9.0-source.json). SHA-256 `8942fd5fe68f45c92b5a1dcd0ce18633b71356ee9e20ee7abeafe1ba06f1b5f1`. JSON pointer `/windows/0/snippet`. Exact selected trace type, delegated arity prefix, and native index-backed usage presentation excerpts; no guest or Rust launch.
- `tcl9.0-arity-header` (source-anchor): [rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl9.0-source.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl9.0-source.json). SHA-256 `8942fd5fe68f45c92b5a1dcd0ce18633b71356ee9e20ee7abeafe1ba06f1b5f1`. JSON pointer `/windows/1/snippet`. Exact selected trace type, delegated arity prefix, and native index-backed usage presentation excerpts; no guest or Rust launch.
- `tcl9.0-indexed-presentation` (source-anchor): [rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl9.0-source.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl9.0-source.json). SHA-256 `8942fd5fe68f45c92b5a1dcd0ce18633b71356ee9e20ee7abeafe1ba06f1b5f1`. JSON pointer `/windows/2/snippet`. Exact selected trace type, delegated arity prefix, and native index-backed usage presentation excerpts; no guest or Rust launch.
- `tcl9.1-selected-type` (source-anchor): [rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl9.1-source.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl9.1-source.json). SHA-256 `acf07f8e16de709453fb031335296e639402a4c921fa2c9e9b3b6cdf31dbaef8`. JSON pointer `/windows/0/snippet`. Exact selected trace type, delegated arity prefix, and native index-backed usage presentation excerpts; no guest or Rust launch.
- `tcl9.1-arity-header` (source-anchor): [rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl9.1-source.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl9.1-source.json). SHA-256 `acf07f8e16de709453fb031335296e639402a4c921fa2c9e9b3b6cdf31dbaef8`. JSON pointer `/windows/1/snippet`. Exact selected trace type, delegated arity prefix, and native index-backed usage presentation excerpts; no guest or Rust launch.
- `tcl9.1-indexed-presentation` (source-anchor): [rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl9.1-source.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type_source/tcl9.1-source.json). SHA-256 `acf07f8e16de709453fb031335296e639402a4c921fa2c9e9b3b6cdf31dbaef8`. JSON pointer `/windows/2/snippet`. Exact selected trace type, delegated arity prefix, and native index-backed usage presentation excerpts; no guest or Rust launch.

## Source inspection

tcl8.4 8.4.20, revision `8.4.20`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdMZ.c`, function `Tcl_TraceObjCmd`, lines 3123–3128. Full-source SHA-256 `b374cf4353b86235b189d432239a1eedf8e854eab266b701e9d9625b969d27dc`; snippet SHA-256 `066480d2088ac40ecb85d6c035b095909a25cc38a1717c311a46004f07ab54c1`; retained evidence `tcl8.4-selected-type`.

```text
	    if (Tcl_GetIndexFromObj(interp, objv[2], traceTypeOptions,
			"option", 0, &typeIndex) != TCL_OK) {
		return TCL_ERROR;
	    }
	    return (traceSubCmds[typeIndex])(interp, optionIndex, objc, objv);
	}

```

tcl8.4 8.4.20, revision `8.4.20`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdMZ.c`, function `TclTraceExecutionObjCmd`, lines 3282–3285. Full-source SHA-256 `b374cf4353b86235b189d432239a1eedf8e854eab266b701e9d9625b969d27dc`; snippet SHA-256 `e522793296312bf3b796e536a635889e6cac31cde9b56cc71e14dde65eb24a46`; retained evidence `tcl8.4-arity-header`.

```text
	    if (objc != 6) {
		Tcl_WrongNumArgs(interp, 3, objv, "name opList command");
		return TCL_ERROR;
	    }

```

tcl8.4 8.4.20, revision `8.4.20`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclIndexObj.c`, function `Tcl_WrongNumArgs`, lines 462–475. Full-source SHA-256 `babe18e9f596d54d49f16c17b0576723d0d6c580ffb4356e1fca9b26ad6576da`; snippet SHA-256 `8667d4237a51693450675d71c16f0072d214c20b03dc86d15b91f9b20c5204c4`; retained evidence `tcl8.4-indexed-presentation`.

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

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclTrace.c`, function `Tcl_TraceObjCmd`, lines 220–225. Full-source SHA-256 `d974ce7d57afd060edbc9a858975e17c9577c7905f5dfaf6df477fa409ef89e6`; snippet SHA-256 `7ed290c1f0de6ba66796bd735c46cda7bb70f81b9a6a905794b6e25b6cf3083f`; retained evidence `tcl8.5-selected-type`.

```text
	if (Tcl_GetIndexFromObj(interp, objv[2], traceTypeOptions, "option",
		0, &typeIndex) != TCL_OK) {
	    return TCL_ERROR;
	}
	return (traceSubCmds[typeIndex])(interp, optionIndex, objc, objv);
    }

```

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclTrace.c`, function `TraceExecutionObjCmd`, lines 408–411. Full-source SHA-256 `d974ce7d57afd060edbc9a858975e17c9577c7905f5dfaf6df477fa409ef89e6`; snippet SHA-256 `bc58ffec8c5985dd00bb4d47d119e864c0765c543e5308d120c28b0095f2f01f`; retained evidence `tcl8.5-arity-header`.

```text
	if (objc != 6) {
	    Tcl_WrongNumArgs(interp, 3, objv, "name opList command");
	    return TCL_ERROR;
	}

```

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclIndexObj.c`, function `Tcl_WrongNumArgs`, lines 586–599. Full-source SHA-256 `5ef48cf4d90044d6513c8b452ca7e9070cc3a14e42e83186f8be2d42e90129aa`; snippet SHA-256 `0ea86f0500ccda739978874fb618860624b51428a045c4800287f762a8eb9063`; retained evidence `tcl8.5-indexed-presentation`.

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

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclTrace.c`, function `Tcl_TraceObjCmd`, lines 237–242. Full-source SHA-256 `8034d0a6da9f529bebeedfa4ab8af88f944bf858ad8c498719a972ac81d503a9`; snippet SHA-256 `4b5e55f82fa1223a7071524983d210e4a270e9df663f072a883a797fe4b8ff5a`; retained evidence `tcl8.6-selected-type`.

```text
	if (Tcl_GetIndexFromObj(interp, objv[2], traceTypeOptions, "option",
		0, &typeIndex) != TCL_OK) {
	    return TCL_ERROR;
	}
	return traceSubCmds[typeIndex](interp, optionIndex, objc, objv);
    }

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclTrace.c`, function `TraceExecutionObjCmd`, lines 427–430. Full-source SHA-256 `8034d0a6da9f529bebeedfa4ab8af88f944bf858ad8c498719a972ac81d503a9`; snippet SHA-256 `bc58ffec8c5985dd00bb4d47d119e864c0765c543e5308d120c28b0095f2f01f`; retained evidence `tcl8.6-arity-header`.

```text
	if (objc != 6) {
	    Tcl_WrongNumArgs(interp, 3, objv, "name opList command");
	    return TCL_ERROR;
	}

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclIndexObj.c`, function `Tcl_WrongNumArgs`, lines 966–979. Full-source SHA-256 `550096cdb69b26ddbcf459c07273db48ad8ed4f5cccd68133559483c4af67c17`; snippet SHA-256 `9a2d638e9cb4dd889fc7cdefb596ee790c09b24d19445ebeb4820b04da235cd6`; retained evidence `tcl8.6-indexed-presentation`.

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

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclTrace.c`, function `Tcl_TraceObjCmd`, lines 225–230. Full-source SHA-256 `b091fe603d801002a17d6d4c18de08f43e7aa7851abc64849ab2b1f7aac52904`; snippet SHA-256 `4b5e55f82fa1223a7071524983d210e4a270e9df663f072a883a797fe4b8ff5a`; retained evidence `tcl9.0-selected-type`.

```text
	if (Tcl_GetIndexFromObj(interp, objv[2], traceTypeOptions, "option",
		0, &typeIndex) != TCL_OK) {
	    return TCL_ERROR;
	}
	return traceSubCmds[typeIndex](interp, optionIndex, objc, objv);
    }

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclTrace.c`, function `TraceExecutionObjCmd`, lines 303–306. Full-source SHA-256 `b091fe603d801002a17d6d4c18de08f43e7aa7851abc64849ab2b1f7aac52904`; snippet SHA-256 `bc58ffec8c5985dd00bb4d47d119e864c0765c543e5308d120c28b0095f2f01f`; retained evidence `tcl9.0-arity-header`.

```text
	if (objc != 6) {
	    Tcl_WrongNumArgs(interp, 3, objv, "name opList command");
	    return TCL_ERROR;
	}

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclIndexObj.c`, function `Tcl_WrongNumArgs`, lines 911–924. Full-source SHA-256 `5ae0b7f7da756611674d4f80dd04a0646e64674fb3f829805ebebd73866d552a`; snippet SHA-256 `8d13ccdd2741e93cc49e66cdf1e2ec15342acc4c4f48277d7715e0412a26afb4`; retained evidence `tcl9.0-indexed-presentation`.

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

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclTrace.c`, function `Tcl_TraceObjCmd`, lines 230–235. Full-source SHA-256 `b39de1869c493f1405355c9f92537bdd5d9839a42a10da78b18a725309d71847`; snippet SHA-256 `4b5e55f82fa1223a7071524983d210e4a270e9df663f072a883a797fe4b8ff5a`; retained evidence `tcl9.1-selected-type`.

```text
	if (Tcl_GetIndexFromObj(interp, objv[2], traceTypeOptions, "option",
		0, &typeIndex) != TCL_OK) {
	    return TCL_ERROR;
	}
	return traceSubCmds[typeIndex](interp, optionIndex, objc, objv);
    }

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclTrace.c`, function `TraceExecutionObjCmd`, lines 308–311. Full-source SHA-256 `b39de1869c493f1405355c9f92537bdd5d9839a42a10da78b18a725309d71847`; snippet SHA-256 `bc58ffec8c5985dd00bb4d47d119e864c0765c543e5308d120c28b0095f2f01f`; retained evidence `tcl9.1-arity-header`.

```text
	if (objc != 6) {
	    Tcl_WrongNumArgs(interp, 3, objv, "name opList command");
	    return TCL_ERROR;
	}

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclIndexObj.c`, function `Tcl_WrongNumArgs`, lines 913–926. Full-source SHA-256 `fd9dfe1a261db55472f88ff9c1a7be53f9c0aee1ea597a6f96ace48b666d8487`; snippet SHA-256 `8d13ccdd2741e93cc49e66cdf1e2ec15342acc4c4f48277d7715e0412a26afb4`; retained evidence `tcl9.1-indexed-presentation`.

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

A named test is a coverage binding, not a claim that it executed.

## Replay

Verify exact retained source/full-file/snippet hashes and LF coordinates. This is source inspection; no executable replay or native/Rust result is asserted.
