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

- `report-section` (observation): [scripts/dev/bigip-probes/resolution-2286/CONTEXT_NAMING_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/CONTEXT_NAMING_RESULTS.md). SHA-256 `c19780153d8f494bc30bda0f4db8036acc9f3e94f7b210be688f0875c24ec1f7`. Lines 145–161. Exact build-specific measured report section; all rows and limits remain in the retained report. Sanitised retained artifact; lab transport/device details removed, without an appliance rerun.
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
