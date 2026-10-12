# naming.tcloo.variable-info-option-order

Kind: `source-anchor`

## Problem statement

Declared-variable introspection needs the original option extent and exact argc/flag grammar independently of target lookup. Display-string decoding, comparison over the whole operand and a different target/flag order can disagree with the selected handler before any variable list is produced.

## Question

Which byte extent and validation order does declared-variable info use for its optional -private operand in C8.6 and C9?

## Conclusion

C8.6 accepts exactly the target operand. C9.0/C9.1 accept the target and optionally one flag; argc and the exact strcmp CString -private check precede object/class lookup. The option has no abbreviation or Index-cache lookup. A raw zero suffix is outside this strcmp view, while encoded C080 and nonzero suffix bytes participate. Retained declared variable objects are appended to the result; this recipe supplies neither target existence nor variable installation, frame, callback or completion closure.

## Scope

Pinned C8.6.18/9.0.4/9.1.0 InfoObjectVariablesCmd and InfoClassVariablesCmd source windows. Only source inspection and separately linked implementation contracts: no native launch, guest operation/error-options observation, cache/header grant or read/write/Normal claim. Other providers uninspected.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.6

Status: `inspected`. Version: 8.6.18. Build: Pinned source inspection only; no executed build association.. Channel: Inspected native declared-variable introspection handler; no guest input.. Dialect: Tcl.

argc before target; C8.6 has no option, C9 accepts only exact CString -private before target lookup.

### tcl9.0

Status: `inspected`. Version: 9.0.4. Build: Pinned source inspection only; no executed build association.. Channel: Inspected native declared-variable introspection handler; no guest input.. Dialect: Tcl.

argc before target; C8.6 has no option, C9 accepts only exact CString -private before target lookup.

### tcl9.1

Status: `inspected`. Version: 9.1.0. Build: Pinned source inspection only; no executed build association.. Channel: Inspected native declared-variable introspection handler; no guest input.. Dialect: Tcl.

argc before target; C8.6 has no option, C9 accepts only exact CString -private before target lookup.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `tcl8.6-InfoObjectVariablesCmd` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_variable_info_source/tcl8.6-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_variable_info_source/tcl8.6-source.json). SHA-256 `1f2ab0a781a94daebd07529eecac560364863c83707cac9562331d61ce263f04`. JSON pointer `/windows/0/snippet`. Pinned whole handler with argc/strcmp validation before target lookup and original declaration result operands.
- `tcl8.6-InfoClassVariablesCmd` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_variable_info_source/tcl8.6-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_variable_info_source/tcl8.6-source.json). SHA-256 `1f2ab0a781a94daebd07529eecac560364863c83707cac9562331d61ce263f04`. JSON pointer `/windows/1/snippet`. Pinned whole handler with argc/strcmp validation before target lookup and original declaration result operands.
- `tcl9.0-InfoObjectVariablesCmd` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_variable_info_source/tcl9.0-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_variable_info_source/tcl9.0-source.json). SHA-256 `baa7d12e616e768725d3bb6acd84e6dfd8f8c515b1863c32d9487d23848b02f6`. JSON pointer `/windows/0/snippet`. Pinned whole handler with argc/strcmp validation before target lookup and original declaration result operands.
- `tcl9.0-InfoClassVariablesCmd` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_variable_info_source/tcl9.0-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_variable_info_source/tcl9.0-source.json). SHA-256 `baa7d12e616e768725d3bb6acd84e6dfd8f8c515b1863c32d9487d23848b02f6`. JSON pointer `/windows/1/snippet`. Pinned whole handler with argc/strcmp validation before target lookup and original declaration result operands.
- `tcl9.1-InfoObjectVariablesCmd` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_variable_info_source/tcl9.1-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_variable_info_source/tcl9.1-source.json). SHA-256 `3a00e747c86e65b22fa48f69cebd207cee60e1ff22afa58d17e869f360148b7d`. JSON pointer `/windows/0/snippet`. Pinned whole handler with argc/strcmp validation before target lookup and original declaration result operands.
- `tcl9.1-InfoClassVariablesCmd` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_variable_info_source/tcl9.1-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_variable_info_source/tcl9.1-source.json). SHA-256 `3a00e747c86e65b22fa48f69cebd207cee60e1ff22afa58d17e869f360148b7d`. JSON pointer `/windows/1/snippet`. Pinned whole handler with argc/strcmp validation before target lookup and original declaration result operands.

