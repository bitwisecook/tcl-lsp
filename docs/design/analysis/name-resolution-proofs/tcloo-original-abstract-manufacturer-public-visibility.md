# naming.tcloo.original-abstract-manufacturer-public-visibility

Kind: `native-observation`

## Problem statement

An abstract metaclass can hide public manufacturer methods without replacing the inherited constructor machinery. Treating failed public Abstract new/create calls as a universal constructor-handler prohibition also mispredicts a concrete subclass or an explicit object-level export.

## Question

For the exact original script, which manufacturer methods are publicly visible on an oo::abstract instance, do new/create/createWithNamespace public calls fail, can a concrete subclass create an instance, and does explicit object export new restore public Abstract new?

## Conclusion

Tcl 9.0.4 and 9.1.0 each close the exact script exit0 with empty stderr: the three manufacturers are private, their public calls return unknown-method code1, a concrete subclass creates normally, and explicitly exporting new enables a public Abstract new with code0. Tcl 8.4.20, 8.5.19, 8.6.18 and Jim 0.84-9-g5bac7c9 stop at unavailable oo::abstract before the intended visibility work. The retained source unexports those names and the manual describes inherited construction; neither printed values nor inspection establish constructor-handler execution, Method/payload identity, physical frames, Native admission or Rust outcomes. BIG-IP is not tested.

## Scope

Six independent ordinary-script observations and six independent version channels; complete original argv/input hashes, exit/duration and stdout/stderr are retained. Two C9 scripts reach the declared visibility work, while four older/Jim scripts measure producer unavailability only. Actual invoked-file/downstream ELF identities reuse the independently whole-hash-joined property335 canonical payloads, without duplicating executables. Reported ldd dependencies lack library-byte and inherited full environment/build closure. Four retained whole source/manual files and exact physical-LF excerpts inspect public unexport declarations independently. No constructor callback chronology, internal error-formatting stop, native header/refcount/Method/Scope/frame identity, BIG-IP/TMM, software assertion or implementation equivalence is measured.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, jim

Status: `observed`. Channel: One exact ordinary script file and independent version script through each recorded CLI argv; whole stdout/stderr and own process receipts. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; jim 0.84-9-g5bac7c9.

The exact script closes exit1 with empty stdout at invalid command name "oo::abstract". Its intended abstract manufacturer visibility and explicit-export work is not reached; this response measures only producer unavailability for the actual invocation.

### tcl9.0, tcl9.1

Status: `observed`. Channel: One exact ordinary script file and independent version script through each recorded CLI argv; whole stdout/stderr and own process receipts. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

The exact script closes exit0 with empty stderr. Abstract initially publishes only destroy; new, create and createWithNamespace remain in its private list and each public attempt returns code1/unknown method. A concrete subclass creates its own instance with code0. Explicit oo::objdefine Abstract export new then permits public Abstract new with code0, and the public list becomes {destroy new}. These values measure public visibility and dispatch results, not constructor-handler entry or physical Method/frame/lifetime identity.

### bigip

Status: `not-tested`. Channel: No BIG-IP script response for this question.

No original BIG-IP response is attached. Authored profiles and these local C/Jim results establish no BIG-IP/TMM manufacturer visibility or constructor behavior.

## Source inspection

tcl9.0 9.0.4, revision `Configured source tree joined by complete SHA; configure/build replay is unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.c`, function `MakeAdditionalClasses`, lines 701–713. Retained reference `source-window-0`.

```text
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

```

tcl9.0 9.0.4, revision `Configured source tree joined by complete SHA; configure/build replay is unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/doc/abstract.n`, function `abstract(n) public manufacturer description`, lines 27–48. Retained reference `source-window-1`.

```text
Abstract classes are classes that can contain definitions, but which cannot be
directly manufactured; they are intended to only ever be inherited from and
instantiated indirectly. The characteristic methods of \fBoo::class\fR
(\fBcreate\fR and \fBnew\fR) are not exported by an instance of
\fBoo::abstract\fR.
.PP
Note that \fBoo::abstract\fR is not itself an instance of \fBoo::abstract\fR.
.SS CONSTRUCTOR
The \fBoo::abstract\fR class does not define an explicit constructor; this
means that it is effectively the same as the constructor of the
\fBoo::class\fR class.
.SS DESTRUCTOR
The \fBoo::abstract\fR class does not define an explicit destructor;
destroying an instance of it is just like destroying an ordinary class (and
will destroy all its subclasses).
.SS "EXPORTED METHODS"
The \fBoo::abstract\fR class defines no new exported methods.
.SS "NON-EXPORTED METHODS"
The \fBoo::abstract\fR class explicitly states that \fBcreate\fR,
\fBcreateWithNamespace\fR, and \fBnew\fR are unexported.
.SH EXAMPLES
.PP

```

tcl9.1 9.1.0, revision `Configured source tree joined by complete SHA; configure/build replay is unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.c`, function `MakeAdditionalClasses`, lines 683–697. Retained reference `source-window-2`.

```text
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

```

tcl9.1 9.1.0, revision `Configured source tree joined by complete SHA; configure/build replay is unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/doc/abstract.n`, function `abstract(n) public manufacturer description`, lines 27–48. Retained reference `source-window-3`.

