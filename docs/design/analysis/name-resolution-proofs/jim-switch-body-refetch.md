# naming.jim-switch-body-refetch

Kind: `native-observation`

## Problem statement

A switch dispatcher may retain an old body object yet fetch a distinct new body after callback work. Keeping the old object alive does not prove it is still the selected execution body.

## Question

Can a Jim switch callback refetch a body after the original retained body changes representation or callback completion?

## Conclusion

The twelve retained Jim rows record old-body lease and refetched-body distinctions on success, guest Error and noninteger OK. The original refetch probe source is absent, so the immutable capture is retained with this replay limitation; it supplies no new callback or body execution owner.

## Scope

Pinned Jim original object rows/refetch.tsv and build/source digests in refetch-manifest.json; original source hash is retained but source bytes are not recovered.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### jim

Status: `observed`. Version: not recorded by the refetch observer. Build: probe SHA-256 889374c48febda50eee65d5ec91dd04f447b84b4a9e89773da251554d1019e11; linked libjim.a SHA-256 75c74eef0c9d1ff2ff8972e6339678564a3439d6332f66032eb4db79004937ff. Channel: original host object probe; original source bytes unavailable. Dialect: Jim Tcl.

The twelve retained rows include three BEFORE/AFTER_SHIMMER/AFTER_SWITCH/RESULT groups. The old body remains live under a host lease while a distinct refetched object is selected on success (MATCH), guest Error (FAILED) and noninteger OK (empty result). These are retained object-window observations; the absent original probe source prevents independent replay and does not establish arbitrary callback semantics.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Exact evidence

- `receipt-0` (observation): [rust/tcl-cmd-core/tests/data/native_jim_switch/refetch-manifest.json](../../../../rust/tcl-cmd-core/tests/data/native_jim_switch/refetch-manifest.json). SHA-256 `df559cb3ee9f02165f24e4a325faf32b4b0a4d60f0e25db265a158f557ac767e`. Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-8` (observation): [rust/tcl-cmd-core/tests/data/native_jim_switch/refetch.tsv](../../../../rust/tcl-cmd-core/tests/data/native_jim_switch/refetch.tsv). SHA-256 `beb0bb0b0f9115f5e12aacee61efc0384080fe8bf4e0a9b6b0951a3ada3012ed`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original producer code/inputs and captured invocation metadata are pinned above. Restore the exact named provider build and run the original probe with its matching public/private headers and library, preserving its observer order and the60-second/two-slot limits. Recorded absolute executable/output paths are metadata, not usable current paths. Original host probes absent from a corpus remain unreplayable; no source is reconstructed from a digest. Maintained source-only replay instructions are supplied separately where available.
