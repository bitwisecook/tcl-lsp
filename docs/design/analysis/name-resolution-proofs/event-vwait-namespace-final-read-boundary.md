# naming.event.vwait-namespace-final-read-boundary

Kind: `native-observation`

## Problem statement

A whole-script namespace result can include a later read failure and cannot identify the wait handler as its cause.

## Question

What does the exact namespace/global composition return, and does its final fresh namespace read isolate vwait behavior?

## Conclusion

All five C providers return LOCAL GLOBAL. Jim returns error bytes can't read "x": no such variable. The script ends with [namespace eval N {set x}] in a new activation without redeclaring variable x; the captured row alone does not locate the failure at vwait versus this final read. It cannot overturn the independently measured same-activation Event106 namespace callback control.

## Scope

One exact ASCII composition per provider, fresh process and queried patchlevel. No intermediate stage getters or command traces were captured, so no failing-stage attribution or general Jim namespace equivalence follows. All ninety original processes completed with exit 0 and no external timeout. Return-options objects, headers, references and command-evaluation stages were not measured. This public direct-source worker question does not certify compiled alias selection.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Fresh configured provider linked by exact header/library/executable hashes in original receipts; info patchlevel queried in each process.. Channel: ASCII NUL-terminated C source literals; Tcl_EvalEx length -1/TCL_EVAL_DIRECT or Jim_Eval; counted public result getter after whole script.. Dialect: tcl8.4.

global-scope: guest code 0, 12 bytes b'LOCAL GLOBAL'

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Fresh configured provider linked by exact header/library/executable hashes in original receipts; info patchlevel queried in each process.. Channel: ASCII NUL-terminated C source literals; Tcl_EvalEx length -1/TCL_EVAL_DIRECT or Jim_Eval; counted public result getter after whole script.. Dialect: tcl8.5.

global-scope: guest code 0, 12 bytes b'LOCAL GLOBAL'

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Fresh configured provider linked by exact header/library/executable hashes in original receipts; info patchlevel queried in each process.. Channel: ASCII NUL-terminated C source literals; Tcl_EvalEx length -1/TCL_EVAL_DIRECT or Jim_Eval; counted public result getter after whole script.. Dialect: tcl8.6.

global-scope: guest code 0, 12 bytes b'LOCAL GLOBAL'

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Fresh configured provider linked by exact header/library/executable hashes in original receipts; info patchlevel queried in each process.. Channel: ASCII NUL-terminated C source literals; Tcl_EvalEx length -1/TCL_EVAL_DIRECT or Jim_Eval; counted public result getter after whole script.. Dialect: tcl9.0.

global-scope: guest code 0, 12 bytes b'LOCAL GLOBAL'

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Fresh configured provider linked by exact header/library/executable hashes in original receipts; info patchlevel queried in each process.. Channel: ASCII NUL-terminated C source literals; Tcl_EvalEx length -1/TCL_EVAL_DIRECT or Jim_Eval; counted public result getter after whole script.. Dialect: tcl9.1.

global-scope: guest code 0, 12 bytes b'LOCAL GLOBAL'

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Fresh configured provider linked by exact header/library/executable hashes in original receipts; info patchlevel queried in each process.. Channel: ASCII NUL-terminated C source literals; Tcl_EvalEx length -1/TCL_EVAL_DIRECT or Jim_Eval; counted public result getter after whole script.. Dialect: jim.

global-scope: guest code 1, 32 bytes b'can\'t read "x": no such variable'

### bigip

Status: `not-tested`. Version: not tested. Build: No attached build.. Channel: Not exercised.. Dialect: bigip.

No BIG-IP provider was launched or inspected for these controls.

## Exact evidence

