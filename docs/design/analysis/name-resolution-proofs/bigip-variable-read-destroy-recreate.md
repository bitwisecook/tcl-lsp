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

Status: `observed`. Version: 21.1.0.1. Build: 0.0.26, Point Release1; no separate hotfix field reported. Channel: The exact main-supported dynamic program on0:0 through0:3; literal trace/proc loader rejections are independent source inputs.. Dialect: F5 iRules and the explicitly recorded hosted contexts.

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
[`vl-main-u0.body`](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-main-u0.body)
through
[`vl-main-u3.body`](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-main-u3.body).


## Exact evidence

- `report-section` (observation): [scripts/dev/bigip-probes/resolution-2286/VARIABLE_LIFECYCLE_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/VARIABLE_LIFECYCLE_RESULTS.md). SHA-256 `469555b59e24291fa07e50a5aaf531ff13034270a20d44cdde4ad4b01cb833d8`. Lines 99–116. Exact build-specific measured report section; all rows and limits remain in the retained report.
- `provider-version` (provider): [scripts/dev/bigip-probes/resolution-2286/VARIABLE_LIFECYCLE_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/VARIABLE_LIFECYCLE_RESULTS.md). SHA-256 `469555b59e24291fa07e50a5aaf531ff13034270a20d44cdde4ad4b01cb833d8`. Lines 11–11. Actual appliance version attribution for this report; no separately named hotfix inferred.
- `raw-0` (observation): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-main-u0.body](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-main-u0.body). SHA-256 `f1911b59cecbf6f55e3e6bb714d57888849a3f3c15295b96a8953b73fea0afc0`. Exact source or raw evidence explicitly linked by the measured section; its own context/channel remains authoritative.
- `raw-1` (observation): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-main-u3.body](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-main-u3.body). SHA-256 `f1911b59cecbf6f55e3e6bb714d57888849a3f3c15295b96a8953b73fea0afc0`. Exact source or raw evidence explicitly linked by the measured section; its own context/channel remains authoritative.
- `source-closure-4` (input): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/appliance/r2286vl1-fixtures-v4/manifest.json](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/appliance/r2286vl1-fixtures-v4/manifest.json). SHA-256 `5d926a45e3efa7916ca5ef8af3800fe9516d5ce5c9af74401854584649fccbbf`. Retained report-specific source manifest or appliance inventory; individual measured inputs remain distinct.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The pinned report contains the exact source-generation, load and traffic protocol. This record claims no new appliance execution; rerun requires the recorded isolated appliance context, exact bytes and independently captured load/event outcomes. Closed implementation source and unmeasured releases are unavailable.
