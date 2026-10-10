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

- `report-section` (observation): [scripts/dev/bigip-probes/resolution-2286/CONTEXT_NAMING_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/CONTEXT_NAMING_RESULTS.md). SHA-256 `33717dce1860e7c7390fc25263011aca3d7831cee683d8c0b1c092e05de1a5f1`. Lines 162–182. Exact build-specific measured report section; all rows and limits remain in the retained report.
- `provider-version` (provider): [scripts/dev/bigip-probes/resolution-2286/CONTEXT_NAMING_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/CONTEXT_NAMING_RESULTS.md). SHA-256 `33717dce1860e7c7390fc25263011aca3d7831cee683d8c0b1c092e05de1a5f1`. Lines 12–12. Actual appliance version attribution for this report; no separately named hotfix inferred.
- `aggregate-2` (observation): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006naming/decoded-results.json](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006naming/decoded-results.json). SHA-256 `31466bb8535715590d7ee32a809f9ce7774cf87d4a4cfe8cb38585c10ebb615d`. Inspected exact decoded aggregate, including reached, failed, unreached and cleanup fields; associated report questions preserve the source/context variants.
- `aggregate-3` (observation): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006naming/r2286t-decoded-results.json](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006naming/r2286t-decoded-results.json). SHA-256 `018f36b752d5b59610c9fd4763fdf72bbfce84d3e6a3aaaccaccb53c7498c092`. Inspected exact decoded aggregate, including reached, failed, unreached and cleanup fields; associated report questions preserve the source/context variants.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The pinned report contains the exact source-generation, load and traffic protocol. This record claims no new appliance execution; rerun requires the recorded isolated appliance context, exact bytes and independently captured load/event outcomes. Closed implementation source and unmeasured releases are unavailable.
