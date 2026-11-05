# naming.literal.counted-opaque-source-object

Kind: `native-observation`

## Problem statement

A counted source can contain raw FF and a zero byte that an ordinary C string would truncate. Reconstructing its literal from display bytes would lose the original counted value and sharing observation.

## Question

Does a compiled opaque literal FF 00 tail retain exact bytes and reuse the original callback object on its second call?

## Conclusion

All five C captures return the exact six bytes FF 00 74 61 69 6C with an untyped primary, and the second callback receives the same original object. The source is a counted Tcl_NewStringObj evaluated with Tcl_EvalObjEx flags0; this observation is separate from CString grammar, UTF character ingress and every consumer lookup key.

## Scope

The exact public C object callback records primary before/after its selected observer, original pointer equality, child equality, reference count and byte hex. Numeric/list callbacks enter ASCII Tcl_Eval source; opaque and eleven initial-number sources use counted Tcl_NewStringObj/Tcl_EvalObjEx flags0. The held first object and held first child are deliberate harness references. Capture metadata identifies five C releases; runtime patchlevel, compiler version and configure flags are unqueried. Jim and BIG-IP are not tested.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Public original callback objects reached from Tcl_Eval or counted Tcl_EvalObjEx flags0, according to selected source mode. Version: tcl8.4 8.4.20 (capture association; launched patchlevel unqueried); tcl8.5 8.5.19 (capture association; launched patchlevel unqueried); tcl8.6 8.6.18 (capture association; launched patchlevel unqueried); tcl9.0 9.0.4 (capture association; launched patchlevel unqueried); tcl9.1 9.1.0 (capture association; launched patchlevel unqueried).

Exact relevant rows, in original column order:

opaque	0	none	none	-1	-1	-1	3	ff007461696c
opaque	1	none	none	1	-1	-1	4	ff007461696c. Reference counts include the explicitly retained harness objects.

### jim, bigip

Status: `not-tested`.

No observation of this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs), `NativeLiteralPool`: Retains selected native allocation, registration and original object-array ownership separately from bytes.
- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs), `literal_pool::tests::native_opaque_literal_bytes_and_c84_initial_cache_match_captured_producers` (linked): Checks exact opaque byte retention independently from initial numeric-primary rules.

## Evidence

- [rust/tcl-vm/tests/data/native_literal_pools/probe.c](../../../../rust/tcl-vm/tests/data/native_literal_pools/probe.c)
- [rust/tcl-vm/tests/data/native_literal_pools/manifest.json](../../../../rust/tcl-vm/tests/data/native_literal_pools/manifest.json)
- [rust/tcl-vm/tests/data/native_literal_pools/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/8.4.20.tsv)
- [rust/tcl-vm/tests/data/native_literal_pools/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/8.5.19.tsv)
- [rust/tcl-vm/tests/data/native_literal_pools/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/8.6.18.tsv)
- [rust/tcl-vm/tests/data/native_literal_pools/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/9.0.4.tsv)
- [rust/tcl-vm/tests/data/native_literal_pools/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/9.1.0.tsv)
