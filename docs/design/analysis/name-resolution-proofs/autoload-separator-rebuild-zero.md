# naming.autoload.separator-rebuild-zero

Kind: `native-observation`

## Problem statement

Raw-zero and modified-zero original objects have different resident bytes. A separator-matching library transformation may return equal candidate bytes even though it does not establish equality of the original objects or general command-name keys.

## Question

Do a-raw-zero::b and a-C080::b yield the same ordered candidate bytes in actual auto_qualify under ::n?

## Conclusion

The five C providers each return the same two ordered candidate keys for both originals: 3a3a6e3a3a61c0803a3a62, then 3a3a61c0803a3a62. Equality of these transformed library outputs does not establish raw-zero versus modified-zero identity at another command, variable or dictionary comparator.

## Scope

Original counted Tcl_NewStringObj names are passed with the separate ASCII ::n namespace to auto_qualify through Tcl_EvalObjv(TCL_EVAL_DIRECT). The retained public C observer reads counted result-list members, not source UTF encoding or displayed names. Each selected release has a successful compile/run association, source/header/static-library/observer/init.tcl digests. Full runtime patchlevel, compiler flags/version, initialisation argument vector and separate raw stderr are unrecorded. No Jim, BIG-IP, loader invocation or command allocation is measured.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (recorded release association; launched patchlevel unqueried). Build: Header SHA256 824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf; static library SHA256 d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; observer binary SHA256 d1924c4ea8413e92468aab28075bdd1caf4da8f2de815c9d4dc00049738cac57; init.tcl SHA256 a7f4388d1989f005872a47ca5d698aa06680aca21407462301972c5a6048d92b; compile/run statuses0. Compiler/configure details are unrecorded.. Channel: Public original counted Tcl_NewStringObj argv into actual auto_qualify; counted original result-list members.. Dialect: C Tcl.

Selected original ordinal rows:

```text
1 code0 3a3a6e3a3a61c0803a3a62 3a3a61c0803a3a62
4 code0 3a3a6e3a3a61c0803a3a62 3a3a61c0803a3a62
```
Each returned token is the counted resident-byte hex of a reached result-list member. No installed-command or native execution outcome is inferred.

### tcl8.5

Status: `observed`. Version: 8.5.19 (recorded release association; launched patchlevel unqueried). Build: Header SHA256 c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5; static library SHA256 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; observer binary SHA256 916e1a6758acc9ae2113eb62a6a6bf37324a2a359957a395c7a07b5445ef81b5; init.tcl SHA256 21a5aad2ed6d69e15c032be72da55dcca8b56580c869e863d87caf2848e5c2b1; compile/run statuses0. Compiler/configure details are unrecorded.. Channel: Public original counted Tcl_NewStringObj argv into actual auto_qualify; counted original result-list members.. Dialect: C Tcl.

Selected original ordinal rows:

```text
1 code0 3a3a6e3a3a61c0803a3a62 3a3a61c0803a3a62
4 code0 3a3a6e3a3a61c0803a3a62 3a3a61c0803a3a62
```
Each returned token is the counted resident-byte hex of a reached result-list member. No installed-command or native execution outcome is inferred.

### tcl8.6

Status: `observed`. Version: 8.6.18 (recorded release association; launched patchlevel unqueried). Build: Header SHA256 aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245; static library SHA256 a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; observer binary SHA256 8cb139256495a2de1cede70d7aebaa848f4d3d857ef27dab202417133e82e9bd; init.tcl SHA256 513985a7db39509e30f1bec641284dff77db22aa44fe9a0e6191eb63dcc28f35; compile/run statuses0. Compiler/configure details are unrecorded.. Channel: Public original counted Tcl_NewStringObj argv into actual auto_qualify; counted original result-list members.. Dialect: C Tcl.

Selected original ordinal rows:

```text
1 code0 3a3a6e3a3a61c0803a3a62 3a3a61c0803a3a62
4 code0 3a3a6e3a3a61c0803a3a62 3a3a61c0803a3a62
```
Each returned token is the counted resident-byte hex of a reached result-list member. No installed-command or native execution outcome is inferred.

### tcl9.0

Status: `observed`. Version: 9.0.4 (recorded release association; launched patchlevel unqueried). Build: Header SHA256 eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a; static library SHA256 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; observer binary SHA256 a7c5ad8f920cb0ba5785ef5342a37424e66c75938be38ebfa9bd08d8daa0f7fe; init.tcl SHA256 8c5907793b2b8aa05e8d8cf39c0cb7f033c01ca1b8be683e7159774d1a01357a; compile/run statuses0. Compiler/configure details are unrecorded.. Channel: Public original counted Tcl_NewStringObj argv into actual auto_qualify; counted original result-list members.. Dialect: C Tcl.

Selected original ordinal rows:

```text
1 code0 3a3a6e3a3a61c0803a3a62 3a3a61c0803a3a62
4 code0 3a3a6e3a3a61c0803a3a62 3a3a61c0803a3a62
```
Each returned token is the counted resident-byte hex of a reached result-list member. No installed-command or native execution outcome is inferred.

### tcl9.1

