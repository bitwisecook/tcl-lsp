# naming.dictionary.original-update-write-error-object-lifetime

Kind: `native-observation`

## Problem statement

Captured caller bytes cannot establish the defined lifetime of a dictionary after original compiled write-error cleanup. A bounded observation needs live pre-release selection checks and reached final-free evidence, while later signal termination and post-free reporting remain distinct outcomes.

## Question

For the unchanged original compiled-update write-error control on the separately pinned current C8.6/C9.0/C9.1 builds, does live cleanup select a defined scalar at refcount1 and reach its watched free entry, and what is reported before source-object release versus later process termination?

## Conclusion

The separate current C8.6.18 observer records errors=1, refcount_before_release=1, same_cell=1 and defined_scalar=1 immediately before the allocated compiled-update dictionary is decremented on the original write-error cleanup path; the watched TclFreeObj entry records final_free=1. Its unchanged case9 baseline and observer stdout are byte-identical and the processes exit0. Those stdout fields, including outer completion, diagnostics and caller read status, are observed after this final free and do not establish defined later storage or a general completion/read guarantee. Request398 final reporting occurs after source-object release and interpreter teardown: its C9.0.4/C9.1.0 case9 and independent case19 processes terminate by signal4 before that reporter. Request399 reports the same integer counters before those later releases, using exactly the398 observer archives: all three case9 rows report 1/1/1/1/1 at that boundary; C9.0/C9.1 later terminate by signal4. Plain case0 and independent with-write-error case19 counters are 0/-1/-1/-1/0; case19 still signals on C9.0/C9.1 and supplies no watched compiled-update lifetime conclusion. The51 exact closed commands comprise21 build/compile steps and30 process windows (27 case processes plus3 independent version queries);39 exit0 and12 terminate by signal4. No successful aggregate provider result, safe freed-content reproduction, other archive private identity, generic dict-update/dict-with lifetime, Native frame/compiler/admission, or Rust assertion pass follows. Current VM software coverage reads the exact398 C8.6 baseline/observer stdout and integer stderr, then checks its own retained dictionary header/member policy under the unchanged original source. That host policy is independent of native freed contents. The broader unchanged329-source comparator retains326 complete results plus three captured post-free C8.5/C8.6 update-error and C8.6 with-error diagnostic/read-status windows; no later outer/result/read field becomes a defined native storage/completion/effect guarantee. Software execution outcomes belong only to their exact pinned Rust source/image in the independent validation ledger.

## Scope

Two independent requests preserve the exact original guest SHA092c3aa2 and C API probe SHAe9545172 from the recorded compiled-update case9, plain0 and with-write-error19 controls. Tcl_CreateInterp and counted Tcl_EvalObjEx run without Tcl_Init; this is not a full-distribution initialisation test. Request398 copies the original current archives, separately compiles only instrumented tclExecute.o/tclObj.o with the configured source build, and retains baseline, observer and version executables. Live counters read the still-live refcount, compare the selected variable cell and check scalar-defined state before the unchanged decrement. The watched free-entry hook increments an integer and clears the watch before ordinary freeing; it does not dereference a freed header or alter Tcl ownership. Request399 only adds an earlier integer report before source-object release/teardown and reuses the exact398 observer archives. Complete source copies/patches, required SDK inputs, original/observer archives and objects, probes/executables, version rows, receipts and raw warning/counter/allocation-error stdout/stderr are retained separately. Later process output remains captured output, not a defined storage, completion, read, private identity or effect contract. C8.4, C8.5, Jim and BIG-IP are not tested by these requests; the separate existing C8.5 lifetime record is an independent observation.

## Provider answers

### tcl8.4, tcl8.5, jim, bigip

Status: `not-tested`.

No process or lifetime counter observation for this provider is executed by request398 or399.

### tcl8.6

Status: `observed`. Channel: Unchanged original counted C API source through Tcl_CreateInterp/Tcl_EvalObjEx without Tcl_Init; separate live integer observer counters and complete public streams. Version: tcl8.6 8.6.18.

