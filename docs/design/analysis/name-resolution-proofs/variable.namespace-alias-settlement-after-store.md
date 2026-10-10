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

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: not recorded in this manifest. Channel: ASCII source script via retained public C probe. Dialect: Tcl.

traced 1 {variable "v" has traces: can't use for upvar} X
scalarArray 1 {can't define "arr(k)": name refers to an element in an array} 0 0 SCALAR
rebound 0 REPLACED REPLACED

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: not recorded in this manifest. Channel: ASCII source script via retained public C probe. Dialect: Tcl.

traced 1 {variable "v" has traces: can't use for upvar} X
scalarArray 1 {can't define "arr(k)": name refers to an element in an array} 0 0 SCALAR
rebound 0 REPLACED REPLACED

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: not recorded in this manifest. Channel: ASCII source script via retained public C probe. Dialect: Tcl.

traced 1 {variable "v" has traces: can't use for upvar} X
scalarArray 1 {can't define "arr(k)": name refers to an element in an array} 0 0 SCALAR
rebound 0 REPLACED REPLACED

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: not recorded in this manifest. Channel: ASCII source script via retained public C probe. Dialect: Tcl.

traced 1 {variable "v" has traces: can't use for upvar} X
scalarArray 1 {can't define "arr(k)": name refers to an element in an array} 0 0 SCALAR
rebound 0 REPLACED REPLACED

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: not recorded in this manifest. Channel: ASCII source script via retained public C probe. Dialect: Tcl.

traced 1 {variable "v" has traces: can't use for upvar} X
scalarArray 1 {can't define "arr(k)": name refers to an element in an array} 0 0 SCALAR
rebound 0 REPLACED REPLACED

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `capture` (provider): [runtime/rust/tests/data/native_namespace_alias_settlement/manifest.json](../../../../runtime/rust/tests/data/native_namespace_alias_settlement/manifest.json). SHA-256 `7c39e18ba5d7d3ded4aeaae7d112ba160e0fab9c86038a5954d6326fae0535d6`. Exact original native provenance/captured observations.
- `input-source.tcl` (input): [runtime/rust/tests/data/native_namespace_alias_settlement/source.tcl](../../../../runtime/rust/tests/data/native_namespace_alias_settlement/source.tcl). SHA-256 `e0bf753afe5e94ebb1a96da5d515278b17dc2b6538619c521f877354ad09b65a`. Retained exact script/case bytes for the stated question.
- `input-probe.c` (input): [runtime/rust/tests/data/native_namespace_alias_settlement/probe.c](../../../../runtime/rust/tests/data/native_namespace_alias_settlement/probe.c). SHA-256 `7759c929348a994705c71747e8ad3b2cc14db6ae67bca5ec7c221d0e5b6bcbe4`. Retained exact script/case bytes for the stated question.
- `rows-tcl8.4` (observation): [runtime/rust/tests/data/native_namespace_alias_settlement/8.4.20.txt](../../../../runtime/rust/tests/data/native_namespace_alias_settlement/8.4.20.txt). SHA-256 `cbc0925ac5c82f88bc4f183b68f1e7bd4249200d8545418f6fba2d2d9a035df7`. Exact three completed native catch/result/post-store rows.
- `rows-tcl8.5` (observation): [runtime/rust/tests/data/native_namespace_alias_settlement/8.5.19.txt](../../../../runtime/rust/tests/data/native_namespace_alias_settlement/8.5.19.txt). SHA-256 `cbc0925ac5c82f88bc4f183b68f1e7bd4249200d8545418f6fba2d2d9a035df7`. Exact three completed native catch/result/post-store rows.
- `rows-tcl8.6` (observation): [runtime/rust/tests/data/native_namespace_alias_settlement/8.6.18.txt](../../../../runtime/rust/tests/data/native_namespace_alias_settlement/8.6.18.txt). SHA-256 `cbc0925ac5c82f88bc4f183b68f1e7bd4249200d8545418f6fba2d2d9a035df7`. Exact three completed native catch/result/post-store rows.
- `rows-tcl9.0` (observation): [runtime/rust/tests/data/native_namespace_alias_settlement/9.0.4.txt](../../../../runtime/rust/tests/data/native_namespace_alias_settlement/9.0.4.txt). SHA-256 `cbc0925ac5c82f88bc4f183b68f1e7bd4249200d8545418f6fba2d2d9a035df7`. Exact three completed native catch/result/post-store rows.
- `rows-tcl9.1` (observation): [runtime/rust/tests/data/native_namespace_alias_settlement/9.1.0.txt](../../../../runtime/rust/tests/data/native_namespace_alias_settlement/9.1.0.txt). SHA-256 `cbc0925ac5c82f88bc4f183b68f1e7bd4249200d8545418f6fba2d2d9a035df7`. Exact three completed native catch/result/post-store rows.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `namespace_alias_settlement_matches_all_15_native_callback_and_error_results`: Current independent Rust fixture comparison.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::namespace_alias_settlement_matches_all_15_native_callback_and_error_results` (linked): Compares the independently scoped original alias/argument outcome with retained native rows; no native authority follows from a Rust result.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cc",
  "-I/path/to/recorded/include",
  "runtime/rust/tests/data/native_namespace_alias_settlement/probe.c",
  "/path/to/recorded/libtcl.so",
  "-o",
  "/tmp/namespace-alias-probe"
]
```

Compile the retained public C probe against the same recorded provider, then execute /tmp/namespace-alias-probe runtime/rust/tests/data/native_namespace_alias_settlement/source.tcl in a fresh process. The probe reads the full source file and calls Tcl_EvalEx with its byte count. Require the three complete stdout rows to match the retained provider file, empty stderr and process exit 0. The original engine build is not fully recorded; no fresh replay or direct Tclsh equivalence is claimed.
