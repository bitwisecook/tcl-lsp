# naming.tcloo.original-configurable-factory-support

Kind: `implementation-contract`

## Problem statement

A source-created configurable class needs the support allocation selected by its native factory constructor and the provider definition namespaces. The factory and support names can move independently; a plain source grammar or a displayed empty constructor cannot supply that identity.

## Question

Does the original source class retain the stock factory recipe and actual selected support allocation, while future configurable factory calls independently reselect the support slot?

## Conclusion

The Registry selects ordinary or configurable stock constructor roles under the actual Tcl release. Known source baselines register the support and property-worker metadata and ordered definition paths once. Each retained class captures current factory/support targets; later constructor entry revalidates those allocations independently of declaration bodies. Configurable property name validity uses the selected C9 declaration recipe. The capture grants no successful factory completion or native class topology.

## Scope

Rust source-analysis contract for independently selected C Tcl 8.6/9.0/9.1 ordinary factories and C Tcl 9.0/9.1 configurable factories, exact current unwrapped registered factory/support allocations, known stock source entry and represented literal definition members. Native configurable snapshots without an independent class-role/topology receipt, custom factories, mutated provider topology, arbitrary immediate bodies and actual guest execution are excluded. Rust selectors are validation obligations; no passing execution receipt is attached.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

No guest execution of this Rust implementation question is claimed.

