# naming.expression.original-implicit-function-slot-presence

Kind: `native-observation`

## Problem statement

Implicit expression calls do not own written command-head words. Determine whether a missing Pi function is a command-slot lookup and whether a published ::tcl::mathfunc::Pi procedure affects dispatch under each measured provider.

## Question

What do the original command inventory and caught Pi() results report before and after the exact namespace/procedure installation controls under each retained provider?

## Conclusion

All C releases begin with an empty Pi command inventory and a failed Pi() call. C8.5–9.1 return 17 after successful ::tcl::mathfunc::Pi installation; C8.4 retains its unknown math function result even though the procedure inventory contains that name. Jim accepts the namespace/procedure controls with its exact reported names but Pi() remains a syntax error.

A marked readonly source occurrence control preserves fixed-function presence/absence and retained snapshot correspondence without manufacturing command references.

## Scope

Six fresh provider processes retain six version rows and 36 sequential caught controls of the exact ASCII source. Initial lookup, namespace creation, installation and later expression evaluation share each provider process; successful namespace and install controls are measured rather than assumed. Binary-scan result hex supplies no native getter or original object identity. These observations bound expression-engine behavior and the selected command-slot purpose only; they prove no generic callback-free lookup, source snapshot completeness, compiler admission, cache/header/physical allocation or Native Normal. Jim names and expression syntax remain distinct from C namespace semantics. BIG-IP is not tested. The linked Rust selector retains its separate source-owner scope and has no execution result here.

No expression evaluation, operand preparation/topology, native function entry, private command slot or Guest failure is observed by this source definition. Jim Pi syntax error belongs to its independent original process purpose. Equal expression bytes, catalogue labels or an absence projection supply no Normal/SSA/frame/body authority; no assertion outcome is attached.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Fixed ASCII LF source-file CLI, one fresh provider process; six caught controls execute sequentially and share namespace/procedure state after the actual version row. Results use binary scan H* projection. Version: tcl8.4 8.4.20.

The initial command inventory is empty and Pi() reports unknown math function "Pi". Namespace creation and procedure installation succeed, and the installed inventory contains ::tcl::mathfunc::Pi; the expression still reports the same unknown math function. This separates the fixed function engine from successful procedure publication.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Fixed ASCII LF source-file CLI, one fresh provider process; six caught controls execute sequentially and share namespace/procedure state after the actual version row. Results use binary scan H* projection. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

The initial command inventory is empty and Pi() reports invalid command name "tcl::mathfunc::Pi". Namespace creation and procedure installation succeed, the installed inventory contains ::tcl::mathfunc::Pi, and Pi() returns 17 afterwards.

### jim

Status: `observed`. Channel: Fixed ASCII LF source-file CLI, one fresh provider process; six caught controls execute sequentially and share namespace/procedure state after the actual version row. Results use binary scan H* projection. Version: jim 0.84-9-g5bac7c9.

The initial inventory is empty and Pi() reports a syntax error. The namespace control succeeds; procedure installation returns ::tcl::mathfunc::Pi and the queried inventory reports ::::tcl::mathfunc::Pi. Pi() retains its syntax error. These exact names and outcomes establish no C namespace or expression-command dispatch behavior for Jim.

### bigip

Status: `not-tested`.

No appliance observation of these exact implicit function and installation controls.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/original_math_function.rs](../../../../rust/tcl-compiler/src/command_binding/original_math_function.rs), `command_binding::original_math_function::tests::original_function_lookup_keeps_absence_and_unknown_separate` (linked): C8.5–9.1 original source function-name receipts retain absence without a fabricated written head; actual Unknown observations and operand/policy/observer barriers remain terminal. These are source-purpose assertions, not a replay of the native installation controls.

- [rust/tcl-compiler/src/command_binding/original_math_function.rs](../../../../rust/tcl-compiler/src/command_binding/original_math_function.rs), `SourceCommandBindings::original_math_functions_in_source`: Join retained actual source/config/Registry snapshot and selected original expression-word ancestry before projecting fixed/implicit function occurrence and diagnostic presence.
- [rust/tcl-compiler/src/command_binding/original_math_function.rs](../../../../rust/tcl-compiler/src/command_binding/original_math_function.rs), `command_binding::original_math_function::tests::original_fixed_function_absence_retains_source_snapshot_without_command_name` (linked): Actual selected C8.4 and Jim source fixtures retain fixed-table Pi absence with no command name/reference/Registry identity; changed image, withdrawn snapshot, dynamic unknown child/future command or shadowed expr withholds presence. abs remains a selected present fixed-function control. Source absence does not execute operands or report Jim Guest syntax failure.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Evidence

- [rust/tcl-registry/tests/data/native_implicit_function_presence_original](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original)
- [rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.6.18](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.6.18)
- [rust/tcl-registry/tests/data/native_implicit_function_presence_original/9.1.0](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/9.1.0)
- [rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.4.20](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.4.20)
- [rust/tcl-registry/tests/data/native_implicit_function_presence_original/9.0.4](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/9.0.4)
- [rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.5.19](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.5.19)
- [rust/tcl-registry/tests/data/native_implicit_function_presence_original/jim](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/jim)
