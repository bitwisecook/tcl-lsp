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

Status: `observed`. Version: 8.4.20 (capture association; launched patchlevel unqueried). Build: binary_sha256=967b5f8fbe53600a82e24d2c4fd619623c036fc9ab26302a011838ba89d33093; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; header_sha256=824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original-object callbacks reached from ASCII Tcl_Eval procedure source.. Dialect: C Tcl.

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

Status: `observed`. Version: 8.5.19 (capture association; launched patchlevel unqueried). Build: binary_sha256=d274a114a3408f5269edc7c8bec88178e403d3aaf72b3aaed69be6a59ae07df4; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; header_sha256=c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original-object callbacks reached from ASCII Tcl_Eval procedure source.. Dialect: C Tcl.

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

Status: `observed`. Version: 8.6.18 (capture association; launched patchlevel unqueried). Build: binary_sha256=c70f7d8b7c2ee00ffbac9ff94b511ee0059b4c61464dea406e504f3f3c9f3819; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; header_sha256=aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original-object callbacks reached from ASCII Tcl_Eval procedure source.. Dialect: C Tcl.

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

Status: `observed`. Version: 9.0.4 (capture association; launched patchlevel unqueried). Build: binary_sha256=7e60c7a0dcebc951eba066b617c15515bb7ce21ab4105d402813726c31836628; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; header_sha256=eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original-object callbacks reached from ASCII Tcl_Eval procedure source.. Dialect: C Tcl.

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

Status: `observed`. Version: 9.1.0 (capture association; launched patchlevel unqueried). Build: binary_sha256=324632a0274597cb9222f35e5c017de25d67c3813a3231784b45f1887ce7add9; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; header_sha256=30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Public original-object callbacks reached from ASCII Tcl_Eval procedure source.. Dialect: C Tcl.

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

## Exact evidence

- `probe` (input): [rust/tcl-vm/tests/data/native_literal_pools/command-actions/probe.c](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-actions/probe.c). SHA-256 `95376e461b9b548cb6ad1fefd0d657028b3700cb08624d814c0def565f998ae4`. Exact original command/data role and source-order programs.
- `receipt` (provider): [rust/tcl-vm/tests/data/native_literal_pools/command-actions/manifest.json](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-actions/manifest.json). SHA-256 `0dad3f8b7aa1e5d82d14766412ffaef6f1e5f7578ee395f1194978585f37e2ed`. Original full stream digest, release association and recorded build fields; absence of compile status/header fields is not repaired from current builds.
- `rows-tcl8.4` (observation): [rust/tcl-vm/tests/data/native_literal_pools/command-actions/8.4.20.txt](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-actions/8.4.20.txt). SHA-256 `8afe2e9e967f23f2972f9eb088c67c299245202260304d845a7f28f6d6c1ada9`. Full exact captured stream; relevant stages ['deleted', 'recreated'].
- `rows-tcl8.5` (observation): [rust/tcl-vm/tests/data/native_literal_pools/command-actions/8.5.19.txt](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-actions/8.5.19.txt). SHA-256 `be8bc459681ef04aa6da6b3238eaece719d6a76a296922cd2d6d7bf56d070f3e`. Full exact captured stream; relevant stages ['deleted', 'recreated'].
- `rows-tcl8.6` (observation): [rust/tcl-vm/tests/data/native_literal_pools/command-actions/8.6.18.txt](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-actions/8.6.18.txt). SHA-256 `51e76c6bf1c7983c5eca7d0b23fe86ae1d6cac77bda708a64b2e8a6cce80754f`. Full exact captured stream; relevant stages ['deleted', 'recreated'].
- `rows-tcl9.0` (observation): [rust/tcl-vm/tests/data/native_literal_pools/command-actions/9.0.4.txt](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-actions/9.0.4.txt). SHA-256 `51e76c6bf1c7983c5eca7d0b23fe86ae1d6cac77bda708a64b2e8a6cce80754f`. Full exact captured stream; relevant stages ['deleted', 'recreated'].
- `rows-tcl9.1` (observation): [rust/tcl-vm/tests/data/native_literal_pools/command-actions/9.1.0.txt](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-actions/9.1.0.txt). SHA-256 `51e76c6bf1c7983c5eca7d0b23fe86ae1d6cac77bda708a64b2e8a6cce80754f`. Full exact captured stream; relevant stages ['deleted', 'recreated'].

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs), `NativeLiteralPool`: Selects original allocation and chronological effects from the retained compiler recipe.
- [rust/tcl-vm/src/interp/native_command_names.rs](../../../../rust/tcl-vm/src/interp/native_command_names.rs), `interp::native_command_names::tests::ordered_literal_actions_match_sixty_original_native_observations` (linked): Checks the exact retained sixty original primary snapshots independently from actual original lookup currency.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native launch or executed Rust result is claimed. A new native run must compile the retained exact probe against independently identified release headers and library, preserve the counted input channel, and record separate status/stdout/stderr before comparing the retained stream. Original absolute compiler paths and binary digests are capture metadata, not a currently runnable command. No native implementation explanation is inferred from the probe.
