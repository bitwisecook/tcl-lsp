# naming.tcloo.native-forward-registration-prefix-boundaries

Kind: `native-observation`

## Problem statement

Forward registration stores prefix values and future evaluation is a separate operation; a definition-scope variable error cannot be relabelled as a forward registration failure.

## Question

Which exact fresh stock forward source scripts register, replace and later invoke their prefix under the six retained providers?

## Conclusion

C86/C90/C91 agree on static missing target, no target call at definition, replacements, empty target, required prefix and superclass controls. The complete dynamic PREFIX_CAPTURE source differs: C86 returns INITIAL EXTRA and C90/C91 error reading value. C84/C85/Jim report absent OO availability.

## Scope

39 exact version/answer rows from separate startup/availability and nine fresh-interpreter ASCII source scripts. No source-preflight/CPP, opaque name, cache/pointer/storage layout, callback absence, generic variable scope equivalence, implementation Normal transfer or arbitrary method chain claim. Native probe result getter records counted bytes, but inputs are ASCII NUL-terminated source, not original object vectors.

## Provider answers

### tcl8.4, tcl8.5, jim

Status: `unsupported`. Channel: separate actual ASCII info patchlevel/availability startup; each case uses a fresh interpreter and exact ASCII NUL-terminated Tcl_Eval or Jim_Eval source. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; jim 0.84-9-g5bac7c9.

Availability is empty; no forward script case is run.

### tcl8.6

Status: `observed`. Channel: separate actual ASCII info patchlevel/availability startup; each case uses a fresh interpreter and exact ASCII NUL-terminated Tcl_Eval or Jim_Eval source. Version: tcl8.6 8.6.18.

Missing-target forward registration succeeds; invocation returns invalid-command error. Definition invokes no target; script/forward replacement and empty target registration succeed, missing prefix errors, superclass forward retains a::b. The complete PREFIX_CAPTURE script returns INITIAL EXTRA.

### tcl9.0, tcl9.1

Status: `observed`. Channel: separate actual ASCII info patchlevel/availability startup; each case uses a fresh interpreter and exact ASCII NUL-terminated Tcl_Eval or Jim_Eval source. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Missing-target forward registration succeeds; invocation returns invalid-command error. Definition invokes no target; script/forward replacement and empty target registration succeed, missing prefix errors, superclass forward retains a::b. The complete PREFIX_CAPTURE script errors reading value; this does not locate a registration or capture failure separately from definition-scope substitution.

### bigip

Status: `not-tested`.

No appliance observation for the exact forward scripts.

## Consumer bindings

- [rust/tcl-registry/src/definer.rs](../../../../rust/tcl-registry/src/definer.rs), `definer::original_forward_tests::original_forward_layout_matches_captured_registration_boundaries` (linked): Pure selected layout compares the exact availability, registration/error/deferred-invocation rows and preserves the dynamic-script scope contrast; no implementation execution result inferred.

## Evidence

- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.4.20](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.4.20)
- [rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.5.19](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.5.19)
- [rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.6.18](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.6.18)
- [rust/tcl-registry/tests/data/native_tcloo_forward_registration/9.0.4](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/9.0.4)
- [rust/tcl-registry/tests/data/native_tcloo_forward_registration/9.1.0](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/9.1.0)
- [rust/tcl-registry/tests/data/native_tcloo_forward_registration/jim](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/jim)
