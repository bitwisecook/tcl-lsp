# naming.substitution.direct-object-api

Kind: `native-observation`

## Problem statement

The public stock subst command and direct Tcl_SubstObj entry can differ in wrapper context and ownership. Assuming an object-vector invocation selects the same direct API frame would obscure the actual source purpose. Separate callbacks invoke Tcl_SubstObj on the original counted template in authentic namespace/procedure frames.

## Question

Do the direct Tcl_SubstObj callback controls preserve the observed namespace/formal result and original template representation independently of the stock command wrapper?

## Conclusion

The direct API controls v3-v6 produce the recorded namespace/formal values on all C5; the clarified v6 A/B setup returns A/B, and P/Q return their actual argument values. C8.4 yields a string result for simple variable templates; C8.5 and the compiled C8.6/C9 releases yield the observed none primary before the getter. Direct API rows are retained separately from ROOT command-vector rows; no Jim or appliance direct API is claimed.

## Scope

Original counted templates passed directly to Tcl_SubstObj inside actual observe callbacks; v6 setup controls are authoritative for namespace-cell interpretation. Native direct API and wrapper rows remain distinct channels of invocation.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Explicit-count original Tcl_NewStringObj templates passed directly to Tcl_SubstObj in actual observe callbacks, v3-v6 only; stock command-vector rows are separate wrapper controls. Version: tcl8.4 8.4.20 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel).

v6 declared A/B/root hex41/42/524f4f54; fresh/reused direct A/B resolve41/42; P/Q match actual formal values. Direct simple-variable result string primary.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Explicit-count original Tcl_NewStringObj templates passed directly to Tcl_SubstObj in actual observe callbacks, v3-v6 only; stock command-vector rows are separate wrapper controls. Version: tcl8.5 8.5.19 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel); tcl8.6 8.6.18 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel); tcl9.0 9.0.4 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel); tcl9.1 9.1.0 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel).

v6 declared A/B/root hex41/42/524f4f54; fresh/reused direct A/B resolve41/42; P/Q match actual formal values. Direct simple-variable result none primary before getter.

### jim, bigip

Status: `not-tested`.

This exact C substitution/private-header question was not tested on this provider.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_substitution_owner](../../../../rust/tcl-registry/tests/data/native_substitution_owner)
- [rust/tcl-registry/tests/data/native_substitution_owner/v1](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1)
- [rust/tcl-registry/tests/data/native_substitution_owner/v2](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2)
- [rust/tcl-registry/tests/data/native_substitution_owner/v3](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3)
- [rust/tcl-registry/tests/data/native_substitution_owner/v5](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5)
- [rust/tcl-registry/tests/data/native_substitution_owner/v6](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6)
- [rust/tcl-registry/tests/data/native_substitution_owner/v4](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4)
