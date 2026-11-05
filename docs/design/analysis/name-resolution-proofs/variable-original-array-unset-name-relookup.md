# naming.variable.original-array-unset-name-relookup

Kind: `native-observation`

## Problem statement

A callback selected by array unset can retarget its original source alias to another array. The whole-root and patterned-member operations require separate public observations across releases rather than a private-receiver inference from a shared source name.

## Question

What four public array/member existence flags do original whole-root and patterned-member array-unset sources return when the array callback retargets held from a to b?

## Conclusion

All five pinned C providers complete the whole-root source with guest code0 and 1 1 0 0: a and a(x) remain present, while b and b(x) are absent. For the patterned-member source, C8.4/C8.5 return code0 and 1 1 1 0; C8.6/C9.0/C9.1 return code0 and 1 0 1 1. These are the four exact written existence queries in order: array a, a(x), array b and b(x). The two original sources therefore distinguish the public release-specific target outcomes after the callback retargets held. Equal names or these existence flags do not identify private roots, members, pointers, alias allocations, lookup tokens or registration lifetimes, and do not prove a universal lookup/disposal algorithm. Jim rejects trace setup with guest code1 and invalid command name "trace" for both originals, supplying no unset/retarget answer. All twelve standalone host processes exit0 with empty stderr. The raw request’s proposed whole-lookup/member-held interpretation and C8.6 source-line references remain input hypotheses; native answers here are bounded to the measured public completion/results, separate from source inspection and software receiver owners.

## Scope

Two original ASCII LF programs, independently evaluated in fresh fully initialised interpreters using the unchanged counted-source CAPI driver. The original source alias setup, array callback, explicit unpatterned/patterned unset words and four public queries remain byte-identical. Whole source/request/driver/compile/ELF/process/code/result/required pin artifacts are retained without an added source envelope or API substitution. No private header/frame/operand identity, general callback traversal, BIG-IP or executed Rust assertion is supplied. The shared software NativeArrayUnsetMemberLookup selector requires the actual C family, release and distinct-array storage purpose: C8.4/C8.5 selects OriginalName and C8.6/C9.0/C9.1 selects SelectedArray; missing release, mixed dictionary storage, Jim and hosted contexts refuse this C rule. Runtime and VM byte patterned-array-unset adapters consume that selected rule, while whole-root CmdCore unset keeps its separate original-name lookup after the array callback. These are current implementation policies; the public original356 flags remain a distinct observation and do not prove private lookup/table/header identities.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Two independent fresh fullInit original source evaluations with separate VERSION and ORIGINAL hex completion/result channels; no original-source envelope or fallback. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

Whole-root original code0 result1 1 0 0; patterned-member original code0 result1 1 1 0. The four written public array/member existence flags are observed, not private receiver/lookup or alias identity. Two fresh host processes exit0 with empty stderr.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Two independent fresh fullInit original source evaluations with separate VERSION and ORIGINAL hex completion/result channels; no original-source envelope or fallback. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Whole-root original code0 result1 1 0 0; patterned-member original code0 result1 0 1 1. The four written public array/member existence flags are observed, not private receiver/lookup or alias identity. Two fresh host processes exit0 with empty stderr.

### jim

Status: `observed`. Channel: Two independent fresh fullInit original source evaluations with separate VERSION and ORIGINAL hex completion/result channels; no original-source envelope or fallback. Version: jim 0.84-9-g5bac7c9.

Both unchanged originals measure guest code1 and invalid command name "trace"; no array-unset target answer is supplied. Two fresh host processes exit0 with empty stderr.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Consumer bindings

- [rust/tcl-runtime-api/src/variable_destruction.rs](../../../../rust/tcl-runtime-api/src/variable_destruction.rs), `NativeArrayUnsetMemberLookup`: Transport OriginalName versus SelectedArray member-lookup purpose without issuing any physical receiver or Native operand/frame proof.
- [rust/tcl-registry/src/native_variable_destruction.rs](../../../../rust/tcl-registry/src/native_variable_destruction.rs), `InvocationDialect::array_unset_member_lookup`: Require actual C release/family and distinct-array storage; select original-name versus selected-array implementation purpose, refusing missing/Jim/hosted/mixed owners.
- [rust/tcl-registry/src/native_variable_destruction.rs](../../../../rust/tcl-registry/src/native_variable_destruction.rs), `native_variable_destruction::tests::array_unset_member_lookup_requires_its_selected_native_release` (linked): Pure selected C release rule is independent of genuine physical receiver: C84/85 OriginalName versus C86/90/91 SelectedArray, missing release/mixed dictionary/Jim/hosted refuse. No private native lookup identity is asserted.
- [runtime/rust/src/interp/native_variable_teardown.rs](../../../../runtime/rust/src/interp/native_variable_teardown.rs), `interp::native_variable_teardown::tests::original_array_unset_keeps_release_selected_member_lookup` (linked): Twelve whole unchanged original356 source completion/results compare the public release-specific flags, including Jim unavailable trace result. Public values do not identify private receiver/table/header or original operand/frame ownership.
- [rust/tcl-vm/src/cmd_array/native_unset_tests.rs](../../../../rust/tcl-vm/src/cmd_array/native_unset_tests.rs), `cmd_array::native_unset_tests::original_array_unset_matches_native_refills_and_release_selected_lookup` (linked): Twenty-four whole unchanged original355/356 source completions/results compare public refill and release-specific flags. Each original source identity remains distinct; no native private table/header/compiler/activation equivalence is inferred.

## Evidence

- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_array_unset_relookup356/8.4.20](../../../../rust/tcl-registry/tests/data/native_array_unset_relookup356/8.4.20)
- [rust/tcl-registry/tests/data/native_array_unset_relookup356/8.5.19](../../../../rust/tcl-registry/tests/data/native_array_unset_relookup356/8.5.19)
- [rust/tcl-registry/tests/data/native_array_unset_relookup356/8.6.18](../../../../rust/tcl-registry/tests/data/native_array_unset_relookup356/8.6.18)
- [rust/tcl-registry/tests/data/native_array_unset_relookup356/9.0.4](../../../../rust/tcl-registry/tests/data/native_array_unset_relookup356/9.0.4)
- [rust/tcl-registry/tests/data/native_array_unset_relookup356/9.1.0](../../../../rust/tcl-registry/tests/data/native_array_unset_relookup356/9.1.0)
- [rust/tcl-registry/tests](../../../../rust/tcl-registry/tests)
- [rust/tcl-registry/tests/data/native_array_unset_relookup356/jim](../../../../rust/tcl-registry/tests/data/native_array_unset_relookup356/jim)
