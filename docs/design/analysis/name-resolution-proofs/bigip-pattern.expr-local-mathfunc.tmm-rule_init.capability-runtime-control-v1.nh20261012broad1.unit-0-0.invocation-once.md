# naming.bigip-pattern.expr-local-mathfunc.tmm-rule_init.capability-runtime-control-v1.nh20261012broad1.unit-0-0.invocation-once

Kind: `native-observation`

## Problem statement

Related question F5-N14; this record retains one reviewed provider/input/context observation independently of the original C/Jim and exact portable evidence.

## Question

What completion and result did retained variant capability-runtime-control-v1 of case expr-local-mathfunc produce in tmm-RULE_INIT, run nh20261012broad1, unit '0:0', invocation 'once'?

## Conclusion

Setup reached with completion 0. Intended lookup reached with completion 1; result encoding binary-scan-tcl-value; exact result hex "756e6b6e6f776e206d6174682066756e6374696f6e20227069636b22". Error metadata and availability: {"errorcode": null, "errorinfo": null, "return_options": null, "unavailable_fields": ["return_options", "errorcode", "errorinfo"]}. Only this recorded invocation is answered.

## Scope

Only retained case expr-local-mathfunc, variant capability-runtime-control-v1, run nh20261012broad1, context tmm-RULE_INIT; event 'RULE_INIT', unit '0:0', invocation 'once'. This independently scoped input does not replace an exact portable-input answer or establish other contexts, units, versions, recogniser coverage or source mechanisms. Authored measurement scope: Custom mathfunc lookup is gated; stock Tcl8.4 need not support it. Authored/generated source descriptor is the retained case metadata, not an unchanged portable Tcl program.. Interpreter/action facts retained in the measurement: {"interpreter_path": null, "interpreter_patchlevel": null, "tcltest_version": null, "execution_namespace": null, "invoking_action": null}. Null means not observed by this receipt. 

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No execution of this exact context/input/invocation is claimed for this provider.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No execution of this exact context/input/invocation is claimed for this provider.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No execution of this exact context/input/invocation is claimed for this provider.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No execution of this exact context/input/invocation is claimed for this provider.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No execution of this exact context/input/invocation is claimed for this provider.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No execution of this exact context/input/invocation is claimed for this provider.

### bigip

Status: `observed`. Version: 21.1.0.1. Build: 0.0.26; hotfix None. Channel: Only retained case expr-local-mathfunc, variant capability-runtime-control-v1, run nh20261012broad1, context tmm-RULE_INIT; event 'RULE_INIT', unit '0:0', invocation 'once'. This independently scoped input does not replace an exact portable-input answer or establish other contexts, units, versions, recogniser coverage or source mechanisms. Authored measurement scope: Custom mathfunc lookup is gated; stock Tcl8.4 need not support it. Authored/generated source descriptor is the retained case metadata, not an unchanged portable Tcl program.. Interpreter/action facts retained in the measurement: {"interpreter_path": null, "interpreter_patchlevel": null, "tcltest_version": null, "execution_namespace": null, "invoking_action": null}. Null means not observed by this receipt. . Dialect: F5 hosted Tcl.

Setup reached with completion 0. Intended lookup reached with completion 1; result encoding binary-scan-tcl-value; exact result hex "756e6b6e6f776e206d6174682066756e6374696f6e20227069636b22". Error metadata and availability: {"errorcode": null, "errorinfo": null, "return_options": null, "unavailable_fields": ["return_options", "errorcode", "errorinfo"]}. Only this recorded invocation is answered.

## Exact evidence

