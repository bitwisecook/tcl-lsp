# naming.bigip.cross-context-lexical-names

Kind: `native-observation`

## Problem statement

A namespace/cell may retain a full byte name while an unbraced variable scanner consumes only a prefix. Applying runtime name-table rules to lexical tokens would misidentify the read.

## Question

Where do the measured braced/unbraced lexical variable forms stop in the recorded contexts?

## Conclusion

The retained context matrix distinguishes full braced name reads from unbraced lexical stopping and preserves their exact errors/results. Source-produced names and runtime name lookup remain separate axes; the dedicated complete-byte scan record supplies the exhaustive finite singleton controls.

## Scope

Only exact context matrix sources/forms and reached rows; no native header layout or untested Unicode class is inferred. BIG-IP21.1.0.1 build0.0.26 Point Release1 only; appliance implementation source is unavailable.

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

Status: `observed`. Version: 21.1.0.1. Build: 0.0.26, Point Release1; no separate hotfix field reported. Channel: Only exact context matrix sources/forms and reached rows; no native header layout or untested Unicode class is inferred.. Dialect: F5 iRules and the explicitly recorded hosted contexts.

The retained context matrix distinguishes full braced name reads from unbraced lexical stopping and preserves their exact errors/results. Source-produced names and runtime name lookup remain separate axes; the dedicated complete-byte scan record supplies the exhaustive finite singleton controls.

The measured report rows and their limits are:


The clean payload seeded the ASCII prefix cell with `SHORT_PREFIX` and the
complete dynamic cell with `FULL_VALUE`, then evaluated one substitution only.

| Runtime suffix bytes | TMM unbraced result | tmsh/iApp/iCall unbraced result | Braced result in all contexts |
| --- | --- | --- | --- |
| `%c 233` -> `e9` | `SHORT_PREFIX` + `e9` | `FULL_VALUE` | `FULL_VALUE` |
| `%c 769` -> `01` | `SHORT_PREFIX` + `01` | `SHORT_PREFIX` + `01` | `FULL_VALUE` |
| binary `c3a9` | `SHORT_PREFIX` + `c3a9` | catch 1; attempted name ended in `c3` | `FULL_VALUE` |
| binary `cc81` | `SHORT_PREFIX` + `cc81` | catch 1; attempted name ended in `cc` | `FULL_VALUE` |
| binary `0042` | `SHORT_PREFIX` + `0042` | `SHORT_PREFIX` + `0042` | `FULL_VALUE` |

Therefore a centralized compiler interface needs a context-specific lexical
name scanner. Dynamic name identity is byte-preserving in these controls, but
unbraced source-token consumption is not the same in TMM and scriptd/tmsh Tcl.


## Exact evidence

- `report-section` (observation): [scripts/dev/bigip-probes/resolution-2286/CONTEXT_NAMING_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/CONTEXT_NAMING_RESULTS.md). SHA-256 `33717dce1860e7c7390fc25263011aca3d7831cee683d8c0b1c092e05de1a5f1`. Lines 145–161. Exact build-specific measured report section; all rows and limits remain in the retained report.
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
