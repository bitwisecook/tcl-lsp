# naming.namespace.dispatch-upvar-zero-argument-usage

Kind: `native-observation`

## Problem statement

The exact upvar worker can be absent or installed with release-specific usage grammar. A malformed call must not be treated as evidence that a link was installed.

## Question

Which worker availability and usage result follows namespace upvar with no namespace operand?

## Conclusion

C8.4 reports bad option upvar because that surface is absent. C8.5 reports namespace upvar ns otherVar myVar ?otherVar myVar ...?; C8.6–9.1 report namespace upvar ns ?otherVar myVar ...?; Jim reports namespace upvar ns ?arg ...?. Those reached workers reject the call before any link; process0 records the surrounding catch/reporter only.

## Scope

Selected upvar from24 actual caught shell controls across six engines. The receipt preserves source SHA, executable SHA, process/guest streams and60-second budgets; original script/presenter bytes and file-versus-stdin argument vectors are not retained. errors.tsv is an18-row message projection, checked against exact embedded outputs. available.tsv records the release-specific surfaced choices. No raw zero/opaque object, callback, primary, return-options or BIG-IP measurement.

## Provider answers

### tcl8.4

Status: `unsupported`. Channel: Shell source; original channel/presenter unavailable. Version: tcl8.4 8.4; full launched patchlevel not independently queried by this capture.

Exact reported output:

```text
1 {bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which}
```

Outer status0 and empty stderr are distinct from each caught guest code1.

### tcl8.5

Status: `observed`. Channel: Shell source; original channel/presenter unavailable. Version: tcl8.5 8.5; full launched patchlevel not independently queried by this capture.

Exact reported output:

```text
1 {wrong # args: should be "namespace upvar ns otherVar myVar ?otherVar myVar ...?"}
```

Outer status0 and empty stderr are distinct from each caught guest code1.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Shell source; original channel/presenter unavailable. Version: tcl8.6 8.6; full launched patchlevel not independently queried by this capture; tcl9.0 9.0; full launched patchlevel not independently queried by this capture; tcl9.1 9.1; full launched patchlevel not independently queried by this capture.

Exact reported output:

```text
1 {wrong # args: should be "namespace upvar ns ?otherVar myVar ...?"}
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
- [rust/tcl-registry/src/native_namespace_upvar.rs](../../../../rust/tcl-registry/src/native_namespace_upvar.rs), `NativeNamespaceUpvarProtocol::arguments`: Keeps absent surface and each installed worker/forwarding argument grammar distinct from successful cell installation.
- [rust/tcl-registry/src/native_namespace_upvar.rs](../../../../rust/tcl-registry/src/native_namespace_upvar.rs), `native_namespace_upvar::tests::original_namespace_upvar_grammar_preserves_worker_and_forwarded_frontiers` (linked): Asserts C84 unavailable and zero-argument refusal on every installed protocol; this pure grammar assertion does not construct a native link.

## Evidence

- [rust/tcl-cmd-core/tests/data/native_namespace_dispatch/provenance.json](../../../../rust/tcl-cmd-core/tests/data/native_namespace_dispatch/provenance.json)
- [rust/tcl-cmd-core/tests/data/native_namespace_dispatch/errors.tsv](../../../../rust/tcl-cmd-core/tests/data/native_namespace_dispatch/errors.tsv)
- [rust/tcl-cmd-core/tests/data/native_namespace_dispatch/available.tsv](../../../../rust/tcl-cmd-core/tests/data/native_namespace_dispatch/available.tsv)
