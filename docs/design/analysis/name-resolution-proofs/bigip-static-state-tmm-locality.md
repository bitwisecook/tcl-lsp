# naming.bigip.static-state-tmm-locality

Kind: `native-observation`

## Problem statement

A static:: cell can be seeded during initialization and mutated in an event. Treating the initialization broadcast as an event-write broadcast would give consumers the wrong state on another TMM. Ordinary globals and collision owners add separate confounders.

## Question

What state did each actually reached TMM observe after initialization, target-unit write, unset/recreate and exact rule recreation?

## Conclusion

For the recorded rule/build, the seed was visible on all four reached TMMs; event write/unset/recreation affected only target0:2. Rule recreation supplied new initialization. The report separately retains ordinary-global demotion and creation-order collision controls; these do not prove arbitrary global sharing.

## Scope

RULE_INIT plus actual traffic on0:0 through0:3, with CMP-disabled0:0 controls; recorded static/global/collision programs only. BIG-IP21.1.0.1 build0.0.26 Point Release1 only; appliance implementation source is unavailable.

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

For the recorded rule/build, the seed was visible on all four reached TMMs; event write/unset/recreation affected only target0:2. Rule recreation supplied new initialization. The report separately retains ordinary-global demotion and creation-order collision controls; these do not prove arbitrary global sharing.

The measured report rows and their limits are:


| Case | CMP mode and reached TMMs | Measured result |
| --- | --- | --- |
| `init_identity` | Rule load; all four TMM logs | `group=0`, units 0–3, `count=4` |
| `static_group` initial read | disabled: `0:0`; enabled: all `0:0`–`0:3` | Every reached TMM had `INIT_r2286_20261006c` |
| `static_group` write | enabled, target `0:2`, all four reached | Only `0:2` changed to `EVENT_0:2`; others remained `INIT_…` |
| `static_group` unset | enabled, target `0:2`, all four reached | First target request changed existence 1→0; later reads on `0:2` were `MISSING`; others retained `INIT_…` |
| `static_group` recreate | enabled, target `0:2`, all four reached | `0:2` became `RECREATED_0:2`; others retained `INIT_…` |
| `static_group` new initialization | delete/recreate exact rule | Four new `RULE_INIT` logs and four fresh `INIT_…` reads |
| `global` complete matrix | disabled: unit 0; enabled requested, only unit 0 reached | Write→`EVENT_0`, unset→missing, recreate→`RECREATED_0`; CMP-enabled distribution remained 128/128 on unit 0 in every traffic batch |
| collision A→B creation | enabled, units 0–3 | Initial observer value `INIT_b` everywhere |
| collision B→A creation | enabled, units 0–3 | Initial observer value `INIT_a` everywhere |
| reverse attachment only | enabled, units 0–3 | Did not replace the current value; explicit event priority controlled writes |
| collision single-rule recreation | enabled, units 0–3 | Initial A→B `INIT_b`; recreate A → `INIT_a` everywhere; recreate B → `INIT_b` everywhere |

Raw state responses are in the `traffic-static-group-*`, `traffic-global-*`, and `traffic-collision-*` files under [dev evidence](../../../../experiments/oracles/f5/captures/README.md) (retained logical path: `scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/dev`). Single-rule recreation is separately retained under [single-recreate](../../../../experiments/oracles/f5/captures/README.md) (retained logical path: `scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/single-recreate`).


## Exact evidence

[bigip-static-state-tmm-locality.json](bigip-static-state-tmm-locality.json) contains the exact evidence descriptors, original hashes and selectors. The canonical manifest remains the proof authority. The indexed F5 store resolves historical paths without retaining expanded duplicate trees.

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH --json-pointer /ROW
```

## Reproduction

The argument vector below records the historical invocation. Restore retained driver directories with `materialise.py extract` before replay, and apply the selected case's current address bindings and state prerequisites.

The pinned report contains the exact source-generation, load and traffic protocol. This record claims no new appliance execution; rerun requires the recorded isolated appliance context, exact bytes and independently captured load/event outcomes. Closed implementation source and unmeasured releases are unavailable.

```json
[]
```
