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

Status: `unsupported`. Channel: Original fixed ASCII LF source-file CLI; stock configurable availability, stored info class definition scripts and caught configure calls. No original Native source-carrier/frame/header or BIG-IP authority is measured. Version: tcl8.4 8.4.20.

Actual 8.4.20 reports configurable_command0 and UNAVAILABLE reason no_original_oo_configurable_command. All property-control branches are skipped in this captured installation; no global provider/package impossibility follows. Process exits zero with empty stderr.

### tcl8.5

Status: `unsupported`. Channel: Original fixed ASCII LF source-file CLI; stock configurable availability, stored info class definition scripts and caught configure calls. No original Native source-carrier/frame/header or BIG-IP authority is measured. Version: tcl8.5 8.5.19.

Actual 8.5.19 reports configurable_command0 and UNAVAILABLE reason no_original_oo_configurable_command. All property-control branches are skipped in this captured installation; no global provider/package impossibility follows. Process exits zero with empty stderr.

### tcl8.6

Status: `unsupported`. Channel: Original fixed ASCII LF source-file CLI; stock configurable availability, stored info class definition scripts and caught configure calls. No original Native source-carrier/frame/header or BIG-IP authority is measured. Version: tcl8.6 8.6.18.

Actual 8.6.18 reports configurable_command0 and UNAVAILABLE reason no_original_oo_configurable_command. All property-control branches are skipped in this captured installation; no global provider/package impossibility follows. Process exits zero with empty stderr.

### tcl9.0

Status: `observed`. Channel: Original fixed ASCII LF source-file CLI; stock configurable availability, stored info class definition scripts and caught configure calls. No original Native source-carrier/frame/header or BIG-IP authority is measured. Version: tcl9.0 9.0.4.

Actual 9.0.4 exposes oo::configurable. Exact measured rows: DEFINITION label option_like_getter method <ReadProp-p> code 0 definition {{} -set}; DEFINITION label option_like_getter method <WriteProp-p> code 0 definition {value {set ::property_width_seen $value}}; CALL label option_like_getter operation read code 0 result getter_option_value; CALL label option_like_getter operation write code 0 result {} seen setter_value; DEFINITION label multiple_names_abbreviations method <ReadProp-p> code 0 definition {{} -set}; DEFINITION label multiple_names_abbreviations method <ReadProp-q> code 1 definition {definition not available for this kind of method}; DEFINITION label multiple_names_abbreviations method <WriteProp-q> code 0 definition {value {set ::property_width_q $value}}; DEFINITION label repeated_getter method <ReadProp-p> code 0 definition {{} -set}; DEFINITION label repeated_getter method <WriteProp-p> code 0 definition {value {set ::property_width_repeat_seen $value}}; DECL label missing_value code 1 result {missing body to go with -get option} errorCode {TCL WRONGARGS}; DECL label unknown_option code 1 result {bad option "-unknown": must be -get, -kind, or -set} errorCode {TCL LOOKUP INDEX option -unknown}; DECL label ambiguous_option code 1 result {ambiguous option "-": must be -get, -kind, or -set} errorCode {TCL LOOKUP INDEX option -}; DECL label invalid_kind code 1 result {ambiguous kind "r": must be readable, readwrite, or writable} errorCode {TCL LOOKUP INDEX kind r}. Option-looking -set remains the getter script and repeated getter retains its final script; quoted setter and abbreviated/multiple-name cases preserve their exact values. The default q getter definition error establishes uninspectability, not absence. Process exits zero with empty stderr.

### tcl9.1

Status: `observed`. Channel: Original fixed ASCII LF source-file CLI; stock configurable availability, stored info class definition scripts and caught configure calls. No original Native source-carrier/frame/header or BIG-IP authority is measured. Version: tcl9.1 9.1.0.

