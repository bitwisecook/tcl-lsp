# naming.tcloo.property-option-value-boundary

Kind: `native-observation`

## Problem statement

An option-looking script value can be confused with another property option, while quoted/abbreviated selectors, multiple property names and repeated getter options require exact width and stored-script observations. Introspection refusal cannot establish that a default property method is absent.

## Question

Which post-keyword words are property names, getter/setter script values and actual options when a script value is itself -get/-set, an option is quoted or abbreviated, names follow option-value pairs, or a getter option repeats?

## Conclusion

Recorded C9.0.4 and C9.1.0 expose oo::configurable. Their custom getter definition retains the exact script -set, while a quoted actual -set selector consumes its following setter script. The caught read returns getter_option_value; the write records setter_value. Multiple names and abbreviations retain custom <ReadProp-p> and <WriteProp-q> scripts. <ReadProp-q> yields definition not available for this kind of method, which establishes an uninspectable method kind and no absence claim. Repeated getter selection retains the final custom -set script. Missing option value, unknown/ambiguous option and invalid kind are caught with the exact retained TCL WRONGARGS/TCL LOOKUP INDEX diagnostics. Stock C8.4/C8.5/C8.6 and Jim report no original oo::configurable command and skip property controls; their result is scoped to these installations. Six process exits are zero with empty stderr. Independently pinned TclOODefinePropertyCmd source excerpts show i+2 value consumption, i advancement by two and replacement of local getter/setter selections before installation; source inspection is an explanatory association, not another execution. BIG-IP is not tested. These original ASCII public outcomes supply no Native source-carrier, original producer/word geometry, installed method/header/cache/frame identity or edit authority.

## Scope

Original fixed ASCII LF CLI source, separate fresh process per provider, exact configurable-command discovery, stored info-class definitions and caught configurable reads/writes/declaration failures. Public case labels, whole definitions/results and exact errorCode lists are retained. The custom method scripts are inspectable; failure to inspect the default q getter says nothing about absence. Native input has no raw/counting NUL, non-ASCII, continuation or control escape. Three named Rust source bindings are independently selected layout/geometry coverage only: their dynamic/Unicode/opaque/raw-zero cases are not measured by this ASCII native program and assert no current installed method or entered frame. Complete executable/source/library/header/build associations are checked independently; excerpts do not prove compiled-source/object equivalence.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Exact CLI executable SHA-256 f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a; every required artifact independently rechecked and retained. No fresh SDK build or compiler version recorded.. Channel: Original fixed ASCII LF source-file CLI; stock configurable availability, stored info class definition scripts and caught configure calls. No original Native source-carrier/frame/header or BIG-IP authority is measured.. Dialect: C Tcl stock CLI.

Actual 8.4.20 reports configurable_command0 and UNAVAILABLE reason no_original_oo_configurable_command. All property-control branches are skipped in this captured installation; no global provider/package impossibility follows. Process exits zero with empty stderr.

### tcl8.5

Status: `unsupported`. Version: 8.5.19. Build: Exact CLI executable SHA-256 e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a; every required artifact independently rechecked and retained. No fresh SDK build or compiler version recorded.. Channel: Original fixed ASCII LF source-file CLI; stock configurable availability, stored info class definition scripts and caught configure calls. No original Native source-carrier/frame/header or BIG-IP authority is measured.. Dialect: C Tcl stock CLI.

Actual 8.5.19 reports configurable_command0 and UNAVAILABLE reason no_original_oo_configurable_command. All property-control branches are skipped in this captured installation; no global provider/package impossibility follows. Process exits zero with empty stderr.

### tcl8.6

Status: `unsupported`. Version: 8.6.18. Build: Exact CLI executable SHA-256 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff; every required artifact independently rechecked and retained. No fresh SDK build or compiler version recorded.. Channel: Original fixed ASCII LF source-file CLI; stock configurable availability, stored info class definition scripts and caught configure calls. No original Native source-carrier/frame/header or BIG-IP authority is measured.. Dialect: C Tcl stock CLI.

Actual 8.6.18 reports configurable_command0 and UNAVAILABLE reason no_original_oo_configurable_command. All property-control branches are skipped in this captured installation; no global provider/package impossibility follows. Process exits zero with empty stderr.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Exact CLI executable SHA-256 f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1; every required artifact independently rechecked and retained. No fresh SDK build or compiler version recorded.. Channel: Original fixed ASCII LF source-file CLI; stock configurable availability, stored info class definition scripts and caught configure calls. No original Native source-carrier/frame/header or BIG-IP authority is measured.. Dialect: C Tcl stock CLI.

