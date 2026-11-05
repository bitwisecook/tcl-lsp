# naming.literal.command-action.ordered-primary

Kind: `native-observation`

## Problem statement

A data occurrence before a command and the same data occurrence after a command can enter different compiler allocation paths. Equal literal bytes do not prove one original cache.

## Question

Does the retained data literal head or ::head arrive with cmdName after data-first versus command-first compilation?

## Conclusion

C8.4 data callbacks always retain cmdName. C8.5 data-first callbacks retain cmdName but command-first callbacks are untyped. C8.6–9.1 initially retain cmdName for all four controls. These original role/order windows do not establish a callable token from typePtr alone.

## Scope

Compiled public original-object callbacks sample the exact four source-order/qualification controls for command actions, or the exact eight namespace/command/data callbacks for registration. The probe prints object primary and, for registration only, pointer equality against the retained first original. Source is ASCII Tcl_Eval; no opaque key, dispatch body or native variable frame is inferred. Metadata records C release association/library/executable hashes. Runtime patchlevel, compiler version and full flags are unqueried; action compile status and registration header digest are not recorded. Jim and BIG-IP are not tested.

## Provider answers

### tcl8.4, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Public original-object callbacks reached from ASCII Tcl_Eval procedure source. Version: tcl8.4 8.4.20 (capture association; launched patchlevel unqueried); tcl8.6 8.6.18 (capture association; launched patchlevel unqueried); tcl9.0 9.0.4 (capture association; launched patchlevel unqueried); tcl9.1 9.1.0 (capture association; launched patchlevel unqueried).

Exact relevant rows:

data-relative	probe	cmdName
command-relative	probe	cmdName
data-absolute	probe	cmdName
command-absolute	probe	cmdName. A cmdName type is an observed primary, not proof of current command lookup.

### tcl8.5

Status: `observed`. Channel: Public original-object callbacks reached from ASCII Tcl_Eval procedure source. Version: tcl8.5 8.5.19 (capture association; launched patchlevel unqueried).

Exact relevant rows:

data-relative	probe	cmdName
command-relative	probe	none
data-absolute	probe	cmdName
command-absolute	probe	none. A cmdName type is an observed primary, not proof of current command lookup.

### jim, bigip

Status: `not-tested`.

No observation of this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs), `NativeLiteralPool`: Selects original allocation and chronological effects from the retained compiler recipe.
- [rust/tcl-vm/src/interp/native_command_names.rs](../../../../rust/tcl-vm/src/interp/native_command_names.rs), `interp::native_command_names::tests::ordered_literal_actions_match_sixty_original_native_observations` (linked): Checks the exact retained sixty original primary snapshots independently from actual original lookup currency.

## Evidence

- [rust/tcl-vm/tests/data/native_literal_pools/command-actions/probe.c](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-actions/probe.c)
- [rust/tcl-vm/tests/data/native_literal_pools/command-actions/manifest.json](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-actions/manifest.json)
- [rust/tcl-vm/tests/data/native_literal_pools/command-actions/8.4.20.txt](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-actions/8.4.20.txt)
- [rust/tcl-vm/tests/data/native_literal_pools/command-actions/8.5.19.txt](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-actions/8.5.19.txt)
- [rust/tcl-vm/tests/data/native_literal_pools/command-actions/8.6.18.txt](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-actions/8.6.18.txt)
- [rust/tcl-vm/tests/data/native_literal_pools/command-actions/9.0.4.txt](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-actions/9.0.4.txt)
- [rust/tcl-vm/tests/data/native_literal_pools/command-actions/9.1.0.txt](../../../../rust/tcl-vm/tests/data/native_literal_pools/command-actions/9.1.0.txt)
