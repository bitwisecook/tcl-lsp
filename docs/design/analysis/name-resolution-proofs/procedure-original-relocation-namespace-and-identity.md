# naming.procedure.original-relocation-namespace-and-identity

Kind: `native-observation`

## Problem statement

Moving a procedure command can preserve its command identity while changing the namespace used by future calls. An already active frame can retain a different namespace, and Jim namespace-object reuse makes pointer comparison unsafe unless the original object is pinned. This probe compares those owners separately within one interpreter.

## Question

Which original command/declaration and active/future namespace owners survive the six recorded procedure moves?

## Conclusion

All C captures retain both original command and Proc identity; Jim retains command identity. C root moves change future namespace to root while the already active frame keeps its original namespace. Jim root moves in the two root cases retain the original procedure namespace object and future calls report ::A; other destination qualification controls select the recorded new namespaces. The Jim observer pins the original namespace object to prevent freed-address reuse. No display name or pointer from another process can mint this identity.

## Scope

Six exact move controls per engine; C8.4.20–9.1.0 and the hashed Jim library are observed. Jim reported patchlevel/revision and executable hashes are not retained. BIG-IP is not tested. Namespace pointer comparison stays within the retained interpreter and lifetime.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Exact compiler command, library/header/source hashes and available executable hashes are retained in the selected receipt. A recorded source/build release is not a newly executed startup check.. Channel: ASCII source via Tcl_Eval/Jim_Eval plus retained original command/private procedure and namespace objects, within one native interpreter.. Dialect: Tcl.

Six moves retain original command and Proc identity; active frames and future namespace move are recorded separately; root moves future namespace ::.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Exact compiler command, library/header/source hashes and available executable hashes are retained in the selected receipt. A recorded source/build release is not a newly executed startup check.. Channel: ASCII source via Tcl_Eval/Jim_Eval plus retained original command/private procedure and namespace objects, within one native interpreter.. Dialect: Tcl.

Six moves retain original command and Proc identity; active frames and future namespace move are recorded separately; root moves future namespace ::.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Exact compiler command, library/header/source hashes and available executable hashes are retained in the selected receipt. A recorded source/build release is not a newly executed startup check.. Channel: ASCII source via Tcl_Eval/Jim_Eval plus retained original command/private procedure and namespace objects, within one native interpreter.. Dialect: Tcl.

Six moves retain original command and Proc identity; active frames and future namespace move are recorded separately; root moves future namespace ::.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Exact compiler command, library/header/source hashes and available executable hashes are retained in the selected receipt. A recorded source/build release is not a newly executed startup check.. Channel: ASCII source via Tcl_Eval/Jim_Eval plus retained original command/private procedure and namespace objects, within one native interpreter.. Dialect: Tcl.

Six moves retain original command and Proc identity; active frames and future namespace move are recorded separately; root moves future namespace ::.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Exact compiler command, library/header/source hashes and available executable hashes are retained in the selected receipt. A recorded source/build release is not a newly executed startup check.. Channel: ASCII source via Tcl_Eval/Jim_Eval plus retained original command/private procedure and namespace objects, within one native interpreter.. Dialect: Tcl.

Six moves retain original command and Proc identity; active frames and future namespace move are recorded separately; root moves future namespace ::.

### jim

Status: `observed`. Version: Jim; patchlevel/revision not retained. Build: Exact compiler command, library/header/source hashes and available executable hashes are retained in the selected receipt. A recorded source/build release is not a newly executed startup check.. Channel: ASCII source via Tcl_Eval/Jim_Eval plus retained original command/private procedure and namespace objects, within one native interpreter.. Dialect: Jim Tcl.

Six moves retain command identity; root rename controls preserve original namespace holder and future ::A; active/future and qualification rows remain separately recorded.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (observation): [runtime/rust/tests/data/native_procedure_relocation/manifest.json](../../../../runtime/rust/tests/data/native_procedure_relocation/manifest.json). SHA-256 `c4b50f9543319bef2dad9cefa47a22392e61972142a33594c636c704b4b90cda`. Six original source/header/library-hashed compile/run receipts with exact inline streams; executable hashes and a Jim patchlevel/revision are not retained.
- `e1` (input): [runtime/rust/tests/data/native_procedure_relocation/probe.c](../../../../runtime/rust/tests/data/native_procedure_relocation/probe.c). SHA-256 `e8d9b48f63b6dd13877186ec1f0b502925e19d82aad96689f0621130c282a754`. Six move/source-call controls, private command/procedure comparison and pinned Jim namespace-object observer.
- `e2` (observation): [runtime/rust/tests/data/native_procedure_relocation/observations.tsv](../../../../runtime/rust/tests/data/native_procedure_relocation/observations.tsv). SHA-256 `2cf71506762fcd5f04d765e1b6bd271df75f8e75f69e0e70c0f42b3a6604712a`. Projected active/first/future namespace and command/procedure identity fields for all36 controls.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/interp/native_procedure_relocation_tests.rs](../../../../runtime/rust/src/interp/native_procedure_relocation_tests.rs), `relocation_retains_native_binding_and_declaration_with_separate_active_namespace` (linked): Compare each actual command/declaration identity and original active/future namespace independently.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cc",
  "-DSTDC_HEADERS=1",
  "-DHAVE_UNISTD_H=1",
  "-I${TCL_SOURCE}/generic",
  "-I${TCL_SOURCE}/unix",
  "runtime/rust/tests/data/native_procedure_relocation/probe.c",
  "${TCL_BUILD}/libtcl${TCL_ABI}.a",
  "-lm",
  "-ldl",
  "-lpthread",
  "-lz",
  "-o",
  "${PROBE_EXE}"
]
```

Jim requires the matching captured jim.h/libjim.a, -DJIM_PROBE=1 and its recorded SSL/crypto links; do not borrow the current Jim executable version for this capture. Replay requires the matching recorded headers/library and original source bytes; compare process exit, stdout and stderr exactly before comparing projected rows. Private header/pointer observations are valid only within each original process and cannot authenticate a different interpreter. No Rust execution or fresh native reconfirmation is asserted.
