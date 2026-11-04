# naming.autoload.root-prefix-rebuild-zero

Kind: `native-observation`

## Problem statement

The rooted original ::a-raw-zero-x has one separator. Keeping the root prefix or returning an unrooted reconstructed key changes the autoload inventory even before a loader can run.

## Question

What candidate bytes does actual auto_qualify return for counted ::a-raw-zero-x under ::n?

## Conclusion

All five C captures return the single unrooted candidate 61c08078. The original root prefix is absent and the zero is represented as C080 in this returned key. The observation proves neither a matching auto_index entry nor installed command ownership.

## Scope

Original counted Tcl_NewStringObj names are passed with the separate ASCII ::n namespace to auto_qualify through Tcl_EvalObjv(TCL_EVAL_DIRECT). The retained public C observer reads counted result-list members, not source UTF encoding or displayed names. Each selected release has a successful compile/run association, source/header/static-library/observer/init.tcl digests. Full runtime patchlevel, compiler flags/version, initialisation argument vector and separate raw stderr are unrecorded. No Jim, BIG-IP, loader invocation or command allocation is measured.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (recorded release association; launched patchlevel unqueried). Build: Header; static library; observer binary; init.tcl; compile/run statuses0. Compiler/configure details are unrecorded.. Channel: Public original counted Tcl_NewStringObj argv into actual auto_qualify; counted original result-list members.. Dialect: C Tcl.

Selected original ordinal rows:

```text
2 code0 61c08078
```
Each returned token is the counted resident-byte hex of a reached result-list member. No installed-command or native execution outcome is inferred.

### tcl8.5

Status: `observed`. Version: 8.5.19 (recorded release association; launched patchlevel unqueried). Build: Header; static library; observer binary; init.tcl; compile/run statuses0. Compiler/configure details are unrecorded.. Channel: Public original counted Tcl_NewStringObj argv into actual auto_qualify; counted original result-list members.. Dialect: C Tcl.

Selected original ordinal rows:

```text
2 code0 61c08078
```
Each returned token is the counted resident-byte hex of a reached result-list member. No installed-command or native execution outcome is inferred.

### tcl8.6

Status: `observed`. Version: 8.6.18 (recorded release association; launched patchlevel unqueried). Build: Header; static library; observer binary; init.tcl; compile/run statuses0. Compiler/configure details are unrecorded.. Channel: Public original counted Tcl_NewStringObj argv into actual auto_qualify; counted original result-list members.. Dialect: C Tcl.

Selected original ordinal rows:

```text
2 code0 61c08078
```
Each returned token is the counted resident-byte hex of a reached result-list member. No installed-command or native execution outcome is inferred.

### tcl9.0

Status: `observed`. Version: 9.0.4 (recorded release association; launched patchlevel unqueried). Build: Header; static library; observer binary; init.tcl; compile/run statuses0. Compiler/configure details are unrecorded.. Channel: Public original counted Tcl_NewStringObj argv into actual auto_qualify; counted original result-list members.. Dialect: C Tcl.

Selected original ordinal rows:

```text
2 code0 61c08078
```
Each returned token is the counted resident-byte hex of a reached result-list member. No installed-command or native execution outcome is inferred.

### tcl9.1

Status: `observed`. Version: 9.1.0 (recorded release association; launched patchlevel unqueried). Build: Header; static library; observer binary; init.tcl; compile/run statuses0. Compiler/configure details are unrecorded.. Channel: Public original counted Tcl_NewStringObj argv into actual auto_qualify; counted original result-list members.. Dialect: C Tcl.

Selected original ordinal rows:

```text
2 code0 61c08078
```
Each returned token is the counted resident-byte hex of a reached result-list member. No installed-command or native execution outcome is inferred.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No Jim auto_qualify original-object capture is attached to this C-library question; a Rust recipe refusal is an independent implementation boundary.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No appliance auto_qualify capture is attached; C library outputs do not establish BIG-IP loader or event behaviour.

## Historical references

- `probe` (input): [rust/tcl-syntax/tests/data/native_autoload_keys/probe.c](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/probe.c). Exact counted original input and reached auto_qualify result-list observer.
- `receipt` (provider): [rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json). Five recorded source/build/init-library associations; unrecorded fields are not inferred.
- `provider-tcl8.4` (provider): [rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json). JSON pointer `/engines/0`. Exact provider build and successful process association.
- `rows-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_autoload_keys/8.4.20.tsv](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/8.4.20.tsv). Exact counted returned keys; selected ordinal rows are named explicitly in the answer.
- `provider-tcl8.5` (provider): [rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json). JSON pointer `/engines/1`. Exact provider build and successful process association.
- `rows-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_autoload_keys/8.5.19.tsv](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/8.5.19.tsv). Exact counted returned keys; selected ordinal rows are named explicitly in the answer.
- `provider-tcl8.6` (provider): [rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json). JSON pointer `/engines/2`. Exact provider build and successful process association.
- `rows-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_autoload_keys/8.6.18.tsv](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/8.6.18.tsv). Exact counted returned keys; selected ordinal rows are named explicitly in the answer.
- `provider-tcl9.0` (provider): [rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json). JSON pointer `/engines/3`. Exact provider build and successful process association.
- `rows-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_autoload_keys/9.0.4.tsv](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/9.0.4.tsv). Exact counted returned keys; selected ordinal rows are named explicitly in the answer.
- `provider-tcl9.1` (provider): [rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json). JSON pointer `/engines/4`. Exact provider build and successful process association.
- `rows-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_autoload_keys/9.1.0.tsv](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/9.1.0.tsv). Exact counted returned keys; selected ordinal rows are named explicitly in the answer.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/naming/autoload.rs](../../../../rust/tcl-syntax/src/naming/autoload.rs), `native_autoload_command_candidates`: Selected C resident-byte candidate recipe; this pure inventory supplies no loader or original-object authority.
- [rust/tcl-syntax/src/naming/autoload.rs](../../../../rust/tcl-syntax/src/naming/autoload.rs), `naming::autoload::tests::native_autoload_keys_follow_original_regsub_rebuilding` (linked): Compares all25 actual C key projection rows by selected release, including these original ordinal cases; the Jim refusal and constructed namespace preservation are independent Rust assertions.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact C probe is retained, but the original argv[1] initialisation script and argv[2] library path are not attached as an argument-vector receipt. The recorded init.tcl digest alone does not reconstruct that source. No exact original-bootstrap runner is claimed. A new reconfirmation must independently supply and hash the selected init.tcl/library and initialisation script, compile this unchanged probe against its selected headers/static library, and record actual runtime version, process status and separate raw streams. Existing five-row projections are retained evidence, not a newly executed run or Rust pass.
