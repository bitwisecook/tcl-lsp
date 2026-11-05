# naming.namespace.source-import-preload-order

Kind: `native-observation`

## Problem statement

Treating namespace import as a table-only operation can skip a current global auto_import callback and publish commands despite an earlier callback error.

## Question

Does the exact installed global auto_import helper run before namespace import validates an absent source namespace?

## Conclusion

Every retained C Tcl release returns PRELOAD91 from the installed helper before reporting any missing-source namespace error. Jim returns empty success and does not enter the helper in this control.

## Scope

One exact ASCII source-file control on Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim 0.84-9-g5bac7c9. The helper raises a fixed ASCII error and the import names an absent source namespace. Result code, provider string length/index and scan %c units are retained as ASCII. No arbitrary helper summary, callback absence, import completion, raw counted NUL, native table, compiler preparation or cache authority follows. BIG-IP was not tested.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII source-file shell ingress; decimal string-index scan-%c result units; no counted API/physical cache grant. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Code 1, length 9, decimal units 80 82 69 76 79 65 68 57 49 (PRELOAD91); the helper error precedes missing-source namespace validation.

### jim

Status: `observed`. Channel: ASCII source-file shell ingress; decimal string-index scan-%c result units; no counted API/physical cache grant. Version: jim 0.84-9-g5bac7c9.

Code 0, empty result; the installed helper is not called.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.4.20](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.4.20)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.5.19](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.5.19)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.6.18](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.6.18)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/9.0.4](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/9.0.4)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/9.1.0](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/9.1.0)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/jim](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/jim)
