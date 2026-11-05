# naming.dictionary.c85-original-update-write-error-object-lifetime

Kind: `native-observation`

## Problem statement

The defined lifetime of a dictionary result needs direct reached cleanup evidence. Public caller bytes alone cannot establish a safe storage-equivalence contract or identify the object ownership exercised by an exact compiled write-trace-error control.

## Question

At the exact original C8.5 dictionary-update write-error control, what live refcount, defined-scalar/same-cell checks and watched free-hook counter does the separately pinned instrumented current archive record?

## Conclusion

The separately pinned current C8.5.19 archive94313727 produces the original case9 public row. Its independent observer archive340cb367 records errors1, live refcount_before_release1, same_cell1 and defined_scalar1 immediately before the selected INST_DICT_UPDATE_END error-cleanup TclDecrRefCount, followed by final_free1 at the watched TclFreeObj entry. The watch is cleared at that entry without reading a freed header. Controls0/19 record errors0, unobserved sentinels-1 and final_free0. Baseline and observer case9 whole JSON stdout are byte-identical; all three observer case/result-code/result-hex fields exactly match their corresponding immutable C8.5 columns; this finite byte agreement does not establish safe object/storage identity or equivalence of another archive. All nine actual baseline/build/observer commands exit0. Build-object stderr retains654 bytes of compiler warnings; observer stderr retains the intentional99/102-byte counter rows. This is an observed live pre-release cell/header check plus a reached watched free hook in the independently instrumented current archive, not a private lifetime observation of the separately preserved archive99e0d524 capture. No freed-header dereference, allocation-reuse model, original fixture replacement, generic dict-update/dict-with lifetime, other provider, compiler/frame/admission or Rust pass follows. The current VM source comparator retains all329 captured outer process-code observations and compares326 complete public result windows plus three bounded captured C8.5/C8.6 update-error and C8.6 with-error diagnostic/read-status windows. Each bounded window is reported after its independently observed original update/with final free: even outer completion, diagnostics and caller read status are captured process observations, without defined storage, completion, read or effect guarantees. Complete other windows do not certify their lifetime. The separate C8.5/C8.6 update and C8.6 with VM host-storage controls check their own safe live headers and dictionary members under unchanged source, independently of native freed contents and private archived-executable identity. An executed software outcome belongs only to its exact source/image in the independent Rust validation ledger.

## Scope

The unchanged original C API probe and original counted case9 source run with Tcl_CreateInterp/Tcl_EvalObjEx and no Tcl_Init. The baseline uses current archive94313727. The scratch observer replaces only tclExecute.o/tclObj.o compiled with the original configured CC_SWITCHES; guest source and original public field reporting are unchanged. The added final probe line reports integer counters. Live pre-decrement checks retain refcount, original variable-pointer comparison and scalar-defined state; TclFreeObj records the watched header and clears its watch. Full original/instrumented sources and patches, actual archive/object/executable bytes, all commands/receipts and complete public stdout/warning/counter stderr are retained. Strict-pin225 input and its exact exec_command tool response retain a pre-compilation rejection. The tool response is not an independently captured launcher stdout/stderr/process receipt or native completion. The C8.5 pre-release counters and free hook remain the bounded original observation. Excluding the freed-value field does not turn remaining post-free fields into defined guarantees; the current comparator and separate own-host controls retain that distinction.

## Provider answers

### tcl8.4, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

No exact original lifetime-counter/provider process is executed for this question.

### tcl8.5

Status: `observed`. Channel: Original unchanged C API probe: Tcl_CreateInterp without Tcl_Init, original counted Tcl_EvalObjEx; independent live integer counters in separately built archive and final stderr only. Version: tcl8.5 8.5.19.

Original case9 observer: errors1/refcount_before_release1/same_cell1/defined_scalar1/final_free1. Controls0/19: errors0/refcount/same_cell/defined_scalar-1/final_free0. Baseline and observed original public row equality is independently checked and does not grant old-archive lifetime identity. All nine actual commands exit0; exact compiler warnings and expected observer stderr are preserved.

## Source inspection

tcl8.5 8.5.19, revision `Exact original/current source and separately instrumented scratch build; source-control revision unrecorded.`, `/workspace/.proofs/native-dict-write-error-lifetime226-request/original-source/tclExecute.c`, function `TclExecuteByteCode::INST_DICT_UPDATE_END`, lines 7253–7270. Retained reference `native_dict_write_error_lifetime226-request-original-source-tclExecute.c`.

```text
	    Tcl_IncrRefCount(dictPtr);
	    TclDecrRefCount(varPtr->value.objPtr);
	    varPtr->value.objPtr = dictPtr;
	} else {
	    DECACHE_STACK_INFO();
	    objResultPtr = TclPtrSetVar(interp, varPtr, NULL, NULL, NULL,
		    dictPtr, TCL_LEAVE_ERR_MSG, opnd);
	    CACHE_STACK_INFO();
	    if (objResultPtr == NULL) {
		if (allocdict) {
		    TclDecrRefCount(dictPtr);
		}
		result = TCL_ERROR;
		goto checkForCatch;
	    }
	}
	NEXT_INST_F(9, 1, 0);
    }

```

tcl8.5 8.5.19, revision `Exact original/current source and separately instrumented scratch build; source-control revision unrecorded.`, `/workspace/.proofs/native-dict-write-error-lifetime226-request/original-source/tclVar.c`, function `TclPtrSetVar`, lines 1927–1955. Retained reference `native_dict_write_error_lifetime226-request-original-source-tclVar.c`.

