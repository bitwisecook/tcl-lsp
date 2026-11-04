# naming.tcloo.method-original-option-layout

Kind: `native-observation`

## Problem statement

A method declaration can have a dash-prefixed name or formal list, so scanning the first dash word does not distinguish a method option from data. C9 additionally resolves an original export flag object by native CString index rules, making raw counted zero different from encoded C080. This matters when the VM and runtime select body/formal operands and visibility. The check is limited to the captured TclOO method argv and the explicit version-dependent forms.

## Question

Which original method argv forms and export option objects do C8.6 and C9 accept, and how do they distinguish public, unexported and true-private declarations?

## Conclusion

C8.6 accepts only the option-free name/args/body form and rejects the fourth operand. C9 selects a single option after the name only in the four-operand form; -private and its -p prefix, including a raw-zero suffix, select true-private visibility, while encoded C080 suffix and -- reject. -export is public and -unexport is protected; true-private is omitted from the -private method roster. Three-operand dashed formal lists and a method named -export remain data. C8.4/C8.5/Jim lack the measured OO command. These outputs do not prove cache, header, refcount, editable source or Rust implementation behavior.

## Scope

Both retained probe variants: public Tcl_NewStringObj(pointer,count)/Tcl_EvalObjv method option vectors, with independent ASCII NUL-terminated Tcl_Eval setup and query scripts; Jim runs only startup and OO-availability queries. Option bytes distinguish binary 00 from encoded C080. v1 dashed-formal script returns a literal $-arg; v2 uses ${-arg} and returns VALUE. No physical original-object window is inspected.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Actual executable; public header, static library, Makefile, source owner, compile command and process/stream hashes retained independently for each variant.. Channel: Original counted Tcl objects passed to Tcl_EvalObjv plus ASCII NUL-terminated Tcl_Eval scripts.. Dialect: Tcl.

The startup query reports this version and the actual OO command query is empty; no OO argv or private body is executed.

### tcl8.5

Status: `unsupported`. Version: 8.5.19. Build: Actual executable; public header, static library, Makefile, source owner, compile command and process/stream hashes retained independently for each variant.. Channel: Original counted Tcl objects passed to Tcl_EvalObjv plus ASCII NUL-terminated Tcl_Eval scripts.. Dialect: Tcl.

The startup query reports this version and the actual OO command query is empty; no OO argv or private body is executed.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable; public header, static library, Makefile, source owner, compile command and process/stream hashes retained independently for each variant.. Channel: Original counted Tcl objects passed to Tcl_EvalObjv plus ASCII NUL-terminated Tcl_Eval scripts.. Dialect: Tcl.

Every four-operand method form rejects with wrong-argument usage; option-free dashed formal/name controls succeed and external same-object Hidden rejects. The private worker controls reject as unavailable.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable; public header, static library, Makefile, source owner, compile command and process/stream hashes retained independently for each variant.. Channel: Original counted Tcl objects passed to Tcl_EvalObjv plus ASCII NUL-terminated Tcl_Eval scripts.. Dialect: Tcl.

Original -private/-p/raw-zero option forms succeed, remain absent from the -private roster, reject external calls and return OK via the declaring method. Encoded C080 and -- reject; -export calls externally, -unexport only internally. The option-free data controls succeed.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable; public header, static library, Makefile, source owner, compile command and process/stream hashes retained independently for each variant.. Channel: Original counted Tcl objects passed to Tcl_EvalObjv plus ASCII NUL-terminated Tcl_Eval scripts.. Dialect: Tcl.

Original -private/-p/raw-zero option forms succeed, remain absent from the -private roster, reject external calls and return OK via the declaring method. Encoded C080 and -- reject; -export calls externally, -unexport only internally. The option-free data controls succeed.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual executable; public header, static library, Makefile, source owner, compile command and process/stream hashes retained independently for each variant.. Channel: ASCII NUL-terminated startup and OO-availability source passed to Jim_Eval; no Jim OO operand vector executes.. Dialect: Jim Tcl.