Actual 9.0.4 exposes oo::configurable. Exact measured rows: DEFINITION label option_like_getter method <ReadProp-p> code 0 definition {{} -set}; DEFINITION label option_like_getter method <WriteProp-p> code 0 definition {value {set ::property_width_seen $value}}; CALL label option_like_getter operation read code 0 result getter_option_value; CALL label option_like_getter operation write code 0 result {} seen setter_value; DEFINITION label multiple_names_abbreviations method <ReadProp-p> code 0 definition {{} -set}; DEFINITION label multiple_names_abbreviations method <ReadProp-q> code 1 definition {definition not available for this kind of method}; DEFINITION label multiple_names_abbreviations method <WriteProp-q> code 0 definition {value {set ::property_width_q $value}}; DEFINITION label repeated_getter method <ReadProp-p> code 0 definition {{} -set}; DEFINITION label repeated_getter method <WriteProp-p> code 0 definition {value {set ::property_width_repeat_seen $value}}; DECL label missing_value code 1 result {missing body to go with -get option} errorCode {TCL WRONGARGS}; DECL label unknown_option code 1 result {bad option "-unknown": must be -get, -kind, or -set} errorCode {TCL LOOKUP INDEX option -unknown}; DECL label ambiguous_option code 1 result {ambiguous option "-": must be -get, -kind, or -set} errorCode {TCL LOOKUP INDEX option -}; DECL label invalid_kind code 1 result {ambiguous kind "r": must be readable, readwrite, or writable} errorCode {TCL LOOKUP INDEX kind r}. Option-looking -set remains the getter script and repeated getter retains its final script; quoted setter and abbreviated/multiple-name cases preserve their exact values. The default q getter definition error establishes uninspectability, not absence. Process exits zero with empty stderr.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Exact CLI executable SHA-256 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d; every required artifact independently rechecked and retained. No fresh SDK build or compiler version recorded.. Channel: Original fixed ASCII LF source-file CLI; stock configurable availability, stored info class definition scripts and caught configure calls. No original Native source-carrier/frame/header or BIG-IP authority is measured.. Dialect: C Tcl stock CLI.

Actual 9.1.0 exposes oo::configurable. Exact measured rows: DEFINITION label option_like_getter method <ReadProp-p> code 0 definition {{} -set}; DEFINITION label option_like_getter method <WriteProp-p> code 0 definition {value {set ::property_width_seen $value}}; CALL label option_like_getter operation read code 0 result getter_option_value; CALL label option_like_getter operation write code 0 result {} seen setter_value; DEFINITION label multiple_names_abbreviations method <ReadProp-p> code 0 definition {{} -set}; DEFINITION label multiple_names_abbreviations method <ReadProp-q> code 1 definition {definition not available for this kind of method}; DEFINITION label multiple_names_abbreviations method <WriteProp-q> code 0 definition {value {set ::property_width_q $value}}; DEFINITION label repeated_getter method <ReadProp-p> code 0 definition {{} -set}; DEFINITION label repeated_getter method <WriteProp-p> code 0 definition {value {set ::property_width_repeat_seen $value}}; DECL label missing_value code 1 result {missing body to go with -get option} errorCode {TCL WRONGARGS}; DECL label unknown_option code 1 result {bad option "-unknown": must be -get, -kind, or -set} errorCode {TCL LOOKUP INDEX option -unknown}; DECL label ambiguous_option code 1 result {ambiguous option "-": must be -get, -kind, or -set} errorCode {TCL LOOKUP INDEX option -}; DECL label invalid_kind code 1 result {ambiguous kind "r": must be readable, readwrite, or writable} errorCode {TCL LOOKUP INDEX kind r}. Option-looking -set remains the getter script and repeated getter retains its final script; quoted setter and abbreviated/multiple-name cases preserve their exact values. The default q getter definition error establishes uninspectability, not absence. Process exits zero with empty stderr.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Exact CLI executable SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806; every required artifact independently rechecked and retained. No fresh SDK build or compiler version recorded.. Channel: Original fixed ASCII LF source-file CLI; stock configurable availability, stored info class definition scripts and caught configure calls. No original Native source-carrier/frame/header or BIG-IP authority is measured.. Dialect: Jim Tcl.