## Source inspection

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source; full file digest identifies the source bytes`, `generic/tclOO.c`, function `MakeAdditionalClasses`, lines 657–787. Retained reference `c-source`.

```text
MakeAdditionalClasses(
    Foundation *fPtr,
    Tcl_Namespace *defineNs,
    Tcl_Namespace *objDefineNs)
{
    Tcl_Interp *interp = fPtr->interp;
    Object *singletonObj;	/* A metaclass that is used to make classes
				 * that only permit one instance of them to
				 * exist. See singleton(n). */
    Object *singletonInst;	/* A mixin used to make an object so it won't
				 * be destroyed or cloned (or at least not
				 * easily). */
    Object *abstractCls;	/* A metaclass that is used to make classes
				 * that can't be directly instantiated. See
				 * abstract(n). */
    Object *cfgSupObj;		/* The class that contains the implementation
				 * of the actual 'configure' method (mixed into
				 * actually configurable classes). The
				 * 'configure' method is in tclOOBasic.c. */
    Object *configurableObj;	/* A metaclass that is used to make classes
				 * that can be configured in their creation
				 * phase (and later too). All the metaclass
				 * itself does is arrange for the class created
				 * to have a 'configure' method and for
				 * oo::define and oo::objdefine (on the class
				 * and its instances) to have a property
				 * definition for setting things up for
				 * 'configure'. */
    Class *singletonCls, *cfgSupCls, *configurableCls;
    Tcl_Namespace *cfgObjNs, *cfgClsNs;
    Tcl_Obj *nsName;

    /*
     * Make the oo::singleton class, the SingletonInstance class, and install
     * their standard defined methods.
     */

    singletonObj = (Object *) Tcl_NewObjectInstance(interp,
	    (Tcl_Class) fPtr->classCls, "::oo::singleton",
	    NULL, TCL_INDEX_NONE, NULL, 0);
    singletonCls = singletonObj->classPtr;
    TclOODefineBasicMethods(singletonCls, singletonMethods);
    /* Set the superclass to oo::class */
    MarkAsMetaclass(fPtr, singletonCls);
    /* Unexport methods */
    TclOOUnexportMethods(singletonCls, "create", "createWithNamespace", NULL);

    singletonInst = (Object *) Tcl_NewObjectInstance(interp,
	    (Tcl_Class) fPtr->classCls, "::oo::SingletonInstance",
	    NULL, TCL_INDEX_NONE, NULL, 0);
    TclOODefineBasicMethods(singletonInst->classPtr, singletonInstanceMethods);

    /*
     * Make the oo::abstract class.
     */

    abstractCls = (Object *) Tcl_NewObjectInstance(interp,
	    (Tcl_Class) fPtr->classCls, "::oo::abstract",
	    NULL, TCL_INDEX_NONE, NULL, 0);
    /* Set the superclass to oo::class */
    MarkAsMetaclass(fPtr, abstractCls->classPtr);
    /* Unexport methods */
    TclOOUnexportMethods(abstractCls->classPtr,
	    "create", "createWithNamespace", "new", NULL);

    /*
     * Make the configurable class and install its standard defined method.
     */

    cfgSupObj = (Object *) Tcl_NewObjectInstance(interp,
	    (Tcl_Class) fPtr->classCls, "::oo::configuresupport::configurable",
	    NULL, TCL_INDEX_NONE, NULL, 0);
    cfgSupCls = cfgSupObj->classPtr;
    TclOODefineBasicMethods(cfgSupCls, cfgMethods);

    /* Namespaces used as implementation vectors for oo::define and
     * oo::objdefine when the class/instance is configurable.
     * Note that these also contain commands implemented in C,
     * especially the [property] definition command. */

    cfgObjNs = Tcl_CreateNamespace(interp,
	    "::oo::configuresupport::configurableobject", NULL, NULL);
    TclCreateObjCommandInNs(interp, "property", cfgObjNs,
	    TclOODefinePropertyCmd, INT2PTR(1) /*useInstance*/, NULL);
    TclCreateObjCommandInNs(interp, "properties", cfgObjNs,
	    TclOODefinePropertyCmd, INT2PTR(1) /*useInstance*/, NULL);
    Tcl_Export(interp, cfgObjNs, "property", /*reset*/1);
    TclSetNsPath((Namespace *) cfgObjNs, 1, &objDefineNs);

    cfgClsNs = Tcl_CreateNamespace(interp,
	    "::oo::configuresupport::configurableclass", NULL, NULL);
    TclCreateObjCommandInNs(interp, "property", cfgClsNs,
	    TclOODefinePropertyCmd, INT2PTR(0) /*useInstance*/, NULL);
    TclCreateObjCommandInNs(interp, "properties", cfgClsNs,
	    TclOODefinePropertyCmd, INT2PTR(0) /*useInstance*/, NULL);
    Tcl_Export(interp, cfgClsNs, "property", /*reset*/1);
    TclSetNsPath((Namespace *) cfgClsNs, 1, &defineNs);

    /* The oo::configurable class itself, a metaclass to apply
     * oo::configuresupport::configurable correctly. */

    configurableObj = (Object *) Tcl_NewObjectInstance(interp,
	    (Tcl_Class) fPtr->classCls, "::oo::configurable",
	    NULL, TCL_INDEX_NONE, NULL, 0);
    configurableCls = configurableObj->classPtr;
    MarkAsMetaclass(fPtr, configurableCls);
    Tcl_ClassSetConstructor(interp, (Tcl_Class) configurableCls, TclNewMethod(
	    (Tcl_Class) configurableCls, NULL, 0, &configurableConstructor, NULL));

    /* Set the definition namespaces of oo::configurable and
     * oo::configuresupport::configurable. */

    nsName = TclNewNamespaceObj(cfgClsNs);
    Tcl_IncrRefCount(nsName);
    if (cfgSupCls->clsDefinitionNs != NULL) {
	Tcl_DecrRefCount(cfgSupCls->clsDefinitionNs);
    }
    cfgSupCls->clsDefinitionNs = nsName;
    Tcl_IncrRefCount(nsName);
    if (configurableCls->clsDefinitionNs != NULL) {
	Tcl_DecrRefCount(configurableCls->clsDefinitionNs);
    }
    configurableCls->clsDefinitionNs = nsName;

    nsName = TclNewNamespaceObj(cfgObjNs);
    Tcl_IncrRefCount(nsName);
    if (cfgSupCls->objDefinitionNs != NULL) {
	Tcl_DecrRefCount(cfgSupCls->objDefinitionNs);
    }
    cfgSupCls->objDefinitionNs = nsName;
}

```

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source; full file digest identifies the source bytes`, `generic/tclOOBasic.c`, function `TclOO_Configurable_Constructor`, lines 597–622. Retained reference `c-source`.

