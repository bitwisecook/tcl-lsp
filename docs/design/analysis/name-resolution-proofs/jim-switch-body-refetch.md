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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### jim

Status: `observed`. Channel: original host object probe; original source bytes unavailable.

The twelve retained rows include three BEFORE/AFTER_SHIMMER/AFTER_SWITCH/RESULT groups. The old body remains live under a host lease while a distinct refetched object is selected on success (MATCH), guest Error (FAILED) and noninteger OK (empty result). These are retained object-window observations; the absent original probe source prevents independent replay and does not establish arbitrary callback semantics.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-cmd-core/tests/data/native_jim_switch/refetch-manifest.json](../../../../rust/tcl-cmd-core/tests/data/native_jim_switch/refetch-manifest.json)
- [rust/tcl-cmd-core/tests/data/native_jim_switch/refetch.tsv](../../../../rust/tcl-cmd-core/tests/data/native_jim_switch/refetch.tsv)
