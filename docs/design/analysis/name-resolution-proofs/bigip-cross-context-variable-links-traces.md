# naming.bigip.cross-context-variable-links-traces

Kind: `native-observation`

## Problem statement

Scalar/array cells, alias links and trace callbacks can receive differently formed names. A display-only subject or callback-name inference could change which cell is accessed.

## Question

What do the exact variable/array/link/trace controls return in each recorded execution context?

## Conclusion

The retained matrix records results and rejected doors for each context independently. Counted names, callback arguments and alias linkage are observable program facts; they supply neither a source editor span nor an independently entered native frame.

## Scope

Recorded context programs only; unavailable operations and failed setup remain explicit and no cross-context capability is borrowed. BIG-IP21.1.0.1 build0.0.26 Point Release1 only; appliance implementation source is unavailable.

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

Status: `observed`. Version: 21.1.0.1. Build: 0.0.26, Point Release1; no separate hotfix field reported. Channel: Recorded context programs only; unavailable operations and failed setup remain explicit and no cross-context capability is borrowed.. Dialect: F5 iRules and the explicitly recorded hosted contexts.

The retained matrix records results and rejected doors for each context independently. Counted names, callback arguments and alias linkage are observable program facts; they supply neither a source editor span nor an independently entered native frame.

The measured report rows and their limits are:


- Scalar names, array roots, and array indexes are counted byte sequences in
  all reached contexts. Embedded NUL did not truncate or alias a longer name.
- Qualified roots under a dynamically created ASCII namespace behaved the same
  way. This does not establish support for NUL inside a namespace component.
- `upvar` selected and mutated the intended NUL-bearing scalar, array root, or
  array element. The sibling short name retained its previous value.
- Direct array-root traces received the complete root bytes and index `6b`.
  Direct index traces received the ASCII root and index `41` or `410042`.
- A write through an array-root alias selected the counted original array but
  the callback's `name1` was hex `61727261795f616c696173` (`array_alias`), not
  the original qualified root. Selected cell identity and callback spelling
  must therefore remain separate compiler concepts.
- The element-alias mutation succeeded, but the root trace did not emit a
  distinct alias-spelled write row for that mutation. Later reads observed the
  changed counted element and supplied the original root plus `410042` index.
- The canonical non-TMM trace failures were exactly `invalid command name
  "TMM::cmp_group"`. The portable wrappers removed only that TMM-specific log
  call and retained all naming operations and callback byte capture.


## Exact evidence

- `report-section` (observation): [scripts/dev/bigip-probes/resolution-2286/CONTEXT_NAMING_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/CONTEXT_NAMING_RESULTS.md). SHA-256 `c19780153d8f494bc30bda0f4db8036acc9f3e94f7b210be688f0875c24ec1f7`. Lines 162–182. Exact build-specific measured report section; all rows and limits remain in the retained report. Sanitised retained artifact; lab transport/device details removed, without an appliance rerun.
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
