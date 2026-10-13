# naming.bigip.actual-responding-unit-coverage

Kind: `native-observation`

## Problem statement

A requested CMP setting or expected roster is not evidence that every TMM actually responded. Traffic runners can report a subset, an empty unit field, a transport failure or a target-only mutation request. Treating each as complete four-TMM execution would overstate the applicability of a naming result.

## Question

Which actual response-unit counts, expected-roster comparisons and runner outcomes are retained for the recorded traffic invocations?

## Conclusion

The retained runner/coverage artifacts separately record actual response-unit counts, expected rosters, missing units, return codes and streams. Counts are scoped to each invocation; a target-only write, unit-only label, empty identity field or failed transport cannot supply unobserved group/unit coverage. Semantic result bytes remain in separate response/log evidence.

## Scope

Retained canonical and expanded procedure/Unicode traffic invocations on BIG-IP21.1.0.1 build0.0.26; no native name-table or cross-blade conclusion.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded.

No independent C Tcl/Jim execution of this appliance traffic or chunk-transport question is claimed.

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

Status: `observed`. Version: 21.1.0.1. Build: 0.0.26, Point Release1.

The scoped answer is stated above.

## Exact evidence

[bigip-actual-responding-unit-coverage.json](bigip-actual-responding-unit-coverage.json) contains the exact evidence descriptors, original hashes and selectors. The canonical manifest remains the proof authority. The indexed F5 store resolves historical paths without retaining expanded duplicate trees.

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH --json-pointer /ROW
```

## Reproduction

The argument vector below records the historical invocation. Restore retained driver directories with `materialise.py extract` before replay, and apply the selected case's current address bindings and state prerequisites.

Retains actual existing execution artifacts only; no new appliance run is claimed. Reconfirmation requires the same isolated contexts and complete raw capture, not inferred expected-roster membership.

```json
[]
```