Status: `observed`. Version: 9.1.0 (recorded release association; launched patchlevel unqueried). Build: Header SHA256 30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950; static library SHA256 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; observer binary SHA256 1a19f25e3534ff22214153adfc3e382ee78cee6facc35e500b89d6bc2e116442; init.tcl SHA256 636818d3e4071d28ffb610c86d895cb4f0cff57ddf7d8523dabf1feed821ca9f; compile/run statuses0. Compiler/configure details are unrecorded.. Channel: Public original counted Tcl_NewStringObj argv into actual auto_qualify; counted original result-list members.. Dialect: C Tcl.

Selected original ordinal rows:

```text
1 code0 3a3a6e3a3a61c0803a3a62 3a3a61c0803a3a62
4 code0 3a3a6e3a3a61c0803a3a62 3a3a61c0803a3a62
```
Each returned token is the counted resident-byte hex of a reached result-list member. No installed-command or native execution outcome is inferred.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No Jim auto_qualify original-object capture is attached to this C-library question; a Rust recipe refusal is an independent implementation boundary.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No appliance auto_qualify capture is attached; C library outputs do not establish BIG-IP loader or event behaviour.

## Exact evidence

- `probe` (input): [rust/tcl-syntax/tests/data/native_autoload_keys/probe.c](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/probe.c). SHA-256 `85208125b17006c35561364e1ea7a8736353bc36731f48480d65777a5d9a0ab5`. Exact counted original input and reached auto_qualify result-list observer.
- `receipt` (provider): [rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json). SHA-256 `7cc2753d7e037243b5d94d610fa501a1ea4be8ad63c5d68bdf23c5870b55f555`. Five recorded source/build/init-library associations; unrecorded fields are not inferred.
- `provider-tcl8.4` (provider): [rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json). SHA-256 `7cc2753d7e037243b5d94d610fa501a1ea4be8ad63c5d68bdf23c5870b55f555`. JSON pointer `/engines/0`. Exact provider build and successful process association.
- `rows-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_autoload_keys/8.4.20.tsv](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/8.4.20.tsv). SHA-256 `66b41df6ac0ee884a59e135b9dedeb1d9a8b5602e9a9c9a261c562adb5a5f58c`. Exact counted returned keys; selected ordinal rows are named explicitly in the answer.
- `provider-tcl8.5` (provider): [rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json). SHA-256 `7cc2753d7e037243b5d94d610fa501a1ea4be8ad63c5d68bdf23c5870b55f555`. JSON pointer `/engines/1`. Exact provider build and successful process association.
- `rows-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_autoload_keys/8.5.19.tsv](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/8.5.19.tsv). SHA-256 `66b41df6ac0ee884a59e135b9dedeb1d9a8b5602e9a9c9a261c562adb5a5f58c`. Exact counted returned keys; selected ordinal rows are named explicitly in the answer.
- `provider-tcl8.6` (provider): [rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json). SHA-256 `7cc2753d7e037243b5d94d610fa501a1ea4be8ad63c5d68bdf23c5870b55f555`. JSON pointer `/engines/2`. Exact provider build and successful process association.
- `rows-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_autoload_keys/8.6.18.tsv](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/8.6.18.tsv). SHA-256 `66b41df6ac0ee884a59e135b9dedeb1d9a8b5602e9a9c9a261c562adb5a5f58c`. Exact counted returned keys; selected ordinal rows are named explicitly in the answer.
- `provider-tcl9.0` (provider): [rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json). SHA-256 `7cc2753d7e037243b5d94d610fa501a1ea4be8ad63c5d68bdf23c5870b55f555`. JSON pointer `/engines/3`. Exact provider build and successful process association.
- `rows-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_autoload_keys/9.0.4.tsv](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/9.0.4.tsv). SHA-256 `66b41df6ac0ee884a59e135b9dedeb1d9a8b5602e9a9c9a261c562adb5a5f58c`. Exact counted returned keys; selected ordinal rows are named explicitly in the answer.
- `provider-tcl9.1` (provider): [rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json). SHA-256 `7cc2753d7e037243b5d94d610fa501a1ea4be8ad63c5d68bdf23c5870b55f555`. JSON pointer `/engines/4`. Exact provider build and successful process association.
- `rows-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_autoload_keys/9.1.0.tsv](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/9.1.0.tsv). SHA-256 `66b41df6ac0ee884a59e135b9dedeb1d9a8b5602e9a9c9a261c562adb5a5f58c`. Exact counted returned keys; selected ordinal rows are named explicitly in the answer.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/naming/autoload.rs](../../../../rust/tcl-syntax/src/naming/autoload.rs), `native_autoload_command_candidates`: Selected C resident-byte candidate recipe; this pure inventory supplies no loader or original-object authority.
- [rust/tcl-syntax/src/naming/autoload.rs](../../../../rust/tcl-syntax/src/naming/autoload.rs), `naming::autoload::tests::native_autoload_keys_follow_original_regsub_rebuilding` (linked): Compares all25 actual C key projection rows by selected release, including these original ordinal cases; the Jim refusal and constructed namespace preservation are independent Rust assertions.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact C probe is retained, but the original argv[1] initialisation script and argv[2] library path are not attached as an argument-vector receipt. The recorded init.tcl digest alone does not reconstruct that source. No exact original-bootstrap runner is claimed. A new reconfirmation must independently supply and hash the selected init.tcl/library and initialisation script, compile this unchanged probe against its selected headers/static library, and record actual runtime version, process status and separate raw streams. Existing five-row projections are retained evidence, not a newly executed run or Rust pass.
