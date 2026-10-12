# naming.variable.explicit-uplevel-level-body-cache-and-frame-restoration

Kind: `native-observation`

## Problem statement

Explicit level objects can acquire a frame cache, contain raw zero suffixes, or fail to parse, while a selected body can fail/parse-error/break or materialise a list string. A final result alone cannot establish the selected frame, cache route, or restoration after failure.

## Question

For fourteen original level/body pairs, which frame/cache/body-string transitions and completion bytes are observed, and is the saved variable frame restored?

## Conclusion

All seventy retained C windows restore the saved frame and produce the listed original level/cache/body/completion results. C84 uses its selected string TclGetFrame route; C85+ use original-object TclObjGetFrame and can retain levelReference caches. This is explicit evaluation, not a native uplevel opcode proof or a Jim/BIG-IP observation.

## Scope

C84.20/C85.19/C86.18/C90.4/C91.0 private interpreter probes: fourteen original objects per release, including raw 00 suffix level objects, root/current/relative/default nonlevel/error selection and error/parse/list/break bodies. Each window uses a fresh interpreter. Returned result bytes and before/after cache/resident-string flags are distinct axes.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: binary_sha256=8b9434ace3947879c3f530f7b77adb8bc54b5502cec9bd9a924af1864649d233; archive_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; internal_header_sha256=f51c8e66c562de424b84a35a04b6b86d9a135ef7d4ca891b35c3b0586bc0ce1a. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

Rows use original ordinal: 0 #0, 1 0, 2 1, 3 bad, 4 +0, 5 99, 6 -1, 7 1.0, 8 counted 1\x00X, 9 counted #0\x00X; 10 error BODY, 11 parse error, 12 pure list script, 13 break.

| ordinal | before type | before string | code | result | after type | after level string | after body string | restored |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 1 | none | 1 | 0 | "1 1" | none | 1 | 1 | 1 |
| 2 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 3 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 4 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 5 | none | 1 | 1 | "bad level \"99\"" | none | 1 | 1 | 1 |
| 6 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 7 | none | 1 | 1 | "expected integer but got \"1.0\"" | none | 1 | 1 | 1 |
| 8 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 9 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 10 | none | 1 | 1 | "BODY" | none | 1 | 1 | 1 |
| 11 | none | 1 | 1 | "missing close-brace" | none | 1 | 1 | 1 |
| 12 | none | 1 | 0 | "VALUE" | none | 1 | 1 | 1 |
| 13 | none | 1 | 1 | "invoked \"break\" outside of a loop" | none | 1 | 1 | 1 |

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: binary_sha256=38b730c3cc2aada9cefb7627b6c184938582923bd6c6e1332a334b526827a60d; archive_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; internal_header_sha256=72078ca207924e61b4fbf637eff263d280a0dbc78e6a5fbb39e2cd980d08a8aa. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

Rows use original ordinal: 0 #0, 1 0, 2 1, 3 bad, 4 +0, 5 99, 6 -1, 7 1.0, 8 counted 1\x00X, 9 counted #0\x00X; 10 error BODY, 11 parse error, 12 pure list script, 13 break.

| ordinal | before type | before string | code | result | after type | after level string | after body string | restored |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | none | 1 | 0 | "0 0" | levelReference | 1 | 1 | 1 |
| 1 | none | 1 | 0 | "1 1" | levelReference | 1 | 1 | 1 |
| 2 | none | 1 | 0 | "0 0" | levelReference | 1 | 1 | 1 |
| 3 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 4 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 5 | none | 1 | 1 | "bad level \"99\"" | levelReference | 1 | 1 | 1 |
| 6 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 7 | none | 1 | 1 | "expected integer but got \"1.0\"" | none | 1 | 1 | 1 |
| 8 | none | 1 | 0 | "0 0" | levelReference | 1 | 1 | 1 |
| 9 | none | 1 | 0 | "0 0" | levelReference | 1 | 1 | 1 |
| 10 | none | 1 | 1 | "BODY" | levelReference | 1 | 1 | 1 |
| 11 | none | 1 | 1 | "missing close-brace" | levelReference | 1 | 1 | 1 |
| 12 | none | 1 | 0 | "VALUE" | levelReference | 1 | 0 | 1 |
| 13 | none | 1 | 1 | "invoked \"break\" outside of a loop" | levelReference | 1 | 1 | 1 |

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: binary_sha256=3b3ca712025bcd47a4f74eb26d907fb33f8b546b9f2ca9633078e4312db43cdb; archive_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; internal_header_sha256=e9e70f6461d7c4b37173173956040b647914157c28425ec859730cd8eaef4d2e. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

Rows use original ordinal: 0 #0, 1 0, 2 1, 3 bad, 4 +0, 5 99, 6 -1, 7 1.0, 8 counted 1\x00X, 9 counted #0\x00X; 10 error BODY, 11 parse error, 12 pure list script, 13 break.

