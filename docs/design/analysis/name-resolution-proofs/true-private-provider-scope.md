# naming.tcloo.true-private-provider-scope

Kind: `native-observation`

## Problem statement

Treating true-private as ordinary unexported visibility lets subclasses call a declaring class private method and incorrectly permits external same-object access to ordinary protected methods. Conversely, restricting every private call to the same object rejects authentic class-scoped access to another instance. This check distinguishes these cases in a single-inheritance class graph and includes a private forward and export conversion; it does not establish arbitrary filter, mixin, object-private or cache behavior.

## Question

For the captured C9 class private methods and forward, which original caller provider can invoke them, and does external same-object access make an ordinary unexported method callable?

## Conclusion

C9 allows an inherited method executing with its original declaring-class scope to invoke that class private method, including external [self] and another same-class object. A subclass-declared caller cannot invoke it. The declaring private takes priority over a public subclass override; a private forward has the same hidden roster/external-call boundary. Export converts the private method to public. External [self] access to a merely unexported Hidden method rejects on C8.6 and C9. Earlier OO/private-unavailable answers remain explicit. No general MRO, object-private, native method-cache or Rust correctness claim is issued.

## Scope

Actual ASCII NUL-terminated Tcl_Eval scripts with fresh interpreters and named single-inheritance classes. C9 class-scoped private calls, private forward, export conversion, inherited/subclass caller and public override controls. C8.6 independently rejects the private worker and rejects external same-object unexported access. Jim and C8.4/C8.5 only report absent OO. v1 and v2 source/outcomes remain separate.

## Provider answers

### tcl8.4, tcl8.5

Status: `unsupported`. Channel: Original counted Tcl objects passed to Tcl_EvalObjv plus ASCII NUL-terminated Tcl_Eval scripts. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

The startup query reports this version and the actual OO command query is empty; no OO argv or private body is executed.

### tcl8.6

Status: `unsupported`. Channel: Original counted Tcl objects passed to Tcl_EvalObjv plus ASCII NUL-terminated Tcl_Eval scripts. Version: tcl8.6 8.6.18.

Every four-operand method form rejects with wrong-argument usage; option-free dashed formal/name controls succeed and external same-object Hidden rejects. The private worker controls reject as unavailable.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Original counted Tcl objects passed to Tcl_EvalObjv plus ASCII NUL-terminated Tcl_Eval scripts. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

INHERITED_OWN_PRIVATE, EXTERNAL_SELF_PRIVATE, CROSS_OBJECT_CLASS_PRIVATE and DECLARER_PRIVATE_BEFORE_OVERRIDE return PRIVATE; SUBCLASS_PRIVATE_MISS errors. PRIVATE_FORWARD is hidden and callable in scope, EXPORT_CONVERTS_PRIVATE returns PRIVATE externally. EXTERNAL_SELF_UNEXPORTED errors.

### jim

Status: `unsupported`. Channel: ASCII NUL-terminated startup and OO-availability source passed to Jim_Eval; no Jim OO operand vector executes. Version: jim 0.84-9-g5bac7c9.

The startup query reports this version and the actual OO command query is empty; no OO argv or private body is executed.

### bigip

Status: `not-tested`.

No appliance observation for this exact method argv or private caller question.

## Source inspection

tcl9.0 9.0.4, revision `Pinned Tcl 9.0.4 release source`, `generic/tclOOCall.c`, function `AddInstancePrivateToCallContext / AddSimpleChainToCallContext`, lines 839–950. Retained reference `tcl9.0-scope-source`.

