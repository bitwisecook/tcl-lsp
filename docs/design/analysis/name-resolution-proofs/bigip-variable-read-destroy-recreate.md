# naming.bigip.variable-read-destroy-recreate

Kind: `native-observation`

## Problem statement

A read callback can destroy and recreate its receiver before a command writes its result. Retaining the prior cell or trace registration would yield a different result and callback order.

## Question

Which cell and registration does the exact scalar/array lifecycle discriminator use after read-triggered unset/recreation?

## Conclusion

Modern and legacy runtime trace APIs install and complete on each recorded TMM. The main result is {NEW EXTRA} {NEW EXTRA}, with read then unset, exact name arguments, and no retained registration after recreation. The later ordinary-global controls are distinct and do not prove arbitrary cross-TMM sharing.

## Scope

The exact main-supported dynamic program on0:0 through0:3; literal trace/proc loader rejections are independent source inputs. BIG-IP21.1.0.1 build0.0.26 Point Release1 only; appliance implementation source is unavailable.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded.

No independent C Tcl or Jim execution of this appliance question is claimed.

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

Status: `observed`. Version: 21.1.0.1. Build: 0.0.26, Point Release1; no separate hotfix field reported.

Modern and legacy runtime trace APIs install and complete on each recorded TMM. The main result is {NEW EXTRA} {NEW EXTRA}, with read then unset, exact name arguments, and no retained registration after recreation. The later ordinary-global controls are distinct and do not prove arbitrary cross-TMM sharing.

The measured report rows and their limits are:


Modern `trace add variable` and legacy `trace variable` each installed at
runtime with catch code 0 on every TMM. The operation also completed with code
0 and returned byte-identical results on `0:0` through `0:3`:

```text
{NEW EXTRA} {NEW EXTRA}
```

For both APIs, callback order was `read`, then `unset`. `name1` was exactly
hex `52323238365f766c315f76616c7565` (`R2286_vl1_value`), `name2` was empty,
and `trace info variable` was empty after recreation. The final read did not
invoke the destroyed registration. The raw per-unit responses are
[`vl-main-u0.body`](../../../../experiments/oracles/f5/captures/README.md) (retained logical path: `scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-main-u0.body`)
through
[`vl-main-u3.body`](../../../../experiments/oracles/f5/captures/README.md) (retained logical path: `scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-main-u3.body`).


## Exact evidence

[bigip-variable-read-destroy-recreate.json](bigip-variable-read-destroy-recreate.json) contains the exact evidence descriptors, original hashes and selectors. The canonical manifest remains the proof authority. The indexed F5 store resolves historical paths without retaining expanded duplicate trees.

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH --json-pointer /ROW
```

## Reproduction

The argument vector below records the historical invocation. Restore retained driver directories with `materialise.py extract` before replay, and apply the selected case's current address bindings and state prerequisites.

The pinned report contains the exact source-generation, load and traffic protocol. This record claims no new appliance execution; rerun requires the recorded isolated appliance context, exact bytes and independently captured load/event outcomes. Closed implementation source and unmeasured releases are unavailable.

```json
[]
```
