# naming.bigip-pattern.command-trace-rename.tmm-traffic-event.capability-runtime-control-v1.nh20261012broad2.unit-0-0.invocation-0

Kind: `native-observation`

## Problem statement

Related question F5-N09; this record retains one reviewed provider/input/context observation independently of the original C/Jim and exact portable evidence.

## Question

What completion and result did retained variant capability-runtime-control-v1 of case command-trace-rename produce in tmm-traffic-event, run nh20261012broad2, unit '0:0', invocation '0'?

## Conclusion

Setup reached with completion 0. Intended lookup reached with completion 0; result encoding binary-scan-tcl-value; exact result hex "7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a746172676574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a72656e616d65642072656e616d657d". Error metadata and availability: {"errorcode": null, "errorinfo": null, "return_options": null, "unavailable_fields": ["return_options", "errorcode", "errorinfo"]}. Only this recorded invocation is answered.

## Scope

Only retained case command-trace-rename, variant capability-runtime-control-v1, run nh20261012broad2, context tmm-traffic-event; event 'HTTP_REQUEST', unit '0:0', invocation '0'. This independently scoped input does not replace an exact portable-input answer or establish other contexts, units, versions, recogniser coverage or source mechanisms. Authored measurement scope: Modern trace syntax is an independently checked capability, despite 8.4-compatible source syntax.. Interpreter/action facts retained in the measurement: {"interpreter_path": null, "interpreter_patchlevel": null, "tcltest_version": null, "execution_namespace": null, "invoking_action": null}. Null means not observed by this receipt.  Recorded adaptation: {"kind": "authored-template-materialisation", "source_provenance": {"path": "experiments/name-handling/f5/answers/nh20261012broad2/broader_cases.py", "sha256": "c0150e5d368d4dcdb447091bb6439947e383ef7c9efe2d7315e0a6f8d49d9ce2", "kind": "authored-template-provenance-not-portable-original"}, "actual_input_authority": {"path": "experiments/name-handling/f5/answers/nh20261012broad2/tmm-traffic-event-12/input.tcl", "sha256": "1d239635a2713db5b678b9a78ec0d57d70acce114747b01b53feb8cc52d17894"}, "case_materialisation": {"path": "experiments/name-handling/f5/answers/nh20261012broad2/tmm-traffic-event-cases.json", "sha256": "daf0ee710876a767cc30d65266e63616c658f19c91ac261de4f3c8cfa503e235", "json_pointer": "/38"}, "warning": "The source artifact is authored Python template provenance, not a portable Tcl original. Exact executed Tcl bytes and materialised setup/operation are retained separately."}

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

[bigip-pattern.command-trace-rename.tmm-traffic-event.capability-runtime-control-v1.nh20261012broad2.unit-0-0.invocation-0.json](bigip-pattern.command-trace-rename.tmm-traffic-event.capability-runtime-control-v1.nh20261012broad2.unit-0-0.invocation-0.json) contains the exact evidence descriptors, original hashes and selectors. The canonical manifest remains the proof authority. The indexed F5 store resolves historical paths without retaining expanded duplicate trees.

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH --json-pointer /ROW
```

## Reproduction

The argument vector below records the historical invocation. Restore retained driver directories with `materialise.py extract` before replay, and apply the selected case's current address bindings and state prerequisites.

Replay requires the retained exact inputs, isolated appliance objects, recorded invoking context and independently captured load, runtime and cleanup outcomes. This registration performs no appliance execution.

```json
[]
```
