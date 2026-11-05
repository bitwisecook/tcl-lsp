# naming.property.original-c9-declaration-validity

Kind: `implementation-contract`

## Problem statement

Source declaration validity is needed for both C9 releases, while the physical property-cache owner is modelled only for C9.1. Using the cache selector to validate a C9.0 declaration conflates these separate purposes.

## Question

Can C9.0 and C9.1 property names use one selected pure declaration-validity kernel without acquiring C9.1 physical lookup or cache authority?

## Conclusion

A pure C9 release-selected facade shares the existing counted list-scan and CString validation kernel with the physical C9.1 property owner. C9.0 can validate declaration fields without issuing a physical property lookup recipe. The facade returns only name validity or the native byte diagnostic.

## Scope

Rust Registry implementation contract for actual C Tcl 9.0/9.1 declaration input bytes. Initial dash, native list quoting, namespace separators and parentheses use the retained existing validation algorithm; release-unavailable requests decline. Counted name and CString validation are distinct. Physical property headers, tables, lookup/cache generations, successful installation, property execution and guest observations are excluded. The named Rust selector is unverified without an attached execution receipt.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

No guest execution of this Rust implementation question is claimed.

## Source inspection

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source; full file digest identifies the source bytes`, `generic/tclOOProp.c`, function `InstallStdPropertyImpls`, lines 968–1029. Retained reference `c-source`.

```text
InstallStdPropertyImpls(
    void *useInstance,
    Tcl_Interp *interp,
    Tcl_Obj *propName,
    int readable,
    int writable)
{
    const char *name, *reason;
    Tcl_Size len;
    char flag = TCL_DONT_QUOTE_HASH;

    /*
     * Validate the property name. Note that just calling TclScanElement() is
     * cheaper than actually formatting a list and comparing the string
     * version of that with the original, as TclScanElement() is one of the
     * core parts of doing that; this skips a whole load of irrelevant memory
     * allocations!
     */

    name = Tcl_GetStringFromObj(propName, &len);
    if (Tcl_StringMatch(name, "-*")) {
	reason = "must not begin with -";
	goto badProp;
    }
    if (TclScanElement(name, len, &flag) != len) {
	reason = "must be a simple word";
	goto badProp;
    }
    if (Tcl_StringMatch(name, "*::*")) {
	reason = "must not contain namespace separators";
	goto badProp;
    }
    if (Tcl_StringMatch(name, "*[()]*")) {
	reason = "must not contain parentheses";
	goto badProp;
    }

    /*
     * Install the implementations... if asked to do so.
     */

    if (useInstance) {
	Tcl_Object object = TclOOGetDefineCmdContext(interp);
	if (!object) {
	    return TCL_ERROR;
	}
	ImplementObjectProperty(object, propName, readable, writable);
    } else {
	Tcl_Class cls = (Tcl_Class) TclOOGetClassDefineCmdContext(interp);
	if (!cls) {
	    return TCL_ERROR;
	}
	ImplementClassProperty(cls, propName, readable, writable);
    }
    return TCL_OK;

  badProp:
    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
	    "bad property name \"%s\": %s", name, reason));
    OO_ERROR(interp, PROPERTY_FORMAT);
    return TCL_ERROR;
}

```

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source; full file digest identifies the source bytes`, `generic/tclOOProp.c`, function `InstallStdPropertyImpls`, lines 1077–1138. Retained reference `c-source`.

```text
InstallStdPropertyImpls(
    void *useInstance,
    Tcl_Interp *interp,
    Tcl_Obj *propName,
    bool readable,
    bool writable)
{
    const char *name, *reason;
    Tcl_Size len;
    char flag = TCL_DONT_QUOTE_HASH;

    /*
     * Validate the property name. Note that just calling TclScanElement() is
     * cheaper than actually formatting a list and comparing the string
     * version of that with the original, as TclScanElement() is one of the
     * core parts of doing that; this skips a whole load of irrelevant memory
     * allocations!
     */

    name = Tcl_GetStringFromObj(propName, &len);
    if (Tcl_StringMatch(name, "-*")) {
	reason = "must not begin with -";
	goto badProp;
    }
    if (TclScanElement(name, len, &flag) != len) {
	reason = "must be a simple word";
	goto badProp;
    }
    if (Tcl_StringMatch(name, "*::*")) {
	reason = "must not contain namespace separators";
	goto badProp;
    }
    if (Tcl_StringMatch(name, "*[()]*")) {
	reason = "must not contain parentheses";
	goto badProp;
    }

    /*
     * Install the implementations... if asked to do so.
     */

    if (useInstance) {
	Tcl_Object object = TclOOGetDefineCmdContext(interp);
	if (!object) {
	    return TCL_ERROR;
	}
	ImplementObjectProperty(object, propName, readable, writable);
    } else {
	Tcl_Class cls = (Tcl_Class) TclOOGetClassDefineCmdContext(interp);
	if (!cls) {
	    return TCL_ERROR;
	}
	ImplementClassProperty(cls, propName, readable, writable);
    }
    return TCL_OK;

  badProp:
    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
	    "bad property name \"%s\": %s", name, reason));
    OO_ERROR(interp, PROPERTY_FORMAT);
    return TCL_ERROR;
}

```

## Consumer bindings

- [rust/tcl-registry/src/native_property_lookup.rs](../../../../rust/tcl-registry/src/native_property_lookup.rs), `native_property_declaration_validity`: Selected pure declaration validity without physical cache or installation authority.
- [rust/tcl-registry/src/native_property_lookup.rs](../../../../rust/tcl-registry/src/native_property_lookup.rs), `NativePropertyLookupProtocol::validate_declaration`: Physical C9.1 owner delegates to the same validation kernel.
- [rust/tcl-registry/src/native_property_lookup.rs](../../../../rust/tcl-registry/src/native_property_lookup.rs), `native_property_lookup::tests::original_c9_property_validity_does_not_issue_a_physical_lookup_recipe` (linked): C9.0/C9.1 pure declaration validation is available independently of the C9.1 physical cache selector.

## Evidence

- [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/source-anchors.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/source-anchors.json)
