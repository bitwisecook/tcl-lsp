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

Status: `observed`. Version: 8.4.20 (capture association; launched patchlevel unqueried). Build: compile_exit=0; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original callback objects reached from Tcl_Eval or counted Tcl_EvalObjEx flags0, according to selected source mode.. Dialect: C Tcl.

Exact relevant rows, in original column order:

number	0	int	wideInt	-1	-1	-1	3	3137
number	1	wideInt	wideInt	1	-1	-1	4	3137
number	2	wideInt	wideInt	1	-1	-1	5	3137
number	3	int	wideInt	0	-1	-1	3	3137. Reference counts include the explicitly retained harness objects.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association; launched patchlevel unqueried). Build: compile_exit=0; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original callback objects reached from Tcl_Eval or counted Tcl_EvalObjEx flags0, according to selected source mode.. Dialect: C Tcl.

Exact relevant rows, in original column order:

number	0	none	int	-1	-1	-1	3	3137
number	1	int	int	1	-1	-1	4	3137
number	2	int	int	1	-1	-1	5	3137
number	3	none	int	0	-1	-1	3	3137. Reference counts include the explicitly retained harness objects.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association; launched patchlevel unqueried). Build: compile_exit=0; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original callback objects reached from Tcl_Eval or counted Tcl_EvalObjEx flags0, according to selected source mode.. Dialect: C Tcl.

Exact relevant rows, in original column order:

number	0	none	int	-1	-1	-1	3	3137
number	1	int	int	1	-1	-1	4	3137
number	2	int	int	1	-1	-1	5	3137
number	3	none	int	0	-1	-1	3	3137. Reference counts include the explicitly retained harness objects.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association; launched patchlevel unqueried). Build: compile_exit=0; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original callback objects reached from Tcl_Eval or counted Tcl_EvalObjEx flags0, according to selected source mode.. Dialect: C Tcl.

Exact relevant rows, in original column order:

number	0	none	int	-1	-1	-1	3	3137
number	1	int	int	1	-1	-1	4	3137
number	2	int	int	1	-1	-1	5	3137
number	3	none	int	0	-1	-1	3	3137. Reference counts include the explicitly retained harness objects.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association; launched patchlevel unqueried). Build: compile_exit=0; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original callback objects reached from Tcl_Eval or counted Tcl_EvalObjEx flags0, according to selected source mode.. Dialect: C Tcl.

Exact relevant rows, in original column order:

number	0	none	int	-1	-1	-1	3	3137
number	1	int	int	1	-1	-1	4	3137
number	2	int	int	1	-1	-1	5	3137
number	3	none	int	0	-1	-1	3	3137. Reference counts include the explicitly retained harness objects.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No observation of this exact question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No observation of this exact question is attached for this provider.

## Historical references

- `probe` (input): [rust/tcl-vm/tests/data/native_literal_pools/probe.c](../../../../rust/tcl-vm/tests/data/native_literal_pools/probe.c). Exact source programs, counted opaque/numeric byte lengths and before-observer pointer/type sampling.
- `receipt` (provider): [rust/tcl-vm/tests/data/native_literal_pools/manifest.json](../../../../rust/tcl-vm/tests/data/native_literal_pools/manifest.json). Original successful compile/run association, source/library/header/executable hashes and full output digests.
- `rows-tcl8.4` (observation): [rust/tcl-vm/tests/data/native_literal_pools/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/8.4.20.tsv). Full 21-row original output. Relevant one-based lines [1, 2, 3, 4]. Columns mode,call,before,after,same-first,children-equal,child-same-first,refs,hex.
- `rows-tcl8.5` (observation): [rust/tcl-vm/tests/data/native_literal_pools/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/8.5.19.tsv). Full 21-row original output. Relevant one-based lines [1, 2, 3, 4]. Columns mode,call,before,after,same-first,children-equal,child-same-first,refs,hex.
- `rows-tcl8.6` (observation): [rust/tcl-vm/tests/data/native_literal_pools/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/8.6.18.tsv). Full 21-row original output. Relevant one-based lines [1, 2, 3, 4]. Columns mode,call,before,after,same-first,children-equal,child-same-first,refs,hex.
- `rows-tcl9.0` (observation): [rust/tcl-vm/tests/data/native_literal_pools/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/9.0.4.tsv). Full 21-row original output. Relevant one-based lines [1, 2, 3, 4]. Columns mode,call,before,after,same-first,children-equal,child-same-first,refs,hex.
- `rows-tcl9.1` (observation): [rust/tcl-vm/tests/data/native_literal_pools/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/9.1.0.tsv). Full 21-row original output. Relevant one-based lines [1, 2, 3, 4]. Columns mode,call,before,after,same-first,children-equal,child-same-first,refs,hex.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs), `NativeLiteralPool`: Retains selected native allocation, registration and original object-array ownership separately from bytes.
- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs), `literal_pool::tests::local_array_lease_preserves_global_cache_and_retires_registration_before_guest_object` (linked): Checks cache/identity sharing under local registration leases and distinct new registration after retirement; no native reference count is inferred from the retained guest handle.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native launch or executed Rust result is claimed. A new native run must compile the retained exact probe against independently identified release headers and library, preserve the counted input channel, and record separate status/stdout/stderr before comparing the retained stream. Original absolute compiler paths and binary digests are capture metadata, not a currently runnable command. No native implementation explanation is inferred from the probe.
