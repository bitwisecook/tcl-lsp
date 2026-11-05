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

### tcl8.4, tcl8.5

Status: `unsupported`. Channel: Original counted Tcl objects passed to Tcl_EvalObjv plus ASCII NUL-terminated Tcl_Eval scripts. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

The startup query reports this version and the actual OO command query is empty; no OO argv or private body is executed.

### tcl8.6

Status: `observed`. Channel: Original counted Tcl objects passed to Tcl_EvalObjv plus ASCII NUL-terminated Tcl_Eval scripts. Version: tcl8.6 8.6.18.

Every four-operand method form rejects with wrong-argument usage; option-free dashed formal/name controls succeed and external same-object Hidden rejects. The private worker controls reject as unavailable.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Original counted Tcl objects passed to Tcl_EvalObjv plus ASCII NUL-terminated Tcl_Eval scripts. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Original -private/-p/raw-zero option forms succeed, remain absent from the -private roster, reject external calls and return OK via the declaring method. Encoded C080 and -- reject; -export calls externally, -unexport only internally. The option-free data controls succeed.

### jim

Status: `unsupported`. Channel: ASCII NUL-terminated startup and OO-availability source passed to Jim_Eval; no Jim OO operand vector executes. Version: jim 0.84-9-g5bac7c9.

The startup query reports this version and the actual OO command query is empty; no OO argv or private body is executed.

### bigip

Status: `not-tested`.

No appliance observation for this exact method argv or private caller question.

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

## Evidence

- [rust/tcl-vm/tests/data/native_method_visibility/v1](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1)
- [rust/tcl-vm/tests/data/native_method_visibility/v2](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2)
- [rust/tcl-vm/tests/data/native_method_visibility](../../../../rust/tcl-vm/tests/data/native_method_visibility)
