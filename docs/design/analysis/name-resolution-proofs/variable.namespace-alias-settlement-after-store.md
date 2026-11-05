# naming.variable.namespace-alias-settlement-after-store

Kind: `native-observation`

## Problem statement

The variable command can store a namespace value before its attempted local link fails, while a callback can replace the target before a later read. Treating the authored local name as a stable alias or assuming all errors undo writes would select the wrong cell/value.

## Question

How do traced local alias rejection, scalar-versus-array definition rejection and callback-driven global target replacement settle the original variable operation?

## Conclusion

All five C captures reject linking an already traced local v while the global has already become X; reject arr(k) against scalar arr without changing SCALAR; and read REPLACED after the write trace unsets/recreates the namespace target. These three exact scripts establish partial stores and alias settlement, not transactional rollback, arbitrary observer closure or Jim semantics.

## Scope

Exact ASCII source.tcl and the retained public C probe; C8.4.20–9.1.0 output/catch windows only. No original-name cache/header/refcount or general native frame identity is inferred. Jim and BIG-IP were not queried.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII source script via retained public C probe. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

traced 1 {variable "v" has traces: can't use for upvar} X
scalarArray 1 {can't define "arr(k)": name refers to an element in an array} 0 0 SCALAR
rebound 0 REPLACED REPLACED

### jim

Status: `not-tested`.

No observation for this question.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `namespace_alias_settlement_matches_all_15_native_callback_and_error_results`: Current independent Rust fixture comparison.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::namespace_alias_settlement_matches_all_15_native_callback_and_error_results` (linked): Compares the independently scoped original alias/argument outcome with retained native rows; no native authority follows from a Rust result.

## Evidence

- [runtime/rust/tests/data/native_namespace_alias_settlement/manifest.json](../../../../runtime/rust/tests/data/native_namespace_alias_settlement/manifest.json)
- [runtime/rust/tests/data/native_namespace_alias_settlement/source.tcl](../../../../runtime/rust/tests/data/native_namespace_alias_settlement/source.tcl)
- [runtime/rust/tests/data/native_namespace_alias_settlement/probe.c](../../../../runtime/rust/tests/data/native_namespace_alias_settlement/probe.c)
- [runtime/rust/tests/data/native_namespace_alias_settlement/8.4.20.txt](../../../../runtime/rust/tests/data/native_namespace_alias_settlement/8.4.20.txt)
- [runtime/rust/tests/data/native_namespace_alias_settlement/8.5.19.txt](../../../../runtime/rust/tests/data/native_namespace_alias_settlement/8.5.19.txt)
- [runtime/rust/tests/data/native_namespace_alias_settlement/8.6.18.txt](../../../../runtime/rust/tests/data/native_namespace_alias_settlement/8.6.18.txt)
- [runtime/rust/tests/data/native_namespace_alias_settlement/9.0.4.txt](../../../../runtime/rust/tests/data/native_namespace_alias_settlement/9.0.4.txt)
- [runtime/rust/tests/data/native_namespace_alias_settlement/9.1.0.txt](../../../../runtime/rust/tests/data/native_namespace_alias_settlement/9.1.0.txt)