```text
Abstract classes are classes that can contain definitions, but which cannot be
directly manufactured; they are intended to only ever be inherited from and
instantiated indirectly. The characteristic methods of \fBoo::class\fR
(\fBcreate\fR and \fBnew\fR) are not exported by an instance of
\fBoo::abstract\fR.
.PP
Note that \fBoo::abstract\fR is not itself an instance of \fBoo::abstract\fR.
.SS CONSTRUCTOR
The \fBoo::abstract\fR class does not define an explicit constructor; this
means that it is effectively the same as the constructor of the
\fBoo::class\fR class.
.SS DESTRUCTOR
The \fBoo::abstract\fR class does not define an explicit destructor;
destroying an instance of it is just like destroying an ordinary class (and
will destroy all its subclasses).
.SS "EXPORTED METHODS"
The \fBoo::abstract\fR class defines no new exported methods.
.SS "NON-EXPORTED METHODS"
The \fBoo::abstract\fR class explicitly states that \fBcreate\fR,
\fBcreateWithNamespace\fR, and \fBnew\fR are unexported.
.SH EXAMPLES
.PP

```

## Consumer bindings

- [rust/tcl-compiler/src/type_infer.rs](../../../../rust/tcl-compiler/src/type_infer.rs), `type_infer::tests::original_caller_safe_factory_does_not_promote_abstract_exports_or_custom_metaclasses` (linked): Finite source-defined control: original caller safe factory does not promote abstract exports or custom metaclasses. The C9 public-call proof permits explicit export of Abstract new. That does not issue the ordinary Native default recipe required by this separate precision gate, or establish a completed allocation. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [rust/tcl-compiler/src/command_binding/source_transition_advice/source_class.rs](../../../../rust/tcl-compiler/src/command_binding/source_transition_advice/source_class.rs), `command_binding::source_transition_advice::source_class::tests::original_abstract_constructor_defaults_and_exports_keep_selected_native_source_grammar` (linked): Finite source-defined control: original abstract constructor defaults and exports keep selected native source grammar. A Native source-name model is not a entered Native constructor receipt. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [rust/tcl-compiler/src/command_binding/source_transition_advice/source_class.rs](../../../../rust/tcl-compiler/src/command_binding/source_transition_advice/source_class.rs), `command_binding::source_transition_advice::source_class::tests::original_abstract_constructor_logical_advice_keeps_defaults_without_native_identity` (linked): Finite source-defined control: original abstract constructor logical advice keeps defaults without native identity. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [rust/tcl-registry/src/definer.rs](../../../../rust/tcl-registry/src/definer.rs), `definer::tests::original_abstract_manufacturer_visibility_keeps_metaclass_and_created_class_surfaces` (linked): Finite source-defined control: original abstract manufacturer visibility keeps metaclass and created class surfaces. Public C9 observations establish defaults, not Native constructor admission. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [rust/tcl-spec-studio/src/render_rs.rs](../../../../rust/tcl-spec-studio/src/render_rs.rs), `render_rs::tests::original_abstract_grammar_rust_render_keeps_the_shared_named_descriptor` (linked): Finite source-defined control: original abstract grammar rust render keeps the shared named descriptor. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [rust/tcl-spec-studio/src/render_spectcl.rs](../../../../rust/tcl-spec-studio/src/render_spectcl.rs), `render_spectcl::tests::original_abstract_grammar_roundtrip_keeps_created_class_and_factory_visibilities` (linked): Finite source-defined control: original abstract grammar roundtrip keeps created class and factory visibilities. Roundtrip the selected semantic data without exporting Native hooks. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [rust/tcl-spectcl/src/loader.rs](../../../../rust/tcl-spectcl/src/loader.rs), `loader::tests::original_abstract_definition_grammar_loads_without_changing_factory_visibility` (linked): Finite source-defined control: original abstract definition grammar loads without changing factory visibility. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.

## Exact original-source Compiler software outcomes

The table reports only admitted selectors attached to this question by exact comments in the retained original test bodies. Each result is limited to its named source snapshot, owned Compiler image and selector invocation. No other source/image, unselected assertion, external-provider observation or performance conclusion follows.

| Original operation | Passed | Failed | Incomplete |
| --- | ---: | ---: | ---: |
| `integration-compiler-focused-consumer-tests344` | 1 | 2 | 0 |

Exact original-source body hashes, receipts, streams, inventories and owned images preserve the per-selector events and independent producer/admission limits. The existing provider answers retain their own experimental scope.

## Exact original-source347 software outcomes

These results belong only to the retained source347 body/question pairs and their owning Compiler or Core image. They do not establish an outcome for current source, unselected controls or external providers.

| Original operation | Passed | Failed |
| --- | ---: | ---: |
| `integration-compiler-focused-consumer-tests347` | 1 | 2 |

`Exact original body hashes, producer/list/admission receipts, raw logs and owned images` retain the finite per-selector results and independent failed Runtime producer.

## Evidence

- [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336)
- [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files)
- [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim/ldd.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim/ldd.stdout)
- [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim/ldd.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim/ldd.stderr)
