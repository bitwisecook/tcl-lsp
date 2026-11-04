# naming.literal.command-action.replacement-primary

Kind: `native-observation`

## Problem statement

Deleting and recreating the command can retire a command-cache primary on only some retained original literals. Preserving bytes or merely observing cmdName cannot establish current worker currency.

## Question

Which retained literal cache primary changes after deleting and recreating head?

## Conclusion

All deleted-stage snapshots retain the previous primary. On recreation C8.6–9.1 change only the command-first relative retained data literal from cmdName to untyped; the absolute counterpart and both data-first literals retain cmdName. C8.4 changes none; C8.5 command-first literals were already untyped and remain so. The probe does not perform a new lookup through these held objects.

## Scope

Compiled public original-object callbacks sample the exact four source-order/qualification controls for command actions, or the exact eight namespace/command/data callbacks for registration. The probe prints object primary and, for registration only, pointer equality against the retained first original. Source is ASCII Tcl_Eval; no opaque key, dispatch body or native variable frame is inferred. Metadata records C release association/library/executable hashes. Runtime patchlevel, compiler version and full flags are unqueried; action compile status and registration header digest are not recorded. Jim and BIG-IP are not tested.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (capture association; launched patchlevel unqueried). Build: exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original-object callbacks reached from ASCII Tcl_Eval procedure source.. Dialect: C Tcl.

Exact relevant rows:

data-relative	deleted	cmdName
data-relative	recreated	cmdName
command-relative	deleted	cmdName
command-relative	recreated	cmdName
data-absolute	deleted	cmdName
data-absolute	recreated	cmdName
command-absolute	deleted	cmdName
command-absolute	recreated	cmdName. A cmdName type is an observed primary, not proof of current command lookup.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association; launched patchlevel unqueried). Build: exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original-object callbacks reached from ASCII Tcl_Eval procedure source.. Dialect: C Tcl.

Exact relevant rows:

data-relative	deleted	cmdName
data-relative	recreated	cmdName
command-relative	deleted	none
command-relative	recreated	none
data-absolute	deleted	cmdName
data-absolute	recreated	cmdName
command-absolute	deleted	none
command-absolute	recreated	none. A cmdName type is an observed primary, not proof of current command lookup.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association; launched patchlevel unqueried). Build: exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original-object callbacks reached from ASCII Tcl_Eval procedure source.. Dialect: C Tcl.

Exact relevant rows:

data-relative	deleted	cmdName
data-relative	recreated	cmdName
command-relative	deleted	cmdName
command-relative	recreated	none
data-absolute	deleted	cmdName
data-absolute	recreated	cmdName
command-absolute	deleted	cmdName
command-absolute	recreated	cmdName. A cmdName type is an observed primary, not proof of current command lookup.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association; launched patchlevel unqueried). Build: exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original-object callbacks reached from ASCII Tcl_Eval procedure source.. Dialect: C Tcl.

Exact relevant rows:

data-relative	deleted	cmdName
data-relative	recreated	cmdName
command-relative	deleted	cmdName
command-relative	recreated	none
data-absolute	deleted	cmdName
data-absolute	recreated	cmdName
command-absolute	deleted	cmdName
command-absolute	recreated	cmdName. A cmdName type is an observed primary, not proof of current command lookup.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association; launched patchlevel unqueried). Build: exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original-object callbacks reached from ASCII Tcl_Eval procedure source.. Dialect: C Tcl.

Exact relevant rows:

data-relative	deleted	cmdName
data-relative	recreated	cmdName
command-relative	deleted	cmdName
command-relative	recreated	none
data-absolute	deleted	cmdName
data-absolute	recreated	cmdName
command-absolute	deleted	cmdName
command-absolute	recreated	cmdName. A cmdName type is an observed primary, not proof of current command lookup.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No observation of this exact question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No observation of this exact question is attached for this provider.

## Historical references

- `probe` (input): [rust/tcl-vm/tests/data/native_literal_pools/command-actions/probe.c](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-actions/probe.c). Exact original command/data role and source-order programs.
- `receipt` (provider): [rust/tcl-vm/tests/data/native_literal_pools/command-actions/manifest.json](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-actions/manifest.json). Original full stream digest, release association and recorded build fields; absence of compile status/header fields is not repaired from current builds.
- `rows-tcl8.4` (observation): [rust/tcl-vm/tests/data/native_literal_pools/command-actions/8.4.20.txt](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-actions/8.4.20.txt). Full exact captured stream; relevant stages ['deleted', 'recreated'].
- `rows-tcl8.5` (observation): [rust/tcl-vm/tests/data/native_literal_pools/command-actions/8.5.19.txt](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-actions/8.5.19.txt). Full exact captured stream; relevant stages ['deleted', 'recreated'].
- `rows-tcl8.6` (observation): [rust/tcl-vm/tests/data/native_literal_pools/command-actions/8.6.18.txt](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-actions/8.6.18.txt). Full exact captured stream; relevant stages ['deleted', 'recreated'].
- `rows-tcl9.0` (observation): [rust/tcl-vm/tests/data/native_literal_pools/command-actions/9.0.4.txt](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-actions/9.0.4.txt). Full exact captured stream; relevant stages ['deleted', 'recreated'].
- `rows-tcl9.1` (observation): [rust/tcl-vm/tests/data/native_literal_pools/command-actions/9.1.0.txt](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-actions/9.1.0.txt). Full exact captured stream; relevant stages ['deleted', 'recreated'].

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs), `NativeLiteralPool`: Selects original allocation and chronological effects from the retained compiler recipe.
- [rust/tcl-vm/src/interp/native_command_names.rs](../../../../rust/tcl-vm/src/interp/native_command_names.rs), `interp::native_command_names::tests::ordered_literal_actions_match_sixty_original_native_observations` (linked): Checks the exact retained sixty original primary snapshots independently from actual original lookup currency.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native launch or executed Rust result is claimed. A new native run must compile the retained exact probe against independently identified release headers and library, preserve the counted input channel, and record separate status/stdout/stderr before comparing the retained stream. Original absolute compiler paths and binary digests are capture metadata, not a currently runnable command. No native implementation explanation is inferred from the probe.
