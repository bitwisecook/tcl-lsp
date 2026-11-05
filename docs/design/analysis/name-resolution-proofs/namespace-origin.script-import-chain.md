# naming.namespace-origin.script-import-chain

Kind: `native-observation`

## Problem statement

A global p and an imported src::p share a tail, while a two-step import chain and redundant namespace separators can obscure which declaration namespace origin reports. A displayed tail cannot identify canonical import origin.

## Question

What result names does namespace origin report for root p, source p, a destination import of a re-exported import, and redundant-separator source spelling?

## Conclusion

All five C results are ::p ::src::p ::src::p ::src::p, with guest code0. The two-hop import reports the source declaration, while root p remains distinct. This script result does not establish native command-name cache residency, captured argument objects or an editor edit owner.

## Scope

One exact original ASCII script with four namespace origin queries per C release. The retained presenter catches the original result and hex-encodes it. Separate full source hex/result window, exact original shell digest/process status are retained. The observer wrapper text/channel and complete build/version receipt are unavailable; no physical object-header field is sampled. Jim/BIG-IP not tested.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original ASCII script; catch/count-result hex presenter described, exact wrapper and stdin/file channel unrecorded. Version: tcl8.4 8.4.20 (capture association; launched patchlevel unqueried); tcl8.5 8.5.19 (capture association; launched patchlevel unqueried); tcl8.6 8.6.18 (capture association; launched patchlevel unqueried); tcl9.0 9.0.4 (capture association; launched patchlevel unqueried); tcl9.1 9.1.0 (capture association; launched patchlevel unqueried).

Original presenter stdout:

```text
0	3a3a70203a3a7372633a3a70203a3a7372633a3a70203a3a7372633a3a70
```
Decoded result: ::p ::src::p ::src::p ::src::p.

### jim, bigip

Status: `not-tested`.

No original import-chain namespace-origin script capture for this provider is attached.

## Consumer bindings

- [runtime/rust/src/cmd_namespace.rs](../../../../runtime/rust/src/cmd_namespace.rs), `cmd_namespace::tests::original_namespace_origin_matches_five_native_import_chains` (linked): Compares the exact original payload guest code/result bytes for the five retained import chains; no physical command cache/header observation is claimed.

## Evidence

- [runtime/rust/tests/data/native_namespace_origin/provenance.json](../../../../runtime/rust/tests/data/native_namespace_origin/provenance.json)
- [runtime/rust/tests/data/native_namespace_origin/windows.tsv](../../../../runtime/rust/tests/data/native_namespace_origin/windows.tsv)