The startup query reports this version and the actual OO command query is empty; no OO argv or private body is executed.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation for this exact method argv or private caller question.

## Historical references

- `v1-input` (input): [rust/tcl-vm/tests/data/native_method_visibility/v1/probe.c](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/probe.c). Exact original probe or complete six-provider receipt; earlier variant is preserved independently.
- `v1-aggregate` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v1/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/receipt.json). Exact original probe or complete six-provider receipt; earlier variant is preserved independently.
- `v2-input` (input): [rust/tcl-vm/tests/data/native_method_visibility/v2/probe.c](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/probe.c). Exact original probe or complete six-provider receipt; earlier variant is preserved independently.
- `v2-aggregate` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v2/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/receipt.json). Exact original probe or complete six-provider receipt; earlier variant is preserved independently.
- `v1-tcl8.4-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v1/8.4.20/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/8.4.20/receipt.json). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl8.4-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/8.4.20/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/8.4.20/stdout.tsv). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl8.4-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/8.4.20/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/8.4.20/stderr). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl8.4-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v2/8.4.20/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/8.4.20/receipt.json). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl8.4-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/8.4.20/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/8.4.20/stdout.tsv). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl8.4-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/8.4.20/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/8.4.20/stderr). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl8.5-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v1/8.5.19/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/8.5.19/receipt.json). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl8.5-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/8.5.19/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/8.5.19/stdout.tsv). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl8.5-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/8.5.19/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/8.5.19/stderr). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl8.5-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v2/8.5.19/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/8.5.19/receipt.json). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl8.5-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/8.5.19/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/8.5.19/stdout.tsv). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl8.5-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/8.5.19/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/8.5.19/stderr). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl8.6-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v1/8.6.18/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/8.6.18/receipt.json). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl8.6-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/8.6.18/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/8.6.18/stdout.tsv). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl8.6-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/8.6.18/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/8.6.18/stderr). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl8.6-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v2/8.6.18/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/8.6.18/receipt.json). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl8.6-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/8.6.18/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/8.6.18/stdout.tsv). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl8.6-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/8.6.18/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/8.6.18/stderr). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl9.0-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v1/9.0.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/9.0.4/receipt.json). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl9.0-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/9.0.4/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/9.0.4/stdout.tsv). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl9.0-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/9.0.4/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/9.0.4/stderr). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl9.0-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v2/9.0.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/9.0.4/receipt.json). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl9.0-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/9.0.4/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/9.0.4/stdout.tsv). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl9.0-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/9.0.4/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/9.0.4/stderr). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl9.1-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v1/9.1.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/9.1.0/receipt.json). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl9.1-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/9.1.0/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/9.1.0/stdout.tsv). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl9.1-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/9.1.0/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/9.1.0/stderr). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl9.1-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v2/9.1.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/9.1.0/receipt.json). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl9.1-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/9.1.0/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/9.1.0/stdout.tsv). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl9.1-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/9.1.0/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/9.1.0/stderr). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-jim-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v1/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/jim/receipt.json). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-jim-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/jim/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/jim/stdout.tsv). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-jim-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/jim/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/jim/stderr). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-jim-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v2/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/jim/receipt.json). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-jim-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/jim/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/jim/stdout.tsv). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-jim-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/jim/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/jim/stderr). Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `tcl8.6-method-source` (source-anchor): [rust/tcl-vm/tests/data/native_method_visibility/source-anchors/8.6.18-method-definition.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/source-anchors/8.6.18-method-definition.json). JSON pointer `/snippet`. Actual pinned C source excerpt, full source digest and exact line coordinates; independent of native result observations.
- `tcl9.0-method-source` (source-anchor): [rust/tcl-vm/tests/data/native_method_visibility/source-anchors/9.0.4-method-definition.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/source-anchors/9.0.4-method-definition.json). JSON pointer `/snippet`. Actual pinned C source excerpt, full source digest and exact line coordinates; independent of native result observations.
- `tcl9.1-method-source` (source-anchor): [rust/tcl-vm/tests/data/native_method_visibility/source-anchors/9.1.0-method-definition.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/source-anchors/9.1.0-method-definition.json). JSON pointer `/snippet`. Actual pinned C source excerpt, full source digest and exact line coordinates; independent of native result observations.

## Source inspection

tcl8.6 8.6.18, revision `Pinned Tcl 8.6.18 release source`, `generic/tclOODefineCmds.c`, function `TclOODefineMethodObjCmd`, lines 1572–1616. Retained reference `tcl8.6-method-source`.

```text
TclOODefineMethodObjCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    int isInstanceMethod = (clientData != NULL);
    Object *oPtr;
    int isPublic;

    if (objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "name args body");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceMethod && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }
    isPublic = Tcl_StringMatch(TclGetString(objv[1]), PUBLIC_PATTERN)
	    ? PUBLIC_METHOD : 0;

    /*
     * Create the method by using the right back-end API.
     */

    if (isInstanceMethod) {
	if (TclOONewProcInstanceMethod(interp, oPtr, isPublic, objv[1],
		objv[2], objv[3], NULL) == NULL) {
	    return TCL_ERROR;
	}
    } else {
	if (TclOONewProcMethod(interp, oPtr->classPtr, isPublic, objv[1],
		objv[2], objv[3], NULL) == NULL) {
	    return TCL_ERROR;
	}
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `Pinned Tcl 9.0.4 release source`, `generic/tclOODefineCmds.c`, function `TclOODefineMethodObjCmd`, lines 2309–2393. Retained reference `tcl9.0-method-source`.

```text
TclOODefineMethodObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    /*
     * Table of export modes for methods and their corresponding enum.
     */

    static const char *const exportModes[] = {
	"-export",
	"-private",
	"-unexport",
	NULL
    };
    enum ExportMode {
	MODE_EXPORT,
	MODE_PRIVATE,
	MODE_UNEXPORT
    } exportMode;

    int isInstanceMethod = (clientData != NULL);
    Object *oPtr;
    int isPublic = 0;

    if (objc < 4 || objc > 5) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?option? args body");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceMethod && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return TCL_ERROR;
    }
    if (objc == 5) {
	if (Tcl_GetIndexFromObj(interp, objv[2], exportModes, "export flag",
		0, &exportMode) != TCL_OK) {
	    return TCL_ERROR;
	}
	switch (exportMode) {
	case MODE_EXPORT:
	    isPublic = PUBLIC_METHOD;
	    break;
	case MODE_PRIVATE:
	    isPublic = TRUE_PRIVATE_METHOD;
	    break;
	case MODE_UNEXPORT:
	    isPublic = 0;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    } else {
	if (IsPrivateDefine(interp)) {
	    isPublic = TRUE_PRIVATE_METHOD;
	} else {
	    isPublic = Tcl_StringMatch(TclGetString(objv[1]), PUBLIC_PATTERN)
		    ? PUBLIC_METHOD : 0;
	}
    }

    /*
     * Create the method by using the right back-end API.
     */

    if (isInstanceMethod) {
	if (TclOONewProcInstanceMethod(interp, oPtr, isPublic, objv[1],
		objv[objc - 2], objv[objc - 1], NULL) == NULL) {
	    return TCL_ERROR;
	}
    } else {
	if (TclOONewProcMethod(interp, oPtr->classPtr, isPublic, objv[1],
		objv[objc - 2], objv[objc - 1], NULL) == NULL) {
	    return TCL_ERROR;
	}
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `Pinned Tcl 9.1.0 release source`, `generic/tclOODefineCmds.c`, function `TclOODefineMethodObjCmd`, lines 2271–2352. Retained reference `tcl9.1-method-source`.

```text
TclOODefineMethodObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    /*
     * Table of export modes for methods and their corresponding enum.
     */

    static const char *const exportModes[] = {
	"-export",
	"-private",
	"-unexport",
	NULL
    };
    enum ExportMode {
	MODE_EXPORT,
	MODE_PRIVATE,
	MODE_UNEXPORT
    } exportMode;

    bool isInstanceMethod = (clientData != NULL);
    Object *oPtr;
    int flags = 0;

    if (objc < 4 || objc > 5) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?option? args body");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceMethod && !oPtr->classPtr) {
	return ReportAbuse(interp);
    }
    if (objc == 5) {
	if (Tcl_GetIndexFromObj(interp, objv[2], exportModes, "export flag",
		0, &exportMode) != TCL_OK) {
	    return TCL_ERROR;
	}
	switch (exportMode) {
	case MODE_EXPORT:
	    flags = PUBLIC_METHOD;
	    break;
	case MODE_PRIVATE:
	    flags = TRUE_PRIVATE_METHOD;
	    break;
	case MODE_UNEXPORT:
	    flags = 0;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    } else {
	if (IsPrivateDefine(interp)) {
	    flags = TRUE_PRIVATE_METHOD;
	} else {
	    flags = Tcl_StringMatch(TclGetString(objv[1]), PUBLIC_PATTERN)
		    ? PUBLIC_METHOD : 0;
	}
    }

    /*
     * Create the method by using the right back-end API.
     */

    if (isInstanceMethod) {
	if (TclOONewProcInstanceMethod(interp, oPtr, flags, objv[1],
		objv[objc - 2], objv[objc - 1], NULL) == NULL) {
	    return TCL_ERROR;
	}
    } else {
	if (TclOONewProcMethod(interp, oPtr->classPtr, flags, objv[1],
		objv[objc - 2], objv[objc - 1], NULL) == NULL) {
	    return TCL_ERROR;
	}
    }
    return TCL_OK;
}

