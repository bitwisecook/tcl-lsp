# naming.namespace.dispatch-prefix-worker-availability

Kind: `native-observation`

## Problem statement

A one-letter namespace prefix can be missing, ambiguous or uniquely select upvar depending on the available surface. A completion or diagnostic consumer must not infer prefix availability from another release.

## Question

Does namespace u reject a missing or ambiguous prefix, or reach the upvar argument guard?

## Conclusion

C8.4 reports bad option u. C8.5 reports ambiguous option u. C8.6–9.1 report unknown or ambiguous subcommand u. Jim uniquely reaches the upvar worker and reports its wrong-argument usage. No successful variable link is reached by this zero-argument control.

## Scope

Selected u from24 actual caught shell controls across six engines. The receipt preserves source SHA, executable SHA, process/guest streams and60-second budgets; original script/presenter bytes and file-versus-stdin argument vectors are not retained. errors.tsv is an18-row message projection, checked against exact embedded outputs. available.tsv records the release-specific surfaced choices. No raw zero/opaque object, callback, primary, return-options or BIG-IP measurement.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4; full launched patchlevel not independently queried by this capture. Build: Recorded executable SHA 551bef3a9b51f256f4ad9dec06bddc42ceab872a72528e83676d1d7df715b9d4; library/header/compiler/configuration closure unrecorded.. Channel: Shell source; original channel/presenter unavailable. Dialect: C Tcl.

Exact reported output:

```text
1 {bad option "u": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which}
```

Outer status0 and empty stderr are distinct from each caught guest code1.

### tcl8.5

Status: `observed`. Version: 8.5; full launched patchlevel not independently queried by this capture. Build: Recorded executable SHA 7a0fb4aa272a45662fb883295d0037a8c18e3e32c635736af8314fb835981935; library/header/compiler/configuration closure unrecorded.. Channel: Shell source; original channel/presenter unavailable. Dialect: C Tcl.

Exact reported output:

```text
1 {ambiguous option "u": must be children, code, current, delete, ensemble, eval, exists, export, forget, import, inscope, origin, parent, path, qualifiers, tail, unknown, upvar, or which}
```

Outer status0 and empty stderr are distinct from each caught guest code1.

### tcl8.6

Status: `observed`. Version: 8.6; full launched patchlevel not independently queried by this capture. Build: Recorded executable SHA 0cc67113840d03c15c15d88188c16b8b019aa2a8c11ca6980f32723fbedacde0; library/header/compiler/configuration closure unrecorded.. Channel: Shell source; original channel/presenter unavailable. Dialect: C Tcl.

Exact reported output:

```text
1 {unknown or ambiguous subcommand "u": must be children, code, current, delete, ensemble, eval, exists, export, forget, import, inscope, origin, parent, path, qualifiers, tail, unknown, upvar, or which}
```

Outer status0 and empty stderr are distinct from each caught guest code1.

### tcl9.0

Status: `observed`. Version: 9.0; full launched patchlevel not independently queried by this capture. Build: Recorded executable SHA cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18; library/header/compiler/configuration closure unrecorded.. Channel: Shell source; original channel/presenter unavailable. Dialect: C Tcl.

Exact reported output:

```text
1 {unknown or ambiguous subcommand "u": must be children, code, current, delete, ensemble, eval, exists, export, forget, import, inscope, origin, parent, path, qualifiers, tail, unknown, upvar, or which}
```

Outer status0 and empty stderr are distinct from each caught guest code1.

### tcl9.1

Status: `observed`. Version: 9.1; full launched patchlevel not independently queried by this capture. Build: Recorded executable SHA d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c; library/header/compiler/configuration closure unrecorded.. Channel: Shell source; original channel/presenter unavailable. Dialect: C Tcl.

Exact reported output:

```text
1 {unknown or ambiguous subcommand "u": must be children, code, current, delete, ensemble, eval, exists, export, forget, import, inscope, origin, parent, path, qualifiers, tail, unknown, upvar, or which}
```

Outer status0 and empty stderr are distinct from each caught guest code1.

### jim

Status: `observed`. Version: jim; full launched patchlevel not independently queried by this capture; revision and UTF configuration unrecorded. Build: Recorded executable SHA d5b47eb75b271d50331ca5a38775492cb186f8feedd3c76bb2a0fbf1905671f0; library/header/compiler/configuration closure unrecorded.. Channel: Shell source; original channel/presenter unavailable. Dialect: Jim Tcl.

Exact reported output:

```text
1 {wrong # args: should be "namespace upvar ns ?arg ...?"}
```

Outer status0 and empty stderr are distinct from each caught guest code1.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No captured result for this exact purpose is attached for this provider.

## Exact evidence

