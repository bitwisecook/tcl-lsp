# naming.namespace-native.opaque-child-enumeration

Kind: `native-observation`

## Problem statement

Opaque parent/child namespace names may be lost if native enumeration is reconstructed through host Unicode or displayed-name parsing.

## Question

Does original namespace children enumerate the raw-FF parent's raw-FD child with exact returned resident bytes?

## Conclusion

All five C providers return code0 and list-primary result bytes 3a3a726177ff3a3a6368696c64fd. The original opaque child spelling survives this native enumeration; no namespace allocation or pattern-matching rule beyond the observed child is inferred.

## Scope

Five C builds each create ::raw-FF and its ::child-FD namespace via native APIs. Original Tcl_NewStringObj parent/pattern argv reaches namespace children with flags0. Result primary is sampled before the counted string getter. Complete original four-row streams are reconstructed only by exact original log SHA256 agreement. No native refcount, return-option, allocator, Jim or BIG-IP observation.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (capture association; launched patchlevel unqueried). Build: Header SHA256 824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf; library SHA256 d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; observer SHA256 6d7a18408c72664ab371f1ba75c92f16b7333d0196b5d2d46152f0610fc1bf98; compile/process0. Compiler version/configure flags unrecorded.. Channel: Native namespace creation and original object-vector children query with flags0; original result primary followed by counted result getter.. Dialect: C Tcl.

Selected original label/code/result-primary/result-hex rows:

```text
enumerate|0|list|3a3a726177ff3a3a6368696c64fd
```
Whole original stream bytes agree with the original recorded log digest; no new native run is claimed.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association; launched patchlevel unqueried). Build: Header SHA256 c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5; library SHA256 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; observer SHA256 86d55c4084ffc39abb037b25a792349e65f0009370a2b8d58639a95d984f4bc7; compile/process0. Compiler version/configure flags unrecorded.. Channel: Native namespace creation and original object-vector children query with flags0; original result primary followed by counted result getter.. Dialect: C Tcl.

Selected original label/code/result-primary/result-hex rows:

```text
enumerate|0|list|3a3a726177ff3a3a6368696c64fd
```
Whole original stream bytes agree with the original recorded log digest; no new native run is claimed.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association; launched patchlevel unqueried). Build: Header SHA256 aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245; library SHA256 a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; observer SHA256 846d5e517e64dd3e1427f6ad5d903f836859ee32e5b1ed78e6179e44d2b2d76c; compile/process0. Compiler version/configure flags unrecorded.. Channel: Native namespace creation and original object-vector children query with flags0; original result primary followed by counted result getter.. Dialect: C Tcl.

Selected original label/code/result-primary/result-hex rows:

```text
enumerate|0|list|3a3a726177ff3a3a6368696c64fd
```
Whole original stream bytes agree with the original recorded log digest; no new native run is claimed.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association; launched patchlevel unqueried). Build: Header SHA256 eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a; library SHA256 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; observer SHA256 d1edfd8cb6830c739fbd64c4a65dbddd11214c0f8115b8bf3e3a7aa91b1b59de; compile/process0. Compiler version/configure flags unrecorded.. Channel: Native namespace creation and original object-vector children query with flags0; original result primary followed by counted result getter.. Dialect: C Tcl.

Selected original label/code/result-primary/result-hex rows:

```text
enumerate|0|list|3a3a726177ff3a3a6368696c64fd
```
Whole original stream bytes agree with the original recorded log digest; no new native run is claimed.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association; launched patchlevel unqueried). Build: Header SHA256 30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950; library SHA256 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; observer SHA256 2b75e84772094e42f761bcfa60bf52605ca5e42712e79d1c8169a34699503f49; compile/process0. Compiler version/configure flags unrecorded.. Channel: Native namespace creation and original object-vector children query with flags0; original result primary followed by counted result getter.. Dialect: C Tcl.

Selected original label/code/result-primary/result-hex rows:

```text
enumerate|0|list|3a3a726177ff3a3a6368696c64fd
```
Whole original stream bytes agree with the original recorded log digest; no new native run is claimed.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No original opaque-child query capture for this provider is attached.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No original opaque-child query capture for this provider is attached.

## Exact evidence

