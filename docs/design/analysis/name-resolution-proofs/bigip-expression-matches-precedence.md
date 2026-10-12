# naming.bigip.expression-matches-precedence

Kind: `native-observation`

## Problem statement

The pair comparing a literal pattern to itself cannot distinguish equality from glob matching. Independently, prefix not can bind before a following string predicate. Both ambiguities affect expression consumers.

## Question

What do the nine exact matches/not discriminators return in TMM, CLI script, iApp implementation and iCall?

## Conclusion

For the recorded387-byte program, bare matches is whole-string glob matching: a* matches abcd, while a.* and bc do not. not binds to its operand before starts_with unless parentheses group the predicate. All four recorded hosted contexts return the same nine rows; APL remains untested.

## Scope

Exact nine-row source and four hosted channels; the initial shorter matrix remains insufficient by itself and no other release is inferred. BIG-IP21.1.0.1 build0.0.26 Point Release1 only; appliance implementation source is unavailable.

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

Status: `observed`. Version: 21.1.0.1. Build: 0.0.26, Point Release1; no separate hotfix field reported. Channel: Exact nine-row source and four hosted channels; the initial shorter matrix remains insufficient by itself and no other release is inferred.. Dialect: F5 iRules and the explicitly recorded hosted contexts.

For the recorded387-byte program, bare matches is whole-string glob matching: a* matches abcd, while a.* and bc do not. not binds to its operand before starts_with unless parentheses group the predicate. All four recorded hosted contexts return the same nine rows; APL remains untested.

The measured report rows and their limits are:


The exact 387-byte expression payload ran in an `HTTP_REQUEST` iRule, a tmsh CLI script, an iApp implementation action and a triggered iCall script. All four BIG-IP-hosted contexts returned the same result bytes:

| Expression | Catch | Result |
| --- | ---: | --- |
| `"abcd" matches "a*"` | 0 | 1 |
| `"a*" matches "a*"` | 0 | 1 |
| `"a*" matches "abcd"` | 0 | 0 |
| `"abcd" matches "a.*"` | 0 | 0 |
| `"abcd" matches "bc"` | 0 | 0 |
| `"abcd" matches "abcd"` | 0 | 1 |
| `1 or 0 matches 0` | 0 | 1 |
| `not "abc" starts_with "a"` | 1 | `can't use non-numeric string as operand of "!"` |
| `not ("abc" starts_with "a")` | 0 | 0 |

For these discriminators, bare `matches` is whole-string glob matching: `a*` matches `abcd`, while `a.*` and `bc` do not. The earlier equality/non-substring pair alone was insufficient to distinguish equality from glob behavior. `not` binds to the following operand before `starts_with` unless parentheses group the string predicate.

The non-TMM fixtures loaded without error, the tmsh script emitted directly, the iApp emitted from its implementation action, and the iCall emitted from a triggered handler in `scriptd`. Raw outputs are [tmsh](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006m/appliance/raw/contexts/cli-run.txt) and [iApp/iCall](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006m/appliance/raw/contexts/scriptd-results.txt). iApp presentation/APL Tcl was not tested and must not inherit the implementation result.


## Exact evidence

- `report-section` (observation): [scripts/dev/bigip-probes/resolution-2286/FOLLOWUP_APPLIANCE_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/FOLLOWUP_APPLIANCE_RESULTS.md). SHA-256 `1f1324bcead38249ef875389a21ca63319c3481f529c857cc31c3fd2a69c0f21`. Lines 100–119. Exact build-specific measured report section; all rows and limits remain in the retained report.
- `provider-version` (provider): [scripts/dev/bigip-probes/resolution-2286/FOLLOWUP_APPLIANCE_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/FOLLOWUP_APPLIANCE_RESULTS.md). SHA-256 `1f1324bcead38249ef875389a21ca63319c3481f529c857cc31c3fd2a69c0f21`. Lines 9–9. Actual appliance version attribution for this report; no separately named hotfix inferred.
- `raw-0` (observation): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006m/appliance/raw/contexts/cli-run.txt](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006m/appliance/raw/contexts/cli-run.txt). SHA-256 `28bda477163bc129badfacc49384c70fbec4165bb44259e853a887f925c163b1`. Exact source or raw evidence explicitly linked by the measured section; its own context/channel remains authoritative.
- `raw-1` (observation): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006m/appliance/raw/contexts/scriptd-results.txt](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006m/appliance/raw/contexts/scriptd-results.txt). SHA-256 `62e4963a474fa9a3894cba03787e7020a3b7b7ac2bddad4a94641f38be10ef14`. Exact source or raw evidence explicitly linked by the measured section; its own context/channel remains authoritative.
- `aggregate-4` (observation): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006m/decoded-results.json](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006m/decoded-results.json). SHA-256 `4dc32dddeb5a60b758a03b04d0a9271e83bdf37c92662d9c7e7b4e519f1d89a9`. Inspected exact decoded aggregate, including reached, failed, unreached and cleanup fields; associated report questions preserve the source/context variants.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The pinned report contains the exact source-generation, load and traffic protocol. This record claims no new appliance execution; rerun requires the recorded isolated appliance context, exact bytes and independently captured load/event outcomes. Closed implementation source and unmeasured releases are unavailable.
