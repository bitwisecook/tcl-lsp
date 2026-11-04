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

Status: `observed`. Version: 8.4.20 (capture association; launched patchlevel unqueried). Build: exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original-object callbacks reached from ASCII Tcl_Eval procedure source.. Dialect: C Tcl.

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

Status: `observed`. Version: 8.5.19 (capture association; launched patchlevel unqueried). Build: exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original-object callbacks reached from ASCII Tcl_Eval procedure source.. Dialect: C Tcl.

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

Status: `observed`. Version: 8.6.18 (capture association; launched patchlevel unqueried). Build: exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original-object callbacks reached from ASCII Tcl_Eval procedure source.. Dialect: C Tcl.

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

Status: `observed`. Version: 9.0.4 (capture association; launched patchlevel unqueried). Build: exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original-object callbacks reached from ASCII Tcl_Eval procedure source.. Dialect: C Tcl.

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

Status: `observed`. Version: 9.1.0 (capture association; launched patchlevel unqueried). Build: exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original-object callbacks reached from ASCII Tcl_Eval procedure source.. Dialect: C Tcl.

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

## Historical references

- `probe` (input): [rust/tcl-vm/tests/data/native_literal_pools/command-registration/probe.c](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-registration/probe.c). Exact original command/data role and source-order programs.
- `receipt` (provider): [rust/tcl-vm/tests/data/native_literal_pools/command-registration/manifest.json](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-registration/manifest.json). Original full stream digest, release association and recorded build fields; absence of compile status/header fields is not repaired from current builds.
- `rows-tcl8.4` (observation): [rust/tcl-vm/tests/data/native_literal_pools/command-registration/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-registration/8.4.20.tsv). Full exact captured stream; relevant stages None.
- `rows-tcl8.5` (observation): [rust/tcl-vm/tests/data/native_literal_pools/command-registration/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-registration/8.5.19.tsv). Full exact captured stream; relevant stages None.
- `rows-tcl8.6` (observation): [rust/tcl-vm/tests/data/native_literal_pools/command-registration/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-registration/8.6.18.tsv). Full exact captured stream; relevant stages None.
- `rows-tcl9.0` (observation): [rust/tcl-vm/tests/data/native_literal_pools/command-registration/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-registration/9.0.4.tsv). Full exact captured stream; relevant stages None.
- `rows-tcl9.1` (observation): [rust/tcl-vm/tests/data/native_literal_pools/command-registration/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-registration/9.1.0.tsv). Full exact captured stream; relevant stages None.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs), `NativeLiteralPool`: Selects original allocation and chronological effects from the retained compiler recipe.
- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs), `literal_pool::tests::command_registration_partitions_match_the_five_native_engines` (linked): Checks the independently issued namespace/qualification/data registration identity partitions across five releases.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native launch or executed Rust result is claimed. A new native run must compile the retained exact probe against independently identified release headers and library, preserve the counted input channel, and record separate status/stdout/stderr before comparing the retained stream. Original absolute compiler paths and binary digests are capture metadata, not a currently runnable command. No native implementation explanation is inferred from the probe.
