# naming.substitution.root-frame-probe-failure

Kind: `native-observation`

## Problem statement

A private-header probe can crash when it assumes a non-null root varFrame. Calling that crash a Tcl semantic error or unavailable naming capability would confuse harness validity with guest behavior. The original failed process and the separately pinned finite null-frame controls must remain independently recorded.

## Question

Did the v4 private root-frame probe execute, and did explicit null-frame handling in v5/v6 restore this harness?

## Conclusion

v4 C8.4 compiled successfully but exited with SIGSEGV (-11) and emitted zero rows; the other four C providers emitted their retained rows with exit0. The probe dereferenced the null root varFrame. v5/v6 retain an explicit null-frame boolean and all five providers compile/process exit0. The failed C8.4 v4 attempt answers process availability only, not any guest substitution or naming question.

## Scope

Exact configured v4 process receipts and raw streams, compared with the separately pinned v5/v6 probes. Signal11 and zero rows are preserved. No Rust result, engine-level guest failure, or availability of an unexecuted operation is inferred.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel). Build: Actual executed v6 probe 01ca0f3ddb0b292c94e267c9341f7e57bf29bb041bfd45a795bd051050241892; matching header 824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf, static library 532be0a794ca8277c5ad979a227c38600f27c3c91d436e5d3731d661c271fe47, Makefile 0fb0c580d2402093bb212ad340ea87f88822aa5844a8114e112bec4c990411b1 and private-header/configured-flag receipts retained. Earlier variants retain their own build hashes.. Channel: Configured native probe process and private varFrame inspection; v4 C8.4 terminates before any guest result row, while v5/v6 explicitly handle a null root frame.. Dialect: Tcl.

v4 SIGSEGV -11 and zero rows; v5/v6 compile/process0 with explicit null-frame handling.

### tcl8.5

Status: `observed`. Version: 8.5.19 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel). Build: Actual executed v6 probe 3089d7fb65500eed6e072d99213cf2d9a2effb815b3f927f34338999c613ae86; matching header c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5, static library 94313727ea507b1b2b00018062a0f57587458a3509c37f2e4b351dd58d0592dc, Makefile 26ae775d2e4657ecfcb45421cc77c4c26170cb2a21985fbb002eb4791bc766a5 and private-header/configured-flag receipts retained. Earlier variants retain their own build hashes.. Channel: Configured native probe process and private varFrame inspection; v4 C8.4 terminates before any guest result row, while v5/v6 explicitly handle a null root frame.. Dialect: Tcl.

v4/v5/v6 compile/process0 and retain actual frame/cache booleans.

### tcl8.6

Status: `observed`. Version: 8.6.18 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel). Build: Actual executed v6 probe a41fc11b43cf4611115f3234ffe29bd4732433aaffe41525d108bca172576a5d; matching header aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245, static library 980630e3a7d37f82322fda1a7e45a85b86d8d905cb395a2aa0c083b3c7eaa8cb, Makefile 8afb8697cb70b90518876861086bdb43f6e31b5e96e6d8091ae7b1de33d90d7e and private-header/configured-flag receipts retained. Earlier variants retain their own build hashes.. Channel: Configured native probe process and private varFrame inspection; v4 C8.4 terminates before any guest result row, while v5/v6 explicitly handle a null root frame.. Dialect: Tcl.

v4/v5/v6 compile/process0 and retain actual frame/cache booleans.

### tcl9.0

Status: `observed`. Version: 9.0.4 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel). Build: Actual executed v6 probe 77a13410411609c8d0b087639070801ec8b09b7d0afa40c734bb819737eb62c3; matching header eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a, static library dace08db925224497714ba86969fabb51c05a78f2bda22db6c05f795c3a70ae4, Makefile 69f1915c208d66c7e38e6871c7f51c8f7be7c2138b45a5361641f9e91854fea5 and private-header/configured-flag receipts retained. Earlier variants retain their own build hashes.. Channel: Configured native probe process and private varFrame inspection; v4 C8.4 terminates before any guest result row, while v5/v6 explicitly handle a null root frame.. Dialect: Tcl.

v4/v5/v6 compile/process0 and retain actual frame/cache booleans.

### tcl9.1

Status: `observed`. Version: 9.1.0 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel). Build: Actual executed v6 probe 74d4c16f0747ad8d33b827cacaca26eeaf1e5f90ce6eff70892d83871e593755; matching header 30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950, static library 513028850edf096856a29e4e16869aac91cba2ef75c3924aaaaeeb6e45e167db, Makefile c1ecfb5a77697f0dc6f1057f63ff7aabd75b444f62b5954480aff01ff0565ab1 and private-header/configured-flag receipts retained. Earlier variants retain their own build hashes.. Channel: Configured native probe process and private varFrame inspection; v4 C8.4 terminates before any guest result row, while v5/v6 explicitly handle a null root frame.. Dialect: Tcl.

