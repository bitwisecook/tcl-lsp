# naming.literal.initial-number-primary

Kind: `native-observation`

## Problem statement

Compiler allocation can prime a numeric primary before any explicit getter, and raw-zero suffixes can affect that producer differently from a later numeric consumer. Assuming every numeric-looking spelling is preconverted would conflate compilation and conversion.

## Question

Which of eleven original counted numeric-looking literals arrive at the callback with an initial numeric primary before a getter runs?

## Conclusion

Only C8.4 arrives with int for 17, 0, -17, 17 followed by raw zero and suffix, 17 followed by raw zero, and positive i64 maximum. It arrives untyped for +17, 017, -0, negative i64 minimum and positive overflow. C8.5–9.1 arrive untyped for all eleven. Initial mode does not invoke a numeric getter, so this answers compiler allocation, not successful arithmetic or complete numeric grammar.

## Scope

The exact public C object callback records primary before/after its selected observer, original pointer equality, child equality, reference count and byte hex. Numeric/list callbacks enter ASCII Tcl_Eval source; opaque and eleven initial-number sources use counted Tcl_NewStringObj/Tcl_EvalObjEx flags0. The held first object and held first child are deliberate harness references. Capture metadata identifies five C releases; runtime patchlevel, compiler version and configure flags are unqueried. Jim and BIG-IP are not tested.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Public original callback objects reached from Tcl_Eval or counted Tcl_EvalObjEx flags0, according to selected source mode. Version: tcl8.4 8.4.20 (capture association; launched patchlevel unqueried).

Exact relevant rows, in original column order:

initial	0	int	int	-1	-1	-1	3	3137
initial	0	none	none	-1	-1	-1	3	2b3137
initial	0	none	none	-1	-1	-1	3	303137
initial	0	none	none	-1	-1	-1	3	2d30
initial	0	int	int	-1	-1	-1	3	30
initial	0	int	int	-1	-1	-1	3	2d3137
initial	0	int	int	-1	-1	-1	3	313700737566666978
initial	0	int	int	-1	-1	-1	3	313700
initial	0	int	int	-1	-1	-1	3	39323233333732303336383534373735383037
initial	0	none	none	-1	-1	-1	3	2d39323233333732303336383534373735383038
initial	0	none	none	-1	-1	-1	3	39323233333732303336383534373735383038. Reference counts include the explicitly retained harness objects.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Public original callback objects reached from Tcl_Eval or counted Tcl_EvalObjEx flags0, according to selected source mode. Version: tcl8.5 8.5.19 (capture association; launched patchlevel unqueried); tcl8.6 8.6.18 (capture association; launched patchlevel unqueried); tcl9.0 9.0.4 (capture association; launched patchlevel unqueried); tcl9.1 9.1.0 (capture association; launched patchlevel unqueried).

Exact relevant rows, in original column order:

initial	0	none	none	-1	-1	-1	3	3137
initial	0	none	none	-1	-1	-1	3	2b3137
initial	0	none	none	-1	-1	-1	3	303137
initial	0	none	none	-1	-1	-1	3	2d30
initial	0	none	none	-1	-1	-1	3	30
initial	0	none	none	-1	-1	-1	3	2d3137
initial	0	none	none	-1	-1	-1	3	313700737566666978
initial	0	none	none	-1	-1	-1	3	313700
initial	0	none	none	-1	-1	-1	3	39323233333732303336383534373735383037
initial	0	none	none	-1	-1	-1	3	2d39323233333732303336383534373735383038
initial	0	none	none	-1	-1	-1	3	39323233333732303336383534373735383038. Reference counts include the explicitly retained harness objects.

### jim, bigip

Status: `not-tested`.

No observation of this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs), `NativeLiteralPool`: Retains selected native allocation, registration and original object-array ownership separately from bytes.
- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs), `literal_pool::tests::native_opaque_literal_bytes_and_c84_initial_cache_match_captured_producers` (linked): Checks selected initial C8.4 numeric cache and negative-minimum decline; this does not cover every native row.

## Evidence

- [rust/tcl-vm/tests/data/native_literal_pools/probe.c](../../../../rust/tcl-vm/tests/data/native_literal_pools/probe.c)
- [rust/tcl-vm/tests/data/native_literal_pools/manifest.json](../../../../rust/tcl-vm/tests/data/native_literal_pools/manifest.json)
- [rust/tcl-vm/tests/data/native_literal_pools/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/8.4.20.tsv)
- [rust/tcl-vm/tests/data/native_literal_pools/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/8.5.19.tsv)
- [rust/tcl-vm/tests/data/native_literal_pools/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/8.6.18.tsv)
- [rust/tcl-vm/tests/data/native_literal_pools/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/9.0.4.tsv)
- [rust/tcl-vm/tests/data/native_literal_pools/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/9.1.0.tsv)