- `receipt` (provider): [rust/tcl-cmd-core/tests/data/native_namespace_dispatch/provenance.json](../../../../rust/tcl-cmd-core/tests/data/native_namespace_dispatch/provenance.json). SHA-256 `c537f7c08df7628e997d0855451857f4e51184d2f8b6d796412868fa0e919786`. Full24 original caught dispatcher/usage results with exact source/executable associations.
- `errors` (observation): [rust/tcl-cmd-core/tests/data/native_namespace_dispatch/errors.tsv](../../../../rust/tcl-cmd-core/tests/data/native_namespace_dispatch/errors.tsv). SHA-256 `e72d24a7447bf92334669c6f2f38884b544f029aa0c2a095c1c0e988692e9c36`. Eighteen original message-byte projections verified against complete embedded outputs.
- `surface` (observation): [rust/tcl-cmd-core/tests/data/native_namespace_dispatch/available.tsv](../../../../rust/tcl-cmd-core/tests/data/native_namespace_dispatch/available.tsv). SHA-256 `063a72cb6b1a7acaafc29ba71102d49f2bdcfd9674754bbf8df2b8b31c908921`. Six captured choice-list surfaces; not an arbitrary current registry grant.
- `row-tcl8.4-u` (observation): [rust/tcl-cmd-core/tests/data/native_namespace_dispatch/provenance.json](../../../../rust/tcl-cmd-core/tests/data/native_namespace_dispatch/provenance.json). SHA-256 `c537f7c08df7628e997d0855451857f4e51184d2f8b6d796412868fa0e919786`. JSON pointer `/rows/3`. Actual selected caught guest result and independent outer status.
- `row-tcl8.5-u` (observation): [rust/tcl-cmd-core/tests/data/native_namespace_dispatch/provenance.json](../../../../rust/tcl-cmd-core/tests/data/native_namespace_dispatch/provenance.json). SHA-256 `c537f7c08df7628e997d0855451857f4e51184d2f8b6d796412868fa0e919786`. JSON pointer `/rows/7`. Actual selected caught guest result and independent outer status.
- `row-tcl8.6-u` (observation): [rust/tcl-cmd-core/tests/data/native_namespace_dispatch/provenance.json](../../../../rust/tcl-cmd-core/tests/data/native_namespace_dispatch/provenance.json). SHA-256 `c537f7c08df7628e997d0855451857f4e51184d2f8b6d796412868fa0e919786`. JSON pointer `/rows/11`. Actual selected caught guest result and independent outer status.
- `row-tcl9.0-u` (observation): [rust/tcl-cmd-core/tests/data/native_namespace_dispatch/provenance.json](../../../../rust/tcl-cmd-core/tests/data/native_namespace_dispatch/provenance.json). SHA-256 `c537f7c08df7628e997d0855451857f4e51184d2f8b6d796412868fa0e919786`. JSON pointer `/rows/15`. Actual selected caught guest result and independent outer status.
- `row-tcl9.1-u` (observation): [rust/tcl-cmd-core/tests/data/native_namespace_dispatch/provenance.json](../../../../rust/tcl-cmd-core/tests/data/native_namespace_dispatch/provenance.json). SHA-256 `c537f7c08df7628e997d0855451857f4e51184d2f8b6d796412868fa0e919786`. JSON pointer `/rows/19`. Actual selected caught guest result and independent outer status.
- `row-jim-u` (observation): [rust/tcl-cmd-core/tests/data/native_namespace_dispatch/provenance.json](../../../../rust/tcl-cmd-core/tests/data/native_namespace_dispatch/provenance.json). SHA-256 `c537f7c08df7628e997d0855451857f4e51184d2f8b6d796412868fa0e919786`. JSON pointer `/rows/23`. Actual selected caught guest result and independent outer status.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-cmd-core/src/namespace.rs](../../../../rust/tcl-cmd-core/src/namespace.rs), `unknown_subcommand_message`: Selects the actual release noun and complete worker choice list for a rejected original byte selector.
- [rust/tcl-cmd-core/src/namespace.rs](../../../../rust/tcl-cmd-core/src/namespace.rs), `namespace::native_dispatch_diagnostic_tests::original_namespace_dispatch_errors_match_all_eighteen_native_results` (linked): Compares all18 attached rejected-worker messages using each captured surface. Jim u uniquely reaches upvar and is outside this unknown/ambiguous-message assertion.

A named test is a coverage binding, not a claim that it executed.

## Replay

No new native or Rust execution is claimed. The original presenter is absent where stated; a source digest alone does not reconstruct it. A new capture must retain exact source bytes, selected provider version/build, channel, separate process/guest codes and streams. Existing capture scripts write the retained fixture and use retained absolute launch paths; they are input/observer context, not an executable safe replay command.