```text
AddInstancePrivateToCallContext(
    Object *const oPtr,		/* Object to add call chain entries for. */
    Tcl_Obj *const methodName,	/* Name of method to add the call chain
				 * entries for. */
    ChainBuilder *const cbPtr,	/* Where to add the call chain entries. */
    int flags)			/* What sort of call chain are we building. */
{
    Tcl_HashEntry *hPtr;
    Method *mPtr;
    int donePrivate = 0;

    if (oPtr->methodsPtr) {
	hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, methodName);
	if (hPtr != NULL) {
	    mPtr = (Method *) Tcl_GetHashValue(hPtr);
	    if (IS_PRIVATE(mPtr)) {
		AddMethodToCallChain(mPtr, cbPtr, NULL, NULL, flags);
		donePrivate = 1;
	    }
	}
    }
    return donePrivate;
}

/*
 * ----------------------------------------------------------------------
 *
 * AddSimpleChainToCallContext --
 *
 *	The core of the call-chain construction engine, this handles calling a
 *	particular method on a particular object. Note that filters and
 *	unknown handling are already handled by the logic that uses this
 *	function. Returns true if a private method was one of those found.
 *
 * ----------------------------------------------------------------------
 */

static inline int
AddSimpleChainToCallContext(
    Object *const oPtr,		/* Object to add call chain entries for. */
    Class *const contextCls,	/* Context class; the currently considered
				 * class is equal to this, private methods may
				 * also be added. [TIP 500] */
    Tcl_Obj *const methodNameObj,
				/* Name of method to add the call chain
				 * entries for. */
    ChainBuilder *const cbPtr,	/* Where to add the call chain entries. */
    Tcl_HashTable *const doneFilters,
				/* Where to record what call chain entries
				 * have been processed. */
    int flags,			/* What sort of call chain are we building. */
    Class *const filterDecl)	/* The class that declared the filter. If
				 * NULL, either the filter was declared by the
				 * object or this isn't a filter. */
{
    Tcl_Size i;
    int foundPrivate = 0, blockedUnexported = 0;
    Tcl_HashEntry *hPtr;
    Method *mPtr;

    if (!(flags & (KNOWN_STATE | SPECIAL)) && oPtr->methodsPtr) {
	hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, methodNameObj);

	if (hPtr != NULL) {
	    mPtr = (Method *) Tcl_GetHashValue(hPtr);
	    if (!IS_PRIVATE(mPtr)) {
		if (WANT_PUBLIC(flags)) {
		    if (!IS_PUBLIC(mPtr)) {
			blockedUnexported = 1;
		    } else {
			flags |= DEFINITE_PUBLIC;
		    }
		} else {
		    flags |= DEFINITE_PROTECTED;
		}
	    }
	}
    }
    if (!(flags & SPECIAL)) {
	Class *mixinPtr;

	FOREACH(mixinPtr, oPtr->mixins) {
	    if (contextCls) {
		foundPrivate |= AddPrivatesFromClassChainToCallContext(
			mixinPtr, contextCls, methodNameObj, cbPtr,
			doneFilters, flags|TRAVERSED_MIXIN, filterDecl);
	    }
	    foundPrivate |= AddSimpleClassChainToCallContext(mixinPtr,
		    methodNameObj, cbPtr, doneFilters,
		    flags | TRAVERSED_MIXIN, filterDecl);
	}
	if (oPtr->methodsPtr && !blockedUnexported) {
	    hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, methodNameObj);
	    if (hPtr != NULL) {
		mPtr = (Method *) Tcl_GetHashValue(hPtr);
		if (!IS_PRIVATE(mPtr)) {
		    AddMethodToCallChain(mPtr, cbPtr, doneFilters, filterDecl,
			    flags);
		}
	    }
	}
    }
    if (!oPtr->selfCls) {
	return foundPrivate;
    }
    if (contextCls) {
	foundPrivate |= AddPrivatesFromClassChainToCallContext(oPtr->selfCls,
		contextCls, methodNameObj, cbPtr, doneFilters, flags,
		filterDecl);
    }
    if (!blockedUnexported) {

```

tcl9.1 9.1.0, revision `Pinned Tcl 9.1.0 release source`, `generic/tclOOCall.c`, function `AddInstancePrivateToCallContext / AddSimpleChainToCallContext`, lines 860–969. Retained reference `tcl9.1-scope-source`.

