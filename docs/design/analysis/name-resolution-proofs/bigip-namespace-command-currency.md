# naming.bigip.namespace-command-currency

Kind: `native-observation`

## Problem statement

A command name and its printed namespace can outlive or differ from the command publication. Retaining a stale rename/delete result or treating an F5 route as Tcl namespace identity would mislead lookup.

## Question

What do the recorded namespace/procedure/rename/command-lookup controls show in each execution context?

## Conclusion

The exact matrix retains mutation visibility, available command doors and context-specific failures. Its F5 owner routes and dynamically constructed Tcl names are separate observations; no display string supplies publication lifetime or source correspondence.

## Scope

Recorded programs and actual context rows only; no later publication or unrecorded provider is inferred. BIG-IP21.1.0.1 build0.0.26 Point Release1 only; appliance implementation source is unavailable.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### bigip

Status: `observed`. Version: 21.1.0.1. Build: 0.0.26, Point Release1; no separate hotfix field reported.

The exact matrix retains mutation visibility, available command doors and context-specific failures. Its F5 owner routes and dynamically constructed Tcl names are separate observations; no display string supplies publication lifetime or source correspondence.

The measured report rows and their limits are:


The command controls created absolute namespaces and procedures using dynamic
names, called them, resolved them with `namespace which -command`, renamed
them, proved the old spelling no longer resolved, called the renamed spelling,
and deleted the namespace. All operations succeeded in all four contexts.

No normalization was observed. Format-produced `e9` and `6501` names remained
distinct from binary-produced `c3a9` and `65cc81` names even when they render
similarly. The cross-producer procedures returned independent values before
and after rename. These results establish command identity for the tested
non-NUL names only; command-name NUL remains untested.

`namespace current` was `::script` in the tmsh CLI wrapper and `::` in TMM,
iApp implementation, and iCall. `namespace which -command set` returned
`::set` everywhere. This is an observed execution-namespace difference and
must not be inferred from the common Tcl patchlevel.


## Exact evidence

[bigip-namespace-command-currency.json](bigip-namespace-command-currency.json) contains the exact evidence descriptors, original hashes and selectors. The canonical manifest remains the proof authority. The indexed F5 store resolves historical paths without retaining expanded duplicate trees.

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH --json-pointer /ROW
```

## Reproduction

The argument vector below records the historical invocation. Restore retained driver directories with `materialise.py extract` before replay, and apply the selected case's current address bindings and state prerequisites.

The pinned report contains the exact source-generation, load and traffic protocol. This record claims no new appliance execution; rerun requires the recorded isolated appliance context, exact bytes and independently captured load/event outcomes. Closed implementation source and unmeasured releases are unavailable.

```json
[]
```
