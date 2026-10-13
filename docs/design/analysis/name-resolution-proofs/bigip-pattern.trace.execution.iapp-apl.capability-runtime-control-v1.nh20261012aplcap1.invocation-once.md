# naming.bigip-pattern.trace.execution.iapp-apl.capability-runtime-control-v1.nh20261012aplcap1.invocation-once

Kind: `native-observation`

## Problem statement

Related question F5-N01-trace.execution; this record retains one reviewed provider/input/context observation independently of the original C/Jim and exact portable evidence.

## Question

What completion and result did retained variant capability-runtime-control-v1 of case trace.execution produce in iapp-apl, run nh20261012aplcap1, unit None, invocation 'once'?

## Conclusion

Setup reached with completion 0. Intended lookup reached with completion 0; result encoding utf-8; exact result hex "544152474554". Error metadata and availability: {"errorcode": null, "errorinfo": null, "return_options": null, "unavailable_fields": ["return_options", "errorcode", "errorinfo"]}. Only this recorded invocation is answered.

## Scope

Only retained case trace.execution, variant capability-runtime-control-v1, run nh20261012aplcap1, context iapp-apl; event None, unit None, invocation 'once'. This independently scoped input does not replace an exact portable-input answer or establish other contexts, units, versions, recogniser coverage or source mechanisms. Authored measurement scope: Actual browser-selected APL choice Tcl callback; each callback returns captured rows as equal label/value pairs. The version control in this same capture reports 8.4.6, Tcl8.4, namespace ::script. This does not equate the APL interpreter with iApp implementation, tmsh, stock or TMM contexts.. Interpreter/action facts retained in the measurement: {"interpreter_path": null, "interpreter_patchlevel": "8.4.6", "tcltest_version": null, "execution_namespace": "::script", "invoking_action": "Chrome Application Services Create form template selection; callback values retained from live DOM"}. Null means not observed by this receipt.  Recorded adaptation: {"kind": "apl-browser-choice-return-capture", "projection": "Tcl splitlist of DOM option.value, followed by each two-element equal label/value pair; one NHO row retained per pair. Projection independently reproduced exactly from browser-options.json.", "source_provenance": {"path": "experiments/name-handling/patterns/f5/answers/nh20261012aplcap1/capture.py", "sha256": "67de5f6a5d39df7640d53aeff2fd5e48a4646bf4f65c06603ea0f653baf0a583", "kind": "authored-capability-template-provenance-not-portable-original"}, "callback_wrapper": {"path": "experiments/name-handling/patterns/f5/answers/nh20261012aplcap1/iapp-apl-0/applied.conf", "sha256": "6ebc2d070825b3b280bc99dff8d46e6798bc4dd946580560b6d30b02cfe6becf"}}

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

[bigip-pattern.trace.execution.iapp-apl.capability-runtime-control-v1.nh20261012aplcap1.invocation-once.json](bigip-pattern.trace.execution.iapp-apl.capability-runtime-control-v1.nh20261012aplcap1.invocation-once.json) contains the exact evidence descriptors, original hashes and selectors. The canonical manifest remains the proof authority. The indexed F5 store resolves historical paths without retaining expanded duplicate trees.

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH --json-pointer /ROW
```

## Reproduction

The argument vector below records the historical invocation. Restore retained driver directories with `materialise.py extract` before replay, and apply the selected case's current address bindings and state prerequisites.

Replay requires the retained exact inputs, isolated appliance objects, recorded invoking context and independently captured load, runtime and cleanup outcomes. This registration performs no appliance execution.

```json
[]
```
