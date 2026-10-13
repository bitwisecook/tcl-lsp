# naming.bigip.lexical-event-transport-completeness

Kind: `native-observation`

## Problem statement

Chunked RULE_INIT/CLIENT_ACCEPTED logs may declare a complete byte-scan payload while losing chunks. Joining partial transport with complete HTTP_REQUEST output would manufacture event equivalence for missing rows.

## Question

Do the actual retained chunk inventories establish a complete lexical matrix for RULE_INIT and CLIENT_ACCEPTED?

## Conclusion

The retained inventories expose missing sequences and incomplete assembled payloads. They do not certify the complete lexical matrix for those events; complete HTTP_REQUEST and hosted-context outputs remain independent evidence. A reached marker and partial transport are preserved without treating absent chunks as observed results.

## Scope

Exact r2286lex1 chunk logs/inventory on the recorded appliance/build, with incomplete event channels explicitly excluded from complete-matrix claims.

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

[bigip-lexical-event-transport-completeness.json](bigip-lexical-event-transport-completeness.json) contains the exact evidence descriptors, original hashes and selectors. The canonical manifest remains the proof authority. The indexed F5 store resolves historical paths without retaining expanded duplicate trees.

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH --json-pointer /ROW
```

## Reproduction

The argument vector below records the historical invocation. Restore retained driver directories with `materialise.py extract` before replay, and apply the selected case's current address bindings and state prerequisites.

Retains actual existing execution artifacts only; no new appliance run is claimed. Reconfirmation requires the same isolated contexts and complete raw capture, not inferred expected-roster membership.

```json
[]
```
