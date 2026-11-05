# naming.variable.original-undefined-array-member-retirement

Kind: `native-observation`

## Problem statement

An unset trace can be registered on an array member without assigning that member a value. A defined-value enumeration cannot by itself answer whether the original trace is called when the array is unset, its procedure returns, or its namespace is deleted.

## Question

What original array/member existence pair and callback log do three independent sources return when an unset trace is registered on an undefined array member and the array is explicitly unset, its procedure frame exits, or its namespace is deleted?

## Conclusion

Each pinned C8.4–C9.1 provider completes explicit-unset and frame-exit with code0 and {1 0} MEMBER; namespace-delete completes with code0 and {1 0} MEMBER 0. The public before pair reports an array and no defined a(x) value, and the finite later log contains MEMBER. The namespace case also reports the namespace absent afterward. These observations show callback participation in these exact three sources; they do not establish private member/cell/header identity, a general enumeration algorithm or a Native entered-frame grant. Jim returns guest code1 and invalid command name "trace" in all three originals, supplying no retirement-callback answer. All eighteen standalone processes exit0 with empty stderr. Source/API controls and software physical inventory identities require their own owners and executed receipts.

## Scope

Three untouched ASCII LF sources, independently evaluated in six fresh fullInit interpreters by the unchanged counted-source CAPI driver. Whole source hashes, VERSION/ORIGINAL channels, process stdout/stderr/receipts, six compile/ELF artifacts and exact required header/archive/build/source pins are retained. Defined-value queries, arbitrary trace registration order, private object identity, BIG-IP and executed Rust controls are outside this observation. The linked Runtime source comparator covers all eighteen whole original public code/results, including Jim trace refusal. Separately, shared array/frame destruction uses its genuinely retained physical member inventory, which can include registered undefined members rather than only defined values. That software owner does not infer Native member identity from the public before pair or callback log; defined-value queries and generated frame-pop admission keep their independent purposes.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Three independent fresh fullInit counted-source API evaluations, separate VERSION and ORIGINAL hex channels, no original-source envelope. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

All three originals measure guest code0: explicit-unset/frame-exit {1 0} MEMBER; namespace-delete {1 0} MEMBER 0. The exact public before pair, callback log and namespace flag are observed, with no private member/cell identity. Three fresh host processes exit0 with empty stderr.

### jim

Status: `observed`. Channel: Three independent fresh fullInit counted-source API evaluations, separate VERSION and ORIGINAL hex channels, no original-source envelope. Version: jim 0.84-9-g5bac7c9.

All three originals measure guest code1 and invalid command name "trace"; no unset-callback answer is supplied. Three fresh host processes exit0 with empty stderr.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Consumer bindings

- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::finish_array_unset`: Visit genuine retained array member inventory, including undefined registered entries; preserve independent native argument/frame purposes and actual member registration horizon.
- [runtime/rust/src/interp/native_variable_teardown.rs](../../../../runtime/rust/src/interp/native_variable_teardown.rs), `interp::native_variable_teardown::tests::original_undefined_member_callbacks_match_six_native_retirement_sources` (linked): Eighteen whole original source/public completion-result comparisons retain explicit/frame/namespace cases and Jim unavailable trace outcome. Public callback participation is separate from Rust physical member inventory and supplies no private identity or generated frame-pop grant.

## Evidence

- [rust/tcl-registry/tests/data/native_array_undefined_member350](../../../../rust/tcl-registry/tests/data/native_array_undefined_member350)
- [rust/tcl-registry/tests/data/native_array_undefined_member350/8.4.20](../../../../rust/tcl-registry/tests/data/native_array_undefined_member350/8.4.20)
- [rust/tcl-registry/tests/data/native_array_undefined_member350/8.5.19](../../../../rust/tcl-registry/tests/data/native_array_undefined_member350/8.5.19)
- [rust/tcl-registry/tests/data/native_array_undefined_member350/8.6.18](../../../../rust/tcl-registry/tests/data/native_array_undefined_member350/8.6.18)
- [rust/tcl-registry/tests/data/native_array_undefined_member350/9.0.4](../../../../rust/tcl-registry/tests/data/native_array_undefined_member350/9.0.4)
- [rust/tcl-registry/tests/data/native_array_undefined_member350/9.1.0](../../../../rust/tcl-registry/tests/data/native_array_undefined_member350/9.1.0)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_array_undefined_member350/jim](../../../../rust/tcl-registry/tests/data/native_array_undefined_member350/jim)
