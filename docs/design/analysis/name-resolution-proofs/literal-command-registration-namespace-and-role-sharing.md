# naming.literal.command-registration.namespace-and-role-sharing

Kind: `native-observation`

## Problem statement

Unqualified command literals belong to namespace lookup while fully qualified command literals and ordinary data may share a different interpreter registration. Reusing a cache by spelling would erase release and role boundaries.

## Question

Which original head and ::head command/data objects share the first procedure head across root and N procedure contexts?

## Conclusion

C8.4 shares both spellings across repeated root command, N command and data. C8.5 splits relative N and data objects but shares absolute command and data. C8.6–9.1 split relative N/data and absolute data, while absolute commands share across namespaces. Distinct data objects are untyped. These pointer/type observations do not prove resolution after command replacement.

## Scope

Compiled public original-object callbacks sample the exact four source-order/qualification controls for command actions, or the exact eight namespace/command/data callbacks for registration. The probe prints object primary and, for registration only, pointer equality against the retained first original. Source is ASCII Tcl_Eval; no opaque key, dispatch body or native variable frame is inferred. Metadata records C release association/library/executable hashes. Runtime patchlevel, compiler version and full flags are unqueried; action compile status and registration header digest are not recorded. Jim and BIG-IP are not tested.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (capture association; launched patchlevel unqueried). Build: binary_sha256=963fcaac048ddd95a264472a372925a5ab1b6c838372268aa90de54d31f74854; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original-object callbacks reached from ASCII Tcl_Eval procedure source.. Dialect: C Tcl.

Exact relevant rows:

relative	0	cmdName	-1
relative	1	cmdName	1
relative	2	cmdName	1
relative	3	cmdName	1
absolute	0	cmdName	-1
absolute	1	cmdName	1
absolute	2	cmdName	1
absolute	3	cmdName	1. A cmdName type is an observed primary, not proof of current command lookup.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association; launched patchlevel unqueried). Build: binary_sha256=a59f22ac1a2f0158b42bd6455964eab239bda149a127dacc62fb8865a9df8659; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original-object callbacks reached from ASCII Tcl_Eval procedure source.. Dialect: C Tcl.

Exact relevant rows:

relative	0	cmdName	-1
relative	1	cmdName	1
relative	2	cmdName	0
relative	3	none	0
absolute	0	cmdName	-1
absolute	1	cmdName	1
absolute	2	cmdName	1
absolute	3	cmdName	1. A cmdName type is an observed primary, not proof of current command lookup.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association; launched patchlevel unqueried). Build: binary_sha256=da5d2c067774e7af0e6752ef0a3f768e340fe1073aad20572a704917164d9dc8; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original-object callbacks reached from ASCII Tcl_Eval procedure source.. Dialect: C Tcl.

Exact relevant rows:

relative	0	cmdName	-1
relative	1	cmdName	1
relative	2	cmdName	0
relative	3	none	0
absolute	0	cmdName	-1
absolute	1	cmdName	1
absolute	2	cmdName	1
absolute	3	none	0. A cmdName type is an observed primary, not proof of current command lookup.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association; launched patchlevel unqueried). Build: binary_sha256=e9e4a9cacd2d4384fa4e5fda3f7039ecb259524d2deba6843e4768a0afcfb022; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original-object callbacks reached from ASCII Tcl_Eval procedure source.. Dialect: C Tcl.

Exact relevant rows:

relative	0	cmdName	-1
relative	1	cmdName	1
relative	2	cmdName	0
relative	3	none	0
absolute	0	cmdName	-1
absolute	1	cmdName	1
absolute	2	cmdName	1
absolute	3	none	0. A cmdName type is an observed primary, not proof of current command lookup.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association; launched patchlevel unqueried). Build: binary_sha256=6a06406634d07dfd3be81ebafa52e8c809075f2b1f84f62d96f75ceb3e2fb7e4; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original-object callbacks reached from ASCII Tcl_Eval procedure source.. Dialect: C Tcl.

Exact relevant rows:

relative	0	cmdName	-1
relative	1	cmdName	1
relative	2	cmdName	0
relative	3	none	0
absolute	0	cmdName	-1
absolute	1	cmdName	1
absolute	2	cmdName	1
absolute	3	none	0. A cmdName type is an observed primary, not proof of current command lookup.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No observation of this exact question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No observation of this exact question is attached for this provider.

## Exact evidence

- `probe` (input): [rust/tcl-vm/tests/data/native_literal_pools/command-registration/probe.c](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-registration/probe.c). SHA-256 `b64302d8c005f89684beb89ec5f8afc7188eb81feecc50ed6b0f387c115b20bc`. Exact original command/data role and source-order programs.
- `receipt` (provider): [rust/tcl-vm/tests/data/native_literal_pools/command-registration/manifest.json](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-registration/manifest.json). SHA-256 `ce389c324a0c5330130baec3b0f4c848dd13c2f6694339dad69b42d5de2df5d1`. Original full stream digest, release association and recorded build fields; absence of compile status/header fields is not repaired from current builds.
- `rows-tcl8.4` (observation): [rust/tcl-vm/tests/data/native_literal_pools/command-registration/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-registration/8.4.20.tsv). SHA-256 `ba407e004ce9b4965d709cf9b94c7277309642bc5e858118bde76cf3be081605`. Full exact captured stream; relevant stages None.
- `rows-tcl8.5` (observation): [rust/tcl-vm/tests/data/native_literal_pools/command-registration/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-registration/8.5.19.tsv). SHA-256 `d01ee4f9499bb8c6cc5ff5a80fbef2f296db21f288bcab7246a195db9f14f5de`. Full exact captured stream; relevant stages None.
- `rows-tcl8.6` (observation): [rust/tcl-vm/tests/data/native_literal_pools/command-registration/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-registration/8.6.18.tsv). SHA-256 `e5bca009b9a69589da46cf015db32790494f51a2a0178b6e4b8d0480b1c3f7d6`. Full exact captured stream; relevant stages None.
- `rows-tcl9.0` (observation): [rust/tcl-vm/tests/data/native_literal_pools/command-registration/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-registration/9.0.4.tsv). SHA-256 `e5bca009b9a69589da46cf015db32790494f51a2a0178b6e4b8d0480b1c3f7d6`. Full exact captured stream; relevant stages None.
- `rows-tcl9.1` (observation): [rust/tcl-vm/tests/data/native_literal_pools/command-registration/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-registration/9.1.0.tsv). SHA-256 `e5bca009b9a69589da46cf015db32790494f51a2a0178b6e4b8d0480b1c3f7d6`. Full exact captured stream; relevant stages None.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs), `NativeLiteralPool`: Selects original allocation and chronological effects from the retained compiler recipe.
- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs), `literal_pool::tests::command_registration_partitions_match_the_five_native_engines` (linked): Checks the independently issued namespace/qualification/data registration identity partitions across five releases.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native launch or executed Rust result is claimed. A new native run must compile the retained exact probe against independently identified release headers and library, preserve the counted input channel, and record separate status/stdout/stderr before comparing the retained stream. Original absolute compiler paths and binary digests are capture metadata, not a currently runnable command. No native implementation explanation is inferred from the probe.
