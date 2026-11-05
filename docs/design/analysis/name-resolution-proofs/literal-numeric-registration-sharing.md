# naming.literal.numeric-registration-sharing

Kind: `native-observation`

## Problem statement

A numeric literal may be shared between compiled procedures and retain a conversion primary, while a later procedure may allocate a different object after the registrations are removed. Equal bytes cannot establish that ownership lifetime.

## Question

Does original literal 17 share one object across p, repeated p and q, and what primary remains when r is defined after deleting p and q?

## Conclusion

All five C releases share the original 17 object for the second p and q callbacks. The later r callback has a different object even though the harness still holds the first. C8.4 begins with int and the explicit Wide getter changes it to wideInt; later C begins untyped and the getter changes it to int. These four snapshots do not prove an unobserved registration count or actual cleanup of the retained first object.

## Scope

The exact public C object callback records primary before/after its selected observer, original pointer equality, child equality, reference count and byte hex. Numeric/list callbacks enter ASCII Tcl_Eval source; opaque and eleven initial-number sources use counted Tcl_NewStringObj/Tcl_EvalObjEx flags0. The held first object and held first child are deliberate harness references. Capture metadata identifies five C releases; runtime patchlevel, compiler version and configure flags are unqueried. Jim and BIG-IP are not tested.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Public original callback objects reached from Tcl_Eval or counted Tcl_EvalObjEx flags0, according to selected source mode. Version: tcl8.4 8.4.20 (capture association; launched patchlevel unqueried).

Exact relevant rows, in original column order:

number	0	int	wideInt	-1	-1	-1	3	3137
number	1	wideInt	wideInt	1	-1	-1	4	3137
number	2	wideInt	wideInt	1	-1	-1	5	3137
number	3	int	wideInt	0	-1	-1	3	3137. Reference counts include the explicitly retained harness objects.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Public original callback objects reached from Tcl_Eval or counted Tcl_EvalObjEx flags0, according to selected source mode. Version: tcl8.5 8.5.19 (capture association; launched patchlevel unqueried); tcl8.6 8.6.18 (capture association; launched patchlevel unqueried); tcl9.0 9.0.4 (capture association; launched patchlevel unqueried); tcl9.1 9.1.0 (capture association; launched patchlevel unqueried).

Exact relevant rows, in original column order:

number	0	none	int	-1	-1	-1	3	3137
number	1	int	int	1	-1	-1	4	3137
number	2	int	int	1	-1	-1	5	3137
number	3	none	int	0	-1	-1	3	3137. Reference counts include the explicitly retained harness objects.

### jim, bigip

Status: `not-tested`.

No observation of this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs), `NativeLiteralPool`: Retains selected native allocation, registration and original object-array ownership separately from bytes.
- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs), `literal_pool::tests::local_array_lease_preserves_global_cache_and_retires_registration_before_guest_object` (linked): Checks cache/identity sharing under local registration leases and distinct new registration after retirement; no native reference count is inferred from the retained guest handle.

## Evidence

- [rust/tcl-vm/tests/data/native_literal_pools/probe.c](../../../../rust/tcl-vm/tests/data/native_literal_pools/probe.c)
- [rust/tcl-vm/tests/data/native_literal_pools/manifest.json](../../../../rust/tcl-vm/tests/data/native_literal_pools/manifest.json)
- [rust/tcl-vm/tests/data/native_literal_pools/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/8.4.20.tsv)
- [rust/tcl-vm/tests/data/native_literal_pools/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/8.5.19.tsv)
- [rust/tcl-vm/tests/data/native_literal_pools/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/8.6.18.tsv)
- [rust/tcl-vm/tests/data/native_literal_pools/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/9.0.4.tsv)
- [rust/tcl-vm/tests/data/native_literal_pools/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/9.1.0.tsv)
