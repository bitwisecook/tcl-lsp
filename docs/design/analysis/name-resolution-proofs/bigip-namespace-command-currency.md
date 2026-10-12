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

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### bigip

Status: `observed`. Version: 21.1.0.1. Build: 0.0.26, Point Release1; no separate hotfix field reported. Channel: Recorded programs and actual context rows only; no later publication or unrecorded provider is inferred.. Dialect: F5 iRules and the explicitly recorded hosted contexts.

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

- `report-section` (observation): [scripts/dev/bigip-probes/resolution-2286/CONTEXT_NAMING_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/CONTEXT_NAMING_RESULTS.md). SHA-256 `c19780153d8f494bc30bda0f4db8036acc9f3e94f7b210be688f0875c24ec1f7`. Lines 274–291. Exact build-specific measured report section; all rows and limits remain in the retained report. Sanitised retained artifact; lab transport/device details removed, without an appliance rerun.
- `provider-version` (provider): [scripts/dev/bigip-probes/resolution-2286/CONTEXT_NAMING_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/CONTEXT_NAMING_RESULTS.md). SHA-256 `c19780153d8f494bc30bda0f4db8036acc9f3e94f7b210be688f0875c24ec1f7`. Lines 12–12. Actual appliance version attribution for this report; no separately named hotfix inferred. Sanitised retained artifact; lab transport/device details removed, without an appliance rerun.
- `aggregate-2` (observation): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006naming/decoded-results.json](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006naming/decoded-results.json). SHA-256 `ee533a62089293ff9c10075994ee3fc81fb01402ca978836a95b2a3dff368426`. Inspected exact decoded aggregate, including reached, failed, unreached and cleanup fields; associated report questions preserve the source/context variants. Sanitised retained artifact; lab transport/device details removed, without an appliance rerun.
- `aggregate-3` (observation): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006naming/r2286t-decoded-results.json](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006naming/r2286t-decoded-results.json). SHA-256 `f34346e0889d7181dc3d2777e0b307f38cf2892ea1c08ce8ffae4e8ed602ad3d`. Inspected exact decoded aggregate, including reached, failed, unreached and cleanup fields; associated report questions preserve the source/context variants. Sanitised retained artifact; lab transport/device details removed, without an appliance rerun.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The pinned report contains the exact source-generation, load and traffic protocol. This record claims no new appliance execution; rerun requires the recorded isolated appliance context, exact bytes and independently captured load/event outcomes. Closed implementation source and unmeasured releases are unavailable.
