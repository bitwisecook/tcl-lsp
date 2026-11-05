# naming.tcloo.bootstrap-intrinsic-manufacture-method-allocations

Kind: `native-observation`

## Problem statement

A class role alone does not certify stock manufacture dispatch when own methods or mixins replace it.

## Question

Which actual own method type/visibility records implement create/new/configure, and does an own create override take precedence?

## Conclusion

C8.6/C9 classRoot class methods contain intrinsic create/new; its object facet has a flags-only unexported new record. C9 support owns public intrinsic configure. Installing an own create method on classRoot changes the actual own table and calling classRoot create returns OVERRIDE. Roles do not certify unchanged dispatch after such mutations.

## Scope

Configured original C private/public object APIs on fresh bootstrap and explicitly printed rename/lifecycle/dispatch controls. ASCII Tcl_EvalEx source controls and actual selected header fields; no compiler admission, cache, variable, method-body, observer, Registry-role or Rust parity capability. C8.4/C8.5/Jim expose no oo::class in the measured initialized interpreter. BIG-IP not tested.

## Provider answers

### tcl8.4, tcl8.5

Status: `unsupported`. Channel: Configured direct C object/private-header APIs plus exact ASCII Tcl_EvalEx source controls. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

Measured info commands ::oo::class is empty; bootstrap field API question has no object to inspect. Probe availability sentinel is not an object observation.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Configured direct C object/private-header APIs plus exact ASCII Tcl_EvalEx source controls. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

C8.6/C9 classRoot class methods contain intrinsic create/new; its object facet has a flags-only unexported new record. C9 support owns public intrinsic configure. Installing an own create method on classRoot changes the actual own table and calling classRoot create returns OVERRIDE. Roles do not certify unchanged dispatch after such mutations.

### jim

Status: `unsupported`. Channel: Exact ASCII Jim_Eval info patchlevel/info commands; no TclOO private-header API. Version: jim 0.84-9-g5bac7c9.

Measured info commands ::oo::class is empty; bootstrap field API question has no object to inspect. Probe availability sentinel is not an object observation.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1)
- [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2)
- [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3)
- [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/9.0.4/compile.stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/9.0.4/compile.stderr)
