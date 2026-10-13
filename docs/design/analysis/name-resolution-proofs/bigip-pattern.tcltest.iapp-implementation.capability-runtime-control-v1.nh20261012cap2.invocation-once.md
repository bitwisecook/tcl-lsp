# naming.bigip-pattern.tcltest.iapp-implementation.capability-runtime-control-v1.nh20261012cap2.invocation-once

Kind: `native-observation`

## Problem statement

Related question F5-N01-tcltest; this record retains one reviewed provider/input/context observation independently of the original C/Jim and exact portable evidence.

## Question

What completion and result did retained variant capability-runtime-control-v1 of case tcltest produce in iapp-implementation, run nh20261012cap2, unit None, invocation 'once'?

## Conclusion

Setup reached with completion 0. Intended lookup reached with completion 1; result encoding utf-8; exact result hex "63616e2774207265616420226465627567223a206e6f2073756368207661726961626c65". Error metadata and availability: {"errorcode": {"encoding": "utf-8", "hex": "4e4f4e45"}, "errorinfo": {"encoding": "utf-8", "hex": "63616e2774207265616420226465627567223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a226966207b246465627567203e3d20246c6576656c7d207b0a09707574732024737472696e670a202020207d220a202020202870726f636564757265202244656275675075747322206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a2244656275675075747320332022656e746572696e6720436f6e73747261696e74496e697469616c697a65722024636f6e73747261696e74202473637269707422220a202020202870726f6365647572652022436f6e73747261696e74496e697469616c697a657222206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a22436f6e73747261696e74496e697469616c697a65722073696e676c6554657374496e74657270207b73696e676c6550726f636573737d220a202020202870726f6365647572652022446566696e65436f6e73747261696e74496e697469616c697a65727322206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a22446566696e65436f6e73747261696e74496e697469616c697a657273220a2020202028696e206e616d657370616365206576616c20223a3a74636c746573742220736372697074206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2074636c74657374207b0a202020202320446566696e6520696e697469616c697a65727320666f7220616c6c20746865206275696c742d696e20636f6e747261696e7420646566696e6974696f6e730a20202020446566696e65436f6e73747261696e74496e697469616c697a6572730a0a2020202023205365742075702074686520636f6e73742e2e2e220a202020202866696c6520222f7573722f73686172652f636f6d7061742d74636c382e342f74636c74657374322e322f74636c746573742e74636c22206c696e652033323732290a20202020696e766f6b65642066726f6d2077697468696e0a22736f75726365202f7573722f73686172652f636f6d7061742d74636c382e342f74636c74657374322e322f74636c746573742e74636c220a2020202028227061636b6167652069666e65656465642220736372697074290a20202020696e766f6b65642066726f6d2077697468696e0a227061636b61676520726571756972652074636c74657374220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22"}, "return_options": null, "unavailable_fields": ["return_options"]}. Only this recorded invocation is answered.

## Scope

Only retained case tcltest, variant capability-runtime-control-v1, run nh20261012cap2, context iapp-implementation; event None, unit None, invocation 'once'. This independently scoped input does not replace an exact portable-input answer or establish other contexts, units, versions, recogniser coverage or source mechanisms. Authored measurement scope:  Authored/generated source descriptor is the retained case metadata, not an unchanged portable Tcl program.. Interpreter/action facts retained in the measurement: {"interpreter_path": null, "interpreter_patchlevel": null, "tcltest_version": null, "execution_namespace": null, "invoking_action": "definition"}. Null means not observed by this receipt.

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

[bigip-pattern.tcltest.iapp-implementation.capability-runtime-control-v1.nh20261012cap2.invocation-once.json](bigip-pattern.tcltest.iapp-implementation.capability-runtime-control-v1.nh20261012cap2.invocation-once.json) contains the exact evidence descriptors, original hashes and selectors. The canonical manifest remains the proof authority. The indexed F5 store resolves historical paths without retaining expanded duplicate trees.

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH --json-pointer /ROW
```

## Reproduction

The argument vector below records the historical invocation. Restore retained driver directories with `materialise.py extract` before replay, and apply the selected case's current address bindings and state prerequisites.

Replay requires the retained exact inputs, isolated appliance objects, recorded invoking context and independently captured load, runtime and cleanup outcomes. This registration performs no appliance execution.

```json
[]
```