- `e5eac3f8f7f00c774be23` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.4.20/global-scope.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.4.20/global-scope.json). SHA-256 `866f44d8c8620cd293c6ec6bddeed06534f0725ba695b76ed8b5db2aed912cc1`. Exact fresh-case outside process status, queried version and counted public result rows.
- `e53af814a8cb5945dbb1b` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.4.20/global-scope.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.4.20/global-scope.stdout.tsv). SHA-256 `137ececc252ec54325dbc3b3dbae8818b6d2e0e63d31379c18a847d09edee9a5`. Immutable original version/result stream; explicit byte count checked against hex.
- `e8a81de9521dbb6ff4770` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.4.20/global-scope.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.4.20/global-scope.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `ee448b8e66d0951a9e5d1` (provider): [rust/tcl-registry/tests/data/native_vwait_original/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.4.20/receipt.json). SHA-256 `56e65e1310881f86e2fca7a88fa3ea367f006e6bfbbd5a1abea3c60420243870`. Exact compiler, matching header/library/executable hashes, compile command and process attempts.
- `ed04d07a1274fc2b1a4b2` (input): [rust/tcl-registry/tests/data/native_vwait_original/probe.c](../../../../rust/tcl-registry/tests/data/native_vwait_original/probe.c). SHA-256 `ac4d4e8e34b71daaf93e4c35d9f92d380d53a0eeafa170a495fabb5bfe98de8b`. Exact original source/queue/harness; no new capture or repaired source. Derived inputs are definitions only.
- `ef0a2c0be217aa0bff877` (input): [rust/tcl-registry/tests/data/native_vwait_original/inputs.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/inputs.json). SHA-256 `d92c0f3d3a95a2fffacfebb864548e895e2c30bc2e380f43225a56b7e2bd7994`. Exact original source/queue/harness; no new capture or repaired source. Derived inputs are definitions only.
- `ee268c728a516f27332ef` (provider): [rust/tcl-registry/tests/data/native_vwait_original/capture.py](../../../../rust/tcl-registry/tests/data/native_vwait_original/capture.py). SHA-256 `5b0c5aa924f5fc6b18b3d3bdb783b163123afd47687cea49d36184a8b06eabe1`. Exact original source/queue/harness; no new capture or repaired source. Derived inputs are definitions only.
- `e270df4a56a72c8a2c89e` (provider): [rust/tcl-registry/tests/data/native_vwait_original/queue.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/queue.json). SHA-256 `5ccf57b0f68527dfa722dd477c174d84d11dcff996a91fa2884f4767704d2fb4`. Exact original source/queue/harness; no new capture or repaired source. Derived inputs are definitions only.
- `e80ed02f79ca4ef3a29b1` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.5.19/global-scope.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.5.19/global-scope.json). SHA-256 `3d45d4e8072c43ac70995e4d92254cefc5779c9bb6a3f5e0ffcecc68fa33ffdf`. Exact fresh-case outside process status, queried version and counted public result rows.
- `eddeb420534d15ec8781b` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.5.19/global-scope.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.5.19/global-scope.stdout.tsv). SHA-256 `4cde8b74f6f3b0a7cd03872e73ab203e07d5a7b0a33a6b174e62d3f2d1093b33`. Immutable original version/result stream; explicit byte count checked against hex.
- `e4ed693f653dca0030432` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.5.19/global-scope.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.5.19/global-scope.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `ea4649d4dd03aa4036a6f` (provider): [rust/tcl-registry/tests/data/native_vwait_original/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.5.19/receipt.json). SHA-256 `e9d0897b97b41b5ecb9bab762d8a633456bae5a44eb0dc120dc23cc16e2ee1bf`. Exact compiler, matching header/library/executable hashes, compile command and process attempts.
- `e6796c85ecbb96a86138c` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.6.18/global-scope.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.6.18/global-scope.json). SHA-256 `b80b2ed284b09077909a04a999f5575b922c13ee49ccae7df9807182d543dcc3`. Exact fresh-case outside process status, queried version and counted public result rows.
- `ed6c3444755e56fbecda1` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.6.18/global-scope.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.6.18/global-scope.stdout.tsv). SHA-256 `b674245cd64b3e80659d126a711d1f4517448a23e9c892f35fc432d7575a6a05`. Immutable original version/result stream; explicit byte count checked against hex.
- `ec8fc9b292fcf43e62ff0` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.6.18/global-scope.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.6.18/global-scope.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `e59663020753e1d549bd1` (provider): [rust/tcl-registry/tests/data/native_vwait_original/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.6.18/receipt.json). SHA-256 `36e0b31eb921934c694c56ffd7df3c71913add0acfc3f66697aa09731e7f1f4c`. Exact compiler, matching header/library/executable hashes, compile command and process attempts.
- `e32a0747aa6834597844b` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.0.4/global-scope.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.0.4/global-scope.json). SHA-256 `a08f498c40e7a2196b1100ab77c6239b8d6ca2458e2c1450711b818040977bc8`. Exact fresh-case outside process status, queried version and counted public result rows.
- `e0b23695df629da94c08c` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.0.4/global-scope.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.0.4/global-scope.stdout.tsv). SHA-256 `b0182538a8d68cf206160d094884c1cdac1625c94868bf3448ba4a4e6ebf3ba8`. Immutable original version/result stream; explicit byte count checked against hex.
- `e81cce8764937a7109770` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.0.4/global-scope.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.0.4/global-scope.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `ef1450973f13447d981ba` (provider): [rust/tcl-registry/tests/data/native_vwait_original/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.0.4/receipt.json). SHA-256 `b28141dad46630d7c783802d7cbf82f4fdea55a3e46d25c4452a4f9e89d4c318`. Exact compiler, matching header/library/executable hashes, compile command and process attempts.
- `ed6f3ad37225bfb752076` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.1.0/global-scope.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.1.0/global-scope.json). SHA-256 `a50e3d7b651420897114c2f2e682ca2b8b05c6f7c317a67ad07ac6c37d8833b2`. Exact fresh-case outside process status, queried version and counted public result rows.
- `e2a42f8edf8ba87798791` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.1.0/global-scope.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.1.0/global-scope.stdout.tsv). SHA-256 `ce0f52c70b073ebae222120eee7b94e77ed5bc1daf8a774b52131079c2b45a91`. Immutable original version/result stream; explicit byte count checked against hex.
- `e532c85b97305be4b30f6` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.1.0/global-scope.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.1.0/global-scope.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `e966e06b74b40b0e83972` (provider): [rust/tcl-registry/tests/data/native_vwait_original/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.1.0/receipt.json). SHA-256 `3d655dffe2f13b503f2480deab9ebe9c5423366eb9d2887283626e7db3ab7b98`. Exact compiler, matching header/library/executable hashes, compile command and process attempts.
- `eeb5b2ce89f8999cf004d` (observation): [rust/tcl-registry/tests/data/native_vwait_original/jim/global-scope.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/jim/global-scope.json). SHA-256 `815f2c851faa2b2776d79c0fccbf08dd9655535d72ab35baefdb00dcf6929ea7`. Exact fresh-case outside process status, queried version and counted public result rows.
- `e3d92222d64f09c639d45` (observation): [rust/tcl-registry/tests/data/native_vwait_original/jim/global-scope.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/jim/global-scope.stdout.tsv). SHA-256 `a92b2747b43017fc4cfb8095f0aac8008a877507c656557ba86a038c165fe5a6`. Immutable original version/result stream; explicit byte count checked against hex.
- `e1c764209686a8a1979a4` (observation): [rust/tcl-registry/tests/data/native_vwait_original/jim/global-scope.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/jim/global-scope.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `e4ee339a7a62e6661e884` (provider): [rust/tcl-registry/tests/data/native_vwait_original/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/jim/receipt.json). SHA-256 `04b0cf9b726b0823a589ba1e1cc46384ec861d88afdbe44908a1b3f2ac82d464`. Exact compiler, matching header/library/executable hashes, compile command and process attempts.
- `e98d4bc726037374525cd` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclEvent.c). SHA-256 `020cc8b9d0b020c2b1dac904f08b03c3e59889dd2ff493677f04696524d40673`. Pinned source inspection from prior Event source archive; not a new compiled-worker execution.
- `ebcd14ee056c41f642dc1` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclEvent.c). SHA-256 `3d05eb479c767520e0b44459e91a0b4930cdb47139c4bfdb86e0beba6d04fb90`. Pinned source inspection from prior Event source archive; not a new compiled-worker execution.
- `e1799f8f23cd143629768` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclEvent.c). SHA-256 `81c0e0b655ecd46bd981750c9d710ec11dd1368b7f3f156886e0965c81788dd6`. Pinned source inspection from prior Event source archive; not a new compiled-worker execution.
- `e30bb9be41c1824d28e5d` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclEvent.c). SHA-256 `e6b69ac0fc6c9c335af90623398d06eb2d8ef7d2df5574327a52f622a37e31e4`. Pinned source inspection from prior Event source archive; not a new compiled-worker execution.
- `eaa5f919dc246958bc5d1` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclEvent.c). SHA-256 `9451a2c540dcd5ed61675c15f833f5ad677b551b61deb210ee6837216f477a54`. Pinned source inspection from prior Event source archive; not a new compiled-worker execution.
- `e4521e47b782c5165c471` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/jim/jim-eventloop.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/jim/jim-eventloop.c). SHA-256 `b9536f7021919026f8ee2cfcdb35a59305db07a23e60daa50d5da966d52a1a12`. Pinned source inspection from prior Event source archive; not a new compiled-worker execution.

