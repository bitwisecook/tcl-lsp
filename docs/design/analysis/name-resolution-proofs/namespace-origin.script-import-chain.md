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

### tcl8.4

Status: `observed`. Version: 8.4.20 (capture association; launched patchlevel unqueried). Build: Original shell SHA256 551bef3a9b51f256f4ad9dec06bddc42ceab872a72528e83676d1d7df715b9d4; process0. Compiler/configure/source-build and separate stderr unrecorded.. Channel: Original ASCII script; catch/count-result hex presenter described, exact wrapper and stdin/file channel unrecorded.. Dialect: C Tcl.

Original presenter stdout:

```text
0	3a3a70203a3a7372633a3a70203a3a7372633a3a70203a3a7372633a3a70
```
Decoded result: ::p ::src::p ::src::p ::src::p.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association; launched patchlevel unqueried). Build: Original shell SHA256 7a0fb4aa272a45662fb883295d0037a8c18e3e32c635736af8314fb835981935; process0. Compiler/configure/source-build and separate stderr unrecorded.. Channel: Original ASCII script; catch/count-result hex presenter described, exact wrapper and stdin/file channel unrecorded.. Dialect: C Tcl.

Original presenter stdout:

```text
0	3a3a70203a3a7372633a3a70203a3a7372633a3a70203a3a7372633a3a70
```
Decoded result: ::p ::src::p ::src::p ::src::p.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association; launched patchlevel unqueried). Build: Original shell SHA256 0cc67113840d03c15c15d88188c16b8b019aa2a8c11ca6980f32723fbedacde0; process0. Compiler/configure/source-build and separate stderr unrecorded.. Channel: Original ASCII script; catch/count-result hex presenter described, exact wrapper and stdin/file channel unrecorded.. Dialect: C Tcl.

Original presenter stdout:

```text
0	3a3a70203a3a7372633a3a70203a3a7372633a3a70203a3a7372633a3a70
```
Decoded result: ::p ::src::p ::src::p ::src::p.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association; launched patchlevel unqueried). Build: Original shell SHA256 cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18; process0. Compiler/configure/source-build and separate stderr unrecorded.. Channel: Original ASCII script; catch/count-result hex presenter described, exact wrapper and stdin/file channel unrecorded.. Dialect: C Tcl.

Original presenter stdout:

```text
0	3a3a70203a3a7372633a3a70203a3a7372633a3a70203a3a7372633a3a70
```
Decoded result: ::p ::src::p ::src::p ::src::p.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association; launched patchlevel unqueried). Build: Original shell SHA256 d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c; process0. Compiler/configure/source-build and separate stderr unrecorded.. Channel: Original ASCII script; catch/count-result hex presenter described, exact wrapper and stdin/file channel unrecorded.. Dialect: C Tcl.

Original presenter stdout:

```text
0	3a3a70203a3a7372633a3a70203a3a7372633a3a70203a3a7372633a3a70
```
Decoded result: ::p ::src::p ::src::p ::src::p.

### jim

Status: `not-tested`. Version: not measured. Build: not measured. Channel: not tested. Dialect: Jim Tcl.

No original import-chain namespace-origin script capture for this provider is attached.

### bigip

Status: `not-tested`. Version: not measured. Build: not measured. Channel: not tested. Dialect: F5 iRules.

No original import-chain namespace-origin script capture for this provider is attached.

## Exact evidence

- `payload` (input): [runtime/rust/tests/data/native_namespace_origin/provenance.json](../../../../runtime/rust/tests/data/native_namespace_origin/provenance.json). SHA-256 `867b10f594c09c68e990f9ac08c4c89abdd5bee7bd917c502bc1d69ff1ab4121`. JSON pointer `/source`. Exact original source payload and described presenter.
- `windows` (observation): [runtime/rust/tests/data/native_namespace_origin/windows.tsv](../../../../runtime/rust/tests/data/native_namespace_origin/windows.tsv). SHA-256 `364d2ee3d3513627873268c4706e9850d74ca4fd7f04ea36f1d62622563f193d`. Original payload hex, guest completion and counted result hex.
- `receipt` (provider): [runtime/rust/tests/data/native_namespace_origin/provenance.json](../../../../runtime/rust/tests/data/native_namespace_origin/provenance.json). SHA-256 `867b10f594c09c68e990f9ac08c4c89abdd5bee7bd917c502bc1d69ff1ab4121`. Independent original shell/source/result associations.
- `provider-tcl8.4` (observation): [runtime/rust/tests/data/native_namespace_origin/provenance.json](../../../../runtime/rust/tests/data/native_namespace_origin/provenance.json). SHA-256 `867b10f594c09c68e990f9ac08c4c89abdd5bee7bd917c502bc1d69ff1ab4121`. JSON pointer `/runs/0`. Exact original caught script result and launched shell association.
- `provider-tcl8.5` (observation): [runtime/rust/tests/data/native_namespace_origin/provenance.json](../../../../runtime/rust/tests/data/native_namespace_origin/provenance.json). SHA-256 `867b10f594c09c68e990f9ac08c4c89abdd5bee7bd917c502bc1d69ff1ab4121`. JSON pointer `/runs/1`. Exact original caught script result and launched shell association.
- `provider-tcl8.6` (observation): [runtime/rust/tests/data/native_namespace_origin/provenance.json](../../../../runtime/rust/tests/data/native_namespace_origin/provenance.json). SHA-256 `867b10f594c09c68e990f9ac08c4c89abdd5bee7bd917c502bc1d69ff1ab4121`. JSON pointer `/runs/2`. Exact original caught script result and launched shell association.
- `provider-tcl9.0` (observation): [runtime/rust/tests/data/native_namespace_origin/provenance.json](../../../../runtime/rust/tests/data/native_namespace_origin/provenance.json). SHA-256 `867b10f594c09c68e990f9ac08c4c89abdd5bee7bd917c502bc1d69ff1ab4121`. JSON pointer `/runs/3`. Exact original caught script result and launched shell association.
- `provider-tcl9.1` (observation): [runtime/rust/tests/data/native_namespace_origin/provenance.json](../../../../runtime/rust/tests/data/native_namespace_origin/provenance.json). SHA-256 `867b10f594c09c68e990f9ac08c4c89abdd5bee7bd917c502bc1d69ff1ab4121`. JSON pointer `/runs/4`. Exact original caught script result and launched shell association.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/cmd_namespace.rs](../../../../runtime/rust/src/cmd_namespace.rs), `cmd_namespace::tests::original_namespace_origin_matches_five_native_import_chains` (linked): Compares the exact original payload guest code/result bytes for the five retained import chains; no physical command cache/header observation is claimed.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original payload bytes are recoverable and hashed against every window/receipt. The catch+binary-scan presenter is described but not retained verbatim; strict whole-stdin replay cannot be claimed from that description. A fresh reconfirmation must identify its new wrapper explicitly and preserve the original payload, selected shell build, guest completion and separate streams.
