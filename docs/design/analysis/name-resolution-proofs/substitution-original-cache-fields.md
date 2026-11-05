# naming.substitution.original-cache-fields

Kind: `native-observation`

## Problem statement

An original subst template can gain an executable representation without becoming an ordinary script or a procedure body. Reusing a script-bytecode header can invent proc ownership or lose the borrowed compiled local-name table. The reached descriptor, bytecode context and literal-child observations must be kept separate from logical result bytes and from unknown pre-execution state.

## Question

Which original template cache fields and literal-child headers are observable after subst and before the reached child getters on these C releases?

## Conclusion

C8.4/C8.5 retain no substcode descriptor on these original inputs. C8.6/C9.0/C9.1 expose substcode with the selected flag mask, actual compile/namespace epochs and opcode listing. Procedure callback rows have localCache present while procPtr is absent. L records print each original child type, resident state and refcount before that child getter, after the result getter. These observations neither establish pointer equality nor create an ordinary script/procedure compilation capability.

## Scope

Exact R/C/L rows from configured v1-v6 probes, excluding the v4 C8.4 process failure. C field inspection occurs after execution and the result getter; child inspection precedes each child getter. It is not an entered-opcode trace or a before-compilation window.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Counted Tcl_NewStringObj template bytes supplied to stock subst through Tcl_EvalObjv, and independently direct Tcl_SubstObj in v3-v6. Setup uses ASCII NUL-terminated Tcl_Eval source. Header windows precede the reached result string getter. Version: tcl8.4 8.4.20 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel); tcl8.5 8.5.19 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel).

Original template no substcode descriptor on the reached rows.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Counted Tcl_NewStringObj template bytes supplied to stock subst through Tcl_EvalObjv, and independently direct Tcl_SubstObj in v3-v6. Setup uses ASCII NUL-terminated Tcl_Eval source. Header windows precede the reached result string getter. Version: tcl8.6 8.6.18 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel); tcl9.0 9.0.4 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel); tcl9.1 9.1.0 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel).

Original substcode flags/epochs/opcode listing; procedure localCache present, procPtr absent. L headers precede child getters.

### jim, bigip

Status: `not-tested`.

This exact C substitution/private-header question was not tested on this provider.

## Consumer bindings

- [rust/tcl-vm/src/command/native_substitution_tests.rs](../../../../rust/tcl-vm/src/command/native_substitution_tests.rs), `command::native_substitution_tests::original_compiled_substitution_matches_counted_native_flags_and_completions` (linked): 480 C86/C90/C91 R-row comparisons: completion, original/result primary and resident state before reached result getter, counted byte result, template-only cache.
- [runtime/rust/src/interp/native_body_artifact/native_substitution.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_substitution.rs), `interp::native_body_artifact::native_substitution::tests::original_compiled_substitution_matches_native_template_windows` (linked): 480 C86/C90/C91 original native object R-row comparisons with independent substcode artifact; template is not ordinary body bytecode.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 1 | 0 | 0 | 0 |
| `isolated` | 1 | 0 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data/native_substitution_owner](../../../../rust/tcl-registry/tests/data/native_substitution_owner)
- [rust/tcl-registry/tests/data/native_substitution_owner/v1](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1)
- [rust/tcl-registry/tests/data/native_substitution_owner/v2](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2)
- [rust/tcl-registry/tests/data/native_substitution_owner/v3](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3)
- [rust/tcl-registry/tests/data/native_substitution_owner/v5](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5)
- [rust/tcl-registry/tests/data/native_substitution_owner/v6](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6)
- [rust/tcl-registry/tests/data/native_substitution_owner/v4](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4)