## Source inspection

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1176–1216. Full-source SHA-256 `020cc8b9d0b020c2b1dac904f08b03c3e59889dd2ff493677f04696524d40673`; snippet SHA-256 `1328a8f513834f5b7a5b16fc36f88138ec507eb04f53c43f8b8a9edda31a52ef`; retained evidence `e98d4bc726037374525cd`.

```text
Tcl_VwaitObjCmd(clientData, interp, objc, objv)
    ClientData clientData;	/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    int done, foundEvent;
    char *nameString;

    if (objc != 2) {
        Tcl_WrongNumArgs(interp, 1, objv, "name");
	return TCL_ERROR;
    }
    nameString = Tcl_GetString(objv[1]);
    if (Tcl_TraceVar(interp, nameString,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, (ClientData) &done) != TCL_OK) {
	return TCL_ERROR;
    };
    done = 0;
    foundEvent = 1;
    while (!done && foundEvent) {
	foundEvent = Tcl_DoOneEvent(TCL_ALL_EVENTS);
    }
    Tcl_UntraceVar(interp, nameString,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, (ClientData) &done);

    /*
     * Clear out the interpreter's result, since it may have been set
     * by event handlers.
     */

    Tcl_ResetResult(interp);
    if (!foundEvent) {
	Tcl_AppendResult(interp, "can't wait for variable \"", nameString,
		"\":  would wait forever", (char *) NULL);
	return TCL_ERROR;
    }
    return TCL_OK;
}
```

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1321–1368. Full-source SHA-256 `3d05eb479c767520e0b44459e91a0b4930cdb47139c4bfdb86e0beba6d04fb90`; snippet SHA-256 `cf24836c48502fc46b580d223b5cf8b21de1d5cad5eef848cd57b5caa2d4b60f`; retained evidence `ebcd14ee056c41f642dc1`.

```text
Tcl_VwaitObjCmd(
    ClientData clientData,	/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *CONST objv[])	/* Argument objects. */
{
    int done, foundEvent;
    char *nameString;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name");
	return TCL_ERROR;
    }
    nameString = Tcl_GetString(objv[1]);
    if (Tcl_TraceVar(interp, nameString,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, (ClientData) &done) != TCL_OK) {
	return TCL_ERROR;
    };
    done = 0;
    foundEvent = 1;
    while (!done && foundEvent) {
	foundEvent = Tcl_DoOneEvent(TCL_ALL_EVENTS);
	if (Tcl_LimitExceeded(interp)) {
	    break;
	}
    }
    Tcl_UntraceVar(interp, nameString,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, (ClientData) &done);

    /*
     * Clear out the interpreter's result, since it may have been set by event
     * handlers.
     */

    Tcl_ResetResult(interp);
    if (!foundEvent) {
	Tcl_AppendResult(interp, "can't wait for variable \"", nameString,
		"\": would wait forever", NULL);
	return TCL_ERROR;
    }
    if (!done) {
	Tcl_AppendResult(interp, "limit exceeded", NULL);
	return TCL_ERROR;
    }
    return TCL_OK;
}
```

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1387–1447. Full-source SHA-256 `81c0e0b655ecd46bd981750c9d710ec11dd1368b7f3f156886e0965c81788dd6`; snippet SHA-256 `6a720ec0b86320ee0c3eb9ac57fe7dda865390cee80306fd1844c2ccb65b85a7`; retained evidence `e1799f8f23cd143629768`.

