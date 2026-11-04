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

### tcl8.4

Status: `observed`. Version: 8.4.20 (capture association; launched patchlevel unqueried). Build: compile_exit=0; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original callback objects reached from Tcl_Eval or counted Tcl_EvalObjEx flags0, according to selected source mode.. Dialect: C Tcl.

Exact relevant rows, in original column order:

opaque	0	none	none	-1	-1	-1	3	ff007461696c
opaque	1	none	none	1	-1	-1	4	ff007461696c. Reference counts include the explicitly retained harness objects.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association; launched patchlevel unqueried). Build: compile_exit=0; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original callback objects reached from Tcl_Eval or counted Tcl_EvalObjEx flags0, according to selected source mode.. Dialect: C Tcl.

Exact relevant rows, in original column order:

opaque	0	none	none	-1	-1	-1	3	ff007461696c
opaque	1	none	none	1	-1	-1	4	ff007461696c. Reference counts include the explicitly retained harness objects.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association; launched patchlevel unqueried). Build: compile_exit=0; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original callback objects reached from Tcl_Eval or counted Tcl_EvalObjEx flags0, according to selected source mode.. Dialect: C Tcl.

Exact relevant rows, in original column order:

opaque	0	none	none	-1	-1	-1	3	ff007461696c
opaque	1	none	none	1	-1	-1	4	ff007461696c. Reference counts include the explicitly retained harness objects.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association; launched patchlevel unqueried). Build: compile_exit=0; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original callback objects reached from Tcl_Eval or counted Tcl_EvalObjEx flags0, according to selected source mode.. Dialect: C Tcl.

Exact relevant rows, in original column order:

opaque	0	none	none	-1	-1	-1	3	ff007461696c
opaque	1	none	none	1	-1	-1	4	ff007461696c. Reference counts include the explicitly retained harness objects.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association; launched patchlevel unqueried). Build: compile_exit=0; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original callback objects reached from Tcl_Eval or counted Tcl_EvalObjEx flags0, according to selected source mode.. Dialect: C Tcl.

Exact relevant rows, in original column order:

opaque	0	none	none	-1	-1	-1	3	ff007461696c
opaque	1	none	none	1	-1	-1	4	ff007461696c. Reference counts include the explicitly retained harness objects.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No observation of this exact question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No observation of this exact question is attached for this provider.

## Historical references

- `probe` (input): [rust/tcl-vm/tests/data/native_literal_pools/probe.c](../../../../rust/tcl-vm/tests/data/native_literal_pools/probe.c). Exact source programs, counted opaque/numeric byte lengths and before-observer pointer/type sampling.
- `receipt` (provider): [rust/tcl-vm/tests/data/native_literal_pools/manifest.json](../../../../rust/tcl-vm/tests/data/native_literal_pools/manifest.json). Original successful compile/run association, source/library/header/executable hashes and full output digests.
- `rows-tcl8.4` (observation): [rust/tcl-vm/tests/data/native_literal_pools/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/8.4.20.tsv). Full 21-row original output. Relevant one-based lines [9, 10]. Columns mode,call,before,after,same-first,children-equal,child-same-first,refs,hex.
- `rows-tcl8.5` (observation): [rust/tcl-vm/tests/data/native_literal_pools/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/8.5.19.tsv). Full 21-row original output. Relevant one-based lines [9, 10]. Columns mode,call,before,after,same-first,children-equal,child-same-first,refs,hex.
- `rows-tcl8.6` (observation): [rust/tcl-vm/tests/data/native_literal_pools/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/8.6.18.tsv). Full 21-row original output. Relevant one-based lines [9, 10]. Columns mode,call,before,after,same-first,children-equal,child-same-first,refs,hex.
- `rows-tcl9.0` (observation): [rust/tcl-vm/tests/data/native_literal_pools/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/9.0.4.tsv). Full 21-row original output. Relevant one-based lines [9, 10]. Columns mode,call,before,after,same-first,children-equal,child-same-first,refs,hex.
- `rows-tcl9.1` (observation): [rust/tcl-vm/tests/data/native_literal_pools/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/9.1.0.tsv). Full 21-row original output. Relevant one-based lines [9, 10]. Columns mode,call,before,after,same-first,children-equal,child-same-first,refs,hex.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs), `NativeLiteralPool`: Retains selected native allocation, registration and original object-array ownership separately from bytes.
- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs), `literal_pool::tests::native_opaque_literal_bytes_and_c84_initial_cache_match_captured_producers` (linked): Checks exact opaque byte retention independently from initial numeric-primary rules.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native launch or executed Rust result is claimed. A new native run must compile the retained exact probe against independently identified release headers and library, preserve the counted input channel, and record separate status/stdout/stderr before comparing the retained stream. Original absolute compiler paths and binary digests are capture metadata, not a currently runnable command. No native implementation explanation is inferred from the probe.
