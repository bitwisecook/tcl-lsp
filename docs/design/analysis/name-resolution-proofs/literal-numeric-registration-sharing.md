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

Status: `observed`. Version: 8.4.20 (capture association; launched patchlevel unqueried). Build: binary_sha256=2e36c1f22e543c069293ab674839e02743fed5a630b01482a5c063384c43c546; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; header_sha256=824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf; compile_exit=0; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original callback objects reached from Tcl_Eval or counted Tcl_EvalObjEx flags0, according to selected source mode.. Dialect: C Tcl.

Exact relevant rows, in original column order:

number	0	int	wideInt	-1	-1	-1	3	3137
number	1	wideInt	wideInt	1	-1	-1	4	3137
number	2	wideInt	wideInt	1	-1	-1	5	3137
number	3	int	wideInt	0	-1	-1	3	3137. Reference counts include the explicitly retained harness objects.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association; launched patchlevel unqueried). Build: binary_sha256=4bbf2251cca4964cdacab4696a43d38a8eb4b01aaa4311945f1d6d680e9852cf; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; header_sha256=c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5; compile_exit=0; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original callback objects reached from Tcl_Eval or counted Tcl_EvalObjEx flags0, according to selected source mode.. Dialect: C Tcl.

Exact relevant rows, in original column order:

number	0	none	int	-1	-1	-1	3	3137
number	1	int	int	1	-1	-1	4	3137
number	2	int	int	1	-1	-1	5	3137
number	3	none	int	0	-1	-1	3	3137. Reference counts include the explicitly retained harness objects.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association; launched patchlevel unqueried). Build: binary_sha256=ba6e843571f3451ebe34c628beb9e7284f7dcfcbff612711b0d400c839560d75; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; header_sha256=aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245; compile_exit=0; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original callback objects reached from Tcl_Eval or counted Tcl_EvalObjEx flags0, according to selected source mode.. Dialect: C Tcl.

Exact relevant rows, in original column order:

number	0	none	int	-1	-1	-1	3	3137
number	1	int	int	1	-1	-1	4	3137
number	2	int	int	1	-1	-1	5	3137
number	3	none	int	0	-1	-1	3	3137. Reference counts include the explicitly retained harness objects.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association; launched patchlevel unqueried). Build: binary_sha256=810dc9b9f342ef8731b90f73c83c206cff75deddc9652bdc73e03f7efefbff69; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; header_sha256=eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a; compile_exit=0; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original callback objects reached from Tcl_Eval or counted Tcl_EvalObjEx flags0, according to selected source mode.. Dialect: C Tcl.

Exact relevant rows, in original column order:

number	0	none	int	-1	-1	-1	3	3137
number	1	int	int	1	-1	-1	4	3137
number	2	int	int	1	-1	-1	5	3137
number	3	none	int	0	-1	-1	3	3137. Reference counts include the explicitly retained harness objects.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association; launched patchlevel unqueried). Build: binary_sha256=ca10a9ebb82d48d01fabc0356ec91dd6d0e0f8cb0baeecb1252e7a76e702eb43; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; header_sha256=30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950; compile_exit=0; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original callback objects reached from Tcl_Eval or counted Tcl_EvalObjEx flags0, according to selected source mode.. Dialect: C Tcl.

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

## Exact evidence

- `probe` (input): [rust/tcl-vm/tests/data/native_literal_pools/probe.c](../../../../rust/tcl-vm/tests/data/native_literal_pools/probe.c). SHA-256 `8ad5d50dae3f2264c99cb88d8ba9c73fdabfb9ad97d191a0aacb581b0e5f2919`. Exact source programs, counted opaque/numeric byte lengths and before-observer pointer/type sampling.
- `receipt` (provider): [rust/tcl-vm/tests/data/native_literal_pools/manifest.json](../../../../rust/tcl-vm/tests/data/native_literal_pools/manifest.json). SHA-256 `1f11c8c83bc5b9abaaf42c82332bed847f87bb9454ff0df5c874944d3ecc7545`. Original successful compile/run association, source/library/header/executable hashes and full output digests.
- `rows-tcl8.4` (observation): [rust/tcl-vm/tests/data/native_literal_pools/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/8.4.20.tsv). SHA-256 `6f9a2f02a57fcb5b97cae21cddcf784fa01587f1c866fcabce842bb8e4c33b41`. Full 21-row original output. Relevant one-based lines [1, 2, 3, 4]. Columns mode,call,before,after,same-first,children-equal,child-same-first,refs,hex.
- `rows-tcl8.5` (observation): [rust/tcl-vm/tests/data/native_literal_pools/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/8.5.19.tsv). SHA-256 `ba3263a820095a02cecc835fdffbe27302f61039be98364532d1eb69a60fb558`. Full 21-row original output. Relevant one-based lines [1, 2, 3, 4]. Columns mode,call,before,after,same-first,children-equal,child-same-first,refs,hex.
- `rows-tcl8.6` (observation): [rust/tcl-vm/tests/data/native_literal_pools/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/8.6.18.tsv). SHA-256 `06c8eeefe0491993248ff66311efbb1866a3c7fbfbae98f475e036e16437b142`. Full 21-row original output. Relevant one-based lines [1, 2, 3, 4]. Columns mode,call,before,after,same-first,children-equal,child-same-first,refs,hex.
- `rows-tcl9.0` (observation): [rust/tcl-vm/tests/data/native_literal_pools/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/9.0.4.tsv). SHA-256 `06c8eeefe0491993248ff66311efbb1866a3c7fbfbae98f475e036e16437b142`. Full 21-row original output. Relevant one-based lines [1, 2, 3, 4]. Columns mode,call,before,after,same-first,children-equal,child-same-first,refs,hex.
- `rows-tcl9.1` (observation): [rust/tcl-vm/tests/data/native_literal_pools/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/9.1.0.tsv). SHA-256 `06c8eeefe0491993248ff66311efbb1866a3c7fbfbae98f475e036e16437b142`. Full 21-row original output. Relevant one-based lines [1, 2, 3, 4]. Columns mode,call,before,after,same-first,children-equal,child-same-first,refs,hex.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs), `NativeLiteralPool`: Retains selected native allocation, registration and original object-array ownership separately from bytes.
- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs), `literal_pool::tests::local_array_lease_preserves_global_cache_and_retires_registration_before_guest_object` (linked): Checks cache/identity sharing under local registration leases and distinct new registration after retirement; no native reference count is inferred from the retained guest handle.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native launch or executed Rust result is claimed. A new native run must compile the retained exact probe against independently identified release headers and library, preserve the counted input channel, and record separate status/stdout/stderr before comparing the retained stream. Original absolute compiler paths and binary digests are capture metadata, not a currently runnable command. No native implementation explanation is inferred from the probe.