```text
Tcl_VwaitObjCmd(
    ClientData clientData,	/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int done, foundEvent;
    const char *nameString;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name");
	return TCL_ERROR;
    }
    nameString = Tcl_GetString(objv[1]);
    if (Tcl_TraceVar2(interp, nameString, NULL,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, &done) != TCL_OK) {
	return TCL_ERROR;
    };
    done = 0;
    foundEvent = 1;
    while (!done && foundEvent) {
	foundEvent = Tcl_DoOneEvent(TCL_ALL_EVENTS);
	if (Tcl_Canceled(interp, TCL_LEAVE_ERR_MSG) == TCL_ERROR) {
	    break;
	}
	if (Tcl_LimitExceeded(interp)) {
	    Tcl_ResetResult(interp);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj("limit exceeded", -1));
	    break;
	}
    }
    Tcl_UntraceVar2(interp, nameString, NULL,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, &done);

    if (!foundEvent) {
	Tcl_ResetResult(interp);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"can't wait for variable \"%s\": would wait forever",
		nameString));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_SOURCES", (char *)NULL);
	return TCL_ERROR;
    }
    if (!done) {
	/*
	 * The interpreter's result was already set to the right error message
	 * prior to exiting the loop above.
	 */

	return TCL_ERROR;
    }

    /*
     * Clear out the interpreter's result, since it may have been set by event
     * handlers.
     */

    Tcl_ResetResult(interp);
    return TCL_OK;
}
```

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1498–1867. Full-source SHA-256 `e6b69ac0fc6c9c335af90623398d06eb2d8ef7d2df5574327a52f622a37e31e4`; snippet SHA-256 `51c2ed4fc19f0138783d2623b8c15f51f5485a04d976d7e8bbb2432d5a59c33b`; retained evidence `e30bb9be41c1824d28e5d`.