Both398 and399 case9 processes exit0; the live pre-release/free counters are1/1/1/1/1 and plain0/with19 counters0/-1/-1/-1/0. Exact398 baseline/observer case9 whole stdout equality is observed after final free; it is not a defined later byte/storage/completion guarantee.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Unchanged original counted C API source through Tcl_CreateInterp/Tcl_EvalObjEx without Tcl_Init; separate live integer observer counters and complete public streams. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

The398 case9/with19 baseline and observer processes terminate by signal4 before the final reporter; no missing counter is inferred. The399 earlier case9 report records1/1/1/1/1, then the process terminates by signal4. Plain0 exits0 with0/-1/-1/-1/0; independent with19 reports0/-1/-1/-1/0 and later signals. These limited counters and raw outputs do not certify successful completion or defined post-free storage.

## Source inspection

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/8.6.18/original-tclExecute.c`, function `TclExecuteByteCode::INST_DICT_UPDATE_END original error cleanup`, lines 7782–7796. Retained reference `native_dict_update_write_error_lifetime398-request-8.6.18-original-tclExecute.c`.

```text
	    DECACHE_STACK_INFO();
	    objResultPtr = TclPtrSetVarIdx(interp, varPtr, NULL, NULL, NULL,
		    dictPtr, TCL_LEAVE_ERR_MSG, opnd);
	    CACHE_STACK_INFO();
	    if (objResultPtr == NULL) {
		if (allocdict) {
		    TclDecrRefCount(dictPtr);
		}
		TRACE_ERROR(interp);
		goto gotError;
	    }
	}
	TRACE_APPEND(("written back\n"));
	NEXT_INST_F(9, 1, 0);

```

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/8.6.18/observed-tclExecute.c`, function `TclExecuteByteCode::INST_DICT_UPDATE_END live pre-release observer`, lines 7791–7810. Retained reference `native_dict_update_write_error_lifetime398-request-8.6.18-observed-tclExecute.c`.