## Source inspection

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOInfo.c`, function `InfoObjectVariablesCmd`, lines 728–753. Full-source SHA-256 `309603344c3aeebe95832928b473a84ebc001b3d85c8489e15675448035de02b`; snippet SHA-256 `dab27f68c49bdc1f4b47b394a55e9b231a54f8f6e37e16c5c69d811ae285993f`; retained evidence `tcl8.6-InfoObjectVariablesCmd`.

```text
InfoObjectVariablesCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const objv[])
{
    Object *oPtr;
    Tcl_Obj *variableObj, *resultObj;
    int i;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "objName");
	return TCL_ERROR;
    }
    oPtr = (Object *) Tcl_GetObjectFromObj(interp, objv[1]);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(variableObj, oPtr->variables) {
	Tcl_ListObjAppendElement(NULL, resultObj, variableObj);
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOInfo.c`, function `InfoClassVariablesCmd`, lines 1408–1433. Full-source SHA-256 `309603344c3aeebe95832928b473a84ebc001b3d85c8489e15675448035de02b`; snippet SHA-256 `760b068365fd4f7faa0e4524dbaed0f50de5fbf42928d774f83077201bab2e8e`; retained evidence `tcl8.6-InfoClassVariablesCmd`.

```text
InfoClassVariablesCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const objv[])
{
    Class *clsPtr;
    Tcl_Obj *variableObj, *resultObj;
    int i;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "className");
	return TCL_ERROR;
    }
    clsPtr = GetClassFromObj(interp, objv[1]);
    if (clsPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(variableObj, clsPtr->variables) {
	Tcl_ListObjAppendElement(NULL, resultObj, variableObj);
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOInfo.c`, function `InfoObjectVariablesCmd`, lines 877–923. Full-source SHA-256 `a1ed9fa857c89ba0cc5eea5a6f4e3be82783130823d5182c8e4a4abc5073158b`; snippet SHA-256 `11f88242b0ca9fa562da1df1a31d82f85634a7f306f43422ca4a69c77f53adb0`; retained evidence `tcl9.0-InfoObjectVariablesCmd`.

```text
InfoObjectVariablesCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const objv[])
{
    Object *oPtr;
    Tcl_Obj *resultObj;
    Tcl_Size i;
    int isPrivate = 0;

    if (objc != 2 && objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "objName ?-private?");
	return TCL_ERROR;
    }
    if (objc == 3) {
	if (strcmp("-private", TclGetString(objv[2])) != 0) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "option \"%s\" is not exactly \"-private\"",
		    TclGetString(objv[2])));
	    OO_ERROR(interp, BAD_ARG);
	    return TCL_ERROR;
	}
	isPrivate = 1;
    }
    oPtr = (Object *) Tcl_GetObjectFromObj(interp, objv[1]);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    if (isPrivate) {
	PrivateVariableMapping *privatePtr;

	FOREACH_STRUCT(privatePtr, oPtr->privateVariables) {
	    Tcl_ListObjAppendElement(NULL, resultObj, privatePtr->variableObj);
	}
    } else {
	Tcl_Obj *variableObj;

	FOREACH(variableObj, oPtr->variables) {
	    Tcl_ListObjAppendElement(NULL, resultObj, variableObj);
	}
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOInfo.c`, function `InfoClassVariablesCmd`, lines 1683–1729. Full-source SHA-256 `a1ed9fa857c89ba0cc5eea5a6f4e3be82783130823d5182c8e4a4abc5073158b`; snippet SHA-256 `b4b4ace6a2895515056741375bb0ce49b1e9fba36f31d161c2f90916b582dfdb`; retained evidence `tcl9.0-InfoClassVariablesCmd`.

```text
InfoClassVariablesCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const objv[])
{
    Class *clsPtr;
    Tcl_Obj *resultObj;
    Tcl_Size i;
    int isPrivate = 0;

    if (objc != 2 && objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "className ?-private?");
	return TCL_ERROR;
    }
    if (objc == 3) {
	if (strcmp("-private", TclGetString(objv[2])) != 0) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "option \"%s\" is not exactly \"-private\"",
		    TclGetString(objv[2])));
	    OO_ERROR(interp, BAD_ARG);
	    return TCL_ERROR;
	}
	isPrivate = 1;
    }
    clsPtr = TclOOGetClassFromObj(interp, objv[1]);
    if (clsPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    if (isPrivate) {
	PrivateVariableMapping *privatePtr;

	FOREACH_STRUCT(privatePtr, clsPtr->privateVariables) {
	    Tcl_ListObjAppendElement(NULL, resultObj, privatePtr->variableObj);
	}
    } else {
	Tcl_Obj *variableObj;

	FOREACH(variableObj, clsPtr->variables) {
	    Tcl_ListObjAppendElement(NULL, resultObj, variableObj);
	}
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInfo.c`, function `InfoObjectVariablesCmd`, lines 944–990. Full-source SHA-256 `5f46472c0039bfd89a5ed06aba1df376942136c6ddf02d5c6a0b2cf0bce2a5a9`; snippet SHA-256 `6f0e63143063b06be3dbc8feb0d8f7f0f845e88bdf0d3b9e869bbfa55dd1990a`; retained evidence `tcl9.1-InfoObjectVariablesCmd`.

```text
InfoObjectVariablesCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr;
    Tcl_Obj *resultObj;
    Tcl_Size i;
    bool isPrivate = false;

    if (objc != 2 && objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "objName ?-private?");
	return TCL_ERROR;
    }
    if (objc == 3) {
	if (strcmp("-private", TclGetString(objv[2])) != 0) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "option \"%s\" is not exactly \"-private\"",
		    TclGetString(objv[2])));
	    OO_ERROR(interp, BAD_ARG);
	    return TCL_ERROR;
	}
	isPrivate = true;
    }
    oPtr = (Object *) Tcl_GetObjectFromObj(interp, objv[1]);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    if (isPrivate) {
	PrivateVariableMapping *privatePtr;

	FOREACH_STRUCT(privatePtr, oPtr->privateVariables) {
	    Tcl_ListObjAppendElement(NULL, resultObj, privatePtr->variableObj);
	}
    } else {
	Tcl_Obj *variableObj;

	FOREACH(variableObj, oPtr->variables) {
	    Tcl_ListObjAppendElement(NULL, resultObj, variableObj);
	}
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInfo.c`, function `InfoClassVariablesCmd`, lines 1675–1721. Full-source SHA-256 `5f46472c0039bfd89a5ed06aba1df376942136c6ddf02d5c6a0b2cf0bce2a5a9`; snippet SHA-256 `dbff1bcf6ab71f390e0baf690ed69e1c789a1e8ddc484f98702f5aff51d11ef1`; retained evidence `tcl9.1-InfoClassVariablesCmd`.

```text
InfoClassVariablesCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Class *clsPtr;
    Tcl_Obj *resultObj;
    Tcl_Size i;
    bool isPrivate = false;

    if (objc != 2 && objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "className ?-private?");
	return TCL_ERROR;
    }
    if (objc == 3) {
	if (strcmp("-private", TclGetString(objv[2])) != 0) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "option \"%s\" is not exactly \"-private\"",
		    TclGetString(objv[2])));
	    OO_ERROR(interp, BAD_ARG);
	    return TCL_ERROR;
	}
	isPrivate = true;
    }
    clsPtr = TclOOGetClassFromObj(interp, objv[1]);
    if (clsPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    if (isPrivate) {
	PrivateVariableMapping *privatePtr;

	FOREACH_STRUCT(privatePtr, clsPtr->privateVariables) {
	    Tcl_ListObjAppendElement(NULL, resultObj, privatePtr->variableObj);
	}
    } else {
	Tcl_Obj *variableObj;

	FOREACH(variableObj, clsPtr->variables) {
	    Tcl_ListObjAppendElement(NULL, resultObj, variableObj);
	}
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

```


## Consumer bindings

- [rust/tcl-registry/src/native_tcloo_info.rs](../../../../rust/tcl-registry/src/native_tcloo_info.rs), `NativeTclooVariableInfoProtocol::accepts_count`: release-selected pre-target argc
- [rust/tcl-registry/src/native_tcloo_info.rs](../../../../rust/tcl-registry/src/native_tcloo_info.rs), `NativeTclooVariableInfoProtocol::private_option`: exact CString flag and diagnostic bytes without Index authority
- [rust/tcl-vm/src/cmd_oo.rs](../../../../rust/tcl-vm/src/cmd_oo.rs), `variable_info_options`: original getter and option validation before VM target lookup
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `variable_info_options`: original getter and option validation before Runtime target lookup
- [rust/tcl-registry/src/native_tcloo_info.rs](../../../../rust/tcl-registry/src/native_tcloo_info.rs), `native_tcloo_info::tests::variable_info_flags_keep_exact_cstring_and_release_arity_separate` (linked): Release argc, raw-zero versus encoded-zero/nonzero suffix and exact flag; no native launch or cache permission.
- [rust/tcl-vm/src/cmd_oo/native_info_option_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_info_option_tests.rs), `cmd_oo::native_info_option_tests::variable_info_validates_original_options_before_target_lookup` (linked): Actual selected VM C contexts validate bad flag/count before missing target; raw-zero option selects original private declaration objects on C9.
- [runtime/rust/src/cmd_oo/native_info_option_tests.rs](../../../../runtime/rust/src/cmd_oo/native_info_option_tests.rs), `cmd_oo::native_info_option_tests::variable_info_validates_original_options_before_target_lookup` (linked): Actual selected Runtime C contexts validate bad flag/count before missing target; raw-zero option selects original private declaration objects on C9.

A named test is a coverage binding, not a claim that it executed.

## Replay

Source-only LF window reproduction against the full files with exact recorded SHA; implementation selectors are independent and not claimed executed.
