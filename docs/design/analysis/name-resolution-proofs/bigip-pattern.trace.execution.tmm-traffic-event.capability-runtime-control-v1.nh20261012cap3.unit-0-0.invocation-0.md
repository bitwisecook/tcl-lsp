# naming.bigip-pattern.trace.execution.tmm-traffic-event.capability-runtime-control-v1.nh20261012cap3.unit-0-0.invocation-0

Kind: `native-observation`

## Problem statement

Related question F5-N01-trace.execution; this record retains one reviewed provider/input/context observation independently of the original C/Jim and exact portable evidence.

## Question

What completion and result did retained variant capability-runtime-control-v1 of case trace.execution produce in tmm-traffic-event, run nh20261012cap3, unit '0:0', invocation '0'?

## Conclusion

Setup reached with completion 0. Intended lookup reached with completion 0; result encoding binary-scan-tcl-value; exact result hex "544152474554". Error metadata and availability: {"errorcode": null, "errorinfo": null, "return_options": null, "unavailable_fields": ["return_options", "errorcode", "errorinfo"]}. Only this recorded invocation is answered.

## Scope

Only retained case trace.execution, variant capability-runtime-control-v1, run nh20261012cap3, context tmm-traffic-event; event 'HTTP_REQUEST', unit '0:0', invocation '0'. This independently scoped input does not replace an exact portable-input answer or establish other contexts, units, versions, recogniser coverage or source mechanisms. Authored measurement scope: Dynamic capability source evaluated in HTTP_REQUEST; results answer only the responding unit/invocation. Literal loader controls and other interpreter contexts remain independently scoped.. Interpreter/action facts retained in the measurement: {"interpreter_path": null, "interpreter_patchlevel": null, "tcltest_version": null, "execution_namespace": null, "invoking_action": null}. Null means not observed by this receipt.  Recorded adaptation: {"kind": "authored-capability-template-materialisation", "source_provenance": {"path": "experiments/name-handling/patterns/f5/answers/nh20261012cap3/runner.py", "sha256": "67de5f6a5d39df7640d53aeff2fd5e48a4646bf4f65c06603ea0f653baf0a583", "kind": "authored-capability-template-provenance-not-portable-original"}, "actual_input_authority": {"path": "experiments/name-handling/patterns/f5/answers/nh20261012cap3/tmm-traffic-event-4/input.tcl", "sha256": "9b6b25599b34816f88176de01ac9dde8645d92868e163061cba5b374c6ac699e"}, "case_materialisation": {"path": "experiments/name-handling/patterns/f5/answers/nh20261012cap3/capabilities.json", "sha256": "ddac6fac40afeb7204bad0cc4f333f50420efad31cf11e02d710c952fce7a641", "json_pointer": "/14"}}

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

[bigip-pattern.trace.execution.tmm-traffic-event.capability-runtime-control-v1.nh20261012cap3.unit-0-0.invocation-0.json](bigip-pattern.trace.execution.tmm-traffic-event.capability-runtime-control-v1.nh20261012cap3.unit-0-0.invocation-0.json) contains the exact evidence descriptors, original hashes and selectors. The canonical manifest remains the proof authority. The indexed F5 store resolves historical paths without retaining expanded duplicate trees.

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH --json-pointer /ROW
```

## Reproduction

The argument vector below records the historical invocation. Restore retained driver directories with `materialise.py extract` before replay, and apply the selected case's current address bindings and state prerequisites.

Replay requires the retained exact inputs, isolated appliance objects, recorded invoking context and independently captured load, runtime and cleanup outcomes. This registration performs no appliance execution.

```json
[]
```