| ordinal | before type | before string | code | result | after type | after level string | after body string | restored |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | none | 1 | 0 | "0 0" | levelReference | 1 | 1 | 1 |
| 1 | none | 1 | 0 | "1 1" | int | 1 | 1 | 1 |
| 2 | none | 1 | 0 | "0 0" | int | 1 | 1 | 1 |
| 3 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 4 | none | 1 | 0 | "1 1" | int | 1 | 1 | 1 |
| 5 | none | 1 | 1 | "bad level \"99\"" | int | 1 | 1 | 1 |
| 6 | none | 1 | 0 | "0 0" | int | 1 | 1 | 1 |
| 7 | none | 1 | 1 | "bad level \"1.0\"" | double | 1 | 1 | 1 |
| 8 | none | 1 | 0 | "0 0" | int | 1 | 1 | 1 |
| 9 | none | 1 | 0 | "0 0" | levelReference | 1 | 1 | 1 |
| 10 | none | 1 | 1 | "BODY" | levelReference | 1 | 1 | 1 |
| 11 | none | 1 | 1 | "missing close-brace" | levelReference | 1 | 1 | 1 |
| 12 | none | 1 | 0 | "VALUE" | levelReference | 1 | 0 | 1 |
| 13 | none | 1 | 1 | "invoked \"break\" outside of a loop" | levelReference | 1 | 1 | 1 |

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: binary_sha256=7ce70e0eeb65af535bcc5e10cb3f87d603827231dd51df21c6290af412fec6d6; archive_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; internal_header_sha256=f000e14fdc4e00da1c5ab98a8087c41be3cbf26564a274152236e0531bb55053. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

Rows use original ordinal: 0 #0, 1 0, 2 1, 3 bad, 4 +0, 5 99, 6 -1, 7 1.0, 8 counted 1\x00X, 9 counted #0\x00X; 10 error BODY, 11 parse error, 12 pure list script, 13 break.

| ordinal | before type | before string | code | result | after type | after level string | after body string | restored |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | none | 1 | 0 | "0 0" | levelReference | 1 | 1 | 1 |
| 1 | none | 1 | 0 | "1 1" | int | 1 | 1 | 1 |
| 2 | none | 1 | 0 | "0 0" | int | 1 | 1 | 1 |
| 3 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 4 | none | 1 | 0 | "1 1" | int | 1 | 1 | 1 |
| 5 | none | 1 | 1 | "bad level \"99\"" | int | 1 | 1 | 1 |
| 6 | none | 1 | 1 | "bad level \"-1\"" | int | 1 | 1 | 1 |
| 7 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 8 | none | 1 | 0 | "0 0" | int | 1 | 1 | 1 |
| 9 | none | 1 | 0 | "0 0" | levelReference | 1 | 1 | 1 |
| 10 | none | 1 | 1 | "BODY" | levelReference | 1 | 1 | 1 |
| 11 | none | 1 | 1 | "missing close-brace" | levelReference | 1 | 1 | 1 |
| 12 | none | 1 | 0 | "VALUE" | levelReference | 1 | 0 | 1 |
| 13 | none | 1 | 1 | "invoked \"break\" outside of a loop" | levelReference | 1 | 1 | 1 |

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: binary_sha256=46487b03637e6716d4460bd4fa6ea422933dc4d14c058f1a2c0509c9c265bbdd; archive_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; internal_header_sha256=fea9d99020df9aec6efebf6e3f18592ee267a9bf80d3843df0d6439efebe65cc. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

Rows use original ordinal: 0 #0, 1 0, 2 1, 3 bad, 4 +0, 5 99, 6 -1, 7 1.0, 8 counted 1\x00X, 9 counted #0\x00X; 10 error BODY, 11 parse error, 12 pure list script, 13 break.