```text
AddInstancePrivateToCallContext(
    Object *const oPtr,		/* Object to add call chain entries for. */
    Tcl_Obj *const methodName,	/* Name of method to add the call chain
				 * entries for. */
    ChainBuilder *const cbPtr,	/* Where to add the call chain entries. */
    int flags)			/* What sort of call chain are we building. */
{
    Tcl_HashEntry *hPtr;
    Method *mPtr;
    bool donePrivate = false;

    if (oPtr->methodsPtr) {
	hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, methodName);
	if (hPtr != NULL) {
	    mPtr = (Method *) Tcl_GetHashValue(hPtr);
	    if (IS_PRIVATE(mPtr)) {
		AddMethodToCallChain(mPtr, cbPtr, NULL, NULL, flags);
		donePrivate = true;
	    }
	}
    }
    return donePrivate;
}

/*
 * ----------------------------------------------------------------------
 *
 * AddSimpleChainToCallContext --
 *
 *	The core of the call-chain construction engine, this handles calling a
 *	particular method on a particular object. Note that filters and
 *	unknown handling are already handled by the logic that uses this
 *	function. Returns true if a private method was one of those found.
 *
 * ----------------------------------------------------------------------
 */
static inline bool
AddSimpleChainToCallContext(
    Object *const oPtr,		/* Object to add call chain entries for. */
    Class *const contextCls,	/* Context class; the currently considered
				 * class is equal to this, private methods may
				 * also be added. [TIP 500] */
    Tcl_Obj *const methodNameObj,
				/* Name of method to add the call chain
				 * entries for. */
    ChainBuilder *const cbPtr,	/* Where to add the call chain entries. */
    Tcl_HashTable *const doneFilters,
				/* Where to record what call chain entries
				 * have been processed. */
    int flags,			/* What sort of call chain are we building. */
    Class *const filterDecl)	/* The class that declared the filter. If
				 * NULL, either the filter was declared by the
				 * object or this isn't a filter. */
{
    Tcl_Size i;
    bool foundPrivate = false, blockedUnexported = false;
    Tcl_HashEntry *hPtr;
    Method *mPtr;

    if (!(flags & (KNOWN_STATE | SPECIAL)) && oPtr->methodsPtr) {
	hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, methodNameObj);

	if (hPtr != NULL) {
	    mPtr = (Method *) Tcl_GetHashValue(hPtr);
	    if (!IS_PRIVATE(mPtr)) {
		if (WANT_PUBLIC(flags)) {
		    if (!IS_PUBLIC(mPtr)) {
			blockedUnexported = true;
		    } else {
			flags |= DEFINITE_PUBLIC;
		    }
		} else {
		    flags |= DEFINITE_PROTECTED;
		}
	    }
	}
    }
    if (!(flags & SPECIAL)) {
	Class *mixinPtr;

	FOREACH(mixinPtr, oPtr->mixins) {
	    if (contextCls) {
		foundPrivate |= AddPrivatesFromClassChainToCallContext(
			mixinPtr, contextCls, methodNameObj, cbPtr,
			doneFilters, flags|TRAVERSED_MIXIN, filterDecl);
	    }
	    foundPrivate |= AddSimpleClassChainToCallContext(mixinPtr,
		    methodNameObj, cbPtr, doneFilters,
		    flags | TRAVERSED_MIXIN, filterDecl);
	}
	if (oPtr->methodsPtr && !blockedUnexported) {
	    hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, methodNameObj);
	    if (hPtr != NULL) {
		mPtr = (Method *) Tcl_GetHashValue(hPtr);
		if (!IS_PRIVATE(mPtr)) {
		    AddMethodToCallChain(mPtr, cbPtr, doneFilters, filterDecl,
			    flags);
		}
	    }
	}
    }
    if (!oPtr->selfCls) {
	return foundPrivate;
    }
    if (contextCls) {
	foundPrivate |= AddPrivatesFromClassChainToCallContext(oPtr->selfCls,
		contextCls, methodNameObj, cbPtr, doneFilters, flags,
		filterDecl);
    }

```

## Consumer bindings

- [rust/tcl-registry/src/native_tcloo_method_definition.rs](../../../../rust/tcl-registry/src/native_tcloo_method_definition.rs), `NativeTclooMethodDefinitionProtocol::default_visibility`: Selected pure method argv/visibility semantics, independent of body or native table capability.
- [rust/tcl-vm/src/cmd_oo.rs](../../../../rust/tcl-vm/src/cmd_oo.rs), `oo_invoke_value_with_head`: Consume original option objects and current method flags, or authentic caller-provider scope without a context-dependent cache grant.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `Interp::oo_invoke_with_original`: Consume the same selected declaration/caller boundary on the runtime port.
- [rust/tcl-vm/src/cmd_oo/native_private_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_private_tests.rs), `cmd_oo::native_private_tests::method_option_layout_and_private_scope_match_original_native_controls` (linked): Compare selected C8.6/C9 original method option definition completion code/bytes and successful/internal/external outputs. Private rosters compare the exact one-element invoke result. Class caller controls compare exact results; subclass scope rejection compares completion code only, leaving bootstrap diagnostic-roster ordering independent. No pass is inferred from the binding.
- [runtime/rust/src/cmd_oo/native_private_tests.rs](../../../../runtime/rust/src/cmd_oo/native_private_tests.rs), `cmd_oo::native_private_tests::method_option_layout_and_private_scope_match_original_native_controls` (linked): Same finite original option/caller comparisons on independently selected runtime cores; no fresh native or Rust execution is implied.

## Evidence

- [rust/tcl-vm/tests/data/native_method_visibility](../../../../rust/tcl-vm/tests/data/native_method_visibility)
- [rust/tcl-vm/tests/data/native_method_visibility/v1](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1)
- [rust/tcl-vm/tests/data/native_method_visibility/v2](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2)