Actual 9.1.0 exposes oo::configurable. Exact measured rows: DEFINITION label option_like_getter method <ReadProp-p> code 0 definition {{} -set}; DEFINITION label option_like_getter method <WriteProp-p> code 0 definition {value {set ::property_width_seen $value}}; CALL label option_like_getter operation read code 0 result getter_option_value; CALL label option_like_getter operation write code 0 result {} seen setter_value; DEFINITION label multiple_names_abbreviations method <ReadProp-p> code 0 definition {{} -set}; DEFINITION label multiple_names_abbreviations method <ReadProp-q> code 1 definition {definition not available for this kind of method}; DEFINITION label multiple_names_abbreviations method <WriteProp-q> code 0 definition {value {set ::property_width_q $value}}; DEFINITION label repeated_getter method <ReadProp-p> code 0 definition {{} -set}; DEFINITION label repeated_getter method <WriteProp-p> code 0 definition {value {set ::property_width_repeat_seen $value}}; DECL label missing_value code 1 result {missing body to go with -get option} errorCode {TCL WRONGARGS}; DECL label unknown_option code 1 result {bad option "-unknown": must be -get, -kind, or -set} errorCode {TCL LOOKUP INDEX option -unknown}; DECL label ambiguous_option code 1 result {ambiguous option "-": must be -get, -kind, or -set} errorCode {TCL LOOKUP INDEX option -}; DECL label invalid_kind code 1 result {ambiguous kind "r": must be readable, readwrite, or writable} errorCode {TCL LOOKUP INDEX kind r}. Option-looking -set remains the getter script and repeated getter retains its final script; quoted setter and abbreviated/multiple-name cases preserve their exact values. The default q getter definition error establishes uninspectability, not absence. Process exits zero with empty stderr.

### jim

Status: `unsupported`. Channel: Original fixed ASCII LF source-file CLI; stock configurable availability, stored info class definition scripts and caught configure calls. No original Native source-carrier/frame/header or BIG-IP authority is measured. Version: jim 0.84-9-g5bac7c9.

Actual 0.84-9-g5bac7c9 reports configurable_command0 and UNAVAILABLE reason no_original_oo_configurable_command. All property-control branches are skipped in this captured installation; no global provider/package impossibility follows. Process exits zero with empty stderr.

### bigip

Status: `not-tested`.

No appliance execution answers this exact original input question; stock C and Jim results supply no hosted result.

## Source inspection

tcl9.0 9.0.4, revision `pinned release source 9.0.4`, `generic/tclOOProp.c`, function `TclOODefinePropertyCmd`, lines 1030–1128. Retained reference `property281-source-anchors-9.0.4-TclOODefinePropertyCmd.c`.

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

tcl9.1 9.1.0, revision `pinned release source 9.1.0`, `generic/tclOOProp.c`, function `TclOODefinePropertyCmd`, lines 1138–1236. Retained reference `property281-source-anchors-9.1.0-TclOODefinePropertyCmd.c`.

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

## Evidence

- [rust/tcl-registry/tests/data/native_property_option_width281](../../../../rust/tcl-registry/tests/data/native_property_option_width281)
- [rust/tcl-registry/tests/data/native_property_option_width281/8.4.20](../../../../rust/tcl-registry/tests/data/native_property_option_width281/8.4.20)
- [rust/tcl-registry/tests/data/native_property_option_width281/8.5.19](../../../../rust/tcl-registry/tests/data/native_property_option_width281/8.5.19)
- [rust/tcl-registry/tests/data/native_property_option_width281/8.6.18](../../../../rust/tcl-registry/tests/data/native_property_option_width281/8.6.18)
- [rust/tcl-registry/tests/data/native_property_option_width281/9.0.4](../../../../rust/tcl-registry/tests/data/native_property_option_width281/9.0.4)
- [rust/tcl-registry/tests/data/native_property_option_width281/9.1.0](../../../../rust/tcl-registry/tests/data/native_property_option_width281/9.1.0)
- [rust/tcl-registry/tests/data/native_property_option_width281/jim](../../../../rust/tcl-registry/tests/data/native_property_option_width281/jim)
- [rust/tcl-registry/tests/data/native_property_option_width281/source-anchors](../../../../rust/tcl-registry/tests/data/native_property_option_width281/source-anchors)
