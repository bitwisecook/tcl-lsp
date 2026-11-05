# naming.tcloo.bootstrap-factory-and-support-allocations

Kind: `native-observation`

## Problem statement

A command/namespace inventory cannot establish which actual allocation is a class factory or support class.

## Question

Which class/maker/root/superclass relationships and own lifecycle slots exist on the observed bootstrap allocations?

## Conclusion

C8.6 has object/class roots with the actual class root as maker, classRoot extending objectRoot and an intrinsic class constructor. C9 additionally has a configurable metaclass extending classRoot with its own intrinsic configurable constructor, and a distinct support class extending objectRoot with absent own constructor/destructor. Rename preserves the support creation identity; replacing its lifecycle slots changes their actual method types.

## Scope

Configured original C private/public object APIs on fresh bootstrap and explicitly printed rename/lifecycle/dispatch controls. ASCII Tcl_EvalEx source controls and actual selected header fields; no compiler admission, cache, variable, method-body, observer, Registry-role or Rust parity capability. C8.4/C8.5/Jim expose no oo::class in the measured initialized interpreter. BIG-IP not tested.

## Provider answers

### tcl8.4, tcl8.5

Status: `unsupported`. Channel: Configured direct C object/private-header APIs plus exact ASCII Tcl_EvalEx source controls. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

Measured info commands ::oo::class is empty; bootstrap field API question has no object to inspect. Probe availability sentinel is not an object observation.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Configured direct C object/private-header APIs plus exact ASCII Tcl_EvalEx source controls. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

C8.6 has object/class roots with the actual class root as maker, classRoot extending objectRoot and an intrinsic class constructor. C9 additionally has a configurable metaclass extending classRoot with its own intrinsic configurable constructor, and a distinct support class extending objectRoot with absent own constructor/destructor. Rename preserves the support creation identity; replacing its lifecycle slots changes their actual method types.

### jim

Status: `unsupported`. Channel: Exact ASCII Jim_Eval info patchlevel/info commands; no TclOO private-header API. Version: jim 0.84-9-g5bac7c9.

Measured info commands ::oo::class is empty; bootstrap field API question has no object to inspect. Probe availability sentinel is not an object observation.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `interp::native_oo_bootstrap_inventory_tests::original_oo_bootstrap_inventory_keeps_roles_methods_and_namespace_currency` (linked): Compare actual backend root/support lifecycle/method/namespace relationships and refuse a foreign interpreter, epoch or replaced command generation. This coverage binding supplies no Rust execution, native compiler or Normal claim.

## Evidence

- [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1)
- [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2)
- [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3)
- [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/9.0.4/compile.stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/9.0.4/compile.stderr)