```text
	    }
	}
    } else if (newValuePtr != oldValuePtr) {
	/*
	 * In this case we are replacing the value, so we don't need to do
	 * more than swap the objects.
	 */

	varPtr->value.objPtr = newValuePtr;
	Tcl_IncrRefCount(newValuePtr);		/* Var is another ref. */
	if (oldValuePtr != NULL) {
	    TclDecrRefCount(oldValuePtr);	/* Discard old value. */
	}
    }

    /*
     * Invoke any write traces for the variable.
     */

    if ((varPtr->flags & VAR_TRACED_WRITE)
	    || (arrayPtr && (arrayPtr->flags & VAR_TRACED_WRITE))) {
	if (TCL_ERROR == TclObjCallVarTraces(iPtr, arrayPtr, varPtr, part1Ptr,
		part2Ptr, (flags & (TCL_GLOBAL_ONLY|TCL_NAMESPACE_ONLY))
		| TCL_TRACE_WRITES, (flags & TCL_LEAVE_ERR_MSG), index)) {
	    goto cleanup;
	}
    }

    /*

```

tcl8.5 8.5.19, revision `Exact original/current source and separately instrumented scratch build; source-control revision unrecorded.`, `/workspace/.proofs/native-dict-write-error-lifetime226-request/original-source/tclObj.c`, function `TclFreeObj`, lines 1432–1459. Retained reference `native_dict_write_error_lifetime226-request-original-source-tclObj.c`.

```text
}
#else /* TCL_MEM_DEBUG */

void
TclFreeObj(
    register Tcl_Obj *objPtr)	/* The object to be freed. */
{
    /* Invalidate the string rep first so we can use the bytes value
     * for our pointer chain, and signal an obj deletion (as opposed
     * to shimmering) with 'length == -1' */

    TclInvalidateStringRep(objPtr);
    objPtr->length = -1;

    if (!objPtr->typePtr || !objPtr->typePtr->freeIntRepProc) {
	/*
	 * objPtr can be freed safely, as it will not attempt to free any
	 * other objects: it will not cause recursive calls to this function.
	 */

	TCL_DTRACE_OBJ_FREE(objPtr);
	TclFreeObjStorage(objPtr);
	TclIncrObjsFreed();
    } else {
	/*
	 * This macro declares a variable, so must come here...
	 */

```

tcl8.5 8.5.19, revision `Exact original/current source and separately instrumented scratch build; source-control revision unrecorded.`, `/workspace/.proofs/native-dict-write-error-lifetime226-request/instrumented-source/tclExecute.c`, function `TclExecuteByteCode::INST_DICT_UPDATE_END observer`, lines 7267–7283. Retained reference `native_dict_write_error_lifetime226-request-instrumented-source-tclExecute.c`.

```text
		    dictPtr, TCL_LEAVE_ERR_MSG, opnd);
	    CACHE_STACK_INFO();
	    if (objResultPtr == NULL) {
		if (allocdict) {
                    tclLspDictErrorObserved++;
                    tclLspDictErrorRefcount = dictPtr->refCount;
                    tclLspDictErrorSameCell = (varPtr->value.objPtr == dictPtr);
                    tclLspDictErrorDefinedScalar =
                        TclIsVarScalar(varPtr) && !TclIsVarUndefined(varPtr);
                    tclLspDictReleaseWatch = dictPtr;
		    TclDecrRefCount(dictPtr);
		}
		result = TCL_ERROR;
		goto checkForCatch;
	    }
	}
	NEXT_INST_F(9, 1, 0);

```

tcl8.5 8.5.19, revision `Exact original/current source and separately instrumented scratch build; source-control revision unrecorded.`, `/workspace/.proofs/native-dict-write-error-lifetime226-request/instrumented-source/tclObj.c`, function `TclFreeObj observer`, lines 1440–1455. Retained reference `native_dict_write_error_lifetime226-request-instrumented-source-tclObj.c`.

```text
TclFreeObj(
    register Tcl_Obj *objPtr)	/* The object to be freed. */
{
    if (objPtr == tclLspDictReleaseWatch) {
        tclLspDictFinalFreeObserved++;
        tclLspDictReleaseWatch = NULL;
    }

    /* Invalidate the string rep first so we can use the bytes value
     * for our pointer chain, and signal an obj deletion (as opposed
     * to shimmering) with 'length == -1' */

    TclInvalidateStringRep(objPtr);
    objPtr->length = -1;

    if (!objPtr->typePtr || !objPtr->typePtr->freeIntRepProc) {

```

## Consumer bindings

- [rust/tcl-vm/src/cmd_dict.rs](../../../../rust/tcl-vm/src/cmd_dict.rs), `cmd_dict::native_rmw_fixture_tests::dictionary_body_writeback_compares_original_native_observation_windows` (linked): Retain329 unchanged captured outer process-code references and326 complete results plus three bounded post-free observations: C8.5/C8.6 update-error and C8.6 with-error. Every later outer/result/diagnostic/caller-read field remains captured output without defined storage/completion/read/effect or another archived executable private lifetime; the original seven signal controls remain separate.
- [rust/tcl-vm/src/cmd_dict.rs](../../../../rust/tcl-vm/src/cmd_dict.rs), `cmd_dict::native_rmw_fixture_tests::c85_dictionary_write_error_retains_defined_host_storage` (linked): Read unchanged baseline/observer bytes and reached counters, then separately assert the VM own safe live header/member policy under original source. Backend host storage is not native freed-content or private archived-header identity.

## Evidence

- [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226)
- [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-launcher225/launcher-tool-response.json](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-launcher225/launcher-tool-response.json)
- [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225)
- [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request)