```


## Consumer bindings

- [rust/tcl-registry/src/native_tcloo_method_definition.rs](../../../../rust/tcl-registry/src/native_tcloo_method_definition.rs), `NativeTclooMethodDefinitionProtocol::layout`: Selected pure method argv/visibility semantics, independent of body or native table capability.
- [rust/tcl-vm/src/cmd_oo.rs](../../../../rust/tcl-vm/src/cmd_oo.rs), `def_method`: Consume original option objects and current method flags, or authentic caller-provider scope without a context-dependent cache grant.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `def_method`: Consume the same selected declaration/caller boundary on the runtime port.
- [rust/tcl-vm/src/cmd_oo/native_private_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_private_tests.rs), `cmd_oo::native_private_tests::method_option_layout_and_private_scope_match_original_native_controls` (linked): Compare selected C8.6/C9 original method option definition completion code/bytes and successful/internal/external outputs. Private rosters compare the exact one-element invoke result. Class caller controls compare exact results; subclass scope rejection compares completion code only, leaving bootstrap diagnostic-roster ordering independent. No pass is inferred from the binding.
- [runtime/rust/src/cmd_oo/native_private_tests.rs](../../../../runtime/rust/src/cmd_oo/native_private_tests.rs), `cmd_oo::native_private_tests::method_option_layout_and_private_scope_match_original_native_controls` (linked): Same finite original option/caller comparisons on independently selected runtime cores; no fresh native or Rust execution is implied.
- [rust/tcl-registry/src/native_tcloo_method_definition.rs](../../../../rust/tcl-registry/src/native_tcloo_method_definition.rs), `native_tcloo_method_definition::tests::method_argc_and_visibility_keep_private_separate_from_unexported` (linked): Selected release-dependent argc, option ordinal and distinct visibility recipe; a pure recipe test grants no native registration or physical method table.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-vm/tests/data/native_method_visibility/replay.py",
  "--c-root",
  "tmp",
  "--jim-root",
  "/workspace/.proofs/native-providers/jimtcl",
  "--output",
  "/tmp/native-method-visibility-reconfirmation"
]
```

Requires exact retained probe/public-header/static-library/Makefile/source-owner hashes and configured matching providers. Recompile both variants and compare full process exit/stdout/stderr; guest errors are expected rows. --verify-only checks inputs/streams without a native launch. No cache/header/refcount, arbitrary observer or Rust execution result is asserted.
