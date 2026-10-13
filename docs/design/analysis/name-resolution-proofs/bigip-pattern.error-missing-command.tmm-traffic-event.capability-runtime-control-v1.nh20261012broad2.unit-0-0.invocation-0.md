# naming.bigip-pattern.error-missing-command.tmm-traffic-event.capability-runtime-control-v1.nh20261012broad2.unit-0-0.invocation-0

Kind: `native-observation`

## Problem statement

Related question F5-N15; this record retains one reviewed provider/input/context observation independently of the original C/Jim and exact portable evidence.

## Question

What completion and result did retained variant capability-runtime-control-v1 of case error-missing-command produce in tmm-traffic-event, run nh20261012broad2, unit '0:0', invocation '0'?

## Conclusion

Setup reached with completion 0. Intended lookup reached with completion 1; result encoding binary-scan-tcl-value; exact result hex "696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6572726f725f6d697373696e675f636f6d6d616e645f616273656e7422". Error metadata and availability: {"errorcode": null, "errorinfo": null, "return_options": null, "unavailable_fields": ["return_options", "errorcode", "errorinfo"]}. Only this recorded invocation is answered.

## Scope

Only retained case error-missing-command, variant capability-runtime-control-v1, run nh20261012broad2, context tmm-traffic-event; event 'HTTP_REQUEST', unit '0:0', invocation '0'. This independently scoped input does not replace an exact portable-input answer or establish other contexts, units, versions, recogniser coverage or source mechanisms. Authored measurement scope: Direct missing-command error, no inner catch.. Interpreter/action facts retained in the measurement: {"interpreter_path": null, "interpreter_patchlevel": null, "tcltest_version": null, "execution_namespace": null, "invoking_action": null}. Null means not observed by this receipt.  Recorded adaptation: {"kind": "authored-template-materialisation", "source_provenance": {"path": "experiments/name-handling/f5/answers/nh20261012broad2/broader_cases.py", "sha256": "c0150e5d368d4dcdb447091bb6439947e383ef7c9efe2d7315e0a6f8d49d9ce2", "kind": "authored-template-provenance-not-portable-original"}, "actual_input_authority": {"path": "experiments/name-handling/f5/answers/nh20261012broad2/tmm-traffic-event-16/input.tcl", "sha256": "2a0c9af3463b2ad04866dd50f115e771b4b6434467d7d3d9f4a16d04ffba26f7"}, "case_materialisation": {"path": "experiments/name-handling/f5/answers/nh20261012broad2/tmm-traffic-event-cases.json", "sha256": "daf0ee710876a767cc30d65266e63616c658f19c91ac261de4f3c8cfa503e235", "json_pointer": "/50"}, "warning": "The source artifact is authored Python template provenance, not a portable Tcl original. Exact executed Tcl bytes and materialised setup/operation are retained separately."}

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

[bigip-pattern.error-missing-command.tmm-traffic-event.capability-runtime-control-v1.nh20261012broad2.unit-0-0.invocation-0.json](bigip-pattern.error-missing-command.tmm-traffic-event.capability-runtime-control-v1.nh20261012broad2.unit-0-0.invocation-0.json) contains the exact evidence descriptors, original hashes and selectors. The canonical manifest remains the proof authority. The indexed F5 store resolves historical paths without retaining expanded duplicate trees.

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH --json-pointer /ROW
```

## Reproduction

The argument vector below records the historical invocation. Restore retained driver directories with `materialise.py extract` before replay, and apply the selected case's current address bindings and state prerequisites.

Replay requires the retained exact inputs, isolated appliance objects, recorded invoking context and independently captured load, runtime and cleanup outcomes. This registration performs no appliance execution.

```json
[]
```