```text
Tcl_VwaitObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int i, done = 0, timedOut = 0, foundEvent, any = 1, timeout = 0;
    int numItems = 0, extended = 0, result, mode, mask = TCL_ALL_EVENTS;
    Tcl_InterpState saved = NULL;
    Tcl_TimerToken timer = NULL;
    Tcl_Time before, after;
    Tcl_Channel chan;
    Tcl_WideInt diff = -1;
    VwaitItem localItems[32], *vwaitItems = localItems;
    static const char *const vWaitOptionStrings[] = {
	"-all",	"-extended", "-nofileevents", "-noidleevents",
	"-notimerevents", "-nowindowevents", "-readable",
	"-timeout", "-variable", "-writable", "--", NULL
    };
    enum vWaitOptions {
	OPT_ALL, OPT_EXTD, OPT_NO_FEVTS, OPT_NO_IEVTS,
	OPT_NO_TEVTS, OPT_NO_WEVTS, OPT_READABLE,
	OPT_TIMEOUT, OPT_VARIABLE, OPT_WRITABLE, OPT_LAST
    } index;

    if ((objc == 2) && (strcmp(Tcl_GetString(objv[1]), "--") != 0)) {
	/*
	 * Legacy "vwait" syntax, skip option handling.
	 */
	i = 1;
	goto endOfOptionLoop;
    }

    if ((unsigned) objc - 1 > sizeof(localItems) / sizeof(localItems[0])) {
	vwaitItems = (VwaitItem *)Tcl_Alloc(sizeof(VwaitItem) * (objc - 1));
    }

    for (i = 1; i < objc; i++) {
	const char *name;

	name = TclGetString(objv[i]);
	if (name[0] != '-') {
	    break;
	}
	if (Tcl_GetIndexFromObj(interp, objv[i], vWaitOptionStrings, "option", 0,
		&index) != TCL_OK) {
	    result = TCL_ERROR;
	    goto done;
	}
	switch (index) {
	case OPT_ALL:
	    any = 0;
	    break;
	case OPT_EXTD:
	    extended = 1;
	    break;
	case OPT_NO_FEVTS:
	    mask &= ~TCL_FILE_EVENTS;
	    break;
	case OPT_NO_IEVTS:
	    mask &= ~TCL_IDLE_EVENTS;
	    break;
	case OPT_NO_TEVTS:
	    mask &= ~TCL_TIMER_EVENTS;
	    break;
	case OPT_NO_WEVTS:
	    mask &= ~TCL_WINDOW_EVENTS;
	    break;
	case OPT_TIMEOUT:
	    if (++i >= objc) {
	needArg:
		Tcl_ResetResult(interp);
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"argument required for \"%s\"", vWaitOptionStrings[index]));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "ARGUMENT", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    if (Tcl_GetIntFromObj(interp, objv[i], &timeout) != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (timeout < 0) {
		Tcl_ResetResult(interp);
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"timeout must be positive", -1));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "NEGTIME", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    break;
	case OPT_LAST:
	    i++;
	    goto endOfOptionLoop;
	case OPT_VARIABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    result = Tcl_TraceVar2(interp, TclGetString(objv[i]), NULL,
		    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		    VwaitVarProc, &vwaitItems[numItems]);
	    if (result != TCL_OK) {
		goto done;
	    }
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = -1;
	    vwaitItems[numItems].mask = 0;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	case OPT_READABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    if (TclGetChannelFromObj(interp, objv[i], &chan, &mode, 0)
		    != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (!(mode & TCL_READABLE)) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"channel \"%s\" wasn't open for reading",
			TclGetString(objv[i])));
		result = TCL_ERROR;
		goto done;
	    }
	    Tcl_CreateChannelHandler(chan, TCL_READABLE,
		    VwaitChannelReadProc, &vwaitItems[numItems]);
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = -1;
	    vwaitItems[numItems].mask = TCL_READABLE;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	case OPT_WRITABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    if (TclGetChannelFromObj(interp, objv[i], &chan, &mode, 0)
		    != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (!(mode & TCL_WRITABLE)) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"channel \"%s\" wasn't open for writing",
			TclGetString(objv[i])));
		result = TCL_ERROR;
		goto done;
	    }
	    Tcl_CreateChannelHandler(chan, TCL_WRITABLE,
		    VwaitChannelWriteProc, &vwaitItems[numItems]);
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = -1;
	    vwaitItems[numItems].mask = TCL_WRITABLE;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    }

  endOfOptionLoop:
    if ((mask & (TCL_FILE_EVENTS | TCL_IDLE_EVENTS |
	    TCL_TIMER_EVENTS | TCL_WINDOW_EVENTS)) == 0) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"can't wait: would block forever", -1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_SOURCES", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    if ((timeout > 0) && ((mask & TCL_TIMER_EVENTS) == 0)) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"timer events disabled with timeout specified", -1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_TIME", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    for (result = TCL_OK; i < objc; i++) {
	result = Tcl_TraceVar2(interp, TclGetString(objv[i]), NULL,
		TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		VwaitVarProc, &vwaitItems[numItems]);
	if (result != TCL_OK) {
	    break;
	}
	vwaitItems[numItems].donePtr = &done;
	vwaitItems[numItems].sequence = -1;
	vwaitItems[numItems].mask = 0;
	vwaitItems[numItems].sourceObj = objv[i];
	numItems++;
    }
    if (result != TCL_OK) {
	result = TCL_ERROR;
	goto done;
    }

    if (!(mask & TCL_FILE_EVENTS)) {
	for (i = 0; i < numItems; i++) {
	    if (vwaitItems[i].mask) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"file events disabled with channel(s) specified", -1));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_FILE_EVENT", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	}
    }

    if (timeout > 0) {
	vwaitItems[numItems].donePtr = &timedOut;
	vwaitItems[numItems].sequence = -1;
	vwaitItems[numItems].mask = 0;
	vwaitItems[numItems].sourceObj = NULL;
	timer = Tcl_CreateTimerHandler(timeout, VwaitTimeoutProc,
		&vwaitItems[numItems]);
	Tcl_GetTime(&before);
    } else {
	timeout = 0;
    }

    if ((numItems == 0) && (timeout == 0)) {
	/*
	 * "vwait" is equivalent to "update",
	 * "vwait -nofileevents -notimerevents -nowindowevents"
	 * is equivalent to "update idletasks"
	 */
	any = 1;
	mask |= TCL_DONT_WAIT;
    }

    foundEvent = 1;
    while (!timedOut && foundEvent &&
	   ((!any && (done < numItems)) || (any && !done))) {
	foundEvent = Tcl_DoOneEvent(mask);
	if (Tcl_Canceled(interp, TCL_LEAVE_ERR_MSG) == TCL_ERROR) {
	    break;
	}
	if (Tcl_LimitExceeded(interp)) {
	    Tcl_ResetResult(interp);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj("limit exceeded", -1));
	    Tcl_SetErrorCode(interp, "TCL", "EVENT", "LIMIT", (char *)NULL);
	    break;
	}
	if ((numItems == 0) && (timeout == 0)) {
	    /*
	     * Behavior like "update": clear interpreter's result because
	     * event handlers could have executed commands.
	     */
	    Tcl_ResetResult(interp);
	    result = TCL_OK;
	    goto done;
	}
    }

    if (!foundEvent) {
	Tcl_ResetResult(interp);
	Tcl_SetObjResult(interp, Tcl_NewStringObj((numItems == 0) ?
		"can't wait: would wait forever" :
		"can't wait for variable(s)/channel(s): would wait forever",
		-1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_SOURCES", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    if (!done && !timedOut) {
	/*
	 * The interpreter's result was already set to the right error message
	 * prior to exiting the loop above.
	 */
	result = TCL_ERROR;
	goto done;
    }

    result = TCL_OK;
    if (timeout <= 0) {
	/*
	 * Clear out the interpreter's result, since it may have been set
	 * by event handlers.
	 */
	Tcl_ResetResult(interp);
	goto done;
    }

    /*
     * When timeout was specified, report milliseconds left or -1 on timeout.
     */
    if (timedOut) {
	diff = -1;
    } else {
	Tcl_GetTime(&after);
	diff = after.sec * 1000 + after.usec / 1000;
	diff -= before.sec * 1000 + before.usec / 1000;
	diff = timeout - diff;
	if (diff < 0) {
	    diff = 0;
	}
    }

  done:
    if ((timeout > 0) && (timer != NULL)) {
	Tcl_DeleteTimerHandler(timer);
    }
    if (result != TCL_OK) {
	saved = Tcl_SaveInterpState(interp, result);
    }
    for (i = 0; i < numItems; i++) {
	if (vwaitItems[i].mask & TCL_READABLE) {
	    if (TclGetChannelFromObj(interp, vwaitItems[i].sourceObj,
		    &chan, &mode, 0) == TCL_OK) {
		Tcl_DeleteChannelHandler(chan, VwaitChannelReadProc,
			&vwaitItems[i]);
	    }
	} else if (vwaitItems[i].mask & TCL_WRITABLE) {
	    if (TclGetChannelFromObj(interp, vwaitItems[i].sourceObj,
		    &chan, &mode, 0) == TCL_OK) {
		Tcl_DeleteChannelHandler(chan, VwaitChannelWriteProc,
			&vwaitItems[i]);
	    }
	} else {
	    Tcl_UntraceVar2(interp, TclGetString(vwaitItems[i].sourceObj),
		    NULL, TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		    VwaitVarProc, &vwaitItems[i]);
	}
    }

    if (result == TCL_OK) {
	if (extended) {
	    int k;
	    Tcl_Obj *listObj, *keyObj;

	    TclNewObj(listObj);
	    for (k = 0; k < done; k++) {
		for (i = 0; i < numItems; i++) {
		    if (vwaitItems[i].sequence != k) {
			continue;
		    }
		    if (vwaitItems[i].mask & TCL_READABLE) {
			TclNewLiteralStringObj(keyObj, "readable");
		    } else if (vwaitItems[i].mask & TCL_WRITABLE) {
			TclNewLiteralStringObj(keyObj, "writable");
		    } else {
			TclNewLiteralStringObj(keyObj, "variable");
		    }
		    Tcl_ListObjAppendElement(NULL, listObj, keyObj);
		    Tcl_ListObjAppendElement(NULL, listObj,
			    vwaitItems[i].sourceObj);
		}
	    }
	    if (timeout > 0) {
		TclNewLiteralStringObj(keyObj, "timeleft");
		Tcl_ListObjAppendElement(NULL, listObj, keyObj);
		Tcl_ListObjAppendElement(NULL, listObj,
			Tcl_NewWideIntObj(diff));
	    }
	    Tcl_SetObjResult(interp, listObj);
	} else if (timeout > 0) {
	    Tcl_SetObjResult(interp, Tcl_NewWideIntObj(diff));
	}
    } else {
	result = Tcl_RestoreInterpState(interp, saved);
    }
    if (vwaitItems != localItems) {
	Tcl_Free(vwaitItems);
    }
    return result;
}
```

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1536–1906. Full-source SHA-256 `9451a2c540dcd5ed61675c15f833f5ad677b551b61deb210ee6837216f477a54`; snippet SHA-256 `0f13359ad3fe496d90ff98f311f7951d3e6c13a770b2f9b2a8e4fdc9fd49d291`; retained evidence `eaa5f919dc246958bc5d1`.

