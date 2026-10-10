# naming.property.original-accessor-clientdata

Kind: `native-observation`

## Problem statement

Default property accessors may retain the original declaration-name object as client data, rather than a separately rendered name. Confusing that ownership with the caller management option can change reference counts or cache the wrong object. The check uses direct counted name operands, including non-UTF8 and NUL, and inspects the exact C9.1 accessor objects.

## Question

Do default C9.1 property accessors retain the original counted declaration-name object and leave its primary representation unchanged?

## Conclusion

For p, xFF and x00tail, both generated accessor client-data pointers equal the original declaration object; it has three references and no typed primary at declaration, write and read. Default write returns empty; read returns the exact supplied VALUE object. The caller management option remains untyped. This does not establish general method-name equality or object ownership outside the captured accessor lifecycle.

## Scope

Only the recorded C Tcl9.1.0 process linked to the hashed private-header build is observed. Other C releases, Jim and BIG-IP are not tested for this question. Native object/cache/reference snapshots are distinct from source-name bytes, command dispatch and current runtime ownership.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Exact compiler command, library/header/source hashes and available executable hashes are retained in the selected receipt. A recorded source/build release is not a newly executed startup check.. Channel: ASCII Tcl_Eval setup plus directly constructed counted Tcl_Obj names used by Tcl_EvalObjv flags0.. Dialect: Tcl.

Three declaration/write/read windows: accessor clientData pointer equality 1/1, declaration refcount3/type none, empty write1, original read-value identity1 and caller primary none.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (observation): [runtime/rust/tests/data/native_property_clientdata/manifest.json](../../../../runtime/rust/tests/data/native_property_clientdata/manifest.json). SHA-256 `1cc949117726faff50a59a5ec4c404f2e1adf06da0d7848afb9d1b88b7dcf57d`. Compile/run receipt, exact source/header/library/executable hashes and all nine output rows.
- `e1` (input): [runtime/rust/tests/data/native_property_clientdata/probe.c](../../../../runtime/rust/tests/data/native_property_clientdata/probe.c). SHA-256 `3104127ec7a6553d9506914baf25c8fc7365dfe5d09a4cf63ded6f6c0492f783`. Direct counted Tcl_NewStringObj name operands and Tcl_EvalObjv calls; private accessor client-data inspection precedes subsequent management operations.
- `e2` (observation): [runtime/rust/tests/data/native_property_clientdata/native.tsv](../../../../runtime/rust/tests/data/native_property_clientdata/native.tsv). SHA-256 `85b10c7190eb4cfe5303e577a31fee9ccb70d060a2f54c395d90988f5e5bddc5`. Exact nine-row stdout, independently equal to the receipt stream.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/cmd_oo/native_properties.rs](../../../../runtime/rust/src/cmd_oo/native_properties.rs), `default_property_methods_retain_original_clientdata_and_leave_its_primary` (linked): Compare accessor client-data identity, original declaration reference count/primary and caller cache separately.
- [rust/tcl-vm/src/cmd_oo/native_properties.rs](../../../../rust/tcl-vm/src/cmd_oo/native_properties.rs), `default_property_methods_retain_original_clientdata_and_leave_its_primary` (linked): Compare accessor client-data identity, original declaration reference count/primary and caller cache separately.

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
  "runtime/rust/tests/data/native_property_clientdata/probe.c",
  "${TCL_BUILD}/libtcl${TCL_ABI}.a",
  "-lm",
  "-ldl",
  "-lpthread",
  "-lz",
  "-o",
  "${PROBE_EXE}"
]
```

 Replay requires the matching recorded headers/library and original source bytes; compare process exit, stdout and stderr exactly before comparing projected rows. Private header/pointer observations are valid only within each original process and cannot authenticate a different interpreter. No Rust execution or fresh native reconfirmation is asserted.
