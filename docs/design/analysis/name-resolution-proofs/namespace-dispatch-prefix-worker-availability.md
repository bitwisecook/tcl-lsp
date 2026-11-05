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

Status: `observed`. Channel: Shell source; original channel/presenter unavailable. Version: tcl8.4 8.4; full launched patchlevel not independently queried by this capture.

Exact reported output:

```text
1 {bad option "u": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which}
```

Outer status0 and empty stderr are distinct from each caught guest code1.

### tcl8.5

Status: `observed`. Channel: Shell source; original channel/presenter unavailable. Version: tcl8.5 8.5; full launched patchlevel not independently queried by this capture.

Exact reported output:

```text
1 {ambiguous option "u": must be children, code, current, delete, ensemble, eval, exists, export, forget, import, inscope, origin, parent, path, qualifiers, tail, unknown, upvar, or which}
```

Outer status0 and empty stderr are distinct from each caught guest code1.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Shell source; original channel/presenter unavailable. Version: tcl8.6 8.6; full launched patchlevel not independently queried by this capture; tcl9.0 9.0; full launched patchlevel not independently queried by this capture; tcl9.1 9.1; full launched patchlevel not independently queried by this capture.

Exact reported output:

```text
1 {unknown or ambiguous subcommand "u": must be children, code, current, delete, ensemble, eval, exists, export, forget, import, inscope, origin, parent, path, qualifiers, tail, unknown, upvar, or which}
```

Outer status0 and empty stderr are distinct from each caught guest code1.

### jim

Status: `observed`. Channel: Shell source; original channel/presenter unavailable. Version: jim jim; full launched patchlevel not independently queried by this capture.

Exact reported output:

```text
1 {wrong # args: should be "namespace upvar ns ?arg ...?"}
```

Outer status0 and empty stderr are distinct from each caught guest code1.

### bigip

Status: `not-tested`.

No captured result for this exact purpose is attached for this provider.

## Consumer bindings

- [rust/tcl-cmd-core/src/namespace.rs](../../../../rust/tcl-cmd-core/src/namespace.rs), `unknown_subcommand_message`: Selects the actual release noun and complete worker choice list for a rejected original byte selector.
- [rust/tcl-cmd-core/src/namespace.rs](../../../../rust/tcl-cmd-core/src/namespace.rs), `namespace::native_dispatch_diagnostic_tests::original_namespace_dispatch_errors_match_all_eighteen_native_results` (linked): Compares all18 attached rejected-worker messages using each captured surface. Jim u uniquely reaches upvar and is outside this unknown/ambiguous-message assertion.

## Evidence

- [rust/tcl-cmd-core/tests/data/native_namespace_dispatch/provenance.json](../../../../rust/tcl-cmd-core/tests/data/native_namespace_dispatch/provenance.json)
- [rust/tcl-cmd-core/tests/data/native_namespace_dispatch/errors.tsv](../../../../rust/tcl-cmd-core/tests/data/native_namespace_dispatch/errors.tsv)
- [rust/tcl-cmd-core/tests/data/native_namespace_dispatch/available.tsv](../../../../rust/tcl-cmd-core/tests/data/native_namespace_dispatch/available.tsv)