```text
Tcl_VwaitObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Size i, done = 0, numItems = 0, timedOut = 0;
    int foundEvent, any = 1, timeout = 0;
    int extended = 0, result, mode, mask = TCL_ALL_EVENTS;
    Tcl_InterpState saved = NULL;
    Tcl_TimerToken timer = NULL;
    long long before = -1, after;
    Tcl_Channel chan;
    Tcl_WideInt diff = -1;
    VwaitItem localItems[32], *vwaitItems = localItems;
    static const char *const vWaitOptionStrings[] = {
	"-all",	"-extended", "-nofileevents", "-noidleevents",
	"-notimerevents", "-nowindowevents", "-readable",
	"-timeout", "-variable", "-writable", "--", NULL
    };
    enum vWaitOptions {
	OPT_ALL, OPT_EXTD, OPT_NO_FEVTS, OPT_NO_IEVTS,
	OPT_NO_TEVTS, OPT_NO_WEVTS, OPT_READABLE,
	OPT_TIMEOUT, OPT_VARIABLE, OPT_WRITABLE, OPT_LAST
    } index;

    if ((objc == 2) && (strcmp(Tcl_GetString(objv[1]), "--") != 0)) {
	/*
	 * Legacy "vwait" syntax, skip option handling.
	 */
	i = 1;
	goto endOfOptionLoop;
    }

    if ((unsigned) objc - 1 > sizeof(localItems) / sizeof(localItems[0])) {
	vwaitItems = (VwaitItem *)Tcl_Alloc(sizeof(VwaitItem) * (objc - 1));
    }

    for (i = 1; i < objc; i++) {
	const char *name;

	name = TclGetString(objv[i]);
	if (name[0] != '-') {
	    break;
	}
	if (Tcl_GetIndexFromObj(interp, objv[i], vWaitOptionStrings, "option", 0,
		&index) != TCL_OK) {
	    result = TCL_ERROR;
	    goto done;
	}
	switch (index) {
	case OPT_ALL:
	    any = 0;
	    break;
	case OPT_EXTD:
	    extended = 1;
	    break;
	case OPT_NO_FEVTS:
	    mask &= ~TCL_FILE_EVENTS;
	    break;
	case OPT_NO_IEVTS:
	    mask &= ~TCL_IDLE_EVENTS;
	    break;
	case OPT_NO_TEVTS:
	    mask &= ~TCL_TIMER_EVENTS;
	    break;
	case OPT_NO_WEVTS:
	    mask &= ~TCL_WINDOW_EVENTS;
	    break;
	case OPT_TIMEOUT:
	    if (++i >= objc) {
	needArg:
		Tcl_ResetResult(interp);
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"argument required for \"%s\"", vWaitOptionStrings[index]));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "ARGUMENT", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    if (Tcl_GetIntFromObj(interp, objv[i], &timeout) != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (timeout < 0) {
		Tcl_ResetResult(interp);
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"timeout must be positive", -1));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "NEGTIME", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    break;
	case OPT_LAST:
	    i++;
	    goto endOfOptionLoop;
	case OPT_VARIABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    result = Tcl_TraceVar2(interp, TclGetString(objv[i]), NULL,
		    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		    VwaitVarProc, &vwaitItems[numItems]);
	    if (result != TCL_OK) {
		goto done;
	    }
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = TCL_INDEX_NONE;
	    vwaitItems[numItems].mask = 0;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	case OPT_READABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    if (TclGetChannelFromObj(interp, objv[i], &chan, &mode, 0)
		    != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (!(mode & TCL_READABLE)) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"channel \"%s\" wasn't open for reading",
			TclGetString(objv[i])));
		result = TCL_ERROR;
		goto done;
	    }
	    Tcl_CreateChannelHandler(chan, TCL_READABLE,
		    VwaitChannelReadProc, &vwaitItems[numItems]);
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = TCL_INDEX_NONE;
	    vwaitItems[numItems].mask = TCL_READABLE;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	case OPT_WRITABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    if (TclGetChannelFromObj(interp, objv[i], &chan, &mode, 0)
		    != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (!(mode & TCL_WRITABLE)) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"channel \"%s\" wasn't open for writing",
			TclGetString(objv[i])));
		result = TCL_ERROR;
		goto done;
	    }
	    Tcl_CreateChannelHandler(chan, TCL_WRITABLE,
		    VwaitChannelWriteProc, &vwaitItems[numItems]);
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = TCL_INDEX_NONE;
	    vwaitItems[numItems].mask = TCL_WRITABLE;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    }

  endOfOptionLoop:
    if ((mask & (TCL_FILE_EVENTS | TCL_IDLE_EVENTS |
	    TCL_TIMER_EVENTS | TCL_WINDOW_EVENTS)) == 0) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"can't wait: would block forever", -1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_SOURCES", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    if ((timeout > 0) && ((mask & TCL_TIMER_EVENTS) == 0)) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"timer events disabled with timeout specified", -1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_TIME", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    for (result = TCL_OK; i < objc; i++) {
	result = Tcl_TraceVar2(interp, TclGetString(objv[i]), NULL,
		TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		VwaitVarProc, &vwaitItems[numItems]);
	if (result != TCL_OK) {
	    break;
	}
	vwaitItems[numItems].donePtr = &done;
	vwaitItems[numItems].sequence = TCL_INDEX_NONE;
	vwaitItems[numItems].mask = 0;
	vwaitItems[numItems].sourceObj = objv[i];
	numItems++;
    }
    if (result != TCL_OK) {
	result = TCL_ERROR;
	goto done;
    }

    if (!(mask & TCL_FILE_EVENTS)) {
	for (i = 0; i < numItems; i++) {
	    if (vwaitItems[i].mask) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"file events disabled with channel(s) specified", -1));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_FILE_EVENT", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	}
    }

    if (timeout > 0) {
	vwaitItems[numItems].donePtr = &timedOut;
	vwaitItems[numItems].sequence = TCL_INDEX_NONE;
	vwaitItems[numItems].mask = 0;
	vwaitItems[numItems].sourceObj = NULL;
	timer = Tcl_CreateTimerHandler(timeout, VwaitTimeoutProc,
		&vwaitItems[numItems]);
	before = Tcl_GetDayTime();
    } else {
	timeout = 0;
    }

    if ((numItems == 0) && (timeout == 0)) {
	/*
	 * "vwait" is equivalent to "update",
	 * "vwait -nofileevents -notimerevents -nowindowevents"
	 * is equivalent to "update idletasks"
	 */
	any = 1;
	mask |= TCL_DONT_WAIT;
    }

    foundEvent = 1;
    while (!timedOut && foundEvent &&
	    ((!any && (done < numItems)) || (any && !done))) {
	foundEvent = Tcl_DoOneEvent(mask);
	if (Tcl_Canceled(interp, TCL_LEAVE_ERR_MSG) == TCL_ERROR) {
	    break;
	}
	if (Tcl_LimitExceeded(interp)) {
	    Tcl_ResetResult(interp);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj("limit exceeded", -1));
	    Tcl_SetErrorCode(interp, "TCL", "EVENT", "LIMIT", (char *)NULL);
	    break;
	}
	if ((numItems == 0) && (timeout == 0)) {
	    /*
	     * Behavior like "update": clear interpreter's result because
	     * event handlers could have executed commands.
	     */
	    Tcl_ResetResult(interp);
	    result = TCL_OK;
	    goto done;
	}
    }

    if (!foundEvent) {
	Tcl_ResetResult(interp);
	Tcl_SetObjResult(interp, Tcl_NewStringObj((numItems == 0) ?
		"can't wait: would wait forever" :
		"can't wait for variable(s)/channel(s): would wait forever",
		-1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_SOURCES", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    if (!done && !timedOut) {
	/*
	 * The interpreter's result was already set to the right error message
	 * prior to exiting the loop above.
	 */
	result = TCL_ERROR;
	goto done;
    }

    result = TCL_OK;
    if (timeout <= 0) {
	/*
	 * Clear out the interpreter's result, since it may have been set
	 * by event handlers.
	 */
	Tcl_ResetResult(interp);
	goto done;
    }

    /*
     * When timeout was specified, report milliseconds left or -1 on timeout.
     */
    if (timedOut) {
	diff = -1;
    } else {
	after = Tcl_GetDayTime();
	diff = after / 1000;
	diff -= before / 1000;
	diff = timeout - diff;
	if (diff < 0) {
	    diff = 0;
	}
    }

  done:
    if ((timeout > 0) && (timer != NULL)) {
	Tcl_DeleteTimerHandler(timer);
    }
    if (result != TCL_OK) {
	saved = Tcl_SaveInterpState(interp, result);
    }
    for (i = 0; i < numItems; i++) {
	if (vwaitItems[i].mask & TCL_READABLE) {
	    if (TclGetChannelFromObj(interp, vwaitItems[i].sourceObj,
		    &chan, &mode, 0) == TCL_OK) {
		Tcl_DeleteChannelHandler(chan, VwaitChannelReadProc,
			&vwaitItems[i]);
	    }
	} else if (vwaitItems[i].mask & TCL_WRITABLE) {
	    if (TclGetChannelFromObj(interp, vwaitItems[i].sourceObj,
		    &chan, &mode, 0) == TCL_OK) {
		Tcl_DeleteChannelHandler(chan, VwaitChannelWriteProc,
			&vwaitItems[i]);
	    }
	} else {
	    Tcl_UntraceVar2(interp, TclGetString(vwaitItems[i].sourceObj),
		    NULL, TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		    VwaitVarProc, &vwaitItems[i]);
	}
    }

    if (result == TCL_OK) {
	if (extended) {
	    Tcl_Size k;
	    Tcl_Obj *listObj, *keyObj;

	    TclNewObj(listObj);
	    for (k = 0; k < done; k++) {
		for (i = 0; i < numItems; i++) {
		    if (vwaitItems[i].sequence != k) {
			continue;
		    }
		    if (vwaitItems[i].mask & TCL_READABLE) {
			TclNewLiteralStringObj(keyObj, "readable");
		    } else if (vwaitItems[i].mask & TCL_WRITABLE) {
			TclNewLiteralStringObj(keyObj, "writable");
		    } else {
			TclNewLiteralStringObj(keyObj, "variable");
		    }
		    Tcl_ListObjAppendElement(NULL, listObj, keyObj);
		    Tcl_ListObjAppendElement(NULL, listObj,
			    vwaitItems[i].sourceObj);
		}
	    }
	    if (timeout > 0) {
		TclNewLiteralStringObj(keyObj, "timeleft");
		Tcl_ListObjAppendElement(NULL, listObj, keyObj);
		Tcl_ListObjAppendElement(NULL, listObj,
			Tcl_NewWideIntObj(diff));
	    }
	    Tcl_SetObjResult(interp, listObj);
	} else if (timeout > 0) {
	    Tcl_SetObjResult(interp, Tcl_NewWideIntObj(diff));
	}
    } else {
	result = Tcl_RestoreInterpState(interp, saved);
    }
    if (vwaitItems != localItems) {
	Tcl_Free(vwaitItems);
    }
    return result;
}
```

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `JimELVwaitCommand`, lines 565–645. Full-source SHA-256 `b9536f7021919026f8ee2cfcdb35a59305db07a23e60daa50d5da966d52a1a12`; snippet SHA-256 `0309243c6a2a7cca9a072f5c5f7ed82248d291eae31886f251c35eb3f730c45a`; retained evidence `e4521e47b782c5165c471`.