- `original-input` (input): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-cases.json](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-cases.json). SHA-256 `276886c26beae43cac425650b30e87f46b504b8d154451d43ca4b96be1ace058`. JSON pointer `/47`. Complete retained original_input bytes.
- `actual-input` (input): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/input.tcl](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/input.tcl). SHA-256 `9ff7632e8163eaef7d5f48838ecf58d2f95227896f78b98beb757335507c199c`. Complete retained actual_input bytes.
- `wrapper` (input): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/applied.conf](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/applied.conf). SHA-256 `a0a12a86936104e92e298bacb17c2f050a1cdc29941a52dd0a21725d46dc492f`. Complete retained wrapper bytes.
- `raw-0` (observation): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/runtime-logs.stdout](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/runtime-logs.stdout). SHA-256 `b63b228b17196728b3824d1f1af8ffff111bc4d7ea69984c6217b8484c45b9c2`. Exact retained capture or provider receipt.
- `raw-1` (observation): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/load.stdout](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/load.stdout). SHA-256 `ee4390bcab8007c72593daedc963baf6675e0a4850f394857aa98bce8b130eae`. Exact retained capture or provider receipt.
- `raw-2` (observation): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/load.stderr](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/load.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained capture or provider receipt.
- `raw-3` (observation): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/invocation.sh](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/invocation.sh). SHA-256 `ded6a4edf1ff22440f06cd93f59c678c9f915a988ff44a6556c76e9c88e6694b`. Exact retained capture or provider receipt.
- `raw-4` (observation): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/driver.json](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/driver.json). SHA-256 `f5873afb2584a611bd284966af0d19d21286ceaff902e78448a16a7d85e27529`. Exact retained capture or provider receipt.
- `raw-5` (observation): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/driver.stdout](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/driver.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained capture or provider receipt.
- `raw-6` (observation): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/driver.stderr](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/driver.stderr). SHA-256 `048e2ba89ae430962e02f5341cf60c694fb7e131cb417790075a56b32ca09992`. Exact retained capture or provider receipt.
- `raw-7` (observation): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/transport.stderr](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/transport.stderr). SHA-256 `048e2ba89ae430962e02f5341cf60c694fb7e131cb417790075a56b32ca09992`. Exact retained capture or provider receipt.
- `raw-8` (provider): [experiments/name-handling/f5/answers/nh20261012broad1/version.stdout](../../../../experiments/name-handling/f5/answers/nh20261012broad1/version.stdout). SHA-256 `ddd147403909c715e5de9c1b1292c9dead5cf2fada532c2eb94b8507483024b9`. Exact retained capture or provider receipt.
- `raw-9` (provider): [experiments/name-handling/f5/answers/nh20261012broad1/platform.stdout](../../../../experiments/name-handling/f5/answers/nh20261012broad1/platform.stdout). SHA-256 `f1dc3e5bc162517771c3ba6cca300cb6448a54daec71dbe12701fa9e8adeb28d`. Exact retained capture or provider receipt.
- `raw-10` (provider): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-roster.stdout](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-roster.stdout). SHA-256 `8df5ed08f6da6bc4f19075c355d37ddde555d253d682b4c0dd6deee54fc80275`. Exact retained capture or provider receipt.
- `raw-11` (provider): [experiments/name-handling/f5/answers/nh20261012broad1/failover.stdout](../../../../experiments/name-handling/f5/answers/nh20261012broad1/failover.stdout). SHA-256 `ca9cb40cb49f748be1e14f1782c6366fe802bb5f6eaaa5faa10d836cc544ae3e`. Exact retained capture or provider receipt.
- `raw-12` (provider): [experiments/name-handling/f5/answers/nh20261012broad1/provenance.json](../../../../experiments/name-handling/f5/answers/nh20261012broad1/provenance.json). SHA-256 `8dcc591477356fe3062b826af8bf4d8b24c1713ac1133223682749af7d388592`. Exact retained capture or provider receipt.
- `raw-13` (provider): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/applied-0.exit](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/applied-0.exit). SHA-256 `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa`. Exact retained capture or provider receipt.
- `raw-14` (provider): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/applied-0.stderr](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/applied-0.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained capture or provider receipt.
- `raw-15` (provider): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/applied-0.stdout](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/applied-0.stdout). SHA-256 `c608bf0bd9ca1ba94e8566e7e2decd17a16aa14f47c1c62a735205a5b6c0c9a6`. Exact retained capture or provider receipt.
- `cleanup-0` (observation): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/after-0.exit](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/after-0.exit). SHA-256 `4355a46b19d348dc2f57c046f8ef63d4538ebb936000f3c9ee954a27460dd865`. Retained cleanup result; scope remains explicit.
- `cleanup-1` (observation): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/after-0.stderr](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/after-0.stderr). SHA-256 `46b14467d12b440690b0bb648ea3c326d33b30be407db5e19f1b9572a39ab3ec`. Retained cleanup result; scope remains explicit.
- `cleanup-2` (observation): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/after-0.stdout](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/after-0.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Retained cleanup result; scope remains explicit.
- `cleanup-3` (observation): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/before-0.exit](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/before-0.exit). SHA-256 `4355a46b19d348dc2f57c046f8ef63d4538ebb936000f3c9ee954a27460dd865`. Retained cleanup result; scope remains explicit.
- `cleanup-4` (observation): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/before-0.stderr](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/before-0.stderr). SHA-256 `46b14467d12b440690b0bb648ea3c326d33b30be407db5e19f1b9572a39ab3ec`. Retained cleanup result; scope remains explicit.
- `cleanup-5` (observation): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/before-0.stdout](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/before-0.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Retained cleanup result; scope remains explicit.
- `cleanup-6` (observation): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/delete-0.exit](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/delete-0.exit). SHA-256 `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa`. Retained cleanup result; scope remains explicit.
- `cleanup-7` (observation): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/delete-0.stderr](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/delete-0.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Retained cleanup result; scope remains explicit.
- `cleanup-8` (observation): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/delete-0.stdout](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/delete-0.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Retained cleanup result; scope remains explicit.
- `cleanup-9` (observation): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/save-status.txt](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-RULE_INIT-15/save-status.txt). SHA-256 `9ab32790137e08ad76065e5e03719856b623e5a6125917840e258e4e73eac446`. Retained cleanup result; scope remains explicit.
- `extra-0` (provider): [experiments/name-handling/f5/answers/nh20261012broad1/version.stdout](../../../../experiments/name-handling/f5/answers/nh20261012broad1/version.stdout). SHA-256 `ddd147403909c715e5de9c1b1292c9dead5cf2fada532c2eb94b8507483024b9`. Independent retained provider or run provenance.
- `extra-1` (provider): [experiments/name-handling/f5/answers/nh20261012broad1/tmm-roster.stdout](../../../../experiments/name-handling/f5/answers/nh20261012broad1/tmm-roster.stdout). SHA-256 `8df5ed08f6da6bc4f19075c355d37ddde555d253d682b4c0dd6deee54fc80275`. Independent retained provider or run provenance.
- `measurement` (observation): [experiments/name-handling/f5/answers/nh20261012broad1/records.json](../../../../experiments/name-handling/f5/answers/nh20261012broad1/records.json). SHA-256 `9a043d9cf74d2b02d76203c639710e3ae7fcd6b8e6f87643122177b45e446cf3`. JSON pointer `/440`. Reviewed measurement record.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Replay requires the retained exact inputs, isolated appliance objects, recorded invoking context and independently captured load, runtime and cleanup outcomes. This registration performs no appliance execution.
