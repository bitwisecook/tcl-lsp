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

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### bigip

Status: `observed`. Version: 21.1.0.1. Build: 0.0.26, Point Release1; no separate hotfix field reported. Channel: RULE_INIT plus actual traffic on0:0 through0:3, with CMP-disabled0:0 controls; recorded static/global/collision programs only.. Dialect: F5 iRules and the explicitly recorded hosted contexts.

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

Raw state responses are in the `traffic-static-group-*`, `traffic-global-*`, and `traffic-collision-*` files under [dev evidence](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/dev). Single-rule recreation is separately retained under [single-recreate](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/single-recreate).


## Exact evidence

- `report-section` (observation): [scripts/dev/bigip-probes/resolution-2286/BIGIP_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/BIGIP_RESULTS.md). SHA-256 `9bb20a639904e1427e4c5cd8ed45dc8808f345fbc96969215eb8fac0f8d3109f`. Lines 66–83. Exact build-specific measured report section; all rows and limits remain in the retained report. Sanitised retained artifact; lab transport/device details removed, without an appliance rerun.
- `provider-version` (provider): [scripts/dev/bigip-probes/resolution-2286/BIGIP_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/BIGIP_RESULTS.md). SHA-256 `9bb20a639904e1427e4c5cd8ed45dc8808f345fbc96969215eb8fac0f8d3109f`. Lines 3–3. Actual appliance version attribution for this report; no separately named hotfix inferred. Sanitised retained artifact; lab transport/device details removed, without an appliance rerun.
- `source-closure-2` (provider): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/inventory.txt](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/inventory.txt). SHA-256 `4daaddca9c1f7f47b318ff85770bc07ad2da6e3582ef8502a2cfe284017201d3`. Retained report-specific source manifest or appliance inventory; individual measured inputs remain distinct. Sanitised retained artifact; lab transport/device details removed, without an appliance rerun.
- `source-closure-3` (input): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/manifest-verification.txt](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/manifest-verification.txt). SHA-256 `f800077eaadc0104201639706c8d912d3804118dd6175be3f9cd727052d1bbbe`. Retained report-specific source manifest or appliance inventory; individual measured inputs remain distinct.
- `source-closure-4` (input): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/appliance/manifest.json](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/appliance/manifest.json). SHA-256 `8f5c9f385b226b5f7da6885ede6a28775a68cdfea7ed44fb28fb213c73f6e6fd`. Retained report-specific source manifest or appliance inventory; individual measured inputs remain distinct.
- `aggregate-5` (observation): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/result.json](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/result.json). SHA-256 `800bb0866f049ab9beea0c772915c94f68052fabab7ba2c4e3b77873248b9b60`. Inspected measured canonical case aggregate and its explicit unmeasured limits; actual case rows remain in this exact file. Sanitised retained artifact; lab transport/device details removed, without an appliance rerun.
- `authored-static-policy-regression675` (implementation): [rust/tcl-vm/src/interp/authored_tmm_static.rs](../../../../rust/tcl-vm/src/interp/authored_tmm_static.rs). SHA-256 `e0c90231d9f056fdbaa48a52a06f034846e857c02b64d29e3aa2902448d6756a`. Existing explicit authored static publication policy and software regressions reading retained build-specific appliance state responses. This authored source is not BIG-IP implementation source and supplies no new provider execution, scalar-trace chronology, collision or native rule-loader proof.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/interp/authored_tmm_static.rs](../../../../rust/tcl-vm/src/interp/authored_tmm_static.rs), `Vm::install_irules_static_simulation`: Install the explicit authored RuleInitPublication policy independently of physical Native object provider and source grammar.
- [rust/tcl-vm/src/interp/authored_tmm_static.rs](../../../../rust/tcl-vm/src/interp/authored_tmm_static.rs), `Vm::publish_authored_static`: Publish only the selected authored initialization context across live enrolled worker namespaces; ordinary executing-worker event mutations remain in their actual worker.
- [rust/tcl-vm/src/interp/authored_tmm_static.rs](../../../../rust/tcl-vm/src/interp/authored_tmm_static.rs), `Vm::apply_static_publication`: Apply explicit authored initialization publication through existing real worker namespace cells and preserve the policy's existing recipient operation semantics. The retained appliance responses constrain only the named software state regressions.
- [rust/tcl-vm/src/interp/authored_tmm_static.rs](../../../../rust/tcl-vm/src/interp/authored_tmm_static.rs), `interp::authored_tmm_static::tests::authored_static_policy_matches_recorded_four_unit_seed_and_target_mutation_states` (linked): Explicit authored RuleInitPublication software policy with four genuine enrolled VM interpreters reads the 512 retained static_group HTTP responses for the four reached TMMs. It compares seed, target-unit write, missing after unset and target-unit recreation states, then checks the policy's explicit reinitialisation against the retained source/report seed. This does not execute BIG-IP or establish native rule-loader, trace, global, CMP or collision behavior.

A named test is a coverage binding, not a claim that it executed.

## Replay

The pinned report contains the exact source-generation, load and traffic protocol. This record claims no new appliance execution; rerun requires the recorded isolated appliance context, exact bytes and independently captured load/event outcomes. Closed implementation source and unmeasured releases are unavailable.