v4/v5/v6 compile/process0 and retain actual frame/cache booleans.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This exact C substitution/private-header question was not tested on this provider.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This exact C substitution/private-header question was not tested on this provider.

## Historical references

- `e0` (input): [rust/tcl-registry/tests/data/native_substitution_owner/inputs.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/inputs.json). Complete ten native template byte arrays as hexadecimal. v1 shortened counts are separately encoded in its original probe.
- `e1` (input): [rust/tcl-registry/tests/data/native_substitution_owner/v1/probe.c](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1/probe.c). Exact immutable v1 translation unit and original counted template/setup operations.
- `e2` (input): [rust/tcl-registry/tests/data/native_substitution_owner/v2/probe.c](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2/probe.c). Exact immutable v2 translation unit and original counted template/setup operations.
- `e3` (input): [rust/tcl-registry/tests/data/native_substitution_owner/v3/probe.c](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3/probe.c). Exact immutable v3 translation unit and original counted template/setup operations.
- `e4` (input): [rust/tcl-registry/tests/data/native_substitution_owner/v4/probe.c](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4/probe.c). Exact immutable v4 translation unit and original counted template/setup operations.
- `e5` (input): [rust/tcl-registry/tests/data/native_substitution_owner/v5/probe.c](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5/probe.c). Exact immutable v5 translation unit and original counted template/setup operations.
- `e6` (input): [rust/tcl-registry/tests/data/native_substitution_owner/v6/probe.c](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/probe.c). Exact immutable v6 translation unit and original counted template/setup operations.
- `e7` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/manifest.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/manifest.json). Finite corpus provider mapping, variant input/setup/process limits and source excerpts.
- `e8` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v1/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1/8.4.20/receipt.json). Original v1 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e9` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v1/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1/8.4.20/stdout.tsv). Exact v1 reached raw stream, 170 rows; exit 0.
- `e10` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v2/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2/8.4.20/receipt.json). Original v2 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e11` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v2/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2/8.4.20/stdout.tsv). Exact v2 reached raw stream, 343 rows; exit 0.
- `e12` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v3/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3/8.4.20/receipt.json). Original v3 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e13` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v3/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3/8.4.20/stdout.tsv). Exact v3 reached raw stream, 355 rows; exit 0.
- `e14` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v4/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4/8.4.20/receipt.json). Original v4 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e15` (limitation): [rust/tcl-registry/tests/data/native_substitution_owner/v4/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4/8.4.20/stdout.tsv). Exact v4 reached raw stream, 0 rows; exit -11.
- `e16` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v5/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5/8.4.20/receipt.json). Original v5 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e17` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v5/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5/8.4.20/stdout.tsv). Exact v5 reached raw stream, 538 rows; exit 0.
- `e18` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v6/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/8.4.20/receipt.json). Original v6 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e19` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v6/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/8.4.20/stdout.tsv). Exact v6 reached raw stream, 539 rows; exit 0.
- `e20` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v1/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1/8.5.19/receipt.json). Original v1 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e21` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v1/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1/8.5.19/stdout.tsv). Exact v1 reached raw stream, 170 rows; exit 0.
- `e22` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v2/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2/8.5.19/receipt.json). Original v2 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e23` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v2/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2/8.5.19/stdout.tsv). Exact v2 reached raw stream, 343 rows; exit 0.
- `e24` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v3/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3/8.5.19/receipt.json). Original v3 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e25` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v3/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3/8.5.19/stdout.tsv). Exact v3 reached raw stream, 355 rows; exit 0.
- `e26` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v4/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4/8.5.19/receipt.json). Original v4 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e27` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v4/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4/8.5.19/stdout.tsv). Exact v4 reached raw stream, 538 rows; exit 0.
- `e28` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v5/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5/8.5.19/receipt.json). Original v5 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e29` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v5/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5/8.5.19/stdout.tsv). Exact v5 reached raw stream, 538 rows; exit 0.
- `e30` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v6/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/8.5.19/receipt.json). Original v6 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e31` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v6/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/8.5.19/stdout.tsv). Exact v6 reached raw stream, 539 rows; exit 0.
- `e32` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v1/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1/8.6.18/receipt.json). Original v1 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e33` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v1/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1/8.6.18/stdout.tsv). Exact v1 reached raw stream, 961 rows; exit 0.
- `e34` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v2/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2/8.6.18/receipt.json). Original v2 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e35` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v2/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2/8.6.18/stdout.tsv). Exact v2 reached raw stream, 1156 rows; exit 0.
- `e36` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v3/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3/8.6.18/receipt.json). Original v3 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e37` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v3/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3/8.6.18/stdout.tsv). Exact v3 reached raw stream, 1182 rows; exit 0.
- `e38` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v4/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4/8.6.18/receipt.json). Original v4 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e39` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v4/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4/8.6.18/stdout.tsv). Exact v4 reached raw stream, 1371 rows; exit 0.
- `e40` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v5/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5/8.6.18/receipt.json). Original v5 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e41` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v5/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5/8.6.18/stdout.tsv). Exact v5 reached raw stream, 1371 rows; exit 0.
- `e42` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v6/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/8.6.18/receipt.json). Original v6 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e43` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v6/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/8.6.18/stdout.tsv). Exact v6 reached raw stream, 1372 rows; exit 0.
- `e44` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v1/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1/9.0.4/receipt.json). Original v1 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e45` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v1/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1/9.0.4/stdout.tsv). Exact v1 reached raw stream, 961 rows; exit 0.
- `e46` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v2/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2/9.0.4/receipt.json). Original v2 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e47` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v2/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2/9.0.4/stdout.tsv). Exact v2 reached raw stream, 1156 rows; exit 0.
- `e48` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v3/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3/9.0.4/receipt.json). Original v3 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e49` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v3/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3/9.0.4/stdout.tsv). Exact v3 reached raw stream, 1182 rows; exit 0.
- `e50` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v4/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4/9.0.4/receipt.json). Original v4 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e51` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v4/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4/9.0.4/stdout.tsv). Exact v4 reached raw stream, 1371 rows; exit 0.
- `e52` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v5/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5/9.0.4/receipt.json). Original v5 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e53` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v5/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5/9.0.4/stdout.tsv). Exact v5 reached raw stream, 1371 rows; exit 0.
- `e54` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v6/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/9.0.4/receipt.json). Original v6 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e55` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v6/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/9.0.4/stdout.tsv). Exact v6 reached raw stream, 1372 rows; exit 0.
- `e56` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v1/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1/9.1.0/receipt.json). Original v1 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e57` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v1/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1/9.1.0/stdout.tsv). Exact v1 reached raw stream, 961 rows; exit 0.
- `e58` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v2/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2/9.1.0/receipt.json). Original v2 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e59` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v2/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2/9.1.0/stdout.tsv). Exact v2 reached raw stream, 1156 rows; exit 0.
- `e60` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v3/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3/9.1.0/receipt.json). Original v3 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e61` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v3/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3/9.1.0/stdout.tsv). Exact v3 reached raw stream, 1182 rows; exit 0.
- `e62` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v4/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4/9.1.0/receipt.json). Original v4 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e63` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v4/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4/9.1.0/stdout.tsv). Exact v4 reached raw stream, 1371 rows; exit 0.
- `e64` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v5/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5/9.1.0/receipt.json). Original v5 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e65` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v5/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5/9.1.0/stdout.tsv). Exact v5 reached raw stream, 1371 rows; exit 0.
- `e66` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v6/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/9.1.0/receipt.json). Original v6 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e67` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v6/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/9.1.0/stdout.tsv). Exact v6 reached raw stream, 1372 rows; exit 0.
- `row-input-audit` (limitation): [docs/design/analysis/name-resolution-proofs/substitution-row-input-audit.json](../../../../docs/design/analysis/name-resolution-proofs/substitution-row-input-audit.json). Offline named-field extraction and per-question one-based original row selections; retains all template ordinals, masks, repeated labels and empty results. No fresh execution.
- `offline-provider-checks` (limitation): [docs/design/analysis/name-resolution-proofs/substitution-offline-checks.json](../../../../docs/design/analysis/name-resolution-proofs/substitution-offline-checks.json). Actual six-variant verify-only checks of30 configured source/header/library/flags/stream associations; no compiler, native executable or Rust assertion was launched.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-registry/tests/data/native_substitution_owner/replay.py",
  "--source-root",
  "tmp",
  "--output",
  "/tmp/native-substitution-reconfirmation",
  "--variant",
  "v6"
]
```

Requires exact configured C5 trees, matching public/private headers, static libraries, Makefiles and AC_FLAGS. It compares whole stdout/stderr and process exit against retained receipts. Run v1-v5 separately with --variant to preserve their input/setup differences; v4 intentionally retains the C8.4 harness crash. No Jim/BIG-IP launch or Rust execution is implied.
