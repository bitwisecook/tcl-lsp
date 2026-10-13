# naming.bigip-pattern.error-import-conflict.tmm-procedure.capability-runtime-control-v1.nh20261012broad2.unit-0-0.invocation-once

Kind: `native-observation`

## Problem statement

Related question F5-N15; this record retains one reviewed provider/input/context observation independently of the original C/Jim and exact portable evidence.

## Question

What completion and result did retained variant capability-runtime-control-v1 of case error-import-conflict produce in tmm-procedure, run nh20261012broad2, unit '0:0', invocation 'once'?

## Conclusion

Setup reached with completion 0. Intended lookup reached with completion 1; result encoding binary-scan-tcl-value; exact result hex "63616e277420696d706f727420636f6d6d616e64202270223a20616c726561647920657869737473". Error metadata and availability: {"errorcode": null, "errorinfo": null, "return_options": null, "unavailable_fields": ["return_options", "errorcode", "errorinfo"]}. Only this recorded invocation is answered.

## Scope

Only retained case error-import-conflict, variant capability-runtime-control-v1, run nh20261012broad2, context tmm-procedure; event 'RULE_INIT', unit '0:0', invocation 'once'. This independently scoped input does not replace an exact portable-input answer or establish other contexts, units, versions, recogniser coverage or source mechanisms. Authored measurement scope: Non-force import conflict between owned commands.. Interpreter/action facts retained in the measurement: {"interpreter_path": null, "interpreter_patchlevel": null, "tcltest_version": null, "execution_namespace": null, "invoking_action": "call measure from RULE_INIT"}. Null means not observed by this receipt.  Recorded adaptation: {"kind": "authored-template-materialisation", "source_provenance": {"path": "experiments/name-handling/f5/answers/nh20261012broad2/broader_cases.py", "sha256": "c0150e5d368d4dcdb447091bb6439947e383ef7c9efe2d7315e0a6f8d49d9ce2", "kind": "authored-template-provenance-not-portable-original"}, "actual_input_authority": {"path": "experiments/name-handling/f5/answers/nh20261012broad2/tmm-procedure-0/input.tcl", "sha256": "e6973034bdc742f662ebbc683d6ba878610871bf03216557ee1178c0c7c1eeb6"}, "case_materialisation": {"path": "experiments/name-handling/f5/answers/nh20261012broad2/tmm-procedure-cases.json", "sha256": "0b63988544778666e9b1f9b6fbbdcb3a7b9b274be3206c52bc7fb54fc18949f7", "json_pointer": "/1"}, "warning": "The source artifact is authored Python template provenance, not a portable Tcl original. Exact executed Tcl bytes and materialised setup/operation are retained separately."}

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded.

No execution of this exact context/input/invocation is claimed for this provider.

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

Status: `observed`. Version: 21.1.0.1. Build: 0.0.26; hotfix None.

The scoped answer is stated above.

## Exact evidence

[bigip-pattern.error-import-conflict.tmm-procedure.capability-runtime-control-v1.nh20261012broad2.unit-0-0.invocation-once.json](bigip-pattern.error-import-conflict.tmm-procedure.capability-runtime-control-v1.nh20261012broad2.unit-0-0.invocation-once.json) contains the exact evidence descriptors, original hashes and selectors. The canonical manifest remains the proof authority. The indexed F5 store resolves historical paths without retaining expanded duplicate trees.

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH --json-pointer /ROW
```

## Reproduction

The argument vector below records the historical invocation. Restore retained driver directories with `materialise.py extract` before replay, and apply the selected case's current address bindings and state prerequisites.

Replay requires the retained exact inputs, isolated appliance objects, recorded invoking context and independently captured load, runtime and cleanup outcomes. This registration performs no appliance execution.

```json
[]
```