```text
	    objResultPtr = TclPtrSetVarIdx(interp, varPtr, NULL, NULL, NULL,
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
		TRACE_ERROR(interp);
		goto gotError;
	    }
	}
	TRACE_APPEND(("written back\n"));
	NEXT_INST_F(9, 1, 0);

```

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/8.6.18/observed-tclObj.c`, function `TclFreeObj watched entry (conditional debug/non-debug branches)`, lines 1295–1309. Retained reference `native_dict_update_write_error_lifetime398-request-8.6.18-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)	/* The object to be freed. */
{
    if (objPtr == tclLspDictReleaseWatch) {
        tclLspDictFinalFreeObserved++;
        tclLspDictReleaseWatch = NULL;
    }
    const Tcl_ObjType *typePtr = objPtr->typePtr;

    /*
     * This macro declares a variable, so must come here...
     */

    ObjInitDeletionContext(context);

```

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/8.6.18/observed-tclObj.c`, function `TclFreeObj watched entry (conditional debug/non-debug branches)`, lines 1424–1438. Retained reference `native_dict_update_write_error_lifetime398-request-8.6.18-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)	/* The object to be freed. */
{
    if (objPtr == tclLspDictReleaseWatch) {
        tclLspDictFinalFreeObserved++;
        tclLspDictReleaseWatch = NULL;
    }
    /*
     * Invalidate the string rep first so we can use the bytes value for our
     * pointer chain, and signal an obj deletion (as opposed to shimmering)
     * with 'length == -1'.
     */

    TclInvalidateStringRep(objPtr);

```

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/8.6.18/original-tclVar.c`, function `TclPtrSetVarIdx original replacement and write-trace failure`, lines 1980–2007. Retained reference `native_dict_update_write_error_lifetime398-request-8.6.18-original-tclVar.c`.

```text
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
     * Return the variable's value unless the variable was changed in some
     * gross way by a trace (e.g. it was unset and then recreated as an
     * array).

```

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/observed-probe.c`, function `Independent final counter report after source-object release and interpreter teardown`, lines 185–186. Retained reference `native_dict_update_write_error_lifetime398-request-observed-probe.c`.

```text
fprintf(stderr, "DICT_RELEASE_OBSERVER|errors=%d|refcount_before_release=%d|same_cell=%d|defined_scalar=%d|final_free=%d\n", tclLspDictErrorObserved, tclLspDictErrorRefcount, tclLspDictErrorSameCell, tclLspDictErrorDefinedScalar, tclLspDictFinalFreeObserved);
return 0;}

```

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime399/observed-probe.c`, function `Independent body-return integer report before source-object release and interpreter teardown`, lines 127–127. Retained reference `native_dict_update_write_error_lifetime399-request-observed-probe.c`.

```text
static void probe_bytes(const char*id,const char*src,int source_len){caseid=id;pathid="fixed-grammar";active=create();Obj*o=NEW(active,src,source_len);INC(o);int code=SCRIPT(active,o);report("native-result",code);fprintf(stderr, "DICT_RELEASE_BODY_RETURN|errors=%d|refcount_before_release=%d|same_cell=%d|defined_scalar=%d|final_free=%d\n", tclLspDictErrorObserved, tclLspDictErrorRefcount, tclLspDictErrorSameCell, tclLspDictErrorDefinedScalar, tclLspDictFinalFreeObserved);DEC(active,o);DESTROY(active);}

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/9.0.4/original-tclExecute.c`, function `TclExecuteByteCode::INST_DICT_UPDATE_END original error cleanup`, lines 7295–7309. Retained reference `native_dict_update_write_error_lifetime398-request-9.0.4-original-tclExecute.c`.

```text
	    DECACHE_STACK_INFO();
	    objResultPtr = TclPtrSetVarIdx(interp, varPtr, NULL, NULL, NULL,
		    dictPtr, TCL_LEAVE_ERR_MSG, opnd);
	    CACHE_STACK_INFO();
	    if (objResultPtr == NULL) {
		if (allocdict) {
		    TclDecrRefCount(dictPtr);
		}
		TRACE_ERROR(interp);
		goto gotError;
	    }
	}
	TRACE_APPEND(("written back\n"));
	NEXT_INST_F(9, 1, 0);

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/9.0.4/observed-tclExecute.c`, function `TclExecuteByteCode::INST_DICT_UPDATE_END live pre-release observer`, lines 7304–7323. Retained reference `native_dict_update_write_error_lifetime398-request-9.0.4-observed-tclExecute.c`.

```text
	    objResultPtr = TclPtrSetVarIdx(interp, varPtr, NULL, NULL, NULL,
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
		TRACE_ERROR(interp);
		goto gotError;
	    }
	}
	TRACE_APPEND(("written back\n"));
	NEXT_INST_F(9, 1, 0);

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/9.0.4/observed-tclObj.c`, function `TclFreeObj watched entry (conditional debug/non-debug branches)`, lines 1271–1285. Retained reference `native_dict_update_write_error_lifetime398-request-9.0.4-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)	/* The object to be freed. */
{
    if (objPtr == tclLspDictReleaseWatch) {
        tclLspDictFinalFreeObserved++;
        tclLspDictReleaseWatch = NULL;
    }
    const Tcl_ObjType *typePtr = objPtr->typePtr;

    /*
     * This macro declares a variable, so must come here...
     */

    ObjInitDeletionContext(context);

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/9.0.4/observed-tclObj.c`, function `TclFreeObj watched entry (conditional debug/non-debug branches)`, lines 1400–1414. Retained reference `native_dict_update_write_error_lifetime398-request-9.0.4-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)	/* The object to be freed. */
{
    if (objPtr == tclLspDictReleaseWatch) {
        tclLspDictFinalFreeObserved++;
        tclLspDictReleaseWatch = NULL;
    }
    /*
     * Invalidate the string rep first so we can use the bytes value for our
     * pointer chain, and signal an obj deletion (as opposed to shimmering)
     * with 'length == -1'.
     */

    TclInvalidateStringRep(objPtr);

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/9.0.4/original-tclVar.c`, function `TclPtrSetVarIdx original replacement and write-trace failure`, lines 2025–2052. Retained reference `native_dict_update_write_error_lifetime398-request-9.0.4-original-tclVar.c`.

```text
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
     * Return the variable's value unless the variable was changed in some
     * gross way by a trace (e.g. it was unset and then recreated as an
     * array).

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/observed-probe.c`, function `Independent final counter report after source-object release and interpreter teardown`, lines 185–186. Retained reference `native_dict_update_write_error_lifetime398-request-observed-probe.c`.

```text
fprintf(stderr, "DICT_RELEASE_OBSERVER|errors=%d|refcount_before_release=%d|same_cell=%d|defined_scalar=%d|final_free=%d\n", tclLspDictErrorObserved, tclLspDictErrorRefcount, tclLspDictErrorSameCell, tclLspDictErrorDefinedScalar, tclLspDictFinalFreeObserved);
return 0;}

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime399/observed-probe.c`, function `Independent body-return integer report before source-object release and interpreter teardown`, lines 127–127. Retained reference `native_dict_update_write_error_lifetime399-request-observed-probe.c`.

```text
static void probe_bytes(const char*id,const char*src,int source_len){caseid=id;pathid="fixed-grammar";active=create();Obj*o=NEW(active,src,source_len);INC(o);int code=SCRIPT(active,o);report("native-result",code);fprintf(stderr, "DICT_RELEASE_BODY_RETURN|errors=%d|refcount_before_release=%d|same_cell=%d|defined_scalar=%d|final_free=%d\n", tclLspDictErrorObserved, tclLspDictErrorRefcount, tclLspDictErrorSameCell, tclLspDictErrorDefinedScalar, tclLspDictFinalFreeObserved);DEC(active,o);DESTROY(active);}

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/9.1.0/original-tclExecute.c`, function `TclExecuteByteCode::INST_DICT_UPDATE_END original error cleanup`, lines 7684–7698. Retained reference `native_dict_update_write_error_lifetime398-request-9.1.0-original-tclExecute.c`.

```text
	    DECACHE_STACK_INFO();
	    objResultPtr = TclPtrSetVarIdx(interp, varPtr, NULL, NULL, NULL,
		    dictPtr, TCL_LEAVE_ERR_MSG, varIdx);
	    CACHE_STACK_INFO();
	    if (objResultPtr == NULL) {
		if (allocateDict) {
		    TclDecrRefCount(dictPtr);
		}
		TRACE_ERROR(interp);
		goto gotError;
	    }
	}
	TRACE_APPEND("written back\n");
	NEXT_INST_F0(9, 1);

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/9.1.0/observed-tclExecute.c`, function `TclExecuteByteCode::INST_DICT_UPDATE_END live pre-release observer`, lines 7693–7712. Retained reference `native_dict_update_write_error_lifetime398-request-9.1.0-observed-tclExecute.c`.

```text
	    objResultPtr = TclPtrSetVarIdx(interp, varPtr, NULL, NULL, NULL,
		    dictPtr, TCL_LEAVE_ERR_MSG, varIdx);
	    CACHE_STACK_INFO();
	    if (objResultPtr == NULL) {
		if (allocateDict) {
                    tclLspDictErrorObserved++;
                    tclLspDictErrorRefcount = dictPtr->refCount;
                    tclLspDictErrorSameCell = (varPtr->value.objPtr == dictPtr);
                    tclLspDictErrorDefinedScalar =
                        TclIsVarScalar(varPtr) && !TclIsVarUndefined(varPtr);
                    tclLspDictReleaseWatch = dictPtr;
		    TclDecrRefCount(dictPtr);
		}
		TRACE_ERROR(interp);
		goto gotError;
	    }
	}
	TRACE_APPEND("written back\n");
	NEXT_INST_F0(9, 1);

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/9.1.0/observed-tclObj.c`, function `TclFreeObj watched entry (conditional debug/non-debug branches)`, lines 1274–1288. Retained reference `native_dict_update_write_error_lifetime398-request-9.1.0-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)		/* The object to be freed. */
{
    if (objPtr == tclLspDictReleaseWatch) {
        tclLspDictFinalFreeObserved++;
        tclLspDictReleaseWatch = NULL;
    }
    const Tcl_ObjType *typePtr = objPtr->typePtr;

    /*
     * This macro declares a variable, so must come here...
     */

    ObjInitDeletionContext(context);

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/9.1.0/observed-tclObj.c`, function `TclFreeObj watched entry (conditional debug/non-debug branches)`, lines 1403–1417. Retained reference `native_dict_update_write_error_lifetime398-request-9.1.0-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)		/* The object to be freed. */
{
    if (objPtr == tclLspDictReleaseWatch) {
        tclLspDictFinalFreeObserved++;
        tclLspDictReleaseWatch = NULL;
    }
    /*
     * Invalidate the string rep first so we can use the bytes value for our
     * pointer chain, and signal an obj deletion (as opposed to shimmering)
     * with 'length == -1'.
     */

    TclInvalidateStringRep(objPtr);

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/9.1.0/original-tclVar.c`, function `TclPtrSetVarIdx original replacement and write-trace failure`, lines 2079–2106. Retained reference `native_dict_update_write_error_lifetime398-request-9.1.0-original-tclVar.c`.

```text
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
     * Return the variable's value unless the variable was changed in some
     * gross way by a trace (e.g. it was unset and then recreated as an
     * array).

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/observed-probe.c`, function `Independent final counter report after source-object release and interpreter teardown`, lines 185–186. Retained reference `native_dict_update_write_error_lifetime398-request-observed-probe.c`.

```text
fprintf(stderr, "DICT_RELEASE_OBSERVER|errors=%d|refcount_before_release=%d|same_cell=%d|defined_scalar=%d|final_free=%d\n", tclLspDictErrorObserved, tclLspDictErrorRefcount, tclLspDictErrorSameCell, tclLspDictErrorDefinedScalar, tclLspDictFinalFreeObserved);
return 0;}

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime399/observed-probe.c`, function `Independent body-return integer report before source-object release and interpreter teardown`, lines 127–127. Retained reference `native_dict_update_write_error_lifetime399-request-observed-probe.c`.

```text
static void probe_bytes(const char*id,const char*src,int source_len){caseid=id;pathid="fixed-grammar";active=create();Obj*o=NEW(active,src,source_len);INC(o);int code=SCRIPT(active,o);report("native-result",code);fprintf(stderr, "DICT_RELEASE_BODY_RETURN|errors=%d|refcount_before_release=%d|same_cell=%d|defined_scalar=%d|final_free=%d\n", tclLspDictErrorObserved, tclLspDictErrorRefcount, tclLspDictErrorSameCell, tclLspDictErrorDefinedScalar, tclLspDictFinalFreeObserved);DEC(active,o);DESTROY(active);}

```

## Consumer bindings

- [rust/tcl-vm/src/cmd_dict.rs](../../../../rust/tcl-vm/src/cmd_dict.rs), `cmd_dict::native_rmw_fixture_tests::dictionary_body_writeback_compares_original_native_observation_windows` (linked): Retain329 unchanged captured outer process-code references and326 complete results plus three bounded post-free observations: C8.5/C8.6 update-error and C8.6 with-error. Every later outer/result/diagnostic/caller-read field remains captured output without defined storage/completion/read/effect or another archived executable private lifetime; the original seven signal controls remain separate.
- [rust/tcl-vm/src/cmd_dict.rs](../../../../rust/tcl-vm/src/cmd_dict.rs), `cmd_dict::native_rmw_fixture_tests::c86_dictionary_write_error_retains_its_own_defined_host_storage` (linked): Verify exact398 source, baseline/observer bytes and reached integer counters, then independently assert VM safe live dictionary header/member storage. Native freed bytes, old captured-header identity and compiled frame ownership are not reconstructed.

## Evidence

- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399)
