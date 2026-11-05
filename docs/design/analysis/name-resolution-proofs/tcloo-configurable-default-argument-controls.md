# naming.tcloo.configurable-default-argument-controls

Kind: `native-observation`

## Problem statement

An empty own constructor inventory must not produce an invented zero-only argument restriction. The selected default chain and actual manufacture completion remain independent obligations.

## Question

Does the created configurable class manufacture objects with zero arguments and with the exact two extra arguments one two?

## Conclusion

C9.0.4 and C9.1.0 successfully manufacture objects from the created class with zero arguments and with the exact two arguments one two. Both objects report class ::C. Other argument counts and constructor overrides are outside these captures.

## Scope

Exact unchanged ASCII cases.tcl source-file ingress to C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and pinned Jim 0.84-9-g5bac7c9. A separate unchanged ASCII stdin query records startup patchlevels. C8.4/C8.5/C8.6/Jim record configurable availability 0 and skip later controls; this does not classify general TclOO availability. No BIG-IP invocation, counted raw-string NUL input, native header/cache identity, Rust execution or entered body frame is inferred.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, jim

Status: `unsupported`. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; jim 0.84-9-g5bac7c9.

The reached available control reports code 0 and value 0 for ::oo::configurable; later controls for this question are skipped.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

default.new: code 0, ::C; default.extra.arguments: code 0, ::C.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap)
- [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.4](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.4)
- [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.5](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.5)
- [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.6](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.6)
- [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.0](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.0)
- [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.1](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.1)
- [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/jim](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/jim)