Actual 0.84-9-g5bac7c9 reports configurable_command0 and UNAVAILABLE reason no_original_oo_configurable_command. All property-control branches are skipped in this captured installation; no global provider/package impossibility follows. Process exits zero with empty stderr.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: BIG-IP.

No appliance execution answers this exact original input question; stock C and Jim results supply no hosted result.

## Exact evidence

- `property281-probe.tcl` (input): [rust/tcl-registry/tests/data/native_property_option_width281/probe.tcl](../../../../rust/tcl-registry/tests/data/native_property_option_width281/probe.tcl). SHA-256 `13d2f4227bc951e627b081da1f5212a1dc4777fce27d273f36e8001fa5cf5d43`. Exact original ASCII LF source file executed by all six providers.
- `property281-request.json` (input): [rust/tcl-registry/tests/data/native_property_option_width281/request.json](../../../../rust/tcl-registry/tests/data/native_property_option_width281/request.json). SHA-256 `9ddbb3accf726bc22f6fb9073d9dbd601ee066ea108b44a2007fb5072962c43c`. Exact original authored question/provider/source request and explicit authority boundary.
- `property281-capture.py` (input): [rust/tcl-registry/tests/data/native_property_option_width281/capture.py](../../../../rust/tcl-registry/tests/data/native_property_option_width281/capture.py). SHA-256 `1d8670eabbc6e8bba2219360a76d0a60a5e02d424fac1303ae4d1209fe9fd26e`. Exact Root-executed native launcher; original commands/environment/executable/artifact checks, no new run.
- `property281-8.4.20-receipt.json` (provider): [rust/tcl-registry/tests/data/native_property_option_width281/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_property_option_width281/8.4.20/receipt.json). SHA-256 `6389458bcac7bcac8a5718d3f775e0ba41850991c5a855f484f8c646c8927ba5`. Exact actual process/version/environment and complete executable/required-artifact SHA associations.
- `property281-8.4.20-stdout` (observation): [rust/tcl-registry/tests/data/native_property_option_width281/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_property_option_width281/8.4.20/stdout). SHA-256 `4f825f5ae737e7ac144703f9d4aeeca9deb3c3559b998aca273e2317652ace2c`. Whole original stdout; exact caught codes, body/query outcomes and value spellings retained.
- `property281-8.4.20-stderr` (observation): [rust/tcl-registry/tests/data/native_property_option_width281/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_property_option_width281/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.
- `property281-8.5.19-receipt.json` (provider): [rust/tcl-registry/tests/data/native_property_option_width281/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_property_option_width281/8.5.19/receipt.json). SHA-256 `50b4c19e096f1c0f7805ac44c93b3c677cc4763f38bf9a601d90aca8094a4092`. Exact actual process/version/environment and complete executable/required-artifact SHA associations.
- `property281-8.5.19-stdout` (observation): [rust/tcl-registry/tests/data/native_property_option_width281/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_property_option_width281/8.5.19/stdout). SHA-256 `fb212c99749bae4801ab00b2a5328d4ea91df1711062249f78bf80828eae008a`. Whole original stdout; exact caught codes, body/query outcomes and value spellings retained.
- `property281-8.5.19-stderr` (observation): [rust/tcl-registry/tests/data/native_property_option_width281/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_property_option_width281/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.
- `property281-8.6.18-receipt.json` (provider): [rust/tcl-registry/tests/data/native_property_option_width281/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_property_option_width281/8.6.18/receipt.json). SHA-256 `b08e53527cd0ad641b154af104a501cff823ea1855ef1a3029d7d9b6b3a0c71f`. Exact actual process/version/environment and complete executable/required-artifact SHA associations.
- `property281-8.6.18-stdout` (observation): [rust/tcl-registry/tests/data/native_property_option_width281/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_property_option_width281/8.6.18/stdout). SHA-256 `89308784c7343001cca5ae7e39c301a3cfd60bd026301208bafb38c22d6d0d5c`. Whole original stdout; exact caught codes, body/query outcomes and value spellings retained.
- `property281-8.6.18-stderr` (observation): [rust/tcl-registry/tests/data/native_property_option_width281/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_property_option_width281/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.
- `property281-9.0.4-receipt.json` (provider): [rust/tcl-registry/tests/data/native_property_option_width281/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_property_option_width281/9.0.4/receipt.json). SHA-256 `791a0ff548f63c2f028b97f34b8efaffe51d4cb8c69c7c13c219e74707d2f3e5`. Exact actual process/version/environment and complete executable/required-artifact SHA associations.
- `property281-9.0.4-stdout` (observation): [rust/tcl-registry/tests/data/native_property_option_width281/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_property_option_width281/9.0.4/stdout). SHA-256 `2979c42df42261c65eb2fc8ce0f24e043e4e00994999bae4b6c8fd0961ca494a`. Whole original stdout; exact caught codes, body/query outcomes and value spellings retained.
- `property281-9.0.4-stderr` (observation): [rust/tcl-registry/tests/data/native_property_option_width281/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_property_option_width281/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.
- `property281-9.1.0-receipt.json` (provider): [rust/tcl-registry/tests/data/native_property_option_width281/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_property_option_width281/9.1.0/receipt.json). SHA-256 `f364be5297098c457f9ec988477af92dfddbc4d2adbce2d4669659e8f0b1ceab`. Exact actual process/version/environment and complete executable/required-artifact SHA associations.
- `property281-9.1.0-stdout` (observation): [rust/tcl-registry/tests/data/native_property_option_width281/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_property_option_width281/9.1.0/stdout). SHA-256 `9e2c40331686339e5744e8f2b122a5b8c717ca2c94c07e3f536fe185fcb42bec`. Whole original stdout; exact caught codes, body/query outcomes and value spellings retained.
- `property281-9.1.0-stderr` (observation): [rust/tcl-registry/tests/data/native_property_option_width281/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_property_option_width281/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.
- `property281-jim-receipt.json` (provider): [rust/tcl-registry/tests/data/native_property_option_width281/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_property_option_width281/jim/receipt.json). SHA-256 `3d773e63e1ee521560aa12e22654aa321acc93e85247a93da0f0d1a571c013ee`. Exact actual process/version/environment and complete executable/required-artifact SHA associations.
- `property281-jim-stdout` (observation): [rust/tcl-registry/tests/data/native_property_option_width281/jim/stdout](../../../../rust/tcl-registry/tests/data/native_property_option_width281/jim/stdout). SHA-256 `b767eb7700f102d8994f16145b7258053fa99e06ddf6638814519431a8edc08e`. Whole original stdout; exact caught codes, body/query outcomes and value spellings retained.
- `property281-jim-stderr` (observation): [rust/tcl-registry/tests/data/native_property_option_width281/jim/stderr](../../../../rust/tcl-registry/tests/data/native_property_option_width281/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.
- `property281-source-anchors-9.0.4-TclOODefinePropertyCmd.c` (source-anchor): [rust/tcl-registry/tests/data/native_property_option_width281/source-anchors/9.0.4-TclOODefinePropertyCmd.c](../../../../rust/tcl-registry/tests/data/native_property_option_width281/source-anchors/9.0.4-TclOODefinePropertyCmd.c). SHA-256 `b3a7714ca4ddf001ec9ea9e22d2ff9cb0943e122a2acf38461a7c95e1f61f769`. Exact independently pinned source excerpt for property option-value width and final selected scripts; inspection is separate from native execution.
- `property281-source-anchors-9.1.0-TclOODefinePropertyCmd.c` (source-anchor): [rust/tcl-registry/tests/data/native_property_option_width281/source-anchors/9.1.0-TclOODefinePropertyCmd.c](../../../../rust/tcl-registry/tests/data/native_property_option_width281/source-anchors/9.1.0-TclOODefinePropertyCmd.c). SHA-256 `54aa701dd1ed7eb6b5c3305ed820feab4f86c2c14f4cdbabc895d41263832479`. Exact independently pinned source excerpt for property option-value width and final selected scripts; inspection is separate from native execution.

## Source inspection

tcl9.0 9.0.4, revision `pinned release source 9.0.4`, `generic/tclOOProp.c`, function `TclOODefinePropertyCmd`, lines 1030–1128. Full-source SHA-256 `43f16f2b596af56b27320ff5e735c3073e081dbf183b05b5aa785ff7a2b57789`; snippet SHA-256 `b3a7714ca4ddf001ec9ea9e22d2ff9cb0943e122a2acf38461a7c95e1f61f769`; retained evidence `property281-source-anchors-9.0.4-TclOODefinePropertyCmd.c`.

```text
TclOODefinePropertyCmd(
    void *useInstance,		/* NULL for class, non-NULL for object. */
    Tcl_Interp *interp,		/* For error reporting and lookup. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Arguments. */
{
    int i;
    static const char *const options[] = {
	"-get", "-kind", "-set", NULL
    };
    enum Options {
	OPT_GET, OPT_KIND, OPT_SET
    };
    static const char *const kinds[] = {
	"readable", "readwrite", "writable", NULL
    };
    enum Kinds {
	KIND_RO, KIND_RW, KIND_WO
    };
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);

    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!useInstance && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return TCL_ERROR;
    }

    for (i = 1; i < objc; i++) {
	Tcl_Obj *propObj = objv[i], *nextObj, *argObj, *hyphenated;
	Tcl_Obj *getterScript = NULL, *setterScript = NULL;

	/*
	 * Parse the extra options for the property.
	 */

	int kind = KIND_RW;
	while (i + 1 < objc) {
	    int option;

	    nextObj = objv[i + 1];
	    if (TclGetString(nextObj)[0] != '-') {
		break;
	    }
	    if (Tcl_GetIndexFromObj(interp, nextObj, options, "option", 0,
		    &option) != TCL_OK) {
		return TCL_ERROR;
	    }
	    if (i + 2 >= objc) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"missing %s to go with %s option",
			(option == OPT_KIND ? "kind value" : "body"),
			options[option]));
		Tcl_SetErrorCode(interp, "TCL", "WRONGARGS", (char *)NULL);
		return TCL_ERROR;
	    }
	    argObj = objv[i + 2];
	    i += 2;
	    switch (option) {
	    case OPT_GET:
		getterScript = argObj;
		break;
	    case OPT_SET:
		setterScript = argObj;
		break;
	    case OPT_KIND:
		if (Tcl_GetIndexFromObj(interp, argObj, kinds, "kind", 0,
			&kind) != TCL_OK) {
		    return TCL_ERROR;
		}
		break;
	    default:
		TCL_UNREACHABLE();
	    }
	}

	/*
	 * Install the property. Note that InstallStdPropertyImpls
	 * validates the property name as well.
	 */

	if (InstallStdPropertyImpls(useInstance, interp, propObj,
		kind != KIND_WO && getterScript == NULL,
		kind != KIND_RO && setterScript == NULL) != TCL_OK) {
	    return TCL_ERROR;
	}

	hyphenated = Tcl_ObjPrintf("-%s", TclGetString(propObj));
	if (useInstance) {
	    TclOORegisterInstanceProperty(oPtr, hyphenated,
		    kind != KIND_WO, kind != KIND_RO);
	} else {
	    TclOORegisterProperty(oPtr->classPtr, hyphenated,
		    kind != KIND_WO, kind != KIND_RO);
	}
	Tcl_BounceRefCount(hyphenated);

```

tcl9.1 9.1.0, revision `pinned release source 9.1.0`, `generic/tclOOProp.c`, function `TclOODefinePropertyCmd`, lines 1138–1236. Full-source SHA-256 `c791423ae9b49c877bea8da8a7c2b74b40520e600863341e8d12a63b10c2aaaf`; snippet SHA-256 `54aa701dd1ed7eb6b5c3305ed820feab4f86c2c14f4cdbabc895d41263832479`; retained evidence `property281-source-anchors-9.1.0-TclOODefinePropertyCmd.c`.

```text
TclOODefinePropertyCmd(
    void *useInstance,		/* NULL for class, non-NULL for object. */
    Tcl_Interp *interp,		/* For error reporting and lookup. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Arguments. */
{
    Tcl_Size i;
    static const char *const options[] = {
	"-get", "-kind", "-set", NULL
    };
    enum Options {
	OPT_GET, OPT_KIND, OPT_SET
    };
    static const char *const kinds[] = {
	"readable", "readwrite", "writable", NULL
    };
    enum Kinds {
	KIND_RO, KIND_RW, KIND_WO
    };
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);

    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!useInstance && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return TCL_ERROR;
    }

    for (i = 1; i < objc; i++) {
	Tcl_Obj *propObj = objv[i], *nextObj, *argObj, *hyphenated;
	Tcl_Obj *getterScript = NULL, *setterScript = NULL;

	/*
	 * Parse the extra options for the property.
	 */

	int kind = KIND_RW;
	while (i + 1 < objc) {
	    int option;

	    nextObj = objv[i + 1];
	    if (TclGetString(nextObj)[0] != '-') {
		break;
	    }
	    if (Tcl_GetIndexFromObj(interp, nextObj, options, "option", 0,
		    &option) != TCL_OK) {
		return TCL_ERROR;
	    }
	    if (i + 2 >= objc) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"missing %s to go with %s option",
			(option == OPT_KIND ? "kind value" : "body"),
			options[option]));
		Tcl_SetErrorCode(interp, "TCL", "WRONGARGS", (char *)NULL);
		return TCL_ERROR;
	    }
	    argObj = objv[i + 2];
	    i += 2;
	    switch (option) {
	    case OPT_GET:
		getterScript = argObj;
		break;
	    case OPT_SET:
		setterScript = argObj;
		break;
	    case OPT_KIND:
		if (Tcl_GetIndexFromObj(interp, argObj, kinds, "kind", 0,
			&kind) != TCL_OK) {
		    return TCL_ERROR;
		}
		break;
	    default:
		TCL_UNREACHABLE();
	    }
	}

	/*
	 * Install the property. Note that InstallStdPropertyImpls
	 * validates the property name as well.
	 */

	if (InstallStdPropertyImpls(useInstance, interp, propObj,
		kind != KIND_WO && getterScript == NULL,
		kind != KIND_RO && setterScript == NULL) != TCL_OK) {
	    return TCL_ERROR;
	}

	hyphenated = Tcl_ObjPrintf("-%s", TclGetString(propObj));
	if (useInstance) {
	    TclOORegisterInstanceProperty(oPtr, hyphenated,
		    kind != KIND_WO, kind != KIND_RO);
	} else {
	    TclOORegisterProperty(oPtr->classPtr, hyphenated,
		    kind != KIND_WO, kind != KIND_RO);
	}
	Tcl_BounceRefCount(hyphenated);

```


## Consumer bindings

- [rust/tcl-registry/src/definer.rs](../../../../rust/tcl-registry/src/definer.rs), `DefinitionBodyGrammar::source_property_declarations_where`: Single byte-selector/argv-width layout behind counted declaration metadata and Name/Body/block projections; final repeated getter/setter per property.
- [rust/tcl-compiler/src/registry_invocation/source_scoped_body.rs](../../../../rust/tcl-compiler/src/registry_invocation/source_scoped_body.rs), `OriginalSourceDefinitionMemberRegion::script_bodies`: Genuine selected definition carrier + complete original argv maps Registry ordinals to whole physical source containers; ordinary getter/setter bodies clear definition vocabulary.
- [rust/tcl-registry/src/definer.rs](../../../../rust/tcl-registry/src/definer.rs), `definer::tests::original_property_role_layout_consumes_option_values_once` (linked): Independent Rust source coverage, not an executed provider result: C90/C91 selected source grammar ordinals for option-like body values, multiple property names, abbreviations and final repeated custom getter agree with counted declaration metadata.
- [rust/tcl-registry/src/definer.rs](../../../../rust/tcl-registry/src/definer.rs), `definer::tests::original_property_role_layout_keeps_unknown_bodies_separate_from_selectors` (linked): Independent Rust source coverage, not an executed provider result: Implementation-only source knowledge controls: Dynamic body values skipped without fake literals; unknown/missing/unsupported/ambiguous selectors and kinds, expansion/opaque argv decline; C8.6 and Jim property vocabulary unavailable. Its raw-zero option-purpose control is a Rust implementation binding, not an observation of the ASCII native probe.
- [rust/tcl-compiler/src/registry_invocation/source_scoped_body.rs](../../../../rust/tcl-compiler/src/registry_invocation/source_scoped_body.rs), `registry_invocation::source_scoped_body::script_purpose_tests::original_property_member_bodies_skip_option_like_values_and_quoted_options` (linked): Independent Rust source coverage, not an executed provider result: Implementation-only genuine source geometry binds ordinals2/4, quoted actual option exclusion, Unicode spans and repeated final getter to complete original argv, while ordinary bodies do not inherit definition vocabulary.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original commands, environment overrides, exact sources/whole streams and required executable/SDK/source pins remain unchanged. Other inherited environment variables are unrecorded. The actual launcher depends on retained earlier native receipts and external SDK paths and refuses existing output directories. Its retained bytes and scratch paths do not promise portable replay; re-running creates new observations. No native or Rust process is launched by publication.
