# naming.source.original-command-lookup

Kind: `implementation-contract`

## Problem statement

Readonly source consumers must join a command use to its original declaration without treating reporting names or a possible root completion as proof of native dispatch.

## Question

How are original command lookup and declaration joins retained in source analysis, independently of a modeled completed command world?

## Conclusion

`invocation_targets_declaration_in` requires current consumer and declaration source images, original lookup and name-input agreement, a reference matching the original invocation site, and a selected definition allocation whose original declaration site matches. Direct joins also require the original publication slot and name policy; following a retained link is an explicit separate choice. `original_completed_command_world` separately projects a modeled whole-root Normal publication ledger only from its retained completed state, and withholds it for an unknown entry, unavailable world, observed source step or source execution. An original lookup can remain retained in a procedure body independently of whether the completed world is available. These source carriers supply no native dispatch, entered frame, executed completion or edit authority.

## Scope

Readonly original source lookup/declaration joins and modeled root command-world projection. Two fixed source-defined controls cover substituted command heads, consumed command names, selected world-available and unavailable shapes, and retained body lookups under Tcl 8.6 and 9.0 analysis. Their bindings attach no execution result. Provider behavior, callback execution and physical command-table state remain separate obligations.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

No provider execution for this source contract.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

No provider execution for this source contract.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

No provider execution for this source contract.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

No provider execution for this source contract.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

No provider execution for this source contract.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No provider execution for this source contract.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No appliance execution for this source contract.

## Historical references

No provider evidence or executed software test receipt is attached.

## Owner-resolution map

| Purpose | Existing owner | Limitation |
| --- | --- | --- |
| Source lookup and declaration join | `invocation_targets_declaration_in` and `reference_matches_declaration` | Current original source carriers are required; no runtime dispatch or edit authority follows. |
| Modeled completed command world | `original_completed_command_world` | Requires a retained completed Normal source state and withholds unavailable or unknown states. |

## Replay

Inspected source and linked control bodies only; no executable replay, test execution or provider execution is attached.
