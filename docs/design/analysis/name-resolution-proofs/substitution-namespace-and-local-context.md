# naming.substitution.namespace-and-local-context

Kind: `native-observation`

## Problem statement

A namespace eval may read an existing global variable when an unqualified set does not create the expected namespace cell. That setup can make a correctly current template appear to reuse the wrong namespace. Fresh templates, explicit declarations, independent qualified reads and actual varFrame/cache booleans distinguish setup fallback from incorrect cache currency; procedure contexts separately test borrowed local names.

## Question

After explicit A/B namespace declarations, do fresh and reused direct templates resolve A, B and ROOT correctly, and what localCache/procPtr fields accompany P/Q procedure contexts?

## Conclusion

v6 independently reads A=41, B=42 and ROOT=524f4f54, and all C5 fresh/reused direct A/B substitutions return 41/42. P, repeated P, Q and returned P values match their actual formal values. On C8.6/C9.0/C9.1 the original substcode changes cached namespace with the actual varFrame and retains a localCache in procedure callbacks while procPtr remains absent. Earlier unqualified-set variants return ROOT and do not prove a wrong cache namespace.

## Scope

v6 explicit variable x A/B and independent qualified reads, fresh/reused direct original templates, actual varFrame/cache boolean observations, P/Q layouts. v2-v5 are separately retained setup controls. No numeric address or identity is inferred from equal epoch fields.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Counted Tcl_NewStringObj template bytes supplied to stock subst through Tcl_EvalObjv, and independently direct Tcl_SubstObj in v3-v6. Setup uses ASCII NUL-terminated Tcl_Eval source. Header windows precede the reached result string getter. Version: tcl8.4 8.4.20 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel).

v6 declared A/B/root hex41/42/524f4f54; fresh/reused direct A/B resolve41/42; P/Q match actual formal values. Direct simple-variable result string primary.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Counted Tcl_NewStringObj template bytes supplied to stock subst through Tcl_EvalObjv, and independently direct Tcl_SubstObj in v3-v6. Setup uses ASCII NUL-terminated Tcl_Eval source. Header windows precede the reached result string getter. Version: tcl8.5 8.5.19 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel); tcl8.6 8.6.18 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel); tcl9.0 9.0.4 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel); tcl9.1 9.1.0 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel).

v6 declared A/B/root hex41/42/524f4f54; fresh/reused direct A/B resolve41/42; P/Q match actual formal values. Direct simple-variable result none primary before getter.

### jim, bigip

Status: `not-tested`.

This exact C substitution/private-header question was not tested on this provider.

## Consumer bindings

- [rust/tcl-vm/src/command/native_substitution_tests.rs](../../../../rust/tcl-vm/src/command/native_substitution_tests.rs), `command::native_substitution_tests::original_substitution_changes_namespace_and_borrowed_local_cache_without_proc_header` (linked): Actual native template A/B/root/P/Q contexts, retained local-name table, no proc-header ownership, repeated/current cache and duplicate string-only separation.

## Evidence

- [rust/tcl-registry/tests/data/native_substitution_owner](../../../../rust/tcl-registry/tests/data/native_substitution_owner)
- [rust/tcl-registry/tests/data/native_substitution_owner/v1](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1)
- [rust/tcl-registry/tests/data/native_substitution_owner/v2](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2)
- [rust/tcl-registry/tests/data/native_substitution_owner/v3](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3)
- [rust/tcl-registry/tests/data/native_substitution_owner/v5](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5)
- [rust/tcl-registry/tests/data/native_substitution_owner/v6](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6)
- [rust/tcl-registry/tests/data/native_substitution_owner/v4](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4)
