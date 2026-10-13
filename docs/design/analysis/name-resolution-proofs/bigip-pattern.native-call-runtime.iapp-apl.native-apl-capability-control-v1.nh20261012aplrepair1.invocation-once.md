# naming.bigip-pattern.native-call-runtime.iapp-apl.native-apl-capability-control-v1.nh20261012aplrepair1.invocation-once

Kind: `native-observation`

## Problem statement

Related question F5-N01-native-call-runtime; this record retains one reviewed provider/input/context observation independently of the original C/Jim and exact portable evidence.

## Question

What completion and result did retained variant native-apl-capability-control-v1 of case native-call-runtime produce in iapp-apl, run nh20261012aplrepair1, unit None, invocation 'once'?

## Conclusion

Setup reached with completion 0. Intended lookup reached with completion 1; result encoding utf-8; exact result hex "696e76616c696420636f6d6d616e64206e616d65202263616c6c22". Error metadata and availability: {"errorcode": {"encoding": "utf-8", "hex": "4e4f4e45"}, "errorinfo": {"encoding": "utf-8", "hex": "696e76616c696420636f6d6d616e64206e616d65202263616c6c220a202020207768696c6520657865637574696e670a22246e685f63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c726570616972315f616273656e743a3a746172676574207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22"}, "return_options": null, "unavailable_fields": ["return_options"]}. Only this recorded invocation is answered.

## Scope

Only retained case native-call-runtime, variant native-apl-capability-control-v1, run nh20261012aplrepair1, context iapp-apl; event None, unit None, invocation 'once'. This independently scoped input does not replace an exact portable-input answer or establish other contexts, units, versions, recogniser coverage or source mechanisms. Authored measurement scope: Independent native APL callback capability control. No substitute harness and no portable original-input lookup claim.; authoring input is structured case metadata, not portable original. Interpreter/action facts retained in the measurement: {"interpreter_path": null, "interpreter_patchlevel": "8.4.6", "tcltest_version": null, "execution_namespace": "::script", "invoking_action": "Chrome Application Services Create form template selection; callback values retained from live DOM"}. Null means not observed by this receipt.

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

[bigip-pattern.native-call-runtime.iapp-apl.native-apl-capability-control-v1.nh20261012aplrepair1.invocation-once.json](bigip-pattern.native-call-runtime.iapp-apl.native-apl-capability-control-v1.nh20261012aplrepair1.invocation-once.json) contains the exact evidence descriptors, original hashes and selectors. The canonical manifest remains the proof authority. The indexed F5 store resolves historical paths without retaining expanded duplicate trees.

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH --json-pointer /ROW
```

## Reproduction

The argument vector below records the historical invocation. Restore retained driver directories with `materialise.py extract` before replay, and apply the selected case's current address bindings and state prerequisites.

Replay requires the retained exact inputs, isolated appliance objects, recorded invoking context and independently captured load, runtime and cleanup outcomes. This registration performs no appliance execution.

```json
[]
```
