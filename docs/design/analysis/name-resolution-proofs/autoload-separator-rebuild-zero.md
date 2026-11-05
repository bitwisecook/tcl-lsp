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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Public original counted Tcl_NewStringObj argv into actual auto_qualify; counted original result-list members. Version: tcl8.4 8.4.20 (recorded release association; launched patchlevel unqueried); tcl8.5 8.5.19 (recorded release association; launched patchlevel unqueried); tcl8.6 8.6.18 (recorded release association; launched patchlevel unqueried); tcl9.0 9.0.4 (recorded release association; launched patchlevel unqueried); tcl9.1 9.1.0 (recorded release association; launched patchlevel unqueried).

Selected original ordinal rows:

```text
1 code0 3a3a6e3a3a61c0803a3a62 3a3a61c0803a3a62
4 code0 3a3a6e3a3a61c0803a3a62 3a3a61c0803a3a62
```
Each returned token is the counted resident-byte hex of a reached result-list member. No installed-command or native execution outcome is inferred.

### jim

Status: `not-tested`.

No Jim auto_qualify original-object capture is attached to this C-library question; a Rust recipe refusal is an independent implementation boundary.

### bigip

Status: `not-tested`.

No appliance auto_qualify capture is attached; C library outputs do not establish BIG-IP loader or event behaviour.

## Consumer bindings

- [rust/tcl-syntax/src/naming/autoload.rs](../../../../rust/tcl-syntax/src/naming/autoload.rs), `native_autoload_command_candidates`: Selected C resident-byte candidate recipe; this pure inventory supplies no loader or original-object authority.
- [rust/tcl-syntax/src/naming/autoload.rs](../../../../rust/tcl-syntax/src/naming/autoload.rs), `naming::autoload::tests::native_autoload_keys_follow_original_regsub_rebuilding` (linked): Compares all25 actual C key projection rows by selected release, including these original ordinal cases; the Jim refusal and constructed namespace preservation are independent Rust assertions.

## Evidence

- [rust/tcl-syntax/tests/data/native_autoload_keys/probe.c](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/probe.c)
- [rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/manifest.json)
- [rust/tcl-syntax/tests/data/native_autoload_keys/8.4.20.tsv](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/8.4.20.tsv)
- [rust/tcl-syntax/tests/data/native_autoload_keys/8.5.19.tsv](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/8.5.19.tsv)
- [rust/tcl-syntax/tests/data/native_autoload_keys/8.6.18.tsv](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/8.6.18.tsv)
- [rust/tcl-syntax/tests/data/native_autoload_keys/9.0.4.tsv](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/9.0.4.tsv)
- [rust/tcl-syntax/tests/data/native_autoload_keys/9.1.0.tsv](../../../../rust/tcl-syntax/tests/data/native_autoload_keys/9.1.0.tsv)
