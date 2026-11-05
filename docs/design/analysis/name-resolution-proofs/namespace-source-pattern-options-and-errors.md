# naming.namespace.source-pattern-options-and-errors

Kind: `native-observation`

## Problem statement

A consumer that assumes identical namespace validation or overwrite rules can publish commands after a rejected import, erase a surviving destination, or treat a no-op export as filtering.

## Question

What catch codes and ordered results do the exact local, missing-source, same-source import, qualified export and force-replacement controls produce?

## Conclusion

C Tcl rejects local, missing-source and same-source imports and qualified exports; a non-force conflict retains DESTINATION and subsequent -force selects SOURCE. Jim accepts local and missing-source imports and qualified export, rejects same-source import, and its first import already replaces DESTINATION with SOURCE.

A marked Compiler control withholds complete source worlds for four independently unresolved namespace prelude/destination/pattern cases across selected C recipes.

## Scope

The exact ASCII source-file controls are captured on Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim 0.84-9-g5bac7c9. Backslash substitutions produce the tested adjacent surrogate units. Attempt 2 reports provider string length/index and scan %c results as decimal ASCII units; it does not assert raw bytes or a universal scalar representation. These observations grant no counted API, raw NUL, physical cache, compiler preparation, arbitrary glob equivalence or source execution authority. BIG-IP was not tested.

Model refusal does not equal an executed native error, public completion, option validation order, force-replacement licence or closed absence. The unchanged original pattern/options/error rows retain their own measured versions and purposes. Current source correspondence and quiet observer obligations stay independent; no assertion result is supplied.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

All four invalid controls return code 1; force result 1 DESTINATION SOURCE.

### jim

Status: `observed`. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim. Version: jim 0.84-9-g5bac7c9.

Local/missing imports and qualified export return code 0 empty; same-source import code 1; force result 0 SOURCE SOURCE.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

No implementation binding is claimed by this observation record.

- [rust/tcl-compiler/src/command_binding/original_namespace_binding.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_binding.rs), `OriginalNamespaceBinding::completed`: Require exact current source/config/vector/handler/namespace/input correspondence and expected closed source table/export state before model completion; unknown prelude or destinations remain terminal.
- [rust/tcl-compiler/src/command_binding/original_namespace_binding.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_binding.rs), `command_binding::original_namespace_binding::tests::original_namespace_import_completion_requires_closed_prelude_and_destinations` (linked): Across five selected C source recipes, an unresolved auto_import prelude, force replacement of an existing destination, self-namespace import or qualified export pattern prevents a completed source command world. These four conservative model refusals do not manufacture an original Guest error or prove all provider option semantics.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Evidence

- [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.4.20](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.4.20)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.5.19](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.5.19)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.6.18](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.6.18)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.0.4](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.0.4)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.1.0](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.1.0)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/jim](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/jim)
