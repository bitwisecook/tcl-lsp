# naming.tcloo.configurable-bootstrap-mixin

Kind: `native-observation`

## Problem statement

A class created by configurable retains a support class as a mixin. Consumers must distinguish that instance method provider from the factory superclass and the created class own methods.

## Question

Which factory, superclass and support mixin identities are reported for the literal configurable class created by this source?

## Conclusion

C9.0.4 and C9.1.0 report the created class maker as ::oo::configurable, its superclass as ::oo::object and its mixin as ::oo::configuresupport::configurable. The factory superclass is independently ::oo::class.

## Scope

Exact unchanged ASCII cases.tcl source-file ingress to C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and pinned Jim 0.84-9-g5bac7c9. A separate unchanged ASCII stdin query records startup patchlevels. C8.4/C8.5/C8.6/Jim record configurable availability 0 and skip later controls; this does not classify general TclOO availability. No BIG-IP invocation, counted raw-string NUL input, native header/cache identity, Rust execution or entered body frame is inferred.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, jim

Status: `unsupported`. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; jim 0.84-9-g5bac7c9.

The reached available control reports code 0 and value 0 for ::oo::configurable; later controls for this question are skipped.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

created.class: ::oo::configurable; created.superclasses: ::oo::object; created.mixins: ::oo::configuresupport::configurable; factory.superclasses: ::oo::class; support.superclasses: ::oo::object; support.mixins: empty; all code 0.

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
