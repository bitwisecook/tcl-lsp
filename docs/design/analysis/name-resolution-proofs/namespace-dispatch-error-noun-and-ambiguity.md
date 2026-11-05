# naming.namespace.dispatch-error-noun-and-ambiguity

Kind: `native-observation`

## Problem statement

An invalid namespace worker and an ambiguous prefix can use different nouns and choice lists across releases. Reporting every rejected word as the same option error would change the actual dialect diagnostic.

## Question

What diagnostic noun and choice list do namespace NOPE and namespace e produce?

## Conclusion

C8.4/8.5 distinguish bad option NOPE from ambiguous option e; C8.6–9.1 use unknown or ambiguous subcommand for both. Jim distinguishes namespace, unknown command NOPE from namespace, ambiguous command e and has its own choice list. All are caught guest errors with outer process0. These strings do not establish a native name primary, result/options object or successful worker dispatch.

## Scope

Selected NOPE,e from24 actual caught shell controls across six engines. The receipt preserves source SHA, executable SHA, process/guest streams and60-second budgets; original script/presenter bytes and file-versus-stdin argument vectors are not retained. errors.tsv is an18-row message projection, checked against exact embedded outputs. available.tsv records the release-specific surfaced choices. No raw zero/opaque object, callback, primary, return-options or BIG-IP measurement.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Shell source; original channel/presenter unavailable. Version: tcl8.4 8.4; full launched patchlevel not independently queried by this capture.

Exact reported output:

```text
1 {bad option "NOPE": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which}
1 {ambiguous option "e": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which}
```

Outer status0 and empty stderr are distinct from each caught guest code1.

### tcl8.5

Status: `observed`. Channel: Shell source; original channel/presenter unavailable. Version: tcl8.5 8.5; full launched patchlevel not independently queried by this capture.

Exact reported output:

```text
1 {bad option "NOPE": must be children, code, current, delete, ensemble, eval, exists, export, forget, import, inscope, origin, parent, path, qualifiers, tail, unknown, upvar, or which}
1 {ambiguous option "e": must be children, code, current, delete, ensemble, eval, exists, export, forget, import, inscope, origin, parent, path, qualifiers, tail, unknown, upvar, or which}
```

Outer status0 and empty stderr are distinct from each caught guest code1.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Shell source; original channel/presenter unavailable. Version: tcl8.6 8.6; full launched patchlevel not independently queried by this capture; tcl9.0 9.0; full launched patchlevel not independently queried by this capture; tcl9.1 9.1; full launched patchlevel not independently queried by this capture.

Exact reported output:

```text
1 {unknown or ambiguous subcommand "NOPE": must be children, code, current, delete, ensemble, eval, exists, export, forget, import, inscope, origin, parent, path, qualifiers, tail, unknown, upvar, or which}
1 {unknown or ambiguous subcommand "e": must be children, code, current, delete, ensemble, eval, exists, export, forget, import, inscope, origin, parent, path, qualifiers, tail, unknown, upvar, or which}
```

Outer status0 and empty stderr are distinct from each caught guest code1.

### jim

Status: `observed`. Channel: Shell source; original channel/presenter unavailable. Version: jim jim; full launched patchlevel not independently queried by this capture.

Exact reported output:

```text
1 {namespace, unknown command "NOPE": should be canonical, code, current, delete, ensemble, eval, export, import, inscope, origin, parent, qualifiers, tail, upvar, which}
1 {namespace, ambiguous command "e": should be canonical, code, current, delete, ensemble, eval, export, import, inscope, origin, parent, qualifiers, tail, upvar, which}
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
