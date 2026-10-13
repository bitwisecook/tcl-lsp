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

RULE_INIT and CLIENT_ACCEPTED reach the recorded active scalar control. The static seed is seen on all four actual units; write and unset/recreation on0:2 leave the other three seeds intact. The missing read before recreation is preserved. This proves only this rule/build and observed units.

The measured report rows and their limits are:


The representative active scalar control completed with identical result
bytes and `read unset` order in RULE_INIT once on each of `0:0`–`0:3`, then in
CLIENT_ACCEPTED on one mapped connection per unit. The complete direct markers
are in
[`vl-context-events.log`](../../../../experiments/oracles/f5/captures/README.md) (retained logical path: `scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/appliance/vl-context-events.log`).

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
[`client-server`](../../../../experiments/oracles/f5/captures/README.md) (retained logical path: `scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server`), and the four
initialization markers are in the continuous LTM capture.


## Exact evidence

[bigip-variable-static-event-currency.json](bigip-variable-static-event-currency.json) contains the exact evidence descriptors, original hashes and selectors. The canonical manifest remains the proof authority. The indexed F5 store resolves historical paths without retaining expanded duplicate trees.

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH --json-pointer /ROW
```

## Reproduction

The argument vector below records the historical invocation. Restore retained driver directories with `materialise.py extract` before replay, and apply the selected case's current address bindings and state prerequisites.

The pinned report contains the exact source-generation, load and traffic protocol. This record claims no new appliance execution; rerun requires the recorded isolated appliance context, exact bytes and independently captured load/event outcomes. Closed implementation source and unmeasured releases are unavailable.

```json
[]
```
