# naming.bigip-pattern.child-tcltest-real-package.tmm-rule_init.real-package-and-owner-diagnostic-v1.nh20261012repair2.unit-0-0.invocation-once

Kind: `native-observation`

## Problem statement

Related question F5-N01-child-tcltest-real-package; this record retains one reviewed provider/input/context observation independently of the original C/Jim and exact portable evidence.

## Question

What completion and result did retained variant real-package-and-owner-diagnostic-v1 of case child-tcltest-real-package produce in tmm-RULE_INIT, run nh20261012repair2, unit '0:0', invocation 'once'?

## Conclusion

Setup reached with completion 0. Intended lookup reached with completion 1; result encoding binary-scan-tcl-value; exact result hex "63616e27742066696e64207061636b6167652074636c74657374". Error metadata and availability: {"errorcode": {"encoding": "binary-scan-tcl-value", "hex": "4e4f4e45"}, "errorinfo": {"encoding": "binary-scan-tcl-value", "hex": "63616e27742066696e64207061636b6167652074636c746573740a202020207768696c6520657865637574696e670a227061636b61676520726571756972652074636c74657374220a20202020696e766f6b65642066726f6d2077697468696e0a22696e74657270206576616c20246e685f6368696c64207b6c697374205b7061636b61676520726571756972652074636c746573745d205b696e666f20636f6d6d616e6473203a3a74636c746573743a3a746573745d205b696e666f20636f6d6d616e6473203a3a746573745d7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22"}, "return_options": null, "unavailable_fields": ["return_options"]}. Only this recorded invocation is answered. This observation describes the captured host-native probe. It does not establish that tcltest runs in a TMM or MCPD F5 Tcl process.

## Scope

Only retained case child-tcltest-real-package, variant real-package-and-owner-diagnostic-v1, run nh20261012repair2, context tmm-RULE_INIT; event 'RULE_INIT', unit '0:0', invocation 'once'. This independently scoped input does not replace an exact portable-input answer or establish other contexts, units, versions, recogniser coverage or source mechanisms. Authored measurement scope: Independent capability/ownership diagnostic; no original lookup answer; original_input identifies the authored diagnostic case metadata, not a portable original; fresh-child results apply only to that new child interpreter. Interpreter/action facts retained in the measurement: {"interpreter_path": null, "interpreter_patchlevel": null, "tcltest_version": null, "execution_namespace": null, "invoking_action": null}. Null means not observed by this receipt. This observation describes the captured host-native probe. It does not establish that tcltest runs in a TMM or MCPD F5 Tcl process.

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

[bigip-pattern.child-tcltest-real-package.tmm-rule_init.real-package-and-owner-diagnostic-v1.nh20261012repair2.unit-0-0.invocation-once.json](bigip-pattern.child-tcltest-real-package.tmm-rule_init.real-package-and-owner-diagnostic-v1.nh20261012repair2.unit-0-0.invocation-once.json) contains the exact evidence descriptors, original hashes and selectors. The canonical manifest remains the proof authority. The indexed F5 store resolves historical paths without retaining expanded duplicate trees.

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH --json-pointer /ROW
```

## Reproduction

The argument vector below records the historical invocation. Restore retained driver directories with `materialise.py extract` before replay, and apply the selected case's current address bindings and state prerequisites.

Replay requires the retained exact inputs, isolated appliance objects, recorded invoking context and independently captured load, runtime and cleanup outcomes. This registration performs no appliance execution.

```json
[]
```
