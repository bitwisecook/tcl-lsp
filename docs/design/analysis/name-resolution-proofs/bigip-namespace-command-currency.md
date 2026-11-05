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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `not-tested`.

No independent C Tcl or Jim execution of this appliance question is claimed.

### bigip

Status: `observed`. Channel: Recorded programs and actual context rows only; no later publication or unrecorded provider is inferred. Version: bigip 21.1.0.1.

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

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [scripts/dev/bigip-probes/resolution-2286/CONTEXT_NAMING_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/CONTEXT_NAMING_RESULTS.md)
