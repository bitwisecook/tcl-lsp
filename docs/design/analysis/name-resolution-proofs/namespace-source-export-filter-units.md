# naming.namespace.source-export-filter-units

Kind: `native-observation`

## Problem statement

Source declarations with adjacent opaque units can be collapsed by display conversion, or a consumer can incorrectly reuse Tcl export filtering for Jim. This changes imported-command candidate sets.

## Question

Do the exact adjacent source-produced surrogate units remain distinct, and does namespace export filter or leading clear select the same commands on C Tcl and Jim?

## Conclusion

All five C releases retain FIRST/SECOND as distinct callable names, import only the exported adjacent-unit name, and leading -clear replaces before with after. Jim retains distinct callable names but ignores export filtering and leading -clear: both names import and before remains available.

A marked Compiler source definition binds exact surrogate export-pattern storage for the five selected C source recipes to the shared namespace transfer owner.

## Scope

The exact ASCII source-file controls are captured on Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim 0.84-9-g5bac7c9. Backslash substitutions produce the tested adjacent surrogate units. Attempt 2 reports provider string length/index and scan %c results as decimal ASCII units; it does not assert raw bytes or a universal scalar representation. These observations grant no counted API, raw NUL, physical cache, compiler preparation, arbitrary glob equivalence or source execution authority. BIG-IP was not tested.

The fixture analyses original Document source under selected C lexer/name policy and Direct mode; it launches no native interpreter and supplies no external output, private character/header, token or loader result. Encoded opaque export-pattern state is conditional source advice. The unchanged original public export-filter observations retain their own release/input scope, and no software passing outcome is attached.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

FIRST SECOND; export-filter result 1 0; leading-clear result 0 AFTER.

### jim

Status: `observed`. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim. Version: jim 0.84-9-g5bac7c9.

FIRST SECOND; export-filter result 1 1; leading-clear result 1 AFTER.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

No implementation binding is claimed by this observation record.

- [rust/tcl-compiler/src/command_binding/original_namespace_binding.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_binding.rs), `apply_exports`: Project selected original namespace export operands through their retained name-purpose recipe into exact opaque export alternatives; quiet source-model completion and Native observation remain separate.
- [rust/tcl-compiler/src/command_binding/original_namespace_binding.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_binding.rs), `command_binding::original_namespace_binding::tests::original_namespace_export_storage_keeps_exact_opaque_units` (linked): Five selected C source-model fixtures retain the original export pattern p plus encoded surrogateD800 as exact opaque NameBytes, distinct from D801, in the checkpoint namespace export alternatives. This inspects conditional source state only, with no new original process or object String conversion.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Evidence

- [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.4.20](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.4.20)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.5.19](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.5.19)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.6.18](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.6.18)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.0.4](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.0.4)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.1.0](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.1.0)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/jim](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/jim)