| ordinal | before type | before string | code | result | after type | after level string | after body string | restored |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | none | 1 | 0 | "0 0" | levelReference | 1 | 1 | 1 |
| 1 | none | 1 | 0 | "1 1" | int | 1 | 1 | 1 |
| 2 | none | 1 | 0 | "0 0" | int | 1 | 1 | 1 |
| 3 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 4 | none | 1 | 0 | "1 1" | int | 1 | 1 | 1 |
| 5 | none | 1 | 1 | "bad level \"99\"" | int | 1 | 1 | 1 |
| 6 | none | 1 | 1 | "bad level \"-1\"" | int | 1 | 1 | 1 |
| 7 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 8 | none | 1 | 0 | "0 0" | int | 1 | 1 | 1 |
| 9 | none | 1 | 0 | "0 0" | levelReference | 1 | 1 | 1 |
| 10 | none | 1 | 1 | "BODY" | levelReference | 1 | 1 | 1 |
| 11 | none | 1 | 1 | "missing close-brace" | levelReference | 1 | 1 | 1 |
| 12 | none | 1 | 0 | "VALUE" | levelReference | 1 | 0 | 1 |
| 13 | none | 1 | 1 | "invoked \"break\" outside of a loop" | levelReference | 1 | 1 | 1 |

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (provider): [rust/tcl-vm/tests/data/native_explicit_uplevel/manifest.json](../../../../rust/tcl-vm/tests/data/native_explicit_uplevel/manifest.json). SHA-256 `5dfa925c198fa40edc5c80928feabc30e46b0004b14509aa182728bda9703ed5`. Five exact builds, private-header/archive/binary/output hashes and fourteen-window counts.
- `e1` (input): [rust/tcl-vm/tests/data/native_explicit_uplevel/probe.c](../../../../rust/tcl-vm/tests/data/native_explicit_uplevel/probe.c). SHA-256 `19b13753a2fbbb7b84840f29c40cf8a22f20c81ba167b383902761b3b65545f8`. Retained exact probe.c; original source/object inputs and reporter boundary, independent of Rust implementation.
- `e2` (observation): [rust/tcl-vm/tests/data/native_explicit_uplevel/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_explicit_uplevel/8.4.20.tsv). SHA-256 `f3d08aba4f5105cb9664064149b09c1101c529380efe9a5ff1650cc86e69981e`. Complete retained semantic TSV for 8.4.20; Rows use original ordinal: 0 #0, 1 0, 2 1, 3 bad, 4 +0, 5 99, 6 -1, 7 1.0, 8 counted 1\x00X, 9 counted #0\x00X; 10 error BODY, 11 parse error, 12 pure list script, 13 break.
- `e3` (observation): [rust/tcl-vm/tests/data/native_explicit_uplevel/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_explicit_uplevel/8.5.19.tsv). SHA-256 `f325c417eec6dc813fbcb8c4ccb8e8e4769fc82a36e17f469432c20840fe6210`. Complete retained semantic TSV for 8.5.19; Rows use original ordinal: 0 #0, 1 0, 2 1, 3 bad, 4 +0, 5 99, 6 -1, 7 1.0, 8 counted 1\x00X, 9 counted #0\x00X; 10 error BODY, 11 parse error, 12 pure list script, 13 break.
- `e4` (observation): [rust/tcl-vm/tests/data/native_explicit_uplevel/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_explicit_uplevel/8.6.18.tsv). SHA-256 `752d4ef6cb72e937cfa499b75d865d339957271d7de7eba60c74a0f28f882908`. Complete retained semantic TSV for 8.6.18; Rows use original ordinal: 0 #0, 1 0, 2 1, 3 bad, 4 +0, 5 99, 6 -1, 7 1.0, 8 counted 1\x00X, 9 counted #0\x00X; 10 error BODY, 11 parse error, 12 pure list script, 13 break.
- `e5` (observation): [rust/tcl-vm/tests/data/native_explicit_uplevel/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_explicit_uplevel/9.0.4.tsv). SHA-256 `8f379a8d2c451a9d9dc9c2cfc0a7c9d80880d9f82137841b1c9074c838c5f15a`. Complete retained semantic TSV for 9.0.4; Rows use original ordinal: 0 #0, 1 0, 2 1, 3 bad, 4 +0, 5 99, 6 -1, 7 1.0, 8 counted 1\x00X, 9 counted #0\x00X; 10 error BODY, 11 parse error, 12 pure list script, 13 break.
- `e6` (observation): [rust/tcl-vm/tests/data/native_explicit_uplevel/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_explicit_uplevel/9.1.0.tsv). SHA-256 `8f379a8d2c451a9d9dc9c2cfc0a7c9d80880d9f82137841b1c9074c838c5f15a`. Complete retained semantic TSV for 9.1.0; Rows use original ordinal: 0 #0, 1 0, 2 1, 3 bad, 4 +0, 5 99, 6 -1, 7 1.0, 8 counted 1\x00X, 9 counted #0\x00X; 10 error BODY, 11 parse error, 12 pure list script, 13 break.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/exec/native_uplevel_tests.rs](../../../../rust/tcl-vm/src/exec/native_uplevel_tests.rs), `explicit_uplevel_preserves_seventy_native_level_body_and_restore_windows`: Independent current Rust comparison at the stated semantic boundary.
- [rust/tcl-vm/src/exec/native_uplevel_tests.rs](../../../../rust/tcl-vm/src/exec/native_uplevel_tests.rs), `exec::native_uplevel_tests::explicit_uplevel_preserves_seventy_native_level_body_and_restore_windows` (linked): Compares the seventy exact original level/cache/body result windows and requires frame restoration, independently of portable uplevel opcode selection.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cc",
  "-DSTDC_HEADERS=1",
  "-DHAVE_UNISTD_H=1",
  "-I/path/to/recorded/generic",
  "-I/path/to/recorded/unix",
  "rust/tcl-vm/tests/data/native_explicit_uplevel/probe.c",
  "/path/to/recorded/libtcl.a",
  "-lm",
  "-ldl",
  "-lpthread",
  "-lz",
  "-o",
  "/tmp/native-variable-proof"
]
```

Replace provider/header/archive paths with the recorded release/build and preserve the manifested original compiler options. Execute /tmp/native-variable-proof in a fresh process; require all stdout bytes to equal that provider's retained transcript, empty stderr and exit 0. The retained probe uses private interpreter/frame/compiler headers; a stock shell invocation is not an equivalent replay. Original artifact hashes identify the observed build, not a fresh rerun. No Rust pass is inferred.