- `probe` (input): [rust/tcl-syntax/tests/data/native_namespace_name/opaque_children.c](../../../../rust/tcl-syntax/tests/data/native_namespace_name/opaque_children.c). SHA-256 `2294889fc0b754ec4d991193ed4cb6470e541d78dbff7646678a8f8def1d00c4`. Exact original namespace creation, pattern argv and result-primary observer.
- `receipt` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json). SHA-256 `41aaaa3f89c8a529ca5f3d356d2805c3c5ba74c7672afe7c7c89c3b656930cc9`. Original selected build/launch/stream associations.
- `projection` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/opaque_children.txt](../../../../rust/tcl-syntax/tests/data/native_namespace_name/opaque_children.txt). SHA-256 `cb6f8dbbd69912f1744d859c75a8dd03502ef47c51ba5f71331855b995572b4a`. Exact20 release/label/result-primary/value projection rows.
- `stream-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-8.4.20.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-8.4.20.stdout). SHA-256 `8e2cc6a190a142b140ff0fa2e8b16380ae08150a7fd7eba44cdf16fc9559522b`. Complete original stdout bytes matching the original recorded whole-stream digest.
- `provider-tcl8.4` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json). SHA-256 `41aaaa3f89c8a529ca5f3d356d2805c3c5ba74c7672afe7c7c89c3b656930cc9`. JSON pointer `/runs/0`. This exact original header/library/observer/build/process association.
- `stream-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-8.5.19.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-8.5.19.stdout). SHA-256 `767b30056126cb1171a27b66d1219b2bbdf65b08573ae799d046ea0aa5ae1a99`. Complete original stdout bytes matching the original recorded whole-stream digest.
- `provider-tcl8.5` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json). SHA-256 `41aaaa3f89c8a529ca5f3d356d2805c3c5ba74c7672afe7c7c89c3b656930cc9`. JSON pointer `/runs/1`. This exact original header/library/observer/build/process association.
- `stream-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-8.6.18.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-8.6.18.stdout). SHA-256 `8e2cc6a190a142b140ff0fa2e8b16380ae08150a7fd7eba44cdf16fc9559522b`. Complete original stdout bytes matching the original recorded whole-stream digest.
- `provider-tcl8.6` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json). SHA-256 `41aaaa3f89c8a529ca5f3d356d2805c3c5ba74c7672afe7c7c89c3b656930cc9`. JSON pointer `/runs/2`. This exact original header/library/observer/build/process association.
- `stream-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-9.0.4.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-9.0.4.stdout). SHA-256 `8e2cc6a190a142b140ff0fa2e8b16380ae08150a7fd7eba44cdf16fc9559522b`. Complete original stdout bytes matching the original recorded whole-stream digest.
- `provider-tcl9.0` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json). SHA-256 `41aaaa3f89c8a529ca5f3d356d2805c3c5ba74c7672afe7c7c89c3b656930cc9`. JSON pointer `/runs/3`. This exact original header/library/observer/build/process association.
- `stream-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-9.1.0.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-9.1.0.stdout). SHA-256 `8e2cc6a190a142b140ff0fa2e8b16380ae08150a7fd7eba44cdf16fc9559522b`. Complete original stdout bytes matching the original recorded whole-stream digest.
- `provider-tcl9.1` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json). SHA-256 `41aaaa3f89c8a529ca5f3d356d2805c3c5ba74c7672afe7c7c89c3b656930cc9`. JSON pointer `/runs/4`. This exact original header/library/observer/build/process association.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-cmd-core/src/namespace.rs](../../../../rust/tcl-cmd-core/src/namespace.rs), `children_original`: Select native original parent/pattern query and produce the observed result primary through the shared backend.
- [runtime/rust/src/interp/native_namespace_names/tests.rs](../../../../runtime/rust/src/interp/native_namespace_names/tests.rs), `interp::native_namespace_names::tests::opaque_children_match_20_native_query_and_primary_windows` (linked): Compares all20 actual query-primary-value projections, including these exact original pattern or enumeration rows.
- [rust/tcl-vm/src/interp/native_namespace_names.rs](../../../../rust/tcl-vm/src/interp/native_namespace_names.rs), `interp::native_namespace_names::tests::opaque_children_match_20_native_query_and_primary_windows` (linked): Compares all20 actual query-primary-value projections, including these exact original pattern or enumeration rows.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact observer source and complete original streams are attached. A fresh reconfirmation must compile against explicitly identified recorded public/private headers and static libraries and preserve original flags0 argv, compile/process statuses and separate streams. Reconstructing retained stream bytes by whole original digest agreement supplies no new provider run or implementation source explanation.