```text
static int JimELVwaitCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_EventLoop *eventLoop = Jim_CmdPrivData(interp);
    Jim_Obj *oldValue = NULL;
    Jim_Obj *scriptObjPtr = NULL;
    int rc;
    int signal = 0;

    if (argc > 2 && Jim_CompareStringImmediate(interp, argv[1], "-signal")) {
        signal++;
    }

    if (argc - signal == 3) {
        scriptObjPtr = argv[2 + signal];
    }
    else if (argc - signal != 2) {
        return JIM_USAGE;
    }

    oldValue = Jim_GetGlobalVariable(interp, argv[1 + signal], JIM_NONE);

    if (oldValue) {
        Jim_IncrRefCount(oldValue);
    }
    else {
        /* If a result was left, it is an error */
        if (Jim_Length(Jim_GetResult(interp))) {
            return JIM_ERR;
        }
    }

    eventLoop->suppress_bgerror = 0;

    while ((rc = Jim_ProcessEvents(interp, JIM_ALL_EVENTS)) >= 0) {
        Jim_Obj *currValue;

        if (signal && interp->sigmask) {
            /* vwait -signal and handled signals were received, so transfer them
             * to ignored signals so that 'signal check -clear' will return them.
             * It's possible that if signals aren't supported we shouldn't even
             * allow the -signal option.
             */
#ifdef jim_ext_signal
            Jim_SignalSetIgnored(interp->sigmask);
#endif
            interp->sigmask = 0;
            break;
        }

        currValue = Jim_GetGlobalVariable(interp, argv[1 + signal], JIM_NONE);
        /* Stop the loop if the vwait-ed variable changed value,
         * or if was unset and now is set (or the contrary)
         * or if a signal was caught
         */
        if ((oldValue && !currValue) ||
            (!oldValue && currValue) ||
            (oldValue && currValue && !Jim_StringEqObj(oldValue, currValue)) ||
            Jim_CheckSignal(interp)) {
            break;
        }
        if (scriptObjPtr) {
            /* Stop the loop if a provided script returns BREAK or ERR */
            int retval = Jim_EvalObj(interp, scriptObjPtr);
            if (retval == JIM_ERR || retval == JIM_BREAK) {
                if (retval == JIM_ERR) {
                    rc = -2;
                }
                break;
            }
        }
    }
    if (oldValue)
        Jim_DecrRefCount(interp, oldValue);

    if (rc == -2) {
        return JIM_ERR;
    }

    Jim_SetEmptyResult(interp);
    return JIM_OK;
}
```


## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

No native/Rust launch is performed by this record. Original immutable receipt commands/probe/queue/harness hashes remain available; original capture directory must not be overwritten. Source windows remain independent inspected Event source evidence.