```text
TclOO_Configurable_Constructor(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    Tcl_Obj *cfgSupportName;
    Class *mixin;

    if (objc != skip && objc != skip + 1) {
	Tcl_WrongNumArgs(interp, skip, objv, "?definitionScript?");
	return TCL_ERROR;
    }
    cfgSupportName = Tcl_NewStringObj(
	    "::oo::configuresupport::configurable", TCL_AUTO_LENGTH);
    mixin = TclOOGetClassFromObj(interp, cfgSupportName);
    Tcl_BounceRefCount(cfgSupportName);
    if (!mixin) {
	return TCL_ERROR;
    }
    TclOOClassSetMixins(interp, oPtr->classPtr, 1, &mixin);
    return TclNRObjectContextInvokeNext(interp, context, objc, objv, skip);
}

```

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source; full file digest identifies the source bytes`, `generic/tclOO.c`, function `MakeAdditionalClasses`, lines 661–776. Retained reference `c-source`.

```text
MakeAdditionalClasses(
    Foundation *fPtr,
    Tcl_Namespace *defineNs,
    Tcl_Namespace *objDefineNs)
{
    Tcl_Interp *interp = fPtr->interp;

    /*
     * Make the singleton class, the SingletonInstance class, and install their
     * standard defined methods.
     */

    // A metaclass that is used to make classes that only permit one instance
    // of them to exist. See singleton(n).
    Object *singletonObj = (Object *) Tcl_NewObjectInstance(interp,
	    (Tcl_Class) fPtr->classCls, "::oo::singleton",
	    NULL, TCL_INDEX_NONE, NULL, 0);
    Class *singletonCls = singletonObj->classPtr;
    TclOODefineBasicMethods(singletonCls, singletonMethods);
    // Set the superclass to oo::class
    MarkAsMetaclass(fPtr, singletonCls);
    // Unexport methods
    TclOOUnexportMethods(singletonCls, "create", "createWithNamespace", NULL);

    // A mixin used to make an object so it won't be destroyed or cloned (or
    // at least not easily).
    Object *singletonInst = (Object *) Tcl_NewObjectInstance(interp,
	    (Tcl_Class) fPtr->classCls, "::oo::SingletonInstance",
	    NULL, TCL_INDEX_NONE, NULL, 0);
    TclOODefineBasicMethods(singletonInst->classPtr, singletonInstanceMethods);

    /*
     * Make the abstract class.
     */

    // A metaclass that is used to make classes that can't be directly
    // instantiated. See abstract(n).
    Object *abstractCls = (Object *) Tcl_NewObjectInstance(interp,
	    (Tcl_Class) fPtr->classCls, "::oo::abstract",
	    NULL, TCL_INDEX_NONE, NULL, 0);
    // Set the superclass to oo::class
    MarkAsMetaclass(fPtr, abstractCls->classPtr);
    // Unexport methods
    TclOOUnexportMethods(abstractCls->classPtr,
	    "create", "createWithNamespace", "new", NULL);

    /*
     * Make the configurable class and install its standard defined method.
     */

    // The class that contains the implementation of the actual
    // 'configure' method (mixed into actually configurable classes).
    // The 'configure' method is in tclOOBasic.c.
    Object *cfgSupObj = (Object *) Tcl_NewObjectInstance(interp,
	    (Tcl_Class) fPtr->classCls, "::oo::configuresupport::configurable",
	    NULL, TCL_INDEX_NONE, NULL, 0);
    Class *cfgSupCls = cfgSupObj->classPtr;
    TclOODefineBasicMethods(cfgSupCls, cfgMethods);

    // Namespaces used as implementation vectors for oo::define and
    // oo::objdefine when the class/instance is configurable.
    // Note that these also contain commands implemented in C,
    // especially the [property] definition command.

    Tcl_Namespace *cfgObjNs = Tcl_CreateNamespace(interp,
	    "::oo::configuresupport::configurableobject", NULL, NULL);
    TclCreateObjCommandInNs(interp, "property", cfgObjNs,
	    TclOODefinePropertyCmd, INT2PTR(1) /*useInstance*/, NULL);
    TclCreateObjCommandInNs(interp, "properties", cfgObjNs,
	    TclOODefinePropertyCmd, INT2PTR(1) /*useInstance*/, NULL);
    Tcl_Export(interp, cfgObjNs, "property", /*reset*/1);
    TclSetNsPath((Namespace *) cfgObjNs, 1, &objDefineNs);

    Tcl_Namespace *cfgClsNs = Tcl_CreateNamespace(interp,
	    "::oo::configuresupport::configurableclass", NULL, NULL);
    TclCreateObjCommandInNs(interp, "property", cfgClsNs,
	    TclOODefinePropertyCmd, INT2PTR(0) /*useInstance*/, NULL);
    TclCreateObjCommandInNs(interp, "properties", cfgClsNs,
	    TclOODefinePropertyCmd, INT2PTR(0) /*useInstance*/, NULL);
    Tcl_Export(interp, cfgClsNs, "property", /*reset*/1);
    TclSetNsPath((Namespace *) cfgClsNs, 1, &defineNs);

    // A metaclass that is used to make classes that can be configured in
    // their creation phase (and later too). All the metaclass itself does is
    // arrange for the class created to have a 'configure' method and for
    // oo::define and oo::objdefine (on the class and its instances) to have
    // a property definition for setting things up for 'configure'.
    Object *configurableObj = (Object *) Tcl_NewObjectInstance(interp,
	    (Tcl_Class) fPtr->classCls, "::oo::configurable",
	    NULL, TCL_INDEX_NONE, NULL, 0);
    Class *configurableCls = configurableObj->classPtr;
    MarkAsMetaclass(fPtr, configurableCls);
    Tcl_ClassSetConstructor(interp, (Tcl_Class) configurableCls, TclNewMethod(
	    (Tcl_Class) configurableCls, NULL, 0, &configurableConstructor, NULL));

    Tcl_Obj *nsName = Tcl_NewStringObj("::oo::configuresupport::configurableclass",
	    TCL_AUTO_LENGTH);
    Tcl_IncrRefCount(nsName);
    if (cfgSupCls->clsDefinitionNs != NULL) {
	Tcl_DecrRefCount(cfgSupCls->clsDefinitionNs);
    }
    cfgSupCls->clsDefinitionNs = nsName;
    Tcl_IncrRefCount(nsName);
    if (configurableCls->clsDefinitionNs != NULL) {
	Tcl_DecrRefCount(configurableCls->clsDefinitionNs);
    }
    configurableCls->clsDefinitionNs = nsName;

    nsName = Tcl_NewStringObj("::oo::configuresupport::configurableobject",
	    TCL_AUTO_LENGTH);
    Tcl_IncrRefCount(nsName);
    if (cfgSupCls->objDefinitionNs != NULL) {
	Tcl_DecrRefCount(cfgSupCls->objDefinitionNs);
    }
    cfgSupCls->objDefinitionNs = nsName;
}

