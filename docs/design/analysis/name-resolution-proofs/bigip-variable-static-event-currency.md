# naming.bigip.variable-static-event-currency

Kind: `native-observation`

## Problem statement

A static initialization seen on all TMMs can be mistaken for a broadcast of later event mutations. The consumer needs actual responding-unit evidence and separate missing/read/recreate results.

## Question

What do the mapped per-unit lifecycle and static-cell controls observe after target-unit write and unset/recreation?

## Conclusion

RULE_INIT and CLIENT_ACCEPTED reach the recorded active scalar control. The static seed is seen on all four actual units; write and unset/recreation on0:2 leave the other three seeds intact. The missing read before recreation is preserved. This proves only this rule/build and observed units.

## Scope

Exact mapped source tuples and per-unit response/log evidence; no chassis-wide or unobserved-TMM claim. BIG-IP21.1.0.1 build0.0.26 Point Release1 only; appliance implementation source is unavailable.

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

Status: `observed`. Version: 21.1.0.1. Build: 0.0.26, Point Release1; no separate hotfix field reported. Channel: Exact mapped source tuples and per-unit response/log evidence; no chassis-wide or unobserved-TMM claim.. Dialect: F5 iRules and the explicitly recorded hosted contexts.

RULE_INIT and CLIENT_ACCEPTED reach the recorded active scalar control. The static seed is seen on all four actual units; write and unset/recreation on0:2 leave the other three seeds intact. The missing read before recreation is preserved. This proves only this rule/build and observed units.

The measured report rows and their limits are:


The representative active scalar control completed with identical result
bytes and `read unset` order in RULE_INIT once on each of `0:0`–`0:3`, then in
CLIENT_ACCEPTED on one mapped connection per unit. The complete direct markers
are in
[`vl-context-events.log`](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/appliance/vl-context-events.log).

The `static::` rule logged a `SEED` initialization from every actual unit. The
following traffic then used the same four stable source tuples:

| Measured configuration | `0:0` | `0:1` | `0:2` | `0:3` |
| --- | --- | --- | --- | --- |
| initial read | `SEED` | `SEED` | `SEED` | `SEED` |
| after write on `0:2` | `SEED` | `SEED` | `MARK_UNIT` | `SEED` |
| after unset/recreate on `0:2` | `SEED` | `SEED` | `RECREATED` | `SEED` |

The unset request on `0:2` observed catch 1 with exact message
`can't read "static::__tcl_lsp_2286_vl1_cell": no such variable` before
recreation. Thus the measured `static::` storage is per TMM for this rule and
version: neither mutation nor unset/recreation broadcast to the other three
units. The raw sequence is the `vl-static-*.body` set in
[`client-server`](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server), and the four
initialization markers are in the continuous LTM capture.


## Exact evidence

- `report-section` (observation): [scripts/dev/bigip-probes/resolution-2286/VARIABLE_LIFECYCLE_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/VARIABLE_LIFECYCLE_RESULTS.md). SHA-256 `469555b59e24291fa07e50a5aaf531ff13034270a20d44cdde4ad4b01cb833d8`. Lines 168–192. Exact build-specific measured report section; all rows and limits remain in the retained report.
- `provider-version` (provider): [scripts/dev/bigip-probes/resolution-2286/VARIABLE_LIFECYCLE_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/VARIABLE_LIFECYCLE_RESULTS.md). SHA-256 `469555b59e24291fa07e50a5aaf531ff13034270a20d44cdde4ad4b01cb833d8`. Lines 11–11. Actual appliance version attribution for this report; no separately named hotfix inferred.
- `raw-0` (observation): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/appliance/vl-context-events.log](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/appliance/vl-context-events.log). SHA-256 `d160f71ccd3657d7918cd7fde5f3f22aacf069c3f62b5ea83d97733d6639d512`. Exact source or raw evidence explicitly linked by the measured section; its own context/channel remains authoritative.
- `source-closure-3` (input): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/appliance/r2286vl1-fixtures-v4/manifest.json](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/appliance/r2286vl1-fixtures-v4/manifest.json). SHA-256 `5d926a45e3efa7916ca5ef8af3800fe9516d5ce5c9af74401854584649fccbbf`. Retained report-specific source manifest or appliance inventory; individual measured inputs remain distinct.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The pinned report contains the exact source-generation, load and traffic protocol. This record claims no new appliance execution; rerun requires the recorded isolated appliance context, exact bytes and independently captured load/event outcomes. Closed implementation source and unmeasured releases are unavailable.
