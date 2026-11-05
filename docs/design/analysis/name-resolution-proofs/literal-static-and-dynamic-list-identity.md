# naming.literal.static-and-dynamic-list-identity

Kind: `native-observation`

## Problem statement

A compiler-created constant List and a List assembled from repeated runtime arguments can have the same bytes while owning different container and child identities. Deduplicating members from spelling would change those windows.

## Question

Do static list A A and dynamic list $x $x reuse containers and equal-member objects across two calls?

## Conclusion

C8.4/8.5 produce different static List containers on the two calls and both members are the same object. C8.6–9.1 reuse the static container while its two A members are distinct objects. All tested C releases produce different dynamic containers with shared Q members and retain the first child identity across calls. The pointer comparisons sample these original callbacks; they do not grant identity to a independently reconstructed List.

## Scope

The exact public C object callback records primary before/after its selected observer, original pointer equality, child equality, reference count and byte hex. Numeric/list callbacks enter ASCII Tcl_Eval source; opaque and eleven initial-number sources use counted Tcl_NewStringObj/Tcl_EvalObjEx flags0. The held first object and held first child are deliberate harness references. Capture metadata identifies five C releases; runtime patchlevel, compiler version and configure flags are unqueried. Jim and BIG-IP are not tested.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Public original callback objects reached from Tcl_Eval or counted Tcl_EvalObjEx flags0, according to selected source mode. Version: tcl8.4 8.4.20 (capture association; launched patchlevel unqueried); tcl8.5 8.5.19 (capture association; launched patchlevel unqueried).

Exact relevant rows, in original column order:

static-list	0	list	list	-1	1	-1	1	412041
static-list	1	list	list	0	1	1	1	412041
dynamic-list	0	list	list	-1	1	-1	1	512051
dynamic-list	1	list	list	0	1	1	1	512051. Reference counts include the explicitly retained harness objects.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Public original callback objects reached from Tcl_Eval or counted Tcl_EvalObjEx flags0, according to selected source mode. Version: tcl8.6 8.6.18 (capture association; launched patchlevel unqueried); tcl9.0 9.0.4 (capture association; launched patchlevel unqueried); tcl9.1 9.1.0 (capture association; launched patchlevel unqueried).

Exact relevant rows, in original column order:

static-list	0	list	list	-1	0	-1	2	412041
static-list	1	list	list	1	0	1	3	412041
dynamic-list	0	list	list	-1	1	-1	1	512051
dynamic-list	1	list	list	0	1	1	1	512051. Reference counts include the explicitly retained harness objects.

### jim, bigip

Status: `not-tested`.

No observation of this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs), `NativeLiteralPool`: Retains selected native allocation, registration and original object-array ownership separately from bytes.
- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs), `literal_pool::tests::private_constant_list_retains_distinct_original_members_and_declines_older_compiler` (linked): Checks retained C8.6 constant List container with distinct equal-valued original children; older compiler recipe is independently declined.

## Evidence

- [rust/tcl-vm/tests/data/native_literal_pools/probe.c](../../../../rust/tcl-vm/tests/data/native_literal_pools/probe.c)
- [rust/tcl-vm/tests/data/native_literal_pools/manifest.json](../../../../rust/tcl-vm/tests/data/native_literal_pools/manifest.json)
- [rust/tcl-vm/tests/data/native_literal_pools/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/8.4.20.tsv)
- [rust/tcl-vm/tests/data/native_literal_pools/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/8.5.19.tsv)
- [rust/tcl-vm/tests/data/native_literal_pools/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/8.6.18.tsv)
- [rust/tcl-vm/tests/data/native_literal_pools/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/9.0.4.tsv)
- [rust/tcl-vm/tests/data/native_literal_pools/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/9.1.0.tsv)