```

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source; full file digest identifies the source bytes`, `generic/tclOOBasic.c`, function `TclOO_Configurable_Constructor`, lines 662–684. Retained reference `c-source`.

```text
TclOO_Configurable_Constructor(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    if (objc != skip && objc != skip + 1) {
	Tcl_WrongNumArgs(interp, skip, objv, "?definitionScript?");
	return TCL_ERROR;
    }
    Tcl_Obj *cfgSupportName = Tcl_NewStringObj(
	    "::oo::configuresupport::configurable", TCL_AUTO_LENGTH);
    Class *mixin = TclOOGetClassFromObj(interp, cfgSupportName);
    Tcl_BounceRefCount(cfgSupportName);
    if (!mixin) {
	return TCL_ERROR;
    }
    TclOOClassSetMixins(interp, oPtr->classPtr, 1, &mixin);
    return TclNRObjectContextInvokeNext(interp, context, objc, objv, skip);
}

```

## Consumer bindings

- [rust/tcl-registry/src/native_tcloo_bootstrap.rs](../../../../rust/tcl-registry/src/native_tcloo_bootstrap.rs), `NativeClassFactoryRecipe`: Stock constructor and support role selection only.
- [rust/tcl-registry/src/registry.rs](../../../../rust/tcl-registry/src/registry.rs), `CommandRegistry::native_class_factory_recipe`: Actual selected Registry release/surface and native stock spec provenance.
- [rust/tcl-compiler/src/command_binding/registered_class_factory.rs](../../../../rust/tcl-compiler/src/command_binding/registered_class_factory.rs), `OriginalClassFactoryState`: Current factory/support allocation dependency capture and revalidation.
- [rust/tcl-compiler/src/command_binding/deferred_method.rs](../../../../rust/tcl-compiler/src/command_binding/deferred_method.rs), `SourceCommandBindings::register_bounded_methods`: Actual selected workers, source declaration fields and current class allocation remain independent.
- [rust/tcl-registry/src/native_tcloo_bootstrap.rs](../../../../rust/tcl-registry/src/native_tcloo_bootstrap.rs), `native_tcloo_bootstrap::tests::original_configurable_bootstrap_roles_require_selected_stock_release` (linked): Stock release selection retains constructor and bootstrap namespace/path roles.
- [rust/tcl-compiler/src/command_binding/registered_class_factory.rs](../../../../rust/tcl-compiler/src/command_binding/registered_class_factory.rs), `command_binding::registered_class_factory::tests::original_configurable_class_retains_the_selected_support_allocation` (linked): Actual source manufacture must retain its selected support token and independently closed own lifecycle.
- [rust/tcl-compiler/src/command_binding/registered_class_factory.rs](../../../../rust/tcl-compiler/src/command_binding/registered_class_factory.rs), `command_binding::registered_class_factory::tests::original_configurable_existing_classes_keep_moved_support_but_new_factories_reselect` (linked): Existing source classes retain the moved support allocation while the original support slot is unavailable to future factory selection.

## Evidence

- [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/source-anchors.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/source-anchors.json)
